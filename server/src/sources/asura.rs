//! Asura Scans. The site moved from `asuracomic.net` to `asurascans.com` and
//! now fronts a JSON API at `api.asurascans.com`, which is what we talk to.
//! Premium (early-access) chapters are listed by the API but their pages are
//! gated, so they are skipped.

use async_trait::async_trait;
use serde::Deserialize;

use super::{Ctx, PageUrl, Source, html::fmt_number, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        sort_volumes,
    },
};

const SITE: &str = "https://asurascans.com";
const API: &str = "https://api.asurascans.com/api";
const SLUG: &str = "asurascans";

pub struct Asura {
    meta: SourceMeta,
}

impl Default for Asura {
    fn default() -> Self {
        Self::new()
    }
}

impl Asura {
    pub fn new() -> Self {
        Self {
            meta: SourceMeta {
                id: 3,
                slug: SLUG,
                name: "Asura Scans",
                base_url: SITE.to_string(),
                speed: Speed::Fast,
                languages: vec!["en"],
                tier: FetchTier::Plain,
                capabilities: SourceCapabilities {
                    search: true,
                    chapters: true,
                    download: true,
                    needs_browser: false,
                },
            },
        }
    }
}

fn referer() -> Option<String> {
    Some(format!("{SITE}/"))
}

// ---- wire types -------------------------------------------------------------

#[derive(Deserialize)]
struct Listing<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct SeriesRow {
    slug: String,
    title: String,
    cover: Option<String>,
}

#[derive(Deserialize)]
struct ChapterRow {
    number: serde_json::Value,
    slug: String,
    page_count: Option<u32>,
    #[serde(default)]
    is_premium: bool,
    title: Option<String>,
}

#[derive(Deserialize)]
struct ChapterEnvelope {
    data: ChapterData,
}

#[derive(Deserialize)]
struct ChapterData {
    #[serde(default)]
    access_gate: String,
    chapter: ChapterPages,
}

#[derive(Deserialize)]
struct ChapterPages {
    #[serde(default)]
    pages: Vec<Page>,
}

#[derive(Deserialize)]
struct Page {
    url: String,
}

fn decode<T: for<'de> Deserialize<'de>>(body: &str) -> AppResult<T> {
    serde_json::from_str(body).map_err(|e| AppError::Parse {
        site: SLUG.into(),
        message: e.to_string(),
    })
}

fn number_label(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Number(n) => n.as_f64().map(fmt_number).unwrap_or_else(|| n.to_string()),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

// ---- parsing ----------------------------------------------------------------

pub fn parse_search(body: &str) -> AppResult<Vec<Comic>> {
    let listing: Listing<SeriesRow> = decode(body)?;
    Ok(listing
        .data
        .into_iter()
        .map(|s| Comic {
            id: s.slug,
            title: [("en".to_string(), s.title)].into_iter().collect(),
            cover: s.cover.filter(|c| !c.is_empty()).map(|url| Cover {
                url,
                referer: referer(),
            }),
            languages: vec!["en".into()],
        })
        .collect())
}

pub fn parse_chapters(series_slug: &str, body: &str) -> AppResult<Vec<Volume>> {
    let listing: Listing<ChapterRow> = decode(body)?;
    let mut vol = Volume::named("Vol 1");
    for c in listing.data.into_iter().filter(|c| !c.is_premium) {
        vol.chapters.push(Chapter {
            id: format!("{series_slug}/{}", c.slug),
            number: number_label(&c.number),
            title: c.title.filter(|t| !t.trim().is_empty()),
            pages: c.page_count,
        });
    }
    let mut volumes = vec![vol];
    sort_volumes(&mut volumes);
    Ok(volumes)
}

pub fn parse_pages(body: &str) -> AppResult<Vec<PageUrl>> {
    let env: ChapterEnvelope = decode(body)?;
    if !env.data.access_gate.is_empty() {
        return Err(AppError::Upstream {
            site: Some(SLUG.into()),
            message: format!(
                "chapter is gated ({}); only free chapters can be downloaded",
                env.data.access_gate
            ),
        });
    }
    let pages: Vec<PageUrl> = env
        .data
        .chapter
        .pages
        .into_iter()
        .map(|p| PageUrl {
            url: p.url,
            referer: referer(),
        })
        .collect();
    if pages.is_empty() {
        return Err(AppError::Parse {
            site: SLUG.into(),
            message: "chapter has no pages".into(),
        });
    }
    Ok(pages)
}

fn split_chapter_id(chapter_id: &str) -> AppResult<(&str, &str)> {
    chapter_id
        .split_once('/')
        .filter(|(s, c)| !s.is_empty() && !c.is_empty())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "asurascans chapter id must be `<series>/<chapter>`, got `{chapter_id}`"
            ))
        })
}

