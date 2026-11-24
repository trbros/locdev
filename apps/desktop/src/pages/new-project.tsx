import {
  ArrowUpRight,
  Check,
  CircleAlert,
  FileCode,
  FolderOpen,
  Loader2,
  Package,
  Play,
  Plus,
  RefreshCw,
  Search,
  Square,
  TerminalSquare,
  X,
} from "lucide-react";
import { type ReactNode, useEffect, useMemo, useState } from "react";

import { CodeView, languageForPath } from "@/components/code-editor";
import { Page, Section, useCrumbs } from "@/components/page";
import { PtyTerminal } from "@/components/pty-terminal";
import { StackIcon } from "@/components/stack-icons/stack-icon";
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
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useProjectAction, useProjects, useRuntimeActions, useRuntimes } from "@/hooks/use-daemon";
import {
  useCreateProject,
  useInstallTool,
  usePackageSearch,
  usePlan,
  useStarters,
  useTools,
} from "@/hooks/use-scaffold";
import { useServices } from "@/hooks/use-services";
import { backend, isMock } from "@/lib/daemon";
import { subById, viewById } from "@/lib/nav";
import {
  type DatabaseChoice,
  type NewProject,
  releaseLine,
  RUNTIME_LABEL,
  type RuntimeKind,
  type Starter,
  type StarterOption,
  starterAsks,
  stepCommand,
  type TerminalInfo,
  type Tool,
} from "@/lib/protocol";
import { openInBrowser, pickFolder } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { intlLocale, t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const FAMILIES: { id: string; label: string }[] = [
  { id: "php", label: t("PHP") },
  { id: "javascript", label: t("JavaScript") },
  { id: "python", label: t("Python") },
  { id: "rust", label: t("Rust") },
  { id: "ruby", label: t("Ruby") },
  { id: "dotnet", label: t(".NET") },
];

const DATABASES: { value: DatabaseChoice["service"]; label: string }[] = [
  { value: "postgres", label: t("PostgreSQL") },
  { value: "mysql", label: t("MySQL") },
  { value: "mariadb", label: t("MariaDB") },
];

/** The folder new projects go in: next to the sites already registered. */
function useDefaultParent(): string {
  const projects = useProjects();
  const first = projects.data?.[0]?.path;
  if (!first) return isMock ? "/home/you/Code" : "";
  return first.replace(/[\\/][^\\/]+[\\/]?$/, "");
}

export function NewProjectView() {
  const view = viewById("tools");
  const sub = subById(view, "new-project")!;
  const crumbs = useCrumbs(view, sub);
  const starters = useStarters();
  const tools = useTools();
  const defaultParent = useDefaultParent();
  const [starterId, setStarterId] = useState<string | null>(null);
  const [run, setRun] = useState<{ project: NewProject; info: TerminalInfo } | null>(null);

  const starter = starters.data?.find((s) => s.id === starterId) ?? null;

  return (
    <Page view={view} title={sub.label} description={sub.description} crumbs={crumbs} wide="full">
      {run ? (
        <RunPanel run={run} onInfo={(info) => setRun({ ...run, info })} onDone={() => setRun(null)} />
      ) : !starters.data ? (
        <Skeleton className="h-64" />
      ) : (
        <>
          <Picker starters={starters.data} selected={starterId} onSelect={setStarterId} />
          {starter && (
            <Form
              key={starter.id}
              starter={starter}
              tools={tools.data ?? []}
              defaultParent={defaultParent}
              onStarted={(project, info) => setRun({ project, info })}
            />
          )}
        </>
      )}
    </Page>
  );
}

function Picker({
  starters,
  selected,
  onSelect,
}: {
  starters: Starter[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <Section title={t("framework")}>
      <div className="flex flex-col gap-4 pt-1">
        {FAMILIES.map((family) => {
          const items = starters.filter((s) => s.family === family.id);
          if (!items.length) return null;
          return (
            <div key={family.id} className="flex flex-col gap-2">
              <span className="font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">
                {family.label}
              </span>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(13.5rem,1fr))] gap-2">
                {items.map((s) => (
                  <button
                    type="button"
                    key={s.id}
                    aria-pressed={selected === s.id}
                    onClick={() => onSelect(s.id)}
                    className={cn(
                      "flex items-start gap-3 rounded-lg border bg-card p-3 text-left transition-colors hover:border-ember/50",
                      selected === s.id && "border-ember bg-ember/6",
                    )}
                  >
                    <StackIcon id={s.framework} className="mt-0.5 size-6 shrink-0" />
                    <span className="min-w-0">
                      <span className="block text-[13.5px] font-medium">{s.name}</span>
                      <span className="line-clamp-2 text-[11.5px] text-muted-foreground">{s.description}</span>
                    </span>
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </Section>
  );
}

function Form({
  starter,
  tools,
  defaultParent,
  onStarted,
}: {
  starter: Starter;
  tools: Tool[];
  defaultParent: string;
  onStarted: (project: NewProject, info: TerminalInfo) => void;
}) {
  const { navigate } = useUi();
  const services = useServices();
  const create = useCreateProject();
  const toolOf = (id: string) => tools.find((tool) => tool.id === id);
  // The first method whose tool is installed.
  const [method, setMethod] = useState(
    () => starter.methods.find((m) => toolOf(m.tool)?.installed)?.id ?? starter.methods[0].id,
  );
  const [name, setName] = useState("");
  const [parent, setParent] = useState(defaultParent);
  const [options, setOptions] = useState<Record<string, string>>({});
  const [packages, setPackages] = useState<string[]>([]);
  const [database, setDatabase] = useState<DatabaseChoice | null>(null);
  const [register, setRegister] = useState(true);
  // The runtime the installer runs with ("" = LocDev's default); the site is pinned to it.
  const runtimeKind = starter.family === "php" ? "php" : starter.family === "javascript" ? "node" : null;
  const [version, setVersion] = useState("");
  const runtimes = useRuntimes();
  const runtime = runtimes.data?.find((r) => r.kind === runtimeKind);
  const lines = [...new Set((runtime?.versions ?? []).map((v) => releaseLine(runtime!.kind, v.version)))];

  useEffect(() => {
    if (!parent && defaultParent) setParent(defaultParent);
  }, [defaultParent, parent]);

  const project: NewProject = {
    starter: starter.id,
    method,
    name,
    parent,
    options,
    packages,
    database,
    register,
    node: runtimeKind === "node" && version ? version : null,
    php: runtimeKind === "php" && version ? version : null,
  };
  const ready = name.length > 0 && parent.length > 0;
  const plan = usePlan(ready ? project : null);
  const visible = starter.options.filter((o) => starterAsks(starter, o, method, options));
  const instances = (services.data?.instances ?? []).filter((i) => i.kind === database?.service);

  const start = async () => {
    const info = await create.mutateAsync(project);
    onStarted(project, info);
  };

  return (
    <div className="grid grid-cols-1 gap-8 xl:grid-cols-[minmax(0,1fr)_minmax(0,26rem)]">
      <div className="flex flex-col gap-6">
        <Section title={t("{name} project", { name: starter.name })}>
          <div className="flex flex-col gap-3 pt-1">
            <Row label={t("Name")} hint={t("The folder and the site: lowercase letters, digits, - and _.")}>
              <Input
                autoFocus
                value={name}
                onChange={(e) => setName(e.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, "-"))}
                placeholder={t("my-app")}
                aria-label={t("Project name")}
                className="h-8 max-w-72 font-mono text-[13px]"
              />
            </Row>
            <Row
              label={t("Folder")}
              hint={parent && name ? `${parent.replace(/[\\/]$/, "")}/${name}` : t("Where it's created.")}
            >
              <span className="flex max-w-xl gap-2">
                <Input
                  value={parent}
                  onChange={(e) => setParent(e.target.value)}
                  aria-label={t("Parent folder")}
                  placeholder={t("/home/you/Code")}
                  className="h-8 font-mono text-[12.5px]"
                />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={async () => {
                    const dir = await pickFolder();
                    if (dir) setParent(dir);
                  }}
                >
                  <FolderOpen data-icon="inline-start" /> {t("Choose…")}
                </Button>
              </span>
            </Row>
          </div>
        </Section>

        <Section title={t("created with")}>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-2 pt-1">
            {starter.methods.map((m) => {
              const tool = toolOf(m.tool);
              return (
                <button
                  type="button"
                  key={m.id}
                  aria-pressed={method === m.id}
                  onClick={() => setMethod(m.id)}
                  className={cn(
                    "flex flex-col gap-1 rounded-lg border p-3 text-left transition-colors hover:border-ember/50",
                    method === m.id && "border-ember bg-ember/6",
                  )}
                >
                  <span className="flex items-center gap-2 text-[13px] font-medium">
                    <Led tone={tool?.installed ? "live" : "warn"} />
                    {m.label}
                  </span>
                  <span className="font-mono text-[11px] text-muted-foreground">
                    {tool?.installed
                      ? (tool.version ?? t("installed"))
                      : t("{name} not installed", { name: tool?.name ?? m.tool })}
                  </span>
                  {m.note && <span className="text-[11.5px] text-muted-foreground">{m.note}</span>}
                </button>
              );
            })}
          </div>
          {starter.methods.some((m) => !toolOf(m.tool)?.installed) && (
            <button
              type="button"
              onClick={() => navigate("tools", "installers")}
              className="mt-2 self-start text-[12px] text-ember underline-offset-2 hover:underline"
            >
              {t("Manage installers →")}
            </button>
          )}
        </Section>

        {runtimeKind && (
          <Section title={t("runtime")}>
            <div className="flex flex-col pt-1">
              <Row
                label={RUNTIME_LABEL[runtimeKind]}
                hint={t("The version the installer runs with; the new site is pinned to it.")}
              >
                <Select
                  items={[
                    { value: "", label: t("Default ({version})", { version: runtime?.default ?? t("system") }) },
                    ...lines.map((line) => ({ value: line, label: line })),
                  ]}
                  value={version}
                  onValueChange={(v) => setVersion(v ?? "")}
                >
                  <SelectTrigger size="sm" aria-label={RUNTIME_LABEL[runtimeKind]} className="h-8 w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">
                      {t("Default ({version})", { version: runtime?.default ?? t("system") })}
                    </SelectItem>
                    {lines.map((line) => (
                      <SelectItem key={line} value={line}>
                        {line}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Row>
            </div>
          </Section>
        )}

        {visible.length > 0 && (
          <Section title={t("options")}>
            <div className="flex flex-col pt-1">
              {visible.map((o) => (
                <OptionRow
                  key={o.id}
                  option={o}
                  value={options[o.id] ?? o.default}
                  onChange={(v) => setOptions({ ...options, [o.id]: v })}
                  locked={o.id === "database" && database !== null ? t("Set by the LocDev database below") : null}
                />
              ))}
            </div>
          </Section>
        )}

        {starter.database && (
          <Section title={t("database")}>
            <div className="flex flex-col gap-3 pt-1">
              <Row
                label={t("LocDev database")}
                hint={t("An instance LocDev runs; the project's .env is pointed at it and the migrations run.")}
              >
                <Select
                  items={[{ value: "none", label: t("None") }, ...DATABASES]}
                  value={database?.service ?? "none"}
                  onValueChange={(v) => {
                    if (!v || v === "none") return setDatabase(null);
                    const service = v as DatabaseChoice["service"];
                    const existing = (services.data?.instances ?? []).find((i) => i.kind === service);
                    setDatabase({ service, instance: existing?.name ?? "db" });
                  }}
                >
                  <SelectTrigger size="sm" aria-label={t("LocDev database")} className="h-8 w-48">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {[{ value: "none", label: t("None") }, ...DATABASES].map((d) => (
                      <SelectItem key={d.value} value={d.value}>
                        {d.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </Row>
              {database && (
                <Row
                  label={t("Instance")}
                  hint={
                    instances.some((i) => i.name === database.instance)
                      ? t("Existing instance; it's started if it's stopped.")
                      : "A new instance, created with the newest installed version."
                  }
                >
                  <Input
                    value={database.instance}
                    onChange={(e) => setDatabase({ ...database, instance: e.target.value })}
                    list="instances"
                    aria-label={t("Instance name")}
                    className="h-8 max-w-48 font-mono text-[12.5px]"
                  />
                  <datalist id="instances">
                    {instances.map((i) => (
                      <option key={i.name} value={i.name} />
                    ))}
                  </datalist>
                </Row>
              )}
            </div>
          </Section>
        )}

        {starter.packages && <Packages registry={starter.packages} value={packages} onChange={setPackages} />}

        <Section title={t("site")}>
          <label className="flex items-start gap-3 pt-1">
            <Switch checked={register} onCheckedChange={setRegister} className="mt-0.5 data-checked:bg-ember" />
            <span>
              <span className="block text-sm">{t("Register it as a site when it's done")}</span>
              <span className="block text-[12px] text-muted-foreground">
                {tx("Detected like any folder you add: dev command, port and {name}.test.", {
                  name: name || t("<name>"),
                })}
              </span>
            </span>
          </label>
        </Section>
      </div>

      <aside className="flex flex-col gap-3 xl:sticky xl:top-0 xl:self-start">
        <Section title={t("what will run")}>
          {!ready ? (
            <p className="pt-1 text-[12.5px] text-muted-foreground">
              {t("Give it a name and a folder to see the commands.")}
            </p>
          ) : plan.error ? (
            <p className="flex gap-2 pt-1 text-[12.5px] text-destructive">
              <CircleAlert className="mt-0.5 size-3.5 shrink-0" />
              {plan.error.message}
            </p>
          ) : !plan.data ? (
            <Skeleton className="h-32" />
          ) : (
            <div className="flex flex-col gap-2 pt-1">
              {(plan.data.files ?? []).length > 0 && (
                <div className="flex flex-col gap-1">
                  <span className="text-[12px] text-muted-foreground">
                    {t("LocDev writes its starter files first:")}
                  </span>
                  {plan.data.files!.map((file) => (
                    <details key={file.path} className="group rounded-md border">
                      <summary className="flex cursor-default items-center gap-2 px-2.5 py-1.5 font-mono text-[11.5px] select-none">
                        <FileCode className="size-3.5 shrink-0 text-ember" />
                        {file.path}
                        <span className="ml-auto text-muted-foreground">
                          {tn("{n} line", "{n} lines", file.contents.split("\n").filter(Boolean).length)}
                        </span>
                      </summary>
                      <CodeView
                        value={file.contents}
                        language={languageForPath(file.path)}
                        className="max-h-64 rounded-none border-0 border-t"
                        ariaLabel={file.path}
                      />
                    </details>
                  ))}
                </div>
              )}
              <ol className="flex flex-col gap-2">
                {[...plan.data.steps, ...plan.data.finish].map((s, i) => (
                  <li key={i} className="flex flex-col gap-1">
                    <span className="text-[12px] text-muted-foreground">
                      {i + 1}. {s.label}
                    </span>
                    <code
                      className="rounded-md bg-well px-2.5 py-1.5 font-mono text-[11.5px] break-all"
                      data-selectable
                    >
                      <span className="text-muted-foreground">$ </span>
                      {stepCommand(s)}
                    </code>
                  </li>
                ))}
              </ol>
              {plan.data.actions.length > 0 && (
                <ul className="flex flex-col gap-1 border-t border-dashed pt-2">
                  {plan.data.actions.map((a) => (
                    <li key={a} className="flex gap-2 text-[12px] text-muted-foreground">
                      <Check className="mt-0.5 size-3 shrink-0 text-phosphor" />
                      {a}
                    </li>
                  ))}
                </ul>
              )}
              {plan.data.notes.map((n) => (
                <p key={n} className="text-[12px] text-muted-foreground">
                  {n}
                </p>
              ))}
              {plan.data.missing.length > 0 && (
                <p className="flex gap-2 rounded-md bg-amber/10 px-2.5 py-2 text-[12px] text-amber">
                  <CircleAlert className="mt-0.5 size-3.5 shrink-0" />
                  {tx("Install {tools} first (Tools → Installers).", { tools: plan.data.missing.join(", ") })}
                </p>
              )}
            </div>
          )}
        </Section>
        <p className="text-[11.5px] text-muted-foreground">
          {t(
            "The framework's own installer runs in a terminal here: you see its output and can answer what it asks. Nothing runs before you click Create, and an existing folder is never touched.",
          )}
        </p>
        <Button
          onClick={start}
          disabled={!plan.data || !!plan.error || plan.isFetching || plan.data.missing.length > 0 || create.isPending}
          className="self-start bg-ember text-ember-foreground hover:bg-ember/85"
        >
          {create.isPending ? (
            <Loader2 data-icon="inline-start" className="animate-spin" />
          ) : (
            <Play data-icon="inline-start" />
          )}
          {t("Create project")}
        </Button>
      </aside>
    </div>
  );
}

function Row({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[9rem_minmax(0,1fr)] items-start gap-3">
      <span className="pt-1.5 text-[13px] text-muted-foreground">{label}</span>
      <div className="flex flex-col gap-1">
        {children}
        {hint && <span className="text-[11.5px] text-muted-foreground">{hint}</span>}
      </div>
    </div>
  );
}

function OptionRow({
  option,
  value,
  onChange,
  locked,
}: {
  option: StarterOption;
  value: string;
  onChange: (v: string) => void;
  locked: string | null;
}) {
  return (
    <div className="flex items-center gap-3 border-b border-dashed py-2 last:border-b-0">
      <span className="min-w-0 flex-1">
        <span className="block text-[13px]">{option.label}</span>
        {(locked ?? option.help) && (
          <span className="block text-[11.5px] text-muted-foreground">{locked ?? option.help}</span>
        )}
      </span>
      {option.choices.length === 0 ? (
        <Switch
          checked={value === "true"}
          onCheckedChange={(on) => onChange(String(on))}
          aria-label={option.label}
          className="data-checked:bg-ember"
        />
      ) : (
        <Select
          items={option.choices}
          value={value}
          onValueChange={(v) => v && onChange(String(v))}
          disabled={!!locked}
        >
          <SelectTrigger size="sm" aria-label={option.label} className="h-7 min-w-44 text-[12.5px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {option.choices.map((c) => (
              <SelectItem key={c.value} value={c.value} className="text-[12.5px]">
                {c.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
    </div>
  );
}

function Packages({
  registry,
  value,
  onChange,
}: {
  registry: "npm" | "packagist";
  value: string[];
  onChange: (v: string[]) => void;
}) {
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(query), 350);
    return () => window.clearTimeout(timer);
  }, [query]);
  const search = usePackageSearch(registry, debounced);
  const label = registry === "npm" ? "npm" : "Packagist";
  return (
    <Section title={t("packages · {label}", { label })}>
      <div className="flex flex-col gap-2 pt-1">
        <p className="text-[12px] text-muted-foreground">
          {tx(
            "Added after the project is created, with {manager}. Scripts the packages ship run as the manager decides.",
            { manager: registry === "npm" ? t("the chosen package manager") : t("composer require") },
          )}
        </p>
        {value.length > 0 && (
          <div className="flex flex-wrap gap-1.5">
            {value.map((p) => (
              <span
                key={p}
                className="flex items-center gap-1 rounded-md border bg-well py-0.5 pr-1 pl-2 font-mono text-[11.5px]"
              >
                {p}
                <button
                  type="button"
                  aria-label={t("Remove {p}", { p: p })}
                  onClick={() => onChange(value.filter((x) => x !== p))}
                  className="rounded p-0.5 text-muted-foreground hover:text-foreground"
                >
                  <X className="size-3" />
                </button>
              </span>
            ))}
          </div>
        )}
        <span className="relative max-w-md">
          <Search className="pointer-events-none absolute top-2 left-2.5 size-3.5 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("Search {label}…", { label })}
            aria-label={t("Search {label}", { label: label })}
            className="h-8 pl-8 text-[13px]"
          />
        </span>
        {search.error && <p className="text-[12px] text-destructive">{search.error.message}</p>}
        {search.isFetching && !search.data && <Skeleton className="h-16 max-w-xl" />}
        {search.data && debounced.length >= 2 && (
          <ul className="flex max-w-xl flex-col divide-y rounded-md border">
            {search.data.length === 0 && (
              <li className="px-3 py-2 text-[12px] text-muted-foreground">{t("No packages found.")}</li>
            )}
            {search.data.slice(0, 8).map((hit) => (
              <li key={hit.name} className="flex items-center gap-3 px-3 py-2">
                <Package className="size-3.5 shrink-0 text-muted-foreground" />
                <span className="min-w-0 flex-1">
                  <span className="flex items-baseline gap-2">
                    <span className="truncate font-mono text-[12.5px]">{hit.name}</span>
                    {hit.version && <span className="font-mono text-[11px] text-muted-foreground">{hit.version}</span>}
                    {hit.downloads != null && (
                      <span className="font-mono text-[10.5px] text-muted-foreground">
                        {Intl.NumberFormat(intlLocale, { notation: "compact" }).format(hit.downloads)} {t("dl")}
                      </span>
                    )}
                  </span>
                  {hit.description && (
                    <span className="block truncate text-[11.5px] text-muted-foreground">{hit.description}</span>
                  )}
                </span>
                {hit.url && (
                  <button
                    type="button"
                    aria-label={t("{name} on {label}", { name: hit.name, label })}
                    onClick={() => openInBrowser(hit.url!)}
                    className="text-muted-foreground hover:text-foreground"
                  >
                    <ArrowUpRight className="size-3.5" />
                  </button>
                )}
                <Button
                  size="xs"
                  variant="outline"
                  disabled={value.includes(hit.name)}
                  onClick={() => onChange([...value, hit.name])}
                >
                  {value.includes(hit.name) ? <Check data-icon="inline-start" /> : <Plus data-icon="inline-start" />}
                  {value.includes(hit.name) ? t("Added") : t("Add")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Section>
  );
}

function RunPanel({
  run,
  onInfo,
  onDone,
}: {
  run: { project: NewProject; info: TerminalInfo };
  onInfo: (info: TerminalInfo) => void;
  onDone: () => void;
}) {
  const { openSite } = useUi();
  const action = useProjectAction();
  const { info } = run;
  const ok = !info.running && info.code === 0;
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3">
        <TerminalSquare className="size-4 text-muted-foreground" />
        <span className="text-sm font-medium">{info.title}</span>
        <span className="font-mono text-[11.5px] text-muted-foreground">
          {info.running
            ? t("step {step} of {steps}", { step: info.step, steps: info.steps })
            : ok
              ? t("done")
              : info.code == null
                ? t("stopped")
                : t("failed (exit {code})", { code: info.code })}
        </span>
        <Led tone={info.running ? "ember" : ok ? "live" : "fail"} />
        <span className="ml-auto flex gap-2">
          {info.running ? (
            <Button
              variant="outline"
              size="sm"
              onClick={() => void backend.request({ type: "terminal_kill", id: info.id })}
            >
              <Square data-icon="inline-start" /> {t("Stop")}
            </Button>
          ) : (
            <>
              {info.site && (
                <>
                  <Button size="sm" onClick={() => openSite(info.site!)}>
                    {t("Open site")}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => action.mutate({ action: "start_project", name: info.site! })}
                  >
                    <Play data-icon="inline-start" /> {t("Start it")}
                  </Button>
                </>
              )}
              <Button size="sm" variant="ghost" onClick={onDone}>
                {t("New project")}
              </Button>
            </>
          )}
        </span>
      </div>
      <PtyTerminal id={info.id} onInfo={onInfo} className="h-[30rem]" />
      <p className="text-[11.5px] text-muted-foreground">
        {t(
          "Type here to answer the installer's questions. Lines starting with › are LocDev's own steps (database, site).",
        )}
      </p>
    </div>
  );
}

export function InstallersView() {
  const view = viewById("tools");
  const sub = subById(view, "installers")!;
  const crumbs = useCrumbs(view, sub);
  const tools = useTools();
  const install = useInstallTool();
  const runtime = useRuntimeActions();
  const { navigate } = useUi();
  const [confirm, setConfirm] = useState<Tool | null>(null);
  const [terminal, setTerminal] = useState<TerminalInfo | null>(null);
  const sorted = useMemo(
    () => [...(tools.data ?? [])].sort((a, b) => Number(a.installed) - Number(b.installed)),
    [tools.data],
  );

  return (
    <Page
      view={view}
      title={sub.label}
      description={sub.description}
      crumbs={crumbs}
      wide
      actions={
        <Button variant="outline" size="sm" onClick={() => tools.refetch()} disabled={tools.isFetching}>
          <RefreshCw data-icon="inline-start" className={cn(tools.isFetching && "animate-spin")} /> {t("Check again")}
        </Button>
      }
    >
      {!tools.data ? (
        <Skeleton className="h-64" />
      ) : (
        <Section title={t("tools")}>
          <ul className="flex flex-col">
            {sorted.map((tool) => (
              <li key={tool.id} className="flex items-center gap-3 border-b border-dashed py-2.5 last:border-b-0">
                <Led tone={tool.installed ? "live" : "warn"} />
                <span className="min-w-0 flex-1">
                  <span className="flex items-baseline gap-2">
                    <span className="text-sm font-medium">{tool.name}</span>
                    <span className="truncate font-mono text-[11.5px] text-muted-foreground">
                      {tool.installed ? (tool.version ?? t("installed")) : t("not installed")}
                    </span>
                  </span>
                  <span className="block text-[12px] text-muted-foreground">
                    {tool.description}
                    {tool.used_by.length > 0 && <> · for {tool.used_by.join(", ")}</>}
                  </span>
                </span>
                {!tool.installed &&
                  (tool.install?.kind === "command" ? (
                    <Button size="sm" variant="outline" onClick={() => setConfirm(tool)}>
                      {t("Install…")}
                    </Button>
                  ) : tool.install?.kind === "runtime" ? (
                    // A verified download, like Runtimes → Install; the progress shows there.
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={runtime.install.isPending}
                      onClick={() => {
                        const kind = (tool.install as { runtime: RuntimeKind }).runtime;
                        runtime.install.mutate(
                          { kind, version: kind === "node" ? "lts" : "latest" },
                          {
                            onSuccess: () =>
                              navigate("runtimes", kind === "node" || kind === "php" ? kind : "package-managers"),
                          },
                        );
                      }}
                    >
                      {t("Install")}
                    </Button>
                  ) : (
                    <Button size="sm" variant="ghost" onClick={() => openInBrowser(tool.docs)}>
                      {t("Get it")} <ArrowUpRight data-icon="inline-end" />
                    </Button>
                  ))}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {terminal && (
        <Section
          title={terminal.title.toLowerCase()}
          aside={
            !terminal.running && (
              <Button
                size="xs"
                variant="ghost"
                onClick={() => {
                  setTerminal(null);
                  tools.refetch();
                }}
              >
                {t("Close")}
              </Button>
            )
          }
        >
          <PtyTerminal id={terminal.id} onInfo={setTerminal} className="h-72" />
        </Section>
      )}

      <Dialog open={confirm !== null} onOpenChange={(open) => !open && setConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("Install {name}", { name: confirm?.name ?? "" })}</DialogTitle>
            <DialogDescription>{t("This runs in a terminal here, as your user:")}</DialogDescription>
          </DialogHeader>
          {confirm?.install?.kind === "command" &&
            confirm.install.steps.map((s) => (
              <code key={s.label} className="rounded-md bg-well px-3 py-2 font-mono text-[12px]" data-selectable>
                $ {stepCommand(s)}
              </code>
            ))}
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirm(null)}>
              {t("Cancel")}
            </Button>
            <Button
              onClick={async () => {
                const tool = confirm!;
                setConfirm(null);
                setTerminal(await install.mutateAsync(tool.id));
              }}
            >
              {t("Install")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Page>
  );
}
