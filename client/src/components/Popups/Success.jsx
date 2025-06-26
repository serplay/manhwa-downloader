import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faCircleCheck } from "@fortawesome/free-solid-svg-icons";

export default function SuccessPopup({ downloadSuccess, setDownloadSuccess }) {
  return (
    <AnimatePresence>
      {downloadSuccess && (
        <motion.div
          initial={{ opacity: 0, y: -50 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -50 }}
          className="w-full pointer-events-auto"
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
  );
}
