import {
  Bug,
  ChevronDown,
  ChevronRight,
  Code2,
  Copy,
  Download,
  Filter,
  Layers,
  ListTree,
  Lightbulb,
  Pause,
  Play,
  Search,
  Trash2,
} from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";

import { Led, type Tone } from "@/components/status";
import { useCrumbs, ViewHeading } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { errorMessage, useProjects } from "@/hooks/use-daemon";
import { useDebugActions, useDebugEvent, useDebugEvents, useDebugGroups, useDebugStatus } from "@/hooks/use-debug";
import { useDefaultEditorName, useOpenInEditor } from "@/hooks/use-editors";
import { backend } from "@/lib/daemon";
import { subById, viewById } from "@/lib/nav";
import type {
  DebugEvent,
  DumpEntry,
  DumpNode,
  EventFilter,
  EventGroup,
  EventKind,
  EventSeverity,
  ProjectView,
  StackFrame,
} from "@/lib/protocol";
import { copyText } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const KINDS: { id: EventKind; label: string }[] = [
  { id: "exception", label: t("Exceptions") },
  { id: "dump", label: t("Dumps") },
  { id: "log", label: t("Logs") },
  { id: "http", label: t("HTTP") },
  { id: "process", label: t("Processes") },
  { id: "build", label: t("Builds") },
  { id: "job", label: t("Jobs") },
];

const KIND_LABEL: Record<EventKind, string> = {
  exception: t("exception"),
  dump: t("dump"),
  log: t("log"),
  http: t("http"),
  process: t("process"),
  build: t("build"),
  job: t("job"),
};

const LEVELS: { id: EventSeverity | null; label: string }[] = [
  { id: null, label: t("Every level") },
  { id: "warn", label: t("Warnings and up") },
  { id: "error", label: t("Errors and up") },
];

const SEVERITY_LABEL: Record<EventSeverity, string> = {
  debug: t("debug"),
  info: t("info"),
  warn: t("warning"),
  error: t("error"),
  fatal: t("fatal"),
};

const tone = (s: EventSeverity): Tone =>
  s === "fatal" || s === "error" ? "fail" : s === "warn" ? "warn" : s === "info" ? "info" : "off";

function clock(ms: number) {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function ago(ms: number) {
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000));
  if (s < 60) return t("{n}s ago", { n: s });
  if (s < 3600) return t("{n}m ago", { n: Math.round(s / 60) });
  if (s < 86400) return t("{n}h ago", { n: Math.round(s / 3600) });
  return new Date(ms).toLocaleDateString();
}

const base = (file?: string | null) => file?.split(/[\\/]/).pop() ?? "";

/** A path inside the site, for the editor (or null when it's elsewhere). */
function relative(file: string | null | undefined, site: ProjectView | undefined): string | null {
  if (!file || !site) return null;
  const root = site.path.replace(/[\\/]$/, "");
  return file.startsWith(`${root}/`) || file.startsWith(`${root}\\`) ? file.slice(root.length + 1) : null;
}

/** Text to paste in an issue or a chat. */
function asText(e: DebugEvent): string {
  const lines = [`[${e.severity.toUpperCase()}] ${e.code ? `${e.code}: ` : ""}${e.message}`];
  if (e.file) lines.push(`at ${e.file}${e.line ? `:${e.line}` : ""}`);
  if (e.request?.url) lines.push(`${e.request.method ?? ""} ${e.request.url} ${e.request.status ?? ""}`.trim());
  for (const f of e.stack ?? []) lines.push(`  ${f.function ?? ""} ${f.file ?? ""}${f.line ? `:${f.line}` : ""}`);
  if (e.hint) lines.push(`likely cause: ${e.hint}`);
  return lines.join("\n");
}

