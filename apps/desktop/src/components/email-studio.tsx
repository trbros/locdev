import {
  ArrowDown,
  ArrowUp,
  Code2,
  Columns2,
  Copy,
  Eye,
  GripVertical,
  Heading,
  Image,
  LayoutPanelTop,
  Minus,
  MousePointerClick,
  PanelBottom,
  Plus,
  Puzzle,
  Redo2,
  RefreshCw,
  Save,
  Send,
  Space,
  SquareDashed,
  Trash2,
  TriangleAlert,
  Type,
  Undo2,
} from "lucide-react";
import { type DragEvent, type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";

import { CodeEditor, CodeView, languageForPath } from "@/components/code-editor";
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
import { buildEmail, emailStat, openEmail, previewEmail, useEmailActions, useEmailCatalog } from "@/hooks/use-email";
import type {
  EmailCatalog,
  EmailComponent,
  EmailDoc,
  EmailPreview,
  JsonValue,
  PreviewMode,
  ProjectView,
  Segment,
} from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t, tn } from "@/lib/i18n";

type BlockSegment = Extract<Segment, { kind: "block" }>;
type Props = Record<string, JsonValue>;

const VISUAL = new Set(["blade", "markdown", "html"]);
const CSP = "default-src 'none'; img-src data: https: http:; style-src 'unsafe-inline'; font-src data:";

const BLOCKS: Record<string, { label: string; icon: typeof Type }> = {
  header: { label: t("Header"), icon: LayoutPanelTop },
  heading: { label: t("Heading"), icon: Heading },
  text: { label: t("Text"), icon: Type },
  button: { label: t("Button"), icon: MousePointerClick },
  image: { label: t("Image"), icon: Image },
  columns: { label: t("Columns"), icon: Columns2 },
  divider: { label: t("Divider"), icon: Minus },
  spacer: { label: t("Spacer"), icon: Space },
  footer: { label: t("Footer"), icon: PanelBottom },
  panel: { label: t("Panel"), icon: SquareDashed },
  component: { label: t("Component"), icon: Puzzle },
};

function palette(format: string) {
  return format === "markdown"
    ? ["heading", "text", "button", "panel", "image", "divider"]
    : ["header", "heading", "text", "button", "image", "columns", "divider", "spacer", "footer"];
}

function defaults(block: string, format: string): Props {
  const blade = format !== "html";
  switch (block) {
    case "header":
      return { title: blade ? "{{ config('app.name') }}" : "Your app", align: "center" };
    case "heading":
      return { text: "Heading", level: 1 };
    case "text":
      return { text: "Write something here." };
    case "button":
      return { text: "Open", url: blade ? "{{ $url }}" : "https://example.com" };
    case "image":
      return { src: "https://placehold.co/536x200/png", alt: "", width: 536 };
    case "columns":
      return { cells: ["Left column", "Right column"] };
    case "spacer":
      return { height: 24 };
    case "footer":
      return { text: blade ? "© {{ date('Y') }} {{ config('app.name') }}" : "© Your app", align: "center" };
    case "panel":
      return { text: "Something to point out." };
    default:
      return {};
  }
}

let counter = 0;
const newId = () => `new-${Date.now().toString(36)}-${counter++}`;

function summary(s: Segment, components: EmailComponent[]): string {
  if (s.kind === "code") {
    const line =
      s.code
        .split("\n")
        .map((l) => l.trim())
        .find((l) => l && !l.startsWith("{{--")) ?? "";
    return line.length > 48 ? `${line.slice(0, 48)}…` : line || t("(blank)");
  }
  const p = s.props as Record<string, string>;
  if (s.block === "component") return `<${p.tag}>`;
  const label = p.text ?? p.title ?? (s.block === "image" ? p.src : "");
  void components;
  return typeof label === "string" ? (label.length > 40 ? `${label.slice(0, 40)}…` : label) : "";
}

function srcdoc(html: string, selected: string | null) {
  const highlight = selected
    ? `<style>[data-locdev-block="${selected}"]{outline:2px solid #ea580c;outline-offset:-2px}</style>`
    : "";
  const meta = `<meta http-equiv="Content-Security-Policy" content="${CSP}">${highlight}`;
  return /<head[^>]*>/i.test(html) ? html.replace(/<head[^>]*>/i, (h) => `${h}${meta}`) : `${meta}${html}`;
}

