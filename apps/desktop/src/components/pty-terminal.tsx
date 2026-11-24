import "@xterm/xterm/css/xterm.css";

import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { useEffect, useRef } from "react";

import { backend } from "@/lib/daemon";
import type { TerminalInfo } from "@/lib/protocol";
import { cn } from "@/lib/utils";

/** The app's colors, read from the CSS variables so both themes match. */
function theme() {
  const css = getComputedStyle(document.documentElement);
  const v = (name: string) => css.getPropertyValue(name).trim();
  return {
    background: v("--well"),
    foreground: v("--foreground"),
    cursor: v("--ember"),
    cursorAccent: v("--well"),
    selectionBackground: "rgba(214, 112, 52, 0.3)",
  };
}

/**
 * A terminal the daemon runs (project creation, a tool install): its output
 * as it comes, and what you type goes to the program (answers to prompts).
 * Polls `terminal_read` from the last offset; stops polling once it ended.
 */
export function PtyTerminal({
  id,
  onInfo,
  className,
}: {
  id: number;
  onInfo?: (info: TerminalInfo) => void;
  className?: string;
}) {
  const host = useRef<HTMLDivElement>(null);
  const latest = useRef(onInfo);
  latest.current = onInfo;

  useEffect(() => {
    const term = new Terminal({
      fontFamily: getComputedStyle(document.documentElement).getPropertyValue("--font-mono") || "monospace",
      fontSize: 12.5,
      lineHeight: 1.25,
      cursorBlink: true,
      convertEol: false,
      scrollback: 5000,
      theme: theme(),
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(host.current!);
    const resize = () => {
      try {
        fit.fit();
        void backend.request({ type: "terminal_resize", id, cols: term.cols, rows: term.rows }).catch(() => {});
      } catch {
        // Not laid out yet.
      }
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(host.current!);
    const input = term.onData((data) => {
      void backend.request({ type: "terminal_write", id, data }).catch(() => {});
    });
    // The theme follows the app's.
    const themeObserver = new MutationObserver(() => {
      term.options.theme = theme();
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });

    let from = 0;
    let stopped = false;
    let timer = 0;
    const poll = async () => {
      try {
        const out = await backend.request({ type: "terminal_read", id, from });
        if (out.type === "terminal_output") {
          if (out.truncated) term.write("\x1b[2m… (earlier output dropped)\x1b[0m\r\n");
          if (out.data) term.write(out.data);
          from = out.next;
          latest.current?.(out.info);
          if (!out.info.running) return;
        }
      } catch {
        // The daemon may be restarting; keep trying.
      }
      if (!stopped) timer = window.setTimeout(poll, 200);
    };
    void poll();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
      observer.disconnect();
      themeObserver.disconnect();
      input.dispose();
      term.dispose();
    };
  }, [id]);

  return (
    <div
      data-selectable
      className={cn("min-h-0 overflow-hidden rounded-md border bg-well p-2 [&_.xterm]:h-full", className)}
    >
      <div ref={host} className="h-full" />
    </div>
  );
}
