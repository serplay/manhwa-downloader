# Bot protection plan

How to get the blocked sources working again without making the working ones worse. Status: B0 shipped; B1 onward is a proposal waiting on the decisions in section 5.

## 1. Where we stand (measured 2026-09-18, residential IP)

| Source | Front door | What we send today | Result |
|---|---|---|---|
| mangapill, mangahere, asurascans, weebcentral, ravenscans, mangadex | none or passive | plain `reqwest` | pass |
| toonily | Cloudflare, passive checks only (TLS / HTTP2 fingerprint) | `wreq` Chrome impersonation | pass |
| kunmanga, manhuaus, toongod | Cloudflare **managed challenge**: `403`, `cf-mitigated: challenge`, "Just a moment..." | `wreq` Chrome impersonation | blocked |
| bato | not Cloudflare. `bato.si` serves a placeholder page, `batotoo.com`/`zbato.org` serve an ad-block-detection JS redirect, `xbato.com`/`mto.to` redirect to `:444`, which times out | n/a | dead |

Impersonation beats checks that only look at *how* we connect. A managed challenge runs JavaScript in the page (proof-of-work, browser probes, sometimes Turnstile), then sets a `cf_clearance` cookie. Only something that executes the page's JS can earn that cookie, so this needs a real browser.

The nightly workflow (`.github/workflows/nightly.yml`) reruns this table every day from a GitHub runner. Runners use datacenter IPs, as the production host does, so it also shows whether a source that passes from home fails from a server.

## 2. The mechanism: borrow a browser's clearance, don't browse with it

Rendering every page and image in a browser would be slow (seconds per request) and memory-hungry. The standard approach is:

1. Hit the site with `wreq` as today.
2. On a challenge, ask a browser to load the same URL once. It solves the challenge and we keep its `cf_clearance` cookie and **exact User-Agent**.
3. Replay the cookie and UA on `wreq` for every later request to that host, pages and images alike, until it expires or gets challenged again.
4. Re-solve on the next challenge. Only one solve runs per host at a time (single-flight); other requests wait for it.

Constraints that shape everything else:

- **`cf_clearance` is bound to the solving IP and User-Agent.** The browser must send traffic from the same public IP as the API, and `wreq` must send the browser's UA exactly. A solver on a different machine is useless.
- **The TLS fingerprint should match the UA's Chrome version.** `wreq` emulates Chrome 149. Pin the solver's Chrome to a nearby major version, or pick the `wreq_util::Emulation` variant from the solver's UA.
- **Lifetime is set by the site** (commonly 30 minutes to a day). Store the expiry from the cookie, but trust the next challenge over it.
- **Images:** Madara sites usually serve pages from `wp-content/uploads` on the same Cloudflare zone, so image fetches need the cookie too. The `Fetcher` already routes images through the source's tier, so cookie injection there covers them.

## 3. Options

| | A. Solver sidecar (recommended) | B. Embedded Chrome (`chromiumoxide`) | C. Residential proxy |
|---|---|---|---|
| What | Separate container that exposes the FlareSolverr `/v1` API (FlareSolverr, or Byparr, which claims the same API). Rust calls it over HTTP | Headless Chrome driven over CDP from the Rust binary, behind the existing `browser` cargo feature | Send the blocked sources' traffic through a residential or ISP proxy |
| Pass rate | Good. Camoufox / nodriver-based solvers work on stealth hardening full-time | Poor unless we redo that stealth work ourselves (webdriver flags, headless tells) | Raises pass rate for A or B. Datacenter IPs get harder challenges |
| Image size / RAM | API image unchanged. Solver ~300-600 MB RAM, idle cost low | +400 MB image, Chrome in the API process | none |
| Failure isolation | A crashed browser doesn't take down the API | It shares the API process | n/a |
| Cost | free | free | paid per GB |
| Verdict | **Build this** | Not worth it | Only if A fails from the production IP |