#[async_trait]
impl Source for Asura {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let url = http::with_query(&format!("{API}/series"), &[("search", query)]);
        let body = ctx
            .fetcher
            .get(url)
            .source(SLUG)
            .accept("application/json")
            .header(http::header::ORIGIN, SITE)
            .referer(&format!("{SITE}/"))
            .text()
            .await?;
        parse_search(&body)
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let body = ctx
            .fetcher
            .get(format!("{API}/series/{comic_id}/chapters"))
            .source(SLUG)
            .accept("application/json")
            .header(http::header::ORIGIN, SITE)
            .referer(&format!("{SITE}/"))
            .text()
            .await?;
        parse_chapters(comic_id, &body)
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let (series, chapter) = split_chapter_id(chapter_id)?;
        let body = ctx
            .fetcher
            .get(format!("{API}/series/{series}/chapters/{chapter}"))
            .source(SLUG)
            .accept("application/json")
            .header(http::header::ORIGIN, SITE)
            .referer(&format!("{SITE}/"))
            .text()
            .await?;
        parse_pages(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_fixture() {
        let comics = parse_search(include_str!("fixtures/asura_search.json")).unwrap();
        assert!(comics.len() >= 5);
        assert!(comics.iter().any(|c| c.id == "emperor-of-solo-play"));
        let c = comics
            .iter()
            .find(|c| c.id == "emperor-of-solo-play")
            .unwrap();
        assert_eq!(c.title["en"], "Emperor of Solo Play");
        assert!(
            c.cover
                .as_ref()
                .unwrap()
                .url
                .starts_with("https://cdn.asurascans.com/")
        );
    }

    #[test]
    fn parses_chapters_fixture() {
        let volumes = parse_chapters(
            "emperor-of-solo-play",
            include_str!("fixtures/asura_chapters.json"),
        )
        .unwrap();
        let chapters = &volumes[0].chapters;
        assert_eq!(chapters.len(), 83);
        assert_eq!(chapters[0].number, "1");
        // Chapter slugs are sometimes UUIDs rather than `chapter-N`; the id keeps
        // whatever the API uses because the pages endpoint is keyed by it.
        assert_eq!(
            chapters[0].id,
            "emperor-of-solo-play/e6bf0655-ff90-45b7-8130-dfe10b8ac014"
        );
        assert!(chapters.iter().all(|c| c.pages.is_some()));
        assert_eq!(chapters.last().unwrap().number, "83");
        assert_eq!(
            chapters.last().unwrap().id,
            "emperor-of-solo-play/chapter-83"
        );
    }

    #[test]
    fn skips_premium_chapters() {
        let body = r#"{"data":[
            {"id":1,"number":2,"slug":"chapter-2","page_count":10,"is_premium":true},
            {"id":2,"number":1.5,"slug":"chapter-1-5","page_count":8,"is_premium":false},
            {"id":3,"number":1,"slug":"chapter-1","page_count":9}
        ]}"#;
        let volumes = parse_chapters("x", body).unwrap();
        let numbers: Vec<&str> = volumes[0]
            .chapters
            .iter()
            .map(|c| c.number.as_str())
            .collect();
        assert_eq!(numbers, ["1", "1.5"]);
    }

    #[test]
    fn parses_pages_fixture() {
        let pages = parse_pages(include_str!("fixtures/asura_chapter_pages.json")).unwrap();
        assert!(pages.len() > 10);
        assert!(
            pages[0]
                .url
                .contains("/asura-images/chapters/emperor-of-solo-play/83/")
        );
        assert_eq!(pages[0].referer.as_deref(), Some("https://asurascans.com/"));
    }

    #[test]
    fn gated_chapters_are_errors() {
        let body = r#"{"data":{"access_gate":"premium","chapter":{"id":1,"pages":[]}}}"#;
        assert!(matches!(parse_pages(body), Err(AppError::Upstream { .. })));
    }
}
