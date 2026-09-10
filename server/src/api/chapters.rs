use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    cache::Caches,
    error::{AppError, AppResult},
    model::Volume,
    sources::Source,
    state::SharedState,
};

#[derive(Deserialize, IntoParams)]
pub struct ChaptersParams {
    /// Source id or slug.
    pub source: String,
    /// Comic id as returned by search.
    pub id: String,
    /// Chapter language for multi-language sources. Defaults to `en`.
    pub lang: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ChaptersResponse {
    pub source: String,
    pub comic_id: String,
    pub volumes: Vec<Volume>,
    pub total_chapters: usize,
}

pub async fn chapters_for(
    state: &SharedState,
    source: &Arc<dyn Source>,
    comic_id: &str,
    lang: Option<&str>,
) -> AppResult<Arc<Vec<Volume>>> {
    let comic_id = comic_id.trim();
    if comic_id.is_empty() {
        return Err(AppError::Validation("comic id must not be empty".into()));
    }
    let slug = source.meta().slug;
    let key = Caches::chapters_key(slug, comic_id, lang);
    if let Some(hit) = state.caches.chapters.get(&key).await {
        return Ok(hit);
    }
    let ctx = state.ctx();
    let volumes = Arc::new(source.chapters(&ctx, comic_id, lang).await?);
    if !volumes.is_empty() {
        state
            .caches
            .chapters
            .insert(key, Arc::clone(&volumes))
            .await;
    }
    Ok(volumes)
}

#[utoipa::path(get, path = "/chapters", tag = "catalogue", params(ChaptersParams),
    responses(
        (status = 200, body = ChaptersResponse),
        (status = 400, body = crate::error::ErrorBody),
        (status = 502, body = crate::error::ErrorBody)))]
pub async fn chapters(
    State(state): State<SharedState>,
    Query(params): Query<ChaptersParams>,
) -> AppResult<Json<ChaptersResponse>> {
    let source = state.registry.get(&params.source)?;
    let volumes = chapters_for(&state, &source, &params.id, params.lang.as_deref()).await?;
    Ok(Json(ChaptersResponse {
        source: source.meta().slug.to_string(),
        comic_id: params.id.trim().to_string(),
        total_chapters: volumes.iter().map(|v| v.chapters.len()).sum(),
        volumes: (*volumes).clone(),
    }))
}
