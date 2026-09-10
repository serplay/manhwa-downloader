# Manhwa Downloader - Modernization Plan

Status: APPROVED (decisions D1-D9 accepted, 2026-09-10). Progress: Phase 0 done, Phase 1 done, Phase 2 done, Phase 3 done (2026-09-10; see the 3c results under Phase 3). Next: Phase 4.
Branch: `rust-dev`. Inputs: `server-old/` (Python/FastAPI/Celery, gitignored), `server/` (Rust stub, two routes), `client/` (Vite + React 19 + Tailwind v4).

This plan has three parts: what exists today (audit), what we build (backend, API, frontend, deployment), and how we get there (phases, parity checklist, decisions you need to confirm).

---

## 0. Executive summary

- **Backend:** one Rust binary (Axum + Tokio) replaces FastAPI + Celery + Redis + Consumet + Selenium. Downloads run on an in-process task engine with cancellation, progress, and Server-Sent Events. No Redis. No Python. Optional headless-browser sidecar behind a Cargo feature for Cloudflare-gated sources.
- **Sources:** 11 sources collapse into 6 implementations. Five of them (Manhuaus, Yakshascans, Kunmanga, Toonily, Toongod) run the same WordPress "Madara" theme and become one parameterized module.
- **API:** same paths the client already calls, so the client keeps working during the migration. Then additive improvements: consistent error envelope with real status codes, JSON body for downloads, `/sources` metadata endpoint, SSE progress stream, OpenAPI docs.
- **Frontend:** redesign-overhaul on the existing Vite/React/Tailwind v4 stack. TypeScript, TanStack Query for server state, Radix primitives for accessible controls, Motion for animation, Phosphor icons, Geist type, one locked accent color. Cover-first results, a slide-over chapter picker with virtualization, and a single downloads tray replacing four overlapping popups.
- **Deployment:** one multi-stage Dockerfile, one container. Vercel keeps serving the client with the existing `/api` rewrite.

Design read (per the taste skill): *Reading this as: redesign-overhaul of a consumer utility app (search, pick chapters, download) for manga readers, with a calm premium-tool language, leaning toward Tailwind v4 utilities + Radix primitives + Geist + restrained Motion.*
Dials: existing app reads as VARIANCE 3 / MOTION 4 / DENSITY 6. Target: **VARIANCE 5 / MOTION 5 / DENSITY 5**. This is a product UI, not a landing page, so the taste skill's landing-page rules (hero stack, bento, marquee) are out of scope. Its typography, color, state, motion, and anti-tell rules apply in full.

---

## 1. Audit of the current state

### 1.1 Backend (`server-old/`, Python)

| Area | Today | Problems |
|---|---|---|
| Web | FastAPI, 13 routes, CORS `*` | `/search/` and `/chapters/` return HTTP 200 with `{"message": err}` or `{"error": err}` on failure, so the client cannot tell errors from results. |
| Queue | Celery + Redis, 2 worker processes, per-task time limits scaled by source | Three processes and a broker for a single-user tool. Cancel path only cleans temp dirs that were recorded on success, so cancelling mid-download leaks the directory. |
| Cache | `fastapi-cache2` on Redis DB 1, 12h TTL for search and chapters | Redis flushed on shutdown, cache key includes the raw query. |
| Sources | 11 classes, one file each, `match` dispatch by numeric string | 7 of 11 use SeleniumBase with `uc=True, xvfb=True` (undetected Chrome + virtual display) for search, chapters, and every chapter page. Each search spawns a fresh browser. |
| Consumet | Mangapill and Mangahere go through a self-hosted Consumet API | External Node service, unmaintained upstream, extra container. |
| Images | `requests` sequential per image, PIL verify, webp to jpeg, 72px minimum, corrupt placeholder | Sequential (slow), webp branch has a path bug (`x.webp.jpg`), size-check `continue` skips the append but leaves the file on disk. |
| Formats | PDF via `img2pdf`, CBZ via `cbz` lib + ComicInfo, CBR via `rar` binary, EPUB via `ebooklib` | CBR needs proprietary `rar`. CBZ typo `serites=` means series name is dropped. |
| Chapter IDs | Client sends `"{id}_{chapter}"`, server splits on `_` | Breaks for IDs that contain underscores (Bato and Mangapill work around it, others do not). |
| Cover art | Server bakes `/api/proxy-image?url=...` into responses | Server hardcodes the client's proxy prefix. |
| Cloudflare | Cookies scraped by Selenium, cached in Redis 30 min | Toonily and Toongod downloads marked "working on it" in README. |
| Docker | Python 3.10 + Chromium + Xvfb + unrar, Compose with 4 services | Dockerfile CMD is commented out. |

Bugs worth fixing during the port (not just re-implementing):
- Bato `get_chapters` indexes `data[-1]` before checking for an empty page.
- MangaDex chapters are hardcoded to `translatedLanguage=en` even though search returns `availableLanguages`.
- Chapter numbers are used as directory names; duplicates (two "Chapter 10" entries) overwrite each other.
- `/download` accepts `format` as a query param but the client passes `comic_title` unsanitized into `Content-Disposition`.

