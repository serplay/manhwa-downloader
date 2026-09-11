import { ArrowRight } from "@phosphor-icons/react";
import type { Comic } from "@/api/types";
import { Chip } from "@/components/ui/Chip";
import { displayTitle } from "@/lib/format";
import { CoverImage } from "./CoverImage";

export function ComicCard({ comic, onOpen, priority }: { comic: Comic; onOpen: (comic: Comic) => void; priority?: boolean }) {
  const title = displayTitle(comic.title);
  const langs = comic.languages.filter((l) => l !== "en" || comic.languages.length > 1);
  return (
    <button
      type="button"
      onClick={() => onOpen(comic)}
      className="group flex flex-col gap-2 rounded-panel p-1.5 text-left transition-[transform,background-color] duration-200 ease-spring hover:-translate-y-0.5 hover:bg-surface focus-visible:bg-surface"
    >
      <CoverImage cover={comic.cover} alt="" priority={priority} />
      <div className="flex min-w-0 flex-col gap-1 px-1 pb-1">
        <span className="line-clamp-2 text-sm leading-snug font-medium">{title}</span>
        <span className="flex items-center justify-between gap-2">
          <span className="flex flex-wrap gap-1">
            {langs.slice(0, 3).map((l) => (
              <Chip key={l} className="h-5 px-1.5 text-[11px] uppercase">
                {l}
              </Chip>
            ))}
          </span>
          <span className="flex items-center gap-1 text-xs text-fg-faint opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100">
            Open <ArrowRight size={12} />
          </span>
        </span>
      </div>
    </button>
  );
}

export function ComicCardSkeleton() {
  return (
    <div className="flex flex-col gap-2 p-1.5">
      <div className="shimmer aspect-[2/3] w-full rounded-control" />
      <div className="shimmer h-4 w-4/5 rounded-chip" />
      <div className="shimmer h-4 w-2/5 rounded-chip" />
    </div>
  );
}
