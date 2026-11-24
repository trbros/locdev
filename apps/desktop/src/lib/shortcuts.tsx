// Keyboard shortcuts: one registry (what exists, its defaults), the user's
// changes (kept in this webview's storage and followed across windows), one
// keydown listener that runs whichever handler the mounted UI registered.
//
// Combos are written portably: "Mod+K" is ⌘K on macOS and Ctrl+K elsewhere;
// "Ctrl" is the real Control key (on macOS too). Every shortcut is local to
// LocDev's window: none is registered with the operating system.

import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from "react";

import { t } from "@/lib/i18n";
import { TOOLS, VIEWS } from "@/lib/nav";

export const IS_MAC = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);
const OS: "macos" | "windows" | "linux" =
  typeof navigator === "undefined"
    ? "linux"
    : IS_MAC
      ? "macos"
      : /Windows/.test(navigator.userAgent)
        ? "windows"
        : "linux";

export type ShortcutCategory = "general" | "navigation" | "projects" | "servers" | "terminal" | "tools" | "logs";

export const CATEGORIES: { id: ShortcutCategory; label: string }[] = [
  { id: "general", label: t("General") },
  { id: "navigation", label: t("Navigation") },
  { id: "projects", label: t("Projects") },
  { id: "servers", label: t("Servers") },
  { id: "terminal", label: t("Terminal") },
  { id: "tools", label: t("Development tools") },
  { id: "logs", label: t("Logs and diagnostics") },
];

export interface ShortcutDef {
  id: string;
  category: ShortcutCategory;
  label: string;
  defaults: string[];
  /** Only works on this page (the handler is mounted there). */
  where?: string;
  /** Built into an editor or a field: shown, not changeable. */
  fixed?: boolean;
}

const tool = (id: string) => TOOLS.find((s) => s.id === id)!;
const sitesOnly = t("Sites");

export const SHORTCUTS: ShortcutDef[] = [
  { id: "palette", category: "general", label: t("Open the command palette"), defaults: ["Mod+K"] },
  { id: "settings", category: "general", label: t("Open settings"), defaults: ["Mod+,"] },
  { id: "shortcuts", category: "general", label: t("Show keyboard shortcuts"), defaults: ["Mod+/"] },
  { id: "sidebar", category: "general", label: t("Collapse or expand the sidebar"), defaults: ["Mod+B"] },
  { id: "back", category: "general", label: t("Go back"), defaults: ["Alt+ArrowLeft"] },
  { id: "forward", category: "general", label: t("Go forward"), defaults: ["Alt+ArrowRight"] },
  { id: "undo", category: "general", label: t("Undo (Email Studio)"), defaults: ["Mod+Z"], fixed: true },
  { id: "redo", category: "general", label: t("Redo (Email Studio)"), defaults: ["Mod+Shift+Z", "Mod+Y"], fixed: true },
  {
    id: "escape",
    category: "general",
    label: t("Close a dialog or clear a filter"),
    defaults: ["Escape"],
    fixed: true,
  },

  ...VIEWS.map((v, i) => ({
    id: `go:${v.id}`,
    category: "navigation" as const,
    label: t("Go to {page}", { page: v.label }),
    defaults: i < 9 ? [`Mod+${i + 1}`] : [],
  })),

  { id: "add-site", category: "projects", label: t("Add a site"), defaults: ["Mod+N"] },
  { id: "new-project", category: "projects", label: t("Create a new project"), defaults: ["Mod+Shift+N"] },
  { id: "filter-sites", category: "projects", label: t("Filter sites"), defaults: ["Mod+F"], where: sitesOnly },
  { id: "new-tab", category: "projects", label: t("New site tab"), defaults: ["Mod+T"], where: sitesOnly },
  { id: "close-tab", category: "projects", label: t("Close the site tab"), defaults: ["Mod+W"], where: sitesOnly },
  // Ctrl, not ⌘: ⌘Tab switches apps on macOS and never reaches LocDev.
  { id: "next-tab", category: "projects", label: t("Next site tab"), defaults: ["Ctrl+Tab"], where: sitesOnly },
  {
    id: "previous-tab",
    category: "projects",
    label: t("Previous site tab"),
    defaults: ["Ctrl+Shift+Tab"],
    where: sitesOnly,
  },
  {
    id: "site-browser",
    category: "projects",
    label: t("Open the site in the browser"),
    defaults: [],
    where: sitesOnly,
  },
  {
    id: "site-editor",
    category: "projects",
    label: t("Open the site in the editor"),
    defaults: ["Mod+E"],
    where: sitesOnly,
  },
  { id: "site-folder", category: "projects", label: t("Reveal the site's folder"), defaults: [], where: sitesOnly },

  { id: "site-start-stop", category: "servers", label: t("Start or stop the site"), defaults: [], where: sitesOnly },
  { id: "site-restart", category: "servers", label: t("Restart the site"), defaults: [], where: sitesOnly },
  { id: "site-share", category: "servers", label: t("Share the site on the internet"), defaults: [], where: sitesOnly },

  { id: "tool:cli", category: "terminal", label: t("Open {page}", { page: tool("cli").label }), defaults: [] },

  ...["new-project", "installers", "database", "mailbox", "tunnels", "actions"].map((id) => ({
    id: `tool:${id}`,
    category: "tools" as const,
    label: t("Open {page}", { page: tool(id).label }),
    defaults: [],
  })),

  { id: "tool:processes", category: "logs", label: t("Open {page}", { page: tool("processes").label }), defaults: [] },
  {
    id: "tool:debug",
    category: "logs",
    label: t("Open {page}", { page: tool("debug").label }),
    defaults: ["Mod+Shift+D"],
  },
  { id: "tool:doctor", category: "logs", label: t("Open {page}", { page: tool("doctor").label }), defaults: [] },
];

