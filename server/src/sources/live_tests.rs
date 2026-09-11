//! Live smoke tests against the real sites. Ignored by default; run manually
//! with `cargo test live_ -- --ignored --nocapture` to re-measure which
//! sources work from the current network (plan section 2.11 and phase 3c).

use std::{sync::Arc, time::Duration};

use super::{Ctx, SearchOptions, Source, http};
use crate::model::FetchTier;

fn ctx() -> Ctx {
    let timeout = Duration::from_secs(30);
    let client = http::build_client(timeout).unwrap();
    let imp = http::build_impersonating_client(timeout).ok();
    Ctx {
        client: client.clone(),
        fetcher: Arc::new(http::Fetcher::new(client, imp)),
    }
}

/// search -> chapters -> pages of the first chapter, asserting each step
/// returns something. Failures print the source so a run over all sources
/// doubles as the parity report.
async fn roundtrip(source: &dyn Source, query: &str) {
    let ctx = ctx();
    let slug = source.meta().slug;
    let comics = source
        .search(&ctx, query, None, &SearchOptions::default())
        .await
        .unwrap_or_else(|e| panic!("[{slug}] search failed: {e}"));
    assert!(!comics.is_empty(), "[{slug}] search returned nothing");
    let comic = &comics[0];
    let volumes = source
        .chapters(&ctx, &comic.id, None)
        .await
        .unwrap_or_else(|e| panic!("[{slug}] chapters failed for {}: {e}", comic.id));
    let chapter = volumes
        .iter()
        .flat_map(|v| &v.chapters)
        .next()
        .unwrap_or_else(|| panic!("[{slug}] no chapters for {}", comic.id));
    let pages = source
        .chapter_pages(&ctx, &chapter.id)
        .await
        .unwrap_or_else(|e| panic!("[{slug}] pages failed for {}: {e}", chapter.id));
    assert!(
        !pages.is_empty(),
        "[{slug}] chapter {} has no pages",
        chapter.id
    );
    // Fetch one page image through the same tier the pipeline would use.
    let mut req = ctx
        .fetcher
        .get(&pages[0].url)
        .tier(source.meta().tier)
        .source(slug)
        .accept_images();
    if let Some(r) = &pages[0].referer {
        req = req.referer(r);
    }
    let bytes = req
        .bytes()
        .await
        .unwrap_or_else(|e| panic!("[{slug}] page image fetch failed: {e}"));
    assert!(bytes.len() > 1024, "[{slug}] page image suspiciously small");
    eprintln!(
        "[{slug}] ok: {} results, {} chapters, {} pages, first image {} bytes",
        comics.len(),
        volumes.iter().map(|v| v.chapters.len()).sum::<usize>(),
        pages.len(),
        bytes.len()
    );
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_mangapill() {
    roundtrip(&super::mangapill::Mangapill::new(), "solo leveling").await;
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_mangahere() {
    roundtrip(&super::mangahere::Mangahere::new(), "solo leveling").await;
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_asurascans() {
    roundtrip(&super::asura::Asura::new(), "solo leveling").await;
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_weebcentral() {
    roundtrip(&super::weebcentral::Weebcentral::new(), "solo leveling").await;
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_ravenscans() {
    roundtrip(
        &super::mangareader::MangaReader::new(super::mangareader::RAVENSCANS),
        "solo leveling",
    )
    .await;
}

#[tokio::test]
#[ignore = "hits the live site"]
async fn live_toonily() {
    roundtrip(
        &super::madara::Madara::new(super::madara::TOONILY, false),
        "solo leveling",
    )
    .await;
}

/// The three managed-challenge sites. Expected to fail with `SOURCE_BLOCKED`
/// until a browser tier exists; if one starts passing, flip its
/// `needs_browser` flag in `madara.rs`.
#[tokio::test]
#[ignore = "hits the live site"]
async fn live_browser_only_madara_sites() {
    for site in [
        super::madara::MANHUAUS,
        super::madara::KUNMANGA,
        super::madara::TOONGOD,
    ] {
        let source = super::madara::Madara::new(site.clone(), true);
        assert_eq!(source.meta().tier, FetchTier::Browser);
        match source
            .search(&ctx(), "solo", None, &SearchOptions::default())
            .await
        {
            Ok(comics) => eprintln!(
                "[{}] PASSES without a browser now ({} results); consider needs_browser=false",
                site.slug,
                comics.len()
            ),
            Err(e) => eprintln!("[{}] still blocked: {e}", site.slug),
        }
    }
}
