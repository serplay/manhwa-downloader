//! Shared HTTP plumbing for sources: a browser-like `reqwest` client for
//! ordinary sites, a Chrome-impersonating `wreq` client for Cloudflare-fronted
//! ones, escalation between the two, retries, and challenge detection.
//!
//! Sources ask the [`Fetcher`] for a URL with the tier their `SourceMeta`
//! declares. `Plain` goes through `reqwest` and is retried through the
//! impersonating client if Cloudflare answers with a challenge. `Impersonate`
//! and `Browser` go straight to the impersonating client (a headless browser is
//! not built in Phase 3; see `MODERNIZATION_PLAN.md`, decision D5).

use std::{io, time::Duration};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt, stream::BoxStream};
pub use http::header;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use reqwest::{Client, RequestBuilder, Response};
use tokio::sync::OwnedSemaphorePermit;
use wreq_util::Emulation;

use super::throttle::{Event, HostStats, HostThrottle, host_of};
use crate::{
    error::{AppError, AppResult},
    model::FetchTier,
};

/// Longest `Retry-After` we will sit out inside one request. Anything longer is
/// handed back to the caller as the response it is.
pub const MAX_RETRY_WAIT: Duration = Duration::from_secs(30);

pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";

const HTML_ACCEPT: &str =
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8";
pub const IMAGE_ACCEPT: &str = "image/avif,image/webp,image/apng,image/*,*/*;q=0.8";

/// Build the shared plain client. One client for the whole process: connection
/// pool, cookie jar, and compression are all reused across sources.
pub fn build_client(timeout: Duration) -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCEPT_LANGUAGE,
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert(header::ACCEPT, HeaderValue::from_static(HTML_ACCEPT));
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .cookie_store(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .gzip(true)
        .brotli(true)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?)
}

/// Build the impersonating client. Emulates a current Chrome's TLS ClientHello,
/// HTTP/2 settings and header order, which is what Cloudflare's passive checks
/// key on. Cookies (including `cf_clearance`) persist for the process lifetime.
pub fn build_impersonating_client(timeout: Duration) -> anyhow::Result<wreq::Client> {
    Ok(wreq::Client::builder()
        .emulation(Emulation::Chrome149)
        .cookie_store(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .redirect(wreq::redirect::Policy::limited(5))
        .build()?)
}

fn looks_like_challenge(status: StatusCode, headers: &HeaderMap) -> bool {
    let mitigated = headers
        .get("cf-mitigated")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("challenge"));
    mitigated
        || ((status == StatusCode::FORBIDDEN || status == StatusCode::SERVICE_UNAVAILABLE)
            && headers
                .get(header::SERVER)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.to_ascii_lowercase().contains("cloudflare")))
}

/// `Retry-After` in its delta-seconds form. The HTTP-date form is rare from the
/// hosts we talk to and falls back to our own backoff.
pub fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    headers
        .get(header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

/// A status that says "slow down" rather than "broken".
fn is_rate_limit(status: StatusCode, headers: &HeaderMap) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || (status == StatusCode::SERVICE_UNAVAILABLE && headers.contains_key(header::RETRY_AFTER))
}

/// True when the response looks like a Cloudflare interstitial rather than the
/// real page.
pub fn is_cloudflare_challenge(resp: &Response) -> bool {
    looks_like_challenge(resp.status(), resp.headers())
}

// ---- unified response ------------------------------------------------------

/// A response from either client, normalized so callers never see which one
/// answered.
pub struct Fetched {
    pub status: StatusCode,
    pub headers: HeaderMap,
    body: BoxStream<'static, io::Result<Bytes>>,
    /// The host slot this request holds. Released when the body is read or
    /// the response is dropped, so a slow body counts against the host's cap.
    slot: OwnedSemaphorePermit,
}

impl Fetched {
    fn from_reqwest(resp: Response, slot: OwnedSemaphorePermit) -> Self {
        Self {
            status: resp.status(),
            headers: resp.headers().clone(),
            body: resp.bytes_stream().map_err(io::Error::other).boxed(),
            slot,
        }
    }

