//! Weebcentral. The site is HTMX-driven: search results, the full chapter
//! list and the page images all come from partial-HTML endpoints, so the
//! "Show All Chapters" click the old Selenium scraper performed is a GET here.

use async_trait::async_trait;
use scraper::Html;

use super::{Ctx, PageUrl, Source, html::*, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        sort_volumes,
    },
};

const BASE: &str = "https://weebcentral.com";
const SLUG: &str = "weebcentral";

pub struct Weebcentral {
    meta: SourceMeta,
}

impl Default for Weebcentral {
    fn default() -> Self {
        Self::new()
    }
}

impl Weebcentral {
    pub fn new() -> Self {
        Self {
            meta: SourceMeta {
                id: 10,
                slug: SLUG,
                name: "Weebcentral",
                base_url: BASE.to_string(),
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
    Some(format!("{BASE}/"))
}

pub fn parse_search(body: &str) -> Vec<Comic> {
    let doc = Html::parse_document(body);
    let article = sel("article.bg-base-300");
    let link = sel(r#"a[href*="/series/"]"#);
    let title = sel("div.text-lg");
    let img = sel("img");
    let mut seen = std::collections::HashSet::new();
    doc.select(&article)
        .filter_map(|art| {
            let a = art.select(&link).next()?;
            let href = a.value().attr("href")?;
            let id = path_after(href, "/series/")?.split('/').next()?.to_string();
            if !seen.insert(id.clone()) {
                return None;
            }
            let img_el = art.select(&img).next();
            let title = art
                .select(&title)
                .next()
                .map(text)
                .or_else(|| {
                    img_el
                        .and_then(|i| i.value().attr("alt"))
                        .map(|alt| collapse_ws(alt.trim_end_matches(" cover")))
                })
                .filter(|t| !t.is_empty())?;
            let cover = img_el.and_then(image_src).map(|url| Cover {
                url: absolutize(BASE, &url),
                referer: referer(),
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

pub fn parse_chapters(body: &str) -> Vec<Volume> {
    let doc = Html::parse_document(body);
    let link = sel(r#"a[href*="/chapters/"]"#);
    let label_sel = sel("span.grow > span");
    let mut vol = Volume::named("Vol 1");
    let mut seen = std::collections::HashSet::new();
    for a in doc.select(&link) {
        let Some(id) = a.value().attr("href").and_then(last_path_segment) else {
            continue;
        };
        if !seen.insert(id.clone()) {
            continue;
        }
        let label = a
            .select(&label_sel)
            .next()
            .map(text)
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| text(a));
        let number = chapter_number(&label);
        vol.chapters.push(Chapter {
            id,
            title: title_if_informative(&label, &number),
            number,
            pages: None,
        });
    }
    let mut volumes = vec![vol];
    sort_volumes(&mut volumes);
    volumes
}

pub fn parse_pages(body: &str) -> AppResult<Vec<PageUrl>> {
    let doc = Html::parse_document(body);
    let scoped = sel("section#chapter-images img");
    let any = sel("img");
    let collect = |s: &scraper::Selector| -> Vec<PageUrl> {
        doc.select(s)
            .filter_map(image_src)
            .filter(|u| u.starts_with("http") && !u.contains("/static/"))
            .map(|url| PageUrl {
                url,
                referer: referer(),
            })
            .collect()
    };
    let mut pages = collect(&scoped);
    if pages.is_empty() {
        pages = collect(&any);
    }
    if pages.is_empty() {
        return Err(AppError::Parse {
            site: SLUG.into(),
            message: "chapter has no images".into(),
        });
    }
    Ok(pages)
}

#[async_trait]
impl Source for Weebcentral {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let url = http::with_query(
            &format!("{BASE}/search/data"),
            &[
                ("author", ""),
                ("text", query),
                ("sort", "Best Match"),
                ("order", "Descending"),
                ("official", "Any"),
                ("anime", "Any"),
                ("adult", "Any"),
                ("display_mode", "Full Display"),
            ],
        );
        let body = ctx
            .fetcher
            .get(url)
            .source(SLUG)
            .htmx()
            .referer(&format!("{BASE}/search"))
            .text()
            .await?;
        Ok(parse_search(&body))
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let body = ctx
            .fetcher
            .get(format!("{BASE}/series/{comic_id}/full-chapter-list"))
            .source(SLUG)
            .htmx()
            .referer(&format!("{BASE}/series/{comic_id}"))
            .text()
            .await?;
        Ok(parse_chapters(&body))
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let chapter_url = format!("{BASE}/chapters/{chapter_id}");
        let url = http::with_query(
            &format!("{chapter_url}/images"),
            &[
                ("is_prev", "False"),
                ("current_page", "1"),
                ("reading_style", "long_strip"),
            ],
        );
        let body = ctx
            .fetcher
            .get(url)
            .source(SLUG)
            .htmx()
            .referer(&chapter_url)
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
        let comics = parse_search(include_str!("fixtures/weebcentral_search.html"));
        assert!(comics.len() >= 3, "got {}", comics.len());
        assert_eq!(comics[0].title["en"], "Solo Leveling");
        assert_eq!(
            comics[0].id.len(),
            26,
            "id should be a ULID: {}",
            comics[0].id
        );
        assert!(
            comics[0]
                .cover
                .as_ref()
                .unwrap()
                .url
                .contains("compsci88.com")
        );
    }

    #[test]
    fn parses_chapters_fixture() {
        let volumes = parse_chapters(include_str!("fixtures/weebcentral_chapters.html"));
        let chapters = &volumes[0].chapters;
        assert_eq!(chapters.len(), 6);
        assert_eq!(chapters[0].number, "1");
        assert_eq!(chapters[5].number, "6");
        assert_eq!(chapters[5].id, "01J76XYY7Q5DA1HQMDD9D7GQTC");
        assert_eq!(chapters[5].title.as_deref(), Some("Volume 6"));
    }

    #[test]
    fn parses_pages_fixture() {
        let pages = parse_pages(include_str!("fixtures/weebcentral_images.html")).unwrap();
        assert!(pages.len() > 10);
        assert_eq!(
            pages[0].url,
            "https://official.lowee.us/manga/Kobato/0006-001.png"
        );
    }
}
