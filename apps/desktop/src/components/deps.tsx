import { ArrowUpCircle, ChevronRight, RefreshCw, Search, ShieldAlert, X } from "lucide-react";
import { Fragment, useMemo, useState } from "react";

import { Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  type UpdateChoice,
  useCheckDeps,
  useDeps,
  useJob,
  useJobs,
  usePlanUpdate,
  useUpdateDeps,
} from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import {
  type Compatibility,
  type DepStatus,
  jobFinished,
  type PackageReport,
  type ProjectView,
  RUNTIME_LABEL,
} from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

type Filter = "all" | "prod" | "dev" | "outdated" | "security";

const COMPAT: Record<Compatibility, { label: string; tone: Tone; title: string }> = {
  compatible: { label: t("no conflicts"), tone: "live", title: t("No check found a conflict. Not a guarantee.") },
  possibly: {
    label: "possibly",
    tone: "warn",
    title: t("Could break: a new major, or something couldn't be checked."),
  },
  incompatible: { label: "incompatible", tone: "fail", title: t("A requirement isn't met (engines, peers, PHP).") },
  unknown: { label: "unknown", tone: "off", title: t("Not enough information to judge.") },
};

const STATUS: Record<DepStatus, { label: string; tone: Tone }> = {
  up_to_date: { label: t("up to date"), tone: "live" },
  outdated: { label: "outdated", tone: "info" },
  vulnerable: { label: "vulnerable", tone: "fail" },
  unknown: { label: "unknown", tone: "off" },
};

export function DepsTab({ site }: { site: ProjectView }) {
  const deps = useDeps(site.name);
  const jobs = useJobs(site.name);
  const check = useCheckDeps();
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [open, setOpen] = useState<string | null>(null);
  const [updating, setUpdating] = useState(false);

  const running = jobs.data?.find((j) => j.kind === "deps_audit" && !jobFinished(j.status));
  const report = deps.data;
  const needle = query.trim().toLowerCase();
  const packages = useMemo(
    () =>
      (report?.packages ?? []).filter(
        (p) =>
          (filter === "all" ||
            (filter === "prod" && p.kind === "prod") ||
            (filter === "dev" && p.kind === "dev") ||
            (filter === "outdated" && p.status === "outdated") ||
            (filter === "security" && p.status === "vulnerable")) &&
          (!needle || p.name.toLowerCase().includes(needle)),
      ),
    [report, filter, needle],
  );
  const count = (f: Filter) =>
    (report?.packages ?? []).filter((p) =>
      f === "outdated" ? p.status === "outdated" : f === "security" ? p.status === "vulnerable" : true,
    ).length;
  const toggle = (name: string) =>
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });

  if (deps.isPending) {
    return (
      <div className="flex flex-col gap-2">
        {[0, 1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-9" />
        ))}
      </div>
    );
  }
  if (!report || report.files.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {tx("No {package_json} or {composer_json} in this site's folder, so there are no dependencies to show.", {
          package_json: <span className="font-mono">package.json</span>,
          composer_json: <span className="font-mono">composer.json</span>,
        })}
      </p>
    );
  }

  return (
    <>
      <div className="flex flex-wrap items-center gap-3 text-[12.5px]">
        <span className="font-mono text-muted-foreground">
          {report.files.join(" · ")}
          {report.manager && ` · ${RUNTIME_LABEL[report.manager]}`}
        </span>
        <span className="ml-auto font-mono text-[11.5px] text-muted-foreground">
          {running
            ? running.progress
              ? t("checking {done}/{total} · {label}", {
                  done: running.progress.done,
                  total: running.progress.total ?? "?",
                  label: running.progress.label,
                })
              : `check ${running.status}`
            : report.checked_at
              ? t("checked {checked_at} ago", { checked_at: uptime(report.checked_at) })
              : t("never checked against the registries")}
        </span>
        <Button
          size="sm"
          variant="outline"
          disabled={!!running || check.isPending}
          onClick={() => check.mutate(site.name)}
        >
          <RefreshCw data-icon="inline-start" className={cn(running && "animate-spin")} /> {t("Check now")}
        </Button>
      </div>

      {report.errors.map((error) => (
        <p key={error} className="rounded-md border border-amber/30 bg-amber/[0.07] px-3 py-2 text-[13px] text-amber">
          {error}
        </p>
      ))}

      <div className="flex flex-wrap items-center gap-2">
        <div role="radiogroup" aria-label={t("Filter")} className="flex rounded-md border p-0.5 text-[12.5px]">
          {(
            [
              ["all", t("All")],
              ["prod", t("Production")],
              ["dev", t("Development")],
              ["outdated", `Outdated ${count("outdated") || ""}`],
              ["security", `Security ${count("security") || ""}`],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={filter === id}
              onClick={() => setFilter(id)}
              className={cn(
                "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
                filter === id && "bg-foreground/[0.07] text-foreground",
              )}
            >
              {label}
            </button>
          ))}
        </div>
        <label className="ml-auto flex h-8 items-center gap-2 rounded-md border bg-background/60 px-2.5 text-sm focus-within:border-ember/50">
          <Search className="size-3.5 text-muted-foreground" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t("Filter packages")}
            aria-label={t("Filter packages")}
            className="w-40 bg-transparent outline-none placeholder:text-muted-foreground"
          />
          {query && (
            <button type="button" aria-label={t("Clear")} onClick={() => setQuery("")}>
              <X className="size-3.5 text-muted-foreground" />
            </button>
          )}
        </label>
        <Button
          size="sm"
          disabled={selected.size === 0}
          onClick={() => setUpdating(true)}
          className="bg-ember text-ember-foreground hover:bg-ember/85"
        >
          <ArrowUpCircle data-icon="inline-start" /> {t("Update")} {selected.size || ""}…
        </Button>
      </div>

      {packages.length === 0 ? (
        <p className="py-4 text-sm text-muted-foreground">{t("No package matches.")}</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-8" />
              <TableHead>{t("Package")}</TableHead>
              <TableHead>{t("Installed")}</TableHead>
              <TableHead>{t("Declared")}</TableHead>
              <TableHead>{t("Available")}</TableHead>
              <TableHead>{t("Compatibility")}</TableHead>
              <TableHead>{t("Status")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {packages.map((p) => (
              <Fragment key={`${p.ecosystem}:${p.name}`}>
                <PackageRow
                  pkg={p}
                  selected={selected.has(p.name)}
                  onSelect={() => toggle(p.name)}
                  open={open === p.name}
                  onOpen={() => setOpen(open === p.name ? null : p.name)}
                />
                {open === p.name && <PackageDetails pkg={p} />}
              </Fragment>
            ))}
          </TableBody>
        </Table>
      )}
      <p className="text-[12px] text-muted-foreground">
        {t(
          '"Installed" is what\'s in node_modules/vendor (else the lockfile\'s version). "Available" is the newest stable release; the line under it, when different, is the newest your constraint allows. Pre-releases are never proposed.',
        )}
      </p>

      <UpdateDialog
        site={site}
        packages={[...selected]}
        open={updating}
        onClose={(done) => {
          setUpdating(false);
          if (done) setSelected(new Set());
        }}
      />
    </>
  );
}

