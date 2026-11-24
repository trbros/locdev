import { createContext, type ReactNode, useContext, useEffect, useMemo, useState } from "react";

/** App preferences that live in this webview (the daemon never needs them). */
export interface Prefs {
  sidebar: "expanded" | "collapsed";
  /** `system` follows the OS "reduce motion" setting. */
  motion: "system" | "reduce" | "full";
  /** Look for a newer app at launch and once a day. */
  updateCheck: boolean;
  /** Download and install it right away (the restart is still yours). */
  updateInstall: boolean;
}

const DEFAULTS: Prefs = { sidebar: "expanded", motion: "system", updateCheck: true, updateInstall: false };
const KEY = "locdev.prefs";

function load(): Prefs {
  try {
    return { ...DEFAULTS, ...JSON.parse(localStorage.getItem(KEY) ?? "{}") };
  } catch {
    return DEFAULTS;
  }
}

interface PrefsState extends Prefs {
  set: <K extends keyof Prefs>(key: K, value: Prefs[K]) => void;
}

const PrefsContext = createContext<PrefsState | null>(null);

export function PrefsProvider({ children }: { children: ReactNode }) {
  const [prefs, setPrefs] = useState(load);

  useEffect(() => {
    try {
      localStorage.setItem(KEY, JSON.stringify(prefs));
    } catch {
      // Unavailable storage: preferences last for this session only.
    }
    // CSS reads this to tone animations down (or keep them on).
    document.documentElement.dataset.motion = prefs.motion;
  }, [prefs]);

  const value = useMemo(
    () => ({
      ...prefs,
      set: <K extends keyof Prefs>(key: K, value: Prefs[K]) => setPrefs((p) => ({ ...p, [key]: value })),
    }),
    [prefs],
  );
  return <PrefsContext.Provider value={value}>{children}</PrefsContext.Provider>;
}

export function usePrefs() {
  const prefs = useContext(PrefsContext);
  if (!prefs) throw new Error("usePrefs must be used inside <PrefsProvider>");
  return prefs;
}

/** Tracks a media query, e.g. a narrow window. */
export function useMedia(query: string) {
  const [matches, setMatches] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const list = window.matchMedia(query);
    const update = () => setMatches(list.matches);
    list.addEventListener("change", update);
    update();
    return () => list.removeEventListener("change", update);
  }, [query]);
  return matches;
}
