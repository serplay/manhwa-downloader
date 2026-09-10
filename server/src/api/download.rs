//! Download lifecycle: start, poll, stream events, fetch the file, cancel.

use std::convert::Infallible;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, RawQuery, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::{Stream, StreamExt};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    model::Format,
    pipeline::ascii_filename,
    state::SharedState,
    tasks::{ChapterRef, DownloadRequest, TaskStatus},
};

#[derive(Serialize, ToSchema)]
pub struct DownloadAccepted {
    pub task_id: Uuid,
    pub state: crate::tasks::TaskState,
    /// Legacy field the old client displayed.
    pub status: String,
    pub message: String,
    pub status_url: String,
    pub events_url: String,
    pub file_url: String,
}

/// Parse the old query form: `ids[]=<chapter_id>_<number>&source=&comic_title=&format=`.
/// Chapter ids may contain underscores, so we split at the last one.
pub fn parse_legacy_query(raw: &str) -> AppResult<DownloadRequest> {
    let mut chapters = Vec::new();
    let mut source = None;
    let mut comic_title = None;
    let mut format = None;
    for (k, v) in url::form_urlencoded::parse(raw.as_bytes()) {
        match &*k {
            "ids[]" | "ids" => {
                let (id, number) = match v.rsplit_once('_') {
                    Some((id, n)) if !id.is_empty() => (id.to_string(), n.to_string()),
                    _ => (v.to_string(), "0".to_string()),
                };
                chapters.push(ChapterRef { id, number });
            }
            "source" => source = Some(v.to_string()),
            "comic_title" => comic_title = Some(v.to_string()),
            "format" => {
                format = Some(
                    serde_json::from_value::<Format>(serde_json::Value::String(v.to_lowercase()))
                        .map_err(|_| {
                        AppError::Validation("invalid format; allowed: pdf, cbz, cbr, epub".into())
                    })?,
                )
            }
            _ => {}
        }
    }
    Ok(DownloadRequest {
        source: source.ok_or_else(|| AppError::Validation("source must be specified".into()))?,
        comic_title: comic_title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "Chapters".into()),
        format: format.unwrap_or(Format::Pdf),
        chapters,
        lang: None,
    })
}

#[utoipa::path(post, path = "/download", tag = "downloads",
    request_body = DownloadRequest,
    responses(
        (status = 202, body = DownloadAccepted),
        (status = 400, body = crate::error::ErrorBody),
        (status = 422, description = "Unsupported format on this server", body = crate::error::ErrorBody),
        (status = 501, description = "Source cannot download yet", body = crate::error::ErrorBody)))]
pub async fn start_download(
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> AppResult<(StatusCode, Json<DownloadAccepted>)> {
    let req = match query.as_deref() {
        Some(q) if q.contains("ids") => parse_legacy_query(q)?,
        _ => {
            if body.is_empty() {
                return Err(AppError::Validation(
                    "send a JSON body with source, comic_title, format and chapters".into(),
                ));
            }
            serde_json::from_slice::<DownloadRequest>(&body)
                .map_err(|e| AppError::Validation(format!("invalid request body: {e}")))?
        }
    };
    let count = req.chapters.len();
    let status = state.engine.spawn(state.clone(), req)?;
    let id = status.task_id;
    Ok((
        StatusCode::ACCEPTED,
        Json(DownloadAccepted {
            task_id: id,
            state: status.state,
            status: "Task has been added to the queue".into(),
            message: format!("Started downloading {count} chapters"),
            status_url: format!("/download/status/{id}"),
            events_url: format!("/download/events/{id}"),
            file_url: format!("/download/file/{id}"),
        }),
    ))
}

#[utoipa::path(get, path = "/download/status/{task_id}", tag = "downloads",
    params(("task_id" = Uuid, Path)),
    responses((status = 200, body = TaskStatus), (status = 404, body = crate::error::ErrorBody)))]
pub async fn status(
    State(state): State<SharedState>,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<TaskStatus>> {
    Ok(Json(state.engine.status(task_id)?))
}

#[utoipa::path(get, path = "/download/events/{task_id}", tag = "downloads",
    params(("task_id" = Uuid, Path)),
    responses(
        (status = 200, description = "Server-Sent Events stream of TaskStatus objects; closes on a terminal state", content_type = "text/event-stream"),
        (status = 404, body = crate::error::ErrorBody)))]
