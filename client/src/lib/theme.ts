export type ThemeMode = "system" | "light" | "dark";

const STORAGE_KEY = "theme";

export function readThemeMode(): ThemeMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === "light" || v === "dark" ? v : "system";
  } catch {
    return "system";
  }
}

export function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode !== "system") return mode;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** Apply a mode to <html> and persist it. Mirrors the inline script in index.html. */
export function applyThemeMode(mode: ThemeMode): void {
  document.documentElement.classList.toggle("dark", resolveTheme(mode) === "dark");
  try {
    if (mode === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, mode);
  } catch {
    // Storage can be unavailable (private mode); the class still applies.
  }
}
