import { useState, useEffect } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faCircleCheck,
  faCircleXmark,
  faSun,
  faMoon,
  faDownload,
  faSpinner,
  faChevronUp,
  faChevronDown,
} from "@fortawesome/free-solid-svg-icons";
import logo from "./assets/logo.png";
import HealthStatus from "./components/HealthStatus";
import ChapterRangeSlider from "./components/ChapterRangeSlider";

function App() {
  const API_url = "/api";
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
  const [downloadError, setDownloadError] = useState("");
  const [isFetchingChapters, setIsFetchingChapters] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [loadedImages, setLoadedImages] = useState(new Set());

  // Range selection states
  const [chapterRanges, setChapterRanges] = useState({});

  // Chapter dropdown states
  const [expandedChapterSections, setExpandedChapterSections] = useState(new Set());

  // Background task states
  const [activeTasks, setActiveTasks] = useState({});
  const [taskStatuses, setTaskStatuses] = useState({});
  const [downloadingFiles, setDownloadingFiles] = useState(new Set());
  const [downloadSuccess, setDownloadSuccess] = useState("");

  // Format selection states
  const [selectedFormat, setSelectedFormat] = useState({});

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
          setTaskStatuses(prev => ({ ...prev, [taskId]: status }));

          // If task completed successfully, automatically download the file
          if (status.state === "SUCCESS") {
            // Check if the file is not already downloading
            if (!downloadingFiles.has(taskId)) {
              console.log(`DEBUG: Task ${taskId} completed successfully, downloading file...`);
              
              // Mark file as downloading
              setDownloadingFiles(prev => new Set(prev).add(taskId));
              
              try {
                // Get comic title from activeTasks
                const taskInfo = activeTasks[taskId];
                const comicTitle = taskInfo?.comicTitle || "Chapters";
                
                await downloadCompletedFile(taskId, comicTitle);
                console.log(`DEBUG: File for task ${taskId} downloaded successfully`);
              } catch (error) {
                console.error(`DEBUG: Error during automatic file download for task ${taskId}:`, error);
                setDownloadError(`Automatic file download failed: ${error.message}`);
              } finally {
                // Remove from downloading files list
                setDownloadingFiles(prev => {
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
            setActiveTasks(prev => {
              const newTasks = { ...prev };
              delete newTasks[taskId];
              return newTasks;
            });
          } else if (status.state === "FAILURE") {
            // Delay removal for failed tasks to make the error visible
            setTimeout(() => {
              setActiveTasks(prev => {
                const newTasks = { ...prev };
                delete newTasks[taskId];
                return newTasks;
              });
            }, 1000); // 1-second delay
          }
        } else {
          // If response not OK, set task status as FAILURE and remove it after a delay
          console.error(`Status check for ${taskId} failed with status: ${response.status}`);
          setTaskStatuses(prev => ({
            ...prev,
            [taskId]: { state: "FAILURE" }
          }));
          setTimeout(() => {
            setActiveTasks(prev => {
              const newTasks = { ...prev };
              delete newTasks[taskId];
              return newTasks;
            });
          }, 3000);
        }
      } catch (error) {
        console.error(`Error checking status of task ${taskId}:`, error);
        // Mark task as failed and remove it after a delay
        setTaskStatuses(prev => ({
          ...prev,
          [taskId]: { state: "FAILURE" }
        }));
        setTimeout(() => {
          setActiveTasks(prev => {
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

  // Search for comics based on title and source
  const handleSearch = async () => {
    setDownloadError("");
    setIsSearching(true);
    try {
      const res = await fetch(
        `${API_url}/search/?title=${encodeURIComponent(
          title
        )}&source=${source}`
      );
      if (!res.ok) {
        const errorData = await res.json();
        const errorMessage = errorData.error || "Failed to fetch chapters";
        throw new Error(errorMessage);
      }
      
      const data = await res.json();

      if (data.message) {
        setResults({});
        setError(data.message);
      } else if (data.error) {
        setResults({});
        setError(data.error);
      } else {
        setResults(data);
        setError("");
      }
      setAnimationKey((prev) => prev + 1);
    } catch (err) {
      setError("There was an error fetching the data");
      setResults({});
    } finally {
      setIsSearching(false);
    }
  };

  // Fetch chapters for a specific comic
  const fetchChapters = async (comicId) => {
    setDownloadError("");
    setIsFetchingChapters(true);
    try {
      const res = await fetch(
        `${API_url}/chapters/?id=${comicId}&source=${source}`
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
            ...value
          }))
          .sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));
        processedData[displayName] = chaptersArray;
      }

      setChaptersByComicId((prev) => ({ ...prev, [comicId]: processedData }));
    } catch (err) {
      console.error("Error fetching chapters:", err);
      setDownloadError("Failed to fetch chapters.\n" + err.message);
      // Remove the comic from expanded state if there's an error
      setExpandedComics((prev) => {
        const next = new Set(prev);
        next.delete(comicId);
        return next;
      });
    } finally {
      setIsFetchingChapters(false);
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

  // Handle chapter download - now starts background task
  const handleDownload = async (comicId, comicTitle, format = "pdf") => {
    const chapterIds = selectedChapters[comicId] || [];
    if (chapterIds.length === 0) return;
  
    setIsDownloading(true);
    setDownloadError(""); // Clear any previous errors
    try {
      // Format chapter IDs as array parameters with chapter numbers
      const params = new URLSearchParams();
      Object.entries(chaptersByComicId[comicId]).forEach(([volumeName, chapters]) => {
        Object.entries(chapters).forEach(([chNumber, chData]) => {
          if (chapterIds.includes(chData.id)) {
            params.append("ids[]", `${chData.id}_${chData.chapter}`);
          }
        });
      });
      params.append("source", source);
      params.append("comic_title", comicTitle);
      params.append("format", format);
  
      const response = await fetch(`${API_url}/download?${params.toString()}`, {
        method: 'POST'
      });
      
      if (!response.ok) {
        const errorData = await response.json();
        const errorMessage = errorData.detail || "Failed to start download";
        throw new Error(errorMessage);
      }
  
      const data = await response.json();
      
      // Dodaj zadanie do aktywnych
      setActiveTasks(prev => ({
        ...prev,
        [data.task_id]: {
          comicId,
          comicTitle,
          chapterCount: chapterIds.length,
          startTime: new Date().toISOString()
        }
      }));

      // Wyczyść wybrane rozdziały
      setSelectedChapters(prev => {
        const newSelected = { ...prev };
        delete newSelected[comicId];
        return newSelected;
      });

    } catch (error) {
      console.error("Download error:", error);
      setDownloadError("Failed to start download. Please try again later.\n" + error.message);
    } finally {
      setIsDownloading(false);
    }
  };

  // Download completed file
  const downloadCompletedFile = async (taskId, comicTitle = "Chapters") => {
    try {
      console.log(`DEBUG: Attempting to download file for task ${taskId}, title: ${comicTitle}`);
  
      // Prepare filename from comic title
      const safeTitle = comicTitle.replace(/[^a-zA-Z0-9\s-]/g, '').trim().replace(/\s+/g, '_');
      const fileName = `${safeTitle}.zip`;
  
      // Use <a> element with download attribute
      const downloadUrl = `${API_url}/download/file/${taskId}`;
      console.log(`DEBUG: Creating download link: ${downloadUrl}`);
  
      const a = document.createElement("a");
      a.href = downloadUrl;
      a.download = fileName;
      a.style.display = 'none';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
  
      console.log(`DEBUG: File ${fileName} sent for download`);
  
      // Show success notification
      setDownloadSuccess(`File ${fileName} has been sent for download!`);
  
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
  const handleRangeChange = (comicId, { start, end }) => {
    setChapterRanges(prev => ({ ...prev, [comicId]: { start, end } }));
    
    // Don't automatically select chapters - let users choose manually
    // The range is just for visual reference and quick actions
  };

  // Select chapters within the current range
  const selectChaptersInRange = (comicId) => {
    const range = chapterRanges[comicId];
    if (!range) return;
    
    // Get all chapters for this comic
    const allChapters = [];
    Object.values(chaptersByComicId[comicId] || {}).forEach(volume => {
      allChapters.push(...volume);
    });
    
    // Sort chapters by chapter number
    allChapters.sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));
    
    // Select chapters within the range
    const selectedInRange = allChapters
      .slice(range.start, range.end + 1)
      .map(ch => ch.id);
    
    setSelectedChapters(prev => ({
      ...prev,
      [comicId]: selectedInRange
    }));
  };

  // Clear all selections for a comic
  const clearChapterSelections = (comicId) => {
    setSelectedChapters(prev => ({
      ...prev,
      [comicId]: []
    }));
  };

  // Get chapter range info for slider
  const getChapterRangeInfo = (comicId) => {
    const allChapters = [];
    Object.values(chaptersByComicId[comicId] || {}).forEach(volume => {
      allChapters.push(...volume);
    });
    
    if (allChapters.length === 0) return { min: 0, max: 0 };
    
    // Sort chapters by chapter number
    allChapters.sort((a, b) => parseFloat(a.chapter) - parseFloat(b.chapter));
    
    return {
      min: 0,
      max: allChapters.length - 1,
      chapters: allChapters
    };
  };

  return (
    // Main container with theme-aware background
    <div className="min-h-screen transition-colors duration-300 bg-gradient-to-br from-white via-pink-100 to-purple-100 dark:from-[#0d0c1b] dark:via-[#1a152b] dark:to-[#2d1b4d] text-gray-900 dark:text-[#f4f4ff] relative">
      {/* Error popup */}
      <AnimatePresence>
        {downloadError && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -50 }}
            className="fixed top-4 left-1/2 transform -translate-x-1/2 z-50"
          >
            <div className="bg-red-500/75 text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-2">
              <FontAwesomeIcon icon={faCircleXmark} className="text-lg" />
              <div className="flex flex-col">
                <span>Failed to download chapters. Please try again later.</span>
                <span className="text-sm opacity-90">{downloadError.split('\n')[1]}</span>
              </div>
              <button
                onClick={() => setDownloadError("")}
                className="ml-4 hover:opacity-80 cursor-pointer"
              >
                ×
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Success popup */}
      <AnimatePresence>
        {downloadSuccess && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -50 }}
            className="fixed top-4 left-1/2 transform -translate-x-1/2 z-50"
          >
            <div className="bg-green-500 text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-2">
              <FontAwesomeIcon icon={faCircleCheck} className="text-lg" />
              <span>{downloadSuccess}</span>
              <button
                onClick={() => setDownloadSuccess("")}
                className="ml-4 hover:opacity-80 cursor-pointer"
              >
                ×
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Loading popup */}
      <AnimatePresence>
        {(isFetchingChapters || isSearching) && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -50 }}
            className="fixed top-4 left-1/2 transform -translate-x-1/2 z-50"
          >
            <div className="bg-blue-500 text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-2">
              <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
              <span>{isFetchingChapters ? "Fetching chapters..." : "Searching titles..."}</span>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Active tasks popup */}
      <AnimatePresence>
        {Object.keys(activeTasks).length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: -50 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -50 }}
            className="fixed top-4 right-4 z-50 max-w-md"
          >
            <div className="bg-green-500 text-white px-4 py-3 rounded-lg shadow-lg">
              <h3 className="font-semibold mb-2">Active download tasks:</h3>
              {Object.entries(activeTasks).map(([taskId, taskInfo]) => {
                const status = taskStatuses[taskId];
                const isCompleted = status?.state === "SUCCESS";
                const isFailed = status?.state === "FAILURE";
                
                return (
                  <div key={taskId} className={`mb-2 p-2 rounded transition-colors ${isFailed ? 'bg-red-500/50' : 'bg-green-600/50'}`}>
                    <div className="flex justify-between items-center">
                      <div className="flex-1">
                        <p className="text-sm font-medium">{taskInfo.comicTitle}</p>
                        <p className="text-xs opacity-90">
                          {taskInfo.chapterCount} {taskInfo.chapterCount === 1 ? 'chapter' : 'chapters'} - {getTaskStatusDisplay(taskId)}
                        </p>
                      </div>
                      <div className="flex gap-2">
                        {status?.state === "PROGRESS" && (
                          <FontAwesomeIcon icon={faSpinner} className="animate-spin" />
                        )}
                        {isCompleted && (
                          <div className="flex items-center gap-1 text-yellow-200">
                            {downloadingFiles.has(taskId) ? (
                              <>
                                <FontAwesomeIcon icon={faSpinner} className="animate-spin" />
                                <span className="text-xs">Downloading...</span>
                              </>
                            ) : (
                              <>
                                <FontAwesomeIcon icon={faDownload} />
                                <span className="text-xs">Downloaded</span>
                              </>
                            )}
                          </div>
                        )}
                        {isFailed && (
                          <FontAwesomeIcon icon={faCircleXmark} className="text-red-200" />
                        )}
                      </div>
                    </div>
                    {status?.state === "PROGRESS" && status.progress !== undefined && (
                      <div className="w-full bg-green-700 rounded-full h-2 mt-1">
                        <div 
                          className="bg-yellow-300 h-2 rounded-full transition-all duration-300"
                          style={{ width: `${status.progress}%` }}
                        ></div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Theme toggle button */}
      <div className="absolute top-4 right-4">
        <button
          onClick={() => setDarkMode(!darkMode)}
          className="w-14 h-10 rounded-full bg-pink-500 dark:bg-violet-500 text-white font-semibold cursor-pointer transition-colors duration-300 flex items-center justify-center focus:outline-none"
        >
          <AnimatePresence mode="sync">
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
            <option value="9">Bato</option>
            <option value="8">Mangapill</option>
            <option value="7">Mangahere</option>
            <option value="10">Weebcentral</option>
            <option value="1">Manhuaus</option>
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
                <AnimatePresence mode="sync" key={animationKey}>
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
                            {!loadedImages.has(comic.cover_art) && (
                              <div className="absolute inset-0 flex items-center justify-center bg-gray-200 dark:bg-[#2e2b40] rounded-lg">
                                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-pink-500 dark:border-violet-500"></div>
                              </div>
                            )}
                            <img
                              src={comic.cover_art}
                              alt="cover"
                              className={`w-full h-full object-cover rounded-lg group-hover:opacity-50 transition-opacity ${
                                !loadedImages.has(comic.cover_art) ? 'opacity-0' : 'opacity-100'
                              }`}
                              onLoad={() => setLoadedImages(prev => new Set(prev).add(comic.cover_art))}
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
                              {/* Chapter Range Slider */}
                              <div className="mb-4">
                                <button
                                  onClick={() => {
                                    if (expandedChapterSections.has(comic.id)) {
                                      setExpandedChapterSections(prev => {
                                        const next = new Set(prev);
                                        next.delete(comic.id);
                                        return next;
                                      });
                                    } else {
                                      setExpandedChapterSections(prev => new Set(prev).add(comic.id));
                                    }
                                  }}
                                  className="flex items-center gap-2 text-pink-600 dark:text-violet-400 font-semibold hover:opacity-80 transition-opacity"
                                >
                                  <span>Quick Chapter Selection</span>
                                  <FontAwesomeIcon
                                    icon={expandedChapterSections.has(comic.id) ? faChevronUp : faChevronDown}
                                    className="text-sm transition-transform duration-200"
                                  />
                                </button>
                                
                                <AnimatePresence initial={false}>
                                  {expandedChapterSections.has(comic.id) && (
                                    <motion.div
                                      key="quick-chapter-dropdown"
                                      initial={{ height: 0, opacity: 0 }}
                                      animate={{ height: "auto", opacity: 1 }}
                                      exit={{ height: 0, opacity: 0 }}
                                      transition={{ duration: 0.3, ease: "easeOut" }}
                                      className="overflow-hidden mt-3"
                                    >
                                      {(() => {
                                        const rangeInfo = getChapterRangeInfo(comic.id);
                                        if (rangeInfo.max > 0) {
                                          const currentRange = chapterRanges[comic.id];
                                          const selectedCount = selectedChapters[comic.id]?.length || 0;
                                          
                                          return (
                                            <div>
                                              <ChapterRangeSlider
                                                min={rangeInfo.min}
                                                max={rangeInfo.max}
                                                onRangeChange={(range) => handleRangeChange(comic.id, range)}
                                                initialStart={currentRange?.start || rangeInfo.min}
                                                initialEnd={currentRange?.end || rangeInfo.max}
                                              />
                                              
                                              {/* Range Info and Action Buttons */}
                                              {currentRange && (
                                                <div className="mt-3 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                                                  <div className="flex justify-between items-center mb-2">
                                                    <span className="text-sm text-blue-700 dark:text-blue-300">
                                                      Range: Chapter {rangeInfo.chapters[currentRange.start]?.chapter} - Chapter {rangeInfo.chapters[currentRange.end]?.chapter}
                                                    </span>
                                                    <span className="text-sm text-gray-600 dark:text-gray-400">
                                                      {selectedCount} selected
                                                    </span>
                                                  </div>
                                                  <div className="flex gap-2">
                                                    <button
                                                      onClick={() => selectChaptersInRange(comic.id)}
                                                      className="px-3 py-1 text-xs bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
                                                    >
                                                      Select Range
                                                    </button>
                                                    <button
                                                      onClick={() => clearChapterSelections(comic.id)}
                                                      className="px-3 py-1 text-xs bg-gray-500 text-white rounded hover:bg-gray-600 transition-colors"
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
                              <div className="mb-4">
                                <h4 className="font-semibold mb-2 text-pink-600 dark:text-violet-400">
                                  Individual Chapters
                                </h4>
                              {Object.entries(chaptersByComicId[comic.id]).map(
                                ([volumeName, chapters]) => (
                                  <div key={volumeName} className="mb-4">
                                    <h4 className="font-semibold mb-2 text-pink-600 dark:text-violet-400">
                                      {volumeName}
                                    </h4>
                                    <div className="flex flex-wrap gap-2">
                                      {chapters.map(
                                        (chData) => (
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
                                            {`Ch ${chData.chapter}`}
                                          </button>
                                        )
                                      )}
                                    </div>
                                  </div>
                                )
                              )}
                              </div>

                              {/* Chapter action buttons */}
                              <div className="flex gap-4 mt-2">
                                <button
                                  onClick={() => selectAllChapters(comic.id)}
                                  className="px-3 py-1 rounded-md bg-pink-500 rounded-r-none text-white dark:bg-violet-500 hover:opacity-90 cursor-pointer"
                                >
                                  Select All
                                </button>
                                <div className="flex">
                                  <button
                                    onClick={() => handleDownload(comic.id, comic.title.en || Object.values(comic.title)[0], selectedFormat[comic.id] || "pdf")}
                                    disabled={
                                      isDownloading ||
                                      !selectedChapters[comic.id]?.length
                                    }
                                    className={`px-3 py-1 rounded-l-md border-r-0 cursor-pointer ${
                                      isDownloading ||
                                      !selectedChapters[comic.id]?.length
                                        ? "bg-gray-400 cursor-not-allowed"
                                        : "bg-green-500 hover:opacity-90"
                                    } text-white`}
                                  >
                                    {isDownloading ? "Starting..." : "Download"}
                                  </button>
                                  <select
                                    value={selectedFormat[comic.id] || "pdf"}
                                    onChange={e => setSelectedFormat(prev => ({ ...prev, [comic.id]: e.target.value }))}
                                    className="px-2 py-1 rounded-r-md rounded-l-none border border-l-0 border-gray-300 dark:border-gray-600 bg-white dark:bg-[#2e2b40] text-gray-800 dark:text-gray-200 focus:outline-none"
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
                    ))
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
      <HealthStatus />
    </div>
  );
}

export default App;
