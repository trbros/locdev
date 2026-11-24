// Runtimes → PHP → settings: PHP's own values, LocDev's file on top, and
// what's in use. The daemon validates, backs up, checks what PHP loads and
// restarts only the running sites on that line; a system php.ini is only
// ever shown.

import { useEffect, useMemo, useState } from "react";
import { ExternalLink, FileCode, History, RotateCcw, SlidersHorizontal, TriangleAlert } from "lucide-react";

import { CodeEditor, CodeView, languageForPath } from "@/components/code-editor";
import { Section } from "@/components/page";
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
import { useRuntimes } from "@/hooks/use-daemon";
import { usePhpApply, usePhpConfig, usePhpFile, usePhpLines, usePhpPreview } from "@/hooks/use-php";
import { errorMessage } from "@/hooks/use-daemon";
import { tildify, uptime } from "@/lib/format";
import { t, tn } from "@/lib/i18n";
import type { PhpConfigView, PhpDirective, PhpIniChange, PhpIniPreview } from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";

/** The daemon's labels and descriptions (English), as the UI shows them. */
const TEXT: Record<string, [string, string]> = {
  memory_limit: [t("Memory limit"), t("The most memory a script may use; -1 for no limit.")],
  upload_max_filesize: [t("Largest upload"), t("The largest file a form can upload.")],
  post_max_size: [t("Largest POST"), t("The largest request body; must be at least the largest upload.")],
  max_file_uploads: [t("Files per request"), t("How many files one request may upload.")],
  max_execution_time: [
    t("Execution time limit"),
    t("Seconds a request may run; 0 for no limit. Commands (artisan, composer) always run without one."),
  ],
  max_input_time: [t("Input time limit"), t("Seconds to read a request's input; -1 uses the execution limit.")],
  max_input_vars: [t("Input variables"), t("How many GET, POST and cookie variables a request may have.")],
  display_errors: [t("Show errors"), t("Print errors in the page.")],
  display_startup_errors: [t("Show startup errors"), t("Print errors that happen while PHP starts.")],
  log_errors: [t("Log errors"), t("Write errors to the log (the site's output in LocDev).")],
  error_reporting: [
    t("Error level"),
    t("Which errors are reported, as constants (E_ALL & ~E_DEPRECATED) or a number."),
  ],
  "date.timezone": [t("Timezone"), t("The default timezone of date functions (America/Sao_Paulo, UTC).")],
  default_socket_timeout: [t("Socket timeout"), t("Seconds to wait on socket-based streams (HTTP requests from PHP).")],
  realpath_cache_size: [t("Realpath cache"), t("Cache of resolved file paths; bigger helps large projects.")],
  "opcache.enable_cli": [
    t("OPcache for the built-in server"),
    t("Cache compiled code in PHP's built-in server and commands (needs OPcache)."),
  ],
};

/** `null` drops LocDev's value; a string sets it. */
type Draft = Record<string, string | null>;

export function PhpSettings() {
  const lines = usePhpLines();
  const runtimes = useRuntimes();
  const fallback = runtimes.data
    ?.find((r) => r.kind === "php")
    ?.default?.split(".")
    .slice(0, 2)
    .join(".");
  const [picked, setPicked] = useState<string | null>(null);
  const available = lines.data ?? [];
  const line =
    picked && available.some((l) => l.line === picked)
      ? picked
      : (available.find((l) => l.line === fallback) ?? available[0])?.line;

  return (
    <Section
      title={t("settings")}
      aside={
        available.length > 1 && (
          <div role="radiogroup" aria-label={t("PHP version")} className="flex gap-1">
            {available.map((l) => (
              <button
                key={l.line}
                type="button"
                role="radio"
                aria-checked={l.line === line}
                onClick={() => setPicked(l.line)}
                className={cn(
                  "flex items-center gap-1.5 rounded-md border px-2 py-0.5 font-mono text-[12px] transition-colors hover:bg-foreground/4",
                  l.line === line && "border-ember/60 bg-ember/6",
                )}
              >
                {l.configured && <span className="size-1.5 rounded-full bg-ember" />}
                PHP {l.line}
              </button>
            ))}
          </div>
        )
      }
    >
      <p className="pt-1 pb-3 text-[13px] text-muted-foreground">
        {t(
          "LocDev's sites run on PHP's built-in server, which reads the same files as the php command. LocDev adds its own file on top, only for the sites it runs; your system's php.ini is only read. A change is checked against what PHP actually loads, and only the running sites on that version restart.",
        )}
      </p>
      {lines.isPending ? (
        <Skeleton className="h-64" />
      ) : !line ? (
        <p className="text-[13px] text-muted-foreground">{t("No PHP installed yet.")}</p>
      ) : (
        <ConfigBody key={line} line={line} />
      )}
    </Section>
  );
}

