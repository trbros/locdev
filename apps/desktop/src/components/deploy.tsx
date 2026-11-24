import { CircleCheck, CircleX, ExternalLink, FileCode, Rocket, TriangleAlert } from "lucide-react";
import { RemoteDeploySection } from "@/components/deploy-remote";
import { useState } from "react";
import { toast } from "sonner";

import { CodeView, languageForPath } from "@/components/code-editor";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { errorMessage } from "@/hooks/use-daemon";
import { useDeployActions, useDeployReport } from "@/hooks/use-deploy";
import type {
  ReadinessLevel,
  DeployConfig,
  DeployPoint,
  ProjectView,
  ProviderFit,
  Rendering,
  Verdict,
} from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const RENDERING: Record<Rendering, string> = {
  static: t("Static files"),
  spa: "Client-side app (static files)",
  ssr: t("Server rendering"),
  server: t("Application server"),
};

const VERDICT: Record<Verdict, { label: string; className: string }> = {
  fits: { label: "fits", className: "text-phosphor" },
  needs_changes: { label: t("needs changes"), className: "text-amber" },
  not_suitable: { label: t("not suitable"), className: "text-muted-foreground" },
};

const LEVEL: Record<ReadinessLevel, { icon: typeof CircleCheck; className: string }> = {
  ok: { icon: CircleCheck, className: "text-phosphor" },
  warn: { icon: TriangleAlert, className: "text-amber" },
  fail: { icon: CircleX, className: "text-destructive" },
};

