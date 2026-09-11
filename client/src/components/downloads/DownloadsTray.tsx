import { useMemo, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { CaretDown, Tray } from "@phosphor-icons/react";
import { Button } from "@/components/ui/Button";
import { plural } from "@/lib/format";
import { isTerminal, useDownloads } from "@/store/downloads";
import { ProgressRing } from "./ProgressRing";
import { TaskRow } from "./TaskRow";

const spring = { type: "spring", stiffness: 260, damping: 26 } as const;

export function DownloadsTray() {
  const order = useDownloads((s) => s.order);
  const tasks = useDownloads((s) => s.tasks);
  const clearFinished = useDownloads((s) => s.clearFinished);
  const [open, setOpen] = useState(false);
  const reduced = useReducedMotion();

  const list = useMemo(() => order.map((id) => tasks[id]!).filter(Boolean), [order, tasks]);
  const active = list.filter((t) => !isTerminal(t));
  const aggregate = active.length
    ? active.reduce((n, t) => n + (t.status?.progress ?? 0), 0) / active.length
    : null;

  if (list.length === 0) return null;

  const label = active.length ? `${plural(active.length, "download")} running` : `${plural(list.length, "download")}`;

  return (
    <aside aria-label="Downloads" className="fixed right-4 bottom-4 z-30 flex flex-col items-end gap-2 sm:right-6 sm:bottom-6">
      <AnimatePresence>
        {open && (
          <motion.section
            key="panel"
            initial={reduced ? { opacity: 0 } : { opacity: 0, y: 12, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduced ? { opacity: 0 } : { opacity: 0, y: 12, scale: 0.98 }}
            transition={reduced ? { duration: 0.1 } : spring}
            className="flex w-[min(24rem,calc(100vw-2rem))] max-h-[60dvh] flex-col rounded-panel bg-raised shadow-float hairline"
          >
            <header className="flex items-center justify-between gap-2 px-4 pt-3 pb-1">
              <h2 className="text-sm font-semibold">Downloads</h2>
              {list.some(isTerminal) && (
                <Button size="sm" variant="ghost" onClick={clearFinished}>
                  Clear finished
                </Button>
              )}
            </header>
            <ul className="min-h-0 flex-1 divide-y divide-line overflow-y-auto px-4 pb-2">
              {list.map((t) => (
                <TaskRow key={t.id} task={t} />
              ))}
            </ul>
          </motion.section>
        )}
      </AnimatePresence>
      <motion.button
        type="button"
        layout={!reduced}
        transition={spring}
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        className="flex h-11 items-center gap-2 rounded-full bg-raised pr-4 pl-3 text-sm font-medium shadow-float hairline hover:bg-surface active:scale-[0.98]"
      >
        {active.length ? <ProgressRing value={aggregate != null && aggregate > 0 ? aggregate : null} /> : <Tray size={18} className="text-fg-muted" />}
        <span className="tabular">{label}</span>
        <CaretDown size={14} className={open ? "rotate-180 transition-transform" : "transition-transform"} />
      </motion.button>
    </aside>
  );
}
