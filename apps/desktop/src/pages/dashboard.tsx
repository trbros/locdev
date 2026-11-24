import {
  AlertTriangle,
  ArrowUpCircle,
  ExternalLink,
  FolderPlus,
  type LucideIcon,
  Play,
  RotateCw,
  ShieldAlert,
  Sparkles,
  Square,
  Stethoscope,
  XCircle,
} from "lucide-react";
import type { ReactNode } from "react";

import { Page, Section } from "@/components/page";
import { Led, type Tone, toneText } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import {
  useDaemonControl,
  useDaemonDown,
  useDoctor,
  useJobs,
  useNotifications,
  useProjectAction,
  useProjects,
  useRuntimes,
  useStatus,
} from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import { viewById } from "@/lib/nav";
import { ShortcutText } from "@/lib/shortcuts";
import { JOB_LABEL, JOB_STATUS_LABEL, jobFinished, type ProjectView, RUNTIME_LABEL } from "@/lib/protocol";
import { useOpenNotification } from "@/pages/notifications";
import { openInBrowser } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function DashboardView() {
  const view = viewById("dashboard");
  const down = useDaemonDown();
  const { setAddSiteOpen, navigate } = useUi();

  return (
    <Page
      view={view}
      wide
      actions={
        <>
          <Button size="sm" variant="outline" onClick={() => navigate("tools", "new-project")} disabled={down}>
            <Sparkles data-icon="inline-start" /> {t("New project")}
          </Button>
          <Button
            size="sm"
            onClick={() => setAddSiteOpen(true)}
            disabled={down}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <FolderPlus data-icon="inline-start" /> {t("Add site")}
            <span className="ml-1 font-mono text-[10px] opacity-70">
              <ShortcutText id="add-site" />
            </span>
          </Button>
        </>
      }
    >
      {down ? <Offline /> : <Readout />}
      {!down && (
        <div className="grid grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)] gap-8">
          <ActiveSites />
          <Alerts />
        </div>
      )}
      {!down && <RecentActivity />}
    </Page>
  );
}

function Offline() {
  const { start } = useDaemonControl();
  return (
    <div className="dots flex flex-col items-start gap-3 rounded-xl border p-6">
      <div className="flex items-center gap-2 font-mono text-[12px] text-destructive">
        <Led tone="fail" /> {t("locdevd is offline")}
      </div>
      <p className="max-w-lg text-sm text-muted-foreground">
        {t("The daemon serves your sites and keeps their processes alive, even with this window closed.")}
      </p>
      <Button size="sm" onClick={() => start.mutate()} disabled={start.isPending}>
        <Play data-icon="inline-start" className="fill-current" />{" "}
        {start.isPending ? t("Starting…") : t("Start locdevd")}
      </Button>
    </div>
  );
}

/** The environment as one instrument strip, each cell a way into its settings. */
function Readout() {
  const status = useStatus();
  const projects = useProjects();
  const { navigate } = useUi();
  const s = status.data;
  const sites = projects.data ?? [];
  const running = sites.filter((p) => p.process.state === "running").length;

  if (!s) return <Skeleton className="h-18.5" />;

  const cells: { label: string; value: ReactNode; tone: Tone; onClick: () => void }[] = [
    {
      label: t("daemon"),
      value: t("up {time}", { time: uptime(s.started_at) }),
      tone: "live",
      onClick: () => navigate("settings", "daemon"),
    },
    {
      label: t("sites"),
      value: (
        <>
          {running}
          <span className="text-muted-foreground">{t("/{length} running", { length: sites.length })}</span>
        </>
      ),
      tone: running ? "live" : "off",
      onClick: () => navigate("sites"),
    },
    {
      label: t("proxy"),
      value: s.proxy.port ? `:${s.proxy.port}` : t("off"),
      tone: s.proxy.port ? (s.proxy.error ? "warn" : "live") : "fail",
      onClick: () => navigate("settings", "proxy"),
    },
    {
      label: t("dns"),
      value: s.dns.routing === "locdev" ? "*.test" : s.dns.routing === "other" ? t("other tool") : ".localhost",
      tone: s.dns.routing === "locdev" ? "live" : s.dns.routing === "other" ? "warn" : "off",
      onClick: () => navigate("settings", "dns"),
    },
    {
      label: t("https"),
      value: !s.tls.port ? t("off") : s.tls.trusted ? t("trusted") : t("untrusted"),
      tone: !s.tls.port ? "off" : s.tls.trusted ? "live" : "warn",
      onClick: () => navigate("settings", "https"),
    },
  ];

  return (
    <div className="grid grid-cols-5 overflow-hidden rounded-xl border bg-card/40">
      {cells.map((cell) => (
        <button
          type="button"
          key={cell.label}
          onClick={cell.onClick}
          className="flex flex-col gap-1.5 border-r px-4 py-3.5 text-left transition-colors last:border-r-0 hover:bg-foreground/3"
        >
          <span className="flex items-center gap-2 font-mono text-[10.5px] tracking-[0.08em] text-muted-foreground uppercase">
            <Led tone={cell.tone} live={cell.tone === "live"} />
            {cell.label}
          </span>
          <span className="truncate font-mono text-[15px] tabular-nums">{cell.value}</span>
        </button>
      ))}
    </div>
  );
}

