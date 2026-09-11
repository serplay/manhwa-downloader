import { fileUrl } from "@/api/client";
import type { TrackedTask } from "@/store/downloads";

/**
 * Fetch a finished archive and hand it to the browser. Fetching (rather than
 * an anchor click) tells us whether the save happened, which matters because
 * the server deletes the file after the first successful transfer.
 */
export async function saveFile(id: string, fileName: string): Promise<"saved" | "gone" | "error"> {
  try {
    const res = await fetch(fileUrl(id));
    if (res.status === 404 || res.status === 410) return "gone";
    if (!res.ok) return "error";
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = fileName;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
    return "saved";
  } catch {
    return "error";
  }
}

export function fileNameFor(task: TrackedTask): string {
  if (task.status?.file_name) return task.status.file_name;
  const ext = task.format === "epub" ? "epub" : "zip";
  return `${task.comicTitle}.${ext}`;
}
