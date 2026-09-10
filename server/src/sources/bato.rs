//! Bato, via its GraphQL endpoint. Ported from `server-old/Manga/Bato.py`.
//!
//! The base URL is configurable (`BATO_BASE_URL`) because the site rotates
//! mirrors and the main domains sometimes gate non-browser clients.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use super::{Ctx, PageUrl, Source, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        push_chapter, sort_volumes,
    },
};

const SEARCH_QUERY: &str = r#"
query Search($select: Search_Comic_Select) {
  get_search_comic(select: $select) {
    items { data { id name urlCover300 urlPath } }
  }
}"#;

const CHAPTERS_QUERY: &str = r#"
query Chapters($comicId: ID!, $start: Int) {
  get_comic_chapterList(comicId: $comicId, start: $start) {
    data { id volume count_images serial order dname }
  }
}"#;

const IMAGES_QUERY: &str = r#"
query Images($getChapterNodeId: ID!) {
  get_chapterNode(id: $getChapterNodeId) {
    data { imageFile { urlList } }
  }
}"#;

/// Safety cap on chapter-list pagination.
const MAX_PAGES: usize = 200;

pub struct Bato {
    meta: SourceMeta,
    endpoint: String,
}

impl Bato {
    pub fn new(base_url: &str) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            endpoint: format!("{base_url}/ap2/"),
            meta: SourceMeta {
                id: 9,
                slug: "bato",
                name: "Bato",
                base_url,
                speed: Speed::Fastest,
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

    async fn graphql(
        &self,
        ctx: &Ctx,
        query: &str,
        variables: serde_json::Value,
    ) -> AppResult<String> {
        let payload = json!({ "query": query, "variables": variables });
        let resp = http::send_with_retry(
            "bato",
            || {
                ctx.client
                    .post(&self.endpoint)
                    .header("Accept", "application/json")
                    .header("Origin", &self.meta.base_url)
                    .header("Referer", format!("{}/", self.meta.base_url))
                    .json(&payload)
            },
            3,
        )
        .await?;
        http::ensure_success("bato", resp)?
            .text()
            .await
            .map_err(|e| AppError::from_reqwest("bato", e))
    }
}

// ---- wire types -----------------------------------------------------------

#[derive(Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Deserialize)]
struct SearchData {
    get_search_comic: Option<SearchItems>,
}

#[derive(Deserialize)]
struct SearchItems {
    #[serde(default)]
    items: Vec<Node<SearchComic>>,
}

#[derive(Deserialize)]
struct Node<T> {
    data: T,
}

#[derive(Deserialize)]
struct SearchComic {
    id: serde_json::Value,
    name: String,
    #[serde(rename = "urlCover300")]
    url_cover300: Option<String>,
}

#[derive(Deserialize)]
struct ChaptersData {
    #[serde(default)]
    get_comic_chapter_list: Vec<Node<ChapterRow>>,
    // Bato's real field name; serde alias keeps the snake_case struct field readable.
    #[serde(default, rename = "get_comic_chapterList")]
    get_comic_chapter_list_raw: Vec<Node<ChapterRow>>,
}

#[derive(Deserialize)]
struct ChapterRow {
    id: serde_json::Value,
    volume: Option<serde_json::Value>,
    serial: Option<serde_json::Value>,
    order: Option<i64>,
    count_images: Option<u32>,
    dname: Option<String>,
}

#[derive(Deserialize)]
struct ImagesData {
    #[serde(rename = "get_chapterNode")]
    get_chapter_node: Option<Node<ImageFileHolder>>,
}

#[derive(Deserialize)]
struct ImageFileHolder {
    #[serde(rename = "imageFile")]
    image_file: Option<UrlList>,
}

#[derive(Deserialize)]
struct UrlList {
    #[serde(default, rename = "urlList")]
    url_list: Vec<String>,
}

fn scalar_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn decode<T: for<'de> Deserialize<'de>>(body: &str) -> AppResult<T> {
    let env: Envelope<T> = serde_json::from_str(body).map_err(|e| AppError::Parse {
        site: "bato".into(),
        message: e.to_string(),
    })?;
    if let Some(err) = env.errors.first() {
        return Err(AppError::Upstream {
            site: Some("bato".into()),
            message: format!("Bato API error: {}", err.message),
        });
    }
    env.data.ok_or_else(|| AppError::Parse {
        site: "bato".into(),
        message: "response has no data".into(),
    })
}

// ---- parsing (pure, fixture-testable) --------------------------------------

pub fn parse_search(base_url: &str, body: &str) -> AppResult<Vec<Comic>> {
    let data: SearchData = decode(body)?;
    Ok(data
        .get_search_comic
        .map(|s| s.items)
        .unwrap_or_default()
        .into_iter()
        .map(|n| {
            let c = n.data;
            let cover = c.url_cover300.filter(|u| !u.is_empty()).map(|u| Cover {
                url: if u.starts_with("http") {
                    u
                } else {
                    format!("{base_url}{u}")
                },
                referer: Some(format!("{base_url}/")),
            });
            Comic {
                id: scalar_to_string(&c.id),
                title: [("en".to_string(), c.name)].into_iter().collect(),
                cover,
                languages: vec!["en".into()],
            }
        })
        .collect())
}

/// Chapter rows tagged with their volume name, plus the highest `order` seen.
pub type ChapterPage = (Vec<(String, Chapter)>, Option<i64>);

/// Parse one page of the chapter list. Returns the rows and the highest `order`
/// seen, which drives pagination.
pub fn parse_chapter_page(body: &str) -> AppResult<ChapterPage> {
    let data: ChaptersData = decode(body)?;
    let rows = if data.get_comic_chapter_list_raw.is_empty() {
        data.get_comic_chapter_list
    } else {
        data.get_comic_chapter_list_raw
    };
    let mut max_order = None;
    let mut out = Vec::with_capacity(rows.len());
    for n in rows {
        let r = n.data;
        max_order = max_order.max(r.order);
        let volume = match r.volume.as_ref().filter(|v| !v.is_null()) {
            Some(v) => format!("Vol {}", scalar_to_string(v)),
            None => "Vol 1".to_string(),
        };
        let number = r
            .serial
            .as_ref()
            .filter(|v| !v.is_null())
            .map(scalar_to_string)
            .or_else(|| r.order.map(|o| o.to_string()))
            .unwrap_or_else(|| "0".to_string());
        out.push((
            volume,
            Chapter {
                id: scalar_to_string(&r.id),
                number,
                title: r.dname.filter(|t| !t.trim().is_empty()),
                pages: r.count_images,
            },
        ));
    }
    Ok((out, max_order))
}

pub fn parse_images(body: &str) -> AppResult<Vec<PageUrl>> {
    let data: ImagesData = decode(body)?;
    let urls = data
        .get_chapter_node
        .and_then(|n| n.data.image_file)
        .map(|f| f.url_list)
        .unwrap_or_default();
    if urls.is_empty() {
        return Err(AppError::Parse {
            site: "bato".into(),
            message: "chapter has no pages".into(),
        });
    }
    Ok(urls
        .into_iter()
        .map(|url| PageUrl { url, referer: None })
        .collect())
}

// ---- Source impl ----------------------------------------------------------

#[async_trait]
impl Source for Bato {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(&self, ctx: &Ctx, query: &str, _lang: Option<&str>) -> AppResult<Vec<Comic>> {
        let body = self
            .graphql(ctx, SEARCH_QUERY, json!({ "select": { "word": query } }))
            .await?;
        parse_search(&self.meta.base_url, &body)
    }

