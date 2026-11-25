import { ArrowDownToLine, ExternalLink, RefreshCw, RotateCcw } from "lucide-react";
import { useState } from "react";

import { Fact, Section } from "@/components/page";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useAppInfo, useDaemonControl, useSettings, useStatus } from "@/hooks/use-daemon";
import { useAppInstall, useAppRestart, useAppUpdate, useReleases, useReleasesRefresh } from "@/hooks/use-updates";
import { usePrefs } from "@/lib/prefs";
import { isNewer } from "@/lib/protocol";
import type { AppUpdateProgress, ReleaseInfo } from "@/lib/protocol";
import { copyText, openInBrowser } from "@/lib/system";
import { t } from "@/lib/i18n";

const day = (iso?: string | null) => (iso ? new Date(iso).toLocaleDateString() : null);

const PASSWORD = t("Installing asks for your password: the new package goes in like the one you installed.");

/** Why this copy can't replace itself (codes from the shell). */
const REASON: Record<string, string> = {
  development_build: t("This is a development build; updates are for installed copies."),
  unknown_install: t(
    "This copy wasn't installed from LocDev's package or AppImage: download the new version from the release page.",
  ),
};

function Toggle({
  checked,
  disabled,
  onChange,
  title,
  description,
}: {
  checked: boolean;
  disabled?: boolean;
  onChange: (value: boolean) => void;
  title: string;
  description: string;
}) {
  return (
    <label className="flex items-start gap-3 py-1.5">
      <Switch checked={checked} disabled={disabled} onCheckedChange={onChange} className="mt-0.5" />
      <span>
        <span className="block text-sm">{title}</span>
        <span className="block text-[13px] text-muted-foreground">{description}</span>
      </span>
    </label>
  );
}

function InstallProgress({ progress }: { progress: AppUpdateProgress | null }) {
  const percent = progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
  return (
    <div className="mt-3">
      <div className="flex justify-between text-[13px] text-muted-foreground">
        <span>{progress?.done ? t("Installing…") : t("Downloading and checking the signature…")}</span>
        {percent !== null && !progress?.done && <span className="font-mono tabular-nums">{percent}%</span>}
      </div>
      <Progress value={progress?.done ? null : percent} className="mt-2 **:data-[slot=progress-indicator]:bg-ember" />
    </div>
  );
}

/** The release notes as written (Markdown shown as plain text). */
function Notes({ text }: { text: string }) {
  return (
    <pre
      className="mt-3 max-h-56 overflow-auto rounded-lg border bg-foreground/2 p-3 font-sans text-[13px] leading-relaxed whitespace-pre-wrap text-muted-foreground"
      data-selectable
    >
      {text}
    </pre>
  );
}

function RestartPrompt({ version }: { version: string }) {
  const restart = useAppRestart();
  return (
    <div className="mt-3 rounded-lg border border-ember/40 bg-ember/6 p-3">
      <p className="text-sm">{t("LocDev {version} is installed.", { version })}</p>
      <p className="mt-1 text-[13px] text-muted-foreground">
        {t("It runs after a restart. The daemon restarts with it, so sites stop for a moment.")}
      </p>
      <Button size="sm" className="mt-2.5" disabled={restart.isPending} onClick={() => restart.mutate(true)}>
        <RotateCcw /> {t("Restart LocDev")}
      </Button>
    </div>
  );
}

