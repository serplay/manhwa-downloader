import React from "react";

const sources = [
  { id: "0", name: "MangaDex" },
  { id: "1", name: "Manhuaus" },
  { id: "2", name: "Yakshascans" },
  { id: "3", name: "Asurascan" },
  { id: "4", name: "Kunmanga" },
  { id: "5", name: "Toonily" },
  { id: "6", name: "Toongod" },
  { id: "7", name: "Mangahere" },
  { id: "8", name: "Mangapill" },
  { id: "9", name: "Bato" },
  { id: "10", name: "Weebcentral" },
];

const SearchForm = ({ title, setTitle, source, setSource, handleSearch }) => {
  const handleKeyDown = (e) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  return (
    <div className="flex flex-col sm:flex-row gap-4 mb-6">
      <input
        type="text"
        value={title}
        onChange={(e) => setTitle(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Enter title..."
        className="flex-1 p-3 rounded-xl border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff] focus:outline-none focus:border-pink-500 dark:focus:border-violet-500"
      />
      <select
        value={source}
        onChange={(e) => setSource(e.target.value)}
        className="p-3 rounded-xl border border-gray-200 dark:border-[#2e2b40] bg-white dark:bg-[#1c1b29] text-gray-900 dark:text-[#f4f4ff] focus:outline-none focus:border-pink-500 dark:focus:border-violet-500 cursor-pointer"
      >
        {sources.map((src) => (
          <option key={src.id} value={src.id}>
            {src.name}
          </option>
        ))}
      </select>
      <button
        onClick={handleSearch}
        className="px-4 py-2 rounded-xl bg-pink-500 dark:bg-violet-500 cursor-pointer text-white font-semibold focus:outline-none hover:opacity-90 transition-opacity"
      >
        Search
      </button>
    </div>
  );
};

export default SearchForm;
