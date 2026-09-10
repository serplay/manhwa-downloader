//! WordPress "Madara" theme sites: Manhuaus, Kunmanga, Toonily, Toongod. One
//! parameterized implementation replaces four near-identical Selenium scrapers.
//!
//! All four sit behind Cloudflare. Toonily accepts the Chrome-impersonating
//! client; the other three serve a managed JavaScript challenge that only a
//! real browser passes (measured 2026-09-10). They stay registered so a future
//! `browser` feature or a relaxed edge rule lights them up without code
//! changes, but they advertise `needs_browser` and report `blocked` status.

use async_trait::async_trait;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use scraper::Html;

use super::{Ctx, PageUrl, Source, html::*, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        sort_volumes,
    },
};

/// Percent-encode a path segment, keeping the unreserved characters a browser keeps.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStyle {
    /// `/?s=<q>&post_type=wp-manga` (Madara default).
    WpQuery,
    /// `/search/<q>` (Toonily).
    PathSearch,
}

/// Static description of one Madara installation.
#[derive(Debug, Clone)]
pub struct MadaraSite {
    pub id: u8,
    pub slug: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    /// Path segment under which series live: `manga`, `serie`, `webtoon`.
    pub series_path: &'static str,
    pub search: SearchStyle,
    pub speed: Speed,
    /// Serves a managed challenge the impersonating client cannot pass.
    pub needs_browser: bool,
}

pub const MANHUAUS: MadaraSite = MadaraSite {
    id: 1,
    slug: "manhuaus",
    name: "Manhuaus",
    base_url: "https://manhuaus.com",
    series_path: "manga",
    search: SearchStyle::WpQuery,
    speed: Speed::Slow,
    needs_browser: true,
};

pub const KUNMANGA: MadaraSite = MadaraSite {
    id: 4,
    slug: "kunmanga",
    name: "Kunmanga",
    base_url: "https://kunmanga.com",
    series_path: "manga",
    search: SearchStyle::WpQuery,
    speed: Speed::Slow,
    needs_browser: true,
};

pub const TOONILY: MadaraSite = MadaraSite {
    id: 5,
    slug: "toonily",
    name: "Toonily",
    base_url: "https://toonily.com",
    series_path: "serie",
    search: SearchStyle::PathSearch,
    speed: Speed::Fast,
    needs_browser: false,
};

pub const TOONGOD: MadaraSite = MadaraSite {
    id: 6,
    slug: "toongod",
    name: "Toongod",
    base_url: "https://www.toongod.org",
    series_path: "webtoon",
    search: SearchStyle::WpQuery,
    speed: Speed::Slow,
    needs_browser: true,
};

pub struct Madara {
    meta: SourceMeta,
    site: MadaraSite,
}

impl Madara {
    /// `browser_available` decides whether a `needs_browser` site is advertised
    /// as usable. Without a browser its calls still run (and fail with a clear
    /// `SOURCE_BLOCKED`), but it is left out of search-all fan-out.
    pub fn new(site: MadaraSite, browser_available: bool) -> Self {
        let usable = !site.needs_browser || browser_available;
        Self {
            meta: SourceMeta {
                id: site.id,
                slug: site.slug,
                name: site.name,
                base_url: site.base_url.to_string(),
                speed: site.speed,
                languages: vec!["en"],
                tier: if site.needs_browser {
                    FetchTier::Browser
                } else {
                    FetchTier::Impersonate
                },
                capabilities: SourceCapabilities {
                    search: usable,
                    chapters: usable,
                    download: usable,
                    needs_browser: site.needs_browser,
                },
            },
            site,
        }
    }

    fn referer(&self) -> String {
        format!("{}/", self.site.base_url)
    }

    fn search_url(&self, query: &str) -> String {
        match self.site.search {
            SearchStyle::WpQuery => http::with_query(
                &format!("{}/", self.site.base_url),
                &[("s", query), ("post_type", "wp-manga")],
            ),
            // Toonily matches on a hyphenated slug; a percent-encoded space finds nothing.
            SearchStyle::PathSearch => format!(
                "{}/search/{}?op&author&artist&adult",
                self.site.base_url,
                utf8_percent_encode(
                    &query.split_whitespace().collect::<Vec<_>>().join("-"),
                    PATH_SEGMENT
                )
            ),
        }
    }

