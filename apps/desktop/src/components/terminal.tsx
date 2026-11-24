import { ArrowDownToLine, ClipboardCopy, Copy, Eraser, FileText, Pause, Play, Search, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useLogs } from "@/hooks/use-daemon";
import { clock, stripAnsi } from "@/lib/format";
import type { LogLevel, LogLine, LogStream, ProcessRole } from "@/lib/protocol";
import { copyText, revealFolder } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

type StreamFilter = "all" | Exclude<LogStream, "system">;
type LevelFilter = "any" | LogLevel;

const LEVEL_TEXT: Record<LogLevel, string> = {
  error: "text-destructive",
  warn: "text-amber",
  info: "text-signal",
  debug: "text-muted-foreground",
};

/**
 * A process's output, live: stdout, stderr and LocDev's own notes, with
 * filters (stream, level, text), pause, follow and copy. Levels appear only
 * on lines that state one; plain output is never given a guessed level.
 */
export function Terminal({
  name,
  process = "main",
  running,
  logFile,
  label = "output",
  className,
  aux = null,
}: {
  name: string;
  process?: ProcessRole;
  /** An auxiliary process's output instead. */
  aux?: string | null;
  running: boolean;
  /** The file the daemon keeps this output in, for "open log file". */
  logFile?: string | null;
  label?: string;
  className?: string;
}) {
  const [paused, setPaused] = useState(false);
  const lines = useLogs(name, process, !paused, 2000, aux);
  // Keep showing what was there while paused (the hook stops polling).
  const frozen = useRef<LogLine[]>([]);
  if (!paused) frozen.current = lines;
  const source = paused ? frozen.current : lines;

  const scroller = useRef<HTMLDivElement>(null);
  const [follow, setFollow] = useState(true);
  // Lines up to this sequence number are hidden by "clear".
  const [clearedAt, setClearedAt] = useState(0);
  const [stream, setStream] = useState<StreamFilter>("all");
  const [level, setLevel] = useState<LevelFilter>("any");
  const [query, setQuery] = useState("");

  useEffect(() => {
    setClearedAt(0);
    setPaused(false);
  }, [name, process]);

  const needle = query.trim().toLowerCase();
  const visible = useMemo(
    () =>
      source.filter(
        (l) =>
          l.seq > clearedAt &&
          (stream === "all" || l.stream === stream || l.stream === "system") &&
          (level === "any" || l.level === level) &&
          (!needle || stripAnsi(l.text).toLowerCase().includes(needle)),
      ),
    [source, clearedAt, stream, level, needle],
  );
  const filtered = stream !== "all" || level !== "any" || !!needle;
  const counts = useMemo(() => {
    const out: Partial<Record<LogLevel, number>> = {};
    for (const l of source) if (l.level) out[l.level] = (out[l.level] ?? 0) + 1;
    return out;
  }, [source]);

  useEffect(() => {
    if (follow && scroller.current) scroller.current.scrollTop = scroller.current.scrollHeight;
  }, [visible, follow]);

  // Read when the menu opens: clicking an item would clear the selection.
  const [selection, setSelection] = useState("");
  const asText = (all: LogLine[]) =>
    all.map((l) => `${new Date(l.at).toISOString()} ${l.stream.padEnd(6)} ${stripAnsi(l.text)}`).join("\n");
  const copyAll = () => copyText(asText(visible), filtered ? t("Filtered output copied") : t("Output copied"));
  const clear = () => setClearedAt(source.at(-1)?.seq ?? 0);

  const onScroll = () => {
    const el = scroller.current;
    if (el) setFollow(el.scrollHeight - el.scrollTop - el.clientHeight < 24);
  };

  return (
    <ContextMenu onOpenChange={(open) => open && setSelection(window.getSelection()?.toString() ?? "")}>
      <ContextMenuTrigger
        className={cn(
          "flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border bg-well shadow-[inset_0_1px_0_color-mix(in_oklch,var(--foreground)_4%,transparent)]",
          className,
        )}
      >
        <div className="flex h-9 shrink-0 items-center gap-2 overflow-x-auto border-b px-3 font-mono text-[11px] whitespace-nowrap text-muted-foreground scrollbar-none">
          <span className="text-foreground/80">{label}</span>
          <span className="tabular-nums">
            {visible.length}
            {filtered && `/${source.length}`} {t("lines")}
          </span>
          {paused && <span className="rounded border border-amber/40 px-1 text-amber">{t("paused")}</span>}

          <div className="ml-auto flex shrink-0 items-center gap-1">
            <Segmented
              value={stream}
              onChange={(v) => setStream(v as StreamFilter)}
              items={[
                ["all", t("all")],
                ["stdout", "out"],
                ["stderr", "err"],
              ]}
            />
            <Segmented
              value={level}
              onChange={(v) => setLevel(v as LevelFilter)}
              items={[
                ["any", t("any level")],
                ["error", `${t("error")}${counts.error ? ` ${counts.error}` : ""}`],
                ["warn", `${t("warn")}${counts.warn ? ` ${counts.warn}` : ""}`],
                ["info", "info"],
                ["debug", "debug"],
              ]}
            />
            <label className="flex h-6 items-center gap-1 rounded border bg-background/50 px-1.5 focus-within:border-ember/50">
              <Search className="size-3" />
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Escape" && setQuery("")}
                placeholder={t("search")}
                aria-label={t("Search the output")}
                className="w-24 bg-transparent text-foreground outline-none placeholder:text-muted-foreground"
              />
              {query && (
                <button type="button" aria-label={t("Clear search")} onClick={() => setQuery("")}>
                  <X className="size-3" />
                </button>
              )}
            </label>
            {!follow && (
              <Button variant="ghost" size="xs" onClick={() => setFollow(true)}>
                <ArrowDownToLine data-icon="inline-start" /> {t("follow")}
              </Button>
            )}
            <IconButton label={paused ? t("Resume") : t("Pause")} onClick={() => setPaused(!paused)}>
              {paused ? <Play /> : <Pause />}
            </IconButton>
            <IconButton label={t("Clear view")} disabled={!visible.length} onClick={clear}>
              <Eraser />
            </IconButton>
            <IconButton label={t("Copy output")} disabled={!visible.length} onClick={copyAll}>
              <Copy />
            </IconButton>
            {logFile && (
              <IconButton label={t("Show the log file")} onClick={() => revealFolder(logFile)}>
                <FileText />
              </IconButton>
            )}
          </div>
        </div>

        <div
          ref={scroller}
          onScroll={onScroll}
          data-selectable
          role="log"
          aria-live="off"
          className="min-h-0 flex-1 overflow-auto px-3 py-2.5 font-mono text-[12px] leading-[1.65]"
        >
          {visible.map((line) => (
            <div key={line.seq} className="group flex gap-3">
              <span className="w-18 shrink-0 text-muted-foreground/45 tabular-nums group-hover:text-muted-foreground">
                {clock(line.at)}
              </span>
              <span
                className={cn(
                  "min-w-0 break-all whitespace-pre-wrap",
                  line.stream === "stderr" && !line.level && "text-amber/90",
                  line.level && LEVEL_TEXT[line.level],
                  line.stream === "system" && "text-muted-foreground italic",
                )}
              >
                {/* LocDev's own notes: an ember marker, then dimmed text. */}
                {line.stream === "system" && (
                  <span className="text-ember not-italic">{line.text.startsWith("$ ") ? "" : "── "}</span>
                )}
                {line.stream === "system" && line.text.startsWith("$ ") ? (
                  <span className="text-foreground not-italic">
                    <span className="text-ember">$</span>
                    {line.text.slice(1)}
                  </span>
                ) : (
                  <Highlight text={stripAnsi(line.text) || " "} needle={needle} />
                )}
              </span>
            </div>
          ))}

          {/* Prompt line: a live cursor while the process runs. */}
          <div className="flex gap-3 pt-0.5">
            <span className="w-18 shrink-0" />
            <span className="text-muted-foreground">
              {filtered && !visible.length ? (
                <span>{t("nothing matches the filters")}</span>
              ) : running ? (
                <span className="inline-block h-[1.1em] w-[0.55em] translate-y-[0.15em] animate-pulse bg-ember/80" />
              ) : (
                <span>
                  <span className="text-ember">$</span> {source.length ? t("process not running") : t("no output yet")}
                </span>
              )}
            </span>
          </div>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent className="w-52">
        <ContextMenuItem disabled={!selection.trim()} onClick={() => copyText(selection, t("Selection copied"))}>
          <ClipboardCopy /> {t("Copy selection")}
        </ContextMenuItem>
        <ContextMenuItem disabled={!visible.length} onClick={copyAll}>
          <Copy /> {filtered ? t("Copy filtered output") : t("Copy all output")}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={() => setPaused(!paused)}>
          {paused ? <Play /> : <Pause />} {paused ? t("Resume") : t("Pause")}
        </ContextMenuItem>
        <ContextMenuItem disabled={!visible.length} onClick={clear}>
          <Eraser /> {t("Clear view")}
        </ContextMenuItem>
        <ContextMenuItem disabled={follow} onClick={() => setFollow(true)}>
          <ArrowDownToLine /> {t("Follow output")}
        </ContextMenuItem>
        {logFile && (
          <ContextMenuItem onClick={() => revealFolder(logFile)}>
            <FileText /> {t("Show the log file")}
          </ContextMenuItem>
        )}
      </ContextMenuContent>
    </ContextMenu>
  );
}