export function DebugView() {
  const view = viewById("tools");
  const crumbs = useCrumbs(view, subById(view, "debug")!);
  const projects = useProjects();
  const status = useDebugStatus();
  const { clear } = useDebugActions();
  const [site, setSite] = useState<string | null>(null);
  const [kinds, setKinds] = useState<EventKind[]>([]);
  const [level, setLevel] = useState<EventSeverity | null>(null);
  const [query, setQuery] = useState("");
  const [mode, setMode] = useState<"timeline" | "problems">("timeline");
  const [live, setLive] = useState(true);
  const [fingerprint, setFingerprint] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);

  const filter: EventFilter = useMemo(
    () => ({ site, kinds, min_severity: level, query: query.trim() || null, fingerprint }),
    [site, kinds, level, query, fingerprint],
  );
  const events = useDebugEvents(filter, live);
  const groups = useDebugGroups(filter, live && mode === "problems");
  const list = events.data?.events ?? [];
  const current = selected ?? (mode === "timeline" ? (list[0]?.id ?? null) : (groups.data?.[0]?.last_id ?? null));
  const sites = projects.data ?? [];

  const exportEvents = async () => {
    const text = list
      .slice()
      .reverse()
      .map((e) => JSON.stringify(e))
      .join("\n");
    const path = await backend.saveTextFile(`locdev-events${site ? `-${site}` : ""}.jsonl`, `${text}\n`);
    if (path) copyText(path, t("Saved to {path}", { path }));
  };

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-col gap-3 border-b px-8 pt-5 pb-3">
        <ViewHeading
          view={view}
          title={t("Debug Center")}
          crumbs={crumbs}
          description={t(
            "Exceptions, dumps, error logs, HTTP 5xx and process failures from your sites, with secrets hidden.",
          )}
          actions={
            <span className="flex items-center gap-2">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setLive(!live)}
                title={live ? t("Pause live updates") : t("Resume live updates")}
              >
                {live ? <Pause data-icon="inline-start" /> : <Play data-icon="inline-start" />}
                {live ? t("Live") : t("Paused")}
              </Button>
              <Button size="sm" variant="outline" disabled={list.length === 0} onClick={exportEvents}>
                <Download data-icon="inline-start" /> {t("Export")}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={list.length === 0}
                onClick={() => {
                  if (
                    window.confirm(
                      site ? t("Forget every event from {site}?", { site }) : t("Forget every event from every site?"),
                    )
                  ) {
                    clear.mutate(site);
                    setSelected(null);
                  }
                }}
              >
                <Trash2 data-icon="inline-start" /> {t("Clear")}
              </Button>
            </span>
          }
        />
        <div className="flex flex-wrap items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              aria-label={t("Search events")}
              placeholder={t("message, error type, file, URL…")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Escape" && setQuery("")}
              className="h-7 w-64 pl-7 text-[12.5px]"
            />
          </div>
          <div role="group" aria-label={t("Kinds")} className="flex flex-wrap gap-1">
            {KINDS.map((k) => {
              const on = kinds.includes(k.id);
              return (
                <button
                  type="button"
                  key={k.id}
                  aria-pressed={on}
                  onClick={() => setKinds(on ? kinds.filter((x) => x !== k.id) : [...kinds, k.id])}
                  className={cn(
                    "rounded-full border px-2.5 py-0.5 text-[12px] text-muted-foreground hover:text-foreground",
                    on && "border-ember/60 bg-ember/8 text-foreground",
                  )}
                >
                  {k.label}
                </button>
              );
            })}
          </div>
          <select
            aria-label={t("Level")}
            value={level ?? ""}
            onChange={(e) => setLevel((e.target.value || null) as EventSeverity | null)}
            className="h-7 rounded-md border bg-transparent px-2 text-[12.5px]"
          >
            {LEVELS.map((l) => (
              <option key={l.id ?? "all"} value={l.id ?? ""}>
                {l.label}
              </option>
            ))}
          </select>
          <div role="tablist" aria-label={t("View")} className="ml-auto flex gap-1 rounded-md border p-0.5">
            {(
              [
                ["timeline", t("Timeline"), ListTree],
                ["problems", t("Problems"), Layers],
              ] as const
            ).map(([id, label, Icon]) => (
              <button
                type="button"
                role="tab"
                key={id}
                aria-selected={mode === id}
                onClick={() => {
                  setMode(id);
                  setSelected(null);
                }}
                className={cn(
                  "flex items-center gap-1.5 rounded px-2 py-0.5 text-[12.5px] text-muted-foreground",
                  mode === id && "bg-foreground/[0.07] text-foreground",
                )}
              >
                <Icon className="size-3.5" /> {label}
              </button>
            ))}
          </div>
        </div>
        {fingerprint && (
          <p className="flex items-center gap-2 text-[12.5px]">
            <Filter className="size-3.5 text-ember" />
            {t("Showing one problem's occurrences.")}
            <button type="button" className="text-ember hover:underline" onClick={() => setFingerprint(null)}>
              {t("Show everything")}
            </button>
          </p>
        )}
      </div>
      <div className="flex min-h-0 flex-1">
        <SiteNav sites={sites} site={site} onSite={(s) => (setSite(s), setSelected(null))} status={status.data} />
        <ScrollArea className="w-[26rem] shrink-0 border-r">
          {events.isPending ? (
            <Skeleton className="m-3 h-40" />
          ) : events.isError ? (
            <p className="p-4 text-sm text-destructive">{errorMessage(events.error)}</p>
          ) : mode === "timeline" ? (
            list.length === 0 ? (
              <Empty />
            ) : (
              <ul aria-label={t("Events")} className="divide-y">
                {list.map((e) => (
                  <EventRow key={e.id} e={e} active={e.id === current} onClick={() => setSelected(e.id)} />
                ))}
              </ul>
            )
          ) : (groups.data ?? []).length === 0 ? (
            <Empty />
          ) : (
            <ul aria-label={t("Problems")} className="divide-y">
              {(groups.data ?? []).map((g) => (
                <GroupRow
                  key={g.fingerprint}
                  g={g}
                  active={g.last_id === current}
                  onClick={() => setSelected(g.last_id)}
                />
              ))}
            </ul>
          )}
        </ScrollArea>
        <div className="min-w-0 flex-1">
          {current != null ? (
            <EventPane
              id={current}
              site={sites.find((p) => p.name === list.find((e) => e.id === current)?.site)}
              sites={sites}
              onOnly={(fp) => {
                setFingerprint(fp);
                setMode("timeline");
              }}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

function SiteNav({
  sites,
  site,
  onSite,
  status,
}: {
  sites: ProjectView[];
  site: string | null;
  onSite: (site: string | null) => void;
  status?: ReturnType<typeof useDebugStatus>["data"];
}) {
  const { dumps } = useDebugActions();
  const chosen = sites.find((p) => p.name === site);
  const php = chosen && ["laravel", "php", "wordpress"].includes(chosen.kind);
  return (
    <nav aria-label={t("Sites")} className="flex w-48 shrink-0 flex-col gap-0.5 border-r p-2">
      {[null, ...sites.map((p) => p.name)].map((name) => (
        <button
          type="button"
          key={name ?? "*"}
          onClick={() => onSite(name)}
          className={cn(
            "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[13px] text-muted-foreground hover:bg-foreground/5 hover:text-foreground",
            site === name && "bg-foreground/[0.07] text-foreground",
          )}
        >
          <Bug className="size-3.5 shrink-0" />
          <span className="truncate">{name ?? t("All sites")}</span>
          {name && status?.dump_sites.includes(name) && (
            <span className="ml-auto font-mono text-[10px] text-ember" title={t("Sends its dumps here")}>
              dd
            </span>
          )}
        </button>
      ))}
      {chosen && php && (
        <div className="mt-auto rounded-lg border p-2.5">
          <label className="flex items-center justify-between gap-2 text-[12.5px]">
            {t("Capture dump() and dd()")}
            <Switch
              checked={!!chosen.dumps}
              disabled={dumps.isPending}
              onCheckedChange={(enabled) => dumps.mutate({ site: chosen.name, enabled })}
              className="data-checked:bg-ember"
            />
          </label>
          <p className="mt-1.5 text-[11.5px] leading-snug text-muted-foreground">
            {t(
              "No project file changes: LocDev sets VarDumper's variables in the site's process (and a small prepend so PHP's built-in server passes them). dd() still ends the request. The site restarts.",
            )}
          </p>
        </div>
      )}
      {status && (
        <p className="mt-2 px-1 font-mono text-[10.5px] text-muted-foreground/80">
          {tn("{n} event", "{n} events", status.events)} · {(status.disk_bytes / 1_048_576).toFixed(1)} MB ·{" "}
          {t("{days} days", { days: status.settings.retention_days })}
        </p>
      )}
    </nav>
  );
}

function Empty() {
  return (
    <div className="flex flex-col gap-2 p-5 text-[12.5px] text-muted-foreground">
      <p className="text-sm text-foreground">{t("Nothing here yet.")}</p>
      <p>
        {t(
          "Errors appear as your sites run: exceptions and stack traces in their output, Laravel's log, 5xx answers, crashes and failed builds.",
        )}
      </p>
      <p>
        {tx("For PHP dumps, pick a site on the left and turn on {capture}.", {
          capture: <span className="text-foreground">{t("Capture dump() and dd()")}</span>,
        })}
      </p>
    </div>
  );
}

function EventRow({ e, active, onClick }: { e: DebugEvent; active: boolean; onClick: () => void }) {
  return (
    <li>
      <button
        type="button"
        onClick={onClick}
        className={cn(
          "flex w-full flex-col gap-1 px-3 py-2 text-left hover:bg-foreground/[0.03]",
          active && "bg-foreground/[0.06]",
        )}
      >
        <span className="flex items-center gap-2 text-[11.5px] text-muted-foreground">
          <Led tone={tone(e.severity)} />
          <span className="font-mono text-[10.5px] tracking-wide uppercase">{KIND_LABEL[e.kind]}</span>
          {e.site && <span className="font-mono text-[10.5px] text-ember">{e.site}</span>}
          <span className="ml-auto shrink-0 font-mono text-[10.5px]">{clock(e.at)}</span>
        </span>
        <span className="line-clamp-2 text-[13px] break-words">
          {e.code && e.kind !== "http" && (
            <span className="font-mono text-[12px] text-muted-foreground">{e.code.split("\\").pop()}: </span>
          )}
          {e.message}
        </span>
        {e.file && (
          <span className="truncate font-mono text-[10.5px] text-muted-foreground">
            {base(e.file)}
            {e.line ? `:${e.line}` : ""}
          </span>
        )}
      </button>
    </li>
  );
}

function Spark({ hourly }: { hourly: number[] }) {
  const max = Math.max(1, ...hourly);
  return (
    <span aria-hidden className="flex h-4 items-end gap-px">
      {hourly.map((n, i) => (
        <span
          key={i}
          className={cn("w-1 rounded-sm", n ? "bg-ember/70" : "bg-foreground/10")}
          style={{ height: `${n ? Math.max(20, (n / max) * 100) : 12}%` }}
        />
      ))}
    </span>
  );
}

function GroupRow({ g, active, onClick }: { g: EventGroup; active: boolean; onClick: () => void }) {
  return (
    <li>
      <button
        type="button"
        onClick={onClick}
        className={cn(
          "flex w-full items-start gap-3 px-3 py-2.5 text-left hover:bg-foreground/[0.03]",
          active && "bg-foreground/[0.06]",
        )}
      >
        <span className="w-10 shrink-0 text-right font-mono text-[15px] leading-tight tabular-nums">
          {g.count}
          <span className="block text-[10px] text-muted-foreground">×</span>
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex items-center gap-2 text-[11.5px] text-muted-foreground">
            <Led tone={tone(g.severity)} />
            <span className="font-mono text-[10.5px] uppercase">{KIND_LABEL[g.kind]}</span>
            {g.site && <span className="font-mono text-[10.5px] text-ember">{g.site}</span>}
            <span className="ml-auto shrink-0 text-[10.5px]">{ago(g.last)}</span>
          </span>
          <span className="mt-0.5 line-clamp-2 block text-[13px] break-words">
            {g.code && g.kind !== "http" && (
              <span className="font-mono text-[12px] text-muted-foreground">{g.code.split("\\").pop()}: </span>
            )}
            {g.message}
          </span>
          <span className="mt-1 flex items-center gap-2">
            <Spark hourly={g.hourly} />
            <span className="text-[10.5px] text-muted-foreground">{t("last 24 h")}</span>
          </span>
        </span>
      </button>
    </li>
  );
}

function Section({ title, children, right }: { title: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="mt-5">
      <h3 className="mb-2 flex items-center justify-between font-mono text-[11px] tracking-[0.12em] text-muted-foreground uppercase">
        {title}
        {right}
      </h3>
      {children}
    </section>
  );
}

function EventPane({
  id,
  site,
  sites,
  onOnly,
}: {
  id: number;
  site: ProjectView | undefined;
  sites: ProjectView[];
  onOnly: (fingerprint: string) => void;
}) {
  const event = useDebugEvent(id);
  const editor = useDefaultEditorName();
  const open = useOpenInEditor();
  if (event.isPending) return <Skeleton className="m-6 h-64" />;
  if (event.isError || !event.data) return <p className="p-6 text-sm text-destructive">{errorMessage(event.error)}</p>;
  const e = event.data;
  const owner = site ?? sites.find((p) => p.name === e.site);
  const openAt = (file?: string | null, line?: number | null) => {
    const path = relative(file, owner);
    return path && owner && editor ? () => open.mutate({ site: owner.name, path, line: line ?? undefined }) : undefined;
  };
  const main = openAt(e.file, e.line);

  return (
    <ScrollArea className="h-full">
      <div className="px-7 py-5" data-selectable>
        <div className="flex flex-wrap items-center gap-2 text-[11.5px]">
          <span
            className={cn(
              "rounded-full px-2 py-0.5 font-mono text-[10.5px] uppercase",
              tone(e.severity) === "fail" ? "bg-destructive/12 text-destructive" : "bg-foreground/[0.07]",
            )}
          >
            {SEVERITY_LABEL[e.severity]}
          </span>
          <span className="font-mono text-[10.5px] text-muted-foreground uppercase">{KIND_LABEL[e.kind]}</span>
          {e.code && <span className="font-mono text-[12px] text-muted-foreground">{e.code}</span>}
          <span className="ml-auto flex gap-1">
            {main && (
              <Button size="xs" variant="ghost" onClick={main}>
                <Code2 data-icon="inline-start" /> {t("Open in {editor}", { editor: editor! })}
              </Button>
            )}
            <Button size="xs" variant="ghost" onClick={() => copyText(asText(e))}>
              <Copy data-icon="inline-start" /> {t("Copy")}
            </Button>
            <Button size="xs" variant="ghost" onClick={() => copyText(JSON.stringify(e, null, 2), t("JSON copied"))}>
              {t("JSON")}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              onClick={() => onOnly(e.fingerprint)}
              title={t("Every occurrence of this problem")}
            >
              <Filter data-icon="inline-start" /> {t("Occurrences")}
            </Button>
          </span>
        </div>
        <h2 className="mt-3 font-mono text-[15px] leading-snug font-medium break-words">{e.message}</h2>

        {e.hint && (
          <div className="mt-4 flex gap-2.5 rounded-lg border border-amber/40 bg-amber/6 px-3 py-2.5 text-[13px]">
            <Lightbulb className="mt-0.5 size-4 shrink-0 text-amber" />
            <span>
              <span className="text-muted-foreground">{t("Likely cause")}: </span>
              {e.hint}
            </span>
          </div>
        )}

        <dl className="mt-4 grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5 text-[12.5px]">
          <Meta label={t("When")}>{new Date(e.at).toLocaleString()}</Meta>
          {e.site && <Meta label={t("Site")}>{e.site}</Meta>}
          {e.process && <Meta label={t("Process")}>{e.process}</Meta>}
          {e.framework && <Meta label={t("Framework")}>{e.framework}</Meta>}
          {e.runtime && <Meta label={t("Runtime")}>{e.runtime}</Meta>}
          {e.file && (
            <Meta label={t("Where")}>
              {main ? (
                <button type="button" onClick={main} className="text-left hover:text-ember">
                  {e.file}
                  {e.line ? `:${e.line}` : ""}
                </button>
              ) : (
                `${e.file}${e.line ? `:${e.line}` : ""}`
              )}
            </Meta>
          )}
          {e.request && (
            <Meta label={t("Request")}>
              {[
                e.request.method,
                e.request.url,
                e.request.status,
                e.request.duration_ms != null ? `${e.request.duration_ms} ms` : null,
              ]
                .filter((x) => x != null && x !== "")
                .join(" · ")}
              {e.request.controller && <span className="block text-muted-foreground">{e.request.controller}</span>}
            </Meta>
          )}
        </dl>

        {e.dump && (
          <Section title={t("dumped value")}>
            <div className="rounded-lg border bg-foreground/[0.02] p-3 font-mono text-[12.5px]">
              <DumpTree node={e.dump} depth={0} />
            </div>
          </Section>
        )}

        {!!e.stack?.length && (
          <Section title={t("stack trace")}>
            <StackTrace frames={e.stack} openAt={openAt} />
          </Section>
        )}

        {!!Object.keys(e.context ?? {}).length && (
          <Section title={t("context")}>
            <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1 font-mono text-[12px]">
              {Object.entries(e.context ?? {}).map(([k, v]) => (
                <Meta key={k} label={k}>
                  {v}
                </Meta>
              ))}
            </dl>
          </Section>
        )}

        {e.raw && (
          <Section title={t("as written")}>
            <pre className="max-h-72 overflow-auto rounded-lg border bg-foreground/[0.02] p-3 font-mono text-[11.5px] leading-relaxed whitespace-pre-wrap text-muted-foreground">
              {e.raw}
            </pre>
          </Section>
        )}
      </div>
    </ScrollArea>
  );
}

function Meta({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 font-mono break-all">{children}</dd>
    </>
  );
}

/** Frames, with runs of dependency frames folded. */
function StackTrace({
  frames,
  openAt,
}: {
  frames: StackFrame[];
  openAt: (file?: string | null, line?: number | null) => (() => void) | undefined;
}) {
  const [open, setOpen] = useState<Set<number>>(new Set());
  const runs: { vendor: boolean; frames: { f: StackFrame; i: number }[] }[] = [];
  frames.forEach((f, i) => {
    const last = runs[runs.length - 1];
    if (last && last.vendor && f.vendor) last.frames.push({ f, i });
    else runs.push({ vendor: f.vendor, frames: [{ f, i }] });
  });
  return (
    <ol className="overflow-hidden rounded-lg border font-mono text-[12px]">
      {runs.map((run, r) =>
        run.vendor && run.frames.length > 1 && !open.has(r) ? (
          <li key={r} className="border-b last:border-0">
            <button
              type="button"
              onClick={() => setOpen(new Set(open).add(r))}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[11.5px] text-muted-foreground hover:bg-foreground/[0.03]"
            >
              <ChevronRight className="size-3" />
              {tn("{n} frame in dependencies", "{n} frames in dependencies", run.frames.length)}
            </button>
          </li>
        ) : (
          run.frames.map(({ f, i }) => {
            const go = openAt(f.file, f.line);
            return (
              <li key={i} className={cn("border-b px-3 py-1.5 last:border-0", f.vendor && "text-muted-foreground")}>
                <span className="block truncate">{f.function ?? t("(anonymous)")}</span>
                {f.file && (
                  <button
                    type="button"
                    disabled={!go}
                    onClick={go}
                    className={cn(
                      "block max-w-full truncate text-left text-[11px] text-muted-foreground",
                      go && "hover:text-ember",
                    )}
                  >
                    {f.file}
                    {f.line ? `:${f.line}` : ""}
                    {f.column ? `:${f.column}` : ""}
                  </button>
                )}
              </li>
            );
          })
        ),
      )}
    </ol>
  );
}

const KEY_PREFIX: Record<DumpEntry["kind"], string> = {
  key: "",
  public: "+",
  protected: "#",
  private: "-",
  meta: "~",
  dynamic: "+",
};

function Scalar({ node }: { node: DumpNode }) {
  switch (node.t) {
    case "null":
      return <span className="text-[color:var(--syntax-keyword)]">null</span>;
    case "bool":
      return <span className="text-[color:var(--syntax-keyword)]">{String(node.v)}</span>;
    case "int":
    case "float":
      return <span className="text-[color:var(--syntax-number)]">{node.v}</span>;
    case "str":
      return (
        <span className="text-[color:var(--syntax-string)] break-all whitespace-pre-wrap">
          "{node.v}"{!!node.cut && <span className="text-muted-foreground">…+{node.cut}</span>}
        </span>
      );
    case "text":
      return <span className="text-[color:var(--syntax-type)] whitespace-pre-wrap">{node.v}</span>;
    case "seen":
      return (
        <span className="text-muted-foreground">
          {node.class ?? "object"} {node.handle ? `{#${node.handle}}` : ""} ↻
        </span>
      );
    case "redacted":
      return <span className="rounded bg-foreground/[0.08] px-1 text-[11px] text-muted-foreground">{t("hidden")}</span>;
    case "truncated":
      return <span className="text-muted-foreground">…</span>;
    default:
      return null;
  }
}

function DumpTree({ node, depth }: { node: DumpNode; depth: number }) {
  const [open, setOpen] = useState(depth < 2);
  if (node.t !== "array" && node.t !== "object" && node.t !== "resource") return <Scalar node={node} />;
  const items = node.items ?? [];
  const head =
    node.t === "array" ? (
      <span className="text-muted-foreground">array:{node.count}</span>
    ) : node.t === "object" ? (
      <span>
        <span className="text-[color:var(--syntax-type)]">{node.class}</span>{" "}
        {node.handle != null && <span className="text-muted-foreground">{`{#${node.handle}}`}</span>}
      </span>
    ) : (
      <span className="text-[color:var(--syntax-type)]">{node.kind} resource</span>
    );
  if (!items.length) return head;
  return (
    <span>
      <button type="button" onClick={() => setOpen(!open)} className="inline-flex items-center gap-1 hover:text-ember">
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {head}
      </button>
      {open && (
        <ul className="ml-2 border-l border-foreground/10 pl-3">
          {items.map((entry, i) => (
            <li key={i} className="py-px">
              <span
                className={cn(
                  "text-[color:var(--syntax-property)]",
                  entry.kind === "meta" && "text-[color:var(--syntax-function)]",
                  entry.kind === "private" && "opacity-80",
                )}
                title={entry.kind !== "key" ? entry.kind : undefined}
              >
                {entry.kind === "key" ? JSON.stringify(entry.key) : `${KEY_PREFIX[entry.kind]}${entry.key}`}
              </span>
              <span className="text-muted-foreground">{entry.kind === "key" ? " => " : ": "}</span>
              <DumpTree node={entry.value} depth={depth + 1} />
            </li>
          ))}
          {node.t !== "resource" && !!node.cut && (
            <li className="text-muted-foreground">…{tn("{n} more", "{n} more", node.cut)}</li>
          )}
        </ul>
      )}
    </span>
  );
}
