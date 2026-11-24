import { useQueryClient } from "@tanstack/react-query";
import { Command, Copy, Moon, Plus, RefreshCw, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { type MouseEvent, type ReactNode, useEffect, useState } from "react";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { copyText } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { ShortcutText } from "@/lib/shortcuts";
import { t } from "@/lib/i18n";

const EDITABLE = "input, textarea, select, [contenteditable='true']";

/**
 * Replaces the webview's native context menu (with its "Inspect Element")
 * by the app's own. Specific areas (site rows, the terminal) nest their own
 * menus, which take precedence; this one is the fallback for everything else.
 *
 * Text fields keep the native menu for cut/copy/paste. In development builds,
 * Shift+right-click also falls through to the native menu so the inspector
 * stays reachable.
 */
export function AppContextMenu({ children }: { children: ReactNode }) {
  const { setPaletteOpen, setAddSiteOpen } = useUi();
  const { resolvedTheme, setTheme } = useTheme();
  const queries = useQueryClient();
  // Captured when the menu opens; clicking an item would clear the selection.
  const [selection, setSelection] = useState("");

  // Backstop for places outside this tree (dialogs and popovers are portaled):
  // never let the native menu through, except where `passThrough` allows it.
  useEffect(() => {
    const block = (event: globalThis.MouseEvent) => {
      if (event.defaultPrevented || allowsNativeMenu(event)) return;
      event.preventDefault();
    };
    document.addEventListener("contextmenu", block);
    return () => document.removeEventListener("contextmenu", block);
  }, []);

  const passThrough = (event: MouseEvent) => {
    if (allowsNativeMenu(event.nativeEvent)) {
      // Stop before any menu trigger sees it; the browser default then runs.
      event.stopPropagation();
      return;
    }
    setSelection(window.getSelection()?.toString().trim() ?? "");
  };

  return (
    <div className="contents" onContextMenuCapture={passThrough}>
      <ContextMenu>
        <ContextMenuTrigger className="flex h-full flex-col">{children}</ContextMenuTrigger>
        <ContextMenuContent className="w-56">
          {selection && (
            <>
              <ContextMenuItem onClick={() => copyText(selection)}>
                <Copy /> {t("Copy")}
              </ContextMenuItem>
              <ContextMenuSeparator />
            </>
          )}
          <ContextMenuItem onClick={() => setAddSiteOpen(true)}>
            <Plus /> {t("Add site…")}
            <ContextMenuShortcut>
              <ShortcutText id="add-site" />
            </ContextMenuShortcut>
          </ContextMenuItem>
          <ContextMenuItem onClick={() => setPaletteOpen(true)}>
            <Command /> {t("Command palette")}
            <ContextMenuShortcut>
              <ShortcutText id="palette" />
            </ContextMenuShortcut>
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem onClick={() => queries.invalidateQueries()}>
            <RefreshCw /> {t("Refresh")}
          </ContextMenuItem>
          <ContextMenuItem onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}>
            {resolvedTheme === "dark" ? <Sun /> : <Moon />}
            {resolvedTheme === "dark" ? t("Light theme") : t("Dark theme")}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>
    </div>
  );
}

function allowsNativeMenu(event: globalThis.MouseEvent): boolean {
  const target = event.target instanceof Element ? event.target : null;
  return Boolean(target?.closest(EDITABLE)) || (import.meta.env.DEV && event.shiftKey);
}
