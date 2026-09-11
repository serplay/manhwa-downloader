/** Query-string state (`?q=&source=`) so searches are shareable and survive reload. */

export interface UrlState {
  q: string;
  source: string;
}

const listeners = new Set<() => void>();

export function readUrlState(): UrlState {
  const p = new URLSearchParams(window.location.search);
  return { q: p.get("q") ?? "", source: p.get("source") ?? "" };
}

export function writeUrlState(next: Partial<UrlState>, { replace = false } = {}): void {
  const p = new URLSearchParams(window.location.search);
  for (const [k, v] of Object.entries(next)) {
    if (v) p.set(k, v);
    else p.delete(k);
  }
  const qs = p.toString();
  const url = `${window.location.pathname}${qs ? `?${qs}` : ""}`;
  if (replace) history.replaceState(null, "", url);
  else history.pushState(null, "", url);
  listeners.forEach((l) => l());
}

export function subscribeUrlState(listener: () => void): () => void {
  listeners.add(listener);
  window.addEventListener("popstate", listener);
  return () => {
    listeners.delete(listener);
    window.removeEventListener("popstate", listener);
  };
}
