import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {FontAwesomeIcon} from '@fortawesome/react-fontawesome';
import { faCircleCheck, faCircleXmark } from '@fortawesome/free-solid-svg-icons';

function App() {
  const [darkMode, setDarkMode] = useState(() => {
    const saved = localStorage.getItem("theme");
    return saved === "dark";
  });

  const [title, setTitle] = useState("");
  const [source, setSource] = useState("0");
  const [results, setResults] = useState({});
  const [animationKey, setAnimationKey] = useState(0);
  const [error, setError] = useState("");

  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
    localStorage.setItem("theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  const handleSearch = async () => {
    try {
      const res = await fetch(`http://localhost:8000/search/?title=${encodeURIComponent(title)}&source=${source}`);
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
      setAnimationKey(prev => prev + 1);
    } catch (err) {
      setError("There was an error fetching the data");
      setResults({});
    }
  };

  return (
    <div className="min-h-screen transition-colors duration-300 bg-white text-gray-900 dark:bg-[#0d0c1b] dark:text-[#f4f4ff] relative">
      <div className="absolute top-4 right-4">
        <button
          onClick={() => setDarkMode(!darkMode)}
          className="px-4 py-2 rounded bg-pink-500 dark:bg-violet-500 text-white font-semibold"
        >
          {darkMode ? "Light Mode" : "Dark Mode"}
        </button>
      </div>

      <div className="max-w-3xl mx-auto p-6">
        <h1 className="text-2xl font-bold text-center mb-4">Manga & Manhwa Downloader</h1>

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
            className="flex-1 p-3 rounded border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff]"
          />
          <select
            value={source}
            onChange={(e) => setSource(e.target.value)}
            className="p-3 rounded border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff]"
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
            className="px-4 py-2 rounded bg-pink-500 dark:bg-violet-500 text-white font-semibold"
          >
            Search
          </button>
        </div>

        <div className="bg-gray-100 dark:bg-[#1a152b] border border-gray-300 dark:border-[#2e2b40] rounded-xl p-4 inset-shadow-xl max-h-[60vh] overflow-y-auto">
          <div className="grid grid-cols-1 gap-4">
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
                Object.entries(results).map(([key, comic]) => (
                  <motion.div
                    key={key}
                    initial={{ opacity: 0, y: -20 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 20 }}
                    transition={{ duration: 0.4, delay: parseInt(key) * 0.05 }}
                    className="flex items-center gap-4 bg-white dark:bg-[#221c35] rounded-xl p-3 shadow-md relative group"
                  >
                    <div className="relative w-20 h-28 flex-shrink-0">
                      <img
                        src={comic.cover_art}
                        alt="cover"
                        className="w-full h-full object-cover rounded-lg group-hover:opacity-50 transition-opacity"
                      />
                      <button className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                        <span className="text-white text-3xl font-bold">+</span>
                      </button>
                    </div>
                    <div className="flex-1">
                      <p className="text-lg font-semibold">
                        {comic.title.en || Object.values(comic.title)[0]}
                      </p>
                      <div className="flex flex-wrap gap-2 mt-1">
                        <span className="text-sm text-gray-500 dark:text-gray-400">Languages: </span>
                        {comic.availableLanguages.map((lang) => (
                          <span key={lang} className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                            {lang}
                            {lang === 'en' ? (
                              <FontAwesomeIcon icon={faCircleCheck} className="text-green-500" />
                            ) : (
                              <FontAwesomeIcon icon={faCircleCheck} className="text-green-500" />
                            )}
                          </span>
                        ))}
                        {!comic.availableLanguages.includes('en') && (
                          <span className="flex items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                            en
                            <FontAwesomeIcon icon={faCircleXmark} className="text-red-500" />
                          </span>
                        )}
                      </div>
                    </div>
                  </motion.div>
                ))
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
