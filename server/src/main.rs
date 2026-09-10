mod api;
mod cache;
mod config;
mod error;
mod model;
mod pipeline;
mod sources;
mod state;
mod tasks;

use std::{net::SocketAddr, sync::Arc};

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    init_tracing();

    let state = Arc::new(AppState::new(config.clone()).await?);
    state.engine.remove_orphans().await;
    tokio::spawn(Arc::clone(&state.engine).sweep_forever());
    let app = api::router(state);

    let addr = SocketAddr::new(config.bind, config.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, version = env!("CARGO_PKG_VERSION"), "manhwa-server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    tracing::info!("shut down cleanly");
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,hyper=warn"));
    let json = std::env::var("LOG_FORMAT").is_ok_and(|v| v.eq_ignore_ascii_case("json"));
    let registry = tracing_subscriber::registry().with(filter);
    if json {
        registry.with(fmt::layer().json()).init();
    } else {
        registry.with(fmt::layer().compact()).init();
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut s) = signal(SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
