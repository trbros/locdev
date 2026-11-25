import { ExternalLink, EyeOff, Plus, RefreshCw, RotateCcw, Search, SlidersHorizontal } from "lucide-react";
import { useMemo, useState } from "react";

import { Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
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
import { Skeleton } from "@/components/ui/skeleton";
import { SwitchField } from "@/components/ui/switch-field";
import { errorMessage, useSettings } from "@/hooks/use-daemon";
import { useDefinitions, useDefinitionsActions, useStackReport, useStackSet } from "@/hooks/use-stack";
import { uptime } from "@/lib/format";
import {
  type Component,
  type ComponentCategory,
  type DefinitionInfo,
  INTEGRATION_LABEL,
  type Integration,
  type ProjectView,
} from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";
import { StackIcon } from "./stack-icons/stack-icon";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const INTEGRATION_TONE: Record<Integration, Tone> = { native: "live", script: "info", custom: "warn", none: "off" };

const CATEGORY_LABEL: Record<ComponentCategory, string> = {
  framework: t("framework"),
  ui: t("ui"),
  language: t("language"),
  runtime: t("runtime"),
  tool: t("tool"),
};

function ComponentChip({ component, onHide }: { component: Component; onHide?: () => void }) {
  return (
    <span
      className={cn(
        "group inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[12.5px]",
        component.primary && "border-ember/60 bg-ember/6",
      )}
      title={`${CATEGORY_LABEL[component.category]} · ${component.evidence.join(", ")}`}
    >
      <span>{component.name}</span>
      {component.version && (
        <span className="font-mono text-[11px] text-muted-foreground" title={component.version_source ?? undefined}>
          {component.version}
        </span>
      )}
      {component.manual && <span className="font-mono text-[10.5px] text-amber">{t("added")}</span>}
      {onHide && !component.primary && (
        <button
          type="button"
          onClick={onHide}
          aria-label={t("Hide {name}", { name: component.name })}
          className="text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-foreground focus-visible:opacity-100"
        >
          <EyeOff className="size-3" />
        </button>
      )}
    </span>
  );
}

/** The site's stack on its Overview: components, how it runs, what it needs. */
export function StackSection({ site }: { site: ProjectView }) {
  const report = useStackReport(site.name);
  const set = useStackSet();
  const [editing, setEditing] = useState(false);
  const stack = site.stack;

  return (
    <Section
      title={t("stack")}
      aside={
        <Button size="xs" variant="ghost" onClick={() => setEditing(true)}>
          <SlidersHorizontal data-icon="inline-start" /> {t("Edit…")}
        </Button>
      }
    >
      <div className="flex flex-wrap items-center gap-1.5">
        {stack.components.length === 0 ? (
          <span className="text-sm text-muted-foreground">{t("Nothing recognized in this folder.")}</span>
        ) : (
          stack.components.map((component) => (
            <ComponentChip
              key={component.id}
              component={component}
              onHide={() => set.mutate({ name: site.name, remove: [component.id] })}
            />
          ))
        )}
      </div>
      <div className="mt-2 flex items-center gap-2 text-[12.5px] text-muted-foreground">
        <Led tone={INTEGRATION_TONE[stack.integration]} />
        <span>{INTEGRATION_LABEL[stack.integration]}</span>
        {site.adapter && <span className="font-mono text-[11px]">{t("· adapter chosen by hand")}</span>}
        {stack.venv && <span className="font-mono text-[11px]">{t("· venv {venv}", { venv: stack.venv })}</span>}
        {stack.port_env.map((v) => (
          <span key={v} className="font-mono text-[11px]">
            · {v.includes("=") ? v : `${v}=<port>`}
          </span>
        ))}
      </div>
      {report.data?.prerequisites.some((p) => !p.found) && (
        <p className="mt-2 text-[12.5px] text-amber">
          {t("Not found on PATH:")}{" "}
          {report.data.prerequisites
            .filter((p) => !p.found)
            .map((p) => p.program)
            .join(", ")}
        </p>
      )}
      {stack.notes.length > 0 && (
        <ul className="mt-2 flex flex-col gap-1 text-[12.5px] text-muted-foreground">
          {stack.notes.map((note) => (
            <li key={note}>· {note}</li>
          ))}
        </ul>
      )}
      {stack.workspaces.length > 0 && (
        <div className="mt-3">
          <p className="text-[12.5px] text-muted-foreground">{t("A monorepo: add each package as its own site.")}</p>
          <ul className="mt-1 font-mono text-[12px]">
            {stack.workspaces.map((m) => (
              <li key={m.path} className="flex gap-3">
                <span className="w-48 truncate">{m.path}</span>
                <span className="text-muted-foreground">{m.framework}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
      {report.data && (report.data.install || report.data.build || report.data.test) && (
        <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-0.5 font-mono text-[12px]">
          {(["install", "build", "test"] as const).map(
            (k) =>
              report.data![k] && (
                <div key={k} className="contents">
                  <dt className="text-muted-foreground">{k}</dt>
                  <dd data-selectable>{report.data![k]}</dd>
                </div>
              ),
          )}
        </dl>
      )}
      <StackDialog site={site} open={editing} onClose={() => setEditing(false)} />
    </Section>
  );
}

function StackDialog({ site, open, onClose }: { site: ProjectView; open: boolean; onClose: () => void }) {
  const definitions = useDefinitions();
  const set = useStackSet();
  const [query, setQuery] = useState("");
  const [applyCommand, setApplyCommand] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const present = new Set(site.stack.components.map((c) => c.id));
  const hidden = site.hidden_stacks ?? [];
  const needle = query.trim().toLowerCase();
  const list = (definitions.data?.definitions ?? []).filter(
    (d) => !needle || d.name.toLowerCase().includes(needle) || d.id.includes(needle),
  );

  const choose = async (adapter: string) => {
    setError(null);
    try {
      await set.mutateAsync({ name: site.name, adapter, apply_command: applyCommand && adapter !== "" });
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("Stack of {name}", { name: site.name })}</DialogTitle>
          <DialogDescription>
            {t(
              "Add what LocDev couldn't prove from the files, hide what's wrong, or choose the framework that runs the site.",
            )}
          </DialogDescription>
        </DialogHeader>
        <div className="flex items-center gap-3 rounded-lg border px-3 py-2 text-sm">
          <span>
            {t("Runs as")} <span className="font-medium">{site.stack.framework || t("nothing recognized")}</span>
            {site.adapter ? t(" (chosen by hand)") : t(" (detected)")}
          </span>
          {site.adapter && (
            <Button size="xs" variant="ghost" className="ml-auto" onClick={() => choose("")}>
              {t("Back to detection")}
            </Button>
          )}
        </div>
        <SwitchField
          className="flex items-center gap-3 text-sm"
          checked={applyCommand}
          onCheckedChange={setApplyCommand}
          switchClassName="data-checked:bg-ember"
        >
          {t("When choosing a framework, also use its start command for this site")}
        </SwitchField>
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("Search frameworks, languages and tools")}
            className="h-8 pl-8 text-[13px]"
            aria-label={t("Search definitions")}
          />
        </div>
        <div className="max-h-[45vh] divide-y overflow-y-auto rounded-lg border">
          {definitions.isPending ? (
            <Skeleton className="h-32" />
          ) : (
            list.map((d) => {
              const isPrimary = site.stack.adapter === d.id;
              return (
                <div key={d.id} className="flex items-center gap-3 px-3 py-1.5 text-[13px]">
                  <span className="w-44 truncate">{d.name}</span>
                  <span className="w-20 font-mono text-[11px] text-muted-foreground">{CATEGORY_LABEL[d.category]}</span>
                  <span className="flex-1 truncate font-mono text-[11px] text-muted-foreground">{d.dev[0] ?? ""}</span>
                  {hidden.includes(d.id) ? (
                    <Button size="xs" variant="ghost" onClick={() => set.mutate({ name: site.name, add: [d.id] })}>
                      {t("Unhide")}
                    </Button>
                  ) : present.has(d.id) ? (
                    !isPrimary && (
                      <Button size="xs" variant="ghost" onClick={() => set.mutate({ name: site.name, remove: [d.id] })}>
                        {t("Hide")}
                      </Button>
                    )
                  ) : (
                    <Button size="xs" variant="ghost" onClick={() => set.mutate({ name: site.name, add: [d.id] })}>
                      <Plus data-icon="inline-start" /> {t("Add")}
                    </Button>
                  )}
                  {d.dev.length > 0 &&
                    (isPrimary ? (
                      <span className="w-20 text-right font-mono text-[11px] text-ember">{t("runs it")}</span>
                    ) : (
                      <Button size="xs" variant="outline" className="w-20" onClick={() => choose(d.id)}>
                        {t("Run as")}
                      </Button>
                    ))}
                  {d.dev.length === 0 && <span className="w-20" />}
                </div>
              );
            })
          )}
        </div>
        {error && <p className="text-[13px] text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {t("Done")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Settings → Frameworks ────────────────────────────────────────────────────

export function DefinitionsSettings() {
  const definitions = useDefinitions();
  const { refresh, rollback } = useDefinitionsActions();
  const settings = useSettings();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<DefinitionInfo | null>(null);
  const needle = query.trim().toLowerCase();
  const list = useMemo(
    () =>
      (definitions.data?.definitions ?? []).filter(
        (d) => !needle || d.name.toLowerCase().includes(needle) || d.id.includes(needle),
      ),
    [definitions.data, needle],
  );

  if (definitions.isPending) return <Skeleton className="h-64" />;
  if (definitions.isError || !definitions.data) {
    return <p className="text-sm text-destructive">{errorMessage(definitions.error)}</p>;
  }
  const r = definitions.data.registry;
  const auto = settings.query.data?.automation.definitions;

  return (
    <>
      <Section
        title={t("definitions")}
        aside={
          <div className="flex gap-1.5">
            {r.origin === "downloaded" && (
              <Button size="xs" variant="ghost" disabled={rollback.isPending} onClick={() => rollback.mutate()}>
                <RotateCcw data-icon="inline-start" /> {t("Roll back")}
              </Button>
            )}
            <Button size="xs" variant="outline" disabled={refresh.isPending} onClick={() => refresh.mutate(false)}>
              <RefreshCw data-icon="inline-start" className={cn(refresh.isPending && "animate-spin")} />{" "}
              {t("Check now")}
            </Button>
          </div>
        }
      >
        <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 font-mono text-[12px]">
          <dt className="text-muted-foreground">{t("in use")}</dt>
          <dd>
            {tx("revision {revision} · {origin} · {count} frameworks", {
              revision: r.revision,
              origin: r.origin === "bundled" ? t("bundled with LocDev") : t("downloaded, signature verified"),
              count: r.count,
            })}
          </dd>
          <dt className="text-muted-foreground">{t("bundled")}</dt>
          <dd>{t("revision {bundled_revision}", { bundled_revision: r.bundled_revision })}</dd>
          {r.previous && (
            <>
              <dt className="text-muted-foreground">{t("previous")}</dt>
              <dd>{t("revision {previous}", { previous: r.previous })}</dd>
            </>
          )}
          {r.skipped && (
            <>
              <dt className="text-muted-foreground">{t("skipped")}</dt>
              <dd>{t("revision {skipped} (rolled back)", { skipped: r.skipped })}</dd>
            </>
          )}
          <dt className="text-muted-foreground">{t("last check")}</dt>
          <dd>{r.checked_at ? t("{time} ago", { time: uptime(r.checked_at) }) : t("never")}</dd>
        </dl>
        {r.error && (
          <p className="mt-2 text-[12.5px] text-amber">{t("Last update failed: {error}", { error: r.error })}</p>
        )}
        <p className="mt-2 text-[12.5px] text-muted-foreground">
          {t(
            "Definitions are data: how each framework is recognized, how its dev server takes a host and port, its links. A downloaded set is used only when its minisign signature matches the key LocDev ships, its format is one this version understands, and it is newer than any set already seen. Nothing in them can run code LocDev doesn't already have.",
          )}
        </p>
        <SwitchField
          className="mt-3 flex items-center gap-3 text-sm"
          checked={!!auto}
          disabled={!settings.query.data}
          onCheckedChange={(definitions) =>
            settings.query.data &&
            settings.update.mutate({
              ...settings.query.data,
              automation: { ...settings.query.data.automation, definitions },
            })
          }
          switchClassName="data-checked:bg-ember"
        >
          {t("Check for new definitions daily (skipped in offline mode)")}
        </SwitchField>
      </Section>

      <Section title={t("frameworks · {length}", { length: definitions.data.definitions.length })}>
        <div className="relative mb-2">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("Search")}
            className="h-8 pl-8 text-[13px]"
            aria-label={t("Search frameworks")}
          />
        </div>
        <div className="divide-y rounded-lg border">
          {list.map((d) => (
            <button
              type="button"
              key={d.id}
              onClick={() => setOpen(d)}
              className="flex w-full items-center gap-3 px-3 py-1.5 text-left text-[13px] hover:bg-foreground/3"
            >
              <StackIcon id={d.id} className="size-4 shrink-0" aria-hidden="true" />
              <span className="w-44 truncate">{d.name}</span>
              <span className="w-20 font-mono text-[11px] text-muted-foreground">{CATEGORY_LABEL[d.category]}</span>
              <span className="flex w-36 items-center gap-1.5 text-[12px] text-muted-foreground">
                <Led tone={INTEGRATION_TONE[d.integration]} />{" "}
                {d.integration === "none" ? t("component") : INTEGRATION_LABEL[d.integration]}
              </span>
              <span className="flex-1 truncate font-mono text-[11px] text-muted-foreground">{d.dev[0] ?? ""}</span>
            </button>
          ))}
        </div>
      </Section>

      <Dialog open={open !== null} onOpenChange={(next) => !next && setOpen(null)}>
        <DialogContent className="sm:max-w-xl">
          {open && (
            <>
              <DialogHeader>
                <DialogTitle>{open.name}</DialogTitle>
                <DialogDescription>
                  {CATEGORY_LABEL[open.category]} · {INTEGRATION_LABEL[open.integration]}
                  {open.supported && t(" · written for {supported}", { supported: open.supported })}
                </DialogDescription>
              </DialogHeader>
              <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 font-mono text-[12px]">
                {open.dev.map((cmd, i) => (
                  <div key={cmd} className="contents">
                    <dt className="text-muted-foreground">{i === 0 ? t("dev") : ""}</dt>
                    <dd data-selectable>{cmd}</dd>
                  </div>
                ))}
                {(["install", "build", "test"] as const).map(
                  (k) =>
                    open[k] && (
                      <div key={k} className="contents">
                        <dt className="text-muted-foreground">{k}</dt>
                        <dd data-selectable>{open[k]}</dd>
                      </div>
                    ),
                )}
                {open.requires.length > 0 && (
                  <>
                    <dt className="text-muted-foreground">{t("requires")}</dt>
                    <dd>{open.requires.join(", ")}</dd>
                  </>
                )}
                {open.services.length > 0 && (
                  <>
                    <dt className="text-muted-foreground">{t("services")}</dt>
                    <dd>
                      {open.services.join(", ")}{" "}
                      <span className="font-sans text-muted-foreground">
                        {t("(commonly used; not managed by LocDev yet)")}
                      </span>
                    </dd>
                  </>
                )}
              </dl>
              {open.notes.length > 0 && (
                <ul className="text-[12.5px] text-muted-foreground">
                  {open.notes.map((n) => (
                    <li key={n}>· {n}</li>
                  ))}
                </ul>
              )}
              <DialogFooter>
                <Button variant="outline" onClick={() => openInBrowser(open.docs)}>
                  <ExternalLink data-icon="inline-start" /> {t("Official docs")}
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
