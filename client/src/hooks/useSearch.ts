import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { SearchResponse } from "@/api/types";
import { usePrefs } from "@/store/prefs";
import { ALL_SOURCES } from "./useSources";

export function useSearch(q: string, source: string) {
  const query = q.trim();
  const adult = usePrefs((s) => s.showAdult);
  return useQuery({
    queryKey: ["search", source || ALL_SOURCES, query.toLowerCase(), adult],
    queryFn: async () => {
      const { data } = await api.GET("/search", {
        params: { query: { q: query, source: source || ALL_SOURCES, adult } },
      });
      return data as SearchResponse;
    },
    enabled: query.length > 0,
    staleTime: 10 * 60_000,
  });
}
