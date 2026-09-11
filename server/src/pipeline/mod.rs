//! Turns downloaded chapter directories into the archive the user asked for.
//!
//! Layout the packagers expect (produced by the task runner):
//! `workdir/<index>_<number>/<page>.<ext>` with one directory per chapter.

pub mod cbr;
pub mod cbz;
pub mod epub;
pub mod images;
pub mod pdf;

use std::path::{Path, PathBuf};

use crate::{error::AppResult, model::Format};

/// A chapter directory ready for packaging.
#[derive(Debug, Clone)]
pub struct ChapterDir {
    pub number: String,
    pub path: PathBuf,
    /// Sorted page files.
    pub pages: Vec<PathBuf>,
}

/// The finished archive.
#[derive(Debug, Clone)]
pub struct Packaged {
    pub path: PathBuf,
    pub content_type: &'static str,
    /// Extension without the dot, used for the download filename.
    pub extension: &'static str,
}

pub fn available_formats(rar_available: bool) -> Vec<&'static str> {
    let mut v = vec!["pdf", "cbz", "epub"];
    if rar_available {
        v.push("cbr");
    }
    v
}

pub fn format_supported(format: Format, rar_available: bool) -> bool {
    format != Format::Cbr || rar_available
}

/// Progress callback: message only. The runner maps it onto the percentage.
pub type Progress<'a> = &'a (dyn Fn(&str) + Send + Sync);

pub async fn package(
    format: Format,
    workdir: &Path,
    chapters: Vec<ChapterDir>,
    comic_title: &str,
    progress: Progress<'_>,
) -> AppResult<Packaged> {
    match format {
        Format::Pdf => pdf::package(workdir, chapters, progress).await,
        Format::Cbz => cbz::package(workdir, chapters, comic_title, progress).await,
        Format::Epub => epub::package(workdir, chapters, comic_title, progress).await,
        Format::Cbr => cbr::package(workdir, chapters, progress).await,
    }
}

/// Filesystem-safe chapter label: keeps letters, digits, dot and dash.
pub fn safe_label(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('_').to_string();
    if cleaned.is_empty() {
        "chapter".to_string()
    } else {
        cleaned
    }
}

/// Unique per-chapter file stem: "Chapter 12", "Chapter 12 (2)" on collision.
pub fn chapter_stem(number: &str, taken: &mut std::collections::HashSet<String>) -> String {
    let base = format!("Chapter {}", number.trim());
    let mut candidate = base.clone();
    let mut n = 2;
    while !taken.insert(candidate.clone()) {
        candidate = format!("{base} ({n})");
        n += 1;
    }
    candidate
}

/// Sanitized download filename (ASCII only) for the `filename=` parameter.
pub fn ascii_filename(title: &str, extension: &str) -> String {
    let stem: String = title
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'))
        .collect();
    let stem = stem.trim().trim_matches('.');
    let stem = if stem.is_empty() { "Chapters" } else { stem };
    format!("{stem}.{extension}")
}

pub(crate) fn blocking_err(e: impl std::fmt::Display) -> crate::error::AppError {
    crate::error::AppError::Internal(anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_safe() {
        assert_eq!(safe_label(" 12.5 "), "12.5");
        assert_eq!(safe_label("Ch/ap:ter"), "Ch_ap_ter");
        assert_eq!(safe_label("***"), "chapter");
    }

    #[test]
    fn stems_dedupe() {
        let mut taken = Default::default();
        assert_eq!(chapter_stem("1", &mut taken), "Chapter 1");
        assert_eq!(chapter_stem("1", &mut taken), "Chapter 1 (2)");
        assert_eq!(chapter_stem("2", &mut taken), "Chapter 2");
    }

    #[test]
    fn ascii_filename_strips_unsafe_chars() {
        assert_eq!(
            ascii_filename("Solo Leveling: Arise", "zip"),
            "Solo Leveling Arise.zip"
        );
        assert_eq!(ascii_filename("ワンピース", "epub"), "Chapters.epub");
    }
}
