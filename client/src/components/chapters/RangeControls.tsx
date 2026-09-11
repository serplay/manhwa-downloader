import type { Chapter } from "@/api/types";
import { Button } from "@/components/ui/Button";
import { RangeSlider } from "@/components/ui/Slider";

/** Slider plus numeric inputs over chapter indices; labels show chapter numbers. */
export function RangeControls({
  chapters,
  range,
  onRange,
  onSelectRange,
  onSelectAll,
  onClear,
}: {
  chapters: Chapter[];
  range: [number, number];
  onRange: (r: [number, number]) => void;
  onSelectRange: () => void;
  onSelectAll: () => void;
  onClear: () => void;
}) {
  const max = Math.max(chapters.length - 1, 0);
  const label = (i: number) => chapters[i]?.number ?? "";
  const clamp = (n: number) => Math.min(Math.max(n, 0), max);
  const setFrom = (n: number) => onRange([Math.min(clamp(n), range[1]), range[1]]);
  const setTo = (n: number) => onRange([range[0], Math.max(clamp(n), range[0])]);

  return (
    <div className="flex flex-col gap-3">
      <div className="grid grid-cols-[auto_1fr_auto] items-center gap-3">
        <BoundInput label="From" index={range[0]} max={max} display={label(range[0])} onCommit={setFrom} />
        <RangeSlider min={0} max={max} value={range} onChange={onRange} label="Chapter range" thumbLabel={(i) => `Chapter ${label(i)}`} />
        <BoundInput label="To" index={range[1]} max={max} display={label(range[1])} onCommit={setTo} />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button size="sm" variant="primary" onClick={onSelectRange} disabled={chapters.length === 0}>
          Select range
        </Button>
        <Button size="sm" onClick={onSelectAll} disabled={chapters.length === 0}>
          Select all
        </Button>
        <Button size="sm" variant="ghost" onClick={onClear}>
          Clear
        </Button>
      </div>
    </div>
  );
}

/** Position input (1-based) with the chapter number shown beside it. */
function BoundInput({
  label,
  index,
  max,
  display,
  onCommit,
}: {
  label: string;
  index: number;
  max: number;
  display: string;
  onCommit: (index: number) => void;
}) {
  return (
    <label className="flex flex-col gap-1 text-xs font-medium text-fg-muted">
      {label}
      <span className="flex h-9 items-center gap-1 rounded-control bg-raised px-2 hairline focus-within:shadow-[inset_0_0_0_1.5px_var(--color-accent)]">
        <input
          type="number"
          inputMode="numeric"
          min={1}
          max={max + 1}
          value={index + 1}
          onChange={(e) => {
            const n = Number(e.target.value);
            if (Number.isFinite(n)) onCommit(n - 1);
          }}
          className="tabular w-12 bg-transparent text-center font-mono text-sm text-fg outline-none"
          aria-label={`${label} position`}
        />
        <span className="tabular w-10 truncate font-mono text-[11px] text-fg-faint" title={`Chapter ${display}`}>
          ch {display}
        </span>
      </span>
    </label>
  );
}
