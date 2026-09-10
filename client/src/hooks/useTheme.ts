import { useCallback, useEffect, useState } from "react";
import { applyThemeMode, readThemeMode, resolveTheme, type ThemeMode } from "@/lib/theme";

export function useTheme() {
  const [mode, setModeState] = useState<ThemeMode>(() => readThemeMode());
  const [resolved, setResolved] = useState<"light" | "dark">(() => resolveTheme(mode));

  useEffect(() => {
    applyThemeMode(mode);
    setResolved(resolveTheme(mode));
    if (mode !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => {
      applyThemeMode("system");
      setResolved(resolveTheme("system"));
    };
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [mode]);

  const setMode = useCallback((next: ThemeMode) => setModeState(next), []);
  return { mode, resolved, setMode };
}
