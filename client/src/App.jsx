import { useEffect, useState } from "react";
import logo from "./assets/logo.png";

function App() {
  const [darkMode, setDarkMode] = useState(() => {
    // Check localStorage or default to false
    const saved = localStorage.getItem("theme");
    return saved === "dark";
  });

  useEffect(() => {
    // Toggle the HTML class
    document.documentElement.classList.toggle("dark", darkMode);
    // Save preference
    localStorage.setItem("theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  return (
    <div className="min-h-screen flex flex-col transition-colors duration-300 bg-white text-gray-900 dark:bg-[#0d0c1b] dark:text-[#f4f4ff]">
      <header className="border-b border-gray-200 dark:border-[#2e2b40] bg-[#f9f9f9] dark:bg-[#1a152b]">
        <div className="max-w-full mx-auto px-4 py-3 flex justify-between items-center">
          <div className="flex items-center">
            <img src={logo} alt="logo" className="w-8 h-8 sm:w-10 sm:h-10 rounded-md" />
            <h2 className="text-base sm:text-lg font-semibold ml-2 sm:ml-4">
              Manga & Manhwa Downloader
            </h2>
          </div>
          <button
            onClick={() => setDarkMode(!darkMode)}
            className="w-24 sm:w-32 px-2 sm:px-4 py-1.5 sm:py-2 text-sm sm:text-base rounded bg-pink-500 dark:bg-violet-500 text-white font-semibold mr-2 sm:mr-4"
          >
            {darkMode ? "Light Mode" : "Dark Mode"}
          </button>
        </div>
      </header>

      <main className="flex-1 flex flex-col items-center justify-center p-8">
        <div className="w-full max-w-md p-6 bg-[#f9f9f9] dark:bg-[#1a152b] border border-gray-200 dark:border-[#2e2b40] rounded-xl shadow-md dark:shadow-violet-500/50">
          <h1 className="text-2xl font-bold text-center mb-4">
            Manga & Manhwa Downloader
          </h1>

          <input
            type="text"
            placeholder="Enter title..."
            className="w-full p-3 mb-4 rounded border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff]"
          />

          <select className="w-full p-3 mb-4 rounded border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff]">
            <option value="mangadex">MangaDex</option>
            <option value="manhuas">Manhuas</option>
            <option value="yakshascans">Yakshascans</option>
            <option value="asurascans">Asurascan</option>
            <option value="kunmanga">Kunmanga</option>
            <option value="toonily">Toonily</option>
            <option value="toongod">Toongod</option>
          </select>

          <button className="w-full cursor-pointer py-3 rounded bg-pink-500 dark:bg-violet-500 text-white font-semibold hover:opacity-90">
            Search
          </button>
        </div>
      </main>

      <footer className="border-t border-gray-200 dark:border-[#2e2b40] bg-[#f9f9f9] dark:bg-[#1a152b]">
        <div className="max-w-7xl mx-auto px-4 py-3 text-center text-sm text-gray-600 dark:text-gray-400">
          © 2025 Manga & Manhwa Downloader
        </div>
      </footer>
    </div>
  );
}

export default App;