    async fn chapters(
        &self,
        ctx: &Ctx,
        comic_id: &str,
        _lang: Option<&str>,
    ) -> AppResult<Vec<Volume>> {
        let mut volumes = Vec::new();
        let mut start: i64 = 1;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..MAX_PAGES {
            let body = self
                .graphql(
                    ctx,
                    CHAPTERS_QUERY,
                    json!({ "comicId": comic_id, "start": start }),
                )
                .await?;
            let (rows, max_order) = parse_chapter_page(&body)?;
            let mut progressed = false;
            for (volume, chapter) in rows {
                if seen.insert(chapter.id.clone()) {
                    progressed = true;
                    push_chapter(&mut volumes, &volume, chapter);
                }
            }
            match max_order {
                Some(order) if progressed && order + 1 > start => start = order + 1,
                _ => break,
            }
        }
        sort_volumes(&mut volumes);
        Ok(volumes)
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let body = self
            .graphql(ctx, IMAGES_QUERY, json!({ "getChapterNodeId": chapter_id }))
            .await?;
        parse_images(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bato's public endpoints were gated behind a JS challenge when this port was
    // written, so these fixtures are hand-written to the schema the Python client
    // consumed. Replace with captured responses once a live call succeeds.

    #[test]
    fn parses_search() {
        let body = r#"{"data":{"get_search_comic":{"items":[
            {"data":{"id":"123","name":"Solo Leveling","urlCover300":"/media/cover.jpg","urlPath":"/title/123"}},
            {"data":{"id":456,"name":"Other","urlCover300":null,"urlPath":"/title/456"}}
        ]}}}"#;
        let comics = parse_search("https://bato.si", body).unwrap();
        assert_eq!(comics.len(), 2);
        assert_eq!(comics[0].id, "123");
        assert_eq!(
            comics[0].cover.as_ref().unwrap().url,
            "https://bato.si/media/cover.jpg"
        );
        assert_eq!(comics[1].id, "456");
        assert!(comics[1].cover.is_none());
    }

    #[test]
    fn parses_chapter_page_and_groups_volumes() {
        let body = r#"{"data":{"get_comic_chapterList":[
            {"data":{"id":"c1","volume":null,"count_images":10,"serial":1,"order":1,"dname":"Chapter 1"}},
            {"data":{"id":"c2","volume":2,"count_images":12,"serial":2.5,"order":2,"dname":""}},
            {"data":{"id":"c3","volume":2,"count_images":8,"serial":3,"order":3,"dname":null}}
        ]}}"#;
        let (rows, max_order) = parse_chapter_page(body).unwrap();
        assert_eq!(max_order, Some(3));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "Vol 1");
        assert_eq!(rows[1].0, "Vol 2");
        assert_eq!(rows[1].1.number, "2.5");
        assert_eq!(rows[0].1.title.as_deref(), Some("Chapter 1"));
        assert_eq!(rows[1].1.title, None);
    }

    #[test]
    fn graphql_errors_surface_as_upstream() {
        let body = r#"{"data":null,"errors":[{"message":"boom"}]}"#;
        let err = parse_images(body).unwrap_err();
        assert!(matches!(err, AppError::Upstream { .. }));
    }

    #[test]
    fn parses_images() {
        let body = r#"{"data":{"get_chapterNode":{"data":{"imageFile":{"urlList":["https://a/1.jpg","https://a/2.jpg"]}}}}}"#;
        assert_eq!(parse_images(body).unwrap().len(), 2);
    }
}
