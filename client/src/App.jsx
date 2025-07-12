import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faCircleCheck,
  faCircleXmark,
  faChevronUp,
  faChevronDown,
} from "@fortawesome/free-solid-svg-icons";
import logo from "./assets/logo.png";
import HealthStatus from "./components/HealthStatus";
import ChapterRangeSlider from "./components/ChapterRangeSlider";
import ThemeToggle from "./components/ThemeToggle";
import ActiveTasksPopup from "./components/Popups/ActiveTasks";
import LoadingPopup from "./components/Popups/Loading";
import ErrorPopup from "./components/Popups/Error";
import SuccessPopup from "./components/Popups/Success";
import NotificationContainer from "./components/NotificationContainer";
import SearchForm from "./components/SearchForm";

function App() {
  const API_url = "/api";
  // Theme state - persists in localStorage
  const [darkMode, setDarkMode] = useState(() => {
    const saved = localStorage.getItem("theme");
    return saved === "dark";
  });

  // Search and results states
  const [title, setTitle] = useState("");
  const [source, setSource] = useState("0"); // Add source state
  const [resultsBySource, setResultsBySource] = useState({}); // {source_id: [results], ...}
  const [animationKey, setAnimationKey] = useState(0);
  const [error, setError] = useState("");
  const [selectedSourceForComic, setSelectedSourceForComic] = useState({}); // comicId: sourceId

  // Chapter management states
  const [chaptersByComicId, setChaptersByComicId] = useState({});
  const [selectedChapters, setSelectedChapters] = useState({});
  const [expandedComics, setExpandedComics] = useState(new Set());
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadError, setDownloadError] = useState("");
  const [isFetchingChapters, setIsFetchingChapters] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [loadedImages, setLoadedImages] = useState(new Set());

  // Range selection states
  const [chapterRanges, setChapterRanges] = useState({});

  // Chapter dropdown states
  const [expandedChapterSections, setExpandedChapterSections] = useState(
    new Set()
  );

  // Background task states
  const [activeTasks, setActiveTasks] = useState({});
  const [taskStatuses, setTaskStatuses] = useState({});
  const [downloadingFiles, setDownloadingFiles] = useState(new Set());
  const [downloadSuccess, setDownloadSuccess] = useState("");

  // Format selection states
  const [selectedFormat, setSelectedFormat] = useState({});

  // Show health status
  const [showHealthStatus, setShowHealthStatus] = useState(false);
  const [isHealthPanelOpen, setIsHealthPanelOpen] = useState(() => {
    // Check screen width on load
    return window.innerWidth >= 768; // 768px is md breakpoint in Tailwind
  });

  // Get source name by ID
  const getSourceName = (sourceId) => {
    const sourceNames = {
      "0": "MangaDex",
      "1": "Manhuaus", 
      "2": "Yakshascans",
      "3": "Asurascan",
      "4": "Kunmanga",
      "5": "Toonily",
      "6": "Toongod",
      "7": "Mangahere",
      "8": "Mangapill",
      "9": "Bato",
      "10": "Weebcentral"
    };
    return sourceNames[sourceId] || `Source ${sourceId}`;
  };

  // State for collapsed sources
  const [collapsedSources, setCollapsedSources] = useState(new Set());

  // Apply theme changes to document
  useEffect(() => {
    document.documentElement.classList.toggle("dark", darkMode);
    localStorage.setItem("theme", darkMode ? "dark" : "light");
  }, [darkMode]);

  // Polling for active tasks
  useEffect(() => {
    const pollActiveTasks = async () => {
      const activeTaskIds = Object.keys(activeTasks);
      if (activeTaskIds.length === 0) return;

      for (const taskId of activeTaskIds) {
        try {
          const response = await fetch(`${API_url}/download/status/${taskId}`);
          if (response.ok) {
            const status = await response.json();
            setTaskStatuses((prev) => ({ ...prev, [taskId]: status }));

            // If task completed successfully, automatically download the file
            if (status.state === "SUCCESS") {
              // Check if the file is not already downloading
              if (!downloadingFiles.has(taskId)) {
                console.log(
                  `DEBUG: Task ${taskId} completed successfully, downloading file...`
                );

                // Mark file as downloading
                setDownloadingFiles((prev) => new Set(prev).add(taskId));

                try {
                  // Get comic title from activeTasks
                  const taskInfo = activeTasks[taskId];
                  const comicTitle = taskInfo?.comicTitle || "Chapters";

                  await downloadCompletedFile(taskId, comicTitle);
                  console.log(
                    `DEBUG: File for task ${taskId} downloaded successfully`
                  );
                } catch (error) {
                  console.error(
                    `DEBUG: Error during automatic file download for task ${taskId}:`,
                    error
                  );
                  setDownloadError(
                    `Automatic file download failed: ${error.message}`
                  );
                } finally {
                  // Remove from downloading files list
                  setDownloadingFiles((prev) => {
                    const newSet = new Set(prev);
                    newSet.delete(taskId);
                    return newSet;
                  });
                }
              }
            }

            // If task finished (success or failure), handle removal
            if (status.state === "SUCCESS") {
              // Remove successful tasks immediately
              setActiveTasks((prev) => {
                const newTasks = { ...prev };
                delete newTasks[taskId];
                return newTasks;
              });
            } else if (status.state === "FAILURE") {
              // Extract error message from failed task
              const errorMessage = status.error || status.status || "Download failed";
              const comicTitle = status.comic_title || activeTasks[taskId]?.comicTitle || "Chapters";
              setDownloadError(`Failed to download ${comicTitle}.\n${errorMessage}`);
              
              // Delay removal for failed tasks to make the error visible
              setTimeout(() => {
                setActiveTasks((prev) => {
                  const newTasks = { ...prev };
                  delete newTasks[taskId];
                  return newTasks;
                });
              }, 1000); // 1-second delay
            }
          } else {
            // If response not OK, set task status as FAILURE and remove it after a delay
            console.error(
              `Status check for ${taskId} failed with status: ${response.status}`
            );
            const comicTitle = activeTasks[taskId]?.comicTitle || "Chapters";
            setDownloadError(`Failed to check download status for ${comicTitle}.\nNetwork error: ${response.status}`);
            setTaskStatuses((prev) => ({
              ...prev,
              [taskId]: { state: "FAILURE" },
            }));
            setTimeout(() => {
              setActiveTasks((prev) => {
                const newTasks = { ...prev };
                delete newTasks[taskId];
                return newTasks;
              });
            }, 3000);
          }
        } catch (error) {
          console.error(`Error checking status of task ${taskId}:`, error);
          const comicTitle = activeTasks[taskId]?.comicTitle || "Chapters";
          setDownloadError(`Failed to check download status for ${comicTitle}.\n${error.message}`);
          // Mark task as failed and remove it after a delay
          setTaskStatuses((prev) => ({
            ...prev,
            [taskId]: { state: "FAILURE" },
          }));
          setTimeout(() => {
            setActiveTasks((prev) => {
              const newTasks = { ...prev };
              delete newTasks[taskId];
              return newTasks;
            });
          }, 1000);
        }
      }
    };

    const interval = setInterval(pollActiveTasks, 2000); // Check every 2 seconds
    return () => clearInterval(interval);
  }, [activeTasks, downloadingFiles]);

  // Search in all sources at once
  const handleSearch = async () => {
    setDownloadError("");
    setIsSearching(true);
    setShowHealthStatus(false);
    setResultsBySource({});
    setError("");
    setAnimationKey((prev) => prev + 1);
    try {
      const res = await fetch(
        `${API_url}/search/all?title=${encodeURIComponent(title)}`
      );
      if (!res.ok) {
        const errorData = await res.json();
        const errorMessage = errorData.error || "Failed to fetch data";
        throw new Error(errorMessage);
      }
      const data = await res.json();
      setResultsBySource(data);
      setError("");
    } catch (err) {
      console.error("Search error:", err); // Debug log
      setError("An error occurred while fetching data");
      setResultsBySource({});
    } finally {
      setIsSearching(false);
    }
  };

  // Fetch chapters for a specific comic and source
  const fetchChapters = async (comicId, sourceId) => {
    setDownloadError("");
    setIsFetchingChapters(true);
    try {
      const res = await fetch(
        `${API_url}/chapters/?id=${comicId}&source=${sourceId}`
      );
      if (!res.ok) {
        const errorData = await res.json();
        const errorMessage = errorData.error || "Failed to fetch chapters";
        throw new Error(errorMessage);
      }
      const data = await res.json();
      if (data.error) {
        throw new Error(data.error);
      }

      // Process and organize chapters by volume
      const processedData = {};
      for (const volumeName in data) {
        const volume = data[volumeName];
        const displayName =
          volume.volume === "none" || !volume.volume
            ? "Unknown Volume"
            : volume.volume;
        // Convert chapters object to array and sort by chapter number
        const chaptersArray = Object.entries(volume.chapters)
          .map(([key, value]) => ({
            key,
            ...value,
          }))
          .sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));
        processedData[displayName] = chaptersArray;
      }

      setChaptersByComicId((prev) => ({ ...prev, [`${comicId}_${sourceId}`]: processedData }));
    } catch (err) {
      console.error("Error fetching chapters:", err);
      setDownloadError("Failed to fetch chapters.\n" + err.message);
      setExpandedComics((prev) => {
        const next = new Set(prev);
        next.delete(`${comicId}_${sourceId}`);
        return next;
      });
    } finally {
      setIsFetchingChapters(false);
    }
  };

  // Toggle selection of individual chapters
  const toggleChapterSelection = (comicKey, chapterId) => {
    setSelectedChapters((prev) => {
      const current = new Set(prev[comicKey] || []);
      if (current.has(chapterId)) current.delete(chapterId);
      else current.add(chapterId);
      return { ...prev, [comicKey]: Array.from(current) };
    });
  };

  // Toggle selection of all chapters for a comic (per source)
  const selectAllChapters = (comicKey) => {
    const allChapters = [];
    const chaptersByVolume = chaptersByComicId[comicKey];
    Object.values(chaptersByVolume).forEach((chapters) => {
      Object.values(chapters).forEach((ch) => allChapters.push(ch.id));
    });

    // Check if all chapters are currently selected
    const currentSelected = selectedChapters[comicKey] || [];
    const areAllSelected = allChapters.every((id) =>
      currentSelected.includes(id)
    );

    // If all are selected, deselect all. Otherwise, select all
    setSelectedChapters((prev) => ({
      ...prev,
      [comicKey]: areAllSelected ? [] : allChapters,
    }));
  };

  // Handle chapter download - per comic and source
  const handleDownload = async (comicId, sourceId, comicTitle, format = "pdf") => {
    const comicKey = `${comicId}_${sourceId}`;
    const chapterIds = selectedChapters[comicKey] || [];
    if (chapterIds.length === 0) return;

    setIsDownloading(true);
    setDownloadError(""); // Clear any previous errors
    try {
      // Format chapter IDs as array parameters with chapter numbers
      const params = new URLSearchParams();
      Object.entries(chaptersByComicId[comicKey]).forEach(
        ([volumeName, chapters]) => {
          Object.entries(chapters).forEach(([chNumber, chData]) => {
            if (chapterIds.includes(chData.id)) {
              params.append("ids[]", `${chData.id}_${chData.chapter}`);
            }
          });
        }
      );
      params.append("source", sourceId);
      params.append("comic_title", comicTitle);
      params.append("format", format);

      const response = await fetch(`${API_url}/download?${params.toString()}`, {
        method: "POST",
      });

      if (!response.ok) {
        const errorData = await response.json();
        const errorMessage = errorData.detail || "Failed to start download";
        throw new Error(errorMessage);
      }

      const data = await response.json();

      // Add task to active tasks
      setActiveTasks((prev) => ({
        ...prev,
        [data.task_id]: {
          comicId,
          comicTitle,
          chapterCount: chapterIds.length,
          startTime: new Date().toISOString(),
        },
      }));

      // Clear selected chapters
      setSelectedChapters((prev) => {
        const newSelected = { ...prev };
        delete newSelected[comicKey];
        return newSelected;
      });
    } catch (error) {
      console.error("Download error:", error);
      setDownloadError(
        "Failed to start download. Please try again later.\n" + error.message
      );
    } finally {
      setIsDownloading(false);
    }
  };

  // Download completed file
  const downloadCompletedFile = async (taskId, comicTitle = "Chapters") => {
    try {
      console.log(
        `DEBUG: Attempting to download file for task ${taskId}, title: ${comicTitle}`
      );

      // Use <a> element with download attribute
      const downloadUrl = `${API_url}/download/file/${taskId}`;
      console.log(`DEBUG: Creating download link: ${downloadUrl}`);

      const a = document.createElement("a");
      a.href = downloadUrl;
      a.style.display = "none";
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);

      console.log(`DEBUG: File ${comicTitle} sent for download`);

      // Show success notification
      setDownloadSuccess(`File ${comicTitle} has been sent for download!`);

      // Hide notification after 5 seconds
      setTimeout(() => setDownloadSuccess(""), 5000);
    } catch (error) {
      console.error("Error downloading file:", error);
      setDownloadError("Failed to download completed file: " + error.message);
    }
  };

  // Get task status display
  const getTaskStatusDisplay = (taskId) => {
    const status = taskStatuses[taskId];
    if (!status) return "Waiting...";

    switch (status.state) {
      case "PENDING":
        return "In queue...";
      case "PROGRESS":
        return `Processing... ${status.progress || 0}%`;
      case "SUCCESS":
        return "Completed";
      case "FAILURE":
        return "Downloading failed";
      default:
        return "Unknown status";
    }
  };

  // Handle range selection
  const handleRangeChange = (comicKey, { start, end }) => {
    setChapterRanges((prev) => ({ ...prev, [comicKey]: { start, end } }));

    // Don't automatically select chapters - let users choose manually
    // The range is just for visual reference and quick actions
  };

  // Select chapters within the current range
  const selectChaptersInRange = (comicKey) => {
    const range = chapterRanges[comicKey];
    if (!range) return;

    // Get all chapters for this comic
    const allChapters = [];
    Object.values(chaptersByComicId[comicKey] || {}).forEach((volume) => {
      allChapters.push(...volume);
    });

    // Sort chapters by chapter number
    allChapters.sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));

    // Select chapters within the range
    const selectedInRange = allChapters
      .slice(range.start, range.end + 1)
      .map((ch) => ch.id);

    setSelectedChapters((prev) => ({
      ...prev,
      [comicKey]: selectedInRange,
    }));
  };

  // Clear all selections for a comic
  const clearChapterSelections = (comicKey) => {
    setSelectedChapters((prev) => ({
      ...prev,
      [comicKey]: [],
    }));
  };

  // Get chapter range info for slider
  const getChapterRangeInfo = (comicKey) => {
    const allChapters = [];
    Object.values(chaptersByComicId[comicKey] || {}).forEach((volume) => {
      allChapters.push(...volume);
    });

    if (allChapters.length === 0) return { min: 0, max: 0 };

    // Sort chapters by chapter number
    allChapters.sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));

    return {
      min: 0,
      max: allChapters.length - 1,
      chapters: allChapters,
    };
  };


  const cancelTask = async (taskId) => {
    try {
      const response = await fetch(`${API_url}/download/cancel/${taskId}`, {
        method: "POST",
      });
      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.detail || "Failed to cancel task");
      }

      setActiveTasks((prev) => {
        const newTasks = { ...prev };
        delete newTasks[taskId];
        return newTasks;
      });
      setTaskStatuses((prev) => {
        const newStatuses = { ...prev };
        delete newStatuses[taskId];
        return newStatuses;
      });
    } catch (error) {
      setDownloadError(`Failed to cancel download: ${error.message}`);
    }
  };

  return (
    <div className="min-h-screen transition-colors duration-300 bg-gradient-to-br from-white via-pink-100 to-purple-100 dark:from-[#0d0c1b] dark:via-[#1a152b] dark:to-[#2d1b4d] text-gray-900 dark:text-[#f4f4ff] relative">
      <NotificationContainer>
        <ErrorPopup
          downloadError={downloadError}
          setDownloadError={setDownloadError}
        />
        <SuccessPopup
          downloadSuccess={downloadSuccess}
          setDownloadSuccess={setDownloadSuccess}
        />
        <LoadingPopup
          isFetchingChapters={isFetchingChapters}
          isSearching={isSearching}
        />
      </NotificationContainer>

      <ActiveTasksPopup
        activeTasks={activeTasks}
        taskStatuses={taskStatuses}
        downloadingFiles={downloadingFiles}
        getTaskStatusDisplay={getTaskStatusDisplay}
        onCancelTask={cancelTask}
      />

      <ThemeToggle darkMode={darkMode} setDarkMode={setDarkMode} />

      <div className="max-w-3xl w-full mx-auto p-2 sm:p-4 md:p-6">
        <div className="flex flex-col sm:flex-row items-center justify-center gap-2 sm:gap-4 mb-4">
          <img src={logo} alt="Logo" className="w-10 h-10 sm:w-12 sm:h-12 rounded-xl" />
          <button
            onClick={() => window.location.reload()}
            className="text-xl sm:text-2xl font-bold bg-gradient-to-r from-pink-500 via-purple-500 to-pink-500 dark:from-violet-500 dark:to-[#f4f4ff] bg-clip-text text-transparent hover:opacity-80 transition-opacity cursor-pointer focus:outline-none"
          >
            Manga & Manhwa Downloader
          </button>
        </div>

        {/* Search form */}
        <SearchForm
          title={title}
          setTitle={setTitle}
          handleSearch={handleSearch}
        />

        {/* Results from multiple sources */}
        <AnimatePresence>
          {(Object.keys(resultsBySource).length > 0 || error) && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: "auto", opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              transition={{ duration: 0.3, ease: "easeOut" }}
              className="bg-gray-100 dark:bg-[#1a152b] border border-gray-300 dark:border-[#2e2b40] rounded-xl p-2 sm:p-4 inset-shadow-xl overflow-hidden"
            >
              <div className="grid grid-cols-1 gap-2 sm:gap-4 max-h-[60vh] overflow-y-auto">
                <AnimatePresence mode="sync" key={animationKey}>
                  {error ? (
                    <motion.div
                      initial={{ opacity: 0, y: -20 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: 20 }}
                      className="text-center text-gray-600 dark:text-gray-400 py-4 sm:py-8 text-base sm:text-lg"
                    >
                      {error}
                    </motion.div>
                  ) : (
                    Object.entries(resultsBySource).map(([sourceId, comics], srcIdx) => {
                      console.log(`Source ${sourceId}:`, comics); // Debug log
                      console.log(`Source ${sourceId} type:`, typeof comics); // Debug log
                      console.log(`Source ${sourceId} isArray:`, Array.isArray(comics)); // Debug log
                      
                      // Handle different data structures
                      let comicsArray = [];
                      if (Array.isArray(comics)) {
                        comicsArray = comics;
                      } else if (comics && typeof comics === 'object') {
                        // If it's an object, try to extract the comics array
                        if (comics.comics && Array.isArray(comics.comics)) {
                          comicsArray = comics.comics;
                        } else if (comics.results && Array.isArray(comics.results)) {
                          comicsArray = comics.results;
                        } else if (comics.data && Array.isArray(comics.data)) {
                          comicsArray = comics.data;
                        } else {
                          // If it's an object but not the expected structure, try to convert it
                          comicsArray = Object.values(comics).filter(item => 
                            item && typeof item === 'object' && item.id
                          );
                        }
                      }
                      
                      const isCollapsed = collapsedSources.has(sourceId);
                      
                      return (
                        <div key={sourceId} className={`${isCollapsed ? 'mb-1' : 'mb-6'}`}>
                          {/* Source header with dropdown */}
                          <button
                            onClick={() => {
                              const newCollapsed = new Set(collapsedSources);
                              if (newCollapsed.has(sourceId)) {
                                newCollapsed.delete(sourceId);
                              } else {
                                newCollapsed.add(sourceId);
                              }
                              setCollapsedSources(newCollapsed);
                            }}
                            className="w-full flex items-center justify-between p-3 bg-white dark:bg-[#30274c] rounded-lg shadow-sm hover:bg-gray-50 dark:hover:bg-[#3a2f5a] transition-colors cursor-pointer focus:outline-none"
                          >
                            <div className="flex items-center gap-3">
                              <FontAwesomeIcon
                                icon={isCollapsed ? faChevronDown : faChevronUp}
                                className="text-pink-600 dark:text-violet-400 text-sm transition-transform duration-200"
                              />
                              <span className="font-bold text-pink-600 dark:text-violet-400 text-lg">
                                {getSourceName(sourceId)}
                              </span>
                              <span className="text-sm text-gray-500 dark:text-gray-400">
                                ({comicsArray.length} results)
                              </span>
                            </div>
                          </button>
                          
                          {/* Comics list */}
                          <AnimatePresence mode="sync">
                            {!isCollapsed && (
                              <motion.div
                                initial={{ height: 0, opacity: 0 }}
                                animate={{ height: "auto", opacity: 1 }}
                                exit={{ height: 0, opacity: 0 }}
                                transition={{ duration: 0.3, ease: "easeInOut" }}
                                className="mt-3 space-y-3"
                              >
                                {comicsArray.length === 0 && (
                                  <div className="text-gray-500 text-sm p-4 bg-gray-50 dark:bg-[#201a35] rounded-lg">
                                    No results
                                    <details className="mt-2 text-xs">
                                      <summary className="cursor-pointer">Debug: Raw data</summary>
                                      <pre className="mt-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs overflow-auto">
                                        {JSON.stringify(comics, null, 2)}
                                      </pre>
                                    </details>
                                  </div>
                                )}
                                {comicsArray.length > 0 && comicsArray.map((comic, idx) => {
                                  const comicKey = `${comic.id}_${sourceId}`;
                                  return (
                                    <motion.div
                                      key={comicKey}
                                      initial={{ opacity: 0, y: -20 }}
                                      animate={{ opacity: 1, y: 0 }}
                                      exit={{ opacity: 0, y: 20 }}
                                      transition={{ delay: idx * 0.1 }}
                                      className="w-full"
                                    >
                                      {/* Comic card */}
                                      <button
                                        onClick={() => {
                                          if (expandedComics.has(comicKey)) {
                                            setExpandedComics((prev) => {
                                              const next = new Set(prev);
                                              next.delete(comicKey);
                                              return next;
                                            });
                                          } else {
                                            fetchChapters(comic.id, sourceId);
                                            setExpandedComics((prev) => new Set(prev).add(comicKey));
                                          }
                                        }}
                                        className="w-full flex flex-col sm:flex-row items-center gap-2 sm:gap-4 bg-white dark:bg-[#30274c] rounded-xl p-2 sm:p-4 shadow-md relative group overflow-hidden hover:bg-gray-50 dark:hover:bg-[#3a2f5a] transition-colors focus:outline-none cursor-pointer"
                                        id={comicKey}
                                      >
                                        {/* Cover art */}
                                        <div className="relative w-20 h-28 flex-shrink-0 mb-2 sm:mb-0">
                                          {!loadedImages.has(comic.cover_art) && (
                                            <div className="absolute inset-0 flex items-center justify-center bg-gray-200 dark:bg-[#2e2b40] rounded-lg">
                                              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-pink-500 dark:border-violet-500"></div>
                                            </div>
                                          )}
                                          <img
                                            src={comic.cover_art}
                                            alt="cover"
                                            className={`w-full h-full object-cover rounded-lg group-hover:opacity-50 transition-opacity ${
                                              !loadedImages.has(comic.cover_art)
                                                ? "opacity-0"
                                                : "opacity-100"
                                            }`}
                                            onLoad={() =>
                                              setLoadedImages((prev) =>
                                                new Set(prev).add(comic.cover_art)
                                              )
                                            }
                                          />
                                          <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                                            <span className="text-white text-2xl sm:text-3xl font-bold">
                                              +
                                            </span>
                                          </div>
                                        </div>
                                        {/* Comic info */}
                                        <div className="flex-1 relative z-10 w-full">
                                          <p className="text-base sm:text-lg font-semibold text-left">
                                            {comic.title?.en || Object.values(comic.title || {})[0]}
                                          </p>
                                          <div className="flex flex-wrap gap-1 sm:gap-2 mt-1">
                                            <span className="text-xs sm:text-sm text-gray-500 dark:text-gray-400">
                                              Languages: {" "}
                                            </span>
                                            {comic.availableLanguages?.map((lang) => (
                                              <span
                                                key={lang}
                                                className="flex items-center gap-1 text-xs sm:text-sm text-gray-500 dark:text-gray-400"
                                              >
                                                {lang}
                                                <FontAwesomeIcon
                                                  icon={faCircleCheck}
                                                  className="text-green-500"
                                                />
                                              </span>
                                            ))}
                                            {comic.availableLanguages && !comic.availableLanguages.includes("en") && (
                                              <span className="flex items-center gap-1 text-xs sm:text-sm text-gray-500 dark:text-gray-400">
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
                                      {chaptersByComicId[comicKey] && expandedComics.has(comicKey) && (
                                        <div className="ml-0 sm:ml-6 mt-2 bg-gray-50 dark:bg-[#201a35] rounded-lg p-2 sm:p-4">
                                          {/* Chapter Range Slider */}
                                          <div className="mb-2 sm:mb-4">
                                            <button
                                              onClick={() => {
                                                if (expandedChapterSections.has(comicKey)) {
                                                  setExpandedChapterSections((prev) => {
                                                    const next = new Set(prev);
                                                    next.delete(comicKey);
                                                    return next;
                                                  });
                                                } else {
                                                  setExpandedChapterSections((prev) => new Set(prev).add(comicKey));
                                                }
                                              }}
                                              className="flex items-center gap-2 text-pink-600 dark:text-violet-400 font-semibold hover:opacity-80 transition-opacity text-sm sm:text-base"
                                            >
                                              <span>Quick Chapter Selection</span>
                                              <FontAwesomeIcon
                                                icon={expandedChapterSections.has(comicKey) ? faChevronUp : faChevronDown}
                                                className="text-xs sm:text-sm transition-transform duration-200"
                                              />
                                            </button>

                                            <AnimatePresence initial={false}>
                                              {expandedChapterSections.has(comicKey) && (
                                                <motion.div
                                                  key="quick-chapter-dropdown"
                                                  initial={{ height: 0, opacity: 0 }}
                                                  animate={{ height: "auto", opacity: 1 }}
                                                  exit={{ height: 0, opacity: 0 }}
                                                  transition={{ duration: 0.3, ease: "easeOut" }}
                                                  className="overflow-hidden mt-2 sm:mt-3"
                                                >
                                                  {(() => {
                                                    const rangeInfo = getChapterRangeInfo(comicKey);
                                                    if (rangeInfo.max > 0) {
                                                      const currentRange = chapterRanges[comicKey];
                                                      const selectedCount = selectedChapters[comicKey]?.length || 0;

                                                      return (
                                                        <div>
                                                          <ChapterRangeSlider
                                                            min={rangeInfo.min}
                                                            max={rangeInfo.max}
                                                            onRangeChange={(range) => handleRangeChange(comicKey, range)}
                                                            initialStart={currentRange?.start || rangeInfo.min}
                                                            initialEnd={currentRange?.end || rangeInfo.max}
                                                          />

                                                          {/* Range Info and Action Buttons */}
                                                          {currentRange && (
                                                            <div className="mt-2 sm:mt-3 p-2 sm:p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                                                              <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-1 sm:mb-2 gap-1 sm:gap-0">
                                                                <span className="text-xs sm:text-sm text-blue-700 dark:text-blue-300">
                                                                  Range: Chapter {rangeInfo.chapters[currentRange.start]?.chapter} - Chapter {rangeInfo.chapters[currentRange.end]?.chapter}
                                                                </span>
                                                                <span className="text-xs sm:text-sm text-gray-600 dark:text-gray-400">
                                                                  {selectedCount} selected
                                                                </span>
                                                              </div>
                                                              <div className="flex gap-2 flex-wrap">
                                                                <button
                                                                  onClick={() => selectChaptersInRange(comicKey)}
                                                                  className="px-2 sm:px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
                                                                >
                                                                  Select Range
                                                                </button>
                                                                <button
                                                                  onClick={() => clearChapterSelections(comicKey)}
                                                                  className="px-2 sm:px-3 py-1 text-xs bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors"
                                                                >
                                                                  Clear All
                                                                </button>
                                                              </div>
                                                            </div>
                                                          )}
                                                        </div>
                                                      );
                                                    }
                                                    return null;
                                                  })()}
                                                </motion.div>
                                              )}
                                            </AnimatePresence>
                                          </div>

                                          {/* Individual Chapter Selection */}
                                          <div className="mb-2 sm:mb-4">
                                            <h4 className="font-semibold mb-1 sm:mb-2 text-pink-600 dark:text-violet-400 text-sm sm:text-base">
                                              Chapters
                                            </h4>
                                            {Object.entries(chaptersByComicId[comicKey]).map(([volumeName, chapters]) => (
                                              <div key={volumeName} className="mb-2 sm:mb-4">
                                                <h4 className="font-semibold mb-1 sm:mb-2 text-pink-600 dark:text-violet-400 text-xs sm:text-base">
                                                  {volumeName}
                                                </h4>
                                                <div className="flex flex-wrap gap-1 sm:gap-2">
                                                  {chapters.map((chData) => (
                                                    <button
                                                      key={chData.id}
                                                      onClick={() => toggleChapterSelection(comicKey, chData.id)}
                                                      className={`px-2 sm:px-3 py-1 rounded-md text-xs sm:text-sm transition-colors focus:outline-none cursor-pointer ${
                                                        selectedChapters[comicKey]?.includes(chData.id)
                                                          ? "bg-pink-500 text-white dark:bg-violet-500"
                                                          : "bg-gray-200 dark:bg-[#2e2b40] text-gray-800 dark:text-gray-300 hover:bg-pink-100 dark:hover:bg-violet-600"
                                                      }`}
                                                    >
                                                      {`Ch ${chData.chapter}`}
                                                    </button>
                                                  ))}
                                                </div>
                                              </div>
                                            ))}
                                          </div>

                                          {/* Chapter action buttons */}
                                          <div className="flex flex-col sm:flex-row gap-2 sm:gap-4 mt-2">
                                            <button
                                              onClick={() => selectAllChapters(comicKey)}
                                              className="px-2 sm:px-3 py-1 rounded-md bg-pink-500 rounded-r-none text-white dark:bg-violet-500 hover:opacity-90 cursor-pointer text-xs sm:text-base"
                                            >
                                              Select All
                                            </button>
                                            <div className="flex">
                                              <button
                                                onClick={() =>
                                                  handleDownload(
                                                    comic.id,
                                                    sourceId,
                                                    comic.title?.en || Object.values(comic.title || {})[0],
                                                    selectedFormat[comicKey] || "pdf"
                                                  )
                                                }
                                                disabled={
                                                  isDownloading ||
                                                  !selectedChapters[comicKey]?.length
                                                }
                                                className={`px-2 sm:px-3 py-1 rounded-l-md border-r-0 cursor-pointer text-xs sm:text-base ${
                                                  isDownloading ||
                                                  !selectedChapters[comicKey]?.length
                                                    ? "bg-gray-400 cursor-not-allowed"
                                                    : "bg-green-500 hover:opacity-90"
                                                } text-white`}
                                              >
                                                {isDownloading ? "Starting..." : "Download"}
                                              </button>
                                              <select
                                                value={selectedFormat[comicKey] || "pdf"}
                                                onChange={(e) =>
                                                  setSelectedFormat((prev) => ({
                                                    ...prev,
                                                    [comicKey]: e.target.value,
                                                  }))
                                                }
                                                className="px-2 py-1 rounded-r-md rounded-l-none border border-l-0 border-gray-300 dark:border-gray-600 bg-white dark:bg-[#2e2b40] text-gray-800 dark:text-gray-200 focus:outline-none text-xs sm:text-base"
                                                style={{ minWidth: 70 }}
                                              >
                                                <option value="pdf">PDF</option>
                                                <option value="cbz">CBZ</option>
                                                <option value="cbr">CBR</option>
                                                <option value="epub">ePUB</option>
                                              </select>
                                            </div>
                                          </div>
                                        </div>
                                      )}
                                    </motion.div>
                                  );
                                })}
                                {comics && comics.error && (
                                  <div className="text-red-500 text-sm p-4 bg-red-50 dark:bg-red-900/20 rounded-lg">
                                    Error: {comics.error}
                                  </div>
                                )}
                              </motion.div>
                            )}
                          </AnimatePresence>
                        </div>
                      );
                    })
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
      {showHealthStatus && <HealthStatus />}
      
      {/* Health Status Marker */}
      {!showHealthStatus && (
        <div className="fixed left-0 top-1/2 transform -translate-y-1/2 z-40">
          <button
            onClick={() => setIsHealthPanelOpen(!isHealthPanelOpen)}
            className="bg-pink-500 dark:bg-violet-500 text-white p-3 rounded-r-lg shadow-lg hover:opacity-90 transition-opacity cursor-pointer focus:outline-none"
            title="Source Status"
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M3 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zm0 4a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z" clipRule="evenodd" />
            </svg>
          </button>
        </div>
      )}
      
      {/* Health Status Panel */}
      <AnimatePresence>
        {isHealthPanelOpen && (
          <motion.div
            initial={{ x: "-100%" }}
            animate={{ x: 0 }}
            exit={{ x: "-100%" }}
            transition={{ type: "spring", damping: 25, stiffness: 200 }}
            className="fixed left-0 top-0 h-full w-80 bg-white dark:bg-[#1c1b29] shadow-2xl z-50 border-r border-gray-200 dark:border-[#2e2b40]"
          >
            <div className="p-4">
              <div className="flex justify-between items-center mb-4">
                <h3 className="font-bold text-lg text-gray-900 dark:text-[#f4f4ff]">
                  Source Status
                </h3>
                <button
                  onClick={() => setIsHealthPanelOpen(false)}
                  className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 transition-colors"
                >
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <HealthStatus />
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export default App;