### 1.2 Frontend (`client/`)

Stack: Vite 6, React 19, Tailwind v4 (via `@tailwindcss/vite`), framer-motion 12, FontAwesome, `@vercel/analytics`.

| Area | Today | Problems |
|---|---|---|
| Structure | `App.jsx` is 900+ lines with 20 `useState` hooks, polling loop, fetch calls, and the whole render tree | No separation of server state, UI state, and presentation. Not typed. |
| Dead code | `package.json` scripts reference `api-server.js` (missing); `express`, `cors`, `http-proxy-middleware`, `concurrently` are unused | Dependency noise. `tailwind.config.js` is ignored by Tailwind v4 and `index.css` mixes v4 `@import` with v3 `@tailwind` directives. |
| Visuals | Pink-to-purple gradient background, gradient-text title, violet accent in dark and pink in light, FontAwesome icons, spinners everywhere, cards with border + shadow + white | Textbook "AI purple gradient" fingerprint. Two accents. No type choice. |
| Search | Input + native `<select>` + button. Bato default. `/search/all` exists on the server but is not wired | Source names are hardcoded in three places. |
| Results | Accordion inside a `max-h-[60vh]` scroll box, comic row expands to chapters inline | Nested scroll regions, every chapter rendered as a button (1000+ buttons for long series). |
| Chapter picker | Hand-rolled range slider (mouse + touch handlers), "Select Range", "Select All", format `<select>`, Download | Slider is not keyboard accessible, no numeric input. |
| Notifications | Error, Success, Loading popups top-center plus ActiveTasks top-right | They overlap (open TODO item). Loading is a modal toast rather than skeletons. |
| Source status | Large card with a 4-item legend, polls `/status` every 5 min | `dangerouslySetInnerHTML` on plain strings. |
| Downloads | Polls every 2s per task, triggers `<a>` click on SUCCESS | Works. No SSE. |
| Theme | Class toggle persisted in localStorage, inline script prevents flash | Fine. Sun/moon toggle is the default cliche. |
| A11y | No focus rings (`focus:outline-none` everywhere), no labels, no aria-live | Needs a full pass. |

### 1.3 What to preserve

- Information architecture: search, results grouped by source, expand a comic, pick chapters (individual, range, all), choose format, download, watch progress, cancel. Source status visibility. Light and dark themes.
- The `/api` prefix and Vercel rewrite to the API host.
- Numeric source IDs `"0"` to `"10"` as accepted input (add slugs alongside).
- Task states `PENDING`, `PROGRESS`, `SUCCESS`, `FAILURE` and the status response shape.
- Logo asset.
- Analytics.

---

## 2. Backend architecture (Rust)

### 2.1 Crate layout

Single binary crate at `server/` (a workspace is overkill for now, but modules are cut so a split into `core` + `sources` + `api` later is mechanical).

```
server/
  Cargo.toml
  src/
    main.rs              # bootstrap: config, tracing, router, graceful shutdown
    config.rs            # env parsing into Config
    error.rs             # AppError -> HTTP mapping, error envelope
    state.rs             # AppState: http client, cache, task engine, config
    api/
      mod.rs             # Router assembly, middleware stack
      search.rs          # GET /search, GET /search/all
      chapters.rs        # GET /chapters
      download.rs        # POST /download, status, file, cancel, events (SSE)
      sources.rs         # GET /sources, GET /status
      proxy.rs           # GET /proxy-image
      health.rs          # GET /health, GET /
      docs.rs            # utoipa OpenAPI + Swagger UI at /docs
    model/
      mod.rs             # Comic, Chapter, Volume, SourceId, Format, TaskState...
    sources/
      mod.rs             # trait Source, registry, SourceId <-> impl
      http.rs            # shared fetch helpers, header profiles, retry
      mangadex.rs
      bato.rs
      madara.rs          # Manhuaus, Yakshascans, Kunmanga, Toonily, Toongod
      asura.rs
      weebcentral.rs
      mangapill.rs
      mangahere.rs
      fixtures/          # saved HTML/JSON for parser tests
    pipeline/
      mod.rs
      images.rs          # concurrent image download, verify, convert
      pdf.rs
      cbz.rs
      cbr.rs             # feature-gated shell-out to `rar`
      epub.rs
    tasks/
      mod.rs             # TaskEngine: spawn, progress, cancel, list, GC
      progress.rs        # watch channel + broadcast for SSE
      cleanup.rs         # temp dir lifecycle, orphan sweep on boot
    browser/             # feature = "browser"
      mod.rs             # chromiumoxide session pool for CF-gated sources
```

### 2.2 Dependencies

