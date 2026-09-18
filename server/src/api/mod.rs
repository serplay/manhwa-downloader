//! HTTP surface. Every route is documented in OpenAPI at `/docs`.

pub mod chapters;
pub mod download;
pub mod health;
pub mod proxy;
pub mod ratelimit;
pub mod search;
pub mod sources;
pub mod status;

#[cfg(test)]
mod flow_tests;

use std::time::Duration;

use axum::{Router, http::HeaderValue};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

use crate::state::SharedState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Manhwa Downloader API",
        description = "Search manga sources, list chapters, and build downloadable archives.",
        version = env!("CARGO_PKG_VERSION")
    ),
    tags(
        (name = "sources", description = "Source catalogue and health"),
        (name = "catalogue", description = "Search and chapter listings"),
        (name = "downloads", description = "Background download tasks"),
        (name = "images", description = "Cover image proxy"),
        (name = "system", description = "Health and diagnostics")
    )
)]
struct ApiDoc;

/// The documented routes and the OpenAPI document that describes them.
fn documented() -> (Router<SharedState>, utoipa::openapi::OpenApi) {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .routes(routes!(sources::list_sources))
        .routes(routes!(search::search))
        .routes(routes!(chapters::chapters))
        .routes(routes!(proxy::proxy_image))
        .routes(routes!(download::start_download))
        .routes(routes!(download::status))
        .routes(routes!(download::events))
        .routes(routes!(download::file))
        .routes(routes!(download::cancel))
        .routes(routes!(download::list_tasks))
        .split_for_parts()
}

/// The OpenAPI document as pretty JSON, for `manhwa-server --openapi`. The
/// client generates its TypeScript types from this without running the server.
pub fn openapi_json() -> String {
    documented()
        .1
        .to_pretty_json()
        .expect("OpenAPI document serializes")
}

pub fn router(state: SharedState) -> Router {
    let (modern, api) = documented();

    let cors = if state.config.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::new().allow_origin(Any)
    } else {
        let origins: Vec<HeaderValue> = state
            .config
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new().allow_origin(AllowOrigin::list(origins))
    }
    .allow_methods(Any)
    .allow_headers(Any);

    Router::new()
        .merge(modern)
        .merge(SwaggerUi::new("/docs").url("/openapi.json", api))
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(120),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
