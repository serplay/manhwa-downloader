import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { SearchResponse } from "@/api/types";
import { ALL_SOURCES } from "./useSources";

export function useSearch(q: string, source: string) {
  const query = q.trim();
  return useQuery({
    queryKey: ["search", source || ALL_SOURCES, query.toLowerCase()],
    queryFn: async () => {
      const { data } = await api.GET("/search", {
        params: { query: { q: query, source: source || ALL_SOURCES } },
      });
      return data as SearchResponse;
    },
    enabled: query.length > 0,
    staleTime: 10 * 60_000,
  });
}
