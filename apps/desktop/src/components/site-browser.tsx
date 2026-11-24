// The Sites page as a browser: a strip of tabs (one site each), an address
// bar that shows the site's URL and switches sites when typed into, and a
// new-tab page. Tabs are UI state only (`ui-state.tsx`); the daemon never
// knows about them.

import {
  ExternalLink,
  FolderOpen,
  Globe,
  Lock,
  LockOpen,
  MoreHorizontal,
  Play,
  Plus,
  RotateCw,
  Share2,
  Square,
  X,
} from "lucide-react";
import { type KeyboardEvent, type MouseEvent, useEffect, useMemo, useRef, useState } from "react";

import { SiteDropdownItems } from "@/components/site-menu";
import { Led, StateChip, stateTone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { useProjectAction } from "@/hooks/use-daemon";
import { STATE_LABEL, tildify, uptime } from "@/lib/format";
import { t } from "@/lib/i18n";
import { MOD } from "@/lib/nav";
import { useShortcutLabel, withKeys } from "@/lib/shortcuts";
import { KIND_LABEL, type ProjectView } from "@/lib/protocol";
import { openInBrowser, revealFolder } from "@/lib/system";
import { type SiteTabState, useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

/** Ctrl/⌘+click or a middle click opens in a new tab, as in a browser. */
export const wantsNewTab = (e: MouseEvent) => e.ctrlKey || e.metaKey || e.button === 1;

export function SiteTabStrip({ sites }: { sites: ProjectView[] }) {
  const { tabs, activeTab, activateTab, closeTab, newTab, moveTab } = useUi();
  const newTabKeys = useShortcutLabel("new-tab");
  const [dragging, setDragging] = useState<string | null>(null);
  const strip = useRef<HTMLDivElement>(null);

  // Keep the active tab in view when there are more than fit.
  useEffect(() => {
    strip.current
      ?.querySelector<HTMLElement>(`[data-tab="${activeTab}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [activeTab]);

  return (
    <div className="flex h-10 shrink-0 items-end gap-1 border-b bg-well/50 pr-2 pl-2">
      <div
        ref={strip}
        role="tablist"
        aria-label={t("Open sites")}
        className="flex min-w-0 items-end gap-1 overflow-x-auto [scrollbar-width:none]"
      >
        {tabs.map((tab, index) => (
          <TabButton
            key={tab.id}
            tab={tab}
            site={sites.find((s) => s.name === tab.site)}
            active={tab.id === activeTab}
            closable={tabs.length > 1 || tab.site !== null}
            dragging={dragging === tab.id}
            onActivate={() => activateTab(tab.id)}
            onClose={() => closeTab(tab.id)}
            onDragStart={() => setDragging(tab.id)}
            onDragEnd={() => setDragging(null)}
            onDrop={() => dragging && moveTab(dragging, index)}
          />
        ))}
      </div>
      <Button
        variant="ghost"
        size="icon-xs"
        className="mb-1.5 shrink-0"
        onClick={() => newTab()}
        aria-label={t("New tab")}
        title={withKeys(t("New tab"), newTabKeys)}
      >
        <Plus />
      </Button>
    </div>
  );
}

function TabButton({
  tab,
  site,
  active,
  closable,
  dragging,
  onActivate,
  onClose,
  onDragStart,
  onDragEnd,
  onDrop,
}: {
  tab: SiteTabState;
  site: ProjectView | undefined;
  active: boolean;
  closable: boolean;
  dragging: boolean;
  onActivate: () => void;
  onClose: () => void;
  onDragStart: () => void;
  onDragEnd: () => void;
  onDrop: () => void;
}) {
  const closeKeys = useShortcutLabel("close-tab");
  const state = site?.process.state;
  const [name, ...rest] = (site?.domain ?? "").split(".");

  return (
    // The close button sits next to the tab, not inside it: a control
    // within a role="tab" is unreachable for screen readers.
    <div
      role="presentation"
      data-tab={tab.id}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        onDrop();
      }}
      // A middle click closes, as in a browser (and must not paste or scroll on Linux).
      onMouseDown={(e) => e.button === 1 && e.preventDefault()}
      onAuxClick={(e) => e.button === 1 && closable && onClose()}
      onClick={onActivate}
      className={cn(
        "group relative flex h-8 w-44 min-w-28 shrink cursor-default items-center gap-2 rounded-t-md border border-b-0 pr-1 pl-2.5 text-[12.5px] transition-colors select-none",
        "has-[[role=tab]:focus-visible]:outline-2 has-[[role=tab]:focus-visible]:-outline-offset-2 has-[[role=tab]:focus-visible]:outline-ring",
        active
          ? "-mb-px h-[33px] border-border bg-background text-foreground before:absolute before:inset-x-2 before:top-0 before:h-0.5 before:rounded-b-full before:bg-ember"
          : "border-transparent text-muted-foreground hover:bg-foreground/4 hover:text-foreground",
        dragging && "opacity-50",
      )}
    >
      <div
        role="tab"
        aria-selected={active}
        aria-keyshortcuts={closable ? (closeKeys ?? undefined) : undefined}
        tabIndex={active ? 0 : -1}
        title={site ? `${site.domain} · ${tildify(site.path)}` : t("New tab")}
        onKeyDown={(e) => (e.key === "Enter" || e.key === " ") && onActivate()}
        className="flex min-w-0 flex-1 items-center gap-2 self-stretch outline-none"
      >
        {site ? (
          <Led tone={stateTone(state!)} live={state === "running"} />
        ) : (
          <Globe className="size-3.5 shrink-0" strokeWidth={1.75} />
        )}
        <span className="min-w-0 flex-1 truncate font-mono">
          {site ? (
            <>
              {name}
              <span className="text-muted-foreground/60">.{rest.join(".")}</span>
            </>
          ) : (
            <span className="font-sans">{t("New tab")}</span>
          )}
        </span>
      </div>
      {closable && (
        // Out of the accessibility tree (a tablist holds only tabs): the
        // keyboard closes the focused tab with its shortcut, announced on the tab.
        <button
          type="button"
          tabIndex={-1}
          aria-hidden
          aria-label={t("Close tab")}
          title={withKeys(t("Close tab"), closeKeys)}
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          className={cn(
            "grid size-5 shrink-0 place-items-center rounded-sm text-muted-foreground hover:bg-foreground/8 hover:text-foreground",
            !active && "opacity-0 group-hover:opacity-100",
          )}
        >
          <X className="size-3" />
        </button>
      )}
    </div>
  );
}

/** Sites matching what was typed in the address bar, best first. */
function matchSites(sites: ProjectView[], typed: string): ProjectView[] {
  const host = typed
    .trim()
    .toLowerCase()
    .replace(/^[a-z]+:\/\//, "")
    .replace(/[/:?#].*$/, "");
  if (!host) return sites.slice(0, 8);
  const rank = (s: ProjectView) =>
    s.domain === host || s.name === host
      ? 0
      : host.endsWith(`.${s.domain}`)
        ? 1 // a subdomain belongs to its site
        : s.name.startsWith(host) || s.domain.startsWith(host)
          ? 2
          : s.domain.includes(host) || s.path.toLowerCase().includes(host)
            ? 3
            : 9;
  return sites
    .map((s) => [rank(s), s] as const)
    .filter(([r]) => r < 9)
    .sort((a, b) => a[0] - b[0] || a[1].name.localeCompare(b[1].name))
    .map(([, s]) => s)
    .slice(0, 8);
}

/** The site's address, its security and state, and its controls. */
export function AddressBar({ site, sites }: { site: ProjectView; sites: ProjectView[] }) {
  const action = useProjectAction();
  const { setSharing } = useUi();
  const state = site.process.state;
  const running = state === "running";
  const busy = action.isPending || state === "stopping";
  const run = (kind: "start_project" | "stop_project" | "restart_project") =>
    action.mutate({ action: kind, name: site.name });

  return (
    <div className="flex shrink-0 items-center gap-2 border-b px-4 py-2">
      <Omnibox site={site} sites={sites} />
      <div className="flex shrink-0 items-center gap-0.5">
        {running ? (
          <>
            <Button
              variant="ghost"
              size="icon-sm"
              disabled={busy}
              onClick={() => run("restart_project")}
              aria-label={t("Restart")}
              title={t("Restart")}
            >
              <RotateCw />
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              disabled={busy}
              onClick={() => run("stop_project")}
              aria-label={t("Stop")}
              title={t("Stop")}
            >
              <Square className="fill-current" />
            </Button>
          </>
        ) : (
          <Button
            size="sm"
            disabled={busy || !site.command}
            title={site.command ? undefined : t("Set a dev server command first")}
            onClick={() => run("start_project")}
            className="mr-1 bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <Play data-icon="inline-start" className="fill-current" /> {t("Start")}
          </Button>
        )}
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={!running || !site.url}
          onClick={() => site.url && openInBrowser(site.url)}
          aria-label={t("Open in browser")}
          title={running ? t("Open in browser") : t("Start the site to open it")}
        >
          <ExternalLink />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={!running || !site.port}
          onClick={() => setSharing(site.name)}
          aria-label={t("Share on the internet…")}
          title={running ? t("Share on the internet…") : t("Start the site to share it")}
        >
          <Share2 />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={() => revealFolder(site.path)}
          aria-label={t("Reveal folder")}
          title={tildify(site.path)}
        >
          <FolderOpen />
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button variant="ghost" size="icon-sm" aria-label={t("More actions")} />}>
            <MoreHorizontal />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-56">
            <SiteDropdownItems site={site} />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}

function Omnibox({ site, sites }: { site: ProjectView; sites: ProjectView[] }) {
  const { select, openInNewTab, navigate } = useUi();
  const [editing, setEditing] = useState(false);
  const [typed, setTyped] = useState("");
  const [cursor, setCursor] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const shown = site.url ?? site.https_url;
  const matches = useMemo(() => (editing ? matchSites(sites, typed) : []), [editing, sites, typed]);
  const { state, started_at } = site.process;
  const running = state === "running";

  const start = () => {
    setTyped(site.domain);
    setCursor(0);
    setEditing(true);
  };
  useEffect(() => {
    if (editing) input.current?.select();
  }, [editing]);

  const go = (target: ProjectView | undefined, newTab: boolean) => {
    setEditing(false);
    if (!target) return;
    if (newTab) openInNewTab(target.name);
    else select(target.name);
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Escape") {
      e.preventDefault();
      setEditing(false);
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const step = e.key === "ArrowDown" ? 1 : -1;
      setCursor((c) => (matches.length ? (c + step + matches.length) % matches.length : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      go(matches[cursor], e.ctrlKey || e.metaKey || e.altKey);
    }
  };

  const [scheme, rest] = shown ? (shown.match(/^(https?:\/\/)(.*)$/)?.slice(1) ?? ["", shown]) : ["", site.domain];
  const [host, ...port] = rest.split(":");
  const [label, ...tld] = host.split(".");
  const secure = shown?.startsWith("https://");

  return (
    // A container: what fits next to the address depends on the bar's own width.
    <div className="@container relative min-w-0 flex-1">
      <div
        className={cn(
          "flex h-9 min-w-0 items-center gap-2 rounded-lg border bg-well/60 pr-1.5 pl-2.5 transition-colors",
          editing ? "border-ember/60 bg-background" : "hover:border-foreground/20",
        )}
      >
        {secure ? (
          <span title={t("HTTPS with a trusted certificate")} className="text-phosphor">
            <Lock className="size-3.5" />
          </span>
        ) : site.https_url ? (
          <button
            type="button"
            onClick={() => navigate("settings", "https")}
            title={t("HTTPS works, but browsers will warn until the LocDev CA is trusted")}
            aria-label={t("trust https")}
            className="text-amber hover:text-ember"
          >
            <LockOpen className="size-3.5" />
          </button>
        ) : (
          <span title={t("HTTP only")} className="text-muted-foreground">
            <Globe className="size-3.5" />
          </span>
        )}

        {editing ? (
          <input
            ref={input}
            value={typed}
            onChange={(e) => {
              setTyped(e.target.value);
              setCursor(0);
            }}
            onKeyDown={onKey}
            onBlur={() => setEditing(false)}
            aria-label={t("Site address")}
            aria-autocomplete="list"
            aria-controls="omnibox-matches"
            spellCheck={false}
            className="min-w-0 flex-1 bg-transparent font-mono text-[13.5px] outline-none"
          />
        ) : (
          <button
            type="button"
            onClick={start}
            // The name starts with what's shown, so voice control can say it.
            aria-label={`${shown ?? site.domain} — ${t("Site address")}`}
            title={t("Type a site's name to switch to it")}
            className="min-w-0 flex-1 truncate text-left font-mono text-[13.5px]"
            data-selectable
          >
            <span className="text-muted-foreground/60">{scheme}</span>
            <span className="font-semibold">{label}</span>
            <span className="text-muted-foreground/60">
              {tld.length ? `.${tld.join(".")}` : ""}
              {port.length ? `:${port.join(":")}` : ""}
            </span>
            {!shown && (
              <span className="ml-3 font-sans text-[12px] text-muted-foreground">
                {t("not routed: no port or proxy off")}
              </span>
            )}
          </button>
        )}

        {!editing && (
          <span className="hidden shrink-0 items-center gap-2 @lg:flex">
            {site.url?.includes(".localhost") && (
              <button
                type="button"
                onClick={() => navigate("settings", "dns")}
                className="hidden font-mono text-[11px] text-muted-foreground underline decoration-dotted underline-offset-4 hover:text-ember @2xl:inline"
              >
                {t("set up .test")}
              </button>
            )}
            {site.direct_url && (
              <span className="hidden font-mono text-[11px] text-muted-foreground @3xl:inline" data-selectable>
                → {site.direct_url.replace(/^http:\/\//, "")}
              </span>
            )}
            <span className="rounded border border-ember/30 bg-ember/8 px-1.5 py-0.5 font-mono text-[10.5px] text-ember">
              {site.stack.framework || KIND_LABEL[site.kind]}
            </span>
            <StateChip tone={stateTone(state)} live={running}>
              {STATE_LABEL[state]}
              {running && started_at ? ` · ${uptime(started_at)}` : ""}
            </StateChip>
          </span>
        )}
      </div>

      {editing && (
        <ul
          id="omnibox-matches"
          role="listbox"
          aria-label={t("Sites")}
          className="absolute inset-x-0 top-full z-30 mt-1 overflow-hidden rounded-lg border bg-popover py-1 shadow-lg"
        >
          {matches.length === 0 ? (
            <li className="px-3 py-2 text-[13px] text-muted-foreground">
              {t("No site matches “{filter}”.", { filter: typed })}
            </li>
          ) : (
            matches.map((s, i) => (
              <li
                key={s.name}
                role="option"
                aria-selected={i === cursor}
                // Before the input's blur, so the click lands.
                onMouseDown={(e) => {
                  e.preventDefault();
                  go(s, wantsNewTab(e));
                }}
                onMouseEnter={() => setCursor(i)}
                className={cn(
                  "flex cursor-default items-center gap-2.5 px-3 py-1.5 text-[13px]",
                  i === cursor && "bg-foreground/5",
                )}
              >
                <Led tone={stateTone(s.process.state)} live={s.process.state === "running"} />
                <span className="min-w-0 flex-1 truncate font-mono">{s.domain}</span>
                <span className="truncate text-[12px] text-muted-foreground">
                  {s.stack.framework || KIND_LABEL[s.kind]}
                </span>
              </li>
            ))
          )}
          <li className="border-t px-3 pt-1.5 pb-1 font-mono text-[10.5px] text-muted-foreground">
            {t("Enter opens here · {MOD}+Enter in a new tab", { MOD })}
          </li>
        </ul>
      )}
    </div>
  );
}

/** What an empty tab shows: every site, to open one here. */
export function NewTabPage({ sites }: { sites: ProjectView[] }) {
  const { select, setAddSiteOpen, openInNewTab } = useUi();
  const sorted = [...sites].sort(
    (a, b) =>
      Number(b.process.state === "running") - Number(a.process.state === "running") || a.name.localeCompare(b.name),
  );

  return (
    <div className="dots h-full overflow-y-auto">
      <div className="mx-auto flex max-w-3xl flex-col gap-5 px-8 py-12">
        <div>
          <div className="font-mono text-[11px] text-ember">{t("new tab")}</div>
          <h2 className="mt-1 text-xl font-semibold tracking-[-0.01em]">{t("Open a site")}</h2>
        </div>
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-2">
          {sorted.map((site) => (
            <li key={site.name}>
              <button
                type="button"
                onClick={(e) => (wantsNewTab(e) ? openInNewTab(site.name) : select(site.name))}
                onAuxClick={(e) => e.button === 1 && openInNewTab(site.name)}
                className="flex w-full flex-col gap-1 rounded-lg border bg-background/90 px-3.5 py-3 text-left transition-colors hover:border-ember/50"
              >
                <span className="flex items-center gap-2 font-mono text-[13px]">
                  <Led tone={stateTone(site.process.state)} live={site.process.state === "running"} />
                  <span className="truncate">{site.domain}</span>
                </span>
                <span className="truncate text-[12px] text-muted-foreground">
                  {site.stack.framework || KIND_LABEL[site.kind]} · {tildify(site.path)}
                </span>
              </button>
            </li>
          ))}
          <li>
            <button
              type="button"
              onClick={() => setAddSiteOpen(true)}
              className="flex h-full min-h-16 w-full items-center justify-center gap-2 rounded-lg border border-dashed text-[13px] text-muted-foreground transition-colors hover:border-ember/50 hover:text-foreground"
            >
              <Plus className="size-4" /> {t("Add site")}
            </button>
          </li>
        </ul>
      </div>
    </div>
  );
}