| Purpose | Crate | Notes |
|---|---|---|
| HTTP server | `axum` 0.8, `tokio` (full), `tower-http` (cors, compression, trace, timeout, limit) | |
| Serialization | `serde`, `serde_json` | |
| HTTP client | `reqwest` 0.12 (rustls, json, cookies, stream, gzip, brotli) | Default fetcher. |
| Browser-like TLS | `wreq` (Chrome TLS + HTTP/2 fingerprint impersonation) | Second-tier fetcher for Cloudflare-fronted sites. Passes the passive checks that block python-requests without a browser. |
| HTML parsing | `scraper` (CSS selectors) | Replaces BeautifulSoup. |
| Cache | `moka` (future) | In-memory TTL cache for search and chapters. |
| Task primitives | `tokio-util` (CancellationToken, ReaderStream), `dashmap` | |
| Images | `image` 0.25 | Decode, verify dimensions, webp to jpeg. |
| PDF | `lopdf` | Hand-built image-only PDF: JPEG pages embedded with DCTDecode losslessly (what img2pdf does), PNG re-encoded to JPEG. No rasterization. |
| CBZ / ZIP | `zip`, `quick-xml` | ComicInfo.xml written by hand (small schema). |
| EPUB | `epub-builder` | |
| Errors, logs | `thiserror`, `anyhow`, `tracing`, `tracing-subscriber` (json + env filter) | |
| Config | `dotenvy` + manual env parsing | No framework needed. |
| IDs, misc | `uuid`, `url`, `percent-encoding`, `regex`, `mime_guess`, `tempfile`, `futures`, `bytes`, `async-stream` | |
| OpenAPI | `utoipa`, `utoipa-axum`, `utoipa-swagger-ui` | Restores the `/docs` page FastAPI gave for free. |
| Tests | `wiremock`, `tokio::test`, `insta` (snapshot parsers) | |
| Browser (feature) | `chromiumoxide` | Only compiled with `--features browser`. |

### 2.3 Source abstraction

```rust
#[async_trait]
pub trait Source: Send + Sync {
    fn meta(&self) -> &SourceMeta;        // id, slug, name, base_url, speed tier, capabilities
    async fn search(&self, ctx: &Ctx, query: &str) -> Result<Vec<Comic>>;
    async fn chapters(&self, ctx: &Ctx, comic_id: &str, lang: Option<&str>) -> Result<Vec<Volume>>;
    async fn chapter_pages(&self, ctx: &Ctx, chapter_id: &str) -> Result<Vec<PageUrl>>;  // url + referer + cookies profile
    async fn probe(&self, ctx: &Ctx) -> Health;   // for /status
}
```

- `Ctx` carries the fetcher tier (`Plain`, `Impersonate`, `Browser`), cookie jar per domain, and a per-source rate limiter (`tokio::sync::Semaphore` + interval).
- Downloading page images is NOT per-source. Sources return URLs plus a header profile; the shared pipeline downloads them. That removes the copy-pasted `download_chapters` from every source.
- Fetch escalation: try `Plain`, on Cloudflare signature (403 + `cf-mitigated` header or challenge HTML) retry with `Impersonate`, and if the `browser` feature is on and the source is flagged `needs_browser`, fall back to a headless Chromium session that solves the managed challenge and hands cookies back to the impersonating client. Cookies cached in memory per domain for 30 minutes (same TTL as today).

Source inventory and implementation strategy:

| ID | Slug | Today | New implementation | Fetch tier |
|---|---|---|---|---|
| 0 | mangadex | REST API | Native REST. Add `lang` param. Keep skipping titles with an `amz` link (official publisher, cannot download). | Plain |
| 9 | bato | GraphQL `/ap2/` | Native GraphQL. Fix pagination. | Plain |
| 8 | mangapill | Consumet | **Done.** Native HTML (`mangapill.rs`); server-rendered, no JS. Consumet dropped. | Plain |
| 7 | mangahere | Consumet | **Done.** Native HTML (`mangahere.rs`). Page images sit in Dean Edwards packed scripts; a small unpacker reads the embedded `newImgs` list and falls back to paging `chapterfun.ashx`. Native proved live, so no Consumet fallback was kept (D4). | Plain |
| 1, 4, 5, 6 | manhuaus, kunmanga, toonily, toongod | 4 near-identical SeleniumBase scrapers | **Done.** One `madara.rs` parameterized by base URL, series path (`manga`/`serie`/`webtoon`) and search style. Chapters come from the theme's `ajax/chapters/` POST with the series page as fallback. Toonily works through impersonation; the other three serve a managed JS challenge (see 3c) and are registered with `needs_browser`. | Impersonate; Browser for the three |
| 2 | yakshascans -> **ravenscans** | SeleniumBase | **Done.** `yakshascans.com` now redirects to `ravenscans.org`, which runs the Themesia "MangaReader" theme, not Madara. New `mangareader.rs` (`ts_reader.run` JSON for pages). Slug is `ravenscans`; `yakshascans` still resolves as an alias and legacy id 2 is unchanged. | Plain |
| 3 | asurascans | SeleniumBase | **Done.** The site moved to `asurascans.com` and exposes a JSON API at `api.asurascans.com` (`/api/series?search=`, `/api/series/{slug}/chapters`, `/api/series/{slug}/chapters/{slug}`). `asura.rs` talks to it directly; premium chapters are skipped. | Plain |
| 10 | weebcentral | SeleniumBase, clicks "Show All Chapters" | **Done.** Everything is HTMX: `/search/data`, `/series/{id}/full-chapter-list`, `/chapters/{id}/images`. `weebcentral.rs` calls them with `HX-Request`. | Plain |

