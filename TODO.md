## 🛠️ **High Priority: Core Features & Functionality**

### 📥 Download Queue & Formats

* [x] ✅ **Refactor download logic to use background task queue** (Celery + Redis)
* [x] ✅ **Build download queue UI** to show queued, in-progress, and completed downloads
* [x] ✅ **Add progress tracking** for background tasks with real-time updates
* [x] ✅ **Implement task status monitoring** with health checks
* [x] ✅ **Add support for alternative formats**
  * [x] ✅ CBZ
  * [x] ✅ CBR
  * [x] ✅ ePUB
* [x] ✅ **Modify `/download` endpoint to accept `format=` param and respond accordingly**
* [x] ✅ **Add format selection dropdown** connected to download button

### 📚 Favorites / Collections System

* [ ] Add **user account system** (FastAPI Users or custom JWT-based auth)
* [ ] Create DB models: `User`, `Favorite`, `Comic`, etc.
* [ ] Add frontend ability to login/register
* [ ] Allow users to **favorite series**, store favorites in DB
* [ ] Create **"My Library"** page showing user's saved comics

---

## 💡 **Major UX/UI Improvements**

### 🖼 Loading & Navigation

* [ ] Implement **loading states and skeleton screens**
* [ ] Add **infinite scroll** to search results

### 🔔 User Feedback

* [x] ✅ **Add progress bar** for downloads (frontend polling)
* [x] ✅ **Add toast notifications** for task status updates
* [x] ✅ **Create task monitoring dashboard** showing active downloads
* [x] ✅ **Add "Select Range" UI component** for chapter selection
* [ ] Fix **Pop-up notifications** overlapping
* [x] ✅ Add **monitoring dashboard** for source status (up/down)

### 🗂 Filtering

* [ ] Add **tags/genre system** in backend + UI filter

---

## ⚙️ **Backend Enhancements**

### 📦 Scraper Architecture

* [x] ✅ Refactor `scraper.py` into modular **plugin system** (1 file per source)
* [x] ✅ Create `BaseTypes` module with types: `Comic`, `ChapterInfo`, `VolumeData`, `ComicDict`, `ChapterDict`
* [x] ✅ Dynamically load source scraper class based on `source` param

### 🔄 Caching

* [x] ✅ **Install and configure Redis** (for Celery broker)
* [x] ✅ Use `fastapi-cache2` to cache `/search` and `/chapters` results
* [x] ✅ Add cache invalidation strategy (time-based or manual clear)

### 📈 Analytics & Logging

* [x] ✅ **Implement error logging** for Celery tasks
* [ ] Add **analytics** for most searched titles/sources
* [x] ✅ Track download stats (count, size, user)

### 🔐 Security & Resilience

* [ ] Add **rate limiting** (e.g., via `slowapi` or `fastapi-limiter`)
* [x] ✅ **Implement retry logic** for failed scraper/download requests
* [x] Enable **GZip compression** for API responses
* [x] ✅ **Setup auto-cleanup system** for temporary files

---

## 🌍 **Extensions & Scalability**

* [x] ✅ Add support for **additional manga sources** (via plugin system)
* [ ] ✅ **Create API documentation** (Swagger / OpenAPI)

---

## 📱 Mobile & Accessibility

* [ ] Make frontend a **PWA (Progressive Web App)** for offline use
* [ ] Add **keyboard navigation** support
* [ ] Implement **screen reader accessibility**
* [ ] Add **offline chapter browsing** (store local copies)

---

## 🔧 DevOps & Deployment

* [x] ✅ **Add Docker configuration for api** (backend + Redis)
* [x] ✅ **Configure health check endpoints**
* [ ] Create **auto-backup cron job** for server files

---
