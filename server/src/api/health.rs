use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::SharedState;

#[derive(Serialize, ToSchema)]
pub struct Capabilities {
    /// Output formats this build can produce. Filled in by the pipeline (Phase 2).
    pub formats: Vec<&'static str>,
    /// Chrome-impersonating client available for Cloudflare-fronted sources.
    pub impersonation: bool,
    /// Headless browser available for managed challenges (not built yet).
    pub browser: bool,
}

#[derive(Serialize, ToSchema)]
pub struct SourceCounts {
    pub implemented: usize,
    pub total: usize,
}

#[derive(Serialize, ToSchema)]
pub struct Health {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime_s: u64,
    pub active_tasks: usize,
    pub capabilities: Capabilities,
    pub sources: SourceCounts,
}

#[utoipa::path(get, path = "/health", tag = "system",
    responses((status = 200, body = Health)))]
pub async fn health(State(state): State<SharedState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_s: state.started_at.elapsed().as_secs(),
        active_tasks: state.engine.active_count(),
        capabilities: Capabilities {
            formats: crate::pipeline::available_formats(state.engine.rar_available),
            impersonation: state.fetcher.can_impersonate(),
            browser: state.config.browser_enabled,
        },
        sources: SourceCounts {
            implemented: state.registry.implemented().count(),
            total: state.registry.all().len(),
        },
    })
}