Speed tiers (`fastest`, `fast`, `slow`) move out of the client into `SourceMeta` and are served by `/sources`.

### 2.4 Task engine

Replaces Celery. In-process, single binary.

- `TaskEngine` holds `DashMap<TaskId, TaskEntry>` where `TaskEntry` has `state: watch::Sender<TaskStatus>`, `cancel: CancellationToken`, `created_at`, `workdir`, `result`.
- Global `Semaphore(MAX_CONCURRENT_DOWNLOADS)` (default 2, same as the old worker concurrency). Per-source semaphore too, so two Asura jobs do not hammer one site.
- Each job runs as a `tokio::spawn` with the token checked between chapters and between images. Cancel = cancel token + remove workdir.
- Progress is a `watch` channel. Polling reads the latest value. SSE subscribes to the same channel.
- Per-job timeout derived like today: `base_seconds * chapters * source_multiplier`, enforced with `tokio::time::timeout`.
- Result retention: 1 hour after SUCCESS or until the file has been streamed (whichever first), 10 minutes after FAILURE. Background sweeper every minute. On boot, orphaned directories under `DOWNLOAD_DIR` are deleted.
- Task IDs are UUID v4. Task list endpoint returns everything in the map (this replaces Celery `inspect`).
- Persistence: none. A restart drops in-flight jobs. That matches today's behavior (Celery results expired in 1h and files lived on a volume). If you later want multi-instance, the trait boundary (`TaskStore`) is where Redis goes back in.

### 2.5 Download pipeline

1. Resolve pages for each chapter via `Source::chapter_pages` (respects cancel).
2. Download images with bounded concurrency (default 6 in flight per chapter) using the source's header profile. Retry 3x with jittered backoff on network errors and 5xx.
3. Verify with `image`: decodable, both sides >= 72 px. Otherwise substitute the bundled `corrupt.jpg` placeholder (ported from `Formats/corrupt.jpg`) so page order is preserved.
4. Normalize: webp and png to jpeg (quality 92) for PDF and EPUB; CBZ keeps originals.
5. Write into `DOWNLOAD_DIR/<task_id>/<chapter_index>_<chapter_number>/<page_index>.<ext>` (index prefix fixes the duplicate-chapter-number collision).
6. Format step:
   - **PDF**: one PDF per chapter (JPEG embedded losslessly), zipped as `Chapters.zip`. Same as today.
   - **CBZ**: one `.cbz` per chapter with `ComicInfo.xml` (Series, Title, Number, PageCount, page types), all zipped. Fixes the dropped series name.
   - **EPUB**: single `.epub`, one XHTML per chapter, images embedded, nav + NCX, minimal CSS. Same as today.
   - **CBR**: only if `rar` is on PATH at startup; advertised in `/health.capabilities`. Otherwise `POST /download` with `format=cbr` returns 422 with a clear message. (See decision D3.)
7. Delete the raw image directories after packaging. Record `file_path`, `file_size`, `content_type`.
8. Progress messages mirror today's strings ("Downloading chapter 3/12", "Creating PDFs...", "Creating ZIP archive...") so the client copy does not change.

### 2.6 File streaming and cleanup

`GET /download/file/{task_id}` streams with `ReaderStream` in 1 MiB chunks, `Content-Length`, `Content-Type` by extension, `Content-Disposition` with both `filename=` (ASCII-sanitized) and `filename*=UTF-8''` (RFC 5987) so non-Latin titles survive. Cleanup runs when the body stream is dropped, whether the transfer finished or the client aborted. A `Retry-After` and 409 are returned if the task is still running; 410 if the file was already collected.

### 2.7 Image proxy

`GET /proxy-image?url=&referer=` (accepts `hd` as a legacy alias for one release). Allowlist of hostnames derived from `SourceMeta.base_url` and known CDN hosts. Streams the upstream body (no buffering into memory like today), forwards `Content-Type`, sets `Cache-Control: public, max-age=86400`, and uses the Cloudflare cookie cache for Toonily and Toongod. Rejects non-image content types.

### 2.8 Caching

`moka` async caches: `search:{source}:{query_lowercase}` and `chapters:{source}:{id}:{lang}` with 12 h TTL, 10k entries max. `/status` results cached 60 s. Cache bypass with `Cache-Control: no-cache` request header for debugging.

### 2.9 Config (environment)

| Var | Default | Purpose |
|---|---|---|
| `PORT` | `8000` | |
| `BIND` | `0.0.0.0` | |
| `DOWNLOAD_DIR` | `./Downloads` | temp work dir |
| `MAX_CONCURRENT_DOWNLOADS` | `2` | global |
| `IMAGE_CONCURRENCY` | `6` | per chapter |
| `CACHE_TTL_SECS` | `43200` | |
| `CORS_ORIGINS` | `*` | comma list |
| `CONSUMET_URL` | unset | optional Mangahere fallback |
| `BROWSER_ENABLED` | `false` | requires `--features browser` and Chromium |
| `RUST_LOG` | `info` | |
| `PUBLIC_BASE_PATH` | `` | prefix for cover proxy URLs if not served at root |

### 2.10 Observability and safety

