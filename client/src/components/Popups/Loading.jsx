import { motion, AnimatePresence } from "framer-motion";

export default function LoadingPopup({ isFetchingChapters, isSearching }) {
  return (
    <AnimatePresence>
      {(isFetchingChapters || isSearching) && (
        <motion.div
          initial={{ opacity: 0, y: -50 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -50 }}
          className="w-full pointer-events-auto"
        >
          <div className="bg-blue-500 text-white px-6 py-3 rounded-lg shadow-lg flex items-center gap-2">
            <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
            <span>
              {isFetchingChapters
                ? "Fetching chapters..."
                : "Searching titles..."}
            </span>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