/** Projetos → site → Email Studio. */
export function SiteEmailTab({ site }: { site: ProjectView }) {
  const catalog = useEmailCatalog(site.name);
  const [path, setPath] = useState<string | null>(null);
  if (catalog.isPending) return <Skeleton className="h-96" />;
  if (catalog.isError || !catalog.data)
    return <p className="text-sm text-destructive">{errorMessage(catalog.error)}</p>;
  const c = catalog.data;
  const current = path ?? c.templates.find((t) => VISUAL.has(t.format))?.path ?? c.templates[0]?.path ?? null;

  return (
    <div className="flex min-h-[640px] flex-col gap-3">
      <TemplatePicker site={site} catalog={c} current={current} onOpen={setPath} />
      <div className="min-w-0">
        {current ? (
          <Editor key={current} site={site} path={current} catalog={c} />
        ) : (
          <div className="rounded-lg border p-6 text-[13px] text-muted-foreground">
            <p>{t("No email templates in this site yet.")}</p>
            <p className="mt-1">
              {t("Create one on the left, or with")} <code>locdev new email &lt;name&gt;</code>.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function TemplatePicker({
  site,
  catalog,
  current,
  onOpen,
}: {
  site: ProjectView;
  catalog: EmailCatalog;
  current: string | null;
  onOpen: (p: string) => void;
}) {
  const { create } = useEmailActions(site.name);
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");
  const [component, setComponent] = useState(false);
  const [markdown, setMarkdown] = useState(false);
  const laravel = catalog.conventions.some((c) => c.includes("resources/views"));
  const own = catalog.components.filter((c) => !c.builtin);
  const items = catalog.templates.map((t) => ({
    value: t.path,
    label: `${t.name} · ${t.format}${t.mailables.length ? ` · ${t.mailables.map((m) => m.class.split("\\").pop()).join(", ")}` : ""}`,
  }));

  return (
    <div className="flex flex-wrap items-center gap-2">
      {items.length > 0 && (
        <Select items={items} value={current} onValueChange={(v) => v && onOpen(String(v))}>
          <SelectTrigger size="sm" aria-label={t("Template")} className="h-8 min-w-80 font-mono text-[12.5px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {items.map((i) => (
              <SelectItem key={i.value} value={i.value} className="font-mono text-[12px]">
                {i.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
      <Button size="sm" variant="outline" onClick={() => setOpen(true)}>
        <Plus data-icon="inline-start" /> {t("New template or component")}
      </Button>
      <span
        className="text-[11.5px] text-muted-foreground"
        title={own.map((c) => `<${c.usage}> ${c.props.map((p) => p.name).join(", ")}`).join("\n")}
      >
        {tn("{n} template", "{n} templates", catalog.templates.length)} ·{" "}
        {tn("{n} component", "{n} components", own.length)} · {catalog.framework}
      </span>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("New")} {component ? t("component") : t("template")}
            </DialogTitle>
            <DialogDescription>
              {laravel && !component
                ? t(
                    "LocDev runs `php artisan make:mail` for the Mailable class, then fills the Blade view with editable blocks.",
                  )
                : catalog.conventions.join(" · ")}{" "}
              {t("Existing files are never overwritten.")}
            </DialogDescription>
          </DialogHeader>
          <form
            aria-label={t("New email")}
            className="flex flex-col gap-3"
            onSubmit={(e) => {
              e.preventDefault();
              if (!name.trim()) return;
              create.mutate(
                { component, item: name.trim(), markdown: !component && markdown },
                {
                  onSuccess: (r) => {
                    toast.success(tn("Created {n} file", "Created {n} files", r.files.length), {
                      description: [r.ran, ...r.files].filter(Boolean).join("\n"),
                    });
                    setName("");
                    setOpen(false);
                    const view = r.files.find((f) => !f.startsWith("app/") && !f.includes("/components/"));
                    if (view) onOpen(view);
                  },
                  onError: (e) => toast.error(t("Not created"), { description: errorMessage(e) }),
                },
              );
            }}
          >
            <Input
              aria-label={t("New email name")}
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={component ? "button" : "order-shipped"}
              className="font-mono"
            />
            <label className="flex items-center gap-2 text-[12.5px]">
              <Switch
                aria-label={t("Component")}
                checked={component}
                onCheckedChange={setComponent}
                className="data-checked:bg-ember"
              />{" "}
              {t("A reusable component")}
            </label>
            {laravel && !component && (
              <label className="flex items-center gap-2 text-[12.5px]">
                <Switch
                  aria-label={t("Markdown mailable")}
                  checked={markdown}
                  onCheckedChange={setMarkdown}
                  className="data-checked:bg-ember"
                />{" "}
                {t("Markdown mailable")}
              </label>
            )}
            <DialogFooter>
              <Button
                type="submit"
                disabled={!name.trim() || create.isPending}
                className="bg-ember text-ember-foreground hover:bg-ember/85"
              >
                {t("Create")}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface State {
  segments: Segment[];
  theme: Props;
}

function Editor({ site, path, catalog }: { site: ProjectView; path: string; catalog: EmailCatalog }) {
  const actions = useEmailActions(site.name);
  const [doc, setDoc] = useState<EmailDoc | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [state, setState] = useState<State>({ segments: [], theme: {} });
  const [past, setPast] = useState<State[]>([]);
  const [future, setFuture] = useState<State[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [sample, setSample] = useState("{}");
  const [mode, setMode] = useState<PreviewMode>("static");
  const [width, setWidth] = useState<"desktop" | "mobile">("desktop");
  const [view, setView] = useState<"preview" | "html" | "text" | "file">("preview");
  const [preview, setPreview] = useState<EmailPreview | null>(null);
  const [external, setExternal] = useState<string | null>(null);
  const [review, setReview] = useState<{ diff: string; content: string } | null>(null);
  const baseHash = useRef("");
  const dirty = past.length > 0;

  const load = useCallback(async () => {
    try {
      const d = await openEmail(site.name, path);
      setDoc(d);
      setState({ segments: d.segments, theme: d.theme as Props });
      setPast([]);
      setFuture([]);
      setSample(JSON.stringify(d.sample, null, 2));
      baseHash.current = d.hash;
      setExternal(null);
      setLoadError(null);
    } catch (e) {
      setLoadError(errorMessage(e));
    }
  }, [site.name, path]);

  useEffect(() => {
    void load();
  }, [load]);

  const data = useMemo<JsonValue>(() => {
    try {
      return JSON.parse(sample);
    } catch {
      return null;
    }
  }, [sample]);

  // Live preview, a moment after each change.
  useEffect(() => {
    if (!doc || data === null) return;
    const timer = setTimeout(() => {
      previewEmail(site.name, path, state.segments, state.theme, data, mode)
        .then(setPreview)
        .catch((e) => setPreview({ html: "", text: null, content: "", notes: [], error: errorMessage(e) }));
    }, 250);
    return () => clearTimeout(timer);
  }, [doc, state, data, mode, site.name, path]);

  // Edits made in another editor: reload when there's nothing to lose, else warn.
  useEffect(() => {
    if (!doc) return;
    const timer = setInterval(async () => {
      try {
        const hash = await emailStat(site.name, path);
        if (hash === baseHash.current) return;
        if (!dirty) {
          await load();
          toast(t("{path} changed on disk", { path }), { description: t("Reloaded: you had no unsaved changes.") });
        } else {
          setExternal(hash);
        }
      } catch {
        // The file may be mid-write; the next tick tries again.
      }
    }, 2000);
    return () => clearInterval(timer);
  }, [doc, dirty, load, site.name, path]);

  const change = (next: State) => {
    setPast((p) => [...p.slice(-99), state]);
    setFuture([]);
    setState(next);
  };
  const setSegments = (segments: Segment[]) => change({ ...state, segments });
  const undo = () => {
    const prev = past[past.length - 1];
    if (!prev) return;
    setFuture((f) => [state, ...f]);
    setPast((p) => p.slice(0, -1));
    setState(prev);
  };
  const redo = () => {
    const next = future[0];
    if (!next) return;
    setPast((p) => [...p, state]);
    setFuture((f) => f.slice(1));
    setState(next);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || (e.target as HTMLElement)?.closest("input, textarea")) return;
      if (e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undo();
      } else if (e.key === "y" || (e.key === "z" && e.shiftKey)) {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const startSave = async () => {
    try {
      const built = await buildEmail(site.name, path, state.segments, state.theme);
      if (!built.diff) {
        toast(t("No changes to save."));
        return;
      }
      setReview({ diff: built.diff, content: built.content });
    } catch (e) {
      toast.error(t("Can't save"), { description: errorMessage(e) });
    }
  };

  if (loadError) return <p className="text-sm text-destructive">{loadError}</p>;
  if (!doc) return <Skeleton className="h-96" />;
  const visual = VISUAL.has(doc.format);
  const selectedSegment = state.segments.find((s) => s.id === selected) ?? null;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-mono text-[13px] font-medium">{doc.path}</span>
        <span className="font-mono text-[11px] text-muted-foreground">{doc.format}</span>
        {dirty && <span className="font-mono text-[11px] text-amber">{t("unsaved")}</span>}
        <span className="ml-auto flex items-center gap-1">
          <OpenInEditorButton site={site.name} path={doc.path} />
          <Button size="icon-sm" variant="ghost" aria-label={t("Undo")} disabled={!past.length} onClick={undo}>
            <Undo2 />
          </Button>
          <Button size="icon-sm" variant="ghost" aria-label={t("Redo")} disabled={!future.length} onClick={redo}>
            <Redo2 />
          </Button>
          <Select
            items={[
              { value: "static", label: t("Static preview") },
              { value: "framework", label: t("Framework preview") },
            ]}
            value={mode}
            onValueChange={(v) => v && setMode(v as PreviewMode)}
          >
            <SelectTrigger size="sm" aria-label={t("Preview mode")} className="h-7 w-40 text-[12px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="static">{t("Static preview")}</SelectItem>
              <SelectItem value="framework" disabled={!catalog.framework_preview}>
                {t("Framework preview")}
              </SelectItem>
            </SelectContent>
          </Select>
          <Button
            size="sm"
            variant="outline"
            disabled={actions.sendTest.isPending || data === null}
            onClick={() =>
              actions.sendTest.mutate(
                { path, segments: state.segments, theme: state.theme, data, mode },
                {
                  onSuccess: () => toast.success(t("Sent to the Mailbox"), { description: t("Tools → Mailbox") }),
                  onError: (e) => toast.error(t("Not sent"), { description: errorMessage(e) }),
                },
              )
            }
          >
            <Send data-icon="inline-start" /> {t("Send test")}
          </Button>
          <Button
            size="sm"
            disabled={!dirty}
            onClick={startSave}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <Save data-icon="inline-start" /> {t("Review & save")}
          </Button>
        </span>
      </div>
      {external && (
        <div
          role="alert"
          className="flex items-center gap-2 rounded-md border border-amber/40 bg-amber/5 px-3 py-2 text-[12.5px]"
        >
          <TriangleAlert className="size-4 text-amber" />
          <span>{t("{path} changed in another editor while you have unsaved changes here.", { path })}</span>
          <span className="ml-auto flex gap-1">
            <Button size="xs" variant="outline" onClick={load}>
              <RefreshCw data-icon="inline-start" /> {t("Load theirs")}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              onClick={() => {
                baseHash.current = external;
                setExternal(null);
                toast(t("Keeping yours"), {
                  description: t("Saving will show the diff against the file as it is now."),
                });
              }}
            >
              {t("Keep mine")}
            </Button>
          </span>
        </div>
      )}
      {doc.notes.map((n) => (
        <p key={n} className="text-[12px] text-muted-foreground">
          {n}
        </p>
      ))}
      <div className="flex min-h-[600px] gap-3">
        <div className="flex w-80 shrink-0 flex-col gap-4">
          <Structure
            doc={doc}
            catalog={catalog}
            segments={state.segments}
            visual={visual}
            selected={selected}
            onSelect={setSelected}
            onChange={setSegments}
          />
          <Inspector
            doc={doc}
            catalog={catalog}
            segment={selectedSegment}
            theme={state.theme}
            sample={sample}
            sampleValid={data !== null}
            onSample={setSample}
            onSampleSave={() =>
              data !== null &&
              actions.sample(path, data).then(() => toast.success(t("Sample data kept for this template")))
            }
            onTheme={(theme) => change({ ...state, theme })}
            onSegment={(next) => setSegments(state.segments.map((s) => (s.id === next.id ? next : s)))}
          />
        </div>
        <div className="flex min-w-0 flex-1 flex-col rounded-lg border">
          <div className="flex items-center gap-1 border-b px-2 py-1">
            <div role="tablist" aria-label={t("Output")} className="flex items-center gap-1">
              {(["preview", "html", "text", "file"] as const).map((v) => (
                <button
                  type="button"
                  role="tab"
                  key={v}
                  aria-selected={view === v}
                  onClick={() => setView(v)}
                  className={cn(
                    "rounded px-2 py-0.5 font-mono text-[11.5px] text-muted-foreground",
                    view === v && "bg-foreground/[0.07] text-foreground",
                  )}
                >
                  {v === "file" ? t("file") : v}
                </button>
              ))}
            </div>
            {view === "preview" && (
              <span className="ml-auto flex gap-1">
                {(["desktop", "mobile"] as const).map((w) => (
                  <button
                    type="button"
                    key={w}
                    aria-pressed={width === w}
                    onClick={() => setWidth(w)}
                    className={cn(
                      "rounded px-1.5 font-mono text-[11px] text-muted-foreground",
                      width === w && "bg-foreground/[0.07] text-foreground",
                    )}
                  >
                    {w}
                  </button>
                ))}
              </span>
            )}
          </div>
          {preview?.error ? (
            <pre className="m-3 rounded-md bg-destructive/10 p-3 font-mono text-[12px] whitespace-pre-wrap text-destructive">
              {preview.error}
            </pre>
          ) : !preview ? (
            <Skeleton className="m-3 h-64" />
          ) : view === "preview" ? (
            <div className="flex-1 bg-white">
              {/* No scripts, no navigation; only images load. */}
              <iframe
                title={t("Email preview")}
                sandbox=""
                srcDoc={srcdoc(preview.html, mode === "static" ? selected : null)}
                className={cn(
                  "mx-auto h-full min-h-[520px] border-0",
                  width === "mobile" ? "w-[375px] border-x" : "w-full",
                )}
              />
            </div>
          ) : (
            <CodeView
              value={view === "html" ? preview.html : view === "text" ? (preview.text ?? "") : preview.content}
              language={view === "html" ? "html" : view === "text" ? "text" : languageForPath(path)}
              wrap
              ariaLabel={view === "html" ? t("Generated HTML") : view === "text" ? t("Text version") : t("File")}
              className="m-0 flex-1 rounded-none border-0"
            />
          )}
          {preview?.notes.map((n) => (
            <p key={n} className="border-t px-3 py-1.5 text-[11.5px] text-muted-foreground">
              {n}
            </p>
          ))}
        </div>
      </div>
      <Dialog open={review !== null} onOpenChange={(open) => !open && setReview(null)}>
        <DialogContent className="sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{t("Save {path}", { path })}</DialogTitle>
            <DialogDescription>
              {t(
                "Only these lines change; Blade and code outside LocDev blocks are written back exactly. The current version goes to the history.",
              )}
            </DialogDescription>
          </DialogHeader>
          {review && (
            // Long generated lines scroll here instead of widening the dialog.
            <div className="min-w-0 overflow-x-auto">
              <DiffView text={review.diff} />
            </div>
          )}
          <DialogFooter>
            <Button variant="ghost" onClick={() => setReview(null)}>
              {t("Cancel")}
            </Button>
            <Button
              className="bg-ember text-ember-foreground hover:bg-ember/85"
              disabled={actions.save.isPending}
              onClick={() =>
                review &&
                actions.save.mutate(
                  { path, content: review.content, baseHash: baseHash.current },
                  {
                    onSuccess: async () => {
                      toast.success(`Saved ${path}`);
                      setReview(null);
                      await load();
                    },
                    onError: (e) => toast.error(t("Not saved"), { description: errorMessage(e) }),
                  },
                )
              }
            >
              {t("Save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Structure({
  doc,
  catalog,
  segments,
  visual,
  selected,
  onSelect,
  onChange,
}: {
  doc: EmailDoc;
  catalog: EmailCatalog;
  segments: Segment[];
  visual: boolean;
  selected: string | null;
  onSelect: (id: string | null) => void;
  onChange: (segments: Segment[]) => void;
}) {
  const [over, setOver] = useState<{ index: number } | null>(null);
  const components = catalog.components.filter((c) =>
    doc.format === "markdown" ? true : !c.builtin && doc.format !== "html",
  );

  const insert = (index: number, segment: Segment) => {
    const next = [...segments];
    next.splice(index, 0, segment);
    onChange(next);
    onSelect(segment.id);
  };
  const move = (id: string, index: number) => {
    const from = segments.findIndex((s) => s.id === id);
    if (from < 0) return;
    const next = [...segments];
    const [item] = next.splice(from, 1);
    next.splice(index > from ? index - 1 : index, 0, item);
    onChange(next);
  };
  const newBlock = (kind: string): Segment => {
    if (kind.startsWith("component:")) {
      const usage = kind.slice("component:".length);
      const c = catalog.components.find((x) => x.usage === usage);
      const props: Props = {};
      for (const p of c?.props ?? []) props[p.name] = p.default ? p.default.replace(/^'(.*)'$/, "$1") : "";
      return {
        kind: "block",
        id: newId(),
        block: "component",
        props: { tag: usage, props, slot: c?.slot ? t("Button text") : "" },
        edited: false,
      };
    }
    return { kind: "block", id: newId(), block: kind, props: defaults(kind, doc.format), edited: false };
  };
  // Blocks go between the layout's opening and closing code by default.
  const defaultIndex = () => {
    const lastBlock = segments.map((s) => s.kind).lastIndexOf("block");
    return lastBlock >= 0 ? lastBlock + 1 : Math.max(segments.length - 1, 0);
  };
  const drop = (e: DragEvent, index: number) => {
    e.preventDefault();
    setOver(null);
    const moving = e.dataTransfer.getData("text/locdev-move");
    const adding = e.dataTransfer.getData("text/locdev-block");
    if (moving) move(moving, index);
    else if (adding) insert(index, newBlock(adding));
  };
  const zone = (index: number) => ({
    onDragOver: (e: DragEvent) => {
      e.preventDefault();
      setOver({ index });
    },
    onDrop: (e: DragEvent) => drop(e, index),
  });

  return (
    <div className="flex flex-col gap-3">
      {visual && (
        <section aria-label={t("Blocks")}>
          <h4 className="mb-1 font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">
            {t("Drag a block in")}
          </h4>
          <div className="flex flex-wrap gap-1">
            {palette(doc.format).map((b) => {
              const { label, icon: Icon } = BLOCKS[b];
              return (
                <button
                  type="button"
                  key={b}
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData("text/locdev-block", b)}
                  onClick={() => insert(defaultIndex(), newBlock(b))}
                  className="flex items-center gap-1 rounded border px-1.5 py-0.5 text-[11.5px] hover:border-ember"
                  title={t("Add {toLowerCase} (or drag it into place)", { toLowerCase: label.toLowerCase() })}
                >
                  <Icon className="size-3" /> {label}
                </button>
              );
            })}
            {components.map((c) => (
              <button
                type="button"
                key={c.usage}
                draggable
                onDragStart={(e) => e.dataTransfer.setData("text/locdev-block", `component:${c.usage}`)}
                onClick={() => insert(defaultIndex(), newBlock(`component:${c.usage}`))}
                className="flex items-center gap-1 rounded border border-dashed px-1.5 py-0.5 font-mono text-[11px] hover:border-ember"
                title={c.description ?? c.path}
              >
                <Puzzle className="size-3" /> {c.usage}
              </button>
            ))}
          </div>
        </section>
      )}
      <section aria-label={t("Structure")} className="min-h-0 flex-1">
        <h4 className="mb-1 font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{t("Structure")}</h4>
        <ul className="flex flex-col" onDragLeave={() => setOver(null)}>
          {segments.map((s, i) => {
            const info = s.kind === "code" ? { label: t("Code"), icon: Code2 } : (BLOCKS[s.block] ?? BLOCKS.component);
            const Icon = info.icon;
            return (
              <li key={s.id} {...zone(i)}>
                {over?.index === i && <div className="h-0.5 rounded bg-ember" />}
                <div
                  draggable
                  onDragStart={(e) => e.dataTransfer.setData("text/locdev-move", s.id)}
                  onClick={() => onSelect(s.id)}
                  className={cn(
                    "group flex cursor-pointer items-center gap-1.5 rounded-md px-1.5 py-1 text-[12px] hover:bg-foreground/[0.04]",
                    selected === s.id && "bg-foreground/[0.07]",
                    s.kind === "code" && "text-muted-foreground",
                  )}
                >
                  <GripVertical className="size-3 shrink-0 opacity-40" />
                  <Icon className="size-3.5 shrink-0" />
                  <span className="shrink-0">{info.label}</span>
                  {s.kind === "block" && s.edited && (
                    <TriangleAlert className="size-3 text-amber" aria-label={t("edited outside LocDev")} />
                  )}
                  <span className="min-w-0 truncate font-mono text-[11px] text-muted-foreground">
                    {summary(s, catalog.components)}
                  </span>
                  {selected === s.id && (
                    <span className="ml-auto flex shrink-0">
                      <button
                        type="button"
                        aria-label={t("Move up")}
                        disabled={i === 0}
                        onClick={() => move(s.id, i - 1)}
                        className="p-0.5 disabled:opacity-30"
                      >
                        <ArrowUp className="size-3" />
                      </button>
                      <button
                        type="button"
                        aria-label={t("Move down")}
                        disabled={i === segments.length - 1}
                        onClick={() => move(s.id, i + 2)}
                        className="p-0.5 disabled:opacity-30"
                      >
                        <ArrowDown className="size-3" />
                      </button>
                      {s.kind === "block" && (
                        <button
                          type="button"
                          aria-label={t("Duplicate")}
                          onClick={() => insert(i + 1, { ...structuredClone(s), id: newId(), edited: false })}
                          className="p-0.5"
                        >
                          <Copy className="size-3" />
                        </button>
                      )}
                      {s.kind === "block" && (
                        <button
                          type="button"
                          aria-label={t("Delete block")}
                          onClick={() => {
                            onChange(segments.filter((x) => x.id !== s.id));
                            onSelect(null);
                          }}
                          className="p-0.5"
                        >
                          <Trash2 className="size-3" />
                        </button>
                      )}
                    </span>
                  )}
                </div>
              </li>
            );
          })}
          <li {...zone(segments.length)} className="h-6">
            {over?.index === segments.length && <div className="h-0.5 rounded bg-ember" />}
          </li>
        </ul>
        <button
          type="button"
          onClick={() => onSelect(null)}
          className="mt-1 flex items-center gap-1 text-[11.5px] text-muted-foreground hover:text-foreground"
        >
          <Eye className="size-3" /> {t("Theme and sample data")}
        </button>
      </section>
    </div>
  );
}

function Field({ label, children, hint }: { label: string; children: ReactNode; hint?: string }) {
  return (
    <label className="flex flex-col gap-0.5 text-[12px]">
      <span className="text-muted-foreground">{label}</span>
      {children}
      {hint && <span className="text-[10.5px] text-muted-foreground">{hint}</span>}
    </label>
  );
}

function Inspector({
  doc,
  catalog,
  segment,
  theme,
  sample,
  sampleValid,
  onSample,
  onSampleSave,
  onTheme,
  onSegment,
}: {
  doc: EmailDoc;
  catalog: EmailCatalog;
  segment: Segment | null;
  theme: Props;
  sample: string;
  sampleValid: boolean;
  onSample: (s: string) => void;
  onSampleSave: () => void;
  onTheme: (t: Props) => void;
  onSegment: (s: Segment) => void;
}) {
  const vars = doc.variables.map((v) => `{{ $${v} }}`).join("  ");
  const hint =
    doc.format === "html"
      ? undefined
      : vars
        ? t("Blade works here: {vars}", { vars })
        : t("Blade expressions like {{ $name }} work here.");

  if (!segment) {
    return (
      <aside aria-label={t("Inspector")} className="flex flex-col gap-2.5 border-t pt-3">
        <h4 className="font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{t("Theme")}</h4>
        {(["width", "background", "card", "text", "muted", "accent", "font"] as const).map((key) => (
          <Field key={key} label={key}>
            <Input
              aria-label={t("Theme {key}", { key: key })}
              value={String(theme[key] ?? "")}
              onChange={(e) =>
                onTheme({ ...theme, [key]: key === "width" ? Number(e.target.value) || 600 : e.target.value })
              }
              className="h-7 font-mono text-[12px]"
            />
          </Field>
        ))}
        <p className="text-[10.5px] text-muted-foreground">
          {t("New and regenerated blocks use it; the layout around them is code.")}
        </p>
        <h4 className="mt-2 font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">
          {t("Sample data")}
        </h4>
        <CodeEditor
          ariaLabel={t("Sample data")}
          value={sample}
          onChange={onSample}
          language="json"
          lineNumbers={false}
          className={cn("h-40", !sampleValid && "border-destructive")}
        />
        <Button size="xs" variant="outline" disabled={!sampleValid} onClick={onSampleSave}>
          {t("Keep for this template")}
        </Button>
        <p className="text-[10.5px] text-muted-foreground">
          {t("Kept by LocDev, not in the project. Objects work as")} {t("{{ $user->name }}")}.
        </p>
      </aside>
    );
  }

  if (segment.kind === "code") {
    return (
      <aside aria-label={t("Inspector")} className="flex flex-col gap-2 border-t pt-3">
        <h4 className="font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{t("Code")}</h4>
        <p className="text-[11.5px] text-muted-foreground">
          {t("Kept exactly as written: Blade logic, bindings and layout aren't touched by the visual editor.")}
        </p>
        <CodeEditor
          ariaLabel="Code"
          value={segment.code}
          onChange={(code) => onSegment({ ...segment, code })}
          language={languageForPath(doc.path)}
          wrap
          className="h-64"
        />
      </aside>
    );
  }

  const b = segment as BlockSegment;
  const p = b.props as Record<string, JsonValue>;
  const set = (key: string, value: JsonValue) => onSegment({ ...b, props: { ...p, [key]: value } });
  const text = (key: string, label: string, multiline = false) => (
    <Field label={label} hint={key === "text" || key === "title" || key === "url" ? hint : undefined}>
      {multiline ? (
        <textarea
          aria-label={label}
          value={String(p[key] ?? "")}
          onChange={(e) => set(key, e.target.value)}
          className="min-h-24 rounded-md border bg-well p-2 text-[12px]"
        />
      ) : (
        <Input
          aria-label={label}
          value={String(p[key] ?? "")}
          onChange={(e) => set(key, e.target.value)}
          className="h-7 font-mono text-[12px]"
        />
      )}
    </Field>
  );
  const choice = (key: string, label: string, options: string[]) => (
    <Field label={label}>
      <Select
        items={options.map((o) => ({ value: o, label: o }))}
        value={String(p[key] ?? options[0])}
        onValueChange={(v) => v && set(key, String(v))}
      >
        <SelectTrigger size="sm" aria-label={label} className="h-7 text-[12px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((o) => (
            <SelectItem key={o} value={o}>
              {o}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Field>
  );
  const md = doc.format === "markdown";
  const component = b.block === "component" ? catalog.components.find((c) => c.usage === p.tag) : undefined;

  return (
    <aside aria-label={t("Inspector")} className="flex flex-col gap-2.5 border-t pt-3">
      <h4 className="font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">
        {BLOCKS[b.block]?.label ?? b.block}
      </h4>
      {b.edited && (
        <div className="rounded-md border border-amber/40 bg-amber/5 p-2 text-[11.5px]">
          <p>{t("This block's markup was changed outside LocDev, so it's kept as written.")}</p>
          <Button size="xs" variant="outline" className="mt-1.5" onClick={() => onSegment({ ...b, edited: false })}>
            {t("Regenerate from properties")}
          </Button>
        </div>
      )}
      {b.block === "heading" && (
        <>
          {text("text", "Text")}
          {choice("level", "Level", ["1", "2", "3"])}
          {!md && choice("align", "Align", ["left", "center", "right"])}
          {!md && text("color", "Color")}
        </>
      )}
      {(b.block === "text" || b.block === "panel") && (
        <>
          {text("text", "Text", true)}
          {!md && b.block === "text" && choice("align", "Align", ["left", "center", "right"])}
          {!md && b.block === "text" && text("size", t("Font size (px)"))}
          {!md && b.block === "text" && text("color", "Color")}
        </>
      )}
      {b.block === "button" && (
        <>
          {text("text", "Label")}
          {text("url", "Link")}
          {md ? (
            choice("style", "Style", ["primary", "success", "error"])
          ) : (
            <>
              {choice("align", "Align", ["left", "center", "right"])}
              {text("background", "Background")}
              {text("color", t("Text color"))}
              {text("radius", t("Corner radius (px)"))}
            </>
          )}
        </>
      )}
      {b.block === "image" && (
        <>
          {text("src", t("Image URL"))}
          {text("alt", t("Alternative text"))}
          {!md && text("width", t("Width (px)"))}
          {!md && text("url", t("Link (optional)"))}
          {!md && choice("align", "Align", ["left", "center", "right"])}
        </>
      )}
      {b.block === "header" && (
        <>
          {text("logo", t("Logo URL"))}
          {text("title", "Title")}
          {choice("align", "Align", ["center", "left", "right"])}
        </>
      )}
      {b.block === "footer" && (
        <>
          {text("text", "Text", true)}
          {!md && choice("align", "Align", ["center", "left", "right"])}
        </>
      )}
      {b.block === "divider" && text("color", "Color")}
      {b.block === "spacer" && text("height", t("Height (px)"))}
      {b.block === "columns" && (
        <>
          {((p.cells as string[]) ?? []).map((cell, i) => (
            <Field key={i} label={t("Column {n}", { n: i + 1 })}>
              <textarea
                aria-label={t("Column {n}", { n: i + 1 })}
                value={cell}
                onChange={(e) =>
                  set(
                    "cells",
                    ((p.cells as string[]) ?? []).map((c, j) => (j === i ? e.target.value : c)),
                  )
                }
                className="min-h-16 rounded-md border bg-well p-2 text-[12px]"
              />
            </Field>
          ))}
          <span className="flex gap-1">
            <Button
              size="xs"
              variant="outline"
              disabled={((p.cells as string[]) ?? []).length >= 3}
              onClick={() => set("cells", [...((p.cells as string[]) ?? []), ""])}
            >
              {t("Add column")}
            </Button>
            <Button
              size="xs"
              variant="ghost"
              disabled={((p.cells as string[]) ?? []).length <= 2}
              onClick={() => set("cells", ((p.cells as string[]) ?? []).slice(0, -1))}
            >
              {t("Remove")}
            </Button>
          </span>
        </>
      )}
      {b.block === "component" && (
        <>
          <p className="font-mono text-[11.5px]">&lt;{String(p.tag)}&gt;</p>
          {component?.description && <p className="text-[11px] text-muted-foreground">{component.description}</p>}
          {(component?.props ?? Object.keys((p.props as Props) ?? {}).map((name) => ({ name, default: null }))).map(
            (prop) => (
              <Field
                key={prop.name}
                label={prop.name}
                hint={prop.default ? t("default {value}", { value: prop.default }) : undefined}
              >
                <Input
                  aria-label={prop.name}
                  value={String(((p.props as Props) ?? {})[prop.name] ?? "")}
                  onChange={(e) => set("props", { ...((p.props as Props) ?? {}), [prop.name]: e.target.value })}
                  placeholder={t("$variable or text")}
                  className="h-7 font-mono text-[12px]"
                />
              </Field>
            ),
          )}
          {(component?.slot ?? true) && (
            <Field label={t("Content (slot)")} hint={hint}>
              <textarea
                aria-label={t("Content (slot)")}
                value={String(p.slot ?? "")}
                onChange={(e) => set("slot", e.target.value)}
                className="min-h-16 rounded-md border bg-well p-2 text-[12px]"
              />
            </Field>
          )}
          <p className="text-[10.5px] text-muted-foreground">
            {t('A value starting with $ binds a variable (:prop="$value").')}
          </p>
        </>
      )}
    </aside>
  );
}
