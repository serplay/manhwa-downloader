import { fileUrl } from "@/api/client";
import type { TrackedTask } from "@/store/downloads";

export type SaveResult = "started" | "gone" | "error";

/**
 * Hand a finished archive to the browser.
 *
 * A same-origin link with `download` makes the browser stream the file straight
 * to disk. Reading it with `fetch` first would buffer the whole archive in
 * memory and lose everything if the transfer breaks on the way, which is what
 * happens on multi-chapter downloads behind a proxy hop.
 *
 * The HEAD request is there so an archive whose retention window has closed
 * reports a real message instead of saving a file full of error JSON.
 */
export async function saveFile(id: string, fileName: string): Promise<SaveResult> {
  try {
    const probe = await fetch(fileUrl(id), { method: "HEAD" });
    if (probe.status === 404 || probe.status === 410) return "gone";
    if (!probe.ok) return "error";
  } catch {
    return "error";
  }
  const a = document.createElement("a");
  a.href = fileUrl(id);
  a.download = fileName;
  a.rel = "noopener";
  document.body.appendChild(a);
  a.click();
  a.remove();
  return "started";
}

export function fileNameFor(task: TrackedTask): string {
  if (task.status?.file_name) return task.status.file_name;
  const ext = task.format === "epub" ? "epub" : "zip";
  return `${task.comicTitle}.${ext}`;
}
