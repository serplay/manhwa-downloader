import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faMoon, faSun } from "@fortawesome/free-solid-svg-icons";

export default function ThemeToggle({ darkMode, setDarkMode }) {
  return (
    <div className="absolute top-4 right-4">
      <button
        onClick={() => setDarkMode(!darkMode)}
        className="w-14 h-10 rounded-full bg-pink-500 dark:bg-violet-500 text-white font-semibold cursor-pointer transition-colors duration-300 flex items-center justify-center focus:outline-none"
      >
        <AnimatePresence mode="wait">
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
  );
}
