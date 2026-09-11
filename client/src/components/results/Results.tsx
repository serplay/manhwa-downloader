import { ArrowsClockwise } from "@phosphor-icons/react";
import { ApiError } from "@/api/client";
import type { Comic, ErrorDetail, SearchResponse, SourceInfo } from "@/api/types";
import { Button } from "@/components/ui/Button";
import { ALL_SOURCES } from "@/hooks/useSources";
import { plural } from "@/lib/format";
import { ComicCard, ComicCardSkeleton } from "./ComicCard";

const GRID = "grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5";

function GroupHeader({ name, count, error }: { name: string; count?: number; error?: ErrorDetail }) {
  return (
    <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b border-line pb-2">
      <h2 className="text-base font-semibold tracking-tight">{name}</h2>
      {count != null && <span className="tabular text-xs text-fg-faint">{plural(count, "result")}</span>}
      {error && (
        <span className="text-xs text-danger">
          {error.code === "SOURCE_BLOCKED" ? "Needs a browser, skipped" : error.message}
        </span>
      )}
    </div>
  );
}

export function ResultsSkeleton({ groups = 1 }: { groups?: number }) {
  return (
    <div className="flex flex-col gap-8" aria-busy aria-label="Searching">
      {Array.from({ length: groups }).map((_, g) => (
        <section key={g} className="flex flex-col gap-4">
          <div className="shimmer h-6 w-40 rounded-chip" />
          <div className={GRID}>
            {Array.from({ length: 5 }).map((_, i) => (
              <ComicCardSkeleton key={i} />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

export function Results({
  query,
  source,
  sources,
  data,
  error,
  onOpen,
  onSearchAll,
  onRetry,
}: {
  query: string;
  source: string;
  sources: SourceInfo[] | undefined;
  data: SearchResponse | undefined;
  error: unknown;
  onOpen: (comic: Comic, sourceSlug: string) => void;
  onSearchAll: () => void;
  onRetry: () => void;
}) {
  const nameOf = (slug: string) => sources?.find((s) => s.slug === slug)?.name ?? slug;

  if (error) {
    const message = error instanceof ApiError ? error.friendly : "Something went wrong. Try again.";
    return (
      <div className="flex flex-col items-start gap-3 rounded-panel bg-surface p-6">
        <p className="text-sm">{message}</p>
        <div className="flex gap-2">
          <Button onClick={onRetry} size="sm">
            <ArrowsClockwise size={14} /> Try again
          </Button>
          {source !== ALL_SOURCES && source && (
            <Button onClick={onSearchAll} size="sm" variant="ghost">
              Search all sources
            </Button>
          )}
        </div>
      </div>
    );
  }
  if (!data) return null;

  const order = sources?.map((s) => s.slug) ?? Object.keys(data.results);
  const groups = order
    .filter((slug) => slug in data.results || slug in data.errors)
    .map((slug) => ({ slug, comics: data.results[slug] ?? [], error: data.errors[slug] }));
  const total = groups.reduce((n, g) => n + g.comics.length, 0);

  if (total === 0) {
    const where = source && source !== ALL_SOURCES ? `on ${nameOf(source)}` : "on any source";
    return (
      <div className="flex flex-col items-start gap-3 rounded-panel bg-surface p-6">
        <p className="text-sm">
          No results for “{query}” {where}.
          {source && source !== ALL_SOURCES ? " Try another source or search all sources." : " Try a shorter title."}
        </p>
        {groups.some((g) => g.error) && (
          <ul className="text-xs text-fg-muted">
            {groups
              .filter((g) => g.error)
              .map((g) => (
                <li key={g.slug}>
                  {nameOf(g.slug)}: {g.error!.message}
                </li>
              ))}
          </ul>
        )}
        {source && source !== ALL_SOURCES && (
          <Button onClick={onSearchAll} size="sm" variant="primary">
            Search all sources
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-10">
      {groups.map((g) => (
        <section key={g.slug} className="flex flex-col gap-4" aria-label={nameOf(g.slug)}>
          <GroupHeader name={nameOf(g.slug)} count={g.error ? undefined : g.comics.length} error={g.error} />
          {g.comics.length > 0 && (
            <div className={GRID}>
              {g.comics.map((c) => (
                <ComicCard key={c.id} comic={c} onOpen={(comic) => onOpen(comic, g.slug)} />
              ))}
            </div>
          )}
        </section>
      ))}
    </div>
  );
}
