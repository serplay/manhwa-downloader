import { create } from "zustand";
import { persist } from "zustand/middleware";
import { api, ApiError } from "@/api/client";
import type { DownloadAccepted, Format, TaskState, TaskStatus } from "@/api/types";
import type { DownloadIntent } from "@/components/chapters/ChapterPicker";

export interface TrackedTask {
  id: string;
  comicTitle: string;
  source: string;
  format: Format;
  chapterCount: number;
  createdAt: number;
  /** Kept so a failed task can be retried with one click. */
  intent: DownloadIntent;
  /** Live snapshot; not persisted, refetched after reload. */
  status?: TaskStatus;
  /** The browser save was triggered (or the file was collected). */
  saved: boolean;
  /** The server no longer knows this task (restart or retention expired). */
  gone: boolean;
}

interface DownloadsState {
  tasks: Record<string, TrackedTask>;
  order: string[];
  start: (intent: DownloadIntent) => Promise<DownloadAccepted>;
  update: (id: string, status: TaskStatus) => void;
  markSaved: (id: string) => void;
  markGone: (id: string) => void;
  cancel: (id: string) => Promise<void>;
  remove: (id: string) => void;
  retry: (id: string) => Promise<void>;
  clearFinished: () => void;
}

export const TERMINAL: ReadonlySet<TaskState> = new Set(["SUCCESS", "FAILURE", "CANCELLED"]);

export function isTerminal(t: TrackedTask): boolean {
  return t.gone || (t.status ? TERMINAL.has(t.status.state) : false);
}

export const useDownloads = create<DownloadsState>()(
  persist(
    (set, get) => ({
      tasks: {},
      order: [],

      async start(intent) {
        const { data } = await api.POST("/download", {
          body: {
            source: intent.source,
            comic_title: intent.comicTitle,
            chapters: intent.chapters,
            format: intent.format,
            lang: intent.lang ?? null,
          },
        });
        const accepted = data!;
        const task: TrackedTask = {
          id: accepted.task_id,
          comicTitle: intent.comicTitle,
          source: intent.source,
          format: intent.format,
          chapterCount: intent.chapters.length,
          createdAt: Date.now(),
          intent,
          saved: false,
          gone: false,
        };
        set((s) => ({ tasks: { ...s.tasks, [task.id]: task }, order: [task.id, ...s.order] }));
        return accepted;
      },

      update(id, status) {
        set((s) => {
          const t = s.tasks[id];
          if (!t) return s;
          return { tasks: { ...s.tasks, [id]: { ...t, status } } };
        });
      },

      markSaved(id) {
        set((s) => (s.tasks[id] ? { tasks: { ...s.tasks, [id]: { ...s.tasks[id]!, saved: true } } } : s));
      },

      markGone(id) {
        set((s) => (s.tasks[id] ? { tasks: { ...s.tasks, [id]: { ...s.tasks[id]!, gone: true } } } : s));
      },

      async cancel(id) {
        try {
          await api.POST("/download/cancel/{task_id}", { params: { path: { task_id: id } } });
        } catch (e) {
          // Already finished or unknown: nothing left to cancel.
          if (!(e instanceof ApiError && (e.status === 404 || e.status === 409))) throw e;
        }
      },

      remove(id) {
        set((s) => {
          const tasks = { ...s.tasks };
          delete tasks[id];
          return { tasks, order: s.order.filter((x) => x !== id) };
        });
      },

      async retry(id) {
        const t = get().tasks[id];
        if (!t) return;
        get().remove(id);
        await get().start(t.intent);
      },

      clearFinished() {
        set((s) => {
          const keep = s.order.filter((id) => !isTerminal(s.tasks[id]!));
          const tasks: Record<string, TrackedTask> = {};
          for (const id of keep) tasks[id] = s.tasks[id]!;
          return { tasks, order: keep };
        });
      },
    }),
    {
      name: "downloads",
      version: 1,
      // Live status is refetched on load; the rest survives a reload.
      partialize: (s) => ({
        order: s.order,
        tasks: Object.fromEntries(
          Object.entries(s.tasks).map(([id, t]) => [id, { ...t, status: undefined }]),
        ),
      }),
    },
  ),
);