    fn series_url(&self, id: &str) -> String {
        format!("{}/{}/{id}/", self.site.base_url, self.site.series_path)
    }
}

// ---- parsing (shared by every Madara site) ------------------------------------

pub fn parse_search(base_url: &str, body: &str) -> Vec<Comic> {
    let doc = Html::parse_document(body);
    let item = sel("div.c-tabs-item__content, div.page-item-detail");
    let thumb_link = sel(".tab-thumb a[href], .item-thumb a[href]");
    let title_link = sel("h3 a[href], .post-title a[href]");
    let img = sel("img");
    let mut seen = std::collections::HashSet::new();
    doc.select(&item)
        .filter_map(|el| {
            let a = el
                .select(&thumb_link)
                .next()
                .or_else(|| el.select(&title_link).next())?;
            let href = a.value().attr("href")?;
            let id = last_path_segment(href)?;
            if !seen.insert(id.clone()) {
                return None;
            }
            let title = a
                .value()
                .attr("title")
                .map(collapse_ws)
                .filter(|t| !t.is_empty())
                .or_else(|| el.select(&title_link).next().map(text))
                .filter(|t| !t.is_empty())?;
            let cover = el.select(&img).next().and_then(image_src).map(|url| Cover {
                url: absolutize(base_url, &url),
                referer: Some(format!("{base_url}/")),
            });
            Some(Comic {
                id,
                title: [("en".to_string(), title)].into_iter().collect(),
                cover,
                languages: vec!["en".into()],
            })
        })
        .collect()
}

/// Parse a chapter list (series page or the `ajax/chapters` fragment).
pub fn parse_chapters(series_id: &str, body: &str) -> Vec<Volume> {
    let doc = Html::parse_document(body);
    let link = sel("li.wp-manga-chapter a[href]");
    let mut vol = Volume::named("Vol 1");
    let mut seen = std::collections::HashSet::new();
    for a in doc.select(&link) {
        let Some(slug) = a.value().attr("href").and_then(last_path_segment) else {
            continue;
        };
        if !seen.insert(slug.clone()) {
            continue;
        }
        let label = text(a);
        let number = chapter_number(&label);
        vol.chapters.push(Chapter {
            id: format!("{series_id}/{slug}"),
            title: title_if_informative(&label, &number),
            number,
            pages: None,
        });
    }
    let mut volumes = vec![vol];
    sort_volumes(&mut volumes);
    volumes
}

pub fn parse_pages(site_slug: &str, base_url: &str, body: &str) -> AppResult<Vec<PageUrl>> {
    let doc = Html::parse_document(body);
    let img = sel("div.reading-content img, div.read-container img");
    let pages: Vec<PageUrl> = doc
        .select(&img)
        .filter_map(image_src)
        .map(|url| PageUrl {
            url: absolutize(base_url, &url),
            referer: Some(format!("{base_url}/")),
        })
        .collect();
    if pages.is_empty() {
        return Err(AppError::Parse {
            site: site_slug.into(),
            message: "chapter page has no images".into(),
        });
    }
    Ok(pages)
}