function ConfigBody({ line }: { line: string }) {
  const config = usePhpConfig(line);
  const preview = usePhpPreview();
  const [draft, setDraft] = useState<Draft>({});
  const [review, setReview] = useState<{ change: PhpIniChange; preview: PhpIniPreview } | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [history, setHistory] = useState(false);
  const [viewing, setViewing] = useState<string | null>(null);
  const changed = Object.keys(draft).length;

  if (config.isPending) return <Skeleton className="h-64" />;
  if (config.error) return <p className="text-[13px] text-destructive">{errorMessage(config.error)}</p>;
  const c = config.data!;

  const ask = (change: PhpIniChange) =>
    preview.mutate({ line, change }, { onSuccess: (p) => setReview({ change, preview: p }) });

  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-1 rounded-lg border bg-well/40 px-3.5 py-2.5 font-mono text-[12px]">
        <FileRow label={t("PHP")} value={`${c.version} · ${tildify(c.executable)}`} />
        <FileRow
          label="php.ini"
          value={c.loaded_ini ? tildify(c.loaded_ini) : t("none (PHP's built-in defaults)")}
          onView={c.loaded_ini ? () => setViewing(c.loaded_ini!) : undefined}
        />
        {c.scanned.length > 0 && (
          <FileRow
            label={t("scan folder")}
            value={tn("{n} file", "{n} files", c.scanned.length)}
            onView={() => setViewing(c.scanned[0])}
          />
        )}
        <FileRow label="LocDev" value={tildify(c.file)} onView={() => setAdvanced(true)} />
      </div>

      <div className="overflow-x-auto rounded-lg border">
        <table className="w-full text-[13px]">
          <thead>
            <tr className="border-b font-mono text-[11px] text-muted-foreground">
              <th className="px-3 py-2 text-left font-normal">{t("Setting")}</th>
              <th className="px-3 py-2 text-left font-normal">{t("LocDev")}</th>
              <th className="px-3 py-2 text-left font-normal">{t("PHP's own")}</th>
              <th className="px-3 py-2 text-left font-normal">{t("In use")}</th>
            </tr>
          </thead>
          <tbody>
            {c.directives.map((d) => (
              <DirectiveRow
                key={d.key}
                directive={d}
                draft={draft}
                onChange={(value) =>
                  setDraft((current) => {
                    const next = { ...current };
                    const now = d.value ?? null;
                    if (value === now) delete next[d.key];
                    else next[d.key] = value;
                    return next;
                  })
                }
              />
            ))}
          </tbody>
        </table>
      </div>

      {c.warnings.map((w) => (
        <p key={w} className="flex gap-2 text-[12.5px] text-amber">
          <TriangleAlert className="mt-0.5 size-3.5 shrink-0" /> {w}
        </p>
      ))}

      <div className="flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          disabled={changed === 0 || preview.isPending}
          onClick={() => ask({ kind: "set", values: draft })}
          className="bg-ember text-ember-foreground hover:bg-ember/85"
        >
          {tn("Review {n} change", "Review {n} changes", changed)}
        </Button>
        {changed > 0 && (
          <Button size="sm" variant="ghost" onClick={() => setDraft({})}>
            {t("Discard")}
          </Button>
        )}
        <span className="ml-auto flex flex-wrap gap-1">
          <Button size="sm" variant="ghost" onClick={() => setAdvanced(true)}>
            <SlidersHorizontal data-icon="inline-start" /> {t("Advanced configuration")}
          </Button>
          <Button size="sm" variant="ghost" disabled={c.history.length === 0} onClick={() => setHistory(true)}>
            <History data-icon="inline-start" /> {t("History")}
          </Button>
          <Button size="sm" variant="ghost" disabled={!c.text.trim()} onClick={() => ask({ kind: "reset" })}>
            <RotateCcw data-icon="inline-start" /> {t("Use PHP's values")}
          </Button>
        </span>
      </div>
      {preview.error && <p className="text-[12.5px] text-destructive">{errorMessage(preview.error)}</p>}
      {c.sites.length > 0 && (
        <p className="text-[12px] text-muted-foreground">
          {t("Running on PHP {line}: {sites}", { line, sites: c.sites.join(", ") })}
        </p>
      )}

      <ReviewDialog
        line={line}
        review={review}
        onClose={() => setReview(null)}
        onApplied={() => {
          setDraft({});
          setReview(null);
          setAdvanced(false);
          setHistory(false);
        }}
      />
      <AdvancedDialog
        open={advanced}
        config={c}
        busy={preview.isPending}
        error={preview.error ? errorMessage(preview.error) : null}
        onClose={() => setAdvanced(false)}
        onSave={(text) => ask({ kind: "text", text, hash: c.hash })}
        onView={setViewing}
      />
      <HistoryDialog
        open={history}
        config={c}
        onClose={() => setHistory(false)}
        onRestore={(id) => ask({ kind: "restore", id })}
      />
      <FileDialog line={line} path={viewing} onClose={() => setViewing(null)} />
    </div>
  );
}