const BY_ID = new Map(SHORTCUTS.map((s) => [s.id, s]));
export const shortcutById = (id: string) => BY_ID.get(id);

// ── Combos ───────────────────────────────────────────────────────────────────

const PUNCTUATION: Record<string, string> = {
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Minus: "-",
  Equal: "=",
  Backquote: "`",
  Space: "Space",
};
const MODIFIER_KEYS = new Set(["Control", "Shift", "Alt", "Meta", "OS", "AltGraph", "CapsLock", "Fn"]);

/** The combo an event is, or null for a lone modifier. Letters follow the
 * keyboard layout; digits and punctuation the physical key (Shift changes
 * what they type). */
export function comboOf(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  let key: string;
  if (/^[a-z]$/i.test(e.key)) key = e.key.toUpperCase();
  else if (/^(Digit|Numpad)\d$/.test(e.code)) key = e.code.slice(-1);
  else if (PUNCTUATION[e.code]) key = PUNCTUATION[e.code];
  else if (e.key === " ") key = "Space";
  else key = e.key;
  const mods: string[] = [];
  if (IS_MAC) {
    if (e.metaKey) mods.push("Mod");
    if (e.ctrlKey) mods.push("Ctrl");
  } else {
    if (e.ctrlKey) mods.push("Mod");
    if (e.metaKey) mods.push("Meta");
  }
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  return [...mods, key].join("+");
}

/** Same spelling for the same keys: Ctrl is Mod off macOS, fixed modifier order. */
export function normalize(combo: string): string {
  const parts = combo.split("+");
  const key = parts.pop()!;
  const mods = new Set(parts.map((p) => (!IS_MAC && p === "Ctrl" ? "Mod" : p)));
  return [...["Mod", "Ctrl", "Meta", "Alt", "Shift"].filter((m) => mods.has(m)), key].join("+");
}

const KEY_LABEL: Record<string, string> = {
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Escape: "Esc",
  Enter: "↵",
  Backspace: "⌫",
  Delete: "Del",
};

/** The keys to show, one per chip: ["⌘", "K"] / ["Ctrl", "K"]. */
export function keysOf(combo: string): string[] {
  const parts = normalize(combo).split("+");
  const key = parts.pop()!;
  const mod = (m: string) =>
    IS_MAC
      ? ({ Mod: "⌘", Ctrl: "⌃", Alt: "⌥", Shift: "⇧", Meta: "⌘" } as Record<string, string>)[m]
      : (
          { Mod: "Ctrl", Ctrl: "Ctrl", Alt: "Alt", Shift: "Shift", Meta: OS === "windows" ? "Win" : "Super" } as Record<
            string,
            string
          >
        )[m];
  return [...parts.map(mod), KEY_LABEL[key] ?? key];
}

