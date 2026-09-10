//! Mangahere, scraped natively. Search and chapter lists are plain HTML; page
//! images hide behind Dean Edwards "p,a,c,k,e,r" packed scripts, unpacked here
//! without a JavaScript engine. Replaces the Consumet dependency.

use async_trait::async_trait;
use regex::Regex;
use scraper::Html;
use std::sync::LazyLock;

use super::{Ctx, PageUrl, Source, html::*, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        parse_leading_number, push_chapter, sort_volumes,
    },
};

const BASE: &str = "https://www.mangahere.cc";
const SLUG: &str = "mangahere";
/// Safety cap on `chapterfun.ashx` round trips for one chapter.
const MAX_PAGE_CALLS: usize = 200;

pub struct Mangahere {
    meta: SourceMeta,
}

impl Default for Mangahere {
    fn default() -> Self {
        Self::new()
    }
}

impl Mangahere {
    pub fn new() -> Self {
        Self {
            meta: SourceMeta {
                id: 7,
                slug: SLUG,
                name: "Mangahere",
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

// ---- packer -----------------------------------------------------------------

static PACKED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\}\('(.*?)',\s*(\d+),\s*(\d+),\s*'(.*?)'\.split\('\|'\)").unwrap()
});

/// The packer's radix encoder: `e(c)` in the original bootstrap.
fn encode(c: usize, a: usize) -> String {
    let prefix = if c < a {
        String::new()
    } else {
        encode(c / a, a)
    };
    let r = c % a;
    let ch = if r > 35 {
        char::from_u32((r + 29) as u32).unwrap_or('?')
    } else {
        std::char::from_digit(r as u32, 36).unwrap_or('?')
    };
    format!("{prefix}{ch}")
}

fn unpack(payload: &str, a: usize, c: usize, keywords: &[&str]) -> String {
    let payload = payload.replace("\\'", "'").replace("\\\\", "\\");
    let mut dict = std::collections::HashMap::new();
    for i in 0..c {
        if let Some(k) = keywords.get(i).filter(|k| !k.is_empty()) {
            dict.insert(encode(i, a), *k);
        }
    }
    let mut out = String::with_capacity(payload.len() * 2);
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            out.push_str(dict.get(word.as_str()).copied().unwrap_or(word.as_str()));
            word.clear();
        }
    };
    for ch in payload.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

/// Unpack every `eval(function(p,a,c,k,e,d){...})` block in `src`.
pub fn unpack_all(src: &str) -> Vec<String> {
    PACKED
        .captures_iter(src)
        .filter_map(|cap| {
            let a: usize = cap[2].parse().ok()?;
            let c: usize = cap[3].parse().ok()?;
            let keywords: Vec<&str> = cap[4].split('|').collect();
            Some(unpack(&cap[1], a, c, &keywords))
        })
        .collect()
}

// ---- parsing ----------------------------------------------------------------

pub fn parse_search(body: &str) -> Vec<Comic> {
    let doc = Html::parse_document(body);
    let item = sel("ul.manga-list-4-list > li");
    let link = sel(r#"a[href^="/manga/"]"#);
    let title = sel("p.manga-list-4-item-title a");
    let cover = sel("img.manga-list-4-cover");
    doc.select(&item)
        .filter_map(|li| {
            let a = li.select(&link).next()?;
            let id = path_after(a.value().attr("href")?, "/manga/")?;
            let title = li
                .select(&title)
                .next()
                .map(text)
                .or_else(|| a.value().attr("title").map(collapse_ws))
                .filter(|t| !t.is_empty())?;
            let cover = li.select(&cover).next().and_then(image_src).map(|u| Cover {
                url: absolutize(BASE, &u),
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

/// `"Vol.01 Ch.003"` -> (`"Vol 1"`, `"3"`); `"Ch.202"` -> (`"Vol 1"`, `"202"`).
fn split_label(label: &str) -> (String, String) {
    let lower = label.to_ascii_lowercase();
    let volume = lower
        .find("vol")
        .and_then(|i| parse_leading_number(&label[i + 3..]))
        .map(|v| format!("Vol {}", fmt_number(v)))
        .unwrap_or_else(|| "Vol 1".to_string());
    (volume, chapter_number(label))
}

pub fn parse_chapters(body: &str) -> Vec<Volume> {
    let doc = Html::parse_document(body);
    let link = sel("ul.detail-main-list li a[href]");
    let label_sel = sel("p.title3");
    let mut volumes = Vec::new();
    for a in doc.select(&link) {
        let href = a.value().attr("href").unwrap_or_default();
        let Some(rest) = path_after(href, "/manga/") else {
            continue;
        };
        // "solo_leveling/c202/1.html" -> "solo_leveling/c202"
        let id = rest
            .rsplit_once('/')
            .map(|(head, _)| head.to_string())
            .unwrap_or(rest);
        let label = a
            .select(&label_sel)
            .next()
            .map(text)
            .or_else(|| a.value().attr("title").map(collapse_ws))
            .unwrap_or_default();
        let (volume, number) = split_label(&label);
        push_chapter(
            &mut volumes,
            &volume,
            Chapter {
                id,
                number,
                title: None,
                pages: None,
            },
        );
    }
    sort_volumes(&mut volumes);
    volumes
}

static NEW_IMGS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"newImgs\s*=\s*\[([^\]]*)\]").unwrap());
static PIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"var pix\s*=\s*"([^"]*)""#).unwrap());
static PVALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var pvalue\s*=\s*\[([^\]]*)\]").unwrap());
static CHAPTER_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var chapterid\s*=\s*(\d+)").unwrap());
static IMAGE_COUNT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"var imagecount\s*=\s*(\d+)").unwrap());
static DM5_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"id="dm5_key"[^>]*value="([^"]*)""#).unwrap());

fn quoted_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().trim_matches(|c| c == '\'' || c == '"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Reader metadata needed to page through `chapterfun.ashx`.
#[derive(Debug, PartialEq, Eq)]
pub struct ReaderInfo {
    pub chapter_id: String,
    pub image_count: usize,
    pub key: String,
}

pub fn parse_reader_info(body: &str) -> Option<ReaderInfo> {
    Some(ReaderInfo {
        chapter_id: CHAPTER_ID.captures(body)?[1].to_string(),
        image_count: IMAGE_COUNT.captures(body)?[1].parse().ok()?,
        key: DM5_KEY
            .captures(body)
            .map(|c| c[1].to_string())
            .unwrap_or_default(),
    })
}

/// Full page list embedded in the chapter HTML (webtoon-style chapters).
pub fn parse_embedded_pages(body: &str) -> Vec<String> {
    unpack_all(body)
        .iter()
        .find_map(|js| NEW_IMGS.captures(js).map(|c| quoted_list(&c[1])))
        .unwrap_or_default()
        .into_iter()
        .map(|u| absolutize(BASE, &u))
        .collect()
}

/// Pages returned by one `chapterfun.ashx` call (usually two).
pub fn parse_chapterfun(body: &str) -> Vec<String> {
    unpack_all(body)
        .iter()
        .find_map(|js| {
            let pix = PIX.captures(js)?[1].to_string();
            let values = quoted_list(&PVALUE.captures(js)?[1]);
            Some(
                values
                    .into_iter()
                    .map(|v| absolutize(BASE, &format!("{pix}{v}")))
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_default()
}

fn to_pages(urls: Vec<String>) -> Vec<PageUrl> {
    urls.into_iter()
        .map(|url| PageUrl {
            url,
            referer: referer(),
        })
        .collect()
}

#[async_trait]
impl Source for Mangahere {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let url = http::with_query(&format!("{BASE}/search"), &[("title", query)]);
        let body = ctx.fetcher.get(url).source(SLUG).text().await?;
        Ok(parse_search(&body))
    }

    async fn chapters(&self, ctx: &Ctx, comic_id: &str, _: Option<&str>) -> AppResult<Vec<Volume>> {
        let body = ctx
            .fetcher
            .get(format!("{BASE}/manga/{comic_id}/"))
            .source(SLUG)
            .header(http::header::COOKIE, "isAdult=1")
            .text()
            .await?;
        Ok(parse_chapters(&body))
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let page_url = format!("{BASE}/manga/{chapter_id}/1.html");
        let body = ctx
            .fetcher
            .get(&page_url)
            .source(SLUG)
            .header(http::header::COOKIE, "isAdult=1")
            .text()
            .await?;
        let embedded = parse_embedded_pages(&body);
        if !embedded.is_empty() {
            return Ok(to_pages(embedded));
        }
        let info = parse_reader_info(&body).ok_or_else(|| AppError::Parse {
            site: SLUG.into(),
            message: "reader page has no chapter metadata".into(),
        })?;
        let mut urls: Vec<String> = Vec::with_capacity(info.image_count);
        let mut page = 1usize;
        let mut calls = 0usize;
        while urls.len() < info.image_count && calls < MAX_PAGE_CALLS {
            calls += 1;
            let url = http::with_query(
                &format!("{BASE}/manga/{chapter_id}/chapterfun.ashx"),
                &[
                    ("cid", info.chapter_id.as_str()),
                    ("page", &page.to_string()),
                    ("key", info.key.as_str()),
                ],
            );
            let js = ctx
                .fetcher
                .get(url)
                .source(SLUG)
                .referer(&page_url)
                .header(http::header::COOKIE, "isAdult=1")
                .xhr()
                .text()
                .await?;
            let batch = parse_chapterfun(&js);
            if batch.is_empty() {
                break;
            }
            let before = urls.len();
            for u in batch {
                if !urls.contains(&u) {
                    urls.push(u);
                }
            }
            if urls.len() == before {
                break;
            }
            page = urls.len() + 1;
        }
        if urls.is_empty() {
            return Err(AppError::Parse {
                site: SLUG.into(),
                message: "could not resolve any page image".into(),
            });
        }
        Ok(to_pages(urls))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packer_roundtrip() {
        // Packed form of `var a=1;var bb=2;`: every token, digits included, goes
        // through the keyword table, which is why the literals are listed too.
        let js = "eval(function(p,a,c,k,e,d){}('1 0=3;1 2=4;',5,5,'a|var|bb|1|2'.split('|'),0,{}))";
        assert_eq!(unpack_all(js), vec!["var a=1;var bb=2;".to_string()]);
    }

    #[test]
    fn packer_radix_encoding() {
        assert_eq!(encode(0, 62), "0");
        assert_eq!(encode(10, 62), "a");
        assert_eq!(encode(36, 62), "A");
        assert_eq!(encode(62, 62), "10");
    }

    #[test]
    fn parses_search_fixture() {
        let comics = parse_search(include_str!("fixtures/mangahere_search.html"));
        assert!(!comics.is_empty());
        assert_eq!(comics[0].id, "solo_leveling");
        assert_eq!(comics[0].title["en"], "Solo Leveling");
        assert!(
            comics[0]
                .cover
                .as_ref()
                .unwrap()
                .url
                .contains("fmcdn.mangahere.com")
        );
    }

    #[test]
    fn parses_chapters_fixture() {
        let volumes = parse_chapters(include_str!("fixtures/mangahere_series.html"));
        let all: Vec<&Chapter> = volumes.iter().flat_map(|v| &v.chapters).collect();
        assert!(all.len() > 200, "got {}", all.len());
        assert_eq!(all[0].id, "solo_leveling/c001");
        assert_eq!(all[0].number, "1");
        assert!(
            all.iter()
                .any(|c| c.id == "solo_leveling/c202" && c.number == "202")
        );
    }

    #[test]
    fn label_splitting() {
        assert_eq!(split_label("Vol.01 Ch.003"), ("Vol 1".into(), "3".into()));
        assert_eq!(split_label("Ch.202"), ("Vol 1".into(), "202".into()));
        assert_eq!(
            split_label("Vol.2 Ch.10.5"),
            ("Vol 2".into(), "10.5".into())
        );
    }

    #[test]
    fn parses_embedded_pages_fixture() {
        let body = include_str!("fixtures/mangahere_chapter.html");
        let pages = parse_embedded_pages(body);
        assert_eq!(pages.len(), 14);
        assert!(pages[0].starts_with("https://zjcdn.mangahere.org/store/manga/30829/001.0/"));
        let info = parse_reader_info(body).unwrap();
        assert_eq!(info.chapter_id, "562265");
        assert_eq!(info.image_count, 14);
    }

    #[test]
    fn parses_chapterfun_fixture() {
        let pages = parse_chapterfun(include_str!("fixtures/mangahere_chapterfun.js"));
        assert_eq!(pages.len(), 2);
        assert_eq!(
            pages[0],
            "https://zjcdn.mangahere.org/store/manga/30829/001.0/compressed/h20181105_144325_927.jpg"
        );
    }
}
