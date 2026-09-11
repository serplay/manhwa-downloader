//! Routes and response shapes the current React client depends on. They stay
//! until the frontend rebuild (Phase 4) ships, then get deleted in Phase 5.
//!
//! Shapes (from `server-old/main.py`):
//! - `GET /status`            -> `{ "status": { "0": "ok" | "null", ... } }`
//! - `GET /search/?title&source` -> `{ "0": Comic, "1": Comic, ... }` or `{ "message": ... }`
//! - `GET /search/all?title`  -> `{ "0": <search result or {"error"}>, ... }`
//! - `GET /chapters/?id&source` -> `{ "Vol 1": { "volume": "Vol 1", "chapters": { "12": { "id", "chapter" } } } }`
//! - `GET /`                  -> `{ "message": "It Works!" }`

use std::collections::BTreeMap;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    chapters::chapters_for,
    download,
    proxy::proxy_url,
    search::{search_source, validate_query},
    status::probe_all,
};
use crate::{
    error::AppResult,
    model::{Comic, SourceStatus, Volume},
    sources::SearchOptions,
    state::SharedState,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(root))
        .route("/status", get(legacy_status))
        .route("/search/", get(legacy_search))
        .route("/search/all", get(legacy_search_all))
        .route("/chapters/", get(legacy_chapters))
        .route("/tasks/list", get(legacy_task_list))
        .route("/tasks/{task_id}/info", get(download::status))
}

/// Celery-style task listing. The old client never read it; kept for parity.
async fn legacy_task_list(State(state): State<SharedState>) -> Json<Value> {
    let tasks = state.engine.list();
    let active: Vec<_> = tasks.iter().filter(|t| !t.state.is_terminal()).collect();
    Json(json!({
        "active": { "local": active },
        "reserved": {},
        "total_active": active.len(),
        "total_reserved": 0
    }))
}

async fn root() -> Json<Value> {
    Json(json!({ "message": "It Works!" }))
}

#[derive(Serialize)]
struct LegacyComic {
    id: String,
    title: BTreeMap<String, String>,
    cover_art: String,
    #[serde(rename = "availableLanguages")]
    available_languages: Vec<String>,
}

fn legacy_comics(base_path: &str, comics: &[Comic]) -> Value {
    let map: BTreeMap<String, LegacyComic> = comics
        .iter()
        .enumerate()
        .map(|(i, c)| {
            (
                i.to_string(),
                LegacyComic {
                    id: c.id.clone(),
                    title: c.title.clone(),
                    cover_art: c
                        .cover
                        .as_ref()
                        .map(|cover| proxy_url(base_path, cover))
                        .unwrap_or_default(),
                    available_languages: c.languages.clone(),
                },
            )
        })
        .collect();
    serde_json::to_value(map).unwrap_or(Value::Null)
}

fn legacy_volumes(volumes: &[Volume]) -> Value {
    let mut out = serde_json::Map::new();
    for v in volumes {
        let chapters: serde_json::Map<String, Value> = v
            .chapters
            .iter()
            .enumerate()
            .map(|(i, c)| {
                (
                    // Key must be unique; the old server keyed by chapter number
                    // (MangaDex) or index (others). Index works for both.
                    i.to_string(),
                    json!({ "id": c.id, "chapter": c.number }),
                )
            })
            .collect();
        out.insert(
            v.name.clone(),
            json!({ "volume": v.name, "chapters": chapters }),
        );
    }
    Value::Object(out)
}

async fn legacy_status(State(state): State<SharedState>) -> Json<Value> {
    let statuses = probe_all(&state).await;
    let map: BTreeMap<String, &str> = state
        .registry
        .all()
        .iter()
        .map(|s| {
            let ok = matches!(statuses.get(s.meta().slug), Some(SourceStatus::Ok));
            (s.meta().id_str(), if ok { "ok" } else { "null" })
        })
        .collect();
    Json(json!({ "status": map }))
}

#[derive(Deserialize)]
struct LegacySearchParams {
    title: String,
    source: String,
}

async fn legacy_search(
    State(state): State<SharedState>,
    Query(p): Query<LegacySearchParams>,
) -> AppResult<Json<Value>> {
    let query = validate_query(&p.title)?;
    let source = state.registry.get(&p.source)?;
    let comics = search_source(&state, &source, &query, None, SearchOptions::default()).await?;
    if comics.is_empty() {
        return Ok(Json(json!({ "message": "No comics found" })));
    }
    Ok(Json(legacy_comics(&state.config.public_base_path, &comics)))
}

#[derive(Deserialize)]
struct LegacySearchAllParams {
    title: String,
}

async fn legacy_search_all(
    State(state): State<SharedState>,
    Query(p): Query<LegacySearchAllParams>,
) -> AppResult<Json<Value>> {
    let query = validate_query(&p.title)?;
    let mut out = serde_json::Map::new();
    let futures = state.registry.all().iter().map(|source| {
        let source = std::sync::Arc::clone(source);
        let state = state.clone();
        let query = query.clone();
        async move {
            let outcome =
                search_source(&state, &source, &query, None, SearchOptions::default()).await;
            (source.meta().id_str(), outcome)
        }
    });
    for (id, outcome) in futures::future::join_all(futures).await {
        let value = match outcome {
            Ok(comics) if comics.is_empty() => json!({ "message": "No comics found" }),
            Ok(comics) => legacy_comics(&state.config.public_base_path, &comics),
            Err(err) => json!({ "error": err.detail().message }),
        };
        out.insert(id, value);
    }
    Ok(Json(Value::Object(out)))
}

#[derive(Deserialize)]
struct LegacyChaptersParams {
    id: String,
    source: String,
}

async fn legacy_chapters(
    State(state): State<SharedState>,
    Query(p): Query<LegacyChaptersParams>,
) -> AppResult<Json<Value>> {
    let source = state.registry.get(&p.source)?;
    let volumes = chapters_for(&state, &source, &p.id, None).await?;
    Ok(Json(legacy_volumes(&volumes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Chapter, Cover};

    #[test]
    fn comics_are_keyed_by_index_with_proxied_cover() {
        let comics = vec![Comic {
            id: "abc".into(),
            title: [("en".to_string(), "T".to_string())].into_iter().collect(),
            cover: Some(Cover {
                url: "https://uploads.mangadex.org/covers/abc/x.jpg".into(),
                referer: None,
            }),
            languages: vec!["en".into()],
            adult: false,
        }];
        let v = legacy_comics("/api", &comics);
        assert_eq!(v["0"]["id"], "abc");
        assert!(
            v["0"]["cover_art"]
                .as_str()
                .unwrap()
                .starts_with("/api/proxy-image?url=")
        );
        assert_eq!(v["0"]["availableLanguages"][0], "en");
    }

    #[test]
    fn volumes_match_old_shape() {
        let volumes = vec![Volume {
            name: "Vol 1".into(),
            chapters: vec![Chapter {
                id: "c1".into(),
                number: "1".into(),
                title: None,
                pages: None,
            }],
        }];
        let v = legacy_volumes(&volumes);
        assert_eq!(v["Vol 1"]["volume"], "Vol 1");
        assert_eq!(v["Vol 1"]["chapters"]["0"]["chapter"], "1");
        assert_eq!(v["Vol 1"]["chapters"]["0"]["id"], "c1");
    }
}
