//! Domain types shared by sources, the API layer, and (later) the download pipeline.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// How fast a source usually answers. Mirrors the tiers the old client hardcoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Speed {
    Fastest,
    Fast,
    Slow,
}

/// Which fetch strategy a source needs to get past its front door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum FetchTier {
    /// Plain HTTP with browser-like headers.
    Plain,
    /// Chrome TLS/HTTP2 impersonation.
    Impersonate,
    /// Needs a headless browser to pass a managed challenge. Not built; such
    /// sources are listed but unavailable until the `browser` feature exists.
    Browser,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceCapabilities {
    pub search: bool,
    pub chapters: bool,
    pub download: bool,
    pub needs_browser: bool,
}

/// Static description of a source. The numeric `id` is the legacy identifier the
/// client has always sent (`"0"` to `"10"`); `slug` is the new stable name.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceMeta {
    pub id: u8,
    pub slug: &'static str,
    pub name: &'static str,
    pub base_url: String,
    pub speed: Speed,
    pub languages: Vec<&'static str>,
    pub tier: FetchTier,
    pub capabilities: SourceCapabilities,
}

impl SourceMeta {
    pub fn id_str(&self) -> String {
        self.id.to_string()
    }
}

/// Where a cover image lives and which `Referer` (if any) the host expects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Cover {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Comic {
    /// Source-specific identifier, opaque to the client.
    pub id: String,
    /// Localized titles keyed by language code. `en` when the source has one language.
    pub title: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover: Option<Cover>,
    /// Languages chapters are available in.
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Chapter {
    /// Source-specific identifier, opaque to the client.
    pub id: String,
    /// Chapter label as the source shows it (`"12"`, `"12.5"`, `"Extra"`).
    pub number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<u32>,
}

impl Chapter {
    /// Numeric sort key. Non-numeric labels sort last, keeping their relative order.
    pub fn sort_key(&self) -> f64 {
        parse_leading_number(&self.number).unwrap_or(f64::INFINITY)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Volume {
    /// Display name, e.g. `"Vol 1"`. Sources without volumes use `"Vol 1"`.
    pub name: String,
    pub chapters: Vec<Chapter>,
}

impl Volume {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            chapters: Vec::new(),
        }
    }

    pub fn sort_key(&self) -> f64 {
        parse_leading_number(&self.name).unwrap_or(f64::INFINITY)
    }
}

/// Sort volumes numerically and chapters numerically inside each volume.
pub fn sort_volumes(volumes: &mut [Volume]) {
    volumes.sort_by(|a, b| a.sort_key().total_cmp(&b.sort_key()));
    for v in volumes.iter_mut() {
        v.chapters
            .sort_by(|a, b| a.sort_key().total_cmp(&b.sort_key()));
    }
}

/// Group chapters into volumes by label, creating volumes on demand and
/// preserving first-seen order until `sort_volumes` runs.
pub fn push_chapter(volumes: &mut Vec<Volume>, volume_name: &str, chapter: Chapter) {
    match volumes.iter_mut().find(|v| v.name == volume_name) {
        Some(v) => v.chapters.push(chapter),
        None => {
            let mut v = Volume::named(volume_name);
            v.chapters.push(chapter);
            volumes.push(v);
        }
    }
}

/// Extract the first decimal number from a label like `"Vol 3"` or `"Chapter 12.5 - Title"`.
pub fn parse_leading_number(label: &str) -> Option<f64> {
    let mut start = None;
    let mut end = 0;
    for (i, c) in label.char_indices() {
        let numeric = c.is_ascii_digit() || (c == '.' && start.is_some());
        match (start, numeric) {
            (None, true) => {
                start = Some(i);
                end = i + 1;
            }
            (Some(_), true) => end = i + 1,
            (Some(_), false) => break,
            (None, false) => {}
        }
    }
    let s = &label[start?..end];
    s.trim_end_matches('.').parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Pdf,
    Cbz,
    Cbr,
    Epub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatus {
    Ok,
    Down,
    /// Reachable, but Cloudflare demands a browser challenge this build cannot pass.
    Blocked,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leading_number_parsing() {
        assert_eq!(parse_leading_number("Vol 3"), Some(3.0));
        assert_eq!(parse_leading_number("12.5"), Some(12.5));
        assert_eq!(parse_leading_number("Chapter 7 - The End"), Some(7.0));
        assert_eq!(parse_leading_number("Extra"), None);
        assert_eq!(parse_leading_number("10."), Some(10.0));
    }

    #[test]
    fn volumes_and_chapters_sort_numerically() {
        let mut volumes = vec![Volume::named("Vol 10"), Volume::named("Vol 2")];
        volumes[1].chapters = vec![
            Chapter {
                id: "a".into(),
                number: "10".into(),
                title: None,
                pages: None,
            },
            Chapter {
                id: "b".into(),
                number: "2.5".into(),
                title: None,
                pages: None,
            },
            Chapter {
                id: "c".into(),
                number: "Extra".into(),
                title: None,
                pages: None,
            },
        ];
        sort_volumes(&mut volumes);
        assert_eq!(volumes[0].name, "Vol 2");
        let numbers: Vec<_> = volumes[0]
            .chapters
            .iter()
            .map(|c| c.number.as_str())
            .collect();
        assert_eq!(numbers, ["2.5", "10", "Extra"]);
    }
}
