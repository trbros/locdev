// Settings → Parked folders: folders whose projects become sites on their
// own. The daemon finds, names, registers and watches; this lists and asks.

import { useRef, useState } from "react";
import { FolderOpen, FolderSearch, RefreshCw, SlidersHorizontal, Trash2 } from "lucide-react";

import { Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useSaveProject } from "@/hooks/use-daemon";
import { useParked, useParkIgnore, useParkRemove, useParkScan, useParkSet } from "@/hooks/use-park";
import { tildify, uptime } from "@/lib/format";
import { t, tn } from "@/lib/i18n";
import type { OnMissing, OnNew, ParkedProject, ParkedState, ParkedView } from "@/lib/protocol";
import { pickFolder } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

const STATE: Record<ParkedState, { label: string; tone: Tone }> = {
  registered: { label: t("site"), tone: "live" },
  manual: { label: t("added by hand"), tone: "info" },
  suggested: { label: t("suggested"), tone: "warn" },
  ignored: { label: t("ignored"), tone: "off" },
  failed: { label: t("not added"), tone: "fail" },
};

function folderTone(dir: ParkedView): { tone: Tone; label: string } {
  if (!dir.enabled) return { tone: "off", label: t("off") };
  if (!dir.exists) return { tone: "fail", label: t("folder missing") };
  if (dir.watching) return { tone: "live", label: t("watching") };
  return { tone: "warn", label: t("rescanned every 5 minutes") };
}

export function ParkedFolders() {
  const parked = useParked();
  const scan = useParkScan();
  const [editing, setEditing] = useState<ParkedView | "new" | null>(null);
  const [removing, setRemoving] = useState<ParkedView | null>(null);
  const dirs = parked.data;

  return (
    <>
      <p className="text-[13px] text-muted-foreground">
        {t(
          "Every project inside a parked folder becomes a site (shop/ → shop.test) and stays in step as projects are created, cloned or deleted. A subfolder counts only when its files say it's a project (composer.json, package.json, pyproject.toml, Cargo.toml…); dependencies, build output and hidden folders are skipped. New sites start stopped, and removing one never touches its files.",
        )}
      </p>
      <Section
        title={t("parked folders · {count}", { count: dirs?.length ?? 0 })}
        aside={
          <div className="flex gap-1.5">
            {dirs && dirs.length > 0 && (
              <Button variant="ghost" size="sm" disabled={scan.isPending} onClick={() => scan.mutate(null)}>
                <RefreshCw className={cn(scan.isPending && "animate-spin")} /> {t("Scan all")}
              </Button>
            )}
            <Button size="sm" onClick={() => setEditing("new")}>
              <FolderSearch /> {t("Park a folder")}
            </Button>
          </div>
        }
      >
        {!dirs ? (
          <Skeleton className="h-32" />
        ) : dirs.length === 0 ? (
          <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-6 py-10 text-center">
            <FolderSearch className="size-6 text-muted-foreground" />
            <p className="text-sm">{t("No parked folders yet")}</p>
            <p className="max-w-md text-[13px] text-muted-foreground">
              {t("Park the folder where you keep your projects, like ~/Development, and each one becomes a site.")}
            </p>
          </div>
        ) : (
          <div className="flex flex-col gap-4 pt-1">
            {dirs.map((dir) => (
              <ParkedCard key={dir.path} dir={dir} onEdit={() => setEditing(dir)} onRemove={() => setRemoving(dir)} />
            ))}
          </div>
        )}
      </Section>
      <RulesDialog editing={editing} onClose={() => setEditing(null)} />
      <RemoveDialog dir={removing} onClose={() => setRemoving(null)} />
    </>
  );
}

