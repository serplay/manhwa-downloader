import { create } from "zustand";
import { persist } from "zustand/middleware";

interface Prefs {
  /** Show titles and sources flagged adult. Off by default. */
  showAdult: boolean;
  setShowAdult: (v: boolean) => void;
}

export const usePrefs = create<Prefs>()(
  persist(
    (set) => ({
      showAdult: false,
      setShowAdult: (showAdult) => set({ showAdult }),
    }),
    { name: "prefs", version: 1 },
  ),
);
