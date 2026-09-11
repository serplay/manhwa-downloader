import { ClockCounterClockwise, X } from "@phosphor-icons/react";
import { useRecentSearches } from "@/hooks/useRecentSearches";

export function RecentSearches({ onPick }: { onPick: (q: string) => void }) {
  const { items, remove, clear } = useRecentSearches();
  if (items.length === 0) return null;
  return (
    <div className="flex flex-wrap items-center gap-2" aria-label="Recent searches">
      <span className="flex items-center gap-1 text-xs text-fg-faint">
        <ClockCounterClockwise size={14} /> Recent
      </span>
      {items.map((q) => (
        <span key={q} className="inline-flex h-7 items-center overflow-hidden rounded-chip bg-surface text-xs hairline">
          <button
            type="button"
            onClick={() => onPick(q)}
            className="max-w-48 truncate px-2.5 text-fg-muted hover:bg-raised hover:text-fg"
          >
            {q}
          </button>
          <button
            type="button"
            aria-label={`Remove ${q} from recent searches`}
            onClick={() => remove(q)}
            className="flex h-full items-center px-1.5 text-fg-faint hover:bg-raised hover:text-fg"
          >
            <X size={11} />
          </button>
        </span>
      ))}
      <button type="button" onClick={clear} className="text-xs text-fg-faint hover:text-fg">
        Clear
      </button>
    </div>
  );
}
