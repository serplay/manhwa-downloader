# Manhwa Downloader

Search manga and manhwa across several sources, pick chapters, and download them as PDF, CBZ or EPUB. One Rust binary serves the API; a Vite and React client talks to it.

## Sources

| Source | Status |
|---|---|
| MangaDex, Asura Scans, Mangapill, Mangahere, Weebcentral, Raven Scans | Working |
| Toonily | Working through Chrome impersonation. Adult source, hidden until enabled in settings |
| Kunmanga, Manhuaus, Toongod | Listed but unavailable: their Cloudflare challenge needs a real browser |
| Bato | Registered but its mirrors currently gate non-browser clients; set `BATO_BASE_URL` if you find one that answers |

Adult content is hidden by default on the server and in the client. Turn on "Show adult content" in the client to include flagged titles and adult sources.

## Run it

### Docker

```bash
cd server
docker compose up -d --build
# API on http://localhost:8000, docs on http://localhost:8000/docs
```

Downloads are built under a named volume and stay fetchable for 15 minutes after the task finishes, then the sweeper removes them. Sending an archive does not delete it, so a transfer that breaks can be retried from the downloads tray without rebuilding. CBR output needs the proprietary `rar` binary and is only offered when it is on `PATH`; the image does not ship it.

### From source

Prerequisites: Rust 1.88 or newer, `cmake` and a C++ compiler (for BoringSSL), Node 22.

```bash
# API
cd server
cp .env.example .env    # optional
cargo run               # http://localhost:8000

# Client
cd client
npm ci
npm run dev             # http://localhost:5173, proxies /api to the server
```

Point the client at a different API with `VITE_API_TARGET=http://host:port npm run dev`.

### Deploy

- **API**: any Docker host. `server/render.yaml` is a Render blueprint for a single web service with a persistent disk. Set `CORS_ORIGINS` to the client's origin.
- **Client**: static build (`npm run build`) served by Vercel. `client/vercel.json` rewrites `/api/*` to the API host, so the client never needs the API URL at build time.

## Configuration

All settings are environment variables; see `server/.env.example`. The ones that matter most:

| Variable | Default | Purpose |
|---|---|---|
| `PORT`, `BIND` | `8000`, `0.0.0.0` | Listen address |
| `CORS_ORIGINS` | `*` | Comma-separated allowed origins |
| `DOWNLOAD_DIR` | `./Downloads` | Working directory for archives |
| `MAX_CONCURRENT_DOWNLOADS` | `2` | Parallel download jobs |
| `IMAGE_CONCURRENCY` | `6` | Parallel page fetches per chapter |
| `IMPERSONATION_ENABLED` | `true` | Chrome-impersonating client for Cloudflare-fronted sources |
| `BATO_BASE_URL` | `https://bato.si` | Bato mirror |

## API

OpenAPI docs are served at `/docs`, the raw document at `/openapi.json`, and `manhwa-server --openapi` prints it without starting the server. The client's TypeScript types are generated from it with `npm run types`.

| Route | Purpose |
|---|---|
| `GET /sources` | Every source with status, speed and capabilities |
| `GET /search?q=&source=&adult=` | Search one source or `all`; partial failures are reported per source |
| `GET /chapters?source=&id=&lang=` | Chapters grouped by volume |
| `POST /download` | Start a download task (JSON body) |
| `GET /download/status/{id}`, `GET /download/events/{id}` | Poll or stream task progress |
| `GET /download/file/{id}` | Fetch the finished archive once |
| `POST /download/cancel/{id}` | Cancel a running task |
| `GET /proxy-image?url=&referer=` | Cover proxy, allowlisted hosts only |
| `GET /health` | Version, uptime, available formats |

## Development

```bash
cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo test live_ -- --ignored --nocapture   # hits the real sites
cd client && npm run lint && npm run typecheck && npm run build
```

CI runs the same checks and builds the Docker image on every push.

## Layout

```
server/   Rust API: sources/ (one module per site), pipeline/ (images, PDF, CBZ, EPUB), tasks/ (in-process job engine), api/
client/   Vite + React 19 + TypeScript + Tailwind v4 client
```

`MODERNIZATION_PLAN.md` records how the Python and Celery version was replaced and what was decided along the way.