export const formatCombo = (combo: string) => keysOf(combo).join(IS_MAC ? "" : "+");

/** Needs a modifier (or is a function key), so typing never fires it. */
export function usable(combo: string): boolean {
  const parts = normalize(combo).split("+");
  const key = parts.pop()!;
  const mods = parts.filter((p) => p !== "Shift");
  return mods.length > 0 || /^F\d{1,2}$/.test(key);
}

/** Why the system (or text editing) may keep this combo from LocDev. */
export function reservedReason(combo: string): string | null {
  const c = normalize(combo);
  const editing = ["Mod+C", "Mod+V", "Mod+X", "Mod+A", "Mod+Z", "Mod+Shift+Z", "Mod+Y"];
  if (editing.includes(c)) return t("Copy, paste, select all and undo use it in text fields.");
  if (c.split("+").includes("Meta")) {
    return OS === "windows"
      ? t("Windows keeps most combinations with the Windows key.")
      : t("The desktop usually keeps combinations with the Super key.");
  }
  const system: Record<typeof OS, string[]> = {
    macos: [
      "Mod+Q",
      "Mod+H",
      "Mod+M",
      "Mod+Tab",
      "Mod+Space",
      "Mod+`",
      "Mod+Alt+Escape",
      "Mod+Shift+3",
      "Mod+Shift+4",
      "Mod+Shift+5",
      "Ctrl+ArrowLeft",
      "Ctrl+ArrowRight",
      "Ctrl+ArrowUp",
      "Ctrl+ArrowDown",
      "Ctrl+Space",
    ],
    windows: ["Alt+F4", "Alt+Tab", "Alt+Escape", "Mod+Escape", "Mod+Shift+Escape", "Mod+Alt+Delete"],
    linux: [
      "Alt+F4",
      "Alt+Tab",
      "Alt+F2",
      "Mod+Alt+T",
      "Mod+Alt+Delete",
      "Mod+Alt+ArrowLeft",
      "Mod+Alt+ArrowRight",
      "Mod+Alt+ArrowUp",
      "Mod+Alt+ArrowDown",
    ],
  };
  if (system[OS].includes(c)) {
    return IS_MAC
      ? t("macOS uses this combination; it may never reach LocDev.")
      : t("The system uses this combination; it may never reach LocDev.");
  }
  return null;
}

// ── State ────────────────────────────────────────────────────────────────────

const KEY = "locdev.shortcuts";
type Overrides = Record<string, string[]>;

function load(): Overrides {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) ?? "{}");
    return raw && typeof raw === "object" ? raw : {};
  } catch {
    return {};
  }
}

type Handler = (e: KeyboardEvent) => void;

interface ShortcutsState {
  /** Current combos of a shortcut (the user's, or the defaults). */
  combos: (id: string) => string[];
  changed: (id: string) => boolean;
  set: (id: string, combos: string[]) => void;
  reset: (id?: string) => void;
  /** Shortcuts sharing `combo`, other than `except`. */
  usedBy: (combo: string, except?: string) => ShortcutDef[];
  register: (id: string, handler: Handler) => () => void;
  /** While capturing a combo, nothing fires. */
  setCapturing: (on: boolean) => void;
}

const Ctx = createContext<ShortcutsState | null>(null);

