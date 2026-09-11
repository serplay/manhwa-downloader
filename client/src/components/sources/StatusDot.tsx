import type { SourceStatus } from "@/api/types";
import { statusLabel } from "@/hooks/useSources";
import { cn } from "@/lib/cn";

const styles: Record<SourceStatus, string> = {
  ok: "bg-ok",
  down: "bg-danger",
  blocked: "bg-warn",
  unknown: "bg-fg-faint",
};

/** Semantic status indicator; the only place a colored dot is allowed. */
export function StatusDot({ status, className }: { status: SourceStatus; className?: string }) {
  return (
    <span
      role="img"
      aria-label={statusLabel[status]}
      className={cn("inline-block size-2 shrink-0 rounded-full", styles[status], className)}
    />
  );
}
