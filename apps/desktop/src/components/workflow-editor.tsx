import { ArrowDown, ArrowUp, ChevronRight, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { CodeEditor } from "@/components/code-editor";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import {
  type ActionVersion,
  emptyStep,
  type Permissions,
  type WorkflowFilter,
  type WorkflowJob,
  type WorkflowModel,
  type WorkflowStep,
} from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const RUNNERS = ["ubuntu-latest", "ubuntu-24.04", "windows-latest", "macos-latest"];
const SCOPES = [
  "actions",
  "checks",
  "contents",
  "deployments",
  "id-token",
  "issues",
  "packages",
  "pages",
  "pull-requests",
  "security-events",
  "statuses",
];

/** The visual editor: every change produces a new model (the daemon renders it). */
export function WorkflowEditor({
  workflow,
  onChange,
  actions,
}: {
  workflow: WorkflowModel;
  onChange: (next: WorkflowModel) => void;
  actions: ActionVersion[];
}) {
  const set = (patch: Partial<WorkflowModel>) => onChange({ ...workflow, ...patch });
  const on = (patch: Partial<WorkflowModel["on"]>) => set({ on: { ...workflow.on, ...patch } });
  const setJob = (i: number, job: WorkflowJob | null) => {
    const jobs = [...workflow.jobs];
    if (job) jobs[i] = job;
    else {
      // Drop the removed job from the others' `needs` too.
      const [removed] = jobs.splice(i, 1);
      for (const j of jobs) j.needs = j.needs.filter((n) => n !== removed.id);
    }
    set({ jobs });
  };
  const addJob = () => {
    let n = workflow.jobs.length + 1;
    while (workflow.jobs.some((j) => j.id === `job${n}`)) n++;
    set({
      jobs: [
        ...workflow.jobs,
        {
          id: `job${n}`,
          name: null,
          runs_on: "ubuntu-latest",
          needs: [],
          if: null,
          permissions: null,
          environment: null,
          timeout_minutes: 15,
          matrix: {},
          env: {},
          steps: [{ ...emptyStep(), uses: uses(actions, "actions/checkout"), run: null }],
        },
      ],
    });
  };

  return (
    <div className="flex flex-col gap-6">
      <Section title={t("workflow")}>
        <Field label={t("Name")}>
          <Input
            value={workflow.name}
            onChange={(e) => set({ name: e.target.value })}
            aria-label={t("Workflow name")}
          />
        </Field>
      </Section>

      <Section title={t("runs on")}>
        <div className="flex flex-col gap-2.5">
          <Trigger
            label={t("Push")}
            filter={workflow.on.push}
            onChange={(push) => on({ push })}
            hint={t("branches and tags; empty runs on every push")}
          />
          <Trigger
            label={t("Pull request")}
            filter={workflow.on.pull_request}
            onChange={(pull_request) => on({ pull_request })}
            hint={t("target branches; empty runs on all")}
            noTags
          />
          <Toggle
            label={t("Manually (Run workflow button)")}
            checked={workflow.on.workflow_dispatch}
            onChange={(workflow_dispatch) => on({ workflow_dispatch })}
          />
          <Toggle
            label={t("When a release is published")}
            checked={workflow.on.release}
            onChange={(release) => on({ release })}
          />
          <Field label={t("Schedule (cron, UTC)")}>
            <ListInput
              value={workflow.on.schedule}
              onChange={(schedule) => on({ schedule })}
              placeholder={t("27 4 * * 1   (one per line)")}
              lines
            />
          </Field>
        </div>
      </Section>

      <Section title={t("token permissions")}>
        <PermissionsEditor value={workflow.permissions} onChange={(permissions) => set({ permissions })} top />
        <Toggle
          label={t("Cancel older runs of the same branch or pull request")}
          checked={workflow.concurrency !== null}
          onChange={(v) =>
            set({
              concurrency: v ? { group: "${{ github.workflow }}-${{ github.ref }}", cancel_in_progress: true } : null,
            })
          }
        />
      </Section>

      <Section
        title={t("jobs {n}", { n: workflow.jobs.length })}
        aside={
          <Button size="xs" variant="ghost" onClick={addJob}>
            <Plus data-icon="inline-start" /> {t("Add job")}
          </Button>
        }
      >
        <div className="flex flex-col gap-4">
          {workflow.jobs.map((job, i) => (
            <JobEditor
              key={i}
              job={job}
              others={workflow.jobs.filter((_, j) => j !== i).map((j) => j.id)}
              actions={actions}
              onChange={(next) => setJob(i, next)}
              onRemove={workflow.jobs.length > 1 ? () => setJob(i, null) : undefined}
            />
          ))}
        </div>
      </Section>
    </div>
  );
}

const uses = (actions: ActionVersion[], repo: string) =>
  `${repo}@${actions.find((a) => a.repo === repo)?.version ?? "main"}`;

function Field({ label, children, className }: { label: string; children: React.ReactNode; className?: string }) {
  return (
    <label className={cn("flex flex-col gap-1 text-[12.5px] text-muted-foreground", className)}>
      {label}
      {children}
    </label>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="flex items-center gap-2.5 py-1 text-[13px]">
      <Switch checked={checked} onCheckedChange={onChange} className="data-checked:bg-ember" />
      {label}
    </label>
  );
}

/** A list edited as text: comma-separated (or one per line), committed on blur. */
function ListInput({
  value,
  onChange,
  placeholder,
  lines,
  label,
}: {
  value: string[];
  onChange: (v: string[]) => void;
  placeholder?: string;
  lines?: boolean;
  label?: string;
}) {
  const joined = value.join(lines ? "\n" : ", ");
  const [draft, setDraft] = useState<string | null>(null);
  const commit = () => {
    if (draft === null) return;
    onChange(
      draft
        .split(lines ? "\n" : ",")
        .map((s) => s.trim())
        .filter(Boolean),
    );
    setDraft(null);
  };
  const props = {
    value: draft ?? joined,
    onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => setDraft(e.target.value),
    onBlur: commit,
    placeholder,
    "aria-label": label,
    className: "font-mono text-[12.5px]",
  };
  return lines ? <Textarea {...props} className={cn(props.className, "min-h-9")} /> : <Input {...props} />;
}

function Trigger({
  label,
  filter,
  onChange,
  hint,
  noTags,
}: {
  label: string;
  filter: WorkflowFilter | null;
  onChange: (f: WorkflowFilter | null) => void;
  hint: string;
  noTags?: boolean;
}) {
  return (
    <div className="flex flex-col gap-2">
      <Toggle
        label={label}
        checked={filter !== null}
        onChange={(v) => onChange(v ? { branches: [], tags: [], paths: [] } : null)}
      />
      {filter && (
        <div className="ml-11 grid grid-cols-2 gap-2">
          <Field label={t("Branches ({hint})", { hint })} className={noTags ? "col-span-2" : undefined}>
            <ListInput
              value={filter.branches}
              onChange={(branches) => onChange({ ...filter, branches })}
              placeholder={t("main, release/*")}
              label={t("{label} branches", { label })}
            />
          </Field>
          {!noTags && (
            <Field label={t("Tags")}>
              <ListInput value={filter.tags} onChange={(tags) => onChange({ ...filter, tags })} placeholder="v*" />
            </Field>
          )}
        </div>
      )}
    </div>
  );
}

function PermissionsEditor({
  value,
  onChange,
  top,
}: {
  value: Permissions | null;
  onChange: (p: Permissions | null) => void;
  top?: boolean;
}) {
  const mode = value === null ? "unset" : value.kind;
  const scopes = value?.kind === "scopes" ? value.scopes : {};
  return (
    <div className="flex flex-col gap-2 pb-2">
      <div role="radiogroup" aria-label={t("Permissions")} className="flex w-fit rounded-md border p-0.5 text-[12px]">
        {(
          [
            ["unset", top ? t("Repository default") : t("Workflow's")],
            ["scopes", t("Only what's listed")],
            ["read_all", t("Read all")],
            ["write_all", t("Write all")],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={mode === id}
            onClick={() =>
              onChange(
                id === "unset"
                  ? null
                  : id === "scopes"
                    ? { kind: "scopes", scopes: { contents: "read" } }
                    : { kind: id },
              )
            }
            className={cn(
              "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
              mode === id && "bg-foreground/[0.07] text-foreground",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      {mode === "scopes" && (
        <div className="grid gap-x-8 gap-y-1 sm:grid-cols-2">
          {SCOPES.map((scope) => (
            <div key={scope} className="flex items-center justify-between gap-2 text-[12.5px]">
              <span className="font-mono">{scope}</span>
              <Choice
                value={scopes[scope] ?? "none"}
                items={["none", "read", "write"]}
                label={t("{scope} permission", { scope })}
                onChange={(level) => {
                  const next = { ...scopes };
                  if (level === "none") delete next[scope];
                  else next[scope] = level;
                  onChange({ kind: "scopes", scopes: next });
                }}
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Choice({
  value,
  items,
  onChange,
  label,
}: {
  value: string;
  items: string[];
  onChange: (v: string) => void;
  label: string;
}) {
  return (
    <Select
      items={items.map((i) => ({ value: i, label: i }))}
      value={value}
      onValueChange={(v) => v && onChange(String(v))}
    >
      <SelectTrigger size="sm" aria-label={label} className="h-7 min-w-24 font-mono text-[12px]">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {items.map((i) => (
          <SelectItem key={i} value={i} className="font-mono text-[12px]">
            {i}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

function JobEditor({
  job,
  others,
  actions,
  onChange,
  onRemove,
}: {
  job: WorkflowJob;
  others: string[];
  actions: ActionVersion[];
  onChange: (job: WorkflowJob) => void;
  onRemove?: () => void;
}) {
  const set = (patch: Partial<WorkflowJob>) => onChange({ ...job, ...patch });
  const setStep = (i: number, step: WorkflowStep | null) => {
    const steps = [...job.steps];
    if (step) steps[i] = step;
    else steps.splice(i, 1);
    set({ steps });
  };
  const move = (i: number, by: number) => {
    const steps = [...job.steps];
    const [s] = steps.splice(i, 1);
    steps.splice(i + by, 0, s);
    set({ steps });
  };
  const [open, setOpen] = useState<number | null>(null);
  const runner = RUNNERS.includes(job.runs_on) ? job.runs_on : "custom";

  return (
    <div className="rounded-lg border">
      <div className="flex flex-wrap items-end gap-3 border-b px-3 py-3">
        <Field label={t("Job id")}>
          <Input
            value={job.id}
            onChange={(e) => set({ id: e.target.value.replace(/[^A-Za-z0-9_-]/g, "") })}
            className="h-8 w-36 font-mono text-[12.5px]"
            aria-label={t("Job id")}
          />
        </Field>
        <Field label={t("Runner")}>
          <div className="flex gap-1.5">
            <Choice
              value={runner}
              items={[...RUNNERS, "custom"]}
              label={t("Runner")}
              onChange={(v) => set({ runs_on: v === "custom" ? job.runs_on : v })}
            />
            {runner === "custom" && (
              <Input
                value={job.runs_on}
                onChange={(e) => set({ runs_on: e.target.value })}
                className="h-7 w-36 font-mono text-[12px]"
                aria-label={t("Runner label")}
              />
            )}
          </div>
        </Field>
        <Field label={t("Timeout (min)")}>
          <Input
            type="number"
            min={1}
            value={job.timeout_minutes ?? ""}
            onChange={(e) => set({ timeout_minutes: e.target.value ? Number(e.target.value) : null })}
            className="h-8 w-20 font-mono text-[12.5px]"
          />
        </Field>
        <Field label={t("Environment")}>
          <Input
            value={job.environment ?? ""}
            onChange={(e) => set({ environment: e.target.value || null })}
            placeholder={t("none")}
            className="h-8 w-32 font-mono text-[12.5px]"
          />
        </Field>
        {onRemove && (
          <Button
            size="icon-sm"
            variant="ghost"
            className="ml-auto"
            aria-label={t("Remove job {id}", { id: job.id })}
            onClick={onRemove}
          >
            <Trash2 />
          </Button>
        )}
      </div>

      <div className="flex flex-col gap-2 border-b px-3 py-2.5 text-[12.5px]">
        {others.length > 0 && (
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-muted-foreground">{t("Waits for")}</span>
            {others.map((id) => (
              <label key={id} className="flex items-center gap-1.5 font-mono">
                <Checkbox
                  checked={job.needs.includes(id)}
                  onCheckedChange={(v) => set({ needs: v ? [...job.needs, id] : job.needs.filter((n) => n !== id) })}
                />
                {id}
              </label>
            ))}
          </div>
        )}
        <details>
          <summary className="cursor-pointer text-muted-foreground">
            {t("Job permissions (overrides the workflow's)")}
          </summary>
          <div className="pt-2">
            <PermissionsEditor value={job.permissions} onChange={(permissions) => set({ permissions })} />
          </div>
        </details>
      </div>

      <ol className="flex flex-col">
        {job.steps.map((step, i) => (
          <li key={i} className="border-b last:border-b-0">
            <div className="group flex h-9 items-center gap-2 px-3 text-[13px]">
              <button
                type="button"
                className="flex min-w-0 flex-1 items-center gap-2 text-left"
                onClick={() => setOpen(open === i ? null : i)}
                aria-expanded={open === i}
              >
                <ChevronRight className={cn("size-3.5 shrink-0 transition-transform", open === i && "rotate-90")} />
                <span className="w-5 shrink-0 font-mono text-[11px] text-muted-foreground">{i + 1}</span>
                <span className="truncate">
                  {step.name || (step.uses ? step.uses : step.run?.split("\n")[0] || t("empty step"))}
                </span>
                {step.name && (
                  <span className="truncate font-mono text-[11px] text-muted-foreground">
                    {step.uses ?? step.run?.split("\n")[0]}
                  </span>
                )}
              </button>
              <span className="flex opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Move up")}
                  disabled={i === 0}
                  onClick={() => move(i, -1)}
                >
                  <ArrowUp />
                </Button>
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Move down")}
                  disabled={i === job.steps.length - 1}
                  onClick={() => move(i, 1)}
                >
                  <ArrowDown />
                </Button>
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Remove step {i}", { i: i + 1 })}
                  onClick={() => setStep(i, null)}
                >
                  <Trash2 />
                </Button>
              </span>
            </div>
            {open === i && <StepEditor step={step} onChange={(s) => setStep(i, s)} />}
          </li>
        ))}
      </ol>
      <div className="px-3 py-2">
        <DropdownMenu>
          <DropdownMenuTrigger render={<Button size="xs" variant="ghost" />}>
            <Plus data-icon="inline-start" /> {t("Add step")}
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start" className="max-h-80">
            <DropdownMenuItem
              onClick={() => {
                set({ steps: [...job.steps, { ...emptyStep(), name: "Run" }] });
                setOpen(job.steps.length);
              }}
            >
              {t("Run a command")}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            {actions.map((a) => (
              <DropdownMenuItem
                key={a.repo}
                onClick={() => {
                  set({ steps: [...job.steps, { ...emptyStep(), run: null, uses: `${a.repo}@${a.version}` }] });
                  setOpen(job.steps.length);
                }}
              >
                <span className="font-mono text-[12px]">{a.repo}</span>
                <span className="ml-auto pl-4 text-[11px] text-muted-foreground">{a.purpose}</span>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}

function StepEditor({ step, onChange }: { step: WorkflowStep; onChange: (s: WorkflowStep) => void }) {
  const set = (patch: Partial<WorkflowStep>) => onChange({ ...step, ...patch });
  const kind = step.uses !== null ? "uses" : "run";
  return (
    <div className="flex flex-col gap-2.5 bg-foreground/2 px-10 pt-1 pb-3">
      <div className="grid grid-cols-2 gap-2">
        <Field label={t("Name")}>
          <Input
            value={step.name ?? ""}
            onChange={(e) => set({ name: e.target.value || null })}
            className="h-8 text-[13px]"
          />
        </Field>
        <Field label={t("Only if (expression)")}>
          <Input
            value={step.if ?? ""}
            onChange={(e) => set({ if: e.target.value || null })}
            placeholder={t("always")}
            className="h-8 font-mono text-[12.5px]"
          />
        </Field>
      </div>
      {kind === "uses" ? (
        <>
          <Field label={t("Action (owner/repo@version)")}>
            <Input
              value={step.uses ?? ""}
              onChange={(e) => set({ uses: e.target.value })}
              className="h-8 font-mono text-[12.5px]"
              aria-label={t("Action")}
            />
          </Field>
          <Pairs label={t("Inputs (with)")} value={step.with} onChange={(w) => set({ with: w })} />
        </>
      ) : (
        <Field label={t("Commands")}>
          <CodeEditor
            value={step.run ?? ""}
            onChange={(run) => set({ run })}
            language="shell"
            lineNumbers={false}
            ariaLabel="Commands"
            className="h-24 resize-y"
          />
          <span className="text-[11.5px]">
            {tx("Never paste {github_event} here: put it in an env variable below and use {name}.", {
              github_event: <span className="font-mono">{"${{ github.event… }}"}</span>,
              name: <span className="font-mono">"$NAME"</span>,
            })}
          </span>
        </Field>
      )}
      <Pairs label={t("Environment variables")} value={step.env} onChange={(env) => set({ env })} />
    </div>
  );
}

/** Key/value rows (`with:`, `env:`). */
function Pairs({
  label,
  value,
  onChange,
}: {
  label: string;
  value: Record<string, string>;
  onChange: (v: Record<string, string>) => void;
}) {
  const entries = Object.entries(value);
  const rename = (from: string, to: string) =>
    onChange(Object.fromEntries(entries.map(([k, v]) => (k === from ? [to, v] : [k, v]))));
  return (
    <div className="flex flex-col gap-1 text-[12.5px] text-muted-foreground">
      {label}
      {entries.map(([k, v], i) => (
        <div key={i} className="flex gap-1.5">
          <Input
            value={k}
            onChange={(e) => rename(k, e.target.value)}
            className="h-7 w-44 font-mono text-[12px]"
            aria-label={t("Name")}
          />
          <Input
            value={v}
            onChange={(e) => onChange({ ...value, [k]: e.target.value })}
            className="h-7 flex-1 font-mono text-[12px]"
            aria-label={t("Value of {k}", { k })}
          />
          <Button
            size="icon-xs"
            variant="ghost"
            aria-label={t("Remove {k}", { k: k })}
            onClick={() => onChange(Object.fromEntries(entries.filter(([key]) => key !== k)))}
          >
            <Trash2 />
          </Button>
        </div>
      ))}
      <div>
        <Button
          size="xs"
          variant="ghost"
          onClick={() => {
            let n = entries.length + 1;
            while (`NAME_${n}` in value) n++;
            onChange({ ...value, [`NAME_${n}`]: "" });
          }}
        >
          <Plus data-icon="inline-start" /> {t("Add")}
        </Button>
      </div>
    </div>
  );
}
