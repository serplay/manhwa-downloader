//! End-to-end tests of the download lifecycle through the real router: queue a
//! task, watch it move through the queue, fetch the archive, cancel, and sweep.
//!
//! The upstream is faked twice over. `FakeSource` stands in for a scraper and
//! hands out page URLs on a local `wiremock` server, which serves real PNGs, so
//! the image pipeline, packagers and file streaming all run for real. Nothing
//! here touches the network.

use std::{
    collections::HashMap,
    io::{Cursor, Read},
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::Semaphore;
use tower::ServiceExt;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path_regex},
};

use crate::{
    config::Config,
    error::{AppError, AppResult},
    model::{Comic, FetchTier, SourceCapabilities, SourceMeta, Speed, Volume},
    sources::{Ctx, PageUrl, Registry, SearchOptions, Source},
    state::{AppState, SharedState},
    tasks::{SUCCESS_RETENTION, TERMINAL_RETENTION},
};

const SLUG: &str = "fake";
const PAGES_PER_CHAPTER: usize = 3;
/// Upper bound on any wait in these tests. Generous so slow CI runners pass.
const WAIT: Duration = Duration::from_secs(30);

// ---- fixtures ---------------------------------------------------------------

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(w, h, image::Rgb([40, 80, 160]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// Local image host: `/img/*.png` answers a real 200x300 PNG, anything under
/// `/missing/` is a 404.
async fn image_host() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/img/.+\.png$"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "image/png")
                .set_body_bytes(png(200, 300)),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/missing/"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    server
}

/// How the fake source answers `chapter_pages` for a chapter id.
#[derive(Clone)]
enum Chapter {
    /// Every page resolves to a real image.
    Ok,
    /// The source itself fails to list the pages.
    SourceError,
    /// Pages resolve, but every image 404s.
    DeadImages,
}

struct FakeSource {
    meta: SourceMeta,
    image_base: String,
    chapters: HashMap<String, Chapter>,
    /// When set, `chapter_pages` waits for a permit, so a test can hold a task
    /// in PROGRESS for as long as it needs to look at the queue.
    gate: Option<Arc<Semaphore>>,
}

impl FakeSource {
    fn new(image_base: &str) -> Self {
        Self {
            meta: SourceMeta {
                id: 0,
                slug: SLUG,
                name: "Fake",
                base_url: image_base.to_string(),
                speed: Speed::Fastest,
                languages: vec!["en"],
                tier: FetchTier::Plain,
                capabilities: SourceCapabilities {
                    search: true,
                    chapters: true,
                    download: true,
                    needs_browser: false,
                },
                adult: false,
            },
            image_base: image_base.to_string(),
            chapters: HashMap::new(),
            gate: None,
        }
    }

    fn chapter(mut self, id: &str, kind: Chapter) -> Self {
        self.chapters.insert(id.to_string(), kind);
        self
    }

    fn gated(mut self, gate: Arc<Semaphore>) -> Self {
        self.gate = Some(gate);
        self
    }
}

#[async_trait]
impl Source for FakeSource {
    fn meta(&self) -> &SourceMeta {
        &self.meta
    }

    async fn search(
        &self,
        _ctx: &Ctx,
        _query: &str,
        _lang: Option<&str>,
        _opts: &SearchOptions,
    ) -> AppResult<Vec<Comic>> {
        Ok(Vec::new())
    }

    async fn chapters(&self, _ctx: &Ctx, _id: &str, _lang: Option<&str>) -> AppResult<Vec<Volume>> {
        Ok(Vec::new())
    }

    async fn chapter_pages(&self, _ctx: &Ctx, chapter_id: &str) -> AppResult<Vec<PageUrl>> {
        if let Some(gate) = &self.gate {
            gate.acquire().await.expect("gate closed").forget();
        }
        let folder = match self
            .chapters
            .get(chapter_id)
            .cloned()
            .unwrap_or(Chapter::Ok)
        {
            Chapter::Ok => "img",
            Chapter::DeadImages => "missing",
            Chapter::SourceError => {
                return Err(AppError::Upstream {
                    site: Some(SLUG.into()),
                    message: "fake source refused the chapter".into(),
                });
            }
        };
        Ok((1..=PAGES_PER_CHAPTER)
            .map(|p| PageUrl {
                url: format!("{}/{folder}/{chapter_id}/{p}.png", self.image_base),
                referer: Some(format!("{}/", self.image_base)),
            })
            .collect())
    }
}

struct Harness {
    app: Router,
    state: SharedState,
    downloads: TempDir,
    _images: MockServer,
}

async fn harness(configure: impl FnOnce(FakeSource) -> FakeSource, slots: usize) -> Harness {
    let images = image_host().await;
    let downloads = tempfile::tempdir().unwrap();
    let mut config = Config::for_tests(downloads.path().to_path_buf());
    config.max_concurrent_downloads = slots;
    let source: Arc<dyn Source> = Arc::new(configure(FakeSource::new(&images.uri())));
    let state = Arc::new(
        AppState::with_registry(config, Registry::from_sources(vec![source]))
            .await
            .unwrap(),
    );
    Harness {
        app: super::router(Arc::clone(&state)),
        state,
        downloads,
        _images: images,
    }
}

// ---- HTTP helpers -----------------------------------------------------------

struct Reply {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: Vec<u8>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("not JSON ({e}): {}", String::from_utf8_lossy(&self.body)))
    }
}

