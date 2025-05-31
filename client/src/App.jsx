import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faCircleCheck,
  faCircleXmark,
  faSun,
  faMoon,
} from "@fortawesome/free-solid-svg-icons";
import logo from "./assets/logo.png";

function App() {
  // Theme state - persists in localStorage
  const [darkMode, setDarkMode] = useState(() => {
    const saved = localStorage.getItem("theme");
    return saved === "dark";
  });

  // Search and results states
  const [title, setTitle] = useState("");
  const [source, setSource] = useState("0");
  const [results, setResults] = useState({});
  const [animationKey, setAnimationKey] = useState(0);
  const [error, setError] = useState("");

  // Chapter management states
  const [chaptersByComicId, setChaptersByComicId] = useState({});
  const [selectedChapters, setSelectedChapters] = useState({});
  const [expandedComics, setExpandedComics] = useState(new Set());
  const [isDownloading, setIsDownloading] = useState(false);

  // Apply theme changes to document
  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
    localStorage.setItem("theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  // Search for comics based on title and source
  const handleSearch = async () => {
    try {
      const res = await fetch(
        `http://localhost:8000/search/?title=${encodeURIComponent(
          title
        )}&source=${source}`
      );
      const data = await res.json();

      if (data.message === "No comics found") {
        setResults({});
        setError("No comics found");
      } else if (data.message === "Invalid source") {
        setResults({});
        setError("Invalid source");
      } else {
        setResults(data);
        setError("");
      }
      setAnimationKey((prev) => prev + 1);
    } catch (err) {
      setError("There was an error fetching the data");
      setResults({});
    }
  };

  // Fetch chapters for a specific comic
  const fetchChapters = async (comicId) => {
    try {
      const res = await fetch(
        `http://localhost:8000/chapters/?id=${comicId}&source=${source}`
      );
      const data = await res.json();

      // Process and organize chapters by volume
      const processedData = {};
      for (const volumeName in data) {
        const volume = data[volumeName];
        const displayName =
          volume.volume === "none" || !volume.volume
            ? "Unknown Volume"
            : volume.volume;
        processedData[displayName] = volume.chapters;
      }

      setChaptersByComicId((prev) => ({ ...prev, [comicId]: processedData }));
    } catch (err) {
      console.error("Error fetching chapters:", err);
    }
  };

  // Toggle selection of individual chapters
  const toggleChapterSelection = (comicId, chapterId) => {
    setSelectedChapters((prev) => {
      const current = new Set(prev[comicId] || []);
      if (current.has(chapterId)) current.delete(chapterId);
      else current.add(chapterId);
      return { ...prev, [comicId]: Array.from(current) };
    });
  };

  // Toggle selection of all chapters for a comic
  const selectAllChapters = (comicId) => {
    const allChapters = [];
    const chaptersByVolume = chaptersByComicId[comicId];
    Object.values(chaptersByVolume).forEach((chapters) => {
      Object.values(chapters).forEach((ch) => allChapters.push(ch.id));
    });

    // Check if all chapters are currently selected
    const currentSelected = selectedChapters[comicId] || [];
    const areAllSelected = allChapters.every((id) =>
      currentSelected.includes(id)
    );

    // If all are selected, deselect all. Otherwise, select all
    setSelectedChapters((prev) => ({
      ...prev,
      [comicId]: areAllSelected ? [] : allChapters,
    }));
  };

  // Handle chapter download
  const handleDownload = async (comicId) => {
    const chapterIds = selectedChapters[comicId] || [];
    if (chapterIds.length === 0) return;

    setIsDownloading(true);
    try {
      // Format chapter IDs as array parameters with chapter numbers
      const params = new URLSearchParams();
      Object.entries(chaptersByComicId[comicId]).forEach(([volumeName, chapters]) => {
        Object.entries(chapters).forEach(([chNumber, chData]) => {
          if (chapterIds.includes(chData.id)) {
            params.append("ids[]", `${chData.id}_${chNumber}`);
          }
        });
      });
      params.append("source", source);

      const response = await fetch(
        `http://localhost:8000/download/?${params.toString()}`
      );
      if (!response.ok) {
        throw new Error("Download failed");
      }
      // Handle successful download response here
    } catch (error) {
      console.error("Download error:", error);
      // Handle error here
    } finally {
      setIsDownloading(false);
    }
  };

  return (
    // Main container with theme-aware background
    <div className="min-h-screen transition-colors duration-300 bg-gradient-to-br from-white via-pink-100 to-purple-100 dark:from-[#0d0c1b] dark:via-[#1a152b] dark:to-[#2d1b4d] text-gray-900 dark:text-[#f4f4ff] relative">
      {/* Theme toggle button */}
      <div className="absolute top-4 right-4">
        <button
          onClick={() => setDarkMode(!darkMode)}
          className="w-14 h-10 rounded-full bg-pink-500 dark:bg-violet-500 text-white font-semibold cursor-pointer transition-colors duration-300 flex items-center justify-center focus:outline-none"
        >
          <AnimatePresence mode="wait">
            <motion.div
              key={darkMode ? "moon" : "sun"}
              initial={{ opacity: 0, rotate: -90 }}
              animate={{ opacity: 1, rotate: 0 }}
              exit={{ opacity: 0, rotate: 90 }}
              transition={{ duration: 0.2 }}
            >
              <FontAwesomeIcon
                icon={darkMode ? faMoon : faSun}
                className="text-lg"
              />
            </motion.div>
          </AnimatePresence>
        </button>
      </div>

      {/* Main content container */}
      <div className="max-w-3xl mx-auto p-6">
        {/* Header with logo and title */}
        <div className="flex items-center justify-center gap-4 mb-4">
          <img src={logo} alt="Logo" className="w-12 h-12 rounded-xl" />
          <button
            onClick={() => window.location.reload()}
            className="text-2xl font-bold bg-gradient-to-r from-pink-500 via-purple-500 to-pink-500 dark:from-violet-500 dark:to-[#f4f4ff] bg-clip-text text-transparent hover:opacity-80 transition-opacity cursor-pointer focus:outline-none"
          >
            Manga & Manhwa Downloader
          </button>
        </div>

        {/* Search form */}
        <div className="flex flex-col sm:flex-row gap-4 mb-6">
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                handleSearch();
              }
            }}
            placeholder="Enter title..."
            className="flex-1 p-3 rounded-xl border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff] focus:outline-none focus:border-pink-500 dark:focus:border-violet-500"
          />
          <select
            value={source}
            onChange={(e) => setSource(e.target.value)}
            className="p-3 rounded-xl border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff] hover:outline-none focus:outline-none focus:border-pink-500 dark:focus:border-violet-500"
          >
            <option value="0">MangaDex</option>
            <option value="1">Manghuas</option>
            <option value="2">Yakshascans</option>
            <option value="3">Asurascan</option>
            <option value="4">Kunmanga</option>
            <option value="5">Toonily</option>
            <option value="6">Toongod</option>
          </select>
          <button
            onClick={handleSearch}
            className="px-4 py-2 rounded-xl bg-pink-500 dark:bg-violet-500 cursor-pointer text-white font-semibold focus:outline-none hover:opacity-90 transition-opacity"
          >
            Search
          </button>
        </div>

        {/* Results section with animations */}
        <AnimatePresence>
          {(Object.keys(results).length > 0 || error) && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.3, ease: "easeOut" }}
              className="bg-gray-100 dark:bg-[#1a152b] border border-gray-300 dark:border-[#2e2b40] rounded-xl p-4 inset-shadow-xl overflow-hidden"
            >
              <div className="grid grid-cols-1 gap-4 max-h-[60vh] overflow-y-auto">
                <AnimatePresence mode="wait" key={animationKey}>
                  {error ? (
                    <motion.div
                      initial={{ opacity: 0, y: -20 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: 20 }}
                      className="text-center text-gray-600 dark:text-gray-400 py-8"
                    >
                      {error}
                    </motion.div>
                  ) : (
                    Object.entries(results).map(([key, comic], index) => (
                      <motion.div
                        key={key}
                        initial={{ opacity: 0, y: -20 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 20 }}
                        transition={{ delay: index * 0.1 }}
                        className="w-full"
                      >
                        {/* Comic card */}
                        <button
                          onClick={() => {
                            if (expandedComics.has(comic.id)) {
                              setExpandedComics((prev) => {
                                const next = new Set(prev);
                                next.delete(comic.id);
                                return next;
                              });
                            } else {
                              fetchChapters(comic.id);
                              setExpandedComics((prev) =>
                                new Set(prev).add(comic.id)
                              );
                            }
                          }}
                          className="w-full flex items-center gap-4 bg-white dark:bg-[#30274c] rounded-xl p-4 shadow-md relative group overflow-hidden hover:bg-gray-50 dark:hover:bg-[#3a2f5a] transition-colors focus:outline-none cursor-pointer"
                          id={comic.id}
                        >
                          {/* Cover art */}
                          <div className="relative w-20 h-28 flex-shrink-0">
                            <img
                              src={comic.cover_art}
                              alt="cover"
                              className="w-full h-full object-cover rounded-lg group-hover:opacity-50 transition-opacity"
                            />
                            <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                              <span className="text-white text-3xl font-bold">
                                +
                              </span>
                            </div>
                          </div>
                          {/* Comic info */}
                          <div className="flex-1 relative z-10">
                            <p className="text-lg font-semibold text-left">
                              {comic.title.en || Object.values(comic.title)[0]}
                            </p>
                            <div className="flex flex-wrap gap-2 mt-1">
                              <span className="text-sm text-gray-500 dark:text-gray-400">
                                Languages:{" "}
                              </span>
                              {comic.availableLanguages.map((lang) => (
                                <span
                                  key={lang}
                                  className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400"
                                >
                                  {lang}
                                  <FontAwesomeIcon
                                    icon={faCircleCheck}
                                    className="text-green-500"
                                  />
                                </span>
                              ))}
                              {!comic.availableLanguages.includes("en") && (
                                <span className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                  en
                                  <FontAwesomeIcon
                                    icon={faCircleXmark}
                                    className="text-red-500"
                                  />
                                </span>
                              )}
                            </div>
                          </div>
                          {/* Hover effect gradient */}
                          <div className="absolute inset-0 bg-gradient-to-tl from-pink-500/0 via-pink-500/0 to-pink-500/0 group-hover:from-pink-500/20 group-hover:via-pink-500/10 group-hover:to-pink-500/0 dark:from-violet-500/0 dark:via-violet-500/0 dark:to-violet-500/0 dark:group-hover:from-violet-500/20 dark:group-hover:via-violet-500/10 dark:group-hover:to-violet-500/0 transition-all duration-300"></div>
                        </button>

                        {/* Chapters section */}
                        {chaptersByComicId[comic.id] &&
                          expandedComics.has(comic.id) && (
                            <div className="ml-6 mt-2 bg-gray-50 dark:bg-[#201a35] rounded-lg p-4">
                              {Object.entries(chaptersByComicId[comic.id]).map(
                                ([volumeName, chapters]) => (
                                  <div key={volumeName} className="mb-4">
                                    <h4 className="font-semibold mb-2 text-pink-600 dark:text-violet-400">
                                      {volumeName}
                                    </h4>
                                    <div className="flex flex-wrap gap-2">
                                      {Object.entries(chapters).map(
                                        ([chNumber, chData]) => (
                                          <button
                                            key={chData.id}
                                            onClick={() =>
                                              toggleChapterSelection(
                                                comic.id,
                                                chData.id
                                              )
                                            }
                                            className={`px-3 py-1 rounded-md text-sm transition-colors focus:outline-none cursor-pointer ${
                                              selectedChapters[
                                                comic.id
                                              ]?.includes(chData.id)
                                                ? "bg-pink-500 text-white dark:bg-violet-500"
                                                : "bg-gray-200 dark:bg-[#2e2b40] text-gray-800 dark:text-gray-300 hover:bg-pink-100 dark:hover:bg-violet-600"
                                            }`}
                                          >
                                            {`Ch ${chNumber}`}
                                          </button>
                                        )
                                      )}
                                    </div>
                                  </div>
                                )
                              )}
                              {/* Chapter action buttons */}
                              <div className="flex gap-4 mt-2">
                                <button
                                  onClick={() => selectAllChapters(comic.id)}
                                  className="px-3 py-1 rounded-md bg-pink-500 text-white dark:bg-violet-500 hover:opacity-90 cursor-pointer"
                                >
                                  Select All
                                </button>
                                <button
                                  onClick={() => handleDownload(comic.id)}
                                  disabled={
                                    isDownloading ||
                                    !selectedChapters[comic.id]?.length
                                  }
                                  className={`px-3 py-1 rounded-md cursor-pointer ${
                                    isDownloading ||
                                    !selectedChapters[comic.id]?.length
                                      ? "bg-gray-400 cursor-not-allowed"
                                      : "bg-green-500 hover:opacity-90"
                                  } text-white`}
                                >
                                  {isDownloading
                                    ? "Downloading..."
                                    : "Download"}
                                </button>
                              </div>
                            </div>
                          )}
                      </motion.div>
                    ))
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

export default App;
