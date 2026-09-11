import { useEffect } from "react";
import { api, ApiError, eventsUrl } from "@/api/client";
import type { TaskStatus } from "@/api/types";
import { TERMINAL, useDownloads } from "@/store/downloads";

const POLL_MS = 2000;

/**
 * Follow one task: Server-Sent Events first, polling if the stream fails.
 * Stops on a terminal state. Marks the task gone on 404.
 */
export function useTaskStream(id: string, active: boolean) {
  const update = useDownloads((s) => s.update);
  const markGone = useDownloads((s) => s.markGone);

  useEffect(() => {
    if (!active) return;
    let stopped = false;
    let es: EventSource | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;

    const apply = (status: TaskStatus) => {
      update(id, status);
      if (TERMINAL.has(status.state)) stop();
    };
    const stop = () => {
      stopped = true;
      es?.close();
      if (timer) clearTimeout(timer);
    };
    const poll = async () => {
      if (stopped) return;
      try {
        const { data } = await api.GET("/download/status/{task_id}", { params: { path: { task_id: id } } });
        if (data) apply(data);
      } catch (e) {
        if (e instanceof ApiError && e.status === 404) {
          markGone(id);
          stop();
          return;
        }
      }
      if (!stopped) timer = setTimeout(() => void poll(), POLL_MS);
    };

    if (typeof EventSource !== "undefined") {
      es = new EventSource(eventsUrl(id));
      es.onmessage = (ev) => {
        try {
          apply(JSON.parse(ev.data) as TaskStatus);
        } catch {
          // ignore malformed frames
        }
      };
      es.onerror = () => {
        // Closed by the server after a terminal state, or a real failure: either
        // way one status fetch settles it, and polling continues if needed.
        es?.close();
        es = null;
        if (!stopped) void poll();
      };
    } else {
      void poll();
    }
    return stop;
  }, [id, active, update, markGone]);
}
