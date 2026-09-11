//! Liveness probe for every source. Cached briefly so the client's periodic
//! refresh does not hammer eleven sites.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::future::join_all;
use http::StatusCode;

use crate::{error::AppError, model::SourceStatus, state::SharedState};

const PROBE_TIMEOUT: Duration = Duration::from_secs(6);

pub async fn probe_all(state: &SharedState) -> Arc<HashMap<String, SourceStatus>> {
    state
        .caches
        .status
        .get_with((), async {
            let probes = state.registry.all().iter().map(|s| {
                let fetcher = Arc::clone(&state.fetcher);
                let meta = s.meta().clone();
                async move {
                    let result = fetcher
                        .get(&meta.base_url)
                        .tier(meta.tier)
                        .source(meta.slug)
                        .timeout(PROBE_TIMEOUT)
                        .attempts(1)
                        .send()
                        .await;
                    let status = match result {
                        // A plain 403 still means the site is up (hotlink rules, geo blocks).
                        Ok(r) if r.status.is_success() || r.status == StatusCode::FORBIDDEN => {
                            SourceStatus::Ok
                        }
                        Ok(r) => {
                            tracing::debug!(source = meta.slug, status = %r.status, "probe failed");
                            SourceStatus::Down
                        }
                        Err(AppError::Blocked { .. }) => SourceStatus::Blocked,
                        Err(e) => {
                            tracing::debug!(source = meta.slug, error = %e, "probe failed");
                            SourceStatus::Down
                        }
                    };
                    (meta.slug.to_string(), status)
                }
            });
            Arc::new(join_all(probes).await.into_iter().collect())
        })
        .await
}
