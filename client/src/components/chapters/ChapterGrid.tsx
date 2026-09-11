import { useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { Volume } from "@/api/types";
import { ToggleChip } from "@/components/ui/Chip";
import { plural } from "@/lib/format";

type Row = { kind: "header"; name: string; count: number } | { kind: "chips"; start: number; ids: string[]; labels: string[] };

const CHIP_MIN_PX = 76;
const GAP_PX = 8;

/**
 * Chapters grouped by volume in a virtualized list of rows. Volume headers are
 * rows too, so the scroll stays one container. Shift-click selects a span.
 */
export function ChapterGrid({
  volumes,
  selected,
  onToggle,
  onSpan,
  scrollRef,
}: {
  volumes: Volume[];
  selected: Set<string>;
  onToggle: (id: string) => void;
  onSpan: (fromIndex: number, toIndex: number) => void;
  scrollRef: React.RefObject<HTMLDivElement | null>;
}) {
  const [columns, setColumns] = useState(6);
  const lastClicked = useRef<number | null>(null);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const width = entry?.contentRect.width ?? 480;
      setColumns(Math.max(3, Math.floor((width + GAP_PX) / (CHIP_MIN_PX + GAP_PX))));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, [scrollRef]);

  const rows = useMemo<Row[]>(() => {
    const out: Row[] = [];
    let index = 0;
    const many = volumes.length > 1;
    for (const v of volumes) {
      if (many) out.push({ kind: "header", name: v.name, count: v.chapters.length });
      for (let i = 0; i < v.chapters.length; i += columns) {
        const slice = v.chapters.slice(i, i + columns);
        out.push({ kind: "chips", start: index, ids: slice.map((c) => c.id), labels: slice.map((c) => c.number) });
        index += slice.length;
      }
    }
    return out;
  }, [volumes, columns]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (i) => (rows[i]?.kind === "header" ? 36 : 36),
    overscan: 8,
  });

  const click = (globalIndex: number, id: string) => (e: MouseEvent) => {
    if (e.shiftKey && lastClicked.current != null) {
      onSpan(Math.min(lastClicked.current, globalIndex), Math.max(lastClicked.current, globalIndex));
    } else {
      onToggle(id);
    }
    lastClicked.current = globalIndex;
  };

  return (
    <div style={{ height: virtualizer.getTotalSize() }} className="relative w-full">
      {virtualizer.getVirtualItems().map((item) => {
        const row = rows[item.index]!;
        return (
          <div
            key={item.key}
            data-index={item.index}
            ref={virtualizer.measureElement}
            className="absolute top-0 left-0 w-full"
            style={{ transform: `translateY(${item.start}px)` }}
          >
            {row.kind === "header" ? (
              <div className="flex items-baseline gap-2 pt-3 pb-2 text-xs font-medium text-fg-muted">
                {row.name}
                <span className="tabular text-fg-faint">{plural(row.count, "chapter")}</span>
              </div>
            ) : (
              <div
                className="grid pb-2"
                style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`, gap: GAP_PX }}
              >
                {row.ids.map((id, i) => (
                  <ToggleChip
                    key={id}
                    pressed={selected.has(id)}
                    onClick={click(row.start + i, id)}
                    title={`Chapter ${row.labels[i]}`}
                    className="tabular w-full justify-center px-1 font-mono"
                  >
                    {row.labels[i]}
                  </ToggleChip>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