- `tracing` spans per request and per task with `task_id`, `source`, `chapter_count`.
- Request body limit 64 KiB, per-IP rate limit on `POST /download` (tower `GovernorLayer` or a simple token bucket) since the old TODO wanted rate limiting.
- Graceful shutdown: stop accepting, cancel running tasks, sweep workdirs.
- Input validation: max 500 chapters per request, source must exist, format must be known, title max 200 chars.

### 2.11 Testing

- Parser unit tests against saved fixtures in `sources/fixtures/` (one search page, one series page, one chapter page per source). `insta` snapshots of the parsed structs. These run offline and protect against regressions when refactoring.
- `wiremock` integration tests for the API: search happy path, error envelope, download lifecycle (start, progress, file, cleanup), cancel.
- Live smoke tests behind `--ignored`, run manually or nightly, hitting real sites.
- `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo test` in GitHub Actions.

---

## 3. API contract

### 3.1 Compatibility layer (Phase 1 and 2)

All existing paths stay valid with the same response shapes so the current client works against the Rust server before the frontend rebuild starts:

`GET /`, `GET /health`, `GET /status`, `GET /search/?title&source`, `GET /search/all?title`, `GET /chapters/?id&source`, `GET /proxy-image?url&hd`, `POST /download?ids[]&source&comic_title&format`, `GET /download/status/{id}`, `GET /download/file/{id}`, `POST /download/cancel/{id}`, `GET /tasks/list`, `GET /tasks/{id}/info`.

### 3.2 Modern surface (added alongside, used by the new frontend)

| Method | Path | Notes |
|---|---|---|
| `GET` | `/sources` | `[{ id: "9", slug: "bato", name: "Bato", speed: "fastest", languages: ["en"], capabilities: { search, download, needs_browser }, status: "ok" \| "down" \| "unknown" }]` |
| `GET` | `/search?q=&source=&lang=` | `source` accepts id or slug, or `all`. Response `{ results: { [source]: Comic[] }, errors: { [source]: { code, message } } }`. Partial failures do not fail the request. |
| `GET` | `/chapters?source=&id=&lang=` | `{ volumes: [{ name, chapters: [{ id, number, title?, pages? }] }] }` sorted numerically. |
| `POST` | `/download` | JSON body `{ source, comic_id, comic_title, format, chapters: [{ id, number }] }`. Returns `202 { task_id, status_url, events_url }`. No more `id_number` string splitting. |
| `GET` | `/download/status/{id}` | Same shape as today plus `file_size`, `content_type`, `created_at`, `chapter_current`, `chapter_total`. |
| `GET` | `/download/events/{id}` | SSE stream of the same status objects, closes on terminal state. |
| `GET` | `/download/file/{id}` | Streaming, see 2.6. |
| `POST` | `/download/cancel/{id}` | `200 { status: "cancelled" }`, `404`, or `409` if already finished. |
| `GET` | `/tasks` | Replaces `/tasks/list`. |
| `GET` | `/health` | `{ status, version, uptime_s, active_tasks, capabilities: { formats: [...], browser: bool } }` |
| `GET` | `/docs`, `/openapi.json` | utoipa. |

Error envelope everywhere: `{ "error": { "code": "SOURCE_UNAVAILABLE", "message": "Toonily did not respond in time", "source": "toonily" } }` with proper HTTP status (400 validation, 404 unknown task, 409 state conflict, 422 unsupported format, 502 upstream failure, 504 upstream timeout, 429 rate limited).

`Comic` shape: `{ id, title: { en, ... }, cover: { url, referer? }, languages: [..], source }`. The client builds the proxy URL from `cover`. The legacy `cover_art` field is kept in Phase 1 responses.

---

## 4. Frontend modernization

### 4.1 Stack changes (keep Vite + React 19 + Tailwind v4)

| Change | Why |
|---|---|
| Migrate to TypeScript (`.tsx`, strict) | Shared API types generated from the Rust OpenAPI spec (`openapi-typescript`). Catches the shape drift that today's `comicsArray` guessing code works around. |
| `@tanstack/react-query` | Search, chapters, sources, task status. Replaces the hand-rolled polling loop with `useQuery({ refetchInterval })` and an SSE hook. Caching and dedupe for free. |
| `zustand` (one small store) | Downloads tray state and chapter selections. Everything else is local state. |
| `motion` (import from `motion/react`) | Replaces `framer-motion` import path. Same library, current name. |
| `radix-ui` primitives | Dialog (slide-over and bottom sheet), Popover, Select, Slider (replaces the 200-line hand-rolled range slider with an accessible one), ToggleGroup (format picker), Tooltip, Toast. Unstyled, styled with Tailwind. |
| `@phosphor-icons/react` | Replaces four FontAwesome packages. One family, one weight. |
| `@fontsource-variable/geist` + `@fontsource-variable/geist-mono` | Self-hosted, `font-display: swap`. Verify package names at install; fall back to downloading the OFL files into `public/fonts`. |
| `@tanstack/react-virtual` | Virtualized chapter grid for long series. |
| `sonner` | One toast system for transient messages. |
| Remove | `express`, `cors`, `http-proxy-middleware`, `concurrently`, all `@fortawesome/*`, `framer-motion`, `tailwind.config.js`, the `dev:api` and `dev:full` scripts, the `@tailwind` v3 directives. |
| Keep | `@vercel/analytics`, `vercel.json` rewrite, Vite proxy for local dev. |

