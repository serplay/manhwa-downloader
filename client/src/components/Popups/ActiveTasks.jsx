import { motion, AnimatePresence } from "framer-motion";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  faSpinner,
  faDownload,
  faCircleXmark,
} from "@fortawesome/free-solid-svg-icons";

export default function ActiveTasksPopup({
  activeTasks,
  taskStatuses,
  downloadingFiles,
  getTaskStatusDisplay,
  onCancelTask,
}) {
  return (
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
                <div
                  key={taskId}
                  className={`mb-2 p-2 rounded transition-colors ${
                    isFailed ? "bg-red-500/50" : "bg-green-600/50"
                  }`}
                >
                  <div className="flex justify-between items-center">
                    <div className="flex-1">
                      <p className="text-sm font-medium">
                        {taskInfo.comicTitle}
                      </p>
                      <p className="text-xs opacity-90">
                        {taskInfo.chapterCount}{" "}
                        {taskInfo.chapterCount === 1 ? "chapter" : "chapters"} -{" "}
                        {getTaskStatusDisplay(taskId)}
                      </p>
                    </div>
                    <div className="flex gap-2 items-center">
                      {status?.state === "PROGRESS" && (
                        <>
                          <FontAwesomeIcon
                            icon={faSpinner}
                            className="animate-spin"
                          />
                          <button
                            onClick={() => onCancelTask && onCancelTask(taskId)}
                            className="ml-2 p-1 rounded hover:bg-red-700 text-white cursor-pointer"
                            title="Cancel download"
                          >
                            <FontAwesomeIcon icon={faCircleXmark} />
                          </button>
                        </>
                      )}
                      {isCompleted && (
                        <div className="flex items-center gap-1 text-yellow-200">
                          {downloadingFiles.has(taskId) ? (
                            <>
                              <FontAwesomeIcon
                                icon={faSpinner}
                                className="animate-spin"
                              />
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
                        <FontAwesomeIcon
                          icon={faCircleXmark}
                          className="text-red-200"
                        />
                      )}
                    </div>
                  </div>
                  {status?.state === "PROGRESS" &&
                    status.progress !== undefined && (
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
  );
}
