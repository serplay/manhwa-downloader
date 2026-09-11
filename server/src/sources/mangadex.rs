//! MangaDex, via its public REST API. Ported from `server-old/Manga/MangaDex.py`.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::Deserialize;

use super::{Ctx, PageUrl, SearchOptions, Source, http};
use crate::{
    error::{AppError, AppResult},
    model::{
        Chapter, Comic, Cover, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume,
        push_chapter, sort_volumes,
    },
};

const API: &str = "https://api.mangadex.org";
const COVERS: &str = "https://uploads.mangadex.org/covers";
const SEARCH_LIMIT: u32 = 30;

pub struct MangaDex {
    meta: SourceMeta,
}

impl Default for MangaDex {
    fn default() -> Self {
        Self::new()
    }
}

impl MangaDex {
    pub fn new() -> Self {
        Self {
            meta: SourceMeta {
                id: 0,
                slug: "mangadex",
                name: "MangaDex",
                base_url: API.to_string(),
                speed: Speed::Fastest,
                languages: vec!["en", "*"],
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

// ---- wire types -----------------------------------------------------------

#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<Manga>,
}

#[derive(Deserialize)]
struct Manga {
    id: String,
    attributes: MangaAttributes,
    #[serde(default)]
    relationships: Vec<Relationship>,
}

#[derive(Deserialize)]
struct MangaAttributes {
    #[serde(default)]
    title: BTreeMap<String, String>,
    #[serde(default, rename = "availableTranslatedLanguages")]
    available_translated_languages: Vec<Option<String>>,
    /// Null, an object, or (rarely) an empty array.
    #[serde(default)]
    links: serde_json::Value,
    #[serde(default, rename = "contentRating")]
    content_rating: Option<String>,
}

#[derive(Deserialize)]
struct Relationship {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    attributes: Option<RelationshipAttributes>,
}

#[derive(Deserialize)]
struct RelationshipAttributes {
    #[serde(rename = "fileName")]
    file_name: Option<String>,
}

#[derive(Deserialize)]
struct AggregateResponse {
    #[serde(default)]
    volumes: MaybeMap<AggregateVolume>,
}

#[derive(Deserialize)]
struct AggregateVolume {
    volume: String,
    #[serde(default)]
    chapters: MaybeMap<AggregateChapter>,
}

#[derive(Deserialize)]
struct AggregateChapter {
    chapter: String,
    id: String,
    #[serde(default, rename = "isUnavailable")]
    is_unavailable: bool,
}

/// MangaDex serializes empty maps as `[]`. Accept both.
#[derive(Deserialize)]
#[serde(untagged)]
enum MaybeMap<T> {
    Map(BTreeMap<String, T>),
    List(Vec<T>),
}

impl<T> Default for MaybeMap<T> {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl<T> MaybeMap<T> {
    fn into_values(self) -> Vec<T> {
        match self {
            Self::Map(m) => m.into_values().collect(),
            Self::List(l) => l,
        }
    }
}

#[derive(Deserialize)]
struct AtHomeResponse {
    #[serde(rename = "baseUrl")]
    base_url: String,
    chapter: AtHomeChapter,
}

#[derive(Deserialize)]
struct AtHomeChapter {
    hash: String,
    #[serde(default)]
    data: Vec<String>,
}

// ---- parsing (pure, fixture-testable) --------------------------------------

pub fn parse_search(body: &str) -> AppResult<Vec<Comic>> {
    let resp: SearchResponse = serde_json::from_str(body).map_err(|e| AppError::Parse {
        site: "mangadex".into(),
        message: e.to_string(),
    })?;
    Ok(resp
        .data
        .into_iter()
        .filter(|m| {
            // Titles with an Amazon link are official-publisher releases that
            // have no downloadable chapters. Same rule as the Python port.
            !m.attributes
                .links
                .as_object()
                .is_some_and(|o| o.contains_key("amz"))
        })
        .map(|m| {
            let cover = m
                .relationships
                .iter()
                .find(|r| r.kind == "cover_art")
                .and_then(|r| r.attributes.as_ref())
                .and_then(|a| a.file_name.as_deref())
                .map(|file| Cover {
                    url: format!("{COVERS}/{}/{file}.256.jpg", m.id),
                    referer: None,
                });
            Comic {
                adult: matches!(
                    m.attributes.content_rating.as_deref(),
                    Some("erotica" | "pornographic")
                ),
                id: m.id,
                title: m.attributes.title,
                cover,
                languages: m
                    .attributes
                    .available_translated_languages
                    .into_iter()
                    .flatten()
                    .collect(),
            }
        })
        .collect())
}

pub fn parse_aggregate(body: &str) -> AppResult<Vec<Volume>> {
    let resp: AggregateResponse = serde_json::from_str(body).map_err(|e| AppError::Parse {
        site: "mangadex".into(),
        message: e.to_string(),
    })?;
    let mut volumes = Vec::new();
    for vol in resp.volumes.into_values() {
        let name = if vol.volume == "none" || vol.volume.is_empty() {
            "Vol 1".to_string()
        } else {
            format!("Vol {}", vol.volume)
        };
        for ch in vol.chapters.into_values() {
            if ch.is_unavailable {
                continue;
            }
            push_chapter(
                &mut volumes,
                &name,
                Chapter {
                    id: ch.id,
                    number: if ch.chapter == "none" {
                        "0".to_string()
                    } else {
                        ch.chapter
                    },
                    title: None,
                    pages: None,
                },
            );
        }
    }
    sort_volumes(&mut volumes);
    Ok(volumes)
}

pub fn parse_at_home(body: &str) -> AppResult<Vec<PageUrl>> {
    let resp: AtHomeResponse = serde_json::from_str(body).map_err(|e| AppError::Parse {
        site: "mangadex".into(),
        message: e.to_string(),
    })?;
    if resp.chapter.data.is_empty() {
        return Err(AppError::Parse {
            site: "mangadex".into(),
            message: "chapter has no pages on MangaDex (external or licensed release)".into(),
        });
    }
    let base = resp.base_url.trim_end_matches('/');
    Ok(resp
        .chapter
        .data
        .iter()
        .map(|file| PageUrl {
            url: format!("{base}/data/{}/{file}", resp.chapter.hash),
            referer: None,
        })
        .collect())
}

// ---- Source impl ----------------------------------------------------------

#[async_trait]
impl Source for MangaDex {
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
        let resp = http::send_with_retry(
            "mangadex",
            || {
                let mut params = vec![
                    ("title", query.to_string()),
                    ("includes[]", "cover_art".to_string()),
                    ("limit", SEARCH_LIMIT.to_string()),
                    ("contentRating[]", "safe".to_string()),
                    ("contentRating[]", "suggestive".to_string()),
                ];
                if opts.include_adult {
                    params.push(("contentRating[]", "erotica".to_string()));
                    params.push(("contentRating[]", "pornographic".to_string()));
                }
                ctx.client.get(format!("{API}/manga")).query(&params)
            },
            3,
        )
        .await?;
        let body = http::ensure_success("mangadex", resp)?
            .text()
            .await
            .map_err(|e| AppError::from_reqwest("mangadex", e))?;
        parse_search(&body)
    }

    async fn chapters(
        &self,
        ctx: &Ctx,
        comic_id: &str,
        lang: Option<&str>,
    ) -> AppResult<Vec<Volume>> {
        let lang = lang.unwrap_or("en");
        let resp = http::send_with_retry(
            "mangadex",
            || {
                ctx.client
                    .get(format!("{API}/manga/{comic_id}/aggregate"))
                    .query(&[("translatedLanguage[]", lang)])
            },
            3,
        )
        .await?;
        let body = http::ensure_success("mangadex", resp)?
            .text()
            .await
            .map_err(|e| AppError::from_reqwest("mangadex", e))?;
        parse_aggregate(&body)
    }

    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        let resp = http::send_with_retry(
            "mangadex",
            || ctx.client.get(format!("{API}/at-home/server/{chapter_id}")),
            3,
        )
        .await?;
        let body = http::ensure_success("mangadex", resp)?
            .text()
            .await
            .map_err(|e| AppError::from_reqwest("mangadex", e))?;
        parse_at_home(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_fixture() {
        let comics = parse_search(include_str!("fixtures/mangadex_search.json")).unwrap();
        assert_eq!(comics.len(), 3);
        let arise = comics
            .iter()
            .find(|c| c.title.get("en").is_some_and(|t| t.contains("Arise")))
            .expect("Solo Leveling: Arise present");
        assert!(arise.languages.iter().any(|l| l == "en"));
        let cover = arise.cover.as_ref().unwrap();
        assert!(
            cover
                .url
                .starts_with("https://uploads.mangadex.org/covers/")
        );
        assert!(cover.url.ends_with(".256.jpg"));
        insta::assert_json_snapshot!(comics);
    }

    #[test]
    fn parses_aggregate_fixture() {
        let volumes = parse_aggregate(include_str!("fixtures/mangadex_aggregate.json")).unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].name, "Vol 1");
        let numbers: Vec<_> = volumes[0]
            .chapters
            .iter()
            .map(|c| c.number.as_str())
            .collect();
        assert_eq!(numbers, ["1", "2", "3"]);
    }

    #[test]
    fn aggregate_accepts_empty_list_form() {
        let volumes = parse_aggregate(r#"{"result":"ok","volumes":[]}"#).unwrap();
        assert!(volumes.is_empty());
    }

    #[test]
    fn skips_official_publisher_titles() {
        let body = r#"{"data":[{"id":"x","attributes":{"title":{"en":"Licensed"},"links":{"amz":"1"}},"relationships":[]}]}"#;
        assert!(parse_search(body).unwrap().is_empty());
    }

    #[test]
    fn at_home_urls() {
        let body = r#"{"baseUrl":"https://cdn.example/","chapter":{"hash":"abc","data":["1.png","2.jpg"]}}"#;
        let pages = parse_at_home(body).unwrap();
        assert_eq!(pages[0].url, "https://cdn.example/data/abc/1.png");
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn flags_adult_content_ratings() {
        let body = r#"{"data":[
          {"id":"1","attributes":{"title":{"en":"A"},"contentRating":"pornographic","links":{}},"relationships":[]},
          {"id":"2","attributes":{"title":{"en":"B"},"contentRating":"erotica","links":{}},"relationships":[]},
          {"id":"3","attributes":{"title":{"en":"C"},"contentRating":"suggestive","links":{}},"relationships":[]},
          {"id":"4","attributes":{"title":{"en":"D"},"links":{}},"relationships":[]}
        ]}"#;
        let comics = parse_search(body).unwrap();
        assert_eq!(
            comics.iter().map(|c| c.adult).collect::<Vec<_>>(),
            [true, true, false, false]
        );
    }
}
