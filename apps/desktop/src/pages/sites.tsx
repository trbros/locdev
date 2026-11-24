import { FolderPlus, Play, Power } from "lucide-react";
import { useEffect, useState } from "react";

import { NewTabPage, SiteTabStrip } from "@/components/site-browser";
import { SiteDetail } from "@/components/site-detail";
import { SiteList } from "@/components/site-list";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useDaemonControl, useDaemonDown, useProjects } from "@/hooks/use-daemon";
import { copyText } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { ShortcutText } from "@/lib/shortcuts";
import { t } from "@/lib/i18n";

export function SitesView() {
  const projects = useProjects();
  const down = useDaemonDown();
  const { selected, reconcileTabs } = useUi();
  const [filter, setFilter] = useState("");

  const all = projects.data ?? [];
  const needle = filter.trim().toLowerCase();
  const sites = all.filter(
    (p) => !needle || p.domain.includes(needle) || p.path.toLowerCase().includes(needle) || p.kind.includes(needle),
  );

  // Tabs follow the site list: a removed site's tab closes, and the tab
  // LocDev opened on first launch gets a running site.
  const loaded = projects.data;
  useEffect(() => {
    if (!loaded) return;
    const preferred = loaded.find((p) => p.process.state === "running") ?? loaded[0];
    reconcileTabs(
      loaded.map((p) => p.name),
      preferred?.name ?? null,
    );
  }, [loaded, reconcileTabs]);

  const current = all.find((p) => p.name === selected);

  return (
    <div className="flex h-full min-w-0">
      <SiteList sites={sites} filter={filter} onFilter={setFilter} disabled={down} />
      <section className="flex min-w-0 flex-1 flex-col">
        {down ? (
          <Offline />
        ) : projects.isPending ? (
          <div className="flex flex-col gap-4 p-8">
            <Skeleton className="h-6 w-40" />
            <Skeleton className="h-10 w-72" />
            <Skeleton className="h-64" />
          </div>
        ) : !all.length ? (
          <Onboarding />
        ) : (
          <>
            <SiteTabStrip sites={all} />
            <div className="min-h-0 flex-1">
              {current ? <SiteDetail site={current} sites={all} /> : <NewTabPage sites={all} />}
            </div>
          </>
        )}
      </section>
    </div>
  );
}

function EmptyStage({ children }: { children: React.ReactNode }) {
  return (
    <div className="dots flex h-full items-center justify-center p-10">
      <div className="w-full max-w-md rounded-xl border bg-background/90 p-7 shadow-[0_24px_60px_-30px_rgb(0_0_0/0.5)] backdrop-blur-sm">
        {children}
      </div>
    </div>
  );
}

function Onboarding() {
  const { setAddSiteOpen } = useUi();
  const command = "locdev add . --start";

  return (
    <EmptyStage>
      <div className="font-mono text-[11px] text-ember">{t("nothing served yet")}</div>
      <h2 className="mt-2 text-xl font-semibold tracking-[-0.01em]">{t("Point LocDev at a project folder.")}</h2>
      <p className="mt-2 text-sm text-muted-foreground">
        {t("It detects 40+ frameworks, picks a port and serves them at")}{" "}
        <span className="font-mono text-foreground">{t("name.test")}</span>.
      </p>
      <div className="mt-5 flex items-center gap-3">
        <Button onClick={() => setAddSiteOpen(true)} className="bg-ember text-ember-foreground hover:bg-ember/85">
          <FolderPlus data-icon="inline-start" /> {t("Add a folder")}
        </Button>
        <span className="font-mono text-[11px] text-muted-foreground">
          <ShortcutText id="add-site" />
        </span>
      </div>
      <button
        type="button"
        onClick={() => copyText(command)}
        className="mt-5 flex w-full items-center justify-between rounded-md border bg-well px-3 py-2.5 text-left font-mono text-xs transition-colors hover:border-ember/40"
      >
        <span>
          <span className="text-ember">$</span> {command}
        </span>
        <span className="text-muted-foreground">{t("copy")}</span>
      </button>
    </EmptyStage>
  );
}

function Offline() {
  const { start } = useDaemonControl();

  return (
    <EmptyStage>
      <div className="flex items-center gap-2 font-mono text-[11px] text-destructive">
        <Power className="size-3.5" /> {t("locdevd is offline")}
      </div>
      <h2 className="mt-2 text-xl font-semibold tracking-[-0.01em]">{t("The daemon is not running.")}</h2>
      <p className="mt-2 text-sm text-muted-foreground">
        {t("It serves your sites and keeps their processes alive, even with this window closed.")}
      </p>
      <Button
        className="mt-5 bg-ember text-ember-foreground hover:bg-ember/85"
        onClick={() => start.mutate()}
        disabled={start.isPending}
      >
        <Play data-icon="inline-start" className="fill-current" />
        {start.isPending ? t("Starting…") : t("Start locdevd")}
      </Button>
    </EmptyStage>
  );
}
