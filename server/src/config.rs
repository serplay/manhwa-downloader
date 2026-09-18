//! Runtime configuration, read once from the environment at startup.

use std::{env, net::IpAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub bind: IpAddr,
    pub port: u16,
    pub download_dir: PathBuf,
    pub max_concurrent_downloads: usize,
    pub image_concurrency: usize,
    pub cache_ttl: Duration,
    pub status_ttl: Duration,
    pub request_timeout: Duration,
    pub cors_origins: Vec<String>,
    /// Extra hostname suffixes the image proxy may fetch from.
    pub proxy_extra_hosts: Vec<String>,
    pub browser_enabled: bool,
    /// Use the Chrome-impersonating client for Cloudflare-fronted sources.
    pub impersonation_enabled: bool,
    pub bato_base_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        Ok(Self {
            bind: var_or("BIND", "0.0.0.0").parse()?,
            port: var_or("PORT", "8000").parse()?,
            download_dir: PathBuf::from(var_or("DOWNLOAD_DIR", "./Downloads")),
            max_concurrent_downloads: var_or("MAX_CONCURRENT_DOWNLOADS", "2").parse()?,
            image_concurrency: var_or("IMAGE_CONCURRENCY", "6").parse()?,
            cache_ttl: Duration::from_secs(var_or("CACHE_TTL_SECS", "43200").parse()?),
            status_ttl: Duration::from_secs(var_or("STATUS_TTL_SECS", "60").parse()?),
            request_timeout: Duration::from_secs(var_or("REQUEST_TIMEOUT_SECS", "30").parse()?),
            cors_origins: split_list(&var_or("CORS_ORIGINS", "*")),
            proxy_extra_hosts: split_list(&var_or("PROXY_EXTRA_HOSTS", "")),
            browser_enabled: var_or("BROWSER_ENABLED", "false").eq_ignore_ascii_case("true"),
            impersonation_enabled: !var_or("IMPERSONATION_ENABLED", "true")
                .eq_ignore_ascii_case("false"),
            bato_base_url: var_or("BATO_BASE_URL", "https://bato.si")
                .trim_end_matches('/')
                .to_string(),
        })
    }
}

#[cfg(test)]
impl Config {
    /// Defaults for tests: no network features, one download slot.
    pub fn for_tests(download_dir: PathBuf) -> Self {
        Self {
            bind: "127.0.0.1".parse().unwrap(),
            port: 0,
            download_dir,
            max_concurrent_downloads: 1,
            image_concurrency: 2,
            cache_ttl: Duration::from_secs(1),
            status_ttl: Duration::from_secs(1),
            request_timeout: Duration::from_secs(5),
            cors_origins: vec!["*".into()],
            proxy_extra_hosts: vec![],
            browser_enabled: false,
            impersonation_enabled: false,
            bato_base_url: "https://bato.si".into(),
        }
    }
}

fn var_or(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}