### 4.2 Design system

**Type.** Geist for UI, Geist Mono for chapter numbers, file sizes, and progress percentages (`font-variant-numeric: tabular-nums` everywhere numbers change). Scale: display `text-2xl/3xl tracking-tight font-semibold` for the wordmark and section titles, body `text-sm/base`, labels `text-xs font-medium`. No all-caps eyebrows. Sentence case throughout.

**Color.** Tailwind v4 `@theme` tokens, class-based dark mode kept (`@custom-variant dark`). One neutral family (zinc) and one accent (decision D1). Backgrounds are off-white `zinc-50` and off-black `zinc-950`, never pure black or a gradient wash. Surfaces are one step up (`zinc-100` / `zinc-900`) with hairline borders at 8% alpha instead of solid gray borders. Shadows are tinted to the background hue and used only on floating layers (tray, popovers, slide-over). Semantic colors: success emerald, error rose-red, warning amber, used only for real state.

**Shape lock.** Cards and panels 16 px, inputs and buttons 10 px, chips 8 px, tray pill full. Documented once in `@theme`.

**Motion.** Spring-based (`type: "spring", stiffness: 260, damping: 26`) for the tray, slide-over, and toasts. Results stagger in once on first render (`whileInView` is unnecessary, the list is above the fold). Layout animation on chapter selection count. `useReducedMotion` collapses everything to opacity fades. No infinite loops except the progress bar's indeterminate state.

**Icons.** Phosphor, `weight="regular"`, size 18 in controls and 16 inline.

**States.** Every async surface gets loading (skeleton shaped like the result), empty, error, and success. Buttons get `hover`, `active:scale-[0.98]`, `disabled`, and visible `focus-visible` rings. No `focus:outline-none` without a replacement.

### 4.3 Screens and components

The app stays a single page. Layout is a left-aligned header, a search bar as the primary control, results below, and a downloads tray fixed bottom-right.

**Header.** Logo mark + wordmark "Manhwa Downloader" on the left (plain text, no gradient). Right side: a Sources button (opens the status popover) and a settings menu containing the theme choice (system / light / dark as a three-way segmented control).

**SearchBar.** One composed control: source picker on the left (Radix Select showing a live status dot next to each source name and its speed tier as a subtle label), text input with label "Search titles" rendered above it, and a submit button. A checkbox-styled toggle "All sources" switches to `/search?source=all`. Query and source are mirrored into the URL (`?q=&source=`) so searches are shareable and survive reload. Recent searches (localStorage, last 8) appear as chips under the bar when the input is focused and empty.

**Results.** Grouped by source when searching all; single group otherwise. Each group is a header row (source name, result count, an inline error if that source failed) and a responsive cover grid: `grid-cols-2 sm:grid-cols-3 lg:grid-cols-5`, each card a 2:3 cover with the title below (two-line clamp) and language chips. Covers load lazily through the proxy with a shimmer skeleton and `decoding="async"`. Hovering or focusing lifts the card 2 px and shows an "Open" affordance. Clicking opens the ChapterPicker. Empty state: "No results for 'x' on Bato. Try another source or search all sources." with a one-click "Search all" action.

**ChapterPicker.** Radix Dialog rendered as a right slide-over on `md+` and a bottom sheet on mobile. Header shows cover thumbnail, title, source, and chapter count. Body:
- Range row: Radix Slider (two thumbs, keyboard accessible) with numeric "From" and "To" inputs bound to it, and buttons "Select range", "Select all", "Clear".
- Language select when the source reports more than one language (MangaDex).
- Virtualized chapter grid grouped by volume with sticky volume headers. Each chapter is a toggle chip showing the number in mono. Shift-click selects a span.
- Sticky footer: selected count ("14 chapters selected"), format ToggleGroup (PDF / CBZ / EPUB, plus CBR only when `/health` advertises it), and the primary "Download" button. Disabled with a reason tooltip when nothing is selected.
Loading state is a skeleton of the grid. Error state is inline with a retry button.

**DownloadsTray.** A single fixed pill bottom-right ("3 downloads", with a small ring showing aggregate progress). Click expands it into a panel listing every task: title, chapter count, state, progress bar, and actions (cancel while running, "Save file" when done, "Retry" on failure, dismiss). Uses SSE per task with polling fallback. When a task succeeds the file download is triggered once and the row shows "Saved" with the size. Failed tasks stay until dismissed. Tray state persists across reloads (task IDs in localStorage) so a refresh does not lose track of running jobs.

**SourceStatusPopover.** Replaces the sidebar card. A compact list of sources with a semantic dot (up / down) and speed tier text. Refreshes on open and every 5 minutes.

**Toasts.** Only for transient facts: "Download started", "Cancelled", network errors that have no inline home. Never for loading.

