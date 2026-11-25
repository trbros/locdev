import { FolderOpen } from "lucide-react";
import { type ReactNode, type SyntheticEvent, useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useDebounced, useProjectAction, useProjects, useRuntimes, useSaveProject } from "@/hooks/use-daemon";
import { useDetectFolder } from "@/hooks/use-stack";
import { isMock } from "@/lib/daemon";
import { type ProjectView, RUNTIME_LABEL, type RuntimeInfo, type RuntimeKind, releaseLine } from "@/lib/protocol";
import { pickFolder } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";
import { SwitchField } from "@/components/ui/switch-field";

const slug = (path: string) =>
  (path.split(/[\\/]/).filter(Boolean).pop() ?? "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

function parsePort(raw: string): number | undefined | "invalid" {
  if (!raw.trim()) return undefined;
  const port = Number(raw);
  return Number.isInteger(port) && port > 0 && port < 65536 ? port : "invalid";
}

/** Add-site dialog, driven by `addSiteOpen` in the UI state. */
export function AddSiteDialog() {
  const { addSiteOpen, setAddSiteOpen, navigate, select } = useUi();
  const save = useSaveProject();
  const action = useProjectAction();

  const [path, setPath] = useState("");
  const detected = useDetectFolder(useDebounced(path.trim(), 400) || null);
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [port, setPort] = useState("");
  const [autoStart, setAutoStart] = useState(false);
  const [startNow, setStartNow] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (addSiteOpen) {
      setPath("");
      setName("");
      setCommand("");
      setPort("");
      setAutoStart(false);
      setStartNow(true);
      setError(null);
    }
  }, [addSiteOpen]);

  const browse = async () => {
    const folder = await pickFolder();
    if (folder) setPath(folder);
    else if (isMock) setPath("/home/you/Code/new-app");
  };

  const submit = async (e: SyntheticEvent) => {
    e.preventDefault();
    const parsedPort = parsePort(port);
    if (parsedPort === "invalid") return setError(t("Port must be a number between 1 and 65535."));
    if (!path.trim()) return setError(t("Choose the project folder."));

    try {
      const project = await save.mutateAsync({
        type: "add_project",
        path: path.trim(),
        name: name.trim() || undefined,
        command: command.trim() || undefined,
        port: parsedPort,
        auto_start: autoStart,
      });
      setAddSiteOpen(false);
      navigate("sites");
      select(project.name);
      toast.success(`Added ${project.domain}`, {
        description: project.command
          ? t("Runs: {command}", { command: project.command })
          : t("No dev server command was detected."),
      });
      if (startNow && project.command) action.mutate({ action: "start_project", name: project.name });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <Dialog open={addSiteOpen} onOpenChange={setAddSiteOpen}>
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={submit} className="grid gap-4">
          <DialogHeader>
            <DialogTitle>{t("Add site")}</DialogTitle>
            <DialogDescription>
              {t(
                "LocDev recognizes 40+ frameworks from the project's files (nothing is run) and suggests how to start them.",
              )}
            </DialogDescription>
          </DialogHeader>

          <Field label={t("Project folder")} htmlFor="site-path">
            <div className="flex gap-2">
              <Input
                id="site-path"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder={t("/home/you/Code/my-app")}
                className="font-mono text-xs"
                autoFocus
              />
              <Button type="button" variant="outline" onClick={browse}>
                <FolderOpen data-icon="inline-start" /> {t("Browse")}
              </Button>
            </div>
          </Field>

          <div className="grid grid-cols-[1fr_7rem] gap-3">
            <Field
              label={t("Name")}
              htmlFor="site-name"
              hint={`${(name.trim() || slug(path) || "name").toLowerCase()}.test`}
            >
              <Input
                id="site-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={slug(path) || "my-app"}
              />
            </Field>
            <Field label={t("Port")} htmlFor="site-port" hint={t("auto if empty")}>
              <Input
                id="site-port"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                placeholder={t("auto")}
                inputMode="numeric"
              />
            </Field>
          </div>

          <Field
            label={t("Dev server command")}
            htmlFor="site-command"
            hint={t("Leave empty to auto-detect. {port} is replaced with the site's port.")}
          >
            <Input
              id="site-command"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder={detected.data?.stack.command ?? t("detected automatically")}
              className="font-mono text-xs"
            />
          </Field>
          {detected.data && (
            <p className="-mt-2 text-[12.5px] text-muted-foreground">
              {detected.data.stack.components.length === 0
                ? t("Nothing recognized in this folder; set the command yourself.")
                : `Detected: ${detected.data.stack.framework}${
                    detected.data.stack.command ? "" : " (no start command it can build; set one)"
                  }`}
              {detected.data.stack.workspaces.length > 0 &&
                t(" · a monorepo with {length} packages; you may want to add one of them instead", {
                  length: detected.data.stack.workspaces.length,
                })}
            </p>
          )}

          <div className="grid gap-2.5">
            <Toggle checked={startNow} onChange={setStartNow} label={t("Start it now")} />
            <Toggle checked={autoStart} onChange={setAutoStart} label={t("Start with LocDev")} />
          </div>

          {error && <p className="rounded-lg bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setAddSiteOpen(false)}>
              {t("Cancel")}
            </Button>
            <Button
              type="submit"
              disabled={save.isPending}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
            >
              {save.isPending ? t("Adding…") : t("Add site")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

/** Edit dialog for the project named in `editing`. */
export function EditSiteDialog() {
  const { editing, setEditing } = useUi();
  const projects = useProjects();
  const site = projects.data?.find((p) => p.name === editing);

  return (
    <Dialog open={!!editing} onOpenChange={(open) => !open && setEditing(null)}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="font-mono">{site?.domain ?? editing}</DialogTitle>
          <DialogDescription className="truncate font-mono text-xs">{site?.path}</DialogDescription>
        </DialogHeader>
        {site && <SiteSettingsForm site={site} onDone={() => setEditing(null)} />}
      </DialogContent>
    </Dialog>
  );
}

/**
 * A site's settings: dev server command, port, runtime and package manager
 * pins, startup. Used by the edit dialog and the site's Settings tab.
 */
export function SiteSettingsForm({ site, onDone }: { site: ProjectView; onDone?: () => void }) {
  const save = useSaveProject();
  const [command, setCommand] = useState("");
  const [port, setPort] = useState("");
  const [autoStart, setAutoStart] = useState(false);
  const [restart, setRestart] = useState(false);
  // "" means "use the default".
  const [pins, setPins] = useState<Partial<Record<RuntimeKind, string>>>({});
  const [error, setError] = useState<string | null>(null);
  const runtimes = useRuntimes();

  const load = () => {
    setCommand(site.command ?? "");
    setPort(site.port ? String(site.port) : "");
    setAutoStart(site.auto_start);
    setRestart(!!site.restart_on_failure);
    setPins({ ...site.tools, node: site.node ?? "", php: site.php ?? "" });
    setError(null);
  };
  // Load values once per site, not on every poll.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(load, [site.name]);

  const dirty =
    command !== (site.command ?? "") ||
    port !== (site.port ? String(site.port) : "") ||
    autoStart !== site.auto_start ||
    restart !== !!site.restart_on_failure ||
    (pins.node ?? "") !== (site.node ?? "") ||
    (pins.php ?? "") !== (site.php ?? "") ||
    TOOL_PINS.some((k) => (pins[k] ?? "") !== (site.tools?.[k] ?? ""));

  const submit = async (e: SyntheticEvent) => {
    e.preventDefault();
    const parsedPort = parsePort(port);
    if (parsedPort === "invalid") return setError(t("Port must be a number between 1 and 65535."));

    try {
      await save.mutateAsync({
        type: "update_project",
        name: site.name,
        command: command.trim(),
        port: parsedPort,
        auto_start: autoStart,
        restart_on_failure: restart,
        node: pins.node ?? "",
        php: pins.php ?? "",
        tools: Object.fromEntries(
          TOOL_PINS.filter((k) => (pins[k] ?? "") !== (site.tools?.[k] ?? "")).map((k) => [k, pins[k] ?? ""]),
        ),
      });
      setError(null);
      onDone?.();
      toast.success(`Saved ${site.domain}`, {
        description: site.process.state === "running" ? t("Restart the site to apply the changes.") : undefined,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <form onSubmit={submit} className="grid gap-4">
      <Field
        label={t("Dev server command")}
        htmlFor={`${site.name}-command`}
        hint={t("{port} is replaced with the site's port. Empty disables starting.")}
      >
        <Input
          id={`${site.name}-command`}
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          className="font-mono text-xs"
        />
      </Field>
      <div className="grid grid-cols-[7rem_1fr_1fr] gap-3">
        <Field label={t("Port")} htmlFor={`${site.name}-port`}>
          <Input id={`${site.name}-port`} value={port} onChange={(e) => setPort(e.target.value)} inputMode="numeric" />
        </Field>
        {runtimes.data
          ?.filter((info) => info.category === "runtime")
          .map((info) => (
            <Field key={info.kind} label={RUNTIME_LABEL[info.kind]} htmlFor={`${site.name}-${info.kind}`}>
              <PinSelect
                id={`${site.name}-${info.kind}`}
                info={info}
                value={pins[info.kind] ?? ""}
                onChange={(pin) => setPins((p) => ({ ...p, [info.kind]: pin }))}
              />
            </Field>
          ))}
      </div>
      <div className="grid grid-cols-4 gap-3">
        {runtimes.data
          ?.filter((info) => TOOL_PINS.includes(info.kind))
          .map((info) => (
            <Field key={info.kind} label={RUNTIME_LABEL[info.kind]} htmlFor={`${site.name}-${info.kind}`}>
              <PinSelect
                id={`${site.name}-${info.kind}`}
                info={info}
                value={pins[info.kind] ?? ""}
                onChange={(pin) => setPins((p) => ({ ...p, [info.kind]: pin }))}
              />
            </Field>
          ))}
      </div>
      {site.requirements.length > 0 && (
        <p className="text-xs text-muted-foreground">
          {tx("The project asks for {requirements}. Without a pin, those are followed.", {
            requirements: site.requirements.map((r, i) => (
              <span key={`${r.kind}-${r.file}`}>
                {i > 0 && ", "}
                <span className="font-mono text-foreground">
                  {r.kind} {r.constraint}
                </span>{" "}
                ({r.file})
              </span>
            )),
          })}
        </p>
      )}
      <div className="flex flex-wrap gap-x-6 gap-y-2">
        <Toggle checked={autoStart} onChange={setAutoStart} label={t("Start with LocDev")} />
        <Toggle checked={restart} onChange={setRestart} label={t("Restart if it fails (up to 5 times in 2 min)")} />
      </div>

      {error && <p className="rounded-lg bg-destructive/10 px-3 py-2 text-sm text-destructive">{error}</p>}

      <div className="flex justify-end gap-2">
        {onDone ? (
          <Button type="button" variant="outline" onClick={onDone}>
            {t("Cancel")}
          </Button>
        ) : (
          <Button type="button" variant="ghost" disabled={!dirty} onClick={load}>
            {t("Discard changes")}
          </Button>
        )}
        <Button
          type="submit"
          disabled={save.isPending || (!onDone && !dirty)}
          className="bg-ember text-ember-foreground hover:bg-ember/85"
        >
          {t("Save")}
        </Button>
      </div>
    </form>
  );
}

function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-1.5">
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {hint && <p className="truncate text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <SwitchField
      className="flex items-center gap-2.5 text-sm"
      checked={checked}
      onCheckedChange={onChange}
      switchClassName="data-checked:bg-ember"
    >
      {label}
    </SwitchField>
  );
}

const DEFAULT_PIN = "default";

/** Package managers a site can pin (npm follows Node.js). */
const TOOL_PINS: RuntimeKind[] = ["pnpm", "yarn", "bun", "composer"];

/**
 * Picks a runtime pin: the default, a release line ("22" or "8.3", the newest
 * installed of that line) or an exact version.
 */
function PinSelect({
  id,
  info,
  value,
  onChange,
}: {
  id: string;
  info: RuntimeInfo;
  value: string;
  onChange: (pin: string) => void;
}) {
  const lines = [...new Set(info.versions.map((v) => releaseLine(info.kind, v.version)))];
  const items = [
    { value: DEFAULT_PIN, label: `Default · ${info.default ?? "PATH"}` },
    ...lines.map((line) => ({
      value: line,
      label: t("{line} · newest {line}.x", { line }),
    })),
    ...info.versions.map((v) => ({ value: v.version, label: v.version })),
  ];
  // Keep a pin that no longer matches anything visible and selectable.
  if (value && !items.some((i) => i.value === value)) {
    items.push({ value, label: t("{value} · not installed", { value }) });
  }

  return (
    <Select
      items={items}
      value={value || DEFAULT_PIN}
      onValueChange={(next) => onChange(next === DEFAULT_PIN || !next ? "" : String(next))}
    >
      <SelectTrigger id={id} className="w-full font-mono text-xs">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {items.map((item) => (
          <SelectItem key={item.value} value={item.value} className="font-mono text-xs">
            {item.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
