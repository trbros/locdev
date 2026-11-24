import { Copy, Download, MoreHorizontal, RefreshCw, Star, StarOff, Trash2 } from "lucide-react";
import { type ReactNode, useState } from "react";

import { PhpSettings } from "@/components/php-settings";
import { Page, Section, SubTabs, useCrumbs } from "@/components/page";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  useCatalog,
  useDaemonDown,
  useProjects,
  useRuntimeActions,
  useRuntimes,
  useTerminalRuntimes,
} from "@/hooks/use-daemon";
import { tildify } from "@/lib/format";
import { RUNTIME_TABS, subById, viewById } from "@/lib/nav";
import {
  type InstallJob,
  pinnable,
  type Release,
  releaseLine,
  RUNTIME_LABEL,
  type RuntimeInfo,
  type RuntimeKind,
  type RuntimeVersion,
  type SupportState,
  versionMatches,
} from "@/lib/protocol";
import { copyText } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t, tn } from "@/lib/i18n";
import { PhpModulesPanel } from "@/components/insights";
import { tx } from "@/lib/i18n-jsx";

export function RuntimesView() {
  const view = viewById("runtimes");
  const { sub: current, navigate } = useUi();
  const sub = subById(view, current)!;
  const crumbs = useCrumbs(view, sub);
  const runtimes = useRuntimes();
  const down = useDaemonDown();
  const { rescan } = useRuntimeActions();
  const [installing, setInstalling] = useState<RuntimeKind | null>(null);

  const shown = (runtimes.data ?? []).filter((info) =>
    sub.id === "node"
      ? info.kind === "node"
      : sub.id === "php"
        ? info.kind === "php"
        : sub.id === "package-managers" && info.category === "package_manager",
  );
  const attention = (kinds: (r: RuntimeInfo) => boolean) =>
    (runtimes.data ?? [])
      .filter(kinds)
      .some((r) => r.updates.length > 0 || r.versions.some((v) => v.support?.state === "eol" && r.usage[v.version]));

  return (
    <Page
      view={view}
      crumbs={crumbs}
      wide
      actions={
        <Button variant="outline" size="sm" onClick={() => rescan.mutate()} disabled={rescan.isPending || down}>
          <RefreshCw data-icon="inline-start" className={cn(rescan.isPending && "animate-spin")} /> {t("Re-scan")}
        </Button>
      }
      tabs={
        <SubTabs
          items={RUNTIME_TABS.map((tab) => ({
            ...tab,
            badge:
              (tab.id === "node" && attention((r) => r.kind === "node")) ||
              (tab.id === "php" && attention((r) => r.kind === "php")) ||
              (tab.id === "package-managers" && attention((r) => r.category === "package_manager")) ? (
                <span className="size-1.5 rounded-full bg-amber" />
              ) : undefined,
          }))}
          value={sub.id}
          onChange={(id) => navigate("runtimes", id)}
        />
      }
    >
      {down ? (
        <p className="text-sm text-muted-foreground">{t("Start the daemon to manage runtimes.")}</p>
      ) : runtimes.isPending ? (
        <div className="flex flex-col gap-2">
          {[0, 1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-11" />
          ))}
        </div>
      ) : sub.id === "overview" ? (
        <RuntimeOverview />
      ) : (
        <>
          {shown.map((info) => (
            <RuntimeSection key={info.kind} info={info} onInstall={() => setInstalling(info.kind)} />
          ))}
          {sub.id === "php" && <PhpSettings />}
        </>
      )}

      <CatalogDialog kind={installing} onClose={() => setInstalling(null)} />
    </Page>
  );
}