function FileRow({ label, value, onView }: { label: string; value: string; onView?: () => void }) {
  return (
    <div className="flex items-baseline gap-3">
      <span className="w-32 shrink-0 text-muted-foreground">{label}</span>
      <span className="min-w-0 flex-1 truncate" title={value} data-selectable>
        {value}
      </span>
      {onView && (
        <button
          type="button"
          onClick={onView}
          className="inline-flex min-h-6 shrink-0 items-center px-1 text-ember hover:underline"
        >
          {t("view")}
        </button>
      )}
    </div>
  );
}

function DirectiveRow({
  directive: d,
  draft,
  onChange,
}: {
  directive: PhpDirective;
  draft: Draft;
  onChange: (value: string | null) => void;
}) {
  const shown = d.key in draft ? draft[d.key] : (d.value ?? null);
  const dirty = d.key in draft;
  const overridden = d.effective != null && d.original != null && d.effective !== d.original;

  return (
    <tr className={cn("border-b last:border-0", dirty && "bg-ember/5")}>
      <td className="px-3 py-1.5 align-middle">
        <div className="flex items-center gap-1.5">
          <span title={TEXT[d.key]?.[1] ?? d.description}>{TEXT[d.key]?.[0] ?? d.label}</span>
          <button
            type="button"
            onClick={() => openInBrowser(d.docs)}
            aria-label={t("Documentation for {key}", { key: d.key })}
            className="text-muted-foreground hover:text-ember"
          >
            <ExternalLink className="size-3" />
          </button>
        </div>
        <div className="font-mono text-[11px] text-muted-foreground">{d.key}</div>
      </td>
      <td className="px-3 py-1.5">
        {d.kind === "bool" ? (
          <Select
            items={[
              { value: "", label: t("PHP's value") },
              { value: "On", label: "On" },
              { value: "Off", label: "Off" },
            ]}
            value={shown ?? ""}
            onValueChange={(v) => onChange(v ? String(v) : null)}
          >
            <SelectTrigger size="sm" aria-label={d.key} className="h-7 w-32 font-mono text-[12px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="">{t("PHP's value")}</SelectItem>
              <SelectItem value="On">On</SelectItem>
              <SelectItem value="Off">Off</SelectItem>
            </SelectContent>
          </Select>
        ) : (
          <Input
            value={shown ?? ""}
            onChange={(e) => onChange(e.target.value.trim() === "" ? null : e.target.value)}
            placeholder={d.original ?? ""}
            aria-label={d.key}
            className={cn("h-7 w-44 font-mono text-[12px]", d.kind === "error_level" && "w-56")}
          />
        )}
      </td>
      <td className="px-3 py-1.5 font-mono text-[12px] text-muted-foreground">{d.original ?? "—"}</td>
      <td className={cn("px-3 py-1.5 font-mono text-[12px]", overridden && "text-ember")}>{d.effective ?? "—"}</td>
    </tr>
  );
}

/** Removed lines (`-`) and added ones (`+`). */
function diffLines(before: string, after: string): { sign: "-" | "+"; text: string }[] {
  const old = before.split("\n").filter(Boolean);
  const neu = after.split("\n").filter(Boolean);
  return [
    ...old.filter((l) => !neu.includes(l)).map((text) => ({ sign: "-" as const, text })),
    ...neu.filter((l) => !old.includes(l)).map((text) => ({ sign: "+" as const, text })),
  ];
}