function AppUpdates() {
  const prefs = usePrefs();
  const info = useAppInfo();
  const update = useAppUpdate();
  const { install, progress } = useAppInstall();
  const offered = update.data;

  return (
    <Section title={t("app")}>
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-sm">
            {t("This app")}: <span className="font-mono">{info.data?.version ?? "—"}</span>
          </p>
          <p className="mt-1 flex items-center gap-2 text-[13px] text-muted-foreground">
            {update.isFetching ? (
              t("Checking…")
            ) : update.isError ? (
              <>
                <Led tone="fail" /> {String((update.error as Error)?.message ?? update.error)}
              </>
            ) : offered?.version ? (
              <>
                <Led tone="warn" />
                {t("LocDev {version} is available", { version: offered.version })}
                {offered.date && ` · ${day(offered.date)}`}
              </>
            ) : (
              <>
                <Led tone="live" /> {t("Up to date")}
              </>
            )}
          </p>
        </div>
        <Button variant="outline" size="sm" disabled={update.isFetching} onClick={() => update.refetch()}>
          <RefreshCw /> {t("Check now")}
        </Button>
      </div>

      {offered?.version && !install.data && (
        <>
          {offered.notes && <Notes text={offered.notes} />}
          {install.isPending ? (
            <InstallProgress progress={progress} />
          ) : (
            <div className="mt-3 flex flex-wrap items-center gap-2">
              <Button size="sm" disabled={!offered.can_install} onClick={() => install.mutate(null)}>
                <ArrowDownToLine /> {t("Install {version}", { version: offered.version })}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => openInBrowser(offered.page)}>
                <ExternalLink /> {t("Release page")}
              </Button>
            </div>
          )}
          {offered.reason && <p className="mt-2 text-[13px] text-muted-foreground">{REASON[offered.reason]}</p>}
          {offered.asks_password && offered.can_install && (
            <p className="mt-2 text-[13px] text-muted-foreground">{PASSWORD}</p>
          )}
        </>
      )}
      {install.data && <RestartPrompt version={install.data} />}

      <div className="mt-4 border-t pt-3">
        <Toggle
          checked={prefs.updateCheck}
          onChange={(v) => prefs.set("updateCheck", v)}
          title={t("Check for updates automatically")}
          description={t("When LocDev opens and once a day, from the releases on GitHub.")}
        />
        <Toggle
          checked={prefs.updateInstall}
          disabled={!prefs.updateCheck}
          onChange={(v) => prefs.set("updateInstall", v)}
          title={t("Install updates automatically")}
          description={t("Downloads and installs a new version as soon as it's found; it runs when you restart.")}
        />
      </div>
      <p className="mt-3 text-[13px] text-muted-foreground">
        {t(
          "Every update is signed by LocDev's release key and checked before it's installed; one that doesn't match is refused. Nothing is installed without your choice above.",
        )}
      </p>
    </Section>
  );
}

function DaemonUpdates() {
  const info = useAppInfo();
  const status = useStatus();
  const releases = useReleases();
  const refresh = useReleasesRefresh();
  const { restart } = useDaemonControl();
  const { query, update } = useSettings();
  const running = status.data?.version;
  const bundled = info.data?.bundled_daemon;
  const latest = releases.data?.latest;
  const install = `curl -fsSL https://github.com/${releases.data?.repo ?? "trbros/locdev"}/releases/latest/download/install.sh | sh`;

  return (
    <Section title={t("daemon and CLI")}>
      <Fact label={t("Running")} mono>
        {running ?? t("not running")}
      </Fact>
      <Fact label={t("Shipped with this app")} mono>
        {bundled ?? "—"}
      </Fact>
      {running && bundled && running !== bundled && (
        <div className="mt-2 rounded-lg border border-ember/40 bg-ember/6 p-3">
          <p className="text-sm">
            {t("The daemon running is {running}; this app ships {bundled}.", { running, bundled })}
          </p>
          <p className="mt-1 text-[13px] text-muted-foreground">
            {t("Restart it to use the one that came with the app (sites stop for a moment).")}
          </p>
          <Button size="sm" className="mt-2.5" disabled={restart.isPending} onClick={() => restart.mutate()}>
            <RotateCcw /> {t("Restart the daemon")}
          </Button>
        </div>
      )}
      <Fact label={t("Latest release")} mono>
        {releases.isPending ? (
          <Skeleton className="h-4 w-24" />
        ) : latest ? (
          <>
            {latest.tag}
            {latest.daemon && ` · ${t("daemon {version}", { version: latest.daemon })}`}
          </>
        ) : (
          "—"
        )}
      </Fact>
      {releases.data?.error && <p className="mt-1 text-[13px] text-destructive">{releases.data.error}</p>}
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" disabled={refresh.isPending} onClick={() => refresh.mutate()}>
          <RefreshCw /> {t("Check now")}
        </Button>
        {releases.data?.checked_at && (
          <span className="text-[13px] text-muted-foreground">
            {t("Checked {time}", { time: new Date(releases.data.checked_at).toLocaleString() })}
          </span>
        )}
      </div>
      <p className="mt-3 text-[13px] text-muted-foreground">
        {t(
          "The app brings its own daemon and CLI, so updating the app updates them. A CLI installed on its own updates with its install script, which checks the download against the release's SHA256SUMS:",
        )}
      </p>
      <button
        type="button"
        onClick={() => copyText(install)}
        className="mt-1.5 w-full truncate rounded-md border bg-foreground/2 px-2.5 py-1.5 text-left font-mono text-xs"
        title={t("Copy")}
      >
        {install}
      </button>
      {query.data && (
        <div className="mt-3 border-t pt-3">
          <Toggle
            checked={query.data.automation.updates}
            onChange={(updates) => update.mutate({ ...query.data, automation: { ...query.data.automation, updates } })}
            title={t("Notify about new releases")}
            description={t("The daemon checks once a day (not when offline) and sends one notification per release.")}
          />
        </div>
      )}
    </Section>
  );
}

