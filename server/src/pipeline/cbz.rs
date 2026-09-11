//! One `.cbz` per chapter with a `ComicInfo.xml`, all wrapped in `Chapters.zip`.

use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use super::{ChapterDir, Packaged, Progress, blocking_err, chapter_stem};
use crate::error::AppResult;

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Minimal ComicInfo.xml (Anansi schema) understood by every mainstream reader.
pub fn comic_info(series: &str, number: &str, index: usize, page_count: usize) -> String {
    let mut pages = String::new();
    for i in 0..page_count {
        let kind = if i == 0 { r#" Type="FrontCover""# } else { "" };
        pages.push_str(&format!(r#"    <Page Image="{i}"{kind} />"#));
        pages.push('\n');
    }
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<ComicInfo xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
  <Title>Chapter {number}</Title>
  <Series>{series}</Series>
  <Number>{number}</Number>
  <Volume>{index}</Volume>
  <PageCount>{page_count}</PageCount>
  <LanguageISO>en</LanguageISO>
  <Format>Web Comic</Format>
  <Manga>YesAndRightToLeft</Manga>
  <Pages>
{pages}  </Pages>
</ComicInfo>
"#,
        number = xml_escape(number),
        series = xml_escape(series),
    )
}

fn chapter_cbz(chapter: &ChapterDir, series: &str, index: usize) -> anyhow::Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("ComicInfo.xml", deflated)?;
    zip.write_all(comic_info(series, &chapter.number, index, chapter.pages.len()).as_bytes())?;
    for (i, page) in chapter.pages.iter().enumerate() {
        let ext = page.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
        zip.start_file(format!("{:04}.{ext}", i + 1), stored)?;
        zip.write_all(&std::fs::read(page)?)?;
    }
    Ok(zip.finish()?.into_inner())
}

pub async fn package(
    workdir: &Path,
    chapters: Vec<ChapterDir>,
    comic_title: &str,
    progress: Progress<'_>,
) -> AppResult<Packaged> {
    progress("Creating CBZ...");
    let out: PathBuf = workdir.join("Chapters.zip");
    let out_clone = out.clone();
    let series = comic_title.to_string();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let mut zip = ZipWriter::new(File::create(&out_clone)?);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut taken = HashSet::new();
        for (i, chapter) in chapters.iter().enumerate() {
            let cbz = chapter_cbz(chapter, &series, i + 1)?;
            zip.start_file(
                format!("{}.cbz", chapter_stem(&chapter.number, &mut taken)),
                opts,
            )?;
            zip.write_all(&cbz)?;
            let _ = std::fs::remove_dir_all(&chapter.path);
        }
        zip.finish()?;
        Ok(())
    })
    .await
    .map_err(blocking_err)?
    .map_err(blocking_err)?;
    Ok(Packaged {
        path: out,
        content_type: "application/zip",
        extension: "zip",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comic_info_is_escaped_and_counts_pages() {
        let xml = comic_info("A & B", "12.5", 3, 2);
        assert!(xml.contains("<Series>A &amp; B</Series>"));
        assert!(xml.contains("<Number>12.5</Number>"));
        assert!(xml.contains("<PageCount>2</PageCount>"));
        assert!(xml.contains(r#"<Page Image="0" Type="FrontCover" />"#));
    }
}