function Points({ title, points }: { title: string; points: DeployPoint[] }) {
  if (points.length === 0) return null;
  return (
    <div>
      <p className="font-mono text-[10.5px] tracking-wide text-muted-foreground uppercase">{title}</p>
      <ul className="mt-0.5 flex flex-col gap-0.5 text-[12.5px]">
        {points.map((p) => (
          <li key={p.text} className="flex items-baseline gap-1.5">
            <span className="text-muted-foreground">·</span>
            <span>{p.text}</span>
            {p.source && (
              <button
                type="button"
                onClick={() => openInBrowser(p.source!)}
                className="shrink-0 text-ember hover:underline"
                aria-label={t("Source: {source}", { source: p.source })}
                title={p.source}
              >
                <ExternalLink className="inline size-3" />
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

function ProviderCard({ fit, onConfig }: { fit: ProviderFit; onConfig: (provider: string) => void }) {
  const [open, setOpen] = useState(fit.verdict !== "not_suitable");
  const verdict = VERDICT[fit.verdict];
  return (
    <li className={cn("rounded-lg border", fit.verdict === "not_suitable" && "opacity-75")}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex w-full items-baseline gap-2 px-4 py-2.5 text-left"
        aria-expanded={open}
      >
        <span className="text-[13.5px] font-medium">{fit.name}</span>
        <span className="font-mono text-[11px] text-muted-foreground">{fit.model}</span>
        <span className={cn("ml-auto font-mono text-[11.5px]", verdict.className)}>{verdict.label}</span>
      </button>
      {open && (
        <div className="flex flex-col gap-2.5 border-t px-4 py-3">
          <p className="text-[12.5px] text-muted-foreground">{fit.summary}</p>
          {fit.verdict === "not_suitable" ? (
            <Points title={t("why not")} points={fit.limitations} />
          ) : (
            <>
              <Points title={t("for this site")} points={fit.requirements} />
              <Points title={t("strengths")} points={fit.strengths} />
              <Points title={t("limitations")} points={fit.limitations} />
            </>
          )}
          <div className="flex flex-wrap items-center gap-2">
            {fit.config_file && fit.verdict !== "not_suitable" && (
              <Button size="xs" variant="outline" onClick={() => onConfig(fit.id)}>
                <FileCode data-icon="inline-start" /> {t("Prepare")} {fit.config_file}…
              </Button>
            )}
            {fit.docs && (
              <Button size="xs" variant="ghost" onClick={() => openInBrowser(fit.docs)}>
                <ExternalLink data-icon="inline-start" /> {t("Docs")}
              </Button>
            )}
            {fit.setup && !fit.config_file && <span className="text-[11.5px] text-muted-foreground">{fit.setup}</span>}
          </div>
        </div>
      )}
    </li>
  );
}

export function SiteDeployTab({ site }: { site: ProjectView }) {
  const report = useDeployReport(site.name);
  const actions = useDeployActions(site.name);
  const [config, setConfig] = useState<DeployConfig | null>(null);

  if (report.isPending) return <Skeleton className="h-64" />;
  if (report.isError || !report.data) return <p className="text-sm text-destructive">{errorMessage(report.error)}</p>;
  const { profile: p, providers, checks, reviewed } = report.data;
  const secrets = p.env.filter((e) => e.secret).length;

  const prepare = async (provider: string) => {
    try {
      setConfig(await actions.config(provider));
    } catch (e) {
      toast.error(t("LocDev can't prepare that file"), { description: errorMessage(e) });
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <RemoteDeploySection site={site} />
      <section aria-label={t("Stack")} className="rounded-lg border px-4 py-3">
        <div className="flex items-baseline gap-2">
          <Rocket className="size-4 self-center text-muted-foreground" />
          <h3 className="text-[14px] font-medium">
            {p.framework}
            {p.version ? ` ${p.version}` : ""}
          </h3>
          <span className="font-mono text-[11.5px] text-muted-foreground">
            {p.runtime} {p.runtime_version ?? t("(version not declared)")}
          </span>
          <span className="ml-auto text-[11.5px] text-muted-foreground">
            {t("Recommendations only; nothing is sent to any platform.")}
          </span>
        </div>
        <p className="mt-1 text-[12.5px]">
          {RENDERING[p.rendering]} <span className="text-muted-foreground">· {p.rendering_evidence}</span>
        </p>
        <dl className="mt-2 grid grid-cols-[7rem_1fr] gap-x-3 gap-y-0.5 font-mono text-[12px]">
          {p.build_command && (
            <>
              <dt className="text-muted-foreground">{t("build")}</dt>
              <dd className="truncate">{p.build_command}</dd>
            </>
          )}
          {p.start_command && (
            <>
              <dt className="text-muted-foreground">{t("start")}</dt>
              <dd>{p.start_command}</dd>
            </>
          )}
          {p.output_dir && (
            <>
              <dt className="text-muted-foreground">{t("output")}</dt>
              <dd>{p.output_dir}</dd>
            </>
          )}
        </dl>
        {p.needs.length > 0 && (
          <ul className="mt-2 flex flex-wrap gap-1.5">
            {p.needs.map((n) => (
              <li key={n.id} title={n.evidence} className="rounded border px-1.5 py-0.5 text-[11.5px]">
                {n.label}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-label={t("Platforms")} className="flex flex-col gap-2">
        <h3 className="text-[13px] font-medium">
          {t("Platforms")}{" "}
          <span className="font-normal text-muted-foreground">
            {t("· ranked by what this site needs, not by popularity")}
          </span>
        </h3>
        <ul className="flex flex-col gap-2">
          {providers.map((f) => (
            <ProviderCard key={f.id} fit={f} onConfig={prepare} />
          ))}
        </ul>
        <p className="text-[11px] text-muted-foreground">
          {t("Platform facts compiled {reviewed} from each platform's docs; every link opens its source.", {
            reviewed,
          })}
        </p>
      </section>

      <div className="grid gap-5 lg:grid-cols-2">
        <section aria-label={t("Readiness")} className="flex flex-col gap-1.5">
          <h3 className="text-[13px] font-medium">{t("Readiness")}</h3>
          <ul className="divide-y rounded-lg border">
            {checks.map((c) => {
              const Icon = LEVEL[c.level].icon;
              return (
                <li key={c.id} className="flex gap-2 px-3 py-2">
                  <Icon className={cn("mt-0.5 size-3.5 shrink-0", LEVEL[c.level].className)} />
                  <div>
                    <p className="text-[12.5px]">{c.title}</p>
                    <p className="text-[11.5px] text-muted-foreground">{c.detail}</p>
                  </div>
                </li>
              );
            })}
          </ul>
        </section>
        <section aria-label={t("Environment variables")} className="flex flex-col gap-1.5">
          <h3 className="text-[13px] font-medium">
            {t("Environment variables")}{" "}
            <span className="font-normal text-muted-foreground">
              {t("· {length} ({secrets} secret), names only", { length: p.env.length, secrets })}
            </span>
          </h3>
          <ul className="max-h-80 divide-y overflow-y-auto rounded-lg border">
            {p.env.map((e) => (
              <li key={e.name} className="flex items-baseline gap-2 px-3 py-1.5">
                <span className="font-mono text-[12px]">{e.name}</span>
                {e.secret && <span className="font-mono text-[10.5px] text-amber">{t("secret")}</span>}
                <span className="ml-auto truncate text-right text-[11.5px] text-muted-foreground">{e.note}</span>
              </li>
            ))}
          </ul>
        </section>
      </div>

      <Dialog open={config !== null} onOpenChange={(open) => !open && setConfig(null)}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>{config?.file}</DialogTitle>
            <DialogDescription>
              {config?.exists
                ? t("This file already exists in the project; LocDev doesn't overwrite it.")
                : t("A starting point written from what LocDev detected. No value from .env is in it.")}
            </DialogDescription>
          </DialogHeader>
          <CodeView
            value={config?.content ?? ""}
            language={languageForPath(config?.file ?? "")}
            ariaLabel={config?.file ?? t("Generated file")}
            className="max-h-96"
          />
          {config?.notes.map((n) => (
            <p key={n} className="text-[12px] text-muted-foreground">
              {n}
            </p>
          ))}
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfig(null)}>
              {t("Close")}
            </Button>
            <Button
              disabled={!config || config.exists || actions.write.isPending}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
              onClick={() =>
                config &&
                actions.write.mutate(
                  { provider: config.provider, hash: config.hash },
                  {
                    onSuccess: (r) => {
                      toast.success(`Created ${r.file}`, {
                        description: t("Review it and commit it with the project."),
                      });
                      setConfig(null);
                    },
                    onError: (e) => toast.error(t("Not written"), { description: errorMessage(e) }),
                  },
                )
              }
            >
              {t("Create")} {config?.file}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
