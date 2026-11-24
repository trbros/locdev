// Site insights: request metrics, Laravel Tinker, PHP extensions, Git
// worktrees, PHP modules of a runtime and service packages. The daemon does
// the work; these only ask and show.

import { FolderGit2, Play, RotateCcw, SquareTerminal, Trash2 } from "lucide-react";
import { useState } from "react";

import { CodeEditor, CodeView } from "@/components/code-editor";
import { Fact, Section } from "@/components/page";
import { PtyTerminal } from "@/components/pty-terminal";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { errorMessage } from "@/hooks/use-daemon";
import {
  useInstallServicePackage,
  usePhpExtensions,
  usePhpModules,
  useRequestMetrics,
  useResetMetrics,
  useTinkerOpen,
  useTinkerRun,
  useWorktrees,
} from "@/hooks/use-insights";
import { tildify } from "@/lib/format";
import { intlLocale, t, tn } from "@/lib/i18n";
import type { ExtRequirement, GitOp, ProjectView, ServiceInfo, TerminalInfo } from "@/lib/protocol";
import { cn } from "@/lib/utils";

const num = (n: number) => n.toLocaleString(intlLocale);

// ── Request metrics ─────────────────────────────────────────────────────────

function Stat({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return (
    <div className="flex flex-col gap-0.5 rounded-lg border px-3 py-2">
      <span className="font-mono text-[10.5px] tracking-[0.06em] text-muted-foreground uppercase">{label}</span>
      <span className={cn("font-mono text-lg tabular-nums", tone)}>{value}</span>
    </div>
  );
}

export function SiteMetricsTab({ site }: { site: ProjectView }) {
  const metrics = useRequestMetrics(site.name);
  const reset = useResetMetrics(site.name);
  if (metrics.isPending) return <Skeleton className="h-48" />;
  if (metrics.isError) return <p className="text-sm text-destructive">{errorMessage(metrics.error)}</p>;
  const m = metrics.data;
  const peak = Math.max(1, ...m.per_minute.map((b) => b.requests));
  const errors = m.status.server_error + m.upstream_down;

  return (
    <>
      <div className="flex items-center gap-3">
        <p className="min-w-0 flex-1 text-[12.5px] text-muted-foreground">
          {m.since
            ? t("Since {time}, through LocDev's proxy. Requests straight to 127.0.0.1 aren't counted.", {
                time: new Date(m.since).toLocaleTimeString(intlLocale),
              })
            : t("No requests through LocDev's proxy yet. Open the site at its .test address.")}
        </p>
        <Button size="sm" variant="outline" disabled={reset.isPending || m.total === 0} onClick={() => reset.mutate()}>
          <RotateCcw data-icon="inline-start" /> {t("Reset")}
        </Button>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <Stat label={t("requests")} value={num(m.total)} />
        <Stat label={t("errors (5xx)")} value={num(errors)} tone={errors ? "text-destructive" : undefined} />
        <Stat label="p50" value={`${num(m.latency.p50_ms)} ms`} />
        <Stat
          label="p95"
          value={`${num(m.latency.p95_ms)} ms`}
          tone={m.latency.p95_ms > 500 ? "text-amber" : undefined}
        />
      </div>

      <Section title={t("last hour")}>
        {m.per_minute.length === 0 ? (
          <p className="text-[12.5px] text-muted-foreground">{t("Nothing yet.")}</p>
        ) : (
          <div
            role="img"
            aria-label={t("{n} requests in the last hour", {
              n: num(m.per_minute.reduce((s, b) => s + b.requests, 0)),
            })}
            className="flex h-24 items-end gap-px"
          >
            {m.per_minute.map((b) => (
              <div
                key={b.minute}
                title={`${new Date(b.minute).toLocaleTimeString(intlLocale, { hour: "2-digit", minute: "2-digit" })} · ${b.requests}`}
                className="flex min-w-1 flex-1 flex-col justify-end"
                style={{ height: "100%" }}
              >
                <div className="bg-destructive/80" style={{ height: `${(b.errors / peak) * 100}%` }} />
                <div className="bg-ember/70" style={{ height: `${((b.requests - b.errors) / peak) * 100}%` }} />
              </div>
            ))}
          </div>
        )}
      </Section>

      <div className="grid gap-6 lg:grid-cols-2">
        <Section title={t("status codes")}>
          <Fact label="2xx">{num(m.status.success)}</Fact>
          <Fact label="3xx">{num(m.status.redirect)}</Fact>
          <Fact label="4xx">{num(m.status.client_error)}</Fact>
          <Fact label="5xx">{num(m.status.server_error)}</Fact>
          <Fact label={t("server didn't answer")}>{num(m.upstream_down)}</Fact>
        </Section>
        <Section title={t("latency (time to headers)")}>
          <Fact label={t("average")} mono>{`${num(m.latency.avg_ms)} ms`}</Fact>
          <Fact label="p95" mono>{`${num(m.latency.p95_ms)} ms`}</Fact>
          <Fact label="p99" mono>{`${num(m.latency.p99_ms)} ms`}</Fact>
          <Fact label={t("slowest")} mono>{`${num(m.latency.max_ms)} ms`}</Fact>
          <Fact label={t("from the last")} mono>
            {tn("{n} request", "{n} requests", m.latency.samples)}
          </Fact>
        </Section>
      </div>

      {m.top_paths.length > 0 && (
        <Section title={t("busiest paths")}>
          <table className="w-full font-mono text-[12px]">
            <thead className="text-left text-[10.5px] text-muted-foreground uppercase">
              <tr>
                <th className="py-1 font-normal">{t("path")}</th>
                <th className="py-1 text-right font-normal">{t("requests")}</th>
                <th className="py-1 text-right font-normal">{t("avg")}</th>
                <th className="py-1 text-right font-normal">{t("errors")}</th>
              </tr>
            </thead>
            <tbody>
              {m.top_paths.map((p) => (
                <tr key={`${p.method} ${p.path}`} className="border-t border-dashed">
                  <td className="max-w-0 truncate py-1" data-selectable>
                    <span className="text-muted-foreground">{p.method}</span> {p.path}
                  </td>
                  <td className="py-1 text-right tabular-nums">{num(p.requests)}</td>
                  <td className="py-1 text-right tabular-nums">{num(p.avg_ms)} ms</td>
                  <td className={cn("py-1 text-right tabular-nums", p.errors && "text-destructive")}>
                    {num(p.errors)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}

      {m.recent.length > 0 && (
        <Section title={t("recent requests")}>
          <ul className="flex flex-col font-mono text-[12px]">
            {m.recent.slice(0, 30).map((r, i) => (
              <li key={i} className="flex gap-3 border-b border-dashed py-1 last:border-b-0">
                <span className="text-muted-foreground tabular-nums">
                  {new Date(r.at).toLocaleTimeString(intlLocale, { hour12: false })}
                </span>
                <span
                  className={cn(
                    "w-8 tabular-nums",
                    r.status >= 500 ? "text-destructive" : r.status >= 400 ? "text-amber" : "text-phosphor",
                  )}
                >
                  {r.status}
                </span>
                <span className="min-w-0 flex-1 truncate" data-selectable>
                  <span className="text-muted-foreground">{r.method}</span> {r.path}
                </span>
                <span className="text-muted-foreground tabular-nums">{num(r.ms)} ms</span>
              </li>
            ))}
          </ul>
        </Section>
      )}
    </>
  );
}

// ── Tinker ──────────────────────────────────────────────────────────────────

const SAMPLE = "User::count();";

export function SiteTinkerTab({ site }: { site: ProjectView }) {
  const [code, setCode] = useState(SAMPLE);
  const run = useTinkerRun(site.name);
  const open = useTinkerOpen(site.name);
  const [terminal, setTerminal] = useState<TerminalInfo | null>(null);
  const go = () => code.trim() && !run.isPending && run.mutate(code);
  const result = run.data;

  return (
    <>
      <p className="text-[12.5px] text-muted-foreground">
        {t(
          "Runs in your Laravel app with the site's PHP and environment (php artisan tinker). Each run starts fresh; the console keeps state between lines.",
        )}
      </p>
      <Section
        title={t("code")}
        aside={
          <div className="flex items-center gap-1.5">
            <Button
              size="xs"
              variant="ghost"
              disabled={open.isPending || terminal?.running}
              onClick={() => open.mutate(undefined, { onSuccess: setTerminal })}
            >
              <SquareTerminal data-icon="inline-start" /> {t("Console")}
            </Button>
            <Button
              size="xs"
              disabled={run.isPending || !code.trim()}
              onClick={go}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
            >
              <Play data-icon="inline-start" /> {run.isPending ? t("Running…") : t("Run")}
            </Button>
          </div>
        }
      >
        <CodeEditor
          value={code}
          onChange={setCode}
          language="php"
          lineNumbers
          onSubmit={go}
          ariaLabel={t("PHP code for Tinker")}
          className="min-h-32 rounded-md border"
        />
        <p className="pt-1 text-[11.5px] text-muted-foreground">{t("Ctrl/⌘+Enter runs it.")}</p>
      </Section>
      {result && (
        <Section
          title={t("output")}
          aside={
            <span className="font-mono text-[10.5px] text-muted-foreground">
              {result.timed_out
                ? t("stopped after 30 s")
                : t("exit {code} · {ms} ms", { code: result.code ?? "?", ms: num(result.duration_ms) })}
              {result.truncated ? ` · ${t("cut at 256 KB")}` : ""}
            </span>
          }
        >
          <CodeView
            value={result.output || " "}
            language="text"
            wrap
            ariaLabel={t("Tinker output")}
            className="rounded-md border"
          />
        </Section>
      )}
      {terminal && (
        <Section
          title={t("console")}
          aside={
            !terminal.running && (
              <Button size="xs" variant="ghost" onClick={() => setTerminal(null)}>
                {t("Close")}
              </Button>
            )
          }
        >
          <PtyTerminal id={terminal.id} onInfo={setTerminal} className="h-72" />
        </Section>
      )}
    </>
  );
}

// ── PHP extensions of a site ────────────────────────────────────────────────

function ExtList({ items, loaded, tone }: { items: ExtRequirement[]; loaded: string[]; tone: "fail" | "warn" }) {
  return (
    <ul className="flex flex-col">
      {items.map((e) => {
        const has = loaded.includes(e.name);
        return (
          <li
            key={e.name}
            className="flex items-baseline gap-2.5 border-b border-dashed py-1.5 text-[12.5px] last:border-b-0"
          >
            <Led tone={has ? "live" : tone} />
            <span className="w-28 shrink-0 font-mono">{e.name}</span>
            <span className="min-w-0 text-muted-foreground">
              {e.from}
              {e.reason ? `: ${e.reason}` : ""}
            </span>
          </li>
        );
      })}
    </ul>
  );
}

export function PhpExtensionsSection({ site }: { site: ProjectView }) {
  const php = site.stack.components.some((c) => c.id === "php") || site.kind === "php";
  const report = usePhpExtensions(site.name, php);
  if (!php) return null;
  if (!report.data) return report.isError ? null : <Skeleton className="h-24" />;
  const r = report.data;
  return (
    <Section
      title={t("php extensions")}
      aside={
        <span className="font-mono text-[10.5px] text-muted-foreground">
          {r.version ? `PHP ${r.version} · ${r.origin ?? ""}` : ""}
        </span>
      }
    >
      {r.error ? (
        <p className="text-[12.5px] text-destructive">{r.error}</p>
      ) : (
        <>
          <p className="pb-1 text-[12.5px] text-muted-foreground">
            {r.missing.length === 0
              ? t("Every required extension is loaded ({n} loaded).", { n: r.loaded.length })
              : tn("{n} required extension is missing.", "{n} required extensions are missing.", r.missing.length)}
          </p>
          {r.missing.length > 0 && <ExtList items={r.missing} loaded={r.loaded} tone="fail" />}
          {r.remedy && (
            <p className="py-2 text-[12px] text-muted-foreground" data-selectable>
              {r.remedy}
            </p>
          )}
          {r.recommended.length > 0 && (
            <>
              <h3 className="pt-3 pb-1 text-[12px] font-medium">{t("Recommended by the project's packages")}</h3>
              <ExtList items={r.recommended} loaded={r.loaded} tone="warn" />
            </>
          )}
        </>
      )}
    </Section>
  );
}

// ── PHP modules of a runtime ────────────────────────────────────────────────

export function PhpModulesPanel({ executable }: { executable: string }) {
  const modules = usePhpModules(executable);
  const [filter, setFilter] = useState("");
  if (modules.isPending) return <Skeleton className="h-32" />;
  if (modules.isError) return <p className="text-[12.5px] text-destructive">{errorMessage(modules.error)}</p>;
  const m = modules.data;
  const q = filter.trim().toLowerCase();
  const shown = m.extensions.filter((e) => e.name.toLowerCase().includes(q));
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Input
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          placeholder={t("Filter extensions")}
          aria-label={t("Filter extensions")}
          className="h-8 max-w-56 font-mono text-[12.5px]"
        />
        <span className="font-mono text-[11px] text-muted-foreground">
          {tn("{n} extension loaded", "{n} extensions loaded", m.extensions.length)}
        </span>
      </div>
      <div className="flex flex-wrap gap-1.5">
        {shown.map((e) => (
          <span
            key={e.name}
            title={e.version ?? undefined}
            className={cn(
              "rounded border px-1.5 py-0.5 font-mono text-[11.5px]",
              e.zend && "border-ember/40 text-ember",
            )}
          >
            {e.name}
          </span>
        ))}
      </div>
      {m.available.length > 0 && (
        <p className="text-[12px] text-muted-foreground" data-selectable>
          {t("In its extension folder but not loaded (enable in php.ini): {list}", { list: m.available.join(", ") })}
        </p>
      )}
      {m.ini && (
        <p className="font-mono text-[11px] text-muted-foreground" data-selectable>
          php.ini · {tildify(m.ini)}
        </p>
      )}
    </div>
  );
}

// ── Git worktrees ───────────────────────────────────────────────────────────

export function Worktrees({
  name,
  run,
  busy,
}: {
  name: string;
  run: (op: GitOp, done?: () => void) => Promise<void>;
  busy: boolean;
}) {
  const trees = useWorktrees(name);
  const [branch, setBranch] = useState("");
  const [create, setCreate] = useState(true);
  const [asSite, setAsSite] = useState(true);
  const [copyEnv, setCopyEnv] = useState(true);
  if (trees.isPending) return <Skeleton className="h-32" />;
  if (trees.isError) return <p className="text-sm text-destructive">{errorMessage(trees.error)}</p>;

  const add = () =>
    branch.trim() &&
    run({ op: "worktree_add", branch: branch.trim(), create, site: asSite, copy_env: copyEnv }, () => {
      setBranch("");
      trees.refetch();
    });

  return (
    <>
      <p className="text-[12.5px] text-muted-foreground">
        {t(
          "A worktree checks a branch out in its own folder, next to this one. As its own site it runs side by side with this one, at its own .test address.",
        )}
      </p>
      <Section title={t("working trees")}>
        <ul className="flex flex-col">
          {trees.data.map((w) => (
            <li
              key={w.path}
              className="flex items-center gap-3 border-b border-dashed py-2 text-[13px] last:border-b-0"
            >
              <FolderGit2 className={cn("size-4 shrink-0", w.main ? "text-ember" : "text-muted-foreground")} />
              <span className="min-w-0 flex-1">
                <span className="block font-mono">{w.branch ?? t("detached HEAD")}</span>
                <span className="block truncate font-mono text-[11.5px] text-muted-foreground" data-selectable>
                  {tildify(w.path)}
                  {w.site ? ` · ${w.site}.test` : ""}
                  {w.prunable ? ` · ${t("folder missing")}` : ""}
                </span>
              </span>
              {w.main ? (
                <span className="font-mono text-[10.5px] text-muted-foreground">{t("main")}</span>
              ) : (
                <Button
                  size="icon-sm"
                  variant="ghost"
                  disabled={busy}
                  aria-label={t("Remove the worktree {path}", { path: w.path })}
                  onClick={() => run({ op: "worktree_remove", path: w.path }, () => trees.refetch())}
                >
                  <Trash2 />
                </Button>
              )}
            </li>
          ))}
        </ul>
        {trees.data.some((w) => w.prunable) && (
          <Button
            size="sm"
            variant="ghost"
            className="mt-1 self-start"
            onClick={() => run({ op: "worktree_prune" }, () => trees.refetch())}
          >
            {t("Forget missing folders")}
          </Button>
        )}
      </Section>
      <Section title={t("new worktree")}>
        <div className="flex flex-col gap-2.5">
          <div className="flex gap-2">
            <Input
              value={branch}
              onChange={(e) => setBranch(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && add()}
              placeholder="feature/login"
              aria-label={t("Branch")}
              className="h-8 max-w-72 font-mono text-[12.5px]"
            />
            <Button size="sm" disabled={busy || !branch.trim()} onClick={add}>
              {t("Create worktree")}
            </Button>
          </div>
          {(
            [
              [create, setCreate, t("New branch (from the current HEAD)")],
              [asSite, setAsSite, t("Register it as its own site")],
              [copyEnv, setCopyEnv, t("Copy the .env files it lacks")],
            ] as const
          ).map(([value, set, label]) => (
            <label key={label} className="flex items-center gap-2 text-[12.5px]">
              <Checkbox checked={value} onCheckedChange={(v) => set(v === true)} />
              {label}
            </label>
          ))}
        </div>
      </Section>
    </>
  );
}

// ── Service versions from the system's package manager ─────────────────────

export function ServicePackages({ service }: { service: ServiceInfo }) {
  const install = useInstallServicePackage();
  const [terminal, setTerminal] = useState<TerminalInfo | null>(null);
  const packages = service.packages ?? [];
  if (packages.length === 0) return null;
  return (
    <div className="flex flex-col gap-1.5 pt-2">
      <span className="font-mono text-[10.5px] tracking-[0.06em] text-muted-foreground uppercase">
        {t("from {manager}", { manager: packages[0].manager })}
      </span>
      <div className="flex flex-wrap gap-1.5">
        {packages.map((p) => (
          <Button
            key={p.id}
            size="xs"
            variant="outline"
            disabled={install.isPending || terminal?.running}
            title={p.commands.map((c) => c.join(" ")).join(" && ")}
            onClick={() => install.mutate({ kind: service.kind, id: p.id }, { onSuccess: setTerminal })}
          >
            {p.version}
          </Button>
        ))}
      </div>
      {terminal && (
        <div className="flex flex-col gap-1">
          <PtyTerminal id={terminal.id} onInfo={setTerminal} className="h-56" />
          {!terminal.running && (
            <Button size="xs" variant="ghost" className="self-start" onClick={() => setTerminal(null)}>
              {t("Close")}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