Last-resort fallback within A: if a site ties clearance to a fingerprint `wreq` can't match, the solver's response already contains the page HTML. Use it directly for HTML pages (search, chapter list, reader page). This is slow but works; images would still need the cookie path.

## 4. Implementation phases

### B0. Be a quieter client first (small, benefits every source) — done 2026-09-18

Shipped: `sources/throttle.rs` (per-host concurrency cap, start spacing with jitter, shared back-off), `Retry-After` honoured in both retry loops (up to 30s, longer gives up at once), per-IP token bucket on `POST /download` returning `429 RATE_LIMITED` with `Retry-After`, and per-host counters on `/health` under `upstream`. The original notes follow.

Getting flagged is partly about volume. Today one task can open `IMAGE_CONCURRENCY` (6) × `MAX_CONCURRENT_DOWNLOADS` (2) = 12 parallel connections to one host.
- Per-host limits in `Fetcher`: a concurrency cap (for example 4) plus a token bucket with jitter.
- Honour `Retry-After` on 429 and 503 instead of fixed backoff.
- Per-IP rate limit on `POST /download` (already in TODO), so one user can't get the server's IP banned for everyone.
- Count challenges per host (`tracing` counter, and include it in `/sources` status).

### B1. Clearance store and solver client (the core)
- `sources/clearance.rs`: `ClearanceStore` keyed by host → `{ cookies, user_agent, expires_at }`, with a per-host `tokio::sync::Mutex` for single-flight solving.
- `sources/solver.rs`: FlareSolverr `/v1` client (`request.get`, `maxTimeout` 60s, optional `session` to keep the browser warm).
- `Fetcher`: for the `Browser` tier, attach the stored cookie and UA. On a challenge, solve, then retry once. If there's no solver or the solve fails, return `SOURCE_BLOCKED` (503, same as today).
- Config: `SOLVER_URL` (unset = feature off, same behaviour as now). When set, `browser_enabled` becomes true, so kunmanga, manhuaus and toongod are advertised again.
- **CI tests, no network:** a wiremock "Cloudflare" that answers `403 cf-mitigated: challenge` unless the request carries the right cookie *and* UA, plus a wiremock solver that returns that cookie. Assertions: the first request solves; later requests reuse the cookie without solving; concurrent requests trigger exactly one solve; an expired or re-challenged clearance re-solves; a wrong UA is still blocked; a failing solver maps to `SOURCE_BLOCKED`. These run in the normal `cargo test` job.

### B2. Deployment
- `docker-compose.yml`: add the solver service on the same host with `SOLVER_URL=http://solver:8191/v1`. Same machine, same egress IP.
- **Render can't do this as-is.** A Render web service is one container, the solver must share its egress IP, and Chrome won't fit comfortably in 512 MB. Either bake the solver into the API image (larger image, needs a process supervisor) on a 2 GB instance, or move the API to a small VPS running compose. See decision P1.
- Nightly: add a job that runs compose with the solver and a live download from kunmanga.

### B3. Measure from production, then decide on C
Run the nightly live download against the deployed API's network. If clearance reuse holds for at least ~30 minutes and images pass, we're done. If the sites escalate to interactive Turnstile from the server IP, try the HTML fallback, then proxies (decision P3).

### B4. Bato
Not a bot-protection problem. The mirrors are parked or broken. Either drop it from the registry, or spend a short timebox finding the current live domain (the JS redirect on `batotoo.com` may encode it). See decision P4.

## 5. Decisions needed

- **P1. Hosting for the solver.** (a) Move the API to a VPS with compose (recommended: cheapest route to same-IP). (b) Stay on Render with a combined image on a larger plan.
- **P2. Solver.** Byparr (recommended, if its FlareSolverr API compatibility holds up in B1) or FlareSolverr. Because B1 targets the shared `/v1` API, switching later is a config change.
- **P3. Paid proxy** if B3 shows the server IP can't pass: yes or no.
- **P4. Bato:** drop it, or timebox a search for a new mirror.
- **P5. Scope:** are kunmanga, manhuaus and toongod worth B1 and B2? B0 is worth doing regardless.
