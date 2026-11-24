import {
  BookOpen,
  Copy,
  FileCog,
  FolderOpen,
  History,
  RotateCcw,
  ShieldAlert,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";

import { CodeEditor, languageForPath } from "@/components/code-editor";
import { DiffView } from "@/components/git";
import { OpenInEditorButton } from "@/components/open-in-editor";
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
import { errorMessage } from "@/hooks/use-daemon";
import { request } from "@/lib/daemon";
import {
  useConfigActions,
  useConfigFile,
  useConfigFiles,
  useConfigFindings,
  useConfigHistory,
} from "@/hooks/use-config";
import type {
  ConfigChange,
  ConfigFileView,
  ConfigFinding,
  ConfigOption,
  ConfigPreview,
  FindingKind,
  JsonValue,
  ProjectView,
} from "@/lib/protocol";
import { copyText, openInBrowser, revealFolder } from "@/lib/system";
import { cn } from "@/lib/utils";
import { intlLocale, t, tn } from "@/lib/i18n";

const KIND: Record<FindingKind, { label: string; className: string }> = {
  incompatible: { label: "incompatible", className: "text-destructive" },
  deprecated: { label: "deprecated", className: "text-amber" },
  security: { label: "security", className: "text-destructive" },
  performance: { label: "performance", className: "text-amber" },
  development: { label: "development", className: "text-amber" },
  available: { label: "available", className: "text-signal" },
};

type Mode = "options" | "text" | "history";

/** What the user is about to save: the change, its preview, and the file's hash when it was reviewed. */
interface Review {
  path: string;
  change: ConfigChange;
  preview: ConfigPreview;
  baseHash: string;
  title: string;
  revertId?: string;
}

const show = (v: JsonValue) => (typeof v === "string" ? v : JSON.stringify(v));

export function SiteConfigTab({ site }: { site: ProjectView }) {
  const files = useConfigFiles(site.name);
  const findings = useConfigFindings(site.name);
  const actions = useConfigActions(site.name);
  const [selected, setSelected] = useState<string | null>(null);
  const [review, setReview] = useState<Review | null>(null);
  const list = files.data ?? [];
  const current = selected ?? list[0]?.path ?? null;

  const open = async (path: string, change: ConfigChange, baseHash: string, title: string, revertId?: string) => {
    try {
      const preview = await actions.preview(path, change);
      if (!preview.diff) {
        toast(t("No change: the file already says that."));
        return;
      }
      setReview({ path, change, preview, baseHash, title, revertId });
    } catch (e) {
      toast.error(t("LocDev can't make that change"), { description: errorMessage(e) });
    }
  };

  if (files.isPending) return <Skeleton className="h-64" />;
  if (files.isError) return <p className="text-sm text-destructive">{errorMessage(files.error)}</p>;
  if (list.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("No config files LocDev knows in this site. Nothing is created to fill this page.")}
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <Findings
        findings={findings.data ?? []}
        loading={findings.isPending}
        onFix={async (f) => {
          if (!f.fix) return;
          const [key, value] = f.fix;
          setSelected(f.file);
          const view = await fetchView(site.name, f.file);
          if (view) open(f.file, { type: "set", key, value }, view.hash, `${key} → ${show(value)}`);
        }}
      />
      <div className="flex min-h-[420px] gap-5">
        <nav aria-label={t("Config files")} className="flex w-60 shrink-0 flex-col gap-0.5">
          {list.map((f) => (
            <button
              type="button"
              key={f.path}
              onClick={() => setSelected(f.path)}
              className={cn(
                "flex flex-col items-start rounded-md px-2.5 py-1.5 text-left hover:bg-foreground/[0.04]",
                current === f.path && "bg-foreground/[0.07]",
              )}
            >
              <span className="w-full truncate font-mono text-[12.5px]">{f.path}</span>
              <span className="text-[11.5px] text-muted-foreground">
                {f.label}
                {f.version ? ` · ${f.package} ${f.version}` : ""}
                {f.visual ? "" : t(" · text")}
              </span>
            </button>
          ))}
        </nav>
        {current && (
          <FilePanel
            key={current}
            site={site}
            path={current}
            onChange={(view, change, title) => open(view.file.path, change, view.hash, title)}
            onRevert={(view, id, content) =>
              open(view.file.path, { type: "text", content }, view.hash, t("revert to {id}", { id }), id)
            }
          />
        )}
      </div>
      <ReviewDialog
        review={review}
        busy={actions.save.isPending || actions.revert.isPending}
        onCancel={() => setReview(null)}
        onSave={() => {
          if (!review) return;
          const done = {
            onSuccess: () => {
              toast.success(`Saved ${review.path}`, { description: t("The previous version is in its history.") });
              setReview(null);
            },
            onError: (e: unknown) => toast.error(t("Not saved"), { description: errorMessage(e) }),
          };
          if (review.revertId)
            actions.revert.mutate({ path: review.path, id: review.revertId, baseHash: review.baseHash }, done);
          else actions.save.mutate({ path: review.path, change: review.change, baseHash: review.baseHash }, done);
        }}
      />
    </div>
  );
}