pub async fn events(
    State(state): State<SharedState>,
    Path(task_id): Path<Uuid>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let mut rx = state.engine.subscribe(task_id)?;
    let stream = async_stream::stream! {
        loop {
            let snapshot = rx.borrow_and_update().clone();
            let terminal = snapshot.state.is_terminal();
            if let Ok(event) = Event::default().event("status").json_data(&snapshot) {
                yield Ok(event);
            }
            if terminal || rx.changed().await.is_err() {
                break;
            }
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

fn content_disposition(title: &str, extension: &str) -> HeaderValue {
    let ascii = ascii_filename(title, extension);
    let full = format!("{}.{extension}", title.trim());
    let utf8 = utf8_percent_encode(&full, NON_ALPHANUMERIC);
    HeaderValue::from_str(&format!(
        "attachment; filename=\"{ascii}\"; filename*=UTF-8''{utf8}"
    ))
    .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

#[utoipa::path(get, path = "/download/file/{task_id}", tag = "downloads",
    params(("task_id" = Uuid, Path)),
    responses(
        (status = 200, description = "The archive", content_type = "application/octet-stream"),
        (status = 404, description = "Unknown task, failed task, or file already collected", body = crate::error::ErrorBody),
        (status = 409, description = "Task still running", body = crate::error::ErrorBody)))]
pub async fn file(
    State(state): State<SharedState>,
    Path(task_id): Path<Uuid>,
) -> AppResult<Response> {
    let collectable = state.engine.collectable(task_id)?;
    let packaged = collectable.packaged;
    let file = tokio::fs::File::open(&packaged.path)
        .await
        .map_err(|e| AppError::NotFound(format!("archive missing on disk: {e}")))?;
    let total = file.metadata().await.map(|m| m.len()).unwrap_or(0);

    let engine_state = state.clone();
    let body_stream = async_stream::stream! {
        let mut reader = ReaderStream::with_capacity(file, 1 << 20);
        let mut sent: u64 = 0;
        while let Some(chunk) = reader.next().await {
            if let Ok(c) = &chunk {
                sent += c.len() as u64;
            }
            // Only a complete transfer releases the file. An aborted download (or a
            // browser's exploratory first request) leaves it for a retry; the sweeper
            // removes it after the retention window. This runs before the final
            // yield because hyper stops polling once Content-Length bytes are out.
            if sent >= total {
                engine_state.engine.mark_collected(task_id);
            }
            yield chunk;
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(packaged.content_type),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(total));
    headers.insert(
        header::CONTENT_DISPOSITION,
        content_disposition(&collectable.comic_title, packaged.extension),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((StatusCode::OK, headers, Body::from_stream(body_stream)).into_response())
}

#[derive(Serialize, ToSchema)]
pub struct Cancelled {
    pub status: &'static str,
    pub task_id: Uuid,
}

#[utoipa::path(post, path = "/download/cancel/{task_id}", tag = "downloads",
    params(("task_id" = Uuid, Path)),
    responses(
        (status = 200, body = Cancelled),
        (status = 404, body = crate::error::ErrorBody),
        (status = 409, description = "Already finished", body = crate::error::ErrorBody)))]
pub async fn cancel(
    State(state): State<SharedState>,
    Path(task_id): Path<Uuid>,
) -> AppResult<Json<Cancelled>> {
    state.engine.cancel(task_id)?;
    Ok(Json(Cancelled {
        status: "cancelled",
        task_id,
    }))
}

#[utoipa::path(get, path = "/tasks", tag = "downloads",
    responses((status = 200, body = Vec<TaskStatus>)))]
pub async fn list_tasks(State(state): State<SharedState>) -> Json<Vec<TaskStatus>> {
    Json(state.engine.list())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_query_splits_at_last_underscore() {
        let req = parse_legacy_query(
            "ids%5B%5D=abc_def_12.5&ids%5B%5D=xyz_3&source=0&comic_title=Solo&format=CBZ",
        )
        .unwrap();
        assert_eq!(req.chapters.len(), 2);
        assert_eq!(req.chapters[0].id, "abc_def");
        assert_eq!(req.chapters[0].number, "12.5");
        assert_eq!(req.chapters[1].id, "xyz");
        assert_eq!(req.source, "0");
        assert_eq!(req.comic_title, "Solo");
        assert_eq!(req.format, Format::Cbz);
    }

    #[test]
    fn legacy_query_rejects_bad_format() {
        assert!(parse_legacy_query("ids%5B%5D=a_1&source=0&format=docx").is_err());
    }

    #[test]
    fn disposition_has_ascii_and_utf8_names() {
        let v = content_disposition("ワンピース: Vol 1", "zip");
        let s = v.to_str().unwrap();
        assert!(s.contains("filename=\"Vol 1.zip\""));
        assert!(s.contains("filename*=UTF-8''"));
    }
}
