import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { ChaptersResponse } from "@/api/types";

export function useChapters(source: string, id: string, lang?: string) {
  return useQuery({
    queryKey: ["chapters", source, id, lang ?? ""],
    queryFn: async () => {
      const { data } = await api.GET("/chapters", {
        params: { query: { source, id, lang: lang || undefined } },
      });
      return data as ChaptersResponse;
    },
    enabled: Boolean(source && id),
    staleTime: 10 * 60_000,
  });
}
