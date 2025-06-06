
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
  - `/download/` - Download specified chapters
- 📥 Download-ready architecture (future feature).
- 🧼 Clean HTML scraping with BeautifulSoup.

---

## 🧰 Tech Stack

| Layer     | Tech                                                                 |
|-----------|----------------------------------------------------------------------|
| Frontend  | React, Tailwind CSS, Framer Motion, FontAwesome                      |
| Backend   | FastAPI, Uvicorn, BeautifulSoup, img2pdf, requests                   |

---

## 🚀 Getting Started

### 🔧 Prerequisites

- Node.js (>=14)
- Python (>=3.10)
- npm or yarn
- pip

---

### 📦 Backend Setup

```bash
# Clone repository
git clone https://github.com/yourusername/manga-downloader.git
cd manga-downloader/backend

# Create virtual environment
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows

# Install Python dependencies
pip install -r requirements.txt

# Run FastAPI server
uvicorn main:app --port 8000
````

> Ensure your backend is running at `http://localhost:8000` for frontend requests.

---

### 🎨 Frontend Setup

```bash
cd ../frontend  # or wherever your React code lives

# Install Node dependencies
npm install

# Start development server
npm run dev
```

---

## 🛠️ To-Do

- [x] Title search from multiple sources
- [x] Animated dark mode toggle
- [x] Display chapter list under each comic with dropdown
- [x] Chapter selection UI
- [x] Implement backend support for chapter image retrieval
- [x] Add "Download" functionality (PDF/image bundles)
- [ ] Deploy frontend & backend
- [ ] Write tests and error handling

---

## 🤝 Contributions

Pull requests, suggestions, and feedback are welcome! Open an issue or fork the project to get started.

---

## 👨‍💻 Author

Created by [Infinity](https://github.com/serplay) — feel free to reach out or fork this project!

---