function EarlierVersions() {
  const info = useAppInfo();
  const releases = useReleases();
  const [target, setTarget] = useState<ReleaseInfo | null>(null);
  const check = useAppUpdate(target?.tag ?? null, !!target);
  const { install, progress } = useAppInstall();
  const current = info.data?.version;
  const earlier = (releases.data?.releases ?? []).filter(
    (r) => r.updater && current && isNewer(current, r.app ?? r.tag.replace(/^v/, "")),
  );
  if (!earlier.length && !install.data) return null;

  return (
    <Section title={t("earlier versions")}>
      <p className="text-[13px] text-muted-foreground">
        {t(
          "If a new version misbehaves, go back to an earlier release. It's signed and checked like any update; your sites, settings and data stay where they are.",
        )}
      </p>
      <ul className="mt-2 divide-y">
        {earlier.slice(0, 5).map((r) => (
          <li key={r.tag} className="flex items-center justify-between gap-3 py-2">
            <span className="text-sm">
              <span className="font-mono">{r.tag}</span>
              {r.published_at && <span className="text-muted-foreground"> · {day(r.published_at)}</span>}
              {r.prerelease && <span className="text-muted-foreground"> · {t("prerelease")}</span>}
            </span>
            <span className="flex gap-1">
              <Button
                variant="ghost"
                size="sm"
                aria-label={t("Release page")}
                title={t("Release page")}
                onClick={() => openInBrowser(r.url)}
              >
                <ExternalLink />
              </Button>
              <Button variant="outline" size="sm" disabled={install.isPending} onClick={() => setTarget(r)}>
                {t("Go back to this version")}
              </Button>
            </span>
          </li>
        ))}
      </ul>
      {install.data && <RestartPrompt version={install.data} />}

      <Dialog open={!!target && !install.data} onOpenChange={(open) => !open && !install.isPending && setTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("Go back to {tag}?", { tag: target?.tag ?? "" })}</DialogTitle>
            <DialogDescription>
              {t(
                "LocDev {current} is replaced by {version}, with the daemon and CLI it shipped. An older version ignores settings it doesn't know and may drop them when it saves; back up your configuration folder first if in doubt.",
                { current: current ?? "", version: target?.app ?? target?.tag ?? "" },
              )}
            </DialogDescription>
          </DialogHeader>
          {check.isFetching && <p className="text-[13px] text-muted-foreground">{t("Checking…")}</p>}
          {check.isError && <p className="text-[13px] text-destructive">{String(check.error)}</p>}
          {check.data && !check.data.version && (
            <p className="text-[13px] text-destructive">{t("That release has no update for this system.")}</p>
          )}
          {check.data?.reason && <p className="text-[13px] text-muted-foreground">{REASON[check.data.reason]}</p>}
          {check.data?.asks_password && check.data.can_install && (
            <p className="text-[13px] text-muted-foreground">{PASSWORD}</p>
          )}
          {install.isPending && <InstallProgress progress={progress} />}
          <DialogFooter>
            <Button variant="outline" disabled={install.isPending} onClick={() => setTarget(null)}>
              {t("Cancel")}
            </Button>
            <Button
              disabled={!check.data?.version || !check.data.can_install || install.isPending}
              onClick={() => target && install.mutate(target.tag)}
            >
              {t("Install {version}", { version: target?.app ?? target?.tag ?? "" })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Section>
  );
}

export function UpdatesSection() {
  return (
    <>
      <AppUpdates />
      <DaemonUpdates />
      <EarlierVersions />
    </>
  );
}
