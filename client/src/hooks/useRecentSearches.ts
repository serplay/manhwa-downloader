import { useCallback, useSyncExternalStore } from "react";

const KEY = "recent-searches";
const MAX = 8;
const listeners = new Set<() => void>();
let cache: string[] | null = null;

function read(): string[] {
  if (cache) return cache;
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "[]");
    cache = Array.isArray(raw) ? raw.filter((x): x is string => typeof x === "string").slice(0, MAX) : [];
  } catch {
    cache = [];
  }
  return cache;
}

function write(next: string[]) {
  cache = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // storage unavailable; in-memory list still works for this session
  }
  listeners.forEach((l) => l());
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

/** Last eight distinct queries, most recent first. */
export function useRecentSearches() {
  const items = useSyncExternalStore(subscribe, read, () => []);
  const add = useCallback((q: string) => {
    const t = q.trim();
    if (!t) return;
    write([t, ...read().filter((x) => x.toLowerCase() !== t.toLowerCase())].slice(0, MAX));
  }, []);
  const remove = useCallback((q: string) => write(read().filter((x) => x !== q)), []);
  const clear = useCallback(() => write([]), []);
  return { items, add, remove, clear };
}
