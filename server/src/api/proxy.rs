//! Cover image proxy. Sources block hotlinking or need a `Referer`; the client
//! never talks to them directly.

use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

use crate::{
    error::{AppError, AppResult},
    model::{Cover, FetchTier},
    state::SharedState,
};

/// Hostname suffixes covers are known to live on, beyond each source's own domain.
const KNOWN_IMAGE_HOSTS: &[&str] = &[
    "mangadex.org",
    "mangadex.network",
    "bato.si",
    "bato.to",
    "batcg.org",
    "batocc.org",
    "batotwo.com",
    "mangapill.com",
    "mangahere.cc",
    "fmcdn.mangahere.com",
    "weebcentral.com",
    "compsci88.com",
    "asuracomic.net",
    "asurascans.com",
    "readdetectiveconan.com",
    "mangahere.org",
    "lowee.us",
    "ravenscans.org",
    "tnlycdn.com",
    "kunmanga.com",
    "toonily.com",
    "toongod.org",
    "manhuaus.com",
    "yakshascans.com",
    "wp.com",
];

#[derive(Deserialize, IntoParams)]
pub struct ProxyParams {
    /// Absolute image URL.
    pub url: String,
    /// Referer the host expects.
    pub referer: Option<String>,
    /// Legacy alias for `referer`.
    pub hd: Option<String>,
}

/// Build the proxied URL the client should put in an `<img src>`.
pub fn proxy_url(base_path: &str, cover: &Cover) -> String {
    let mut out = format!(
        "{base_path}/proxy-image?url={}",
        utf8_percent_encode(&cover.url, NON_ALPHANUMERIC)
    );
    if let Some(r) = &cover.referer {
        out.push_str("&referer=");
        out.push_str(&utf8_percent_encode(r, NON_ALPHANUMERIC).to_string());
    }
    out
}

pub fn host_allowed(host: &str, extra: &[String], source_hosts: &[String]) -> bool {
    let host = host.to_ascii_lowercase();
    let suffix_match = |allowed: &str| {
        let allowed = allowed.to_ascii_lowercase();
        host == allowed || host.ends_with(&format!(".{allowed}"))
    };
    KNOWN_IMAGE_HOSTS.iter().any(|h| suffix_match(h))
        || extra.iter().any(|h| suffix_match(h))
        || source_hosts.iter().any(|h| suffix_match(h))
}

/// Fetch tier for an image: that of the source whose site the image or its
/// referer belongs to, so Cloudflare-fronted CDNs get the impersonating client.
fn tier_for(state: &SharedState, target: &Url, referer: Option<&str>) -> FetchTier {
    let hosts: Vec<String> = [Some(target.host_str().unwrap_or("")), referer]
        .into_iter()
        .flatten()
        .filter_map(|h| {
            if h.contains("://") {
                Url::parse(h).ok()?.host_str().map(str::to_string)
            } else {
                Some(h.to_string())
            }
        })
        .map(|h| h.to_ascii_lowercase())
        .collect();
    let mut tier = FetchTier::Plain;
    for s in state.registry.all() {
        let Some(site) = Url::parse(&s.meta().base_url).ok().and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        }) else {
            continue;
        };
        if hosts
            .iter()
            .any(|h| *h == site || h.ends_with(&format!(".{site}")))
        {
            tier = match (tier, s.meta().tier) {
                (FetchTier::Plain, t) => t,
                (t, _) => t,
            };
        }
    }
    tier
}

fn validate_target(state: &SharedState, raw: &str) -> AppResult<Url> {
    let url = Url::parse(raw).map_err(|_| AppError::Validation("invalid image url".into()))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::Validation(
            "image url must be http or https".into(),
        ));
    }
    let host = match url.host() {
        Some(url::Host::Domain(d)) => d.to_string(),
        _ => return Err(AppError::Validation("image url must use a hostname".into())),
    };
    let source_hosts: Vec<String> = state
        .registry
        .all()
        .iter()
        .filter_map(|s| Url::parse(&s.meta().base_url).ok())
        .filter_map(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
        .collect();
    if !host_allowed(&host, &state.config.proxy_extra_hosts, &source_hosts) {
        tracing::warn!(%host, "image proxy refused host; add it to PROXY_EXTRA_HOSTS if legitimate");
        return Err(AppError::Validation(format!(
            "image host `{host}` is not allowed"
        )));
    }
    Ok(url)
}

#[utoipa::path(get, path = "/proxy-image", tag = "images", params(ProxyParams),
    responses(
        (status = 200, description = "The image bytes", content_type = "image/*"),
        (status = 400, body = crate::error::ErrorBody),
        (status = 502, body = crate::error::ErrorBody)))]
pub async fn proxy_image(
    State(state): State<SharedState>,
    Query(params): Query<ProxyParams>,
) -> AppResult<Response> {
    let target = validate_target(&state, &params.url)?;
    let referer = params.referer.or(params.hd).filter(|r| !r.is_empty());

    let tier = tier_for(&state, &target, referer.as_deref());
    let mut req = state
        .fetcher
        .get(target.as_str())
        .tier(tier)
        .source("image host")
        .accept_images()
        .attempts(2);
    if let Some(r) = &referer {
        req = req.referer(r);
    }
    let upstream = req.send().await?;
    if !upstream.status.is_success() {
        return Err(AppError::Upstream {
            site: None,
            message: format!("image host responded with HTTP {}", upstream.status),
        });
    }
    let content_type = upstream.content_type().unwrap_or("image/jpeg").to_string();
    if !content_type.starts_with("image/") {
        return Err(AppError::Upstream {
            site: None,
            message: "upstream did not return an image".into(),
        });
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&content_type).unwrap_or(HeaderValue::from_static("image/jpeg")),
    );
    if let Some(len) = upstream.headers.get(header::CONTENT_LENGTH) {
        headers.insert(header::CONTENT_LENGTH, len.clone());
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400"),
    );
    let body = Body::from_stream(upstream.into_stream());
    Ok((StatusCode::OK, headers, body).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_matches_suffixes_only() {
        let extra = vec!["cdn.example".to_string()];
        let sources = vec!["mangapill.com".to_string()];
        assert!(host_allowed("uploads.mangadex.org", &extra, &sources));
        assert!(host_allowed("img.cdn.example", &extra, &sources));
        assert!(host_allowed("mangapill.com", &extra, &sources));
        assert!(!host_allowed("evilmangadex.org", &extra, &sources));
        assert!(!host_allowed("localhost", &extra, &sources));
    }

    #[test]
    fn proxy_url_encodes_parts() {
        let cover = Cover {
            url: "https://a.b/c d.jpg?x=1&y=2".into(),
            referer: Some("https://a.b/".into()),
        };
        let url = proxy_url("/api", &cover);
        assert!(
            url.starts_with(
                "/api/proxy-image?url=https%3A%2F%2Fa%2Eb%2Fc%20d%2Ejpg%3Fx%3D1%26y%3D2"
            )
        );
        assert!(url.ends_with("&referer=https%3A%2F%2Fa%2Eb%2F"));
    }
}
