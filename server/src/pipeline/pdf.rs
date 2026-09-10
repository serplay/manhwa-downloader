//! One PDF per chapter, JPEG pages embedded losslessly (DCTDecode), then all
//! PDFs zipped as `Chapters.zip`. Same output the Python `img2pdf` path produced.

use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};

use lopdf::{
    Document, Object, Stream,
    content::{Content, Operation},
    dictionary,
};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use super::{ChapterDir, Packaged, Progress, blocking_err, chapter_stem};
use crate::error::AppResult;

/// Basic JPEG header facts read straight from the SOF marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegInfo {
    pub width: u32,
    pub height: u32,
    pub components: u8,
}

/// Parse width/height/components from a JPEG's Start-Of-Frame marker.
pub fn jpeg_info(bytes: &[u8]) -> Option<JpegInfo> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }
    let mut i = 2;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0xFF {
            i += 1;
            continue;
        }
        // Standalone markers without a length.
        if matches!(marker, 0xD8 | 0x01 | 0xD0..=0xD7) {
            i += 2;
            continue;
        }
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        let is_sof = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_sof {
            if i + 9 >= bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            let width = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
            let components = bytes[i + 9];
            return Some(JpegInfo {
                width,
                height,
                components,
            });
        }
        if marker == 0xDA {
            // Start of scan: no SOF seen before image data, give up.
            return None;
        }
        i += 2 + len;
    }
    None
}

/// Load a page as JPEG bytes: JPEG passes through, PNG is re-encoded.
fn page_as_jpeg(path: &Path) -> anyhow::Result<(Vec<u8>, JpegInfo)> {
    let bytes = std::fs::read(path)?;
    if let Some(info) = jpeg_info(&bytes) {
        return Ok((bytes, info));
    }
    let img = image::load_from_memory(&bytes)?.to_rgb8();
    let (width, height) = img.dimensions();
    let mut out = Cursor::new(Vec::new());
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 92).encode_image(&img)?;
    Ok((
        out.into_inner(),
        JpegInfo {
            width,
            height,
            components: 3,
        },
    ))
}

/// Build a PDF from JPEG pages. Pure and synchronous, easy to test.
pub fn build_pdf(pages: &[(Vec<u8>, JpegInfo)]) -> anyhow::Result<Vec<u8>> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids: Vec<Object> = Vec::with_capacity(pages.len());

    for (bytes, info) in pages {
        let mut image_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => info.width as i64,
            "Height" => info.height as i64,
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode",
        };
        match info.components {
            1 => image_dict.set("ColorSpace", "DeviceGray"),
            4 => {
                image_dict.set("ColorSpace", "DeviceCMYK");
                // Adobe CMYK JPEGs are stored inverted.
                image_dict.set(
                    "Decode",
                    vec![
                        1.into(),
                        0.into(),
                        1.into(),
                        0.into(),
                        1.into(),
                        0.into(),
                        1.into(),
                        0.into(),
                    ],
                );
            }
            _ => image_dict.set("ColorSpace", "DeviceRGB"),
        }
        let image_id = doc.add_object(Stream::new(image_dict, bytes.clone()));

        let (w, h) = (info.width as i64, info.height as i64);
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![w.into(), 0.into(), 0.into(), h.into(), 0.into(), 0.into()],
                ),
                Operation::new("Do", vec![Object::Name(b"Im1".to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
            "Contents" => content_id,
            "Resources" => dictionary! {
                "XObject" => dictionary! { "Im1" => image_id },
            },
        });
        kids.push(page_id.into());
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => pages.len() as i64,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);

    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn chapter_pdf(chapter: &ChapterDir) -> anyhow::Result<Vec<u8>> {
    let pages = chapter
        .pages
        .iter()
        .filter_map(|p| match page_as_jpeg(p) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(path = %p.display(), error = %e, "skipping unreadable page");
                None
            }
        })
        .collect::<Vec<_>>();
    anyhow::ensure!(
        !pages.is_empty(),
        "chapter {} has no readable pages",
        chapter.number
    );
    build_pdf(&pages)
}

pub async fn package(
    workdir: &Path,
    chapters: Vec<ChapterDir>,
    progress: Progress<'_>,
) -> AppResult<Packaged> {
    progress("Creating PDFs...");
    let workdir = workdir.to_path_buf();
    let out: PathBuf = workdir.join("Chapters.zip");
    let out_clone = out.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let file = File::create(&out_clone)?;
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        let mut taken = HashSet::new();
        for chapter in &chapters {
            let pdf = chapter_pdf(chapter)?;
            let name = format!("{}.pdf", chapter_stem(&chapter.number, &mut taken));
            zip.start_file(name, opts)?;
            zip.write_all(&pdf)?;
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
    fn reads_jpeg_header_of_placeholder() {
        let info = jpeg_info(super::super::images::PLACEHOLDER).unwrap();
        assert!(info.width > 0 && info.height > 0);
        assert_eq!(info.components, 3);
    }

    #[test]
    fn builds_a_valid_pdf() {
        let bytes = super::super::images::PLACEHOLDER.to_vec();
        let info = jpeg_info(&bytes).unwrap();
        let pdf = build_pdf(&[(bytes.clone(), info), (bytes, info)]).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.5"));
        let doc = Document::load_mem(&pdf).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }
}
