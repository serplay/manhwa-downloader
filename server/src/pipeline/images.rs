//! Download the page images of one chapter with bounded concurrency, verify
//! them, and normalize exotic formats so the packagers only see JPEG and PNG.

use std::{
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::{StreamExt, stream};
use image::{ImageFormat, ImageReader};
use tokio::io::AsyncWriteExt;

use crate::{
    error::AppResult,
    model::FetchTier,
    sources::{PageUrl, http::Fetcher},
};

/// Pages narrower or shorter than this are treated as broken (ad pixels, error tiles).
pub const MIN_DIMENSION: u32 = 72;
const JPEG_QUALITY: u8 = 92;

/// Shown in place of a page that could not be fetched, so page order survives.
pub static PLACEHOLDER: &[u8] = include_bytes!("../../assets/corrupt.jpg");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stored {
    Jpeg,
    Png,
}

impl Stored {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

/// Decide what to write for a fetched image. Runs on a blocking thread.
/// Returns `None` when the image is undecodable or too small.
pub fn normalize(bytes: Vec<u8>) -> Option<(Stored, Vec<u8>)> {
    let reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .ok()?;
    let format = reader.format()?;
    let (w, h) = reader.into_dimensions().ok()?;
    if w < MIN_DIMENSION || h < MIN_DIMENSION {
        return None;
    }
    match format {
        ImageFormat::Jpeg => Some((Stored::Jpeg, bytes)),
        ImageFormat::Png => Some((Stored::Png, bytes)),
        _ => {
            let img = image::load_from_memory(&bytes).ok()?.to_rgb8();
            let mut out = Cursor::new(Vec::with_capacity(bytes.len()));
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
            enc.encode_image(&img).ok()?;
            Some((Stored::Jpeg, out.into_inner()))
        }
    }
}

async fn fetch(fetcher: &Fetcher, tier: FetchTier, page: &PageUrl) -> AppResult<Vec<u8>> {
    let mut req = fetcher
        .get(&page.url)
        .tier(tier)
        .source("image host")
        .accept_images();
    if let Some(r) = &page.referer {
        req = req.referer(r);
    }
    Ok(req.bytes().await?.to_vec())
}

/// Download all pages into `dir` as `0001.jpg`, `0002.png`, ... Returns the
/// sorted file list. Broken pages get the placeholder; a chapter with zero
/// usable pages is an error.
pub async fn download_chapter(
    fetcher: &Arc<Fetcher>,
    tier: FetchTier,
    pages: &[PageUrl],
    dir: &Path,
    concurrency: usize,
) -> AppResult<Vec<PathBuf>> {
    tokio::fs::create_dir_all(dir).await?;
    let dir: Arc<Path> = Arc::from(dir);
    let mut results: Vec<(usize, PathBuf, bool)> = stream::iter(pages.iter().cloned().enumerate())
        .map(|(i, page)| {
            let fetcher = Arc::clone(fetcher);
            let dir = Arc::clone(&dir);
            async move {
                let fetched = match fetch(&fetcher, tier, &page).await {
                    Ok(bytes) => tokio::task::spawn_blocking(move || normalize(bytes))
                        .await
                        .ok()
                        .flatten(),
                    Err(e) => {
                        tracing::warn!(url = %page.url, error = %e, "page fetch failed");
                        None
                    }
                };
                let (stored, bytes, ok) = match fetched {
                    Some((s, b)) => (s, b, true),
                    None => (Stored::Jpeg, PLACEHOLDER.to_vec(), false),
                };
                let path = dir.join(format!("{:04}.{}", i + 1, stored.extension()));
                let mut f = tokio::fs::File::create(&path).await?;
                f.write_all(&bytes).await?;
                f.flush().await?;
                Ok::<_, std::io::Error>((i, path, ok))
            }
        })
        .buffer_unordered(concurrency.max(1))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;
    results.sort_by_key(|(i, _, _)| *i);
    let usable = results.iter().filter(|(_, _, ok)| *ok).count();
    if usable == 0 {
        return Err(crate::error::AppError::Upstream {
            site: None,
            message: "none of the chapter's pages could be downloaded".into(),
        });
    }
    if usable < results.len() {
        tracing::warn!(
            broken = results.len() - usable,
            total = results.len(),
            "some pages were replaced by the placeholder"
        );
    }
    Ok(results.into_iter().map(|(_, p, _)| p).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([10, 20, 30]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn keeps_png_and_jpeg_as_is() {
        let bytes = png(100, 100);
        let (kind, out) = normalize(bytes.clone()).unwrap();
        assert_eq!(kind, Stored::Png);
        assert_eq!(out, bytes);
        let (kind, _) = normalize(PLACEHOLDER.to_vec()).unwrap();
        assert_eq!(kind, Stored::Jpeg);
    }

    #[test]
    fn rejects_tiny_and_garbage() {
        assert!(normalize(png(10, 300)).is_none());
        assert!(normalize(b"not an image".to_vec()).is_none());
    }

    #[test]
    fn converts_webp_to_jpeg() {
        let img = image::RgbImage::from_pixel(120, 90, image::Rgb([200, 100, 50]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::WebP).unwrap();
        let (kind, bytes) = normalize(out.into_inner()).unwrap();
        assert_eq!(kind, Stored::Jpeg);
        assert_eq!(image::guess_format(&bytes).unwrap(), ImageFormat::Jpeg);
    }
}