function Highlight({ text, needle }: { text: string; needle: string }) {
  if (!needle) return <>{text}</>;
  const at = text.toLowerCase().indexOf(needle);
  if (at < 0) return <>{text}</>;
  return (
    <>
      {text.slice(0, at)}
      <mark className="rounded-sm bg-ember/30 text-inherit">{text.slice(at, at + needle.length)}</mark>
      <Highlight text={text.slice(at + needle.length)} needle={needle} />
    </>
  );
}

function Segmented({
  value,
  onChange,
  items,
}: {
  value: string;
  onChange: (value: string) => void;
  items: [string, string][];
}) {
  return (
    <div role="radiogroup" className="flex h-6 items-center rounded border bg-background/50 p-px">
      {items.map(([id, label]) => (
        <button
          key={id}
          type="button"
          role="radio"
          aria-checked={value === id}
          onClick={() => onChange(id)}
          className={cn(
            "h-full rounded-[3px] px-1.5 whitespace-nowrap transition-colors hover:text-foreground",
            value === id && "bg-foreground/8 text-foreground",
          )}
        >
          {label}
        </button>
      ))}
    </div>
  );
}

function IconButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Button variant="ghost" size="icon-xs" aria-label={label} title={label} disabled={disabled} onClick={onClick}>
      {children}
    </Button>
  );
}
