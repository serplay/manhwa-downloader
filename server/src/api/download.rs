//! Download lifecycle: start, poll, stream events, fetch the file, cancel.

use std::convert::Infallible;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::Stream;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    pipeline::ascii_filename,
    state::SharedState,
    tasks::{DownloadRequest, TaskStatus},
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

#[utoipa::path(post, path = "/download", tag = "downloads",
    request_body = DownloadRequest,
    responses(
        (status = 202, body = DownloadAccepted),
        (status = 400, body = crate::error::ErrorBody),
        (status = 422, description = "Unsupported format on this server", body = crate::error::ErrorBody),
        (status = 501, description = "Source cannot download yet", body = crate::error::ErrorBody)))]
pub async fn start_download(
    State(state): State<SharedState>,
    body: Bytes,
) -> AppResult<(StatusCode, Json<DownloadAccepted>)> {
    if body.is_empty() {
        return Err(AppError::Validation(
            "send a JSON body with source, comic_title, format and chapters".into(),
        ));
    }
    let req = serde_json::from_slice::<DownloadRequest>(&body)
        .map_err(|e| AppError::Validation(format!("invalid request body: {e}")))?;
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
        (status = 404, description = "Unknown task, failed task, or an archive whose retention window has closed", body = crate::error::ErrorBody),
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

    // The archive is not deleted when it is sent. The bytes leaving this socket
    // do not mean the browser received them: a proxy hop or a dropped connection
    // can lose the transfer, and the next attempt has to find the file still
    // here. The sweeper removes it when the retention window closes.
    let body_stream = ReaderStream::with_capacity(file, 1 << 20);

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
    fn disposition_has_ascii_and_utf8_names() {
        let v = content_disposition("ワンピース: Vol 1", "zip");
        let s = v.to_str().unwrap();
        assert!(s.contains("filename=\"Vol 1.zip\""));
        assert!(s.contains("filename*=UTF-8''"));
    }
}
