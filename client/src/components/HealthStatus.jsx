import { useState, useEffect } from "react";

const sourceNames = {
  0: ["MangaDex", "fastest"],
  1: ["Manhuaus", "slow"],
  2: ["Yakshascans", "slow"],
  3: ["Asurascans", "slow"],
  4: ["Kunmanga", "fast"],
  5: ["Toonily", "slow"],
  6: ["Toongod", "slow"],
  7: ["Mangahere", "fast"],
  8: ["Mangapill", "fast"],
  9: ["Bato", "fastest"],
  10: ["Weebcentral", "slow"],
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
    <div className="bg-white/80 dark:bg-[#1c1b29]/80 backdrop-blur-sm p-4 rounded-xl shadow-lg border border-gray-200 dark:border-[#2e2b40]">
      <h3 className="font-bold text-lg mb-2 text-gray-900 dark:text-[#f4f4ff]">
        Source Status
      </h3>
      <div className="mb-3 text-xs text-gray-600 dark:text-gray-400">
        <div className="flex items-center gap-2 mb-1">
          <span className="w-2 h-2 rounded-full bg-green-500"></span>
          <span>Fast & Reliable</span>
        </div>
        <div className="flex items-center gap-2 mb-1">
          <span className="w-2 h-2 rounded-full bg-cyan-500"></span>
          <span>Fastest</span>
        </div>
        <div className="flex items-center gap-2 mb-1">
          <span className="w-2 h-2 rounded-full bg-amber-500"></span>
          <span>Slow but Steady</span>
        </div>
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-red-500"></span>
          <span>Down/Unavailable</span>
        </div>
      </div>
      {isLoading ? (
        <div className="flex items-center justify-center">
          <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-pink-500 dark:border-violet-500"></div>
          <span className="ml-2">Loading...</span>
        </div>
      ) : (
        <ul className="space-y-2">
          {Object.entries(sourceNames).map(([id, [name, speed]]) => (
            <li key={id} className="flex items-center justify-between">
              <span
                className="text-sm text-gray-800 dark:text-gray-300"
                dangerouslySetInnerHTML={{ __html: name }}
              ></span>
              <span
                className={`w-3 h-3 rounded-full ${
                  status[id] !== "ok"
                    ? "bg-red-500"
                    : speed === "slow"
                    ? "bg-amber-500"
                    : speed === "fastest"
                    ? "bg-cyan-500"
                    : "bg-green-500"
                }`}
                title={`Speed: ${speed}`}
              ></span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default HealthStatus;
