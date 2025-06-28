import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faCircleXmark } from "@fortawesome/free-solid-svg-icons";

export default function ErrorPopup({ downloadError, setDownloadError }) {
  return (
    <AnimatePresence>
      {downloadError && (
        <motion.div
          initial={{ opacity: 0, y: -50 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -50 }}
          className="w-full pointer-events-auto"
        >
          <div className="bg-red-500/75 text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-2">
            <FontAwesomeIcon icon={faCircleXmark} className="text-lg" />
            <div className="flex flex-col">
              <span>{downloadError.split("\n")[0]}</span>
              {downloadError.split("\n").length > 1 && (
                <span className="text-sm opacity-90">
                  {downloadError.split("\n").slice(1).join("\n")}
                </span>
              )}
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
  );
}
