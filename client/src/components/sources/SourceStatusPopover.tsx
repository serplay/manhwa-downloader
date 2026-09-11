import { useState } from "react";
import { Popover } from "radix-ui";
import { ArrowsClockwise, Broadcast } from "@phosphor-icons/react";
import { Button } from "@/components/ui/Button";
import { IconButton } from "@/components/ui/IconButton";
import { speedLabel, statusLabel, useVisibleSources } from "@/hooks/useSources";
import { StatusDot } from "./StatusDot";

export function SourceStatusPopover() {
  const sources = useVisibleSources();
  const [open, setOpen] = useState(false);
  const up = sources.data?.filter((s) => s.status === "ok").length ?? 0;
  const total = sources.data?.length ?? 0;

  return (
    <Popover.Root
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (o) void sources.refetch();
      }}
    >
      <Popover.Trigger asChild>
        <Button variant="ghost" size="sm" className="gap-2">
          <Broadcast size={16} />
          <span className="hidden sm:inline">Sources</span>
          {total > 0 && (
            <span className="tabular text-xs text-fg-faint">
              {up}/{total}
            </span>
          )}
        </Button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content align="end" sideOffset={8} className="z-30 w-[min(22rem,calc(100vw-2rem))] rounded-panel bg-raised p-2 shadow-float hairline">
          <div className="flex items-center justify-between px-2 pt-1 pb-2">
            <p className="text-xs font-medium text-fg-muted">Source status</p>
            <IconButton label="Refresh" size="sm" onClick={() => void sources.refetch()} disabled={sources.isFetching}>
              <ArrowsClockwise size={14} className={sources.isFetching ? "animate-spin" : undefined} />
            </IconButton>
          </div>
          <ul className="max-h-80 overflow-y-auto">
            {(sources.data ?? []).map((s) => (
              <li key={s.slug} className="flex items-center gap-3 rounded-control px-2 py-1.5 text-sm">
                <StatusDot status={s.status} />
                <span className="min-w-0 flex-1 truncate">
                  {s.name}
                  {s.adult && <span className="ml-1.5 rounded-chip bg-warn/15 px-1 text-[10px] font-semibold text-warn">18+</span>}
                </span>
                <span className="text-xs text-fg-faint">{speedLabel(s.speed)}</span>
                <span className="shrink-0 text-xs text-fg-muted">{statusLabel[s.status]}</span>
              </li>
            ))}
            {sources.isPending && <li className="px-2 py-3 text-xs text-fg-faint">Checking sources</li>}
            {sources.error && <li className="px-2 py-3 text-xs text-danger">Could not reach the server.</li>}
          </ul>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
