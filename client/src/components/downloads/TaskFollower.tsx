import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { useTaskStream } from "@/hooks/useTaskStream";
import { fileNameFor, saveFile } from "@/lib/save-file";
import { isTerminal, useDownloads, type TrackedTask } from "@/store/downloads";

/**
 * Renders nothing. Follows a task's progress and hands the archive to the
 * browser once on success, whether or not the tray panel is open. The archive
 * stays on the server for its retention window, so a failed attempt can be
 * repeated from the tray without rebuilding it.
 */
export function TaskFollower({ task }: { task: TrackedTask }) {
  const { markSaved, markGone } = useDownloads();
  useTaskStream(task.id, !isTerminal(task));

  const state = task.status?.state;
  const attempted = useRef(task.saved);
  useEffect(() => {
    if (state !== "SUCCESS" || attempted.current) return;
    attempted.current = true;
    void saveFile(task.id, fileNameFor(task)).then((result) => {
      if (result === "started") markSaved(task.id);
      else if (result === "gone") markGone(task.id);
      else
        toast.error(
          `Could not start the download for ${task.comicTitle}. Use Save file in the downloads tray.`,
        );
    });
  }, [state, task, markSaved, markGone]);

  return null;
}