function PackageRow({
  pkg,
  selected,
  onSelect,
  open,
  onOpen,
}: {
  pkg: PackageReport;
  selected: boolean;
  onSelect: () => void;
  open: boolean;
  onOpen: () => void;
}) {
  const current = pkg.installed ?? pkg.locked;
  const updatable = pkg.update !== "none";
  const compat = COMPAT[pkg.compatibility];
  const status = STATUS[pkg.status];
  return (
    <TableRow className="font-mono text-[12px]">
      <TableCell>
        <Checkbox
          checked={selected}
          disabled={!updatable}
          onCheckedChange={onSelect}
          aria-label={t("Select {name}", { name: pkg.name })}
        />
      </TableCell>
      <TableCell>
        <button type="button" onClick={onOpen} className="flex items-center gap-1.5 text-left hover:text-ember">
          <ChevronRight className={cn("size-3 shrink-0 transition-transform", open && "rotate-90")} />
          <span className="max-w-56 truncate">{pkg.name}</span>
          <span className="rounded border px-1 text-[10px] text-muted-foreground">
            {pkg.ecosystem === "composer" ? t("composer") : t("npm")}
            {pkg.kind !== "prod" && ` · ${pkg.kind}`}
          </span>
        </button>
      </TableCell>
      <TableCell
        title={
          pkg.locked && pkg.installed && pkg.locked !== pkg.installed
            ? t("locked {version}", { version: pkg.locked })
            : undefined
        }
      >
        {current ?? <span className="text-muted-foreground">—</span>}
        {pkg.locked && pkg.installed && pkg.locked !== pkg.installed && <span className="text-amber"> *</span>}
      </TableCell>
      <TableCell className="text-muted-foreground">{pkg.declared}</TableCell>
      <TableCell>
        {pkg.latest ?? <span className="text-muted-foreground">—</span>}
        {pkg.update === "major" && <span className="ml-1 text-amber">{t("major")}</span>}
        {pkg.wanted && pkg.wanted !== pkg.latest && pkg.wanted !== current && (
          <div className="text-[11px] text-muted-foreground">{t("in range {wanted}", { wanted: pkg.wanted })}</div>
        )}
      </TableCell>
      <TableCell title={compat.title}>
        {updatable ? (
          <span className="flex items-center gap-1.5">
            <Led tone={compat.tone} /> {compat.label}
          </span>
        ) : (
          <span className="text-muted-foreground">—</span>
        )}
      </TableCell>
      <TableCell>
        <span className="flex items-center gap-1.5">
          {pkg.status === "vulnerable" ? (
            <ShieldAlert className="size-3.5 text-destructive" />
          ) : (
            <Led tone={status.tone} />
          )}
          {status.label}
        </span>
      </TableCell>
    </TableRow>
  );
}

