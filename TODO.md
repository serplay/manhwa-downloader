## 🛠️ **High Priority: Core Features & Functionality**

### 📥 Download Queue & Formats

* [ ] Refactor download logic to use **background task queue** (e.g., Celery + Redis)
* [ ] Build **download queue UI** to show queued, in-progress, and completed downloads
* [ ] Add support for **alternative formats** (CBZ, CBR, EPUB)
* [ ] Modify `/download` endpoint to accept `format=` param and respond accordingly

### 📚 Favorites / Collections System

* [ ] Add **user account system** (FastAPI Users or custom JWT-based auth)
* [ ] Create DB models: `User`, `Favorite`, `Comic`, etc.
* [ ] Add frontend ability to login/register
* [ ] Allow users to **favorite series**, store favorites in DB
* [ ] Create **"My Library"** page showing user’s saved comics

---

## 💡 **Major UX/UI Improvements**

### 🖼 Loading & Navigation

* [ ] Implement **loading states and skeleton screens**
* [ ] Add **infinite scroll** to search results

### 🔔 User Feedback

* [ ] Add **toast notifications** for actions (e.g., downloads, errors, login)
* [ ] Add **progress bar** for downloads (frontend polling or WebSockets)
* [ ] Create **"Select Range"** UI component for chapter selection
* [ ] Add **monitoring dashboard** for source status (up/down)

### 🗂 Filtering

* [ ] Add **tags/genre system** in backend + UI filter

---

## ⚙️ **Backend Enhancements**

### 📦 Scraper Architecture

* [ ] Refactor `scraper.py` into modular **plugin system** (1 file per source)
* [ ] Create `BaseScraper` abstract class with methods: `search`, `get_chapters`, `get_images`
* [ ] Dynamically load source scraper class based on `source` param

### 🔄 Caching

* [ ] Install and configure **Redis**
* [ ] Use `fastapi-cache2` to cache `/search` and `/chapters` results
* [ ] Add cache invalidation strategy (time-based or manual clear)

### 📈 Analytics & Logging

* [ ] Implement **error logging** (e.g., Sentry or basic file logs)
* [ ] Add **analytics** for most searched titles/sources
* [ ] Track download stats (count, size, user)

### 🔐 Security & Resilience

* [ ] Add **rate limiting** (e.g., via `slowapi` or `fastapi-limiter`)
* [ ] Implement **retry logic** for failed scraper/download requests
* [ ] Enable **GZip compression** for API responses
* [ ] Setup **auto-backup system** for downloaded files

---

## 🌍 **Extensions & Scalability**

* [ ] Add support for **additional manga sources** (via plugin system)
* [ ] Create **API documentation** (Swagger / OpenAPI)

---

## 📱 Mobile & Accessibility

* [ ] Make frontend a **PWA (Progressive Web App)** for offline use
* [ ] Add **keyboard navigation** support
* [ ] Implement **screen reader accessibility**
* [ ] Add **offline chapter browsing** (store local copies)

---

## 🔧 DevOps & Deployment

* [ ] Add **Docker configuration** (backend + frontend + Redis)
* [ ] Configure **health check endpoints**
* [ ] Create **auto-backup cron job** for server files

---
