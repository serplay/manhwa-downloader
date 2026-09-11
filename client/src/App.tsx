import { useState } from "react";
import { Header } from "@/components/layout/Header";
import { SearchBar } from "@/components/search/SearchBar";
import { Results, ResultsSkeleton } from "@/components/results/Results";
import { useSearch } from "@/hooks/useSearch";
import { ALL_SOURCES, useSources } from "@/hooks/useSources";
import { useUrlState } from "@/hooks/useUrlState";
import type { Comic } from "@/api/types";

export default function App() {
  const [{ q, source }, setUrl] = useUrlState();
  const sources = useSources();
  const search = useSearch(q, source);
  const [, setSelected] = useState<{ comic: Comic; source: string } | null>(null);

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
              onOpen={(comic, s) => setSelected({ comic, source: s })}
              onSearchAll={() => setUrl({ source: "" })}
              onRetry={() => void search.refetch()}
            />
          )
        ) : (
          <p className="text-sm text-fg-muted">Search a title to get started. Results show covers first; open one to pick chapters.</p>
        )}
      </main>
    </div>
  );
}
