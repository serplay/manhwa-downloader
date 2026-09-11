import { useQuery } from "@tanstack/react-query";
import { api } from "@/api/client";
import type { SourceInfo, SourceStatus } from "@/api/types";

export const ALL_SOURCES = "all";

export function useSources() {
  return useQuery({
    queryKey: ["sources"],
    queryFn: async () => (await api.GET("/sources")).data as SourceInfo[],
    staleTime: 5 * 60_000,
    refetchInterval: 5 * 60_000,
  });
}

export function useHealth() {
  return useQuery({
    queryKey: ["health"],
    queryFn: async () => (await api.GET("/health")).data!,
    staleTime: 5 * 60_000,
  });
}

/** A source can be searched from this build. */
export function isUsable(s: SourceInfo): boolean {
  return s.capabilities.search;
}

export function speedLabel(speed: SourceInfo["speed"]): string {
  return speed === "fastest" ? "Fastest" : speed === "fast" ? "Fast" : "Slow";
}

export const statusLabel: Record<SourceStatus, string> = {
  ok: "Up",
  down: "Down",
  blocked: "Needs a browser",
  unknown: "Unknown",
};