/** A fresh read for fixes started from the analysis (the hash must be current). */
async function fetchView(site: string, path: string): Promise<ConfigFileView | null> {
  try {
    const { type: _, ...view } = await request({ type: "config_file", name: site, path }, "config_file");
    return view;
  } catch (e) {
    toast.error(t("Could not read the file"), { description: errorMessage(e) });
    return null;
  }
}

function Findings({
  findings,
  loading,
  onFix,
}: {
  findings: ConfigFinding[];
  loading: boolean;
  onFix: (f: ConfigFinding) => void;
}) {
  return (
    <section aria-label={t("Analysis")} className="rounded-lg border">
      <div className="flex items-center gap-2 border-b px-4 py-2">
        <ShieldAlert className="size-4 text-muted-foreground" />
        <h3 className="text-[13px] font-medium">{t("Analysis")}</h3>
        <span className="text-[12px] text-muted-foreground">
          {loading
            ? t("checking…")
            : findings.length === 0
              ? t("nothing found")
              : tn("{n} finding", "{n} findings", findings.length)}
        </span>
        <span className="ml-auto text-[11.5px] text-muted-foreground">
          {t("Rules checked against the installed versions; nothing changes without you.")}
        </span>
      </div>
      {findings.length > 0 && (
        <ul className="divide-y">
          {findings.map((f) => (
            <li key={`${f.rule}-${f.file}-${f.key}`} className="flex flex-col gap-1 px-4 py-2.5">
              <div className="flex items-baseline gap-2">
                <span className={cn("font-mono text-[11px]", KIND[f.kind].className)}>{KIND[f.kind].label}</span>
                <span className="text-[13px] font-medium">{f.title}</span>
                <span className="font-mono text-[11.5px] text-muted-foreground">{f.file}</span>
                {f.fix && (
                  <Button size="xs" variant="outline" className="ml-auto" onClick={() => onFix(f)}>
                    <Sparkles data-icon="inline-start" /> {t("Review fix")}
                  </Button>
                )}
              </div>
              <p className="text-[12.5px] text-muted-foreground">{f.detail}</p>
              <p className="flex gap-3 font-mono text-[11px] text-muted-foreground">
                <span>{f.applies}</span>
                <button
                  type="button"
                  onClick={() => openInBrowser(f.source)}
                  className="truncate text-ember hover:underline"
                >
                  {f.source}
                </button>
              </p>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function FilePanel({
  site,
  path,
  onChange,
  onRevert,
}: {
  site: ProjectView;
  path: string;
  onChange: (view: ConfigFileView, change: ConfigChange, title: string) => void;
  onRevert: (view: ConfigFileView, id: string, content: string) => void;
}) {
  const file = useConfigFile(site.name, path);
  const [mode, setMode] = useState<Mode | null>(null);
  if (file.isPending) return <Skeleton className="h-64 flex-1" />;
  if (file.isError || !file.data) return <p className="flex-1 text-sm text-destructive">{errorMessage(file.error)}</p>;
  const view = file.data;
  const visual = view.options.length > 0;
  const active: Mode = mode ?? (visual ? "options" : "text");
  const full = `${site.path.replace(/[\\/]$/, "")}/${view.file.path}`;

  return (
    <div className="flex min-w-0 flex-1 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <FileCog className="size-4 text-muted-foreground" />
        <span className="font-mono text-[13px] font-medium">{view.file.path}</span>
        {view.file.package && (
          <span className="font-mono text-[11.5px] text-muted-foreground">
            {view.file.package} {view.file.version ?? t("(not installed)")}
          </span>
        )}
        <span className="ml-auto flex items-center gap-1">
          <Button size="xs" variant="ghost" onClick={() => openInBrowser(view.file.docs)}>
            <BookOpen data-icon="inline-start" /> {t("Docs")}
          </Button>
          <OpenInEditorButton site={site.name} path={view.file.path} />
          <Button size="xs" variant="ghost" onClick={() => revealFolder(full)} title={t("Show the file in its folder")}>
            <FolderOpen data-icon="inline-start" /> {t("Show")}
          </Button>
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Copy path")}
            onClick={() => copyText(full, t("Path copied"))}
          >
            <Copy />
          </Button>
        </span>
      </div>
      <div role="tablist" aria-label={t("View")} className="flex gap-1">
        {(["options", "text", "history"] as const)
          .filter((m) => m !== "options" || visual)
          .map((m) => (
            <button
              type="button"
              role="tab"
              key={m}
              aria-selected={active === m}
              onClick={() => setMode(m)}
              className={cn(
                "rounded-md px-2.5 py-1 font-mono text-[11.5px] text-muted-foreground hover:text-foreground",
                active === m && "bg-foreground/[0.07] text-foreground",
              )}
            >
              {m}
            </button>
          ))}
      </div>
      {view.syntax_error && (
        <p className="flex items-center gap-1.5 text-[12.5px] text-destructive">
          <TriangleAlert className="size-3.5" /> {view.syntax_error}
        </p>
      )}
      {active === "options" ? (
        <OptionsForm view={view} onChange={(change, title) => onChange(view, change, title)} />
      ) : active === "text" ? (
        <TextEditor view={view} onReview={(content) => onChange(view, { type: "text", content }, t("text edit"))} />
      ) : (
        <HistoryList site={site.name} view={view} onRevert={(id, content) => onRevert(view, id, content)} />
      )}
    </div>
  );
}

function OptionsForm({
  view,
  onChange,
}: {
  view: ConfigFileView;
  onChange: (change: ConfigChange, title: string) => void;
}) {
  const sections = useMemo(() => {
    const map = new Map<string, ConfigOption[]>();
    for (const o of view.options) map.set(o.section, [...(map.get(o.section) ?? []), o]);
    return [...map.entries()];
  }, [view.options]);
  const set = (o: ConfigOption, value: JsonValue) =>
    onChange({ type: "set", key: o.key, value }, `${o.key} → ${show(value)}`);

  return (
    <div className="flex flex-col gap-4">
      {view.visual_blocked && <p className="text-[12.5px] text-amber">{view.visual_blocked}</p>}
      {view.notes.map((n) => (
        <p key={n} className="font-mono text-[11.5px] text-muted-foreground">
          {n}
        </p>
      ))}
      {sections.map(([section, options]) => (
        <section key={section} aria-label={section}>
          <h4 className="mb-1 font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{section}</h4>
          <div className="divide-y rounded-lg border">
            {options.map((o) => (
              <div key={o.key} className="grid grid-cols-[1fr_auto] items-center gap-x-4 gap-y-0.5 px-3 py-2">
                <div className="min-w-0">
                  <p className="text-[13px]">
                    {o.label} <span className="font-mono text-[11px] text-muted-foreground">{o.key}</span>
                  </p>
                  <p className="text-[12px] text-muted-foreground">{o.description}</p>
                  <p className="flex flex-wrap gap-x-3 font-mono text-[10.5px] text-muted-foreground">
                    {o.default !== null && (
                      <span>
                        {t("default")} {show(o.default)}
                      </span>
                    )}
                    {o.example && <span>e.g. {o.example}</span>}
                    {o.versions && <span>{o.versions}</span>}
                    {o.docs && (
                      <button
                        type="button"
                        onClick={() => openInBrowser(o.docs!)}
                        className="text-ember hover:underline"
                      >
                        {t("docs")}
                      </button>
                    )}
                  </p>
                  {o.state.state === "code" && (
                    <p className="mt-0.5 truncate font-mono text-[11.5px]" title={o.state.code}>
                      = {o.state.code}
                    </p>
                  )}
                  {o.blocked && <p className="text-[11.5px] text-amber">{o.blocked}</p>}
                </div>
                <OptionControl option={o} onSet={(v) => set(o, v)} />
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

function OptionControl({ option: o, onSet }: { option: ConfigOption; onSet: (value: JsonValue) => void }) {
  const current = o.state.state === "set" ? o.state.value : null;
  const [draft, setDraft] = useState(current === null ? "" : show(current));
  useEffect(() => setDraft(current === null ? "" : show(current)), [current]);
  const disabled = !o.editable;

  if (o.kind === "bool") {
    const value = typeof current === "boolean" ? current : o.default === true;
    return (
      <Switch
        aria-label={o.label}
        checked={value}
        disabled={disabled}
        onCheckedChange={(next) => onSet(next)}
        className="data-checked:bg-ember"
      />
    );
  }
  if (o.kind === "choice") {
    const items = o.choices.map((c) => ({ value: JSON.stringify(c), label: show(c) }));
    return (
      <Select
        items={items}
        value={current === null ? null : JSON.stringify(current)}
        onValueChange={(v) => v && onSet(JSON.parse(String(v)))}
        disabled={disabled}
      >
        <SelectTrigger size="sm" aria-label={o.label} className="h-7 min-w-32 font-mono text-[12px]">
          <SelectValue placeholder={o.default !== null ? show(o.default) : t("not set")} />
        </SelectTrigger>
        <SelectContent>
          {items.map((i) => (
            <SelectItem key={i.value} value={i.value} className="font-mono text-[12px]">
              {i.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    );
  }
  const commit = () => {
    if (draft === (current === null ? "" : show(current)) || draft === "") return;
    if (o.kind === "number") {
      const n = Number(draft);
      if (!Number.isFinite(n)) {
        toast.error(t("{label} must be a number", { label: o.label }));
        return;
      }
      onSet(n);
    } else onSet(draft);
  };
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        commit();
      }}
      className="flex items-center gap-1"
    >
      <Input
        aria-label={o.label}
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        disabled={disabled}
        inputMode={o.kind === "number" ? "numeric" : undefined}
        placeholder={o.default !== null ? show(o.default) : (o.example ?? "")}
        className="h-7 w-40 font-mono text-[12px]"
      />
      <Button
        type="submit"
        size="xs"
        variant="outline"
        disabled={disabled || draft === (current === null ? "" : show(current)) || draft === ""}
      >
        {t("Review")}
      </Button>
    </form>
  );
}

function TextEditor({ view, onReview }: { view: ConfigFileView; onReview: (content: string) => void }) {
  const [text, setText] = useState(view.content);
  const changed = text !== view.content;
  return (
    <div className="flex flex-col gap-2">
      <CodeEditor
        value={text}
        onChange={setText}
        language={languageForPath(view.file.path)}
        ariaLabel={`${view.file.path} text`}
        onSave={() => {
          if (changed) onReview(text);
        }}
        className="h-[28rem] resize-y"
      />
      <div className="flex items-center gap-2">
        <Button size="sm" disabled={!changed} onClick={() => onReview(text)}>
          {t("Review changes…")}
        </Button>
        <Button size="sm" variant="ghost" disabled={!changed} onClick={() => setText(view.content)}>
          {t("Discard")}
        </Button>
        <span className="ml-auto text-[11.5px] text-muted-foreground">
          {t("The syntax is checked before saving; Ctrl+S reviews.")}
        </span>
      </div>
    </div>
  );
}

function HistoryList({
  site,
  view,
  onRevert,
}: {
  site: string;
  view: ConfigFileView;
  onRevert: (id: string, content: string) => void;
}) {
  const history = useConfigHistory(site, view.file.path);
  const { version } = useConfigActions(site);
  if (history.isPending) return <Skeleton className="h-24" />;
  const entries = history.data ?? [];
  if (entries.length === 0) {
    return (
      <p className="text-[12.5px] text-muted-foreground">
        {t("Nothing kept yet. Before every save made through LocDev, the previous content is kept here.")}
      </p>
    );
  }
  return (
    <ul className="divide-y rounded-lg border">
      {entries.map((e) => (
        <li key={e.id} className="flex items-center gap-3 px-3 py-2 text-[12.5px]">
          <History className="size-3.5 text-muted-foreground" />
          <span className="font-mono text-[11.5px]">{new Date(e.at).toLocaleString(intlLocale)}</span>
          <span className="truncate text-muted-foreground">{t("replaced by: {reason}", { reason: e.reason })}</span>
          <Button
            size="xs"
            variant="outline"
            className="ml-auto"
            onClick={async () => {
              try {
                onRevert(e.id, await version(view.file.path, e.id));
              } catch (err) {
                toast.error(t("Could not read that version"), { description: errorMessage(err) });
              }
            }}
          >
            <RotateCcw data-icon="inline-start" /> {t("Compare & revert…")}
          </Button>
        </li>
      ))}
    </ul>
  );
}

function ReviewDialog({
  review,
  busy,
  onCancel,
  onSave,
}: {
  review: Review | null;
  busy: boolean;
  onCancel: () => void;
  onSave: () => void;
}) {
  const p = review?.preview;
  return (
    <Dialog open={review !== null} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>
            {t("Review")} {review?.path}
          </DialogTitle>
          <DialogDescription>
            {review?.title}
            {t(". Only the lines below change; the current version goes to the file's history.")}
          </DialogDescription>
        </DialogHeader>
        {p && (
          <div className="min-w-0 overflow-x-auto">
            <DiffView text={p.diff} />
          </div>
        )}
        {p?.syntax_error && (
          <p className="text-[12.5px] text-destructive">
            {t("It wouldn't parse: {syntax_error}", { syntax_error: p.syntax_error })}
          </p>
        )}
        {p && p.removed_comments.length > 0 && (
          <div className="text-[12.5px] text-amber">
            <p>{t("These comments would be removed:")}</p>
            <ul className="font-mono text-[11.5px]">
              {p.removed_comments.map((c) => (
                <li key={c}>{c.trim()}</li>
              ))}
            </ul>
          </div>
        )}
        <DialogFooter>
          <Button variant="ghost" onClick={onCancel}>
            {t("Cancel")}
          </Button>
          <Button
            onClick={onSave}
            disabled={busy || !!p?.syntax_error}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            {review?.revertId ? t("Revert") : t("Save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
