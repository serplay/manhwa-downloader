//! Source abstraction and registry.
//!
//! A source knows how to search, list chapters, and resolve the page images of
//! a chapter. Downloading and packaging are not source concerns; the pipeline
//! handles them using the `PageUrl` header profile a source returns and the
//! fetch tier the source's `SourceMeta` declares.

pub mod asura;
pub mod bato;
pub mod html;
pub mod http;
pub mod madara;
pub mod mangadex;
pub mod mangahere;
pub mod mangapill;
pub mod mangareader;
pub mod throttle;
pub mod weebcentral;

#[cfg(test)]
mod live_tests;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use reqwest::Client;

use crate::{
    config::Config,
    error::{AppError, AppResult},
    model::{Comic, SourceMeta, Volume},
};

/// One page image to download, plus the headers its host expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUrl {
    pub url: String,
    pub referer: Option<String>,
}

/// Per-request search switches.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchOptions {
    /// Include titles the source flags as adult. Off by default.
    pub include_adult: bool,
}

/// Everything a source needs at call time.
pub struct Ctx {
    /// Plain client, for sources that need `reqwest` specifics (JSON APIs).
    pub client: Client,
    /// Tiered fetcher with Cloudflare escalation. Preferred.
    pub fetcher: Arc<http::Fetcher>,
}

#[async_trait]
pub trait Source: Send + Sync {
    fn meta(&self) -> &SourceMeta;

    async fn search(
        &self,
        ctx: &Ctx,
        query: &str,
        lang: Option<&str>,
        opts: &SearchOptions,
    ) -> AppResult<Vec<Comic>>;

    async fn chapters(
        &self,
        ctx: &Ctx,
        comic_id: &str,
        lang: Option<&str>,
    ) -> AppResult<Vec<Volume>>;

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>>;
}

/// Old slugs that still resolve, mapped to the source that replaced them.
const ALIASES: &[(&str, &str)] = &[("yakshascans", "ravenscans"), ("asura", "asurascans")];

/// All known sources in legacy id order. Lookup by numeric id, slug, or alias.
pub struct Registry {
    ordered: Vec<Arc<dyn Source>>,
    by_key: HashMap<String, Arc<dyn Source>>,
}

impl Registry {
    pub fn new(config: &Config) -> Self {
        let browser = config.browser_enabled;
        let ordered: Vec<Arc<dyn Source>> = vec![
            Arc::new(mangadex::MangaDex::new()),
            Arc::new(madara::Madara::new(madara::MANHUAUS, browser)),
            Arc::new(mangareader::MangaReader::new(mangareader::RAVENSCANS)),
            Arc::new(asura::Asura::new()),
            Arc::new(madara::Madara::new(madara::KUNMANGA, browser)),
            Arc::new(madara::Madara::new(madara::TOONILY, browser)),
            Arc::new(madara::Madara::new(madara::TOONGOD, browser)),
            Arc::new(mangahere::Mangahere::new()),
            Arc::new(mangapill::Mangapill::new()),
            Arc::new(bato::Bato::new(&config.bato_base_url)),
            Arc::new(weebcentral::Weebcentral::new()),
        ];
        Self::from_sources(ordered)
    }

    /// Build from an explicit list. Tests use it to register fake sources.
    pub fn from_sources(ordered: Vec<Arc<dyn Source>>) -> Self {
        let mut by_key = HashMap::new();
        for s in &ordered {
            by_key.insert(s.meta().id_str(), Arc::clone(s));
            by_key.insert(s.meta().slug.to_string(), Arc::clone(s));
        }
        for (alias, target) in ALIASES {
            if let Some(s) = by_key.get(*target).cloned() {
                by_key.insert((*alias).to_string(), s);
            }
        }
        Self { ordered, by_key }
    }

    pub fn get(&self, key: &str) -> AppResult<Arc<dyn Source>> {
        self.by_key
            .get(key.trim().to_ascii_lowercase().as_str())
            .cloned()
            .ok_or_else(|| AppError::UnknownSource(key.to_string()))
    }

    pub fn all(&self) -> &[Arc<dyn Source>] {
        &self.ordered
    }

    /// Sources that can be used from this build (excludes browser-only ones
    /// unless a browser is configured).
    pub fn implemented(&self) -> impl Iterator<Item = &Arc<dyn Source>> {
        self.ordered.iter().filter(|s| s.meta().capabilities.search)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::new(&Config::for_tests(std::env::temp_dir()))
    }

    #[test]
    fn every_legacy_id_resolves_in_order() {
        let r = registry();
        assert_eq!(r.all().len(), 11);
        for (i, s) in r.all().iter().enumerate() {
            assert_eq!(s.meta().id as usize, i);
            assert_eq!(r.get(&i.to_string()).unwrap().meta().slug, s.meta().slug);
        }
    }

    #[test]
    fn aliases_and_case_insensitivity() {
        let r = registry();
        assert_eq!(r.get("yakshascans").unwrap().meta().slug, "ravenscans");
        assert_eq!(r.get(" Toonily ").unwrap().meta().id, 5);
        assert!(r.get("nope").is_err());
    }

    #[test]
    fn browser_only_sources_are_not_advertised_without_a_browser() {
        let r = registry();
        let implemented: Vec<&str> = r.implemented().map(|s| s.meta().slug).collect();
        assert_eq!(implemented.len(), 8, "{implemented:?}");
        for blocked in ["manhuaus", "kunmanga", "toongod"] {
            assert!(!implemented.contains(&blocked));
            assert!(r.get(blocked).unwrap().meta().capabilities.needs_browser);
        }
    }

    #[test]
    fn adult_sources_are_marked() {
        let r = registry();
        let adult: Vec<&str> = r
            .all()
            .iter()
            .filter(|s| s.meta().adult)
            .map(|s| s.meta().slug)
            .collect();
        assert_eq!(adult, ["toonily", "toongod"]);
    }
}
