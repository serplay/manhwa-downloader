//! In-memory TTL caches. Replaces the Redis-backed fastapi-cache from the Python server.

use std::{collections::HashMap, sync::Arc, time::Duration};

use moka::future::Cache;

use crate::model::{Comic, SourceStatus, Volume};

#[derive(Clone)]
pub struct Caches {
    pub search: Cache<String, Arc<Vec<Comic>>>,
    pub chapters: Cache<String, Arc<Vec<Volume>>>,
    pub status: Cache<(), Arc<HashMap<String, SourceStatus>>>,
}

impl Caches {
    pub fn new(ttl: Duration, status_ttl: Duration) -> Self {
        Self {
            search: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(ttl)
                .build(),
            chapters: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(ttl)
                .build(),
            status: Cache::builder()
                .max_capacity(1)
                .time_to_live(status_ttl)
                .build(),
        }
    }

    pub fn search_key(slug: &str, query: &str, lang: Option<&str>) -> String {
        format!(
            "{slug}:{}:{}",
            query.trim().to_lowercase(),
            lang.unwrap_or("")
        )
    }

    pub fn chapters_key(slug: &str, comic_id: &str, lang: Option<&str>) -> String {
        format!("{slug}:{comic_id}:{}", lang.unwrap_or(""))
    }
}
