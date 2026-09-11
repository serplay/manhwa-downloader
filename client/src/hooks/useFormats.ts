import type { Format } from "@/api/types";
import { useHealth } from "./useSources";

const ORDER: Format[] = ["pdf", "cbz", "epub", "cbr"];

/** Formats the running server advertises, in a stable order. */
export function useFormats(): Format[] {
  const health = useHealth();
  const available = new Set(health.data?.capabilities.formats ?? ["pdf", "cbz", "epub"]);
  return ORDER.filter((f) => available.has(f));
}

