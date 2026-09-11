import { Suspense, lazy, useEffect, useState } from "react";
import { toast } from "sonner";
import { ApiError } from "@/api/client";
import type { DownloadIntent } from "@/components/chapters/ChapterPicker";
import { Header } from "@/components/layout/Header";
import { SearchBar } from "@/components/search/SearchBar";
import { Results, ResultsSkeleton } from "@/components/results/Results";
import { useSearch } from "@/hooks/useSearch";
import { ALL_SOURCES, useSources } from "@/hooks/useSources";
import { useUrlState } from "@/hooks/useUrlState";
import { useDownloads } from "@/store/downloads";
import { usePrefs } from "@/store/prefs";
import type { Comic } from "@/api/types";

const ChapterPicker = lazy(() =>
  import("@/components/chapters/ChapterPicker").then((m) => ({ default: m.ChapterPicker })),
);
const DownloadsTray = lazy(() =>
  import("@/components/downloads/DownloadsTray").then((m) => ({ default: m.DownloadsTray })),
);

export default function App() {
  const [{ q, source }, setUrl] = useUrlState();
  const sources = useSources();
  const search = useSearch(q, source);
  const showAdult = usePrefs((s) => s.showAdult);

  // Turning adult content off while an adult source is selected falls back to all sources.
  useEffect(() => {
    if (!showAdult && sources.data?.some((s) => s.slug === source && s.adult)) setUrl({ source: "" }, { replace: true });
  }, [showAdult, source, sources.data, setUrl]);
  const [selected, setSelected] = useState<{ comic: Comic; source: string } | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);

  const openPicker = (comic: Comic, s: string) => {
    setSelected({ comic, source: s });
    setPickerOpen(true);
  };
  const start = useDownloads((s) => s.start);
  const startDownload = async (intent: DownloadIntent) => {
    try {
      await start(intent);
      toast.success("Download started");
      setPickerOpen(false);
    } catch (e) {
      toast.error(e instanceof ApiError ? e.friendly : "Could not start the download");
    }
  };

  const onSearch = (nextQ: string, nextSource: string) =>
    setUrl({ q: nextQ, source: nextSource === ALL_SOURCES ? "" : nextSource });

  return (
    <div className="mx-auto flex min-h-dvh w-full max-w-6xl flex-col px-4 sm:px-6">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:rounded-control focus:bg-raised focus:px-3 focus:py-2 focus:text-sm"
      >
        Skip to content
      </a>
      <Header />
      <main id="main" className="flex flex-1 flex-col gap-8 py-6 sm:py-8">
        <SearchBar query={q} source={source} onSearch={onSearch} busy={search.isFetching} />
        {q ? (
          search.isPending ? (
            <ResultsSkeleton groups={source ? 1 : 3} />
          ) : (
            <Results
              query={q}
              source={source || ALL_SOURCES}
              sources={sources.data}
              data={search.data}
              error={search.error}
              onOpen={openPicker}
              onSearchAll={() => setUrl({ source: "" })}
              onRetry={() => void search.refetch()}
            />
          )
        ) : (
          <p className="text-sm text-fg-muted">Search a title to get started. Results show covers first; open one to pick chapters.</p>
        )}
      </main>
      <Suspense fallback={null}>
        <DownloadsTray />
      </Suspense>
      {selected && (
        <Suspense fallback={null}>
          <ChapterPicker
        comic={selected?.comic ?? null}
        source={selected?.source ?? ""}
        sourceName={sources.data?.find((s) => s.slug === selected?.source)?.name ?? selected?.source ?? ""}
        open={pickerOpen}
        onOpenChange={setPickerOpen}
        onDownload={(intent) => void startDownload(intent)}
          />
        </Suspense>
      )}
    </div>
  );
}