export function ShortcutsProvider({ children }: { children: ReactNode }) {
  const [overrides, setOverrides] = useState<Overrides>(load);
  const handlers = useRef(new Map<string, Handler[]>());
  const capturing = useRef(false);

  // Another window changed them: follow.
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === KEY) setOverrides(load());
    };
    window.addEventListener("storage", onStorage);
    return () => window.removeEventListener("storage", onStorage);
  }, []);

  const save = useCallback((next: Overrides) => {
    setOverrides(next);
    try {
      localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      // No storage: the change lasts for this session.
    }
  }, []);

  const combos = useCallback(
    (id: string) => (overrides[id] ?? BY_ID.get(id)?.defaults ?? []).map(normalize),
    [overrides],
  );

  // combo → shortcut ids, for the listener and conflict checks.
  const index = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const s of SHORTCUTS) for (const c of combos(s.id)) map.set(c, [...(map.get(c) ?? []), s.id]);
    return map;
  }, [combos]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (capturing.current || e.defaultPrevented) return;
      const combo = comboOf(e);
      if (!combo) return;
      const ids = index.get(normalize(combo));
      if (!ids) return;
      // In a text field only combos with Ctrl/⌘ fire; Alt+arrows and plain
      // keys keep their editing meaning.
      const typing = (e.target as HTMLElement | null)?.closest?.("input, textarea, select, [contenteditable]");
      if (typing && !/^(Mod|Ctrl)\+/.test(normalize(combo))) return;
      for (const id of ids) {
        const stack = handlers.current.get(id);
        const run = stack?.[stack.length - 1];
        if (run) {
          e.preventDefault();
          run(e);
          return;
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [index]);

  const value = useMemo<ShortcutsState>(
    () => ({
      combos,
      changed: (id) => id in overrides,
      set: (id, list) => {
        const def = BY_ID.get(id);
        const normal = [...new Set(list.map(normalize))];
        const same =
          def && normal.length === def.defaults.length && def.defaults.every((d) => normal.includes(normalize(d)));
        const next = { ...overrides };
        if (same) delete next[id];
        else next[id] = normal;
        save(next);
      },
      reset: (id) => {
        if (!id) return save({});
        const next = { ...overrides };
        delete next[id];
        save(next);
      },
      usedBy: (combo, except) =>
        (index.get(normalize(combo)) ?? []).filter((id) => id !== except).map((id) => BY_ID.get(id)!),
      register: (id, handler) => {
        const stack = handlers.current.get(id) ?? [];
        handlers.current.set(id, [...stack, handler]);
        return () => {
          const now = handlers.current.get(id) ?? [];
          handlers.current.set(
            id,
            now.filter((h) => h !== handler),
          );
        };
      },
      setCapturing: (on) => {
        capturing.current = on;
      },
    }),
    [combos, overrides, index, save],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useShortcuts() {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error("useShortcuts must be used inside <ShortcutsProvider>");
  return ctx;
}

/** Runs `handler` on the shortcut while the calling component is mounted. */
export function useShortcut(id: string, handler: Handler, enabled = true) {
  const { register } = useShortcuts();
  const latest = useRef(handler);
  latest.current = handler;
  useEffect(() => {
    if (!enabled) return;
    return register(id, (e) => latest.current(e));
  }, [id, enabled, register]);
}

/** Several at once: `{ id: handler }`, a missing or undefined handler is
 * off. Handlers may change every render; the registration doesn't. */
export function useShortcutMap(map: Record<string, Handler | false | undefined>) {
  const { register } = useShortcuts();
  const latest = useRef(map);
  latest.current = map;
  const active = Object.keys(map)
    .filter((id) => map[id])
    .sort()
    .join(" ");
  useEffect(() => {
    const offs = active
      .split(" ")
      .filter(Boolean)
      .map((id) =>
        register(id, (e) => {
          const run = latest.current[id];
          if (run) run(e);
        }),
      );
    return () => offs.forEach((off) => off());
  }, [active, register]);
}

/** The first combo of a shortcut, formatted ("Ctrl+K", "⌘K"); null when unset. */
export function useShortcutLabel(id: string): string | null {
  const { combos } = useShortcuts();
  const first = combos(id)[0];
  return first ? formatCombo(first) : null;
}

/** A shortcut's keys as text, or nothing when it has none. */
export function ShortcutText({ id }: { id: string }) {
  const label = useShortcutLabel(id);
  return label ? <>{label}</> : null;
}

/** "Add site · Ctrl+N" (just the label when the shortcut is unset). */
export const withKeys = (label: string, keys: string | null) => (keys ? `${label} · ${keys}` : label);
