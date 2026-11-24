import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";

import type { SiteTab, ViewId } from "./nav";

/** Where the app is: a page, its subpage (tab or section) and, on Sites, the site's tab. */
export interface Route {
  view: ViewId;
  sub?: string;
}

/** One browser-like tab on the Sites page: a site (or none: the new-tab page) and its subpage. */
export interface SiteTabState {
  id: string;
  site: string | null;
  tab: SiteTab;
  /** Opened by LocDev itself (first launch): filled with a running site. */
  auto?: boolean;
}

interface UiState {
  view: ViewId;
  sub: string | undefined;
  /** Goes to a page (and optionally one of its subpages), recording history. */
  navigate: (view: ViewId, sub?: string) => void;
  back: () => void;
  forward: () => void;
  canBack: boolean;
  canForward: boolean;
  /** Site tabs, in order, and the one in front. */
  tabs: SiteTabState[];
  activeTab: string;
  activateTab: (id: string) => void;
  /** Opens an empty tab, or one with `site`. */
  newTab: (site?: string | null) => void;
  closeTab: (id: string) => void;
  /** Moves a tab to another position (drag and drop). */
  moveTab: (id: string, to: number) => void;
  /** Opens `name` in its own tab (switching to it when one is already open). */
  openInNewTab: (name: string) => void;
  /** Fills the tabs LocDev opened on its own, and drops tabs of sites that are gone. */
  reconcileTabs: (sites: string[], preferred: string | null) => void;
  /** Name of the site in the front tab. */
  selected: string | null;
  /** Shows a site in the front tab (or switches to the tab that has it). */
  select: (name: string | null) => void;
  siteTab: SiteTab;
  setSiteTab: (tab: SiteTab) => void;
  /** Opens a site on one of its tabs. */
  openSite: (name: string, tab?: SiteTab) => void;
  addSiteOpen: boolean;
  setAddSiteOpen: (open: boolean) => void;
  /** Name of the site whose edit dialog is open. */
  editing: string | null;
  setEditing: (name: string | null) => void;
  /** Name of the site waiting for removal confirmation. */
  removing: string | null;
  setRemoving: (name: string | null) => void;
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
  /** The site in the "Share on the internet" dialog ("" = pick one). */
  sharing: string | null;
  setSharing: (site: string | null) => void;
}

const UiContext = createContext<UiState | null>(null);

const same = (a: Route, b: Route) => a.view === b.view && a.sub === b.sub;

