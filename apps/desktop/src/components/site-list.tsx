import { Plus, Search } from "lucide-react";
import { useRef } from "react";

import { wantsNewTab } from "@/components/site-browser";
import { SiteContextMenu } from "@/components/site-menu";
import { Led, stateTone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Kbd } from "@/components/ui/kbd";
import { viewById } from "@/lib/nav";
import { useShortcut, useShortcutLabel, withKeys } from "@/lib/shortcuts";
import type { ProjectKind, ProjectView } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const KIND_TAG: Record<ProjectKind, string> = {
  laravel: "laravel",
  wordpress: "wp",
  php: "php",
  next: "next",
  nuxt: "nuxt",
  astro: "astro",
  sveltekit: "kit",
  vite: "vite",
  node: "node",
  static: "static",
  unknown: "—",
};

export function SiteList({
  sites,
  filter,
  onFilter,
  disabled,
}: {
  sites: ProjectView[];
  filter: string;
  onFilter: (value: string) => void;
  disabled?: boolean;
}) {
  const { setAddSiteOpen } = useUi();
  const filterRef = useRef<HTMLInputElement>(null);
  const view = viewById("sites");

  useShortcut("filter-sites", () => filterRef.current?.focus());
  const addKeys = useShortcutLabel("add-site");
  const filterKeys = useShortcutLabel("filter-sites");

  const running = sites.filter((s) => s.process.state === "running" || s.process.state === "stopping");
  const idle = sites.filter((s) => !running.includes(s));

  return (
    <aside className="flex w-72.5 shrink-0 flex-col border-r bg-well/50">
      <div className="px-4 pt-6 pb-3">
        <div className="flex items-start justify-between">
          <div>
            <div className="font-mono text-[11px] text-ember">{view.path}</div>
            <h1 className="mt-1 flex items-baseline gap-2 text-[22px] leading-none font-semibold tracking-[-0.02em]">
              {t("Sites")}
              <span className="font-mono text-xs font-normal text-muted-foreground tabular-nums">{sites.length}</span>
            </h1>
          </div>
          <Button
            size="icon-sm"
            onClick={() => setAddSiteOpen(true)}
            disabled={disabled}
            aria-label={t("Add site")}
            title={withKeys(t("Add site"), addKeys)}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <Plus />
          </Button>
        </div>

        <label className="mt-4 flex h-8 items-center gap-2 rounded-md border bg-background/60 px-2.5 text-sm focus-within:border-ember/50">
          <Search className="size-3.5 shrink-0 text-muted-foreground" />
          <input
            ref={filterRef}
            value={filter}
            onChange={(e) => onFilter(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && onFilter("")}
            placeholder={t("Filter")}
            className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted-foreground"
          />
          {filterKeys && <Kbd className="text-[10px]">{filterKeys}</Kbd>}
        </label>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        <Group label={t("running")} sites={running} offset={0} />
        <Group label={t("idle")} sites={idle} offset={running.length} />
        {!sites.length && filter && (
          <p className="px-3 py-6 text-center text-sm text-muted-foreground">
            {t("No site matches “{filter}”.", { filter })}
          </p>
        )}
      </div>
    </aside>
  );
}

function Group({ label, sites, offset }: { label: string; sites: ProjectView[]; offset: number }) {
  if (!sites.length) return null;

  return (
    <div className="mt-2">
      <div className="flex items-center gap-2 px-2.5 pt-2 pb-1 font-mono text-[10.5px] text-muted-foreground">
        {label}
        <span className="h-px flex-1 bg-border" />
        <span className="tabular-nums">{sites.length}</span>
      </div>
      <ul>
        {sites.map((site, i) => (
          <SiteRow key={site.name} site={site} index={offset + i} />
        ))}
      </ul>
    </div>
  );
}

function SiteRow({ site, index }: { site: ProjectView; index: number }) {
  const { selected, select, openInNewTab } = useUi();
  const active = selected === site.name;
  const state = site.process.state;

  return (
    <li className="rise" style={{ "--i": index } as React.CSSProperties}>
      <SiteContextMenu site={site}>
        <button
          type="button"
          onClick={(e) => (wantsNewTab(e) ? openInNewTab(site.name) : select(site.name))}
          // Middle click: a new tab (and no paste or autoscroll on Linux).
          onMouseDown={(e) => e.button === 1 && e.preventDefault()}
          onAuxClick={(e) => e.button === 1 && openInNewTab(site.name)}
          aria-current={active ? "true" : undefined}
          className={cn(
            "group relative flex h-11 w-full items-center gap-3 rounded-md px-2.5 text-left transition-colors",
            active ? "bg-card shadow-[0_1px_0_var(--border),0_0_0_1px_var(--border)]" : "hover:bg-foreground/4",
            "group-data-popup-open/ctx:ring-1 group-data-popup-open/ctx:ring-ember/50",
          )}
        >
          {active && <span className="absolute top-2.5 bottom-2.5 left-0 w-0.5 rounded-full bg-ember" />}
          <Led tone={stateTone(state)} live={state === "running"} />
          <span className="min-w-0 flex-1 truncate font-mono text-[13px]">
            <span className={cn(state === "running" || active ? "text-foreground" : "text-foreground/75")}>
              {site.name}
            </span>
            <span className="text-muted-foreground/60">{t(".test")}</span>
          </span>
          <span className="flex shrink-0 flex-col items-end font-mono text-[10px] leading-tight text-muted-foreground">
            <span className={cn(state === "failed" && "text-destructive")}>
              {state === "failed" ? t("failed") : (site.stack.adapter ?? KIND_TAG[site.kind])}
            </span>
            {site.port && <span className="opacity-60">:{site.port}</span>}
          </span>
        </button>
      </SiteContextMenu>
    </li>
  );
}