function ActiveSites() {
  const projects = useProjects();
  const { openSite, setAddSiteOpen } = useUi();
  const all = projects.data ?? [];
  const active = all.filter((p) => p.process.state === "running" || p.process.state === "stopping");

  return (
    <Section title={t("active environments · {length}", { length: active.length })}>
      {projects.isPending ? (
        <div className="flex flex-col gap-2 pt-1">
          <Skeleton className="h-14" />
          <Skeleton className="h-14" />
        </div>
      ) : !all.length ? (
        <div className="flex flex-col items-start gap-3 py-4">
          <p className="text-sm text-muted-foreground">
            {tx(
              "No sites yet. Point LocDev at a project folder and it picks the dev server, a port and a {test} domain.",
              { test: <span className="font-mono text-foreground">.test</span> },
            )}
          </p>
          <Button size="sm" variant="outline" onClick={() => setAddSiteOpen(true)}>
            <FolderPlus data-icon="inline-start" /> {t("Add a folder")}
          </Button>
        </div>
      ) : !active.length ? (
        <p className="py-3 text-sm text-muted-foreground">
          {tx("Nothing is running. Start a site from {sites}, or mark it to start with LocDev.", {
            sites: (
              <button type="button" onClick={() => openSite(all[0].name)} className="text-ember hover:underline">
                {t("Sites")}
              </button>
            ),
          })}
        </p>
      ) : (
        <ul>
          {active.map((site, i) => (
            <ActiveRow key={site.name} site={site} index={i} />
          ))}
        </ul>
      )}
    </Section>
  );
}

function ActiveRow({ site, index }: { site: ProjectView; index: number }) {
  const { openSite } = useUi();
  const action = useProjectAction();
  const busy = action.isPending || site.process.state === "stopping";
  const runtime = site.runtimes
    .filter((r) => r.version && r.kind !== "npm")
    .map((r) => `${r.kind} ${r.version!.version}`)
    .join(" · ");

  return (
    <li
      className="rise group flex items-center gap-3 border-b border-dashed py-2.5 last:border-b-0"
      style={{ "--i": index } as React.CSSProperties}
    >
      <Led tone="live" live />
      <button type="button" onClick={() => openSite(site.name)} className="min-w-0 flex-1 text-left">
        <div className="truncate font-mono text-[13.5px] hover:text-ember">{site.domain}</div>
        <div className="truncate font-mono text-[11px] text-muted-foreground">
          {site.process.started_at ? t("up {time}", { time: uptime(site.process.started_at) }) : t("starting")}
          {runtime && ` · ${runtime}`}
        </div>
      </button>
      <div className="flex shrink-0 items-center gap-1 opacity-70 transition-opacity group-hover:opacity-100">
        {site.url && (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label={t("Open {domain}", { domain: site.domain })}
            onClick={() => openInBrowser(site.url!)}
          >
            <ExternalLink />
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("Restart {domain}", { domain: site.domain })}
          disabled={busy}
          onClick={() => action.mutate({ action: "restart_project", name: site.name })}
        >
          <RotateCw />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("Stop {domain}", { domain: site.domain })}
          disabled={busy}
          onClick={() => action.mutate({ action: "stop_project", name: site.name })}
        >
          <Square className="fill-current" />
        </Button>
      </div>
    </li>
  );
}

interface Alert {
  id: string;
  tone: Tone;
  icon: LucideIcon;
  title: string;
  detail: string;
  onClick: () => void;
}

