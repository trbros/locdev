import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Search, Square, X } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";

import { useStatus } from "@/hooks/use-daemon";
import { subById, viewById } from "@/lib/nav";
import { ShortcutText } from "@/lib/shortcuts";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

/** macOS keeps its own traffic lights (overlay title bar, set in the shell). */
const MAC = typeof navigator !== "undefined" && /Mac/.test(navigator.platform || navigator.userAgent);

/**
 * The window's top edge, drawn by the app instead of the system: drag to
 * move, double-click to maximize, and the window buttons. In the mock (a
 * plain browser) the buttons are drawn but do nothing, so screenshots match.
 */
export function TitleBar() {
  const { view, sub, selected, setPaletteOpen } = useUi();
  const status = useStatus();
  const def = viewById(view);
  const where = view === "sites" ? selected : sub ? subById(def, sub)?.label.toLowerCase() : null;
  const up = !!status.data;

  return (
    <header
      data-tauri-drag-region
      className={cn(
        "relative flex h-9 shrink-0 select-none items-center border-b bg-chrome pr-0 font-mono text-[11px] text-muted-foreground",
        MAC ? "pl-20" : "pl-3.5",
      )}
    >
      <div data-tauri-drag-region className="flex min-w-0 items-center gap-2">
        <span
          aria-hidden
          className={cn(
            "size-1.5 shrink-0 rounded-full",
            up ? "bg-phosphor shadow-[0_0_6px_var(--phosphor)]" : "bg-muted-foreground/50",
          )}
        />
        <span data-tauri-drag-region className="text-foreground/85">
          {t("locdev")}
        </span>
        <span data-tauri-drag-region aria-hidden className="text-muted-foreground/60">
          ›
        </span>
        <span data-tauri-drag-region className="truncate">
          {def.label.toLowerCase()}
          {where && <span className="text-ember"> · {where}</span>}
        </span>
      </div>

      <button
        type="button"
        onClick={() => setPaletteOpen(true)}
        className="absolute left-1/2 hidden h-6 w-72 -translate-x-1/2 items-center gap-2 rounded-md border bg-well px-2.5 text-left text-[11px] text-muted-foreground transition-colors hover:border-ember/50 hover:text-foreground md:flex"
      >
        <Search className="size-3.5" />
        <span className="flex-1 truncate font-sans">{t("Search sites, pages and commands")}</span>
        <kbd className="text-[10px]">
          <ShortcutText id="palette" />
        </kbd>
      </button>

      <div data-tauri-drag-region className="h-full flex-1" />
      {!MAC && <WindowButtons />}
    </header>
  );
}

function WindowButtons() {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    if (!isTauri()) return;
    const win = getCurrentWindow();
    const sync = () => win.isMaximized().then(setMaximized, () => {});
    sync();
    const off = win.onResized(sync);
    return () => {
      off.then((fn) => fn());
    };
  }, []);
  const act = (fn: (w: ReturnType<typeof getCurrentWindow>) => Promise<void>) => () => {
    if (isTauri()) fn(getCurrentWindow()).catch(() => {});
  };
  return (
    <div className="flex h-full">
      <WindowButton label={t("Minimize")} onClick={act((w) => w.minimize())}>
        <Minus className="size-3.5" />
      </WindowButton>
      <WindowButton label={maximized ? t("Restore") : t("Maximize")} onClick={act((w) => w.toggleMaximize())}>
        {maximized ? <Copy className="size-3 -scale-x-100" /> : <Square className="size-3" />}
      </WindowButton>
      {/* Closing hides the window; LocDev stays in the tray (see the shell). */}
      <WindowButton label={t("Close")} onClick={act((w) => w.close())} danger>
        <X className="size-4" />
      </WindowButton>
    </div>
  );
}

function WindowButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={cn(
        "flex h-full w-11 items-center justify-center text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ember focus-visible:ring-inset",
        danger ? "hover:bg-ember hover:text-ember-foreground" : "hover:bg-foreground/8 hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