/** Session storage survives a webview reload (dev) but not a relaunch. */
function restore<T>(key: string, fallback: T): T {
  try {
    const raw = sessionStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}

function persist(key: string, value: unknown) {
  try {
    sessionStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage unavailable: the route just isn't remembered.
  }
}

interface Tabs {
  list: SiteTabState[];
  active: string;
}

const TABS_KEY = "locdev.siteTabs";
const tabId = () => Math.random().toString(36).slice(2, 10);

/** Tabs come back after a relaunch, as in a browser (local storage, this machine only). */
function restoreTabs(): Tabs {
  try {
    const saved = JSON.parse(localStorage.getItem(TABS_KEY) ?? "null") as Tabs | null;
    if (saved?.list?.length && saved.list.some((t) => t.id === saved.active)) return saved;
  } catch {
    // Unreadable or unavailable: start over.
  }
  const id = tabId();
  return { list: [{ id, site: restore<string | null>("locdev.site", null), tab: "overview", auto: true }], active: id };
}

function persistTabs(tabs: Tabs) {
  try {
    localStorage.setItem(TABS_KEY, JSON.stringify(tabs));
  } catch {
    // Storage unavailable: tabs last for this session only.
  }
}

/** `tabs` with the front tab changed by `change`. */
const withActive = (tabs: Tabs, change: (tab: SiteTabState) => SiteTabState): Tabs => ({
  ...tabs,
  list: tabs.list.map((t) => (t.id === tabs.active ? change(t) : t)),
});

/** Shows `name` in front: its tab if one is open, else in the front tab. */
function focusSite(tabs: Tabs, name: string, tab?: SiteTab): Tabs {
  const open = tabs.list.find((t) => t.site === name);
  const next = open ? { ...tabs, active: open.id } : withActive(tabs, (t) => ({ ...t, site: name, auto: false }));
  return tab ? withActive(next, (t) => ({ ...t, tab })) : next;
}

export function UiProvider({ children }: { children: ReactNode }) {
  const [history, setHistory] = useState(() => ({
    stack: [restore<Route>("locdev.route", { view: "dashboard" })],
    at: 0,
  }));
  const [tabs, setTabs] = useState<Tabs>(restoreTabs);
  const [addSiteOpen, setAddSiteOpen] = useState(false);
  const [editing, setEditing] = useState<string | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [sharing, setSharing] = useState<string | null>(null);

  const route = history.stack[history.at];
  const front = tabs.list.find((t) => t.id === tabs.active) ?? tabs.list[0];
  useEffect(() => persist("locdev.route", route), [route]);
  useEffect(() => persistTabs(tabs), [tabs]);

  const navigate = useCallback((view: ViewId, sub?: string) => {
    setHistory(({ stack, at }) => {
      const next = { view, sub };
      if (same(stack[at], next)) return { stack, at };
      // Keep the last 50 steps; going somewhere new drops the forward branch.
      const kept = [...stack.slice(0, at + 1), next].slice(-50);
      return { stack: kept, at: kept.length - 1 };
    });
  }, []);
  const back = useCallback(() => setHistory((h) => ({ ...h, at: Math.max(0, h.at - 1) })), []);
  const forward = useCallback(() => setHistory((h) => ({ ...h, at: Math.min(h.stack.length - 1, h.at + 1) })), []);

  const select = useCallback(
    (name: string | null) =>
      setTabs((s) => (name === null ? withActive(s, (t) => ({ ...t, site: null, auto: false })) : focusSite(s, name))),
    [],
  );
  const setSiteTab = useCallback((tab: SiteTab) => setTabs((s) => withActive(s, (t) => ({ ...t, tab }))), []);
  const openSite = useCallback(
    (name: string, tab?: SiteTab) => {
      setTabs((s) => focusSite(s, name, tab));
      navigate("sites");
    },
    [navigate],
  );
  const activateTab = useCallback((id: string) => setTabs((s) => ({ ...s, active: id })), []);
  const newTab = useCallback((site: string | null = null) => {
    setTabs((s) => {
      const open = site && s.list.find((t) => t.site === site);
      if (open) return { ...s, active: open.id };
      const tab: SiteTabState = { id: tabId(), site, tab: "overview" };
      // Right after the front tab, as browsers do.
      const at = s.list.findIndex((t) => t.id === s.active) + 1;
      return { list: [...s.list.slice(0, at), tab, ...s.list.slice(at)], active: tab.id };
    });
  }, []);
  const openInNewTab = useCallback(
    (name: string) => {
      newTab(name);
      navigate("sites");
    },
    [newTab, navigate],
  );
  const closeTab = useCallback(
    (id: string) =>
      setTabs((s) => {
        const at = s.list.findIndex((t) => t.id === id);
        if (at < 0) return s;
        const list = s.list.filter((t) => t.id !== id);
        if (!list.length) {
          // Closing the last tab leaves an empty one: the page never goes blank.
          const fresh = { id: tabId(), site: null, tab: "overview" as SiteTab };
          return { list: [fresh], active: fresh.id };
        }
        const active = s.active === id ? (list[at] ?? list[at - 1]).id : s.active;
        return { list, active };
      }),
    [],
  );
  const moveTab = useCallback(
    (id: string, to: number) =>
      setTabs((s) => {
        const tab = s.list.find((t) => t.id === id);
        if (!tab) return s;
        const list = s.list.filter((t) => t.id !== id);
        list.splice(Math.max(0, Math.min(to, list.length)), 0, tab);
        return { ...s, list };
      }),
    [],
  );
  const reconcileTabs = useCallback((sites: string[], preferred: string | null) => {
    setTabs((s) => {
      let changed = false;
      const list: SiteTabState[] = [];
      for (const t of s.list) {
        const gone = t.site !== null && !sites.includes(t.site);
        if (t.auto && (t.site === null || gone)) {
          // The tab LocDev opened on first launch gets a site as soon as there is one.
          if (preferred || gone) changed = true;
          list.push(preferred ? { ...t, site: preferred, auto: false } : { ...t, site: null });
        } else if (gone) {
          // The site was removed: its tab closes.
          changed = true;
        } else {
          list.push(t);
        }
      }
      if (!changed) return s;
      if (!list.length) {
        const fresh = { id: tabId(), site: null, tab: "overview" as SiteTab };
        return { list: [fresh], active: fresh.id };
      }
      const at = s.list.findIndex((t) => t.id === s.active);
      const active = list.some((t) => t.id === s.active) ? s.active : list[Math.min(at, list.length - 1)].id;
      return { list, active };
    });
  }, []);

  const value = useMemo(
    () => ({
      view: route.view,
      sub: route.sub,
      navigate,
      back,
      forward,
      canBack: history.at > 0,
      canForward: history.at < history.stack.length - 1,
      tabs: tabs.list,
      activeTab: front.id,
      activateTab,
      newTab,
      closeTab,
      moveTab,
      openInNewTab,
      reconcileTabs,
      selected: front.site,
      select,
      siteTab: front.tab,
      setSiteTab,
      openSite,
      addSiteOpen,
      setAddSiteOpen,
      editing,
      setEditing,
      removing,
      setRemoving,
      paletteOpen,
      setPaletteOpen,
      sharing,
      setSharing,
    }),
    [
      route,
      history,
      navigate,
      back,
      forward,
      tabs,
      front,
      activateTab,
      newTab,
      closeTab,
      moveTab,
      openInNewTab,
      reconcileTabs,
      select,
      setSiteTab,
      openSite,
      addSiteOpen,
      editing,
      removing,
      paletteOpen,
      sharing,
    ],
  );

  return <UiContext.Provider value={value}>{children}</UiContext.Provider>;
}

export function useUi() {
  const ui = useContext(UiContext);
  if (!ui) throw new Error("useUi must be used inside <UiProvider>");
  return ui;
}
