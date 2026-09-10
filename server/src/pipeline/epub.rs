//! Single EPUB: one XHTML page per chapter with its images embedded.

use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ZipLibrary};

use super::{ChapterDir, Packaged, Progress, ascii_filename, blocking_err, chapter_stem};
use crate::error::AppResult;

const CSS: &str = "body { margin: 0; text-align: center; } img { max-width: 100%; height: auto; display: block; margin: 0 auto; } h1 { font-size: 1.2em; margin: 0.5em 0; }";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn build(chapters: &[ChapterDir], title: &str, out: &Path) -> anyhow::Result<()> {
    let mut book = EpubBuilder::new(ZipLibrary::new()?)?;
    book.epub_version(EpubVersion::V30);
    book.metadata("title", title)?;
    book.metadata("lang", "en")?;
    book.metadata("generator", "manhwa-downloader")?;
    book.stylesheet(CSS.as_bytes())?;

    let mut image_counter = 0usize;
    let mut taken = HashSet::new();
    for (i, chapter) in chapters.iter().enumerate() {
        let heading = chapter_stem(&chapter.number, &mut taken);
        let mut html = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>{0}</title><link rel="stylesheet" type="text/css" href="stylesheet.css"/></head><body><h1>{0}</h1>"#,
            xml_escape(&heading)
        );
        for page in &chapter.pages {
            image_counter += 1;
            let ext = page.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
            let mime = if ext == "png" {
                "image/png"
            } else {
                "image/jpeg"
            };
            let name = format!("images/p{image_counter:05}.{ext}");
            book.add_resource(&name, File::open(page)?, mime)?;
            html.push_str(&format!(
                r#"<p><img src="{name}" alt="Page {image_counter}"/></p>"#
            ));
        }
        html.push_str("</body></html>");
        book.add_content(
            EpubContent::new(format!("chapter_{:04}.xhtml", i + 1), html.as_bytes())
                .title(heading)
                .level(1),
        )?;
    }
    book.generate(File::create(out)?)?;
    Ok(())
}

pub async fn package(
    workdir: &Path,
    chapters: Vec<ChapterDir>,
    comic_title: &str,
    progress: Progress<'_>,
) -> AppResult<Packaged> {
    progress("Creating ePUB...");
    let out: PathBuf = workdir.join(ascii_filename(comic_title, "epub"));
    let out_clone = out.clone();
    let title = comic_title.to_string();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        build(&chapters, &title, &out_clone)?;
        for c in &chapters {
            let _ = std::fs::remove_dir_all(&c.path);
        }
        Ok(())
    })
    .await
    .map_err(blocking_err)?
    .map_err(blocking_err)?;
    Ok(Packaged {
        path: out,
        content_type: "application/epub+zip",
        extension: "epub",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_an_epub_with_two_chapters() {
        let dir = tempfile::tempdir().unwrap();
        let mut chapters = Vec::new();
        for n in 1..=2 {
            let cdir = dir.path().join(format!("{n}"));
            std::fs::create_dir_all(&cdir).unwrap();
            let page = cdir.join("0001.jpg");
            std::fs::write(&page, super::super::images::PLACEHOLDER).unwrap();
            chapters.push(ChapterDir {
                number: n.to_string(),
                path: cdir,
                pages: vec![page],
            });
        }
        let out = dir.path().join("book.epub");
        build(&chapters, "Test & Title", &out).unwrap();
        let file = File::open(&out).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        assert!(names.iter().any(|n| n == "mimetype"));
        assert!(names.iter().any(|n| n.ends_with("chapter_0001.xhtml")));
        assert!(names.iter().any(|n| n.ends_with("chapter_0002.xhtml")));
        assert!(names.iter().any(|n| n.contains("images/p00002.jpg")));
    }
}
