use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use super::status::probe_all;
use crate::{
    model::{SourceMeta, SourceStatus},
    state::SharedState,
};

#[derive(Serialize, ToSchema)]
pub struct SourceInfo {
    #[serde(flatten)]
    pub meta: SourceMeta,
    /// Legacy numeric identifier as a string, the form the old client sends.
    pub legacy_id: String,
    pub status: SourceStatus,
}

#[utoipa::path(get, path = "/sources", tag = "sources",
    responses((status = 200, body = Vec<SourceInfo>)))]
pub async fn list_sources(State(state): State<SharedState>) -> Json<Vec<SourceInfo>> {
    let statuses = probe_all(&state).await;
    Json(
        state
            .registry
            .all()
            .iter()
            .map(|s| {
                let meta = s.meta().clone();
                SourceInfo {
                    legacy_id: meta.id_str(),
                    status: statuses
                        .get(meta.slug)
                        .copied()
                        .unwrap_or(SourceStatus::Unknown),
                    meta,
                }
            })
            .collect(),
    )
}
