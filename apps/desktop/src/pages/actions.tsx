import {
  AlertTriangle,
  CircleCheck,
  Download,
  FileCode,
  History,
  Info,
  OctagonX,
  RefreshCw,
  Save,
  Sparkles,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { CodeEditor, CodeView } from "@/components/code-editor";
import { DiffView } from "@/components/git";
import { Page, Section, useCrumbs } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { WorkflowEditor } from "@/components/workflow-editor";
import {
  useActionsOverview,
  useActionsRender,
  useActionsValidate,
  useDebounced,
  useProjects,
  useRefreshActionVersions,
} from "@/hooks/use-daemon";
import {
  useActionlintInstall,
  useActionsAudit,
  useActionsHistory,
  useActionsHistoryVersion,
} from "@/hooks/use-insights";
import { request } from "@/lib/daemon";
import { uptime } from "@/lib/format";
import { subById, viewById } from "@/lib/nav";
import type { Finding, SavePlan, Validation, WorkflowModel } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { intlLocale, t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

type Doc = { kind: "visual"; workflow: WorkflowModel } | { kind: "yaml"; text: string };

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function ActionsView() {
  const view = viewById("tools");
  const sub = subById(view, "actions")!;
  const crumbs = useCrumbs(view, sub);
  const projects = useProjects();
  const [site, setSite] = useState<string | null>(null);
  const name = site ?? projects.data?.[0]?.name ?? null;
  const overview = useActionsOverview(name);
  const refresh = useRefreshActionVersions();

  const [doc, setDoc] = useState<Doc | null>(null);
  const [file, setFile] = useState("ci.yml");
  const [origin, setOrigin] = useState<string | null>(null);
  const [reviewing, setReviewing] = useState(false);

  // The daemon renders and checks; debounced so typing stays smooth.
  const settled = useDebounced(doc, 250);
  const render = useActionsRender(settled?.kind === "visual" ? settled.workflow : null);
  const validate = useActionsValidate(settled?.kind === "yaml" ? settled.text : null);
  const yaml = doc?.kind === "yaml" ? doc.text : (render.data?.yaml ?? "");
  const validation = doc?.kind === "yaml" ? validate.data : render.data?.validation;

  const reset = () => {
    setDoc(null);
    setOrigin(null);
  };
  useEffect(reset, [name]);

  const fromTemplate = async (id: string) => {
    if (!name) return;
    try {
      const { workflow, file } = await request({ type: "actions_template", name, template: id }, "actions_workflow");
      setDoc({ kind: "visual", workflow });
      setFile(file);
      setOrigin(`template ${id}`);
    } catch (error) {
      toast.error(t("Could not load the template"), { description: message(error) });
    }
  };
  const open = async (target: string) => {
    if (!name) return;
    try {
      const { yaml, validation } = await request({ type: "actions_read", name, file: target }, "actions_yaml");
      setDoc(validation.workflow ? { kind: "visual", workflow: validation.workflow } : { kind: "yaml", text: yaml });
      setFile(target);
      setOrigin(target);
      if (!validation.workflow) toast.info(t("Opened as YAML"), { description: validation.unsupported ?? undefined });
    } catch (error) {
      toast.error(t("Could not open the file"), { description: message(error) });
    }
  };

  const toVisual = validation?.workflow ?? null;

  return (
    <Page
      view={view}
      title={sub.label}
      description={sub.description}
      crumbs={crumbs}
      wide="full"
      actions={
        <div className="flex items-center gap-2">
          <Select
            items={(projects.data ?? []).map((p) => ({ value: p.name, label: p.name }))}
            value={name ?? ""}
            onValueChange={(v) => v && setSite(String(v))}
          >
            <SelectTrigger size="sm" aria-label={t("Site")} className="min-w-36 font-mono text-[12.5px]">
              <SelectValue placeholder={t("Pick a site")} />
            </SelectTrigger>
            <SelectContent>
              {(projects.data ?? []).map((p) => (
                <SelectItem key={p.name} value={p.name} className="font-mono text-[12.5px]">
                  {p.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {doc && (
            <Button size="sm" variant="ghost" onClick={reset}>
              {t("Close")}
            </Button>
          )}
        </div>
      }
    >
      {!name ? (
        <p className="text-sm text-muted-foreground">{t("Add a site first: workflows are saved in its folder.")}</p>
      ) : overview.isPending ? (
        <Skeleton className="h-48" />
      ) : overview.isError ? (
        <p className="text-sm text-destructive">{message(overview.error)}</p>
      ) : !doc ? (
        <Start
          name={name}
          onVersion={(target, yaml, validation) => {
            setDoc(
              validation.workflow ? { kind: "visual", workflow: validation.workflow } : { kind: "yaml", text: yaml },
            );
            setFile(target);
            setOrigin(t("a kept version of {file}", { file: target }));
          }}
          overview={overview.data}
          onTemplate={fromTemplate}
          onOpen={open}
          onRefresh={() => refresh.mutate()}
          refreshing={refresh.isPending}
        />
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <div role="radiogroup" aria-label={t("Editor")} className="flex rounded-md border p-0.5 text-[12.5px]">
              <button
                type="button"
                role="radio"
                aria-checked={doc.kind === "visual"}
                disabled={doc.kind === "yaml" && !toVisual}
                title={doc.kind === "yaml" && !toVisual ? (validation?.unsupported ?? "") : undefined}
                onClick={() => toVisual && setDoc({ kind: "visual", workflow: toVisual })}
                className={cn(
                  "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground disabled:opacity-50",
                  doc.kind === "visual" && "bg-foreground/[0.07] text-foreground",
                )}
              >
                {t("Visual")}
              </button>
              <button
                type="button"
                role="radio"
                aria-checked={doc.kind === "yaml"}
                onClick={() => doc.kind === "visual" && setDoc({ kind: "yaml", text: yaml })}
                className={cn(
                  "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
                  doc.kind === "yaml" && "bg-foreground/[0.07] text-foreground",
                )}
              >
                {t("YAML")}
              </button>
            </div>
            {origin && (
              <span className="font-mono text-[11.5px] text-muted-foreground">{t("from {origin}", { origin })}</span>
            )}
            <div className="ml-auto flex items-center gap-2">
              <span className="font-mono text-[12px] text-muted-foreground">{t(".github/workflows/")}</span>
              <Input
                value={file}
                onChange={(e) => setFile(e.target.value.trim())}
                aria-label={t("File name")}
                className="h-8 w-44 font-mono text-[12.5px]"
              />
              <Button
                size="sm"
                disabled={!yaml}
                onClick={() => setReviewing(true)}
                className="bg-ember text-ember-foreground hover:bg-ember/85"
              >
                <Save data-icon="inline-start" /> {t("Review & save…")}
              </Button>
            </div>
          </div>

          <div className="@container">
            <div className="grid gap-6 @4xl:grid-cols-[minmax(0,1fr)_minmax(0,26rem)]">
              <div className="min-w-0">
                {doc.kind === "visual" ? (
                  <WorkflowEditor
                    workflow={doc.workflow}
                    onChange={(workflow) => setDoc({ kind: "visual", workflow })}
                    actions={overview.data.actions}
                  />
                ) : (
                  <YamlEditor
                    text={doc.text}
                    onChange={(text) => setDoc({ kind: "yaml", text })}
                    findings={validation?.findings ?? []}
                  />
                )}
              </div>
              <div className="flex min-w-0 flex-col gap-5 @4xl:sticky @4xl:top-4 @4xl:self-start">
                <Checks validation={validation} actionlint={overview.data.actionlint} />
                {doc.kind === "visual" && (
                  <Section title={t("yaml")}>
                    <YamlPreview text={yaml} findings={validation?.findings ?? []} />
                  </Section>
                )}
              </div>
            </div>
          </div>
          {reviewing && (
            <ReviewDialog
              name={name}
              file={file}
              yaml={yaml}
              onClose={() => setReviewing(false)}
              onSaved={() => {
                setReviewing(false);
                setOrigin(file);
                overview.refetch();
              }}
            />
          )}
        </>
      )}
    </Page>
  );
}

function Start({
  name,
  onVersion,
  overview,
  onTemplate,
  onOpen,
  onRefresh,
  refreshing,
}: {
  name: string;
  onVersion: (file: string, yaml: string, validation: Validation) => void;
  overview: NonNullable<ReturnType<typeof useActionsOverview>["data"]>;
  onTemplate: (id: string) => void;
  onOpen: (file: string) => void;
  onRefresh: () => void;
  refreshing: boolean;
}) {
  return (
    <div className="grid gap-8 @container lg:grid-cols-[minmax(0,1fr)_18rem]">
      <div className="flex flex-col gap-6">
        <Section title={t("new from a template")}>
          <div className="grid gap-2 sm:grid-cols-2">
            {overview.templates.map((tpl) => (
              <button
                key={tpl.id}
                type="button"
                onClick={() => onTemplate(tpl.id)}
                className="flex flex-col gap-1 rounded-lg border px-3.5 py-3 text-left transition-colors hover:border-ember/50 hover:bg-ember/3"
              >
                <span className="flex items-center gap-2 text-[13.5px] font-medium">
                  {tpl.title}
                  {tpl.recommended && (
                    <span className="flex items-center gap-1 rounded bg-ember/10 px-1.5 font-mono text-[10.5px] font-normal text-ember">
                      <Sparkles className="size-3" /> {t("fits")}
                    </span>
                  )}
                </span>
                <span className="text-[12.5px] text-muted-foreground">{tpl.description}</span>
                <span className="font-mono text-[11px] text-muted-foreground/80">{tpl.reason}</span>
              </button>
            ))}
          </div>
        </Section>
        <Section title={t("workflow files")}>
          {overview.existing.length === 0 ? (
            <p className="px-1 text-[12.5px] text-muted-foreground">
              {tx("No files in {github_workflows} yet.", {
                github_workflows: <span className="font-mono">.github/workflows/</span>,
              })}
            </p>
          ) : (
            overview.existing.map((w) => (
              <WorkflowRow
                key={w.file}
                name={name}
                file={w.file}
                title={w.name}
                onOpen={onOpen}
                onVersion={onVersion}
              />
            ))
          )}
        </Section>
        {overview.existing.length > 0 && <Audit name={name} />}
      </div>
      <div className="flex flex-col gap-6">
        <Section
          title={t("action versions")}
          aside={
            <Button size="xs" variant="ghost" disabled={refreshing} onClick={onRefresh}>
              <RefreshCw data-icon="inline-start" className={cn(refreshing && "animate-spin")} /> {t("Check")}
            </Button>
          }
        >
          {overview.actions.map((a) => (
            <div
              key={a.repo}
              className="flex h-6 items-center justify-between gap-2 font-mono text-[11.5px]"
              title={a.purpose}
            >
              <span className="truncate">{a.repo}</span>
              <span className="text-ember">{a.version}</span>
            </div>
          ))}
          <p className="pt-2 text-[11.5px] text-muted-foreground">
            {overview.versions_checked_at
              ? t("Checked against GitHub {overview_versions_checked_at} ago.", {
                  overview_versions_checked_at: uptime(overview.versions_checked_at).split(" ")[0],
                })
              : t("Built-in list. Check asks GitHub for each action's latest release.")}
          </p>
        </Section>
        <Section title={t("checks")}>
          <p className="text-[12.5px] text-muted-foreground">
            {tx(
              "LocDev checks structure, script injection, {pull_request_target}, token permissions and unpinned or outdated actions. {actionlint}",
              {
                pull_request_target: <span className="font-mono">pull_request_target</span>,
                actionlint: overview.actionlint
                  ? t("actionlint {actionlint} runs too.", { actionlint: overview.actionlint })
                  : t(
                      "Install actionlint for deeper checks (expressions, shellcheck); it runs automatically when found on PATH.",
                    ),
              },
            )}
          </p>
          {!overview.actionlint && <InstallActionlint />}
        </Section>
      </div>
    </div>
  );
}

const LEVEL = {
  error: { icon: OctagonX, className: "text-destructive" },
  warning: { icon: AlertTriangle, className: "text-amber" },
  info: { icon: Info, className: "text-muted-foreground" },
} as const;

function Checks({ validation, actionlint }: { validation: Validation | undefined; actionlint: string | null }) {
  if (!validation) return <Skeleton className="h-20" />;
  const errors = validation.findings.filter((f) => f.level === "error").length;
  return (
    <Section
      title={`${t("checks")} · ${tn("{n} error", "{n} errors", errors)}`}
      aside={
        <span className="font-mono text-[10.5px] text-muted-foreground">
          {actionlint ? t("+ actionlint {actionlint}", { actionlint }) : t("actionlint not installed")}
        </span>
      }
    >
      {validation.findings.length === 0 ? (
        <p className="flex items-center gap-2 px-1 text-[13px] text-phosphor">
          <CircleCheck className="size-4" /> {t("No problems found.")}
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {validation.findings.map((f, i) => (
            <FindingRow key={i} finding={f} />
          ))}
        </ul>
      )}
    </Section>
  );
}

function FindingRow({ finding: f, file }: { finding: Finding; file?: string }) {
  const { icon: Icon, className } = LEVEL[f.level];
  return (
    <li className="flex gap-2 text-[12.5px]">
      <Icon className={cn("mt-0.5 size-3.5 shrink-0", className)} />
      <span className="min-w-0">
        <span data-selectable>{f.message}</span>{" "}
        <span className="font-mono text-[10.5px] text-muted-foreground">
          {file ? `${file} · ` : ""}
          {f.line ? t("line {line} · ", { line: f.line }) : ""}
          {f.rule}
          {f.source === "actionlint" ? t(" · actionlint") : ""}
        </span>
        {f.fix && (
          <pre
            data-selectable
            aria-label={t("Suggested fix")}
            className="mt-1 overflow-x-auto rounded border border-phosphor/25 bg-phosphor/5 px-2 py-1 font-mono text-[11px] text-phosphor"
          >
            {f.fix}
          </pre>
        )}
      </span>
    </li>
  );
}

function InstallActionlint() {
  const install = useActionlintInstall();
  return (
    <Button
      size="sm"
      variant="outline"
      className="mt-2 self-start"
      disabled={install.isPending}
      onClick={() => install.mutate()}
    >
      <Download data-icon="inline-start" /> {install.isPending ? t("Installing…") : t("Install actionlint")}
    </Button>
  );
}

/** Every workflow file, checked: errors, security and the community practices with their fixes. */
function Audit({ name }: { name: string }) {
  const audit = useActionsAudit(name);
  if (!audit.data) return <Skeleton className="h-20" />;
  const all = audit.data.files.flatMap((f) => f.findings.map((finding) => ({ file: f.file, finding })));
  return (
    <Section
      title={t("checks of every workflow")}
      aside={
        <Button size="xs" variant="ghost" disabled={audit.isFetching} onClick={() => audit.refetch()}>
          <RefreshCw data-icon="inline-start" className={cn(audit.isFetching && "animate-spin")} /> {t("Check again")}
        </Button>
      }
    >
      {all.length === 0 ? (
        <p className="flex items-center gap-2 px-1 text-[13px] text-phosphor">
          <CircleCheck className="size-4" /> {t("No problems found.")}
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {all.map(({ file, finding }, i) => (
            <FindingRow key={i} finding={finding} file={file} />
          ))}
        </ul>
      )}
    </Section>
  );
}

function WorkflowRow({
  name,
  file,
  title,
  onOpen,
  onVersion,
}: {
  name: string;
  file: string;
  title: string | null;
  onOpen: (file: string) => void;
  onVersion: (file: string, yaml: string, validation: Validation) => void;
}) {
  const [open, setOpen] = useState(false);
  const history = useActionsHistory(name, open ? file : null);
  const version = useActionsHistoryVersion(name);
  return (
    <div className="flex flex-col">
      <div className="flex h-9 items-center gap-1 rounded px-1 hover:bg-foreground/4">
        <button
          type="button"
          onClick={() => onOpen(file)}
          className="flex min-w-0 flex-1 items-center gap-3 text-left text-[13px]"
        >
          <FileCode className="size-3.5 text-muted-foreground" />
          <span className="font-mono">{file}</span>
          <span className="truncate text-muted-foreground">{title}</span>
        </button>
        <Button
          size="icon-xs"
          variant="ghost"
          aria-label={t("Local versions of {file}", { file })}
          aria-expanded={open}
          onClick={() => setOpen(!open)}
        >
          <History />
        </Button>
      </div>
      {open && (
        <ul className="mb-2 ml-7 flex flex-col border-l pl-3">
          {history.data?.length === 0 && (
            <li className="text-[12px] text-muted-foreground">{t("No versions kept yet.")}</li>
          )}
          {history.data?.map((v) => (
            <li key={v.id}>
              <button
                type="button"
                className="flex h-7 w-full items-center gap-3 text-left font-mono text-[11.5px] hover:text-foreground"
                onClick={() =>
                  version.mutate(
                    { file, id: v.id },
                    { onSuccess: ({ yaml, validation }) => onVersion(file, yaml, validation) },
                  )
                }
              >
                <span>{new Date(v.at).toLocaleString(intlLocale)}</span>
                <span className="text-muted-foreground">{REASON[v.reason] ?? v.reason}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

const REASON: Record<string, string> = {
  read: t("changed outside LocDev"),
  "before-save": t("before a save"),
  saved: t("saved"),
};

function marks(findings: Finding[]) {
  const byLine = new Map<number, Finding["level"]>();
  for (const f of findings) {
    if (f.line && (!byLine.has(f.line) || f.level === "error")) byLine.set(f.line, f.level);
  }
  return byLine;
}

function YamlPreview({ text, findings }: { text: string; findings: Finding[] }) {
  const byLine = useMemo(() => marks(findings), [findings]);
  return (
    <CodeView value={text} language="yaml" marks={byLine} ariaLabel={t("Generated workflow")} className="max-h-144" />
  );
}

/** The workflow as text, with the checks' lines tinted. */
function YamlEditor({
  text,
  onChange,
  findings,
}: {
  text: string;
  onChange: (text: string) => void;
  findings: Finding[];
}) {
  const byLine = useMemo(() => marks(findings), [findings]);
  return (
    <CodeEditor
      value={text}
      onChange={onChange}
      language="yaml"
      marks={byLine}
      ariaLabel={t("Workflow YAML")}
      className="h-144"
    />
  );
}

/** The diff and the checks, then an explicit save. Replacing needs a tick. */
function ReviewDialog({
  name,
  file,
  yaml,
  onClose,
  onSaved,
}: {
  name: string;
  file: string;
  yaml: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [plan, setPlan] = useState<SavePlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [replace, setReplace] = useState(false);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    request({ type: "actions_plan", name, file, yaml }, "actions_plan")
      .then(({ type: _, ...plan }) => setPlan(plan))
      .catch((e) => setError(message(e)));
  }, [name, file, yaml]);

  const blocking = plan?.validation.findings.filter((f) => f.level === "error" && f.source === "locdev") ?? [];
  const unchanged =
    plan?.exists && !plan.diff.split("\n").some((l) => /^[+-]/.test(l) && !l.startsWith("+++") && !l.startsWith("---"));
  const save = async () => {
    if (!plan) return;
    setSaving(true);
    try {
      const { path } = await request(
        { type: "actions_save", name, file, yaml, replace: plan.exists ? plan.current_hash : null },
        "actions_saved",
      );
      toast.success(`Saved ${path}`, { description: t("Commit and push it for GitHub to run it.") });
      onSaved();
    } catch (e) {
      toast.error(t("Not saved"), { description: message(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>
            {plan?.exists ? t("Replace {path}", { path: plan.path }) : t("Create {path}", { path: plan?.path ?? file })}
          </DialogTitle>
          <DialogDescription>
            {plan?.exists
              ? t("This file already exists. Review what changes before replacing it.")
              : t("A new file in the site's folder. Nothing else is touched.")}
          </DialogDescription>
        </DialogHeader>
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : !plan ? (
          <Skeleton className="h-40" />
        ) : (
          <div className="flex flex-col gap-4">
            {unchanged ? (
              <p className="text-sm text-muted-foreground">
                {t("Nothing to change: the file already has this content.")}
              </p>
            ) : (
              <DiffView text={plan.diff} />
            )}
            {blocking.length > 0 ? (
              <div className="rounded-md border border-destructive/30 bg-destructive/6 px-3 py-2 text-[13px] text-destructive">
                {tn("Fix {n} error first: {message}", "Fix {n} errors first: {message}", blocking.length, {
                  message: blocking[0].message,
                })}
              </div>
            ) : (
              plan.validation.findings.length > 0 && (
                <p className="text-[12.5px] text-muted-foreground">
                  {tn(
                    "{n} note from the checks (warnings don't block saving).",
                    "{n} notes from the checks (warnings don't block saving).",
                    plan.validation.findings.length,
                  )}
                </p>
              )
            )}
            {plan.exists && (
              <label className="flex items-center gap-2 text-[13px]">
                <Checkbox checked={replace} onCheckedChange={(v) => setReplace(v === true)} />
                {t("Replace the existing file with this version")}
              </label>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={onClose}>
                {t("Cancel")}
              </Button>
              <Button
                disabled={saving || unchanged || blocking.length > 0 || (plan.exists && !replace)}
                onClick={save}
                className="bg-ember text-ember-foreground hover:bg-ember/85"
              >
                {plan.exists ? t("Replace file") : t("Create file")}
              </Button>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