function ParkedCard({ dir, onEdit, onRemove }: { dir: ParkedView; onEdit: () => void; onRemove: () => void }) {
  const set = useParkSet();
  const scan = useParkScan();
  const state = folderTone(dir);
  const sites = dir.projects.filter((p) => p.state === "registered" || p.state === "manual").length;
  const suggested = dir.projects.filter((p) => p.state === "suggested").length;

  return (
    <div className={cn("rounded-lg border", !dir.enabled && "opacity-70")}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-4 py-3">
        <Led tone={state.tone} live={state.tone === "live"} />
        <div className="min-w-0 flex-1">
          <div className="truncate font-mono text-sm" title={dir.path}>
            {tildify(dir.path)}
          </div>
          <div className="mt-0.5 text-[12px] text-muted-foreground">
            {state.label} · {tn("{n} site", "{n} sites", sites)}
            {suggested > 0 && <> · {tn("{n} suggested project", "{n} suggested projects", suggested)}</>}
            {dir.last_scan && <> · {t("scanned {time} ago", { time: uptime(dir.last_scan) })}</>}
          </div>
        </div>
        <Switch
          checked={dir.enabled}
          onCheckedChange={(enabled) => set.mutate({ path: dir.path, options: { enabled } })}
          aria-label={t("Watch this folder")}
          className="data-checked:bg-ember"
        />
        <Button
          variant="ghost"
          size="icon-sm"
          disabled={!dir.enabled || scan.isPending}
          onClick={() => scan.mutate(dir.path)}
          aria-label={t("Scan now")}
          title={t("Scan now")}
        >
          <RefreshCw className={cn(scan.isPending && "animate-spin")} />
        </Button>
        <Button variant="ghost" size="icon-sm" onClick={onEdit} aria-label={t("Rules")} title={t("Rules")}>
          <SlidersHorizontal />
        </Button>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onRemove}
          aria-label={t("Stop parking")}
          title={t("Stop parking")}
        >
          <Trash2 />
        </Button>
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1 px-4 pt-2.5 font-mono text-[11.5px] text-muted-foreground">
        <span>{dir.depth === 1 ? t("looks in its subfolders") : t("looks {n} levels deep", { n: dir.depth })}</span>
        <span>{dir.on_new === "register" ? t("new projects become sites") : t("new projects are suggested")}</span>
        <span>
          {dir.on_missing === "unregister" ? t("deleted projects are removed") : t("deleted projects are kept")}
        </span>
        {(dir.exclude ?? []).length > 0 && <span>{t("skips {patterns}", { patterns: dir.exclude!.join(", ") })}</span>}
      </div>
      {dir.watch_error && (
        <p className="px-4 pt-2 text-[12.5px] text-amber">
          {t("File events are unavailable ({error}); changes show up on the next rescan.", {
            error: dir.watch_error,
          })}
        </p>
      )}
      <div className="px-2 py-2">
        {dir.projects.length === 0 ? (
          <p className="px-2 py-2 text-[13px] text-muted-foreground">
            {dir.enabled ? t("No projects found yet.") : t("Turned off: not scanned or watched; its sites stay.")}
          </p>
        ) : (
          <ul className="flex flex-col">
            {dir.projects.map((project) => (
              <ProjectRow key={project.path} project={project} />
            ))}
          </ul>
        )}
        {dir.errors.length > 0 && (
          <ul className="mt-1 flex flex-col gap-0.5 px-2">
            {dir.errors.map((error) => (
              <li key={error.path} className="text-[12.5px] text-destructive">
                <span className="font-mono">{tildify(error.path)}</span>: {error.message}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function ProjectRow({ project }: { project: ParkedProject }) {
  const ignore = useParkIgnore();
  const add = useSaveProject();
  const { openSite } = useUi();
  const state = STATE[project.state];
  const isSite = project.state === "registered" || project.state === "manual";

  return (
    <li className="grid grid-cols-[minmax(0,1fr)_minmax(0,11rem)_10rem_8.5rem] items-center gap-3 rounded-md px-2 py-1.5 text-sm hover:bg-foreground/3">
      <span className="flex min-w-0 flex-col">
        <span className="truncate font-mono text-[12.5px]" title={project.path}>
          {project.rel}
        </span>
        <span className="truncate text-[11.5px] text-muted-foreground" title={project.note}>
          {project.framework}
          {project.note && <> · {project.note}</>}
        </span>
      </span>
      {isSite ? (
        <button
          type="button"
          className="truncate text-left font-mono text-[12.5px] text-ember hover:underline"
          onClick={() => openSite(project.name)}
        >
          {project.name}.test
        </button>
      ) : (
        <span className="truncate font-mono text-[12.5px] text-muted-foreground">
          {project.state === "suggested" ? `${project.name}.test` : "—"}
        </span>
      )}
      <span className="flex min-w-0 items-center gap-1.5 font-mono text-[11px] text-muted-foreground">
        <Led tone={state.tone} /> <span className="truncate">{state.label}</span>
      </span>
      <span className="flex justify-end gap-1">
        {project.state === "suggested" && (
          <Button
            size="xs"
            variant="outline"
            disabled={add.isPending}
            onClick={() =>
              add.mutate({ type: "add_project", path: project.path, name: project.name, auto_start: false })
            }
          >
            {t("Add")}
          </Button>
        )}
        {(project.state === "registered" || project.state === "suggested") && (
          <Button
            size="xs"
            variant="ghost"
            disabled={ignore.isPending}
            onClick={() => ignore.mutate({ path: project.path, ignore: true })}
          >
            {t("Ignore")}
          </Button>
        )}
        {project.state === "ignored" && (
          <Button
            size="xs"
            variant="ghost"
            disabled={ignore.isPending}
            onClick={() => ignore.mutate({ path: project.path, ignore: false })}
          >
            {t("Stop ignoring")}
          </Button>
        )}
      </span>
    </li>
  );
}

function Choice<T extends string | number>({
  value,
  options,
  onChange,
  label,
}: {
  value: T;
  options: { id: T; label: string }[];
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="grid auto-cols-fr grid-flow-col gap-2">
      {options.map((option) => (
        <button
          type="button"
          role="radio"
          key={option.id}
          aria-checked={value === option.id}
          onClick={() => onChange(option.id)}
          className={cn(
            "rounded-lg border px-3 py-2 text-left text-sm transition-colors hover:bg-foreground/3",
            value === option.id && "border-ember/60 bg-ember/6",
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** The value while it's set, then the last one, so a closing dialog keeps its content. */
function useLast<T>(value: T | null): T | null {
  const last = useRef(value);
  if (value !== null) last.current = value;
  return value ?? last.current;
}

function RulesDialog({ editing, onClose }: { editing: ParkedView | "new" | null; onClose: () => void }) {
  const shown = useLast(editing);
  // Remounting per target resets the form to that folder's rules.
  const key = shown === "new" ? "new" : (shown?.path ?? "none");
  return (
    <Dialog open={editing !== null} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg">
        {shown && <RulesForm key={key} dir={shown === "new" ? null : shown} onClose={onClose} />}
      </DialogContent>
    </Dialog>
  );
}

function RulesForm({ dir, onClose }: { dir: ParkedView | null; onClose: () => void }) {
  const set = useParkSet();
  const [path, setPath] = useState(dir?.path ?? "");
  const [depth, setDepth] = useState(dir?.depth ?? 1);
  const [onNew, setOnNew] = useState<OnNew>(dir?.on_new ?? "register");
  const [onMissing, setOnMissing] = useState<OnMissing>(dir?.on_missing ?? "unregister");
  const [exclude, setExclude] = useState((dir?.exclude ?? []).join(", "));

  const browse = async () => {
    const folder = await pickFolder(t("Choose the folder that holds your projects"));
    if (folder) setPath(folder);
  };
  const save = () =>
    set.mutate(
      {
        path: path.trim(),
        options: {
          depth,
          on_new: onNew,
          on_missing: onMissing,
          exclude: exclude
            .split(",")
            .map((p) => p.trim())
            .filter(Boolean),
        },
      },
      { onSuccess: onClose },
    );

  return (
    <>
      <DialogHeader>
        <DialogTitle>{dir ? t("Rules for {path}", { path: tildify(dir.path) }) : t("Park a folder")}</DialogTitle>
        <DialogDescription>
          {t("LocDev scans the folder now and whenever something changes in it. Nothing in it is modified.")}
        </DialogDescription>
      </DialogHeader>
      <div className="flex flex-col gap-4">
        {!dir && (
          <label className="flex flex-col gap-1.5 text-sm">
            <span className="text-muted-foreground">{t("Folder")}</span>
            <span className="flex gap-2">
              <Input
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="/home/you/Development"
                className="font-mono"
                autoFocus
              />
              <Button variant="outline" onClick={browse}>
                <FolderOpen /> {t("Browse…")}
              </Button>
            </span>
          </label>
        )}
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">{t("Look for projects")}</span>
          <Choice
            label={t("Look for projects")}
            value={depth}
            onChange={setDepth}
            options={[
              { id: 1, label: t("In its folders") },
              { id: 2, label: t("2 levels deep") },
              { id: 3, label: t("3 levels deep") },
            ]}
          />
        </div>
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">{t("When a new project appears")}</span>
          <Choice
            label={t("When a new project appears")}
            value={onNew}
            onChange={setOnNew}
            options={[
              { id: "register", label: t("Make it a site") },
              { id: "suggest", label: t("Suggest it") },
            ]}
          />
        </div>
        <div className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">{t("When a project's folder is deleted")}</span>
          <Choice
            label={t("When a project's folder is deleted")}
            value={onMissing}
            onChange={setOnMissing}
            options={[
              { id: "unregister", label: t("Remove its site") },
              { id: "keep", label: t("Keep it, marked missing") },
            ]}
          />
        </div>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-muted-foreground">{t("Skip folders")}</span>
          <Input
            value={exclude}
            onChange={(e) => setExclude(e.target.value)}
            placeholder="archive-*, clients/old"
            className="font-mono"
          />
          <span className="text-[12px] text-muted-foreground">
            {t("Names or paths inside the folder, separated by commas; * matches anything.")}
          </span>
        </label>
      </div>
      <DialogFooter>
        <Button variant="outline" onClick={onClose}>
          {t("Cancel")}
        </Button>
        <Button disabled={!path.trim() || set.isPending} onClick={save}>
          {dir ? t("Save") : t("Park")}
        </Button>
      </DialogFooter>
    </>
  );
}

function RemoveDialog({ dir: target, onClose }: { dir: ParkedView | null; onClose: () => void }) {
  const remove = useParkRemove();
  const [keep, setKeep] = useState(false);
  const dir = useLast(target);
  const sites = dir?.projects.filter((p) => p.state === "registered").length ?? 0;

  return (
    <Dialog open={!!target} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Led tone="fail" /> {t("Stop parking {path}?", { path: dir ? tildify(dir.path) : "" })}
          </DialogTitle>
          <DialogDescription>
            {keep
              ? t("Its sites stay in LocDev as regular sites.")
              : tn(
                  "Its {n} site is stopped and removed from LocDev.",
                  "Its {n} sites are stopped and removed from LocDev.",
                  sites,
                )}{" "}
            {t("No folder or file is touched; sites you added by hand stay.")}
          </DialogDescription>
        </DialogHeader>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox checked={keep} onCheckedChange={(checked) => setKeep(checked === true)} />
          {t("Keep its sites as regular sites")}
        </label>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button
            variant="destructive"
            disabled={remove.isPending}
            onClick={() => {
              if (target) remove.mutate({ path: target.path, keep_sites: keep }, { onSuccess: onClose });
            }}
          >
            {t("Stop parking")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
