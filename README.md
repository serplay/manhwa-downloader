
# 📚 Manga & Manhwa Downloader

A full-stack WIP (work-in-progress) project that allows users to search manga/manhwa titles from multiple sources, view available chapters, and prepare for chapter downloading .

> ⚠️ This project is under **active development**. Most functionality is not complete.

---

## ✨ Features

### Frontend (React)

- 🔍 Title search from multiple sources like MangaDex, Asurascan, Toonily, etc. (For now only MangaDex is functional)
- 🌗 Fully functional dark/light mode toggle.
- 🎨 Smooth animations via Framer Motion.
- 📚 Shows available chapters grouped by volume with dynamic dropdowns.
- ✅ Select individual chapters or select all in preparation for downloads.
- 📱 Responsive and visually modern UI with Tailwind CSS.

### Backend (FastAPI)

- 📡 API endpoints:
  - `/search/` – Fetch titles from supported sources.
  - `/chapters/` – Retrieve volume and chapter information for a given title.
  - `/download/` - Download specified chapters.
  - `/proxy-image` - Proxy cover art image through backend.
- 📥 Download-ready architecture.
- 🧼 Clean HTML scraping with BeautifulSoup.

---

## 🧰 Tech Stack

| Layer     | Tech                                                                 |
|-----------|----------------------------------------------------------------------|
| Frontend  | Vite, React, Tailwind CSS, Framer Motion, FontAwesome                |
| Backend   | FastAPI, Uvicorn, BeautifulSoup, img2pdf, requests                   |

---

## 🚀 Getting Started

### 🔧 Prerequisites

- Node.js (>=14)
- Python (>=3.10)
- npm or yarn
- pip
- Self-hosted [Consumet API](https://github.com/consumet/api.consumet.org)

---

### 📦 Backend Setup

Before running the backend, you need to create a `.env` file in the `server` directory with the following keys:

```env
VITE_API_URL=http://localhost:<your_port>
ROOT_URL=http://localhost:<your_port>
MANGAPI_URL=<address_of_your_self_hosted_consumet_api>
```

> Ensure that VITE_API_URL and ROOT_URL have the same port

Then run:

```bash
# Clone repository
git clone https://github.com/yourusername/manga-downloader.git
cd manga-downloader/server

# Create virtual environment
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows

# Install Python dependencies
pip install -r requirements.txt

# Run FastAPI server
uvicorn main:app --reload --port <your_port>
```

---

### 🎨 Frontend Setup

Before running the frontend, you also need to create a `.env` file in the `client` directory with the same keys:

```env
VITE_API_URL=http://localhost:<your_port>
ROOT_URL=http://localhost:<your_port>
MANGAPI_URL=<address_of_your_self_hosted_consumet_api>
```

> Ensure that VITE_API_URL and ROOT_URL have the same port

Then run:

```bash
cd ../client  # or wherever your React code lives

# Install Node dependencies
npm install

# Start development server
npm run dev
```

---

---

## 🛠️ To-Do

- ✅ Title search from multiple sources
- ✅ Animated dark mode toggle
- ✅ Display chapter list under each comic with dropdown
- ✅ Chapter selection UI
- ✅ Implement backend support for chapter image retrieval
- ✅ Add "Download" functionality (PDF/image bundles)
- ✅ Deploy frontend & backend
- 🔄 Write tests and error handling

---

## 🤝 Contributions

Pull requests, suggestions, and feedback are welcome! Open an issue or fork the project to get started.

---

## 👨‍💻 Author

Created by [Infinity](https://github.com/serplay) — feel free to reach out or fork this project!

---
