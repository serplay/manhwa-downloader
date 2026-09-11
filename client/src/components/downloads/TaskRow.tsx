import { useEffect, useRef, useState } from "react";
import { ArrowsClockwise, DownloadSimple, X } from "@phosphor-icons/react";
import { toast } from "sonner";
import { fileUrl } from "@/api/client";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { useTaskStream } from "@/hooks/useTaskStream";
import { formatBytes, plural } from "@/lib/format";
import { isTerminal, useDownloads, type TrackedTask } from "@/store/downloads";
import { cn } from "@/lib/cn";

/**
 * Fetch the finished archive and hand it to the browser. Fetching (rather than
 * an anchor click) tells us whether the save happened, which matters because
 * the server deletes the file after the first successful transfer.
 */
async function saveFile(id: string, fileName: string): Promise<"saved" | "gone" | "error"> {
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

export function TaskRow({ task }: { task: TrackedTask }) {
  const { cancel, remove, retry, markSaved, markGone } = useDownloads();
  const [saving, setSaving] = useState(false);
  const terminal = isTerminal(task);
  useTaskStream(task.id, !terminal);

  const state = task.gone ? "GONE" : (task.status?.state ?? "PENDING");
  const progress = task.status?.progress ?? 0;

  const fileName = task.status?.file_name ?? `${task.comicTitle}.${task.format === "pdf" || task.format === "cbz" || task.format === "cbr" ? "zip" : task.format}`;
  const save = async () => {
    setSaving(true);
    const result = await saveFile(task.id, fileName);
    setSaving(false);
    if (result === "saved") markSaved(task.id);
    else if (result === "gone") {
      markGone(task.id);
      toast.error("That file has expired on the server. Retry to build it again.");
    } else toast.error("Could not save the file. Try again.");
  };

  // Save exactly once per task when it succeeds, including across a reload.
  const attempted = useRef(task.saved);
  useEffect(() => {
    if (state === "SUCCESS" && !attempted.current) {
      attempted.current = true;
      void save();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state, task.id]);

  const line =
    state === "GONE"
      ? "No longer available on the server"
      : state === "SUCCESS"
        ? `${task.saved ? "Saved" : saving ? "Saving" : "Ready"}${task.status?.file_size ? `, ${formatBytes(task.status.file_size)}` : ""}`
        : state === "FAILURE"
          ? (task.status?.error ?? task.status?.status ?? "Failed")
          : state === "CANCELLED"
            ? "Cancelled"
            : (task.status?.status ?? "Queued");

  const onCancel = async () => {
    try {
      await cancel(task.id);
      toast("Cancelled");
    } catch {
      toast.error("Could not cancel");
    }
  };

  return (
    <li className="flex flex-col gap-2 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{task.comicTitle}</p>
          <p className="tabular text-xs text-fg-muted">
            {plural(task.chapterCount, "chapter")}, {task.format.toUpperCase()}
          </p>
        </div>
        <IconButton label="Dismiss" size="sm" onClick={() => remove(task.id)} disabled={!terminal && state !== "PENDING"}>
          <X size={14} />
        </IconButton>
      </div>
      {!terminal && (
        <div className="h-1 w-full overflow-hidden rounded-full bg-line-strong" role="progressbar" aria-valuenow={progress} aria-valuemin={0} aria-valuemax={100}>
          <div className="h-full rounded-full bg-accent transition-[width] duration-300 ease-spring" style={{ width: `${Math.max(progress, 2)}%` }} />
        </div>
      )}
      <div className="flex items-center justify-between gap-3">
        <p
          className={cn(
            "tabular min-w-0 truncate text-xs",
            state === "FAILURE" ? "text-danger" : state === "SUCCESS" ? "text-ok" : "text-fg-muted",
          )}
          aria-live="polite"
        >
          {line}
        </p>
        <div className="flex shrink-0 gap-1">
          {!terminal && (
            <Button size="sm" variant="ghost" onClick={() => void onCancel()}>
              Cancel
            </Button>
          )}
          {state === "SUCCESS" && !task.saved && (
            <Button size="sm" onClick={() => void save()} disabled={saving}>
              <DownloadSimple size={14} /> {saving ? "Saving" : "Save file"}
            </Button>
          )}
          {(state === "FAILURE" || state === "CANCELLED" || state === "GONE") && (
            <Button size="sm" onClick={() => void retry(task.id).catch(() => toast.error("Could not restart"))}>
              <ArrowsClockwise size={14} /> Retry
            </Button>
          )}
        </div>
      </div>
      {task.status?.warnings && task.status.warnings.length > 0 && (
        <details className="text-xs text-fg-muted">
          <summary className="cursor-pointer">{plural(task.status.warnings.length, "chapter")} skipped</summary>
          <ul className="mt-1 list-disc pl-4">
            {task.status.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </details>
      )}
    </li>
  );
}
