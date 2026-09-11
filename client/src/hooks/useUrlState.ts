import { useCallback, useSyncExternalStore } from "react";
import { readUrlState, subscribeUrlState, writeUrlState, type UrlState } from "@/lib/url-state";

let cached = readUrlState();
let cachedKey = window.location.search;

function snapshot(): UrlState {
  if (cachedKey !== window.location.search) {
    cached = readUrlState();
    cachedKey = window.location.search;
  }
  return cached;
}

export function useUrlState() {
  const state = useSyncExternalStore(subscribeUrlState, snapshot);
  const set = useCallback((next: Partial<UrlState>, opts?: { replace?: boolean }) => writeUrlState(next, opts), []);
  return [state, set] as const;
}
