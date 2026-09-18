//! Process-wide shared state handed to every handler.

use std::{sync::Arc, time::Instant};

use reqwest::Client;

use crate::{
    api::ratelimit::RateLimiter,
    cache::Caches,
    config::Config,
    sources::throttle::HostThrottle,
    sources::{
        Ctx, Registry,
        http::{self, Fetcher},
    },
    tasks::TaskEngine,
};

pub struct AppState {
    pub config: Config,
    /// Plain client. Prefer `fetcher`, which knows how to escalate.
    pub client: Client,
    pub fetcher: Arc<Fetcher>,
    pub registry: Registry,
    pub caches: Caches,
    pub engine: Arc<TaskEngine>,
    /// Per-client-IP limit on starting downloads.
    pub download_limiter: RateLimiter,
    pub started_at: Instant,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let registry = Registry::new(&config);
        Self::with_registry(config, registry).await
    }

    pub async fn with_registry(config: Config, registry: Registry) -> anyhow::Result<Self> {
        let client = http::build_client(config.request_timeout)?;
        let impersonating = if config.impersonation_enabled {
            Some(http::build_impersonating_client(config.request_timeout)?)
        } else {
            tracing::warn!("impersonation disabled; Cloudflare-fronted sources will be blocked");
            None
        };
        let throttle = HostThrottle::new(config.host_concurrency, config.host_min_interval);
        let fetcher = Arc::new(Fetcher::new(client.clone(), impersonating, throttle));
        let download_limiter =
            RateLimiter::new(config.download_rate_per_min, config.download_rate_burst);
        let caches = Caches::new(config.cache_ttl, config.status_ttl);
        let engine = Arc::new(
            TaskEngine::new(
                config.download_dir.clone(),
                config.max_concurrent_downloads,
                config.image_concurrency,
            )
            .await?,
        );
        Ok(Self {
            config,
            client,
            fetcher,
            registry,
            caches,
            engine,
            download_limiter,
            started_at: Instant::now(),
        })
    }

    pub fn ctx(&self) -> Ctx {
        Ctx {
            client: self.client.clone(),
            fetcher: Arc::clone(&self.fetcher),
        }
    }
}