/** Defaults, then what each site actually runs with and why. */
function RuntimeOverview() {
  const runtimes = useRuntimes();
  const projects = useProjects();
  const { navigate, openSite } = useUi();
  const all = runtimes.data ?? [];
  const sites = projects.data ?? [];
  const columns: RuntimeKind[] = ["node", "php", "pnpm", "yarn", "composer"].filter((kind) =>
    sites.some((p) => p.runtimes.some((r) => r.kind === kind)),
  ) as RuntimeKind[];

  return (
    <>
      <Section title={t("defaults")}>
        <div className="grid grid-cols-2 gap-x-8">
          {all
            .filter((r) => pinnable(r.kind))
            .map((r) => {
              const resolved = r.default ? r.versions.find((v) => versionMatches(r.default!, v.version)) : undefined;
              return (
                <button
                  type="button"
                  key={r.kind}
                  onClick={() => navigate("runtimes", r.category === "runtime" ? r.kind : "package-managers")}
                  className="flex items-baseline gap-3 border-b border-dashed py-2 text-left text-sm hover:text-foreground"
                >
                  <span className="w-20 shrink-0 text-muted-foreground">{RUNTIME_LABEL[r.kind]}</span>
                  <span className="min-w-0 flex-1 truncate font-mono text-[12.5px]">
                    {r.default ? (
                      <>
                        {r.default}
                        {resolved ? (
                          <span className="text-muted-foreground"> → {resolved.version}</span>
                        ) : (
                          <span className="text-amber"> {t("not installed")}</span>
                        )}
                      </>
                    ) : (
                      <span className="text-muted-foreground">{t("system PATH")}</span>
                    )}
                  </span>
                  <span className="font-mono text-[11px] text-muted-foreground">
                    {t("{length} found", { length: r.versions.length })}
                  </span>
                </button>
              );
            })}
        </div>
      </Section>

      <TerminalSection />

      <Section title={t("what each site runs")}>
        {sites.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("No sites yet.")}</p>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("Site")}</TableHead>
                {columns.map((kind) => (
                  <TableHead key={kind}>{RUNTIME_LABEL[kind]}</TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {sites.map((site) => (
                <TableRow key={site.name}>
                  <TableCell>
                    <button
                      type="button"
                      onClick={() => openSite(site.name, "settings")}
                      className="font-mono text-[12.5px] hover:text-ember"
                    >
                      {site.domain}
                    </button>
                  </TableCell>
                  {columns.map((kind) => {
                    const r = site.runtimes.find((x) => x.kind === kind);
                    return (
                      <TableCell key={kind} className="font-mono text-[12px]">
                        {!r ? (
                          <span className="text-muted-foreground/60">{t("PATH")}</span>
                        ) : r.version ? (
                          <span title={`${r.from}: ${r.requested}`}>
                            {r.version.version}
                            <span className="text-muted-foreground">
                              {" "}
                              ·{" "}
                              {r.from === "default"
                                ? t("default")
                                : r.from === "projects.toml"
                                  ? t("pin")
                                  : r.from.split(" ")[0]}
                            </span>
                            {r.unmet && <span className="text-amber"> ≠ {r.unmet.constraint}</span>}
                            {r.version.support?.state === "eol" && (
                              <span className="text-destructive"> {t("eol")}</span>
                            )}
                          </span>
                        ) : (
                          <span className="text-destructive">
                            {t("{requested} missing", { requested: r.requested })}
                          </span>
                        )}
                      </TableCell>
                    );
                  })}
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </Section>

      <p className="text-[13px] text-muted-foreground">
        {tx(
          "A site uses, in order: its own pin, its version files ({nvmrc}, {php_version}, {packagemanager}), the newest installed version within its declared ranges ({engines}, {require_php}), then the default. Without any of them, it uses whatever is first on your PATH. Package managers run on the site's own Node.js or PHP.",
          {
            nvmrc: <span className="font-mono text-foreground">.nvmrc</span>,
            php_version: <span className="font-mono text-foreground">.php-version</span>,
            packagemanager: <span className="font-mono text-foreground">packageManager</span>,
            engines: <span className="font-mono text-foreground">engines</span>,
            require_php: <span className="font-mono text-foreground">require.php</span>,
          },
        )}
      </p>
    </>
  );
}

/** The defaults in new terminals: LocDev's shim folder first on PATH. */
function TerminalSection() {
  const { query, toggle } = useTerminalRuntimes();
  const state = query.data;
  if (!state) return null;
  return (
    <Section title={t("terminal")}>
      <label className="flex items-start gap-3 text-sm">
        <Switch
          checked={state.enabled}
          disabled={toggle.isPending}
          onCheckedChange={(on) => toggle.mutate(on)}
          className="mt-0.5 data-checked:bg-ember"
        />
        <span className="flex flex-col gap-1">
          <span>{t("Use the defaults in the terminal")}</span>
          <span className="text-[12.5px] text-muted-foreground">
            {t(
              "New terminals run the default versions above. Sites keep their own pins. Turning it off removes only LocDev's lines.",
            )}
          </span>
        </span>
      </label>
      <dl className="mt-3 grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1.5 text-[12.5px]">
        <dt className="text-muted-foreground">{t("commands")}</dt>
        <dd className="font-mono">
          {state.commands.length > 0 ? (
            state.commands.join(" ")
          ) : (
            <span className="text-muted-foreground">{t("none yet: set a default above")}</span>
          )}
        </dd>
        <dt className="text-muted-foreground">{t("folder")}</dt>
        <dd className="min-w-0 truncate font-mono" title={state.dir}>
          {tildify(state.dir)}
        </dd>
        {state.files.length > 0 && (
          <>
            <dt className="text-muted-foreground">{t("added to")}</dt>
            <dd className="font-mono">{state.files.map(tildify).join(", ")}</dd>
          </>
        )}
      </dl>
      {state.enabled && !state.active && (
        <p className="mt-2 text-[12.5px] text-amber">{t("Open a new terminal to use them.")}</p>
      )}
    </Section>
  );
}

function RuntimeSection({ info, onInstall }: { info: RuntimeInfo; onInstall: () => void }) {
  const { setDefault, install } = useRuntimeActions();
  // What the default pin resolves to (versions are newest first).
  const current = info.default ? info.versions.find((v) => versionMatches(info.default!, v.version)) : undefined;

  return (
    <Section
      title={RUNTIME_LABEL[info.kind].toLowerCase()}
      aside={
        <span className="flex items-center gap-2 font-mono text-[11px]">
          {info.default ? (
            <>
              <Led tone={current ? "live" : "warn"} />
              default {info.default}
              {current ? (
                <span className="text-muted-foreground">→ {current.version}</span>
              ) : (
                <span className="text-amber">{t("not installed")}</span>
              )}
              <button
                type="button"
                onClick={() => setDefault.mutate({ kind: info.kind, version: null })}
                className="ml-1 text-muted-foreground underline decoration-dotted underline-offset-4 hover:text-foreground"
              >
                {t("clear")}
              </button>
            </>
          ) : (
            <span className="text-muted-foreground">
              {pinnable(info.kind) ? t("no default · system PATH") : t("follows Node.js")}
            </span>
          )}
        </span>
      }
    >
      {info.job && info.job.state !== "done" && <JobPanel job={info.job} />}

      {info.updates.map((u) => (
        <div
          key={u.line}
          className="mb-2 flex items-center gap-3 rounded-lg border border-signal/30 bg-signal/6 px-3 py-2 font-mono text-[12px]"
        >
          <span>
            {u.line}.x: {u.installed} → <span className="text-signal">{u.latest}</span>
          </span>
          <Button
            size="xs"
            variant="outline"
            className="ml-auto"
            disabled={install.isPending || (!!info.job && !["done", "failed"].includes(info.job.state))}
            onClick={() => install.mutate({ kind: info.kind, version: u.latest })}
          >
            {t("Install {latest}", { latest: u.latest })}
          </Button>
        </div>
      ))}

      {info.versions.length === 0 ? (
        <p className="py-3 text-sm text-muted-foreground">
          {tx("No {runtime} found on this machine.", { runtime: RUNTIME_LABEL[info.kind] })}
        </p>
      ) : (
        <ul>
          {info.versions.map((v, i) => (
            <VersionRow key={`${v.version}-${v.source}`} info={info} v={v} current={current === v} index={i} />
          ))}
        </ul>
      )}

      <div className="mt-3 flex items-center gap-3">
        {info.downloads ? (
          <Button
            size="sm"
            variant="outline"
            onClick={onInstall}
            disabled={!!info.job && !["done", "failed"].includes(info.job.state)}
          >
            <Download data-icon="inline-start" /> {t("Install version…")}
          </Button>
        ) : (
          <p className="text-[13px] text-muted-foreground">
            {info.kind === "npm"
              ? t("npm comes with each Node.js version and follows the site's Node.js.")
              : info.kind === "php"
                ? t(
                    "PHP versions installed on this machine (packages, Homebrew, other tools) are detected. There's no official PHP build with a published SHA-256 for this platform, so LocDev doesn't download one.",
                  )
                : t("{kind} versions installed on this machine are detected; LocDev doesn't download it.", {
                    kind: RUNTIME_LABEL[info.kind],
                  })}
          </p>
        )}
      </div>
    </Section>
  );
}

function JobPanel({ job }: { job: InstallJob }) {
  const percent = job.total ? Math.round((job.downloaded / job.total) * 100) : null;
  const label =
    job.state === "downloading"
      ? `${t("downloading {mb}", { mb: (job.downloaded / 1e6).toFixed(1) })}${job.total ? ` / ${(job.total / 1e6).toFixed(1)}` : ""} MB`
      : job.state === "verifying"
        ? "verifying sha256…"
        : job.state === "extracting"
          ? "extracting…"
          : "failed";

  return (
    <div
      className={cn(
        "mb-3 rounded-lg border px-3.5 py-3",
        job.state === "failed" ? "border-destructive/30 bg-destructive/6" : "bg-well",
      )}
    >
      <div className="flex items-center justify-between font-mono text-[12px]">
        <span>
          {t("installing")} {RUNTIME_LABEL[job.kind]} {job.version}
        </span>
        <span className={job.state === "failed" ? "text-destructive" : "text-muted-foreground"}>{label}</span>
      </div>
      {job.state === "failed" ? (
        <p className="mt-1.5 text-[13px] text-destructive">{job.error}</p>
      ) : (
        <Progress
          value={job.state === "downloading" ? percent : null}
          className="mt-2.5 **:data-[slot=progress-indicator]:bg-ember"
        />
      )}
      {!!job.log?.length && (
        <details className="mt-2 text-[11.5px]">
          <summary className="cursor-pointer text-muted-foreground">{t("install log")}</summary>
          <pre
            className="mt-1.5 overflow-x-auto font-mono leading-relaxed whitespace-pre-wrap text-muted-foreground"
            data-selectable
          >
            {job.log.join("\n")}
          </pre>
        </details>
      )}
    </div>
  );
}

interface VersionAction {
  id: string;
  label: string;
  icon: typeof Star;
  run: () => void;
  destructive?: boolean;
}

/** One source of actions for a version's ⋯ menu and right-click menu. */
function useVersionActions(info: RuntimeInfo, v: RuntimeVersion, onRemove: () => void): VersionAction[][] {
  const { setDefault } = useRuntimeActions();
  const kind = info.kind;
  const line = releaseLine(kind, v.version);

  const defaults: VersionAction[] = !pinnable(kind)
    ? []
    : [
        {
          id: "default-line",
          label: t("Use {line} as default (newest {line}.x)", { line }),
          icon: Star,
          run: () => setDefault.mutate({ kind, version: line }),
        },
        {
          id: "default-exact",
          label: t("Use exactly {version} as default", { version: v.version }),
          icon: Star,
          run: () => setDefault.mutate({ kind, version: v.version }),
        },
      ];
  if (info.default) {
    defaults.push({
      id: "default-clear",
      label: t("Clear default"),
      icon: StarOff,
      run: () => setDefault.mutate({ kind, version: null }),
    });
  }

  const groups = [
    defaults,
    [{ id: "copy", label: t("Copy executable path"), icon: Copy, run: () => copyText(v.executable, t("Path copied")) }],
  ].filter((g) => g.length);
  if (v.source === "managed" && v.origin === "locdev") {
    groups.push([
      {
        id: "uninstall",
        label: t("Uninstall {version}…", { version: v.version }),
        icon: Trash2,
        destructive: true,
        run: onRemove,
      },
    ]);
  }
  return groups;
}

function VersionRow({
  info,
  v,
  current,
  index,
}: {
  info: RuntimeInfo;
  v: RuntimeVersion;
  current: boolean;
  index: number;
}) {
  const [removing, setRemoving] = useState(false);
  const [extensions, setExtensions] = useState(false);
  const groups = useVersionActions(info, v, () => setRemoving(true));
  const users = info.usage[v.version] ?? [];

  return (
    <li className="rise" style={{ "--i": index } as React.CSSProperties}>
      <ContextMenu>
        <ContextMenuTrigger className="group/ctx">
          <div className="flex items-center gap-3 border-b border-dashed py-2.5 group-data-popup-open/ctx:bg-foreground/3">
            <Led tone={current ? "live" : "off"} live={current} />
            <span className="w-24 shrink-0 font-mono text-[14px]">{v.version}</span>
            <SourceBadge v={v} />
            <SupportBadge v={v} />
            <span
              className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-muted-foreground"
              title={v.executable}
            >
              {tildify(v.executable)}
            </span>
            {users.length > 0 && (
              <span className="shrink-0 font-mono text-[11px] text-muted-foreground" title={users.join(", ")}>
                {users.length === 1 ? users[0] : tn("{n} site", "{n} sites", users.length)}
              </span>
            )}
            {info.kind === "php" && (
              <Button size="xs" variant="ghost" aria-expanded={extensions} onClick={() => setExtensions(!extensions)}>
                {t("Extensions")}
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger render={<Button variant="ghost" size="icon-sm" aria-label={t("Version actions")} />}>
                <MoreHorizontal />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-64">
                {groups.map((group, i) => (
                  <MenuGroup key={group[0].id} first={i === 0} kind="dropdown">
                    {group.map((a) => (
                      <DropdownMenuItem key={a.id} onClick={a.run} variant={a.destructive ? "destructive" : "default"}>
                        <a.icon /> {a.label}
                      </DropdownMenuItem>
                    ))}
                  </MenuGroup>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent className="w-64">
          {groups.map((group, i) => (
            <MenuGroup key={group[0].id} first={i === 0} kind="context">
              {group.map((a) => (
                <ContextMenuItem key={a.id} onClick={a.run} variant={a.destructive ? "destructive" : "default"}>
                  <a.icon /> {a.label}
                </ContextMenuItem>
              ))}
            </MenuGroup>
          ))}
        </ContextMenuContent>
      </ContextMenu>
      {extensions && (
        <div className="border-b border-dashed py-3 pl-7">
          <PhpModulesPanel executable={v.executable} />
        </div>
      )}
      <RemoveVersionDialog open={removing} onClose={() => setRemoving(false)} info={info} v={v} users={users} />
    </li>
  );
}

function RemoveVersionDialog({
  open,
  onClose,
  info,
  v,
  users,
}: {
  open: boolean;
  onClose: () => void;
  info: RuntimeInfo;
  v: RuntimeVersion;
  users: string[];
}) {
  const { uninstall } = useRuntimeActions();
  const remove = (force: boolean) =>
    uninstall.mutate({ kind: info.kind, version: v.version, force }, { onSuccess: onClose });

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>
            {t("Remove")} {RUNTIME_LABEL[info.kind]} {v.version}?
          </DialogTitle>
          <DialogDescription>
            {users.length
              ? tn(
                  "{users} uses this version. Running sites block the removal; stopped ones will fall back to another version or fail to start until you install one.",
                  "{users} use this version. Running sites block the removal; stopped ones will fall back to another version or fail to start until you install one.",
                  users.length,
                  { users: users.join(", ") },
                )
              : t("Nothing uses this version. Its folder is deleted; you can install it again later.")}
          </DialogDescription>
        </DialogHeader>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button variant="destructive" disabled={uninstall.isPending} onClick={() => remove(users.length > 0)}>
            <Trash2 data-icon="inline-start" /> {users.length ? t("Remove anyway") : t("Remove")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

const SUPPORT_LABEL: Record<SupportState, string> = {
  active: "",
  maintenance: t("maintenance"),
  security: t("security only"),
  eol: t("end-of-life"),
};

function SupportBadge({ v }: { v: RuntimeVersion }) {
  const state = v.support?.state;
  if (!state || state === "active") return null;
  return (
    <span
      title={
        v.support?.until
          ? t("{state} since/until {date}", { state: SUPPORT_LABEL[state], date: v.support.until })
          : undefined
      }
      className={cn(
        "rounded-md border px-1.5 font-mono text-[10.5px]",
        state === "eol" && "border-destructive/40 text-destructive",
        state === "security" && "border-amber/40 text-amber",
        state === "maintenance" && "text-muted-foreground",
      )}
    >
      {SUPPORT_LABEL[state]}
    </span>
  );
}

function MenuGroup({ first, kind, children }: { first: boolean; kind: "dropdown" | "context"; children: ReactNode }) {
  return (
    <>
      {!first && (kind === "dropdown" ? <DropdownMenuSeparator /> : <ContextMenuSeparator />)}
      {children}
    </>
  );
}

function SourceBadge({ v }: { v: RuntimeVersion }) {
  return v.source === "managed" ? (
    <span className="rounded-md border border-ember/40 px-1.5 font-mono text-[10.5px] text-ember">{t("locdev")}</span>
  ) : (
    <span className="rounded-md border px-1.5 font-mono text-[10.5px] text-muted-foreground">{v.origin}</span>
  );
}

/** Picks a release from the catalog; newest of each major unless "all". */
function CatalogDialog({ kind, onClose }: { kind: RuntimeKind | null; onClose: () => void }) {
  const catalog = useCatalog(kind);
  const runtimes = useRuntimes();
  const { install } = useRuntimeActions();
  const [all, setAll] = useState(false);

  const installed = new Set(runtimes.data?.find((r) => r.kind === kind)?.versions.map((v) => v.version) ?? []);
  const releases = all || !kind ? (catalog.data ?? []) : newestPerLine(kind, catalog.data ?? []);

  return (
    <Dialog open={kind !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("Install {name}", { name: kind ? RUNTIME_LABEL[kind] : "" })}</DialogTitle>
          <DialogDescription>
            {kind === "php"
              ? t(
                  "Ready-made builds from the projects that publish them: StaticPHP (static-php.dev) on Linux and macOS, windows.php.net on Windows. Every download must match the publisher's SHA-256 before anything is installed.",
                )
              : kind === "pnpm" || kind === "yarn"
                ? t(
                    "Published on the npm registry; every tarball must match the registry's sha512 integrity before anything is installed. It runs on the site's own Node.js.",
                  )
                : kind === "composer"
                  ? t("From getcomposer.org, verified against its published SHA-256. It runs on the site's own PHP.")
                  : t(
                      "Official builds, verified against the publisher's SHA-256 checksums before anything is installed.",
                    )}
          </DialogDescription>
        </DialogHeader>

        <label className="flex items-center gap-2.5 text-sm">
          <Switch checked={all} onCheckedChange={setAll} className="data-checked:bg-ember" />
          {t("Show every release")}
        </label>

        <div className="max-h-80 min-w-0 overflow-y-auto rounded-lg border bg-well">
          {catalog.isPending ? (
            <div className="space-y-2 p-3">
              {[0, 1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-7" />
              ))}
            </div>
          ) : catalog.isError ? (
            <p className="p-3 text-sm text-destructive">{(catalog.error as Error).message}</p>
          ) : (
            <ul>
              {releases.map((r) => {
                const has = installed.has(r.version);
                return (
                  <li key={r.version}>
                    <button
                      type="button"
                      disabled={has || install.isPending}
                      onClick={() =>
                        kind && install.mutate({ kind, version: r.version }, { onSuccess: () => onClose() })
                      }
                      className="flex w-full items-center gap-3 border-b px-3 py-2 text-left font-mono text-[13px] last:border-b-0 enabled:hover:bg-foreground/4 disabled:opacity-60"
                    >
                      <span className="w-20">{r.version}</span>
                      <span className="text-[11px] text-muted-foreground">{r.date}</span>
                      {r.lts && (
                        <span className="rounded-md border border-phosphor/40 px-1.5 text-[10.5px] text-phosphor">
                          {t("LTS {lts}", { lts: r.lts })}
                        </span>
                      )}
                      <span className="ml-auto text-[11px] text-muted-foreground">
                        {has ? t("installed") : t("install")}
                      </span>
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/** The newest release of each line (major.minor for PHP, the major otherwise). */
function newestPerLine(kind: RuntimeKind, releases: Release[]): Release[] {
  const seen = new Set<string>();
  return releases.filter((r) => {
    const line = releaseLine(kind, r.version);
    if (seen.has(line)) return false;
    seen.add(line);
    return true;
  });
}
