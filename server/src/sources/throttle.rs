//! Per-host politeness: a cap on parallel requests, a minimum spacing between
//! request starts with a little jitter, a shared back-off window when a host
//! says "slow down", and counters for what each host has been answering.
//!
//! Every request the [`Fetcher`](super::http::Fetcher) makes goes through here,
//! so several download tasks, the cover proxy and the status probe share one
//! budget per host instead of each opening their own connections.

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use dashmap::DashMap;
use serde::Serialize;
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore},
    time::Instant,
};
use utoipa::ToSchema;

/// Longest back-off a single `Retry-After` may impose on a host.
pub const MAX_BACK_OFF: Duration = Duration::from_secs(120);

#[derive(Default)]
struct Counters {
    requests: AtomicU64,
    challenged: AtomicU64,
    blocked: AtomicU64,
    rate_limited: AtomicU64,
}

struct Host {
    slots: Arc<Semaphore>,
    /// Earliest instant the next request may start.
    next_start: Mutex<Instant>,
    counters: Counters,
}

/// What happened to a request, for the per-host counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Cloudflare answered with a challenge (whether or not a later rung passed it).
    Challenged,
    /// No client could get past the challenge.
    Blocked,
    /// 429, or 503 with `Retry-After`.
    RateLimited,
}

/// Snapshot of one host's counters since the process started.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct HostStats {
    pub host: String,
    pub requests: u64,
    pub challenged: u64,
    pub blocked: u64,
    pub rate_limited: u64,
}

pub struct HostThrottle {
    per_host: usize,
    min_interval: Duration,
    hosts: DashMap<String, Arc<Host>>,
}

impl HostThrottle {
    /// `per_host` parallel requests per host (0 means 1), starts spaced at least
    /// `min_interval` apart plus up to half that again as jitter.
    pub fn new(per_host: usize, min_interval: Duration) -> Self {
        Self {
            per_host: per_host.max(1),
            min_interval,
            hosts: DashMap::new(),
        }
    }

    fn host(&self, host: &str) -> Arc<Host> {
        if let Some(h) = self.hosts.get(host) {
            return Arc::clone(&h);
        }
        Arc::clone(&self.hosts.entry(host.to_string()).or_insert_with(|| {
            Arc::new(Host {
                slots: Arc::new(Semaphore::new(self.per_host)),
                next_start: Mutex::new(Instant::now()),
                counters: Counters::default(),
            })
        }))
    }

    /// Wait for a slot and for this request's turn. The permit must be held
    /// until the response body has been read or dropped.
    pub async fn acquire(&self, host: &str) -> OwnedSemaphorePermit {
        let h = self.host(host);
        let permit = Arc::clone(&h.slots)
            .acquire_owned()
            .await
            .expect("host semaphore is never closed");
        let start = {
            let mut next = h.next_start.lock().unwrap();
            let start = (*next).max(Instant::now());
            *next = start + self.min_interval + jitter(self.min_interval / 2);
            start
        };
        tokio::time::sleep_until(start).await;
        h.counters.requests.fetch_add(1, Ordering::Relaxed);
        permit
    }

    /// Hold every new request to `host` back for `wait` (capped at [`MAX_BACK_OFF`]).
    pub fn back_off(&self, host: &str, wait: Duration) {
        let h = self.host(host);
        let until = Instant::now() + wait.min(MAX_BACK_OFF);
        let mut next = h.next_start.lock().unwrap();
        if *next < until {
            *next = until;
        }
    }

    pub fn record(&self, host: &str, event: Event) {
        let h = self.host(host);
        let counter = match event {
            Event::Challenged => &h.counters.challenged,
            Event::Blocked => &h.counters.blocked,
            Event::RateLimited => &h.counters.rate_limited,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Counters for every host seen so far, busiest first.
    pub fn stats(&self) -> Vec<HostStats> {
        let mut v: Vec<HostStats> = self
            .hosts
            .iter()
            .map(|e| {
                let c = &e.counters;
                HostStats {
                    host: e.key().clone(),
                    requests: c.requests.load(Ordering::Relaxed),
                    challenged: c.challenged.load(Ordering::Relaxed),
                    blocked: c.blocked.load(Ordering::Relaxed),
                    rate_limited: c.rate_limited.load(Ordering::Relaxed),
                }
            })
            .collect();
        v.sort_by(|a, b| b.requests.cmp(&a.requests).then(a.host.cmp(&b.host)));
        v
    }
}

/// Uniform-ish random duration in `[0, max)`. Not cryptographic; it only has to
/// keep request starts from landing on an exact beat.
fn jitter(max: Duration) -> Duration {
    let nanos = max.as_nanos();
    if nanos == 0 {
        return Duration::ZERO;
    }
    let r = uuid::Uuid::new_v4().as_u128() % nanos;
    Duration::from_nanos(r as u64)
}

/// The host part of a URL, lowercased. Falls back to the whole string so a
/// malformed URL still gets a bucket of its own.
pub fn host_of(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
        .unwrap_or_else(|| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn spaces_request_starts() {
        let t = HostThrottle::new(8, Duration::from_millis(100));
        let begin = Instant::now();
        for _ in 0..4 {
            drop(t.acquire("a.test").await);
        }
        let elapsed = begin.elapsed();
        // Three gaps of 100ms, each with up to 50ms of jitter.
        assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
        assert!(elapsed < Duration::from_millis(450), "{elapsed:?}");
        // Another host is not held back by the first one.
        let other = Instant::now();
        drop(t.acquire("b.test").await);
        assert_eq!(other.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn caps_parallel_requests_per_host() {
        let t = Arc::new(HostThrottle::new(2, Duration::ZERO));
        let a = t.acquire("a.test").await;
        let _b = t.acquire("a.test").await;
        let waiter = {
            let t = Arc::clone(&t);
            tokio::spawn(async move { t.acquire("a.test").await })
        };
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(!waiter.is_finished(), "a third request got in past the cap");
        drop(a);
        drop(waiter.await.unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn back_off_delays_the_whole_host_and_is_capped() {
        let t = HostThrottle::new(4, Duration::ZERO);
        t.back_off("a.test", Duration::from_secs(10));
        let begin = Instant::now();
        drop(t.acquire("a.test").await);
        assert_eq!(begin.elapsed(), Duration::from_secs(10));

        t.back_off("a.test", Duration::from_secs(3600));
        let begin = Instant::now();
        drop(t.acquire("a.test").await);
        assert_eq!(begin.elapsed(), MAX_BACK_OFF);
    }

    #[tokio::test]
    async fn counts_events_per_host() {
        let t = HostThrottle::new(4, Duration::ZERO);
        drop(t.acquire("a.test").await);
        drop(t.acquire("a.test").await);
        drop(t.acquire("b.test").await);
        t.record("a.test", Event::Challenged);
        t.record("a.test", Event::Blocked);
        t.record("b.test", Event::RateLimited);
        let stats = t.stats();
        assert_eq!(
            stats[0],
            HostStats {
                host: "a.test".into(),
                requests: 2,
                challenged: 1,
                blocked: 1,
                rate_limited: 0
            }
        );
        assert_eq!(stats[1].host, "b.test");
        assert_eq!(stats[1].rate_limited, 1);
    }

    #[test]
    fn host_extraction() {
        assert_eq!(
            host_of("https://CDN.Example.com:8443/a.png"),
            "cdn.example.com"
        );
        assert_eq!(host_of("not a url"), "not a url");
    }
}
