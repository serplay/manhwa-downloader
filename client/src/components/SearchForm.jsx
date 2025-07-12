import React from "react";

const SearchForm = ({ title, setTitle, handleSearch }) => {
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
