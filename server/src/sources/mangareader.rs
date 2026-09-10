//! WordPress "MangaReader" (Themesia) theme sites. Yakshascans, legacy source
//! id 2, now redirects to `ravenscans.org`, which runs this theme rather than
//! Madara, so the old Yakshascans selectors no longer apply.

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

#[derive(Debug, Clone)]
pub struct MangaReaderSite {
    pub id: u8,
    pub slug: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    pub speed: Speed,
}

pub const RAVENSCANS: MangaReaderSite = MangaReaderSite {
    id: 2,
    slug: "ravenscans",
    name: "Raven Scans",
    base_url: "https://ravenscans.org",
    speed: Speed::Fast,
};

pub struct MangaReader {
    meta: SourceMeta,
    site: MangaReaderSite,
}

impl MangaReader {
    pub fn new(site: MangaReaderSite) -> Self {
        Self {
            meta: SourceMeta {
                id: site.id,
                slug: site.slug,
                name: site.name,
                base_url: site.base_url.to_string(),
                speed: site.speed,
                languages: vec!["en"],
                tier: FetchTier::Plain,
                capabilities: SourceCapabilities {
                    search: true,
                    chapters: true,
                    download: true,
                    needs_browser: false,
                },
            },
            site,
        }
    }

    fn referer(&self) -> String {
        format!("{}/", self.site.base_url)
    }
}

pub fn parse_search(base_url: &str, body: &str) -> Vec<Comic> {
    let doc = Html::parse_document(body);
    let item = sel(".listupd .bsx > a[href]");
    let title_sel = sel(".tt");
    let img = sel("img");
    let mut seen = std::collections::HashSet::new();
    doc.select(&item)
        .filter_map(|a| {
            let href = a.value().attr("href")?;
            let id = path_after(href, "/manga/")?;
            if !seen.insert(id.clone()) {
                return None;
            }
            let title = a
                .value()
                .attr("title")
                .map(collapse_ws)
                .filter(|t| !t.is_empty())
                .or_else(|| a.select(&title_sel).next().map(text))
                .filter(|t| !t.is_empty())?;
            let cover = a.select(&img).next().and_then(image_src).map(|url| Cover {
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

pub fn parse_chapters(body: &str) -> Vec<Volume> {
    let doc = Html::parse_document(body);
    let item = sel("#chapterlist li");
    let link = sel("a[href]");
    let num = sel(".chapternum");
    let mut vol = Volume::named("Vol 1");
    for li in doc.select(&item) {
        let Some(a) = li.select(&link).next() else {
            continue;
        };
        let Some(id) = a.value().attr("href").and_then(last_path_segment) else {
            continue;
        };
        let label = li.select(&num).next().map(text).unwrap_or_else(|| text(a));
        let number = li
            .value()
            .attr("data-num")
            .map(collapse_ws)
            .filter(|n| !n.is_empty())
            .map(|n| chapter_number(&n))
            .unwrap_or_else(|| chapter_number(&label));
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

pub fn parse_pages(site_slug: &str, base_url: &str, body: &str) -> AppResult<Vec<PageUrl>> {
    let referer = Some(format!("{base_url}/"));
    // Preferred: the reader config the theme injects as `ts_reader.run({...})`.
    if let Some(start) = body.find("ts_reader.run(")
        && let Some(json) = json_object_at(body, start)
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(json)
    {
        let images: Vec<String> = v["sources"]
            .as_array()
            .and_then(|s| s.first())
            .and_then(|s| s["images"].as_array())
            .map(|imgs| {
                imgs.iter()
                    .filter_map(|i| i.as_str())
                    .map(|u| absolutize(base_url, u))
                    .collect()
            })
            .unwrap_or_default();
        if !images.is_empty() {
            return Ok(images
                .into_iter()
                .map(|url| PageUrl {
                    url,
                    referer: referer.clone(),
                })
                .collect());
        }
    }
    // Fallback: the `<noscript>` image list inside the reader area. The parser
    // treats noscript content as text, so the wrapper tags are removed first.
    let unwrapped = body.replace("<noscript>", "").replace("</noscript>", "");
    let doc = Html::parse_document(&unwrapped);
    let img = sel("#readerarea img");
    let pages: Vec<PageUrl> = doc
        .select(&img)
        .filter_map(image_src)
        .map(|url| PageUrl {
            url: absolutize(base_url, &url),
            referer: referer.clone(),
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
impl Source for MangaReader {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let url = http::with_query(&format!("{}/", self.site.base_url), &[("s", query)]);
        let body = ctx
            .fetcher
            .get(url)
            .source(self.site.slug)
            .referer(&self.referer())
            .text()
            .await?;
        Ok(parse_search(self.site.base_url, &body))
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let body = ctx
            .fetcher
            .get(format!("{}/manga/{comic_id}/", self.site.base_url))
            .source(self.site.slug)
            .referer(&self.referer())
            .text()
            .await?;
        Ok(parse_chapters(&body))
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let body = ctx
            .fetcher
            .get(format!("{}/{chapter_id}/", self.site.base_url))
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
    fn parses_search_fixture() {
        let comics = parse_search(
            RAVENSCANS.base_url,
            include_str!("fixtures/ravenscans_search.html"),
        );
        assert!(comics.len() >= 2);
        assert_eq!(
            comics[0].id,
            "solo-dungeon-im-going-to-enjoy-my-weekends-to-the-fullest"
        );
        assert_eq!(
            comics[0].title["en"],
            "Solo Dungeon: I’m Going to Enjoy My Weekends to the Fullest"
        );
        assert!(
            comics[0]
                .cover
                .as_ref()
                .unwrap()
                .url
                .contains("/wp-content/uploads/")
        );
    }

    #[test]
    fn parses_chapters_fixture() {
        let volumes = parse_chapters(include_str!("fixtures/ravenscans_series.html"));
        let chapters = &volumes[0].chapters;
        assert_eq!(chapters.len(), 11);
        assert_eq!(chapters[0].number, "1");
        assert_eq!(chapters[10].number, "11");
        assert_eq!(
            chapters[10].id,
            "solo-dungeon-im-going-to-enjoy-my-weekends-to-the-fullest-chapter-11"
        );
    }

    #[test]
    fn parses_pages_fixture() {
        let pages = parse_pages(
            "ravenscans",
            RAVENSCANS.base_url,
            include_str!("fixtures/ravenscans_chapter.html"),
        )
        .unwrap();
        assert!(pages.len() > 5);
        assert_eq!(
            pages[0].url,
            "https://cdn4.ravenscans.org/solo-dungeon-im-going-to-enjoy-my-weekends-to-the-fullest/chapter-11/0.webp"
        );
    }

    #[test]
    fn falls_back_to_noscript_images() {
        let body = r#"<div id="readerarea"><noscript><img src="https://c/1.webp"><img src="https://c/2.webp"></noscript></div>"#;
        let pages = parse_pages("ravenscans", "https://r", body).unwrap();
        assert_eq!(pages.len(), 2);
    }
}