function ReviewDialog({
  line,
  review,
  onClose,
  onApplied,
}: {
  line: string;
  review: { change: PhpIniChange; preview: PhpIniPreview } | null;
  onClose: () => void;
  onApplied: () => void;
}) {
  const apply = usePhpApply();
  const diff = useMemo(() => (review ? diffLines(review.preview.before, review.preview.after) : []), [review]);

  return (
    <Dialog open={!!review} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t("Apply to PHP {line}?", { line })}</DialogTitle>
          <DialogDescription>
            {t("The current file is backed up first. If PHP doesn't load the new one, it's put back.")}
          </DialogDescription>
        </DialogHeader>
        {review && (
          <div className="flex flex-col gap-3">
            <div className="max-h-64 overflow-auto rounded-md border bg-well/50 px-3 py-2 font-mono text-[12px]">
              {diff.length === 0 ? (
                <span className="text-muted-foreground">{t("No change.")}</span>
              ) : (
                diff.map((l, i) => (
                  <div key={i} className={l.sign === "+" ? "text-phosphor" : "text-destructive"}>
                    {l.sign} {l.text}
                  </div>
                ))
              )}
            </div>
            {review.preview.warnings.map((w) => (
              <p key={w} className="flex gap-2 text-[12.5px] text-amber">
                <TriangleAlert className="mt-0.5 size-3.5 shrink-0" /> {w}
              </p>
            ))}
            <p className="flex items-center gap-2 text-[12.5px] text-muted-foreground">
              <Led tone={review.preview.restart.length ? "warn" : "off"} />
              {review.preview.restart.length
                ? t("Restarts: {sites}", { sites: review.preview.restart.join(", ") })
                : t("No site on this line is running.")}
            </p>
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button
            disabled={apply.isPending || diff.length === 0}
            onClick={() => review && apply.mutate({ line, change: review.change }, { onSuccess: onApplied })}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            {t("Apply")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function AdvancedDialog({
  open,
  config,
  busy,
  error,
  onClose,
  onSave,
  onView,
}: {
  open: boolean;
  config: PhpConfigView;
  busy: boolean;
  error: string | null;
  onClose: () => void;
  onSave: (text: string) => void;
  onView: (path: string) => void;
}) {
  const [text, setText] = useState(config.text);
  useEffect(() => {
    if (open) setText(config.text || `; LocDev's PHP ${config.line} settings. Lines you add are kept.\n`);
  }, [open, config.text, config.line]);
  const dirty = text !== config.text;

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileCode className="size-4 text-ember" /> {t("Advanced configuration · PHP {line}", { line: config.line })}
          </DialogTitle>
          <DialogDescription>
            {t(
              "LocDev's file for this version. Anything PHP accepts can go here (xdebug.mode, opcache…); PHP checks it before it's applied. Your system's files are read-only:",
            )}{" "}
            {[config.loaded_ini, ...config.scanned].filter(Boolean).length > 0 && (
              <button
                type="button"
                className="text-ember hover:underline"
                onClick={() => onView((config.loaded_ini ?? config.scanned[0])!)}
              >
                {t("view them")}
              </button>
            )}
          </DialogDescription>
        </DialogHeader>
        <CodeEditor
          value={text}
          onChange={setText}
          language={languageForPath("x.ini")}
          lineNumbers
          ariaLabel={t("LocDev's PHP file")}
          className="h-72"
          onSave={() => dirty && onSave(text)}
        />
        {error && <p className="text-[12.5px] text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("Close")}
          </Button>
          <Button
            disabled={!dirty || busy}
            onClick={() => onSave(text)}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            {t("Review and apply")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function HistoryDialog({
  open,
  config,
  onClose,
  onRestore,
}: {
  open: boolean;
  config: PhpConfigView;
  onClose: () => void;
  onRestore: (id: string) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("Earlier versions · PHP {line}", { line: config.line })}</DialogTitle>
          <DialogDescription>{t("Every change keeps the file it replaced.")}</DialogDescription>
        </DialogHeader>
        <ul className="flex max-h-72 flex-col overflow-y-auto">
          {config.history.map((h) => (
            <li key={h.id} className="flex items-center gap-3 border-b py-2 text-[13px] last:border-0">
              <span className="font-mono text-[12px]">{new Date(h.at).toLocaleString()}</span>
              <span className="text-[12px] text-muted-foreground">
                {t("{time} ago", { time: uptime(h.at) })} · {tn("{n} line", "{n} lines", h.lines)}
              </span>
              <Button size="xs" variant="outline" className="ml-auto" onClick={() => onRestore(h.id)}>
                {t("Restore…")}
              </Button>
            </li>
          ))}
        </ul>
      </DialogContent>
    </Dialog>
  );
}

function FileDialog({ line, path, onClose }: { line: string; path: string | null; onClose: () => void }) {
  const file = usePhpFile(line, path);
  return (
    <Dialog open={!!path} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle className="truncate font-mono text-[14px]">{path ? tildify(path) : ""}</DialogTitle>
          <DialogDescription>
            {t("Read-only: LocDev never changes a file it didn't write. Override a value in LocDev's file instead.")}
          </DialogDescription>
        </DialogHeader>
        {file.isPending ? (
          <Skeleton className="h-72" />
        ) : file.error ? (
          <p className="text-[12.5px] text-destructive">{errorMessage(file.error)}</p>
        ) : (
          <CodeView value={file.data ?? ""} language={languageForPath("x.ini")} lineNumbers className="h-96" />
        )}
      </DialogContent>
    </Dialog>
  );
}