**Accessibility.** Skip link, landmarks (`header`, `main`, `aside` for the tray), labels above inputs, `aria-live="polite"` region for progress updates, full keyboard operation of the chapter grid (arrow keys, space to toggle, shift+arrows to extend), focus trap in the dialog, WCAG AA contrast checked in both themes.

**Performance.** Virtualized chapter list, lazy images, `React.lazy` for the ChapterPicker and SourceStatusPopover, no `useState` for pointer-driven values, bundle target under 200 KB gzipped.

### 4.4 Copy rules

Sentence case, no exclamation marks, no em-dashes, no "Oops". Errors say what happened and what to do: "Toonily did not respond. Try again or pick another source." Progress uses the server strings.

### 4.5 File structure

```
client/src/
  main.tsx, App.tsx
  styles/theme.css              # @theme tokens, dark variant, fonts
  api/                          # generated types + typed fetch client + SSE hook
  store/downloads.ts            # zustand
  hooks/                        # useSearch, useChapters, useSources, useTask
  components/
    ui/                         # Button, Input, Select, Slider, Dialog, Sheet, Chip, Skeleton, Tooltip
    layout/Header.tsx
    search/SearchBar.tsx, RecentSearches.tsx
    results/ResultsGroup.tsx, ComicCard.tsx, CoverImage.tsx
    chapters/ChapterPicker.tsx, ChapterGrid.tsx, RangeControls.tsx, FormatPicker.tsx
    downloads/DownloadsTray.tsx, TaskRow.tsx
    sources/SourceStatusPopover.tsx
  lib/format.ts, lib/url-state.ts
```

---

## 5. Deployment and repo hygiene

- **Repo layout:** `client/`, `server/`, `MODERNIZATION_PLAN.md`, root `README.md` rewritten. `server-old/` stays gitignored and is deleted once the parity checklist (section 7) is green.
- **`.gitignore`:** fix `cargo.lock` (wrong case; `Cargo.lock` must be committed for a binary), add `server/Downloads/`.
- **Dockerfile (server):** multi-stage. Stage 1 `rust:1.98-bookworm` builds with `--release` and `--locked`. Stage 2 `debian:bookworm-slim` with `ca-certificates`, optional `chromium` and `xvfb` only in the `browser` variant (`--build-arg BROWSER=1`), non-root user, `HEALTHCHECK` on `/health`. Image size target under 60 MB without browser.
- **Compose:** one service. Redis, worker, and mangapi services are gone. Keep `WEB_PORT` behavior.
- **Nixpacks:** works out of the box for the non-browser build if you prefer it on your host; the Dockerfile is the primary path because the browser variant needs system packages.
- **Client:** unchanged Vercel deploy. `vercel.json` rewrite stays pointed at the API host.
- **CI:** GitHub Actions with two jobs: `server` (fmt, clippy, test, docker build) and `client` (typecheck, lint, build). Nightly `live-smoke` job runs the ignored live tests and opens an issue on failure so you learn when a site changes its markup.
- **Docs:** README covers running each half, env vars, adding a source (trait + fixture + registry entry), and the format matrix.

---

## 6. Phases

Each phase ends in a working state. Estimates are relative effort, not calendar time.

### Phase 0. Hygiene (small)
- Fix `.gitignore`, commit `Cargo.lock`, remove dead client deps and scripts, delete `tailwind.config.js`, clean `index.css`.
- Add CI skeleton.

### Phase 1. Rust core and read-only API (medium)
- Config, error envelope, state, tracing, CORS, compression, OpenAPI.
- Models, `Source` trait, registry, `/sources`.
- MangaDex and Bato, with fixture tests.
- `/search`, `/search/all`, `/chapters`, `/status`, `/health`, `/proxy-image`, moka cache.
- Legacy path aliases so the current client runs against it.
- Exit criterion: current client can search and browse chapters on MangaDex and Bato against the Rust server.

### Phase 2. Task engine, pipeline, formats (large)
- TaskEngine, cancellation, progress, SSE, sweeper.
- Image pipeline with concurrency, verification, conversion, placeholder.
- PDF, CBZ, EPUB. CBR behind capability detection.
- File streaming with cleanup on drop, legacy `POST /download` query form plus new JSON form.
- Exit criterion: full download flow works from the current client for MangaDex and Bato in all formats.

### Phase 3. Remaining sources (large, incremental)
- 3a. Mangapill and Mangahere native (Consumet removed or made optional).
- 3b. `wreq` impersonating fetcher, Cloudflare detection, cookie cache. Madara module wired for all five sites. Asura. Weebcentral.
- 3c. Measure which sources pass without a browser. Only then decide whether to build the `browser` feature (decision D5).
- Exit criterion: parity table in section 7 green for every source that worked in Python.

**Outcome (2026-09-10).** All of 3a and 3b shipped. Live measurement with plain `reqwest` and with `wreq` Chrome impersonation:

| Source | Plain | Impersonate | Result |
|---|---|---|---|
| mangapill, mangahere, asurascans, weebcentral, ravenscans | pass | n/a | Plain tier |
| toonily | 403 challenge | pass | Impersonate tier; search, chapters, pages and image CDN (`tnlycdn.com`) all verified |
| kunmanga, manhuaus, toongod | 403 challenge | 403 `cf-mitigated: challenge` | Managed JS challenge; needs a browser. Registered with `needs_browser`, excluded from search-all, `/sources` reports `blocked`, direct calls return `503 SOURCE_BLOCKED` |