    fn from_wreq(resp: wreq::Response, slot: OwnedSemaphorePermit) -> Self {
        Self {
            status: resp.status(),
            headers: resp.headers().clone(),
            body: resp.bytes_stream().map_err(io::Error::other).boxed(),
            slot,
        }
    }

    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
    }

    /// The body as a stream. The host slot travels with it and is released
    /// when the stream is dropped.
    pub fn into_stream(self) -> BoxStream<'static, io::Result<Bytes>> {
        let slot = self.slot;
        self.body
            .map(move |chunk| {
                let _held = &slot;
                chunk
            })
            .boxed()
    }

    pub async fn bytes(self, source: &str) -> AppResult<Bytes> {
        let mut out = Vec::new();
        let mut body = self.into_stream();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| AppError::Upstream {
                site: Some(source.to_string()),
                message: format!("{source} connection dropped mid-body: {e}"),
            })?;
            out.extend_from_slice(&chunk);
        }
        Ok(Bytes::from(out))
    }

    pub async fn text(self, source: &str) -> AppResult<String> {
        let bytes = self.bytes(source).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

// ---- fetcher ----------------------------------------------------------------

pub struct Fetcher {
    plain: Client,
    impersonate: Option<wreq::Client>,
    throttle: HostThrottle,
}

impl Fetcher {
    pub fn new(plain: Client, impersonate: Option<wreq::Client>, throttle: HostThrottle) -> Self {
        Self {
            plain,
            impersonate,
            throttle,
        }
    }

    /// Per-host request, challenge and rate-limit counters.
    pub fn host_stats(&self) -> Vec<HostStats> {
        self.throttle.stats()
    }

    pub fn can_impersonate(&self) -> bool {
        self.impersonate.is_some()
    }

    pub fn get(&self, url: impl Into<String>) -> FetchBuilder<'_> {
        self.request(Method::GET, url)
    }

    pub fn post(&self, url: impl Into<String>) -> FetchBuilder<'_> {
        self.request(Method::POST, url)
    }

    pub fn request(&self, method: Method, url: impl Into<String>) -> FetchBuilder<'_> {
        let url = url.into();
        FetchBuilder {
            fetcher: self,
            method,
            host: host_of(&url),
            url,
            tier: FetchTier::Plain,
            source: "upstream".into(),
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
            attempts: 3,
        }
    }

    /// Which clients to try, in order, for a declared tier.
    fn ladder(&self, tier: FetchTier) -> Vec<Client2<'_>> {
        match (tier, self.impersonate.as_ref()) {
            (FetchTier::Plain, Some(imp)) => vec![Client2::Plain(&self.plain), Client2::Imp(imp)],
            (FetchTier::Plain, None) => vec![Client2::Plain(&self.plain)],
            (FetchTier::Impersonate | FetchTier::Browser, Some(imp)) => vec![Client2::Imp(imp)],
            (FetchTier::Impersonate | FetchTier::Browser, None) => {
                vec![Client2::Plain(&self.plain)]
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Client2<'a> {
    Plain(&'a Client),
    Imp(&'a wreq::Client),
}

pub struct FetchBuilder<'a> {
    fetcher: &'a Fetcher,
    method: Method,
    url: String,
    host: String,
    tier: FetchTier,
    source: String,
    headers: HeaderMap,
    body: Option<(HeaderValue, Bytes)>,
    timeout: Option<Duration>,
    attempts: u32,
}

impl FetchBuilder<'_> {
    pub fn tier(mut self, tier: FetchTier) -> Self {
        self.tier = tier;
        self
    }

    /// Name used in error messages and logs.
    pub fn source(mut self, source: &str) -> Self {
        self.source = source.to_string();
        self
    }

    pub fn header(mut self, name: header::HeaderName, value: &str) -> Self {
        if let Ok(v) = HeaderValue::from_str(value) {
            self.headers.insert(name, v);
        }
        self
    }

    pub fn referer(self, referer: &str) -> Self {
        self.header(header::REFERER, referer)
    }

    pub fn accept(self, accept: &str) -> Self {
        self.header(header::ACCEPT, accept)
    }

    pub fn accept_images(self) -> Self {
        self.accept(IMAGE_ACCEPT)
    }

    /// Mark the request as an XHR the way jQuery/HTMX would.
    pub fn xhr(self) -> Self {
        self.header(
            header::HeaderName::from_static("x-requested-with"),
            "XMLHttpRequest",
        )
    }

    pub fn htmx(self) -> Self {
        self.header(header::HeaderName::from_static("hx-request"), "true")
    }

    pub fn form(mut self, body: &str) -> Self {
        self.body = Some((
            HeaderValue::from_static("application/x-www-form-urlencoded; charset=UTF-8"),
            Bytes::from(body.to_string()),
        ));
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    /// Send, walking the tier ladder on Cloudflare challenges and retrying
    /// transient failures at each rung. Non-2xx statuses are returned, not
    /// errors; use [`FetchBuilder::send_ok`] to insist on success.
    pub async fn send(self) -> AppResult<Fetched> {
        let ladder = self.fetcher.ladder(self.tier);
        let last = ladder.len() - 1;
        for (i, client) in ladder.into_iter().enumerate() {
            let fetched = self.send_with_retry(client).await?;
            if looks_like_challenge(fetched.status, &fetched.headers) {
                self.fetcher.throttle.record(&self.host, Event::Challenged);
                if i < last {
                    tracing::info!(source = %self.source, url = %self.url, "cloudflare challenge; escalating to impersonation");
                    continue;
                }
                self.fetcher.throttle.record(&self.host, Event::Blocked);
                return Err(AppError::Blocked {
                    site: self.source.clone(),
                });
            }
            return Ok(fetched);
        }
        unreachable!("ladder is never empty")
    }

    /// Like `send`, but maps any non-2xx status to an upstream error.
    pub async fn send_ok(self) -> AppResult<Fetched> {
        let source = self.source.clone();
        let fetched = self.send().await?;
        if fetched.status.is_success() {
            Ok(fetched)
        } else {
            Err(AppError::Upstream {
                site: Some(source.clone()),
                message: format!("{source} responded with HTTP {}", fetched.status),
            })
        }
    }

    /// `send_ok` followed by reading the body as text.
    pub async fn text(self) -> AppResult<String> {
        let source = self.source.clone();
        self.send_ok().await?.text(&source).await
    }

    /// `send_ok` followed by reading the body into memory.
    pub async fn bytes(self) -> AppResult<Bytes> {
        let source = self.source.clone();
        self.send_ok().await?.bytes(&source).await
    }

    async fn send_with_retry(&self, client: Client2<'_>) -> AppResult<Fetched> {
        let mut delay = Duration::from_millis(300);
        let mut last_err: Option<AppError> = None;
        for attempt in 0..self.attempts {
            let slot = self.fetcher.throttle.acquire(&self.host).await;
            let result = match client {
                Client2::Plain(c) => self.send_plain(c, slot).await,
                Client2::Imp(c) => self.send_impersonating(c, slot).await,
            };
            let mut wait = delay;
            match result {
                Ok(fetched) => {
                    let status = fetched.status;
                    let asked = retry_after(&fetched.headers);
                    if is_rate_limit(status, &fetched.headers) {
                        self.fetcher.throttle.record(&self.host, Event::RateLimited);
                        // Every request to this host waits, not just this one.
                        self.fetcher
                            .throttle
                            .back_off(&self.host, asked.unwrap_or(delay));
                    }
                    let retryable =
                        status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                    let too_long = asked.is_some_and(|a| a > MAX_RETRY_WAIT);
                    if retryable
                        && !too_long
                        && attempt + 1 < self.attempts
                        && !looks_like_challenge(status, &fetched.headers)
                    {
                        wait = asked.unwrap_or(delay);
                        tracing::debug!(source = %self.source, %status, attempt, ?wait, "retryable upstream status");
                        last_err = Some(AppError::Upstream {
                            site: Some(self.source.clone()),
                            message: format!("{} responded with HTTP {status}", self.source),
                        });
                    } else {
                        return Ok(fetched);
                    }
                }
                Err(err) => {
                    let retryable = matches!(
                        err,
                        AppError::UpstreamTimeout { .. } | AppError::Upstream { .. }
                    );
                    if !retryable || attempt + 1 >= self.attempts {
                        return Err(err);
                    }
                    tracing::debug!(source = %self.source, attempt, error = %err, "retrying upstream request");
                    last_err = Some(err);
                }
            }
            tokio::time::sleep(wait).await;
            delay = (delay * 2).min(Duration::from_secs(3));
        }
        Err(last_err.unwrap_or_else(|| AppError::Upstream {
            site: Some(self.source.clone()),
            message: format!("request to {} failed", self.source),
        }))
    }

    async fn send_plain(&self, client: &Client, slot: OwnedSemaphorePermit) -> AppResult<Fetched> {
        let mut req = client
            .request(self.method.clone(), &self.url)
            .headers(self.headers.clone());
        if let Some((ct, body)) = &self.body {
            req = req
                .header(header::CONTENT_TYPE, ct.clone())
                .body(body.clone());
        }
        if let Some(t) = self.timeout {
            req = req.timeout(t);
        }
        req.send()
            .await
            .map(|r| Fetched::from_reqwest(r, slot))
            .map_err(|e| AppError::from_reqwest(&self.source, e))
    }

    async fn send_impersonating(
        &self,
        client: &wreq::Client,
        slot: OwnedSemaphorePermit,
    ) -> AppResult<Fetched> {
        let mut req = client
            .request(self.method.clone(), &self.url)
            .headers(self.headers.clone());
        if let Some((ct, body)) = &self.body {
            req = req
                .header(header::CONTENT_TYPE, ct.clone())
                .body(body.clone());
        }
        if let Some(t) = self.timeout {
            req = req.timeout(t);
        }
        req.send()
            .await
            .map(|r| Fetched::from_wreq(r, slot))
            .map_err(|e| AppError::from_wreq(&self.source, e))
    }
}

// ---- legacy reqwest helpers (MangaDex, Bato) --------------------------------

/// Send a request, retrying on connect errors, timeouts, 429 and 5xx with
/// exponential backoff. `source` is only used for error messages.
pub async fn send_with_retry(
    source: &str,
    build: impl Fn() -> RequestBuilder,
    attempts: u32,
) -> AppResult<Response> {
    let mut delay = Duration::from_millis(300);
    let mut last_err: Option<AppError> = None;
    for attempt in 0..attempts.max(1) {
        let mut wait = delay;
        match build().send().await {
            Ok(resp) => {
                let status = resp.status();
                let asked = retry_after(resp.headers());
                let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                let too_long = asked.is_some_and(|a| a > MAX_RETRY_WAIT);
                if retryable && !too_long && attempt + 1 < attempts {
                    wait = asked.unwrap_or(delay);
                    tracing::debug!(%source, %status, attempt, ?wait, "retryable upstream status");
                    last_err = Some(AppError::Upstream {
                        site: Some(source.to_string()),
                        message: format!("{source} responded with HTTP {status}"),
                    });
                } else if is_cloudflare_challenge(&resp) {
                    return Err(AppError::Blocked {
                        site: source.to_string(),
                    });
                } else {
                    return Ok(resp);
                }
            }
            Err(err) => {
                let mapped = AppError::from_reqwest(source, err);
                let retryable = matches!(
                    mapped,
                    AppError::UpstreamTimeout { .. } | AppError::Upstream { .. }
                );
                if !retryable || attempt + 1 >= attempts {
                    return Err(mapped);
                }
                tracing::debug!(%source, attempt, error = %mapped, "retrying upstream request");
                last_err = Some(mapped);
            }
        }
        tokio::time::sleep(wait).await;
        delay = (delay * 2).min(Duration::from_secs(3));
    }
    Err(last_err.unwrap_or_else(|| AppError::Upstream {
        site: Some(source.to_string()),
        message: format!("request to {source} failed"),
    }))
}

/// Turn a non-success status into an upstream error, keeping the body out of it.
pub fn ensure_success(source: &str, resp: Response) -> AppResult<Response> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else {
        Err(AppError::Upstream {
            site: Some(source.to_string()),
            message: format!("{source} responded with HTTP {status}"),
        })
    }
}

/// Append query parameters to a URL, percent-encoding as a browser form would.
pub fn with_query(base: &str, params: &[(&str, &str)]) -> String {
    match url::Url::parse_with_params(base, params) {
        Ok(u) => u.to_string(),
        Err(_) => base.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_detection() {
        let mut h = HeaderMap::new();
        assert!(!looks_like_challenge(StatusCode::OK, &h));
        h.insert(header::SERVER, HeaderValue::from_static("cloudflare"));
        assert!(looks_like_challenge(StatusCode::FORBIDDEN, &h));
        assert!(!looks_like_challenge(StatusCode::OK, &h));
        let mut h = HeaderMap::new();
        h.insert("cf-mitigated", HeaderValue::from_static("challenge"));
        assert!(looks_like_challenge(StatusCode::OK, &h));
    }

    mod fetcher {
        use std::time::Instant;

        use wiremock::{
            Mock, MockServer, ResponseTemplate,
            matchers::{method, path},
        };

        use super::super::*;

        fn fetcher() -> Fetcher {
            let client = build_client(Duration::from_secs(5)).unwrap();
            Fetcher::new(client, None, HostThrottle::new(4, Duration::ZERO))
        }

        async fn stats_for(f: &Fetcher, server: &MockServer) -> HostStats {
            let host = host_of(&server.uri());
            f.host_stats().into_iter().find(|s| s.host == host).unwrap()
        }

        #[tokio::test]
        async fn waits_out_a_short_retry_after_then_succeeds() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/page"))
                .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
                .up_to_n_times(1)
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/page"))
                .respond_with(ResponseTemplate::new(200).set_body_string("hello"))
                .mount(&server)
                .await;

            let f = fetcher();
            let begin = Instant::now();
            let body = f
                .get(format!("{}/page", server.uri()))
                .text()
                .await
                .unwrap();
            assert_eq!(body, "hello");
            assert!(
                begin.elapsed() >= Duration::from_secs(1),
                "did not honour Retry-After"
            );
            let stats = stats_for(&f, &server).await;
            assert_eq!((stats.requests, stats.rate_limited), (2, 1));
        }

        #[tokio::test]
        async fn gives_up_at_once_on_a_long_retry_after() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "3600"))
                .expect(1)
                .mount(&server)
                .await;

            let f = fetcher();
            let begin = Instant::now();
            let err = f
                .get(format!("{}/page", server.uri()))
                .text()
                .await
                .unwrap_err();
            assert!(err.to_string().contains("503"), "{err}");
            assert!(begin.elapsed() < Duration::from_secs(2));
            assert_eq!(stats_for(&f, &server).await.rate_limited, 1);
        }

        #[tokio::test]
        async fn counts_an_unpassable_challenge_as_blocked() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(403)
                        .insert_header("cf-mitigated", "challenge")
                        .insert_header("server", "cloudflare"),
                )
                .mount(&server)
                .await;

            let f = fetcher();
            let err = f
                .get(format!("{}/", server.uri()))
                .tier(FetchTier::Browser)
                .send()
                .await
                .err()
                .unwrap();
            assert!(matches!(err, AppError::Blocked { .. }), "{err}");
            let stats = stats_for(&f, &server).await;
            assert_eq!((stats.challenged, stats.blocked), (1, 1));
        }

        #[tokio::test]
        async fn a_held_body_keeps_its_host_slot() {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(200).set_body_string("x"))
                .mount(&server)
                .await;
            let client = build_client(Duration::from_secs(5)).unwrap();
            let f = Fetcher::new(client, None, HostThrottle::new(1, Duration::ZERO));
            let url = format!("{}/a", server.uri());

            let held = f.get(&url).send().await.unwrap();
            let second = tokio::time::timeout(Duration::from_millis(300), f.get(&url).send()).await;
            assert!(
                second.is_err(),
                "second request ran while the first body was unread"
            );
            drop(held);
            f.get(&url).text().await.unwrap();
        }
    }

    #[test]
    fn query_encoding() {
        assert_eq!(
            with_query("https://x.test/search", &[("q", "solo leveling&more")]),
            "https://x.test/search?q=solo+leveling%26more"
        );
    }
}
