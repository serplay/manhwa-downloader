import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowsClockwise, DownloadSimple } from "@phosphor-icons/react";
import { ApiError } from "@/api/client";
import type { Chapter, ChapterRef, Comic, Format } from "@/api/types";
import { Button } from "@/components/ui/Button";
import { Select } from "@/components/ui/Select";
import { Sheet } from "@/components/ui/Sheet";
import { Skeleton } from "@/components/ui/Skeleton";
import { Tooltip } from "@/components/ui/Tooltip";
import { CoverImage } from "@/components/results/CoverImage";
import { useChapters } from "@/hooks/useChapters";
import { displayTitle, plural } from "@/lib/format";
import { ChapterGrid } from "./ChapterGrid";
import { useFormats } from "@/hooks/useFormats";
import { FormatPicker } from "./FormatPicker";
import { RangeControls } from "./RangeControls";

export interface DownloadIntent {
  source: string;
  comicTitle: string;
  chapters: ChapterRef[];
  format: Format;
  lang?: string;
}

const FORMAT_KEY = "format";

export function ChapterPicker({
  comic,
  source,
  sourceName,
  open,
  onOpenChange,
  onDownload,
}: {
  comic: Comic | null;
  source: string;
  sourceName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDownload: (intent: DownloadIntent) => void;
}) {
  const languages = comic?.languages ?? [];
  const multiLang = languages.length > 1;
  const [lang, setLang] = useState<string>("en");
  const chapters = useChapters(source, comic?.id ?? "", multiLang ? lang : undefined);
  const formats = useFormats();
  const [format, setFormat] = useState<Format>(() => {
    try {
      return (localStorage.getItem(FORMAT_KEY) as Format | null) ?? "pdf";
    } catch {
      return "pdf";
    }
  });
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [range, setRange] = useState<[number, number]>([0, 0]);
  const scrollRef = useRef<HTMLDivElement>(null);

  const flat = useMemo<Chapter[]>(() => chapters.data?.volumes.flatMap((v) => v.chapters) ?? [], [chapters.data]);

  // Reset per comic; default the language to English when available.
  useEffect(() => {
    setSelected(new Set());
    setLang(languages.includes("en") || languages.length === 0 ? "en" : (languages[0] ?? "en"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [comic?.id, source]);
  useEffect(() => setRange([0, Math.max(flat.length - 1, 0)]), [flat.length]);
  useEffect(() => {
    if (formats.length && !formats.includes(format)) setFormat(formats[0]!);
  }, [formats, format]);

  const pickFormat = (f: Format) => {
    setFormat(f);
    try {
      localStorage.setItem(FORMAT_KEY, f);
    } catch {
      // ignore
    }
  };

  const toggle = (id: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  const selectSpan = (from: number, to: number) =>
    setSelected((prev) => {
      const next = new Set(prev);
      for (let i = from; i <= to; i++) next.add(flat[i]!.id);
      return next;
    });

  const title = comic ? displayTitle(comic.title) : "";
  const count = selected.size;
  const download = () => {
    if (!comic || count === 0) return;
    const refs = flat.filter((c) => selected.has(c.id)).map((c) => ({ id: c.id, number: c.number }));
    onDownload({ source, comicTitle: title, chapters: refs, format, lang: multiLang ? lang : undefined });
  };

  return (
    <Sheet
      open={open}
      onOpenChange={onOpenChange}
      title={title}
      description={
        chapters.data ? `${sourceName}, ${plural(chapters.data.total_chapters, "chapter")}` : sourceName
      }
    >
      <div className="flex items-start gap-4 px-5 pb-4 md:px-6">
        {comic && <CoverImage cover={comic.cover} alt="" className="w-16 shrink-0 rounded-chip" />}
        <div className="flex min-w-0 flex-1 flex-col gap-3">
          {multiLang && (
            <div className="flex flex-col gap-1.5">
              <span className="text-xs font-medium text-fg-muted">Language</span>
              <Select
                label="Language"
                value={lang}
                onChange={setLang}
                options={languages.map((l) => ({ value: l, label: l.toUpperCase() }))}
                className="h-9 w-32"
              />
            </div>
          )}
          {flat.length > 0 && (
            <RangeControls
              chapters={flat}
              range={range}
              onRange={setRange}
              onSelectRange={() => selectSpan(range[0], range[1])}
              onSelectAll={() => setSelected(new Set(flat.map((c) => c.id)))}
              onClear={() => setSelected(new Set())}
            />
          )}
        </div>
      </div>

      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto border-t border-line px-5 py-3 md:px-6">
        {chapters.isPending ? (
          <div className="grid grid-cols-4 gap-2 sm:grid-cols-6" aria-busy aria-label="Loading chapters">
            {Array.from({ length: 24 }).map((_, i) => (
              <Skeleton key={i} className="h-7" />
            ))}
          </div>
        ) : chapters.error ? (
          <div className="flex flex-col items-start gap-3 py-4">
            <p className="text-sm">
              {chapters.error instanceof ApiError ? chapters.error.friendly : "Could not load chapters."}
            </p>
            <Button size="sm" onClick={() => void chapters.refetch()}>
              <ArrowsClockwise size={14} /> Try again
            </Button>
          </div>
        ) : flat.length === 0 ? (
          <p className="py-4 text-sm text-fg-muted">
            No chapters {multiLang ? `in ${lang.toUpperCase()}` : "listed"} for this title.
          </p>
        ) : (
          <ChapterGrid
            volumes={chapters.data!.volumes}
            selected={selected}
            onToggle={toggle}
            onSpan={selectSpan}
            scrollRef={scrollRef}
          />
        )}
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3 border-t border-line bg-bg px-5 py-3 md:px-6">
        <span className="tabular text-sm text-fg-muted" aria-live="polite">
          {count === 0 ? "Nothing selected" : `${plural(count, "chapter")} selected`}
        </span>
        <div className="flex items-center gap-2">
          <FormatPicker value={format} onChange={pickFormat} formats={formats} />
          <Tooltip content={count === 0 ? "Pick at least one chapter" : `Download ${plural(count, "chapter")} as ${format.toUpperCase()}`}>
            <span tabIndex={count === 0 ? 0 : -1} className="inline-flex rounded-control">
              <Button variant="primary" onClick={download} disabled={count === 0}>
                <DownloadSimple size={16} /> Download
              </Button>
            </span>
          </Tooltip>
        </div>
      </div>
    </Sheet>
  );
}