Verified end to end against the live sites: search, chapters, cover proxy and a one-chapter CBZ download for each of the six working ported sources, with zero placeholder pages. `cargo test live_ -- --ignored --nocapture` reruns this measurement.

Implementation notes: `sources/http.rs` now holds a `Fetcher` that owns both clients and walks a tier ladder (Plain -> Impersonate on a Cloudflare signature; Impersonate/Browser tiers go straight to `wreq`). The image pipeline, cover proxy and status probe all go through it, so CF cookies collected during search are reused for page images. `wreq` only publishes prerelease versions (the 5.x line was yanked), so `wreq` and `wreq-util` are pinned exactly; the build needs `cmake` and a C++ compiler for BoringSSL (present on GitHub's Ubuntu runners). `IMPERSONATION_ENABLED=false` turns the second client off. Decision D5 stands: no browser feature yet; the three blocked sites are the only candidates for it.

### Phase 4. Frontend rebuild (large)
- 4a. TypeScript, Query, generated API types, design tokens, `ui/` primitives, theme.
- 4b. Header, SearchBar, Results, ComicCard against the new API.
- 4c. ChapterPicker with virtualization and Radix Slider.
- 4d. DownloadsTray with SSE, persistence, and file save.
- 4e. Source status popover, settings menu, recent searches, URL state.
- 4f. Accessibility and reduced-motion pass, both themes audited, Lighthouse.
- Exit criterion: every flow in section 1.3 works, no FontAwesome or framer-motion imports remain, taste-skill pre-flight passes.

### Phase 5. Cutover (small)
- Dockerfile, compose, README, deploy the Rust server to the existing API host, remove legacy route aliases after one release, delete `server-old/`.

---

## 7. Parity checklist

| Capability | Python | Rust target | Verified by |
|---|---|---|---|
| Search per source (11) | yes | 8 of 11 (3 need a browser, see Phase 3) | fixture tests + live smoke |
| Search all sources | yes | yes, partial-failure aware | integration test |
| Chapters grouped by volume | yes | yes, sorted numerically | fixture tests |
| Cover proxy with referer and CF cookies | yes | yes, streaming, allowlisted | integration test |
| Source status probe | yes | yes, cached 60 s | integration test |
| Background download with progress | yes (Celery) | yes (in-process) | integration test |
| Cancel with cleanup | partial | yes | integration test |
| PDF / CBZ / EPUB | yes | yes | golden-file tests (open with a reader) |
| CBR | needs `rar` | needs `rar`, advertised | manual |
| File streaming then cleanup | yes | yes, also on client abort | integration test |
| 12 h cache for search/chapters | yes (Redis) | yes (moka) | unit test |
| Health endpoint | yes | yes, richer | integration test |
| API docs | FastAPI `/docs` | utoipa `/docs` | manual |
| Cloudflare-gated sources | Selenium | impersonation (Toonily passes); kunmanga, manhuaus, toongod need a browser and report `blocked` | live smoke (`cargo test live_ -- --ignored`) |

---

## 8. Decisions I need from you

- **D1. Accent color.** Recommendation: keep a single desaturated rose (around `#d9487b`, derived from the current `#ff5ca8` brand pink) in both themes and drop the violet and the gradients. Alternative: pick a new accent (emerald, electric blue, or burnt orange) and treat this as a rebrand.
- **D2. TypeScript.** Recommendation: yes, migrate the client to TypeScript during Phase 4 and generate API types from the Rust OpenAPI spec.
- **D3. CBR.** Recommendation: keep it, but only when the `rar` binary is present, and hide the option in the UI otherwise. Alternative: drop CBR entirely (it is a RAR-wrapped CBZ and most readers accept CBZ).
- **D4. Consumet.** Recommendation: drop it. Mangapill is trivially scrapable natively. Keep a Mangahere fallback behind `CONSUMET_URL` only until the native parser is proven, then remove.
- **D5. Headless browser.** Recommendation: do not build it in Phase 3. Ship the `wreq` impersonating client first and measure which of the seven Cloudflare-fronted sources still fail. Add the `browser` feature only for sources that need it. This keeps the default image small and the deploy simple. *Measured in Phase 3: only kunmanga, manhuaus and toongod still fail.*
- **D6. Redis.** Recommendation: remove entirely. In-process engine and cache. Revisit only if you want multiple server instances.
- **D7. API break window.** Recommendation: keep legacy paths through Phase 4, delete them in Phase 5. Alternative: version under `/v1` from day one (would require touching `vercel.json` and the Vite proxy).
- **D8. MangaDex language.** Recommendation: expose a language select in the ChapterPicker for sources that report multiple languages; default `en`.
- **D9. Frontend framework.** Recommendation: stay on Vite + React (no Next.js). The app is a single interactive page with no SEO content and is already deployed as static assets on Vercel.

Reply with the decision letters and any changes, and I will start Phase 0 and Phase 1.
