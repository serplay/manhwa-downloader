//! Per-client-IP token bucket for `POST /download`, so one visitor cannot keep
//! the download slots full or get the server's IP banned by a source for
//! everyone else.

use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use axum::http::HeaderMap;
use dashmap::DashMap;

/// Above this many tracked clients, full buckets are dropped on the next check.
const PRUNE_ABOVE: usize = 4096;

struct Bucket {
    tokens: f64,
    updated: Instant,
}

pub struct RateLimiter {
    burst: f64,
    per_sec: f64,
    buckets: DashMap<IpAddr, Bucket>,
}

impl RateLimiter {
    /// `per_min` tokens refilled per minute, up to `burst`. `per_min == 0`
    /// disables the limiter.
    pub fn new(per_min: u32, burst: u32) -> Self {
        Self {
            burst: f64::from(burst.max(1)),
            per_sec: f64::from(per_min) / 60.0,
            buckets: DashMap::new(),
        }
    }

    pub fn enabled(&self) -> bool {
        self.per_sec > 0.0
    }

    /// Take one token for `ip`. `Err` carries how long until one is available.
    pub fn check(&self, ip: IpAddr) -> Result<(), Duration> {
        self.check_at(ip, Instant::now())
    }

    fn check_at(&self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        if !self.enabled() {
            return Ok(());
        }
        if self.buckets.len() > PRUNE_ABOVE {
            self.prune(now);
        }
        let mut b = self.buckets.entry(ip).or_insert(Bucket {
            tokens: self.burst,
            updated: now,
        });
        let elapsed = now.saturating_duration_since(b.updated).as_secs_f64();
        b.tokens = (b.tokens + elapsed * self.per_sec).min(self.burst);
        b.updated = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            Ok(())
        } else {
            Err(Duration::from_secs_f64((1.0 - b.tokens) / self.per_sec))
        }
    }

    /// Forget clients whose bucket has refilled; they are indistinguishable
    /// from new ones.
    fn prune(&self, now: Instant) {
        self.buckets.retain(|_, b| {
            let elapsed = now.saturating_duration_since(b.updated).as_secs_f64();
            b.tokens + elapsed * self.per_sec < self.burst
        });
    }
}

/// The client address, given how many trusted proxies sit in front of us.
///
/// Each proxy appends the address it received the request from to
/// `X-Forwarded-For`, so with `hops` trusted proxies the client is the
/// `hops`-th entry from the right. Entries further left were written by the
/// client and are not trusted. With `hops == 0` the socket peer is the client.
pub fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>, hops: usize) -> Option<IpAddr> {
    if hops == 0 {
        return peer.map(|p| p.ip());
    }
    let chain: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if chain.is_empty() {
        return peer.map(|p| p.ip());
    }
    // A shorter chain than configured means a hop did not append; the leftmost
    // entry is the best guess left.
    let idx = chain.len().saturating_sub(hops);
    Some(chain[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn allows_a_burst_then_refills() {
        let l = RateLimiter::new(6, 3); // one token every 10s
        let t0 = Instant::now();
        let a = ip("203.0.113.1");
        for _ in 0..3 {
            assert!(l.check_at(a, t0).is_ok());
        }
        let wait = l.check_at(a, t0).unwrap_err();
        assert!((9.9..=10.0).contains(&wait.as_secs_f64()), "{wait:?}");
        // Another client has its own bucket.
        assert!(l.check_at(ip("203.0.113.2"), t0).is_ok());
        assert!(l.check_at(a, t0 + Duration::from_secs(5)).is_err());
        assert!(l.check_at(a, t0 + Duration::from_secs(11)).is_ok());
        // The bucket never refills past the burst.
        let later = t0 + Duration::from_secs(3600);
        for _ in 0..3 {
            assert!(l.check_at(a, later).is_ok());
        }
        assert!(l.check_at(a, later).is_err());
    }

    #[test]
    fn zero_rate_disables() {
        let l = RateLimiter::new(0, 1);
        let t0 = Instant::now();
        for _ in 0..100 {
            assert!(l.check_at(ip("203.0.113.1"), t0).is_ok());
        }
    }

    #[test]
    fn prunes_refilled_buckets() {
        let l = RateLimiter::new(60, 2);
        let t0 = Instant::now();
        for i in 0..=PRUNE_ABOVE as u32 {
            let _ = l.check_at(IpAddr::from(i.to_be_bytes()), t0);
        }
        let _ = l.check_at(ip("198.51.100.1"), t0 + Duration::from_secs(60));
        assert_eq!(l.buckets.len(), 1);
    }

    #[test]
    fn client_ip_from_forwarded_chain() {
        let peer: SocketAddr = "10.0.0.9:5000".parse().unwrap();
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            "1.1.1.1, 203.0.113.7, 10.0.0.2".parse().unwrap(),
        );
        assert_eq!(client_ip(&h, Some(peer), 0), Some(ip("10.0.0.9")));
        assert_eq!(client_ip(&h, Some(peer), 1), Some(ip("10.0.0.2")));
        assert_eq!(client_ip(&h, Some(peer), 2), Some(ip("203.0.113.7")));
        // The spoofed leftmost entry is only used when the chain runs out.
        assert_eq!(client_ip(&h, Some(peer), 5), Some(ip("1.1.1.1")));
        assert_eq!(
            client_ip(&HeaderMap::new(), Some(peer), 2),
            Some(ip("10.0.0.9"))
        );
        assert_eq!(client_ip(&HeaderMap::new(), None, 0), None);
    }
}