#[async_trait]
impl Source for Madara {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let body = ctx
            .fetcher
            .get(self.search_url(query))
            .tier(self.meta.tier)
            .source(self.site.slug)
            .referer(&self.referer())
            .text()
            .await?;
        Ok(parse_search(self.site.base_url, &body))
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let series_url = self.series_url(comic_id);
        // The theme's AJAX fragment is small and stable; fall back to the full page.
        let ajax = ctx
            .fetcher
            .post(format!("{series_url}ajax/chapters/"))
            .tier(self.meta.tier)
            .source(self.site.slug)
            .referer(&series_url)
            .xhr()
            .form("")
            .text()
            .await;
        if let Ok(body) = ajax {
            let volumes = parse_chapters(comic_id, &body);
            if volumes.iter().any(|v| !v.chapters.is_empty()) {
                return Ok(volumes);
            }
        }
        let body = ctx
            .fetcher
            .get(&series_url)
            .tier(self.meta.tier)
            .source(self.site.slug)
            .referer(&self.referer())
            .text()
            .await?;
        Ok(parse_chapters(comic_id, &body))
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let body = ctx
            .fetcher
            .get(self.series_url(chapter_id))
            .tier(self.meta.tier)
            .source(self.site.slug)
            .referer(&self.referer())
            .text()
            .await?;
        parse_pages(self.site.slug, self.site.base_url, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toonily_search_fixture() {
        let comics = parse_search(
            TOONILY.base_url,
            include_str!("fixtures/toonily_search.html"),
        );
        assert!(comics.len() >= 2, "got {}", comics.len());
        assert_eq!(comics[0].id, "solo-leveling-ragnarok-9a9db6f6");
        assert_eq!(comics[0].title["en"], "Solo Leveling: Ragnarok");
        let cover = comics[0].cover.as_ref().unwrap();
        assert!(cover.url.starts_with("https://static.tnlycdn.com/"));
        assert_eq!(cover.referer.as_deref(), Some("https://toonily.com/"));
    }

    #[test]
    fn parses_toonily_ajax_chapters_fixture() {
        let volumes = parse_chapters(
            "solo-leveling-ragnarok-9a9db6f6",
            include_str!("fixtures/toonily_ajax_chapters.html"),
        );
        let chapters = &volumes[0].chapters;
        assert_eq!(chapters.len(), 47);
        assert_eq!(chapters[0].number, "1");
        assert_eq!(chapters[46].number, "47");
        assert_eq!(
            chapters[46].id,
            "solo-leveling-ragnarok-9a9db6f6/chapter-47"
        );
        assert_eq!(chapters[46].title, None);
    }

    #[test]
    fn parses_toonily_chapter_fixture() {
        let pages = parse_pages(
            "toonily",
            TOONILY.base_url,
            include_str!("fixtures/toonily_chapter.html"),
        )
        .unwrap();
        assert_eq!(pages.len(), 21);
        assert!(
            pages[0]
                .url
                .starts_with("https://data.tnlycdn.com/chapters/")
        );
        assert_eq!(pages[0].referer.as_deref(), Some("https://toonily.com/"));
    }

    #[test]
    fn parses_classic_madara_search_markup() {
        // The `c-tabs-item__content` layout Manhuaus, Kunmanga and Toongod use.
        let body = r#"<div class="row c-tabs-item__content">
            <div class="col-4"><div class="tab-thumb c-image-hover">
              <a href="https://kunmanga.com/manga/solo-leveling/" title="Solo Leveling">
                <img data-src="https://kunmanga.com/wp-content/uploads/cover.jpg" src="data:image/gif;base64,x"/>
              </a></div></div>
            <div class="col-8"><div class="tab-summary"><div class="post-title"><h3 class="h4">
              <a href="https://kunmanga.com/manga/solo-leveling/">Solo Leveling</a></h3></div></div></div>
          </div>"#;
        let comics = parse_search(KUNMANGA.base_url, body);
        assert_eq!(comics.len(), 1);
        assert_eq!(comics[0].id, "solo-leveling");
        assert_eq!(comics[0].title["en"], "Solo Leveling");
        assert_eq!(
            comics[0].cover.as_ref().unwrap().url,
            "https://kunmanga.com/wp-content/uploads/cover.jpg"
        );
    }

    #[test]
    fn search_urls() {
        let t = Madara::new(TOONILY, false);
        assert_eq!(
            t.search_url("  solo   leveling "),
            "https://toonily.com/search/solo-leveling?op&author&artist&adult"
        );
        let k = Madara::new(KUNMANGA, false);
        assert_eq!(
            k.search_url("solo leveling"),
            "https://kunmanga.com/?s=solo+leveling&post_type=wp-manga"
        );
        assert!(!k.meta().capabilities.search);
        assert!(k.meta().capabilities.needs_browser);
        assert!(t.meta().capabilities.search);
    }
}
