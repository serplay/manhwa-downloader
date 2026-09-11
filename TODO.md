# TODO

Open items after the Rust rewrite. Done items live in `MODERNIZATION_PLAN.md`.

## Sources
- [ ] Bato: find a mirror or request shape that answers non-browser clients, or move it behind the browser tier.
- [ ] Headless browser tier for Kunmanga, Manhuaus and Toongod (plan decision D5). Only worth it if those three matter.
- [ ] Per-title adult rating for Mangahere, Bato and Ravenscans (none exposed in search results today).

## Product
- [ ] Favorites and a library page (needs accounts and storage; out of scope for the rewrite).
- [ ] Genre and tag filters in search where sources expose them.
- [ ] Infinite scroll or paging for long result lists (MangaDex returns 30 per page).
- [ ] PWA manifest and offline shell.

## Operations
- [ ] Per-IP rate limit on `POST /download`.
- [ ] Ship `rar` in a variant image for CBR, if anyone still wants CBR.
- [ ] Usage analytics for searched titles and sources.