impl Harness {
    async fn call(&self, req: Request<Body>) -> Reply {
        let resp = self.app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        Reply {
            status,
            headers,
            body,
        }
    }

    async fn get(&self, uri: &str) -> Reply {
        self.call(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    async fn post(&self, uri: &str, body: Value) -> Reply {
        self.call(
            Request::post(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    /// Queue a download and return its task id.
    async fn start(&self, format: &str, chapters: &[&str]) -> String {
        let reply = self
            .post("/download", download_body(format, chapters))
            .await;
        assert_eq!(reply.status, StatusCode::ACCEPTED, "{}", reply.json());
        let v = reply.json();
        assert_eq!(v["state"], "PENDING");
        v["task_id"].as_str().unwrap().to_string()
    }

    async fn status(&self, id: &str) -> Value {
        let reply = self.get(&format!("/download/status/{id}")).await;
        assert_eq!(reply.status, StatusCode::OK);
        reply.json()
    }

    /// Poll until the task's state is `want`, failing on any other terminal state.
    async fn wait_for(&self, id: &str, want: &str) -> Value {
        let deadline = Instant::now() + WAIT;
        loop {
            let s = self.status(id).await;
            let state = s["state"].as_str().unwrap().to_string();
            if state == want {
                return s;
            }
            let terminal = matches!(state.as_str(), "SUCCESS" | "FAILURE" | "CANCELLED");
            assert!(!terminal, "task {id} ended as {state}, wanted {want}: {s}");
            assert!(Instant::now() < deadline, "task {id} stuck in {state}: {s}");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn workdir(&self, id: &str) -> std::path::PathBuf {
        self.downloads.path().join(id)
    }
}

fn download_body(format: &str, chapters: &[&str]) -> Value {
    json!({
        "source": SLUG,
        "comic_title": "Test Comic",
        "format": format,
        "chapters": chapters
            .iter()
            .map(|c| json!({ "id": c, "number": c }))
            .collect::<Vec<_>>(),
    })
}

fn zip_entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("a valid zip");
    (0..archive.len())
        .map(|i| {
            let mut f = archive.by_index(i).unwrap();
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            (f.name().to_string(), data)
        })
        .collect()
}

// ---- downloads ----------------------------------------------------------------

#[tokio::test]
async fn pdf_download_end_to_end() {
    let h = harness(|s| s, 1).await;
    let id = h.start("pdf", &["1", "2"]).await;
    let done = h.wait_for(&id, "SUCCESS").await;
    assert_eq!(done["progress"], 100);
    assert_eq!(done["chapter_total"], 2);
    assert!(done.get("warnings").is_none(), "{done}");

    let file = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(file.status, StatusCode::OK);
    assert_eq!(file.headers[header::CONTENT_TYPE], "application/zip");
    assert_eq!(
        file.headers[header::CONTENT_LENGTH].to_str().unwrap(),
        done["file_size"].to_string()
    );
    let disposition = file.headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(disposition.contains("Test Comic"), "{disposition}");

    let entries = zip_entries(&file.body);
    assert_eq!(entries.len(), 2, "one PDF per chapter");
    for (name, pdf) in &entries {
        assert!(name.ends_with(".pdf"), "{name}");
        let doc = lopdf::Document::load_mem(pdf).expect("a valid PDF");
        assert_eq!(doc.get_pages().len(), PAGES_PER_CHAPTER, "{name}");
    }
}

#[tokio::test]
async fn cbz_download_contains_every_page() {
    let h = harness(|s| s, 1).await;
    let id = h.start("cbz", &["7"]).await;
    h.wait_for(&id, "SUCCESS").await;

    let file = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(file.status, StatusCode::OK);
    let outer = zip_entries(&file.body);
    assert_eq!(outer.len(), 1);
    assert!(outer[0].0.ends_with(".cbz"), "{}", outer[0].0);
    let inner = zip_entries(&outer[0].1);
    let pages: Vec<_> = inner.iter().filter(|(n, _)| n.ends_with(".png")).collect();
    assert_eq!(pages.len(), PAGES_PER_CHAPTER);
    assert!(inner.iter().any(|(n, _)| n == "ComicInfo.xml"));
    // The bytes are the images the host served, not placeholders.
    assert_eq!(pages[0].1, png(200, 300));
}

#[tokio::test]
async fn epub_download_is_a_valid_book() {
    let h = harness(|s| s, 1).await;
    let id = h.start("epub", &["1", "2"]).await;
    let done = h.wait_for(&id, "SUCCESS").await;

    let file = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(file.status, StatusCode::OK);
    assert_eq!(
        file.headers[header::CONTENT_TYPE].to_str().unwrap(),
        done["content_type"].as_str().unwrap()
    );
    let entries = zip_entries(&file.body);
    let (first, mimetype) = &entries[0];
    assert_eq!(first, "mimetype", "EPUB must start with its mimetype entry");
    assert_eq!(mimetype, b"application/epub+zip");
    let images = entries
        .iter()
        .filter(|(n, _)| n.contains("images/"))
        .count();
    assert_eq!(images, 2 * PAGES_PER_CHAPTER);
}

#[tokio::test]
async fn archive_can_be_fetched_again_until_swept() {
    let h = harness(|s| s, 1).await;
    let id = h.start("cbz", &["1"]).await;
    h.wait_for(&id, "SUCCESS").await;

    let first = h.get(&format!("/download/file/{id}")).await;
    let second = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(
        second.status,
        StatusCode::OK,
        "a retried transfer must still find the file"
    );
    assert_eq!(first.body, second.body);

    // Inside the window nothing is removed.
    assert_eq!(h.state.engine.sweep(Instant::now()).await, 0);
    assert!(h.workdir(&id).exists());

    // Past it, the task and its directory are gone.
    let later = Instant::now() + SUCCESS_RETENTION + Duration::from_secs(1);
    assert_eq!(h.state.engine.sweep(later).await, 1);
    assert!(!h.workdir(&id).exists());
    assert_eq!(
        h.get(&format!("/download/file/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        h.get(&format!("/download/status/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn broken_chapters_are_skipped_with_warnings() {
    let h = harness(
        |s| {
            s.chapter("2", Chapter::SourceError)
                .chapter("3", Chapter::DeadImages)
        },
        1,
    )
    .await;
    let id = h.start("cbz", &["1", "2", "3"]).await;
    let done = h.wait_for(&id, "SUCCESS").await;

    let warnings = done["warnings"].as_array().expect("warnings present");
    assert_eq!(warnings.len(), 2, "{done}");
    assert!(
        warnings[0]
            .as_str()
            .unwrap()
            .starts_with("Chapter 2 skipped")
    );
    assert!(
        warnings[1]
            .as_str()
            .unwrap()
            .starts_with("Chapter 3 skipped")
    );

    let file = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(zip_entries(&file.body).len(), 1, "only chapter 1 made it");
}

#[tokio::test]
async fn task_fails_when_no_chapter_downloads() {
    let h = harness(|s| s.chapter("1", Chapter::DeadImages), 1).await;
    let id = h.start("pdf", &["1"]).await;

    let deadline = Instant::now() + WAIT;
    let failed = loop {
        let s = h.status(&id).await;
        if s["state"] == "FAILURE" {
            break s;
        }
        assert_ne!(s["state"], "SUCCESS");
        assert!(Instant::now() < deadline, "never failed: {s}");
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    assert!(
        failed["error"].as_str().unwrap().contains("no chapter"),
        "{failed}"
    );
    assert!(
        !h.workdir(&id).exists(),
        "failed tasks clean up after themselves"
    );

    let file = h.get(&format!("/download/file/{id}")).await;
    assert_eq!(file.status, StatusCode::NOT_FOUND);

    let later = Instant::now() + TERMINAL_RETENTION + Duration::from_secs(1);
    assert_eq!(h.state.engine.sweep(later).await, 1);
}

// ---- queueing -------------------------------------------------------------------

#[tokio::test]
async fn queue_runs_one_task_at_a_time_per_slot() {
    let gate = Arc::new(Semaphore::new(0));
    let h = harness(|s| s.gated(Arc::clone(&gate)), 1).await;

    let first = h.start("cbz", &["1"]).await;
    let second = h.start("cbz", &["2"]).await;

    let running = h.wait_for(&first, "PROGRESS").await;
    assert_eq!(running["chapter_current"], 0);
    // One slot: the second task must still be waiting its turn.
    let queued = h.status(&second).await;
    assert_eq!(queued["state"], "PENDING", "{queued}");
    assert_eq!(h.state.engine.active_count(), 2);

    // Asking for a file that is not built yet is a conflict, not a 404.
    let early = h.get(&format!("/download/file/{first}")).await;
    assert_eq!(early.status, StatusCode::CONFLICT);
    assert_eq!(early.json()["error"]["code"], "CONFLICT");

    gate.add_permits(1);
    h.wait_for(&first, "SUCCESS").await;
    h.wait_for(&second, "PROGRESS").await;
    gate.add_permits(1);
    h.wait_for(&second, "SUCCESS").await;
    assert_eq!(h.state.engine.active_count(), 0);

    let tasks = h.get("/tasks").await.json();
    let listed = tasks.as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|t| t["state"] == "SUCCESS"));
}

#[tokio::test]
async fn parallel_slots_run_tasks_together() {
    let gate = Arc::new(Semaphore::new(0));
    let h = harness(|s| s.gated(Arc::clone(&gate)), 2).await;

    let a = h.start("cbz", &["1"]).await;
    let b = h.start("cbz", &["2"]).await;
    let c = h.start("cbz", &["3"]).await;
    h.wait_for(&a, "PROGRESS").await;
    h.wait_for(&b, "PROGRESS").await;
    assert_eq!(h.status(&c).await["state"], "PENDING");

    gate.add_permits(3);
    for id in [&a, &b, &c] {
        h.wait_for(id, "SUCCESS").await;
    }
}

#[tokio::test]
async fn cancel_running_and_queued_tasks() {
    let gate = Arc::new(Semaphore::new(0));
    let h = harness(|s| s.gated(Arc::clone(&gate)), 1).await;

    let running = h.start("pdf", &["1"]).await;
    let queued = h.start("pdf", &["2"]).await;
    let behind = h.start("pdf", &["3"]).await;
    h.wait_for(&running, "PROGRESS").await;

    // Cancel the waiting one first: it must never start.
    let reply = h
        .post(&format!("/download/cancel/{queued}"), json!({}))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    h.wait_for(&queued, "CANCELLED").await;

    let reply = h
        .post(&format!("/download/cancel/{running}"), json!({}))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    h.wait_for(&running, "CANCELLED").await;
    assert!(
        !h.workdir(&running).exists(),
        "cancel removes the work directory"
    );

    // The slot is free again, so the task behind them runs.
    h.wait_for(&behind, "PROGRESS").await;
    gate.add_permits(1);
    h.wait_for(&behind, "SUCCESS").await;

    // Cancelling a finished task is a conflict; an unknown one is a 404.
    let again = h
        .post(&format!("/download/cancel/{behind}"), json!({}))
        .await;
    assert_eq!(again.status, StatusCode::CONFLICT);
    let unknown = h
        .post(
            &format!("/download/cancel/{}", uuid::Uuid::new_v4()),
            json!({}),
        )
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn events_stream_ends_on_the_terminal_state() {
    let h = harness(|s| s, 1).await;
    let id = h.start("cbz", &["1", "2"]).await;

    let reply = h.get(&format!("/download/events/{id}")).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let body = String::from_utf8(reply.body).unwrap();
    let states: Vec<Value> = body
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .map(|d| serde_json::from_str(d).unwrap())
        .collect();
    assert!(!states.is_empty());
    assert_eq!(states.last().unwrap()["state"], "SUCCESS");
    let progress: Vec<u64> = states
        .iter()
        .map(|s| s["progress"].as_u64().unwrap())
        .collect();
    assert!(
        progress.windows(2).all(|w| w[0] <= w[1]),
        "progress went backwards: {progress:?}"
    );
}

// ---- validation ---------------------------------------------------------------

#[tokio::test]
async fn rejects_bad_download_requests() {
    let h = harness(|s| s, 1).await;
    let cases = [
        (
            json!({ "source": SLUG, "chapters": [] }),
            StatusCode::BAD_REQUEST,
            "VALIDATION",
        ),
        (
            download_body("pdf", &["1"]).tap(|b| b["source"] = json!("nope")),
            StatusCode::BAD_REQUEST,
            "UNKNOWN_SOURCE",
        ),
        (
            download_body("tiff", &["1"]),
            StatusCode::BAD_REQUEST,
            "VALIDATION",
        ),
        (
            download_body("pdf", &["1"]).tap(|b| b["comic_title"] = json!("x".repeat(201))),
            StatusCode::BAD_REQUEST,
            "VALIDATION",
        ),
    ];
    for (body, status, code) in cases {
        let reply = h.post("/download", body.clone()).await;
        assert_eq!(reply.status, status, "{body}");
        assert_eq!(reply.json()["error"]["code"], code, "{body}");
    }

    let too_many: Vec<String> = (0..=crate::tasks::MAX_CHAPTERS_PER_TASK)
        .map(|i| i.to_string())
        .collect();
    let refs: Vec<&str> = too_many.iter().map(String::as_str).collect();
    let reply = h.post("/download", download_body("pdf", &refs)).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);

    let empty = h
        .call(Request::post("/download").body(Body::empty()).unwrap())
        .await;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);

    assert!(
        h.state.engine.list().is_empty(),
        "rejected requests queue nothing"
    );

    let cbr = h.post("/download", download_body("cbr", &["1"])).await;
    if !h.state.engine.rar_available {
        assert_eq!(cbr.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(cbr.json()["error"]["code"], "UNSUPPORTED");
    }
}

#[tokio::test]
async fn unknown_task_ids_are_404() {
    let h = harness(|s| s, 1).await;
    let id = uuid::Uuid::new_v4();
    for uri in [
        format!("/download/status/{id}"),
        format!("/download/file/{id}"),
        format!("/download/events/{id}"),
    ] {
        assert_eq!(h.get(&uri).await.status, StatusCode::NOT_FOUND, "{uri}");
    }
    assert_eq!(
        h.get("/download/status/not-a-uuid").await.status,
        StatusCode::BAD_REQUEST
    );
}

trait Tap: Sized {
    fn tap(self, f: impl FnOnce(&mut Self)) -> Self;
}

impl Tap for Value {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}