function LinkText({ href, label }: { href: string; label: string }) {
  return (
    <button type="button" onClick={() => openInBrowser(href)} className="text-ember hover:underline">
      {label}
    </button>
  );
}

function PackageDetails({ pkg }: { pkg: PackageReport }) {
  return (
    <TableRow className="bg-well/60 hover:bg-well/60">
      <TableCell />
      <TableCell colSpan={6} className="py-2.5 text-[12.5px] whitespace-normal">
        <div className="mb-1.5 flex gap-3 text-[12px]">
          <LinkText href={pkg.links.registry} label={t("registry")} />
          {pkg.links.releases && <LinkText href={pkg.links.releases} label={t("releases")} />}
          {pkg.links.repository && <LinkText href={pkg.links.repository} label={t("repository")} />}
          {pkg.links.homepage && <LinkText href={pkg.links.homepage} label={t("homepage")} />}
        </div>
        {pkg.deprecated && (
          <p className="text-amber">{t("Deprecated: {deprecated}", { deprecated: pkg.deprecated })}</p>
        )}
        {pkg.reasons.length > 0 ? (
          <ul className="list-disc pl-4 text-muted-foreground">
            {pkg.reasons.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        ) : (
          <p className="text-muted-foreground">{t("Nothing to note.")}</p>
        )}
        {pkg.advisories.map((a) => (
          <div key={a.id} className="mt-2 flex items-start gap-2">
            <ShieldAlert className="mt-0.5 size-3.5 shrink-0 text-destructive" />
            <span>
              <button
                type="button"
                onClick={() => openInBrowser(a.url)}
                className="font-mono text-destructive hover:underline"
              >
                {a.id}
              </button>{" "}
              {a.severity && <span className="font-mono text-[11px]">[{a.severity.toLowerCase()}]</span>} {a.summary}
              {a.fixed && <span className="text-muted-foreground"> {t("· fixed in {fixed}", { fixed: a.fixed })}</span>}
            </span>
          </div>
        ))}
      </TableCell>
    </TableRow>
  );
}

/** Shows the exact plan before anything runs, then follows the job. */
function UpdateDialog({
  site,
  packages,
  open,
  onClose,
}: {
  site: ProjectView;
  packages: string[];
  open: boolean;
  onClose: (done: boolean) => void;
}) {
  const [allowMajor, setAllowMajor] = useState(false);
  const [runScripts, setRunScripts] = useState(false);
  const [jobId, setJobId] = useState<number | null>(null);
  const choice: UpdateChoice | null = open
    ? { name: site.name, packages, allow_major: allowMajor, run_scripts: runScripts }
    : null;
  const plan = usePlanUpdate(jobId === null ? choice : null);
  const update = useUpdateDeps();
  const job = useJob(jobId);
  const done = !!job.data && jobFinished(job.data.status);

  const close = () => {
    onClose(done && job.data?.status === "completed");
    setJobId(null);
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("Update")} {packages.length === 1 ? packages[0] : tn("{n} package", "{n} packages", packages.length)}
          </DialogTitle>
          <DialogDescription>
            {t(
              "Review the versions and the exact commands. Nothing runs until you confirm; your own package manager and lockfile are used.",
            )}
          </DialogDescription>
        </DialogHeader>

        {jobId === null ? (
          <>
            <div className="flex flex-wrap gap-x-6 gap-y-2 text-sm">
              <label className="flex items-center gap-2" htmlFor="major">
                <Switch
                  id="major"
                  checked={allowMajor}
                  onCheckedChange={setAllowMajor}
                  className="data-checked:bg-ember"
                />
                {t("Allow new major versions")}
              </label>
              <label className="flex items-center gap-2" htmlFor="scripts">
                <Switch
                  id="scripts"
                  checked={runScripts}
                  onCheckedChange={setRunScripts}
                  className="data-checked:bg-ember"
                />
                {t("Run install scripts")}
              </label>
            </div>

            <div className="max-h-[50vh] overflow-y-auto">
              {plan.isPending ? (
                <div className="flex flex-col gap-2">
                  <Skeleton className="h-12" />
                  <Skeleton className="h-12" />
                </div>
              ) : plan.isError ? (
                <p className="text-sm text-destructive">{(plan.error as Error).message}</p>
              ) : (
                plan.data && (
                  <div className="flex flex-col gap-3">
                    {plan.data.changes.map((c) => (
                      <div key={c.name} className="rounded-lg border px-3 py-2.5">
                        <div className="flex flex-wrap items-baseline gap-2 font-mono text-[13px]">
                          <span className="font-semibold">{c.name}</span>
                          <span>
                            {c.from ?? "?"} → {c.to}
                          </span>
                          {c.update !== "none" && <span className="text-[11px] text-muted-foreground">{c.update}</span>}
                          <span
                            className="ml-auto flex items-center gap-1.5 text-[11.5px]"
                            title={COMPAT[c.compatibility].title}
                          >
                            <Led tone={COMPAT[c.compatibility].tone} /> {COMPAT[c.compatibility].label}
                          </span>
                        </div>
                        {c.constraint && (
                          <p className="mt-1 text-[12px] text-amber">
                            {t("Constraint becomes {constraint}", { constraint: c.constraint })}
                          </p>
                        )}
                        <ul className="mt-1 list-disc pl-4 text-[12.5px] text-muted-foreground">
                          {c.reasons.map((r) => (
                            <li key={r}>{r}</li>
                          ))}
                        </ul>
                        <div className="mt-1.5 flex gap-3 text-[12px]">
                          {c.links.releases && (
                            <button
                              type="button"
                              onClick={() => openInBrowser(c.links.releases!)}
                              className="text-ember hover:underline"
                            >
                              {t("release notes")}
                            </button>
                          )}
                          {c.links.repository && (
                            <button
                              type="button"
                              onClick={() => openInBrowser(c.links.repository!)}
                              className="text-ember hover:underline"
                            >
                              {t("repository")}
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => openInBrowser(c.links.registry)}
                            className="text-ember hover:underline"
                          >
                            {t("registry")}
                          </button>
                        </div>
                      </div>
                    ))}
                    {plan.data.warnings.map((w) => (
                      <p key={w} className="text-[13px] text-amber">
                        {w}
                      </p>
                    ))}
                    {plan.data.commands.length > 0 && (
                      <Section title={t("will run in the site's folder")}>
                        <pre
                          className="overflow-x-auto rounded-md bg-well px-3 py-2 font-mono text-[12px]"
                          data-selectable
                        >
                          {plan.data.commands.map((c) => `$ ${c.join(" ")}`).join("\n")}
                        </pre>
                        <p className="mt-1.5 text-[12px] text-muted-foreground">
                          {tx("Changes {files}. {scripts} No migrations or other commands run.", {
                            files: plan.data.files.join(", "),
                            scripts: plan.data.run_scripts
                              ? t("Install scripts will run.")
                              : t("Install scripts are skipped."),
                          })}
                        </p>
                      </Section>
                    )}
                  </div>
                )
              )}
            </div>

            <div className="flex justify-end gap-2">
              <Button variant="outline" onClick={close}>
                {t("Cancel")}
              </Button>
              <Button
                disabled={!plan.data?.commands.length || update.isPending}
                onClick={() =>
                  choice &&
                  update.mutate(choice, {
                    onSuccess: (job) => setJobId(job.id),
                  })
                }
                className="bg-ember text-ember-foreground hover:bg-ember/85"
              >
                {t("Run the update")}
              </Button>
            </div>
          </>
        ) : (
          <>
            <div className="flex items-center gap-2 font-mono text-[12.5px]">
              <Led
                tone={
                  job.data?.status === "completed"
                    ? "live"
                    : job.data?.status === "failed"
                      ? "fail"
                      : done
                        ? "warn"
                        : "info"
                }
                live={!done}
              />
              {job.data?.status ?? t("queued")}
              {job.data?.progress && ` · ${job.data.progress.label}`}
            </div>
            <pre
              className="max-h-72 overflow-auto rounded-md bg-well px-3 py-2 font-mono text-[11.5px] leading-relaxed"
              data-selectable
            >
              {(job.data?.log ?? []).join("\n") || "waiting…"}
            </pre>
            {job.data?.result && <p className="text-sm text-phosphor">{job.data.result}</p>}
            {job.data?.error && <p className="text-sm text-destructive">{job.data.error}</p>}
            <div className="flex justify-end">
              <Button variant="outline" onClick={close}>
                {done ? t("Close") : t("Hide (it keeps running)")}
              </Button>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  );
}