/** What needs attention, from real state only: failed sites, Doctor, runtimes. */
function Alerts() {
  const projects = useProjects();
  const doctor = useDoctor();
  const runtimes = useRuntimes();
  const { navigate, openSite } = useUi();

  const alerts: Alert[] = [];
  for (const site of projects.data ?? []) {
    if (site.process.state === "failed") {
      alerts.push({
        id: `failed-${site.name}`,
        tone: "fail",
        icon: XCircle,
        title: t("{domain} stopped with an error", { domain: site.domain }),
        detail: site.process.message ?? t("See its output"),
        onClick: () => openSite(site.name, "server"),
      });
    }
  }
  for (const check of doctor.data ?? []) {
    if (check.level !== "warn" && check.level !== "error") continue;
    alerts.push({
      id: check.id,
      tone: check.level === "error" ? "fail" : "warn",
      icon: check.id.startsWith("runtime") ? ShieldAlert : AlertTriangle,
      title: check.label,
      detail: check.detail,
      onClick: () => navigate("tools", "doctor"),
    });
  }
  for (const info of runtimes.data ?? []) {
    for (const update of info.updates) {
      alerts.push({
        id: `update-${info.kind}-${update.line}`,
        tone: "info",
        icon: ArrowUpCircle,
        title: `${RUNTIME_LABEL[info.kind]} ${update.latest} available`,
        detail: t("You have {installed} ({line}.x)", { installed: update.installed, line: update.line }),
        onClick: () => navigate("runtimes"),
      });
    }
  }

  return (
    <Section
      title={t("alerts · {length}", { length: alerts.length })}
      aside={
        <button
          type="button"
          onClick={() => navigate("tools", "doctor")}
          className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground hover:text-foreground"
        >
          <Stethoscope className="size-3" /> {t("doctor")}
        </button>
      }
    >
      {doctor.isPending ? (
        <div className="flex flex-col gap-2 pt-1">
          <Skeleton className="h-12" />
          <Skeleton className="h-12" />
        </div>
      ) : !alerts.length ? (
        <p className="py-3 text-sm text-muted-foreground">
          <span className="text-phosphor">{t("All clear.")}</span> {t("Nothing needs your attention.")}
        </p>
      ) : (
        <ul>
          {alerts.slice(0, 8).map((alert) => (
            <li key={alert.id}>
              <button
                type="button"
                onClick={alert.onClick}
                className="flex w-full gap-3 border-b border-dashed p-2.5 text-left last:border-b-0 hover:bg-foreground/2"
              >
                <alert.icon className={cn("mt-0.5 size-4 shrink-0", toneText(alert.tone))} strokeWidth={1.75} />
                <span className="min-w-0">
                  <span className="block truncate text-[13.5px]">{alert.title}</span>
                  <span className="line-clamp-2 text-[12.5px] text-muted-foreground">{alert.detail}</span>
                </span>
              </button>
            </li>
          ))}
          {alerts.length > 8 && (
            <li className="pt-2 font-mono text-[11px] text-muted-foreground">
              +{alerts.length - 8} {t("more in Doctor")}
            </li>
          )}
        </ul>
      )}
    </Section>
  );
}

/** What happened lately: notifications and finished jobs, newest first. */
function RecentActivity() {
  const notifications = useNotifications(true);
  const jobs = useJobs();
  const { navigate } = useUi();
  const open = useOpenNotification();

  const items = [
    ...(notifications.data?.notifications ?? []).map((n) => ({
      key: `n${n.id}`,
      at: n.created_at,
      title: n.title,
      detail: n.body.split("\n")[0],
      tone: (n.severity === "error" ? "fail" : n.severity === "warn" ? "warn" : "info") as Tone,
      onClick: () => open(n),
    })),
    ...(jobs.data ?? [])
      .filter((j) => j.automatic && jobFinished(j.status))
      .map((j) => ({
        key: `j${j.id}`,
        at: j.finished_at ?? j.created_at,
        title: `${JOB_LABEL[j.kind]}${j.project ? ` · ${j.project}` : ""} ${JOB_STATUS_LABEL[j.status]}`,
        detail: j.result ?? j.error ?? "",
        tone: (j.status === "completed" ? "live" : j.status === "failed" ? "fail" : "off") as Tone,
        onClick: () => navigate("settings", "automations"),
      })),
  ]
    .sort((a, b) => b.at - a.at)
    .slice(0, 6);

  return (
    <Section
      title={t("recent activity")}
      aside={
        <button
          type="button"
          onClick={() => navigate("notifications", "history")}
          className="font-mono text-[11px] text-muted-foreground hover:text-foreground"
        >
          {t("history →")}
        </button>
      }
    >
      {!items.length ? (
        <p className="py-3 text-sm text-muted-foreground">
          {t("Nothing yet. Dependency checks and results show up here.")}
        </p>
      ) : (
        <ul>
          {items.map((item) => (
            <li key={item.key}>
              <button
                type="button"
                onClick={item.onClick}
                className="flex w-full items-baseline gap-3 border-b border-dashed p-2 text-left last:border-b-0 hover:bg-foreground/2"
              >
                <Led tone={item.tone} />
                <span className="min-w-0 flex-1 truncate text-[13.5px]">
                  {item.title}
                  {item.detail && <span className="text-muted-foreground"> · {item.detail}</span>}
                </span>
                <span className="shrink-0 font-mono text-[11px] text-muted-foreground">
                  {uptime(item.at)} {t("ago")}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
