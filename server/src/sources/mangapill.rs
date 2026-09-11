//! Mangapill, scraped natively. The old server went through Consumet; the site
//! is server-rendered and needs no JavaScript, so that hop is gone.

use std::collections::HashMap;

use async_trait::async_trait;
use scraper::Html;

use super::{Ctx, PageUrl, SearchOptions, Source, html::*, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        sort_volumes,
    },
};

const BASE: &str = "https://mangapill.com";
const SLUG: &str = "mangapill";

pub struct Mangapill {
    meta: SourceMeta,
}

impl Default for Mangapill {
    fn default() -> Self {
        Self::new()
    }
}

impl Mangapill {
    pub fn new() -> Self {
        Self {
            meta: SourceMeta {
                id: 8,
                slug: SLUG,
                name: "Mangapill",
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
                adult: false,
            },
        }
    }
}

fn referer() -> Option<String> {
    Some(format!("{BASE}/"))
}

pub fn parse_search(body: &str) -> Vec<Comic> {
    let doc = Html::parse_document(body);
    let link = sel(r#"a[href^="/manga/"]"#);
    let img = sel("img");
    let title = sel("div.font-black");
    let mut order = Vec::new();
    let genre = sel("div.bg-card.rounded");
    let mut found: HashMap<String, (Option<String>, Option<String>, bool)> = HashMap::new();
    for a in doc.select(&link) {
        let Some(id) = a
            .value()
            .attr("href")
            .and_then(|h| path_after(h, "/manga/"))
        else {
            continue;
        };
        let entry = found.entry(id.clone()).or_insert_with(|| {
            order.push(id.clone());
            (None, None, false)
        });
        if let Some(i) = a.select(&img).next() {
            entry.1 = entry.1.take().or_else(|| image_src(i));
        }
        // Genre chips are siblings of the title link inside the card's text column.
        if let Some(t) = a.select(&title).next() {
            entry.0 = entry.0.take().or_else(|| Some(text(t)));
            if let Some(column) = a.parent().and_then(scraper::ElementRef::wrap) {
                entry.2 |= column.select(&genre).any(|g| is_adult_genre(&text(g)));
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| {
            let (title, cover, adult) = found.remove(&id)?;
            let title = title.filter(|t| !t.is_empty())?;
            Some(Comic {
                id,
                title: [("en".to_string(), title)].into_iter().collect(),
                cover: cover.map(|url| Cover {
                    url: absolutize(BASE, &url),
                    referer: referer(),
                }),
                languages: vec!["en".into()],
                adult,
            })
        })
        .collect()
}

pub fn parse_chapters(body: &str) -> Vec<Volume> {
    let doc = Html::parse_document(body);
    let link = sel(r##"#chapters a[href^="/chapters/"]"##);
    let mut vol = Volume::named("Vol 1");
    for a in doc.select(&link) {
        let Some(id) = a
            .value()
            .attr("href")
            .and_then(|h| path_after(h, "/chapters/"))
        else {
            continue;
        };
        let label = text(a);
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
    let img = sel("img.js-page");
    let pages: Vec<PageUrl> = doc
        .select(&img)
        .filter_map(image_src)
        .map(|url| PageUrl {
            url: absolutize(BASE, &url),
            referer: referer(),
        })
        .collect();
    if pages.is_empty() {
        return Err(AppError::Parse {
            site: SLUG.into(),
            message: "chapter page has no images".into(),
        });
    }
    Ok(pages)
}

#[async_trait]
impl Source for Mangapill {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(
        &self,
        ctx: &Ctx,
        query: &str,
        _lang: Option<&str>,
        opts: &SearchOptions,
    ) -> AppResult<Vec<Comic>> {
        let url = http::with_query(
            &format!("{BASE}/search"),
            &[("q", query), ("type", ""), ("status", "")],
        );
        let body = ctx.fetcher.get(url).source(SLUG).text().await?;
        Ok(parse_search(&body)
            .into_iter()
            .filter(|c| opts.include_adult || !c.adult)
            .collect())
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let body = ctx
            .fetcher
            .get(format!("{BASE}/manga/{comic_id}"))
            .source(SLUG)
            .text()
            .await?;
        Ok(parse_chapters(&body))
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let body = ctx
            .fetcher
            .get(format!("{BASE}/chapters/{chapter_id}"))
            .source(SLUG)
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
        let comics = parse_search(include_str!("fixtures/mangapill_search.html"));
        assert!(comics.len() >= 5, "got {}", comics.len());
        let first = &comics[0];
        assert_eq!(first.id, "8136/solo-leveling-novel");
        assert_eq!(first.title["en"], "Solo Leveling Novel");
        let cover = first.cover.as_ref().unwrap();
        assert!(cover.url.starts_with("https://"), "{}", cover.url);
        assert_eq!(cover.referer.as_deref(), Some("https://mangapill.com/"));
        assert!(
            comics
                .iter()
                .any(|c| c.title["en"] == "Solo Leveling: Ragnarok Novel")
        );
        assert!(comics.iter().all(|c| c.cover.is_some()));
    }

    #[test]
    fn parses_chapters_fixture() {
        let volumes = parse_chapters(include_str!("fixtures/mangapill_series.html"));
        assert_eq!(volumes.len(), 1);
        let chapters = &volumes[0].chapters;
        assert!(chapters.len() > 100, "got {}", chapters.len());
        // Sorted ascending by number; ids keep the full path.
        assert!(chapters[0].sort_key() <= chapters[1].sort_key());
        let last = chapters.last().unwrap();
        assert_eq!(last.number, "203");
        assert_eq!(last.id, "8136-20203000/solo-leveling-novel-chapter-203");
        assert_eq!(last.title.as_deref(), Some("Group 2 Chapter 203"));
    }

    #[test]
    fn parses_pages_fixture() {
        let pages = parse_pages(include_str!("fixtures/mangapill_chapter.html")).unwrap();
        assert!(pages.len() > 3);
        assert_eq!(
            pages[0].url,
            "https://cdn.readdetectiveconan.com/file/mangap/8136/20203000/1.jpeg"
        );
        assert_eq!(pages[0].referer.as_deref(), Some("https://mangapill.com/"));
    }

    #[test]
    fn flags_adult_genres() {
        let body = r#"<div>
          <a href="/manga/1/x" class="relative block"><figure><img data-src="https://c/1.jpg"/></figure></a>
          <div><a href="/manga/1/x"><div class="mt-3 font-black">X</div></a>
            <div class="flex"><div class="text-xs bg-card rounded px-1.5">Action</div><div class="text-xs bg-card rounded px-1.5">Ecchi</div></div>
          </div></div>
          <div>
          <a href="/manga/2/y" class="relative block"><figure><img data-src="https://c/2.jpg"/></figure></a>
          <div><a href="/manga/2/y"><div class="mt-3 font-black">Y</div></a>
            <div class="flex"><div class="text-xs bg-card rounded px-1.5">Comedy</div></div>
          </div></div>"#;
        let comics = parse_search(body);
        assert_eq!(comics.len(), 2);
        assert!(comics[0].adult);
        assert!(!comics[1].adult);
        // In the real fixture only the Ecchi-tagged title is flagged.
        let flagged: Vec<String> = parse_search(include_str!("fixtures/mangapill_search.html"))
            .into_iter()
            .filter(|c| c.adult)
            .map(|c| c.title["en"].clone())
            .collect();
        assert_eq!(flagged, ["Futari Solo Camp"]);
    }
}
