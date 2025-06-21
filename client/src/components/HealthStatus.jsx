import { useState, useEffect } from "react";

const sourceNames = {
  0: "MangaDex",
  1: "Manhuaus",
  2: "Yakshascans",
  3: "Asurascan",
  4: "Kunmanga",
  5: "Toonily",
  6: "Toongod",
  7: "Mangahere",
  8: "Mangapill",
  9: "Bato (Fastest)",
  10: "Weebcentral",
};

function HealthStatus() {
  const [status, setStatus] = useState({});
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const res = await fetch("/api/status");
        const data = await res.json();
        setStatus(data.status);
      } catch (error) {
        console.error("Failed to fetch server status:", error);
      } finally {
        setIsLoading(false);
      }
    };

    fetchStatus();
    // Refresh status every 5 minutes
    const interval = setInterval(fetchStatus, 5 * 60 * 1000); 
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="fixed bottom-4 left-4 bg-white/80 dark:bg-[#1c1b29]/80 backdrop-blur-sm p-4 rounded-xl shadow-lg border border-gray-200 dark:border-[#2e2b40] z-50">
      <h3 className="font-bold text-lg mb-2 text-gray-900 dark:text-[#f4f4ff]">Source Status</h3>
      {isLoading ? (
        <div className="flex items-center justify-center">
          <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-pink-500 dark:border-violet-500"></div>
          <span className="ml-2">Loading...</span>
        </div>
      ) : (
        <ul className="space-y-2">
          {Object.entries(sourceNames).map(([id, name]) => (
            <li key={id} className="flex items-center justify-between">
              <span className="text-sm text-gray-800 dark:text-gray-300">{name}</span>
              <span className={`w-3 h-3 rounded-full ${status[id] === "ok" ? "bg-green-500" : "bg-red-500"}`}></span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default HealthStatus; 