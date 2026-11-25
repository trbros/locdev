import { AlertTriangle, FolderOpen, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";

import { useState } from "react";

import { DepsTab } from "@/components/deps";
import { AddressBar } from "@/components/site-browser";
import { DevServerControls, DevServerPrompt } from "@/components/dev-server";
import { SiteConfigTab } from "@/components/config-files";
import { SiteDeployTab } from "@/components/deploy";
import { SiteEmailTab } from "@/components/email-studio";
import { SiteEnvTab } from "@/components/env";
import { MailSection } from "@/components/mail-setup";
import { SubdomainsSection } from "@/components/subdomains";
import { GitTab } from "@/components/git";
import { PhpExtensionsSection, SiteMetricsTab, SiteTinkerTab } from "@/components/insights";
import { GithubTab } from "@/components/github";
import { Section, SubTabs } from "@/components/page";
import { ProcessesPanel } from "@/components/processes-panel";
import { SiteSettingsForm } from "@/components/site-dialog";
import { StackSection } from "@/components/stack";
import { Led, levelTone } from "@/components/status";
import { Terminal } from "@/components/terminal";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { SwitchField } from "@/components/ui/switch-field";
import { useAppInfo, useGitStatus, useSetSecure, useSiteHttps, useStatus } from "@/hooks/use-daemon";
import { useAuxProcesses } from "@/hooks/use-procs";
import { tildify } from "@/lib/format";
import { SITE_TABS, type SiteTab } from "@/lib/nav";
import { DEV_TOOL_LABEL, type ProcessRole, type ProjectView, RUNTIME_LABEL } from "@/lib/protocol";
import { revealFolder } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function SiteDetail({ site, sites }: { site: ProjectView; sites: ProjectView[] }) {
  const { siteTab, setSiteTab } = useUi();
  // The GitHub tab only exists for repositories with a GitHub remote.
  const github = useGitStatus(site.name).data?.github ?? null;
  // Tinker only for Laravel apps.
  const tabs = SITE_TABS.filter(
    (t) => (t.id !== "github" || github) && (t.id !== "tinker" || site.stack.adapter === "laravel"),
  );
  const tab = tabs.find((t) => t.id === siteTab) ?? tabs[0];

  return (
    <div key={site.name} className="flex h-full min-w-0 flex-col">
      <AddressBar site={site} sites={sites} />
      <div className="px-8 pt-1">
        <SubTabs items={tabs} value={tab.id} onChange={(id) => setSiteTab(id as SiteTab)} />
      </div>

      <div key={tab.id} className="min-h-0 flex-1 animate-in fade-in-0 duration-150">
        {tab.id === "server" ? (
          <ServerTab site={site} />
        ) : (
          <ScrollArea className="h-full">
            <div
              className={cn(
                "flex flex-col gap-6 px-8 pt-5 pb-10",
                tab.id === "email"
                  ? "gap-4"
                  : tab.id === "dependencies" ||
                      tab.id === "metrics" ||
                      tab.id === "tinker" ||
                      tab.id === "config" ||
                      tab.id === "deploy" ||
                      tab.id === "git" ||
                      tab.id === "github"
                    ? "max-w-6xl gap-4"
                    : "max-w-3xl",
              )}
            >
              {tab.id === "overview" ? (
                <Overview site={site} />
              ) : tab.id === "dependencies" ? (
                <DepsTab site={site} />
              ) : tab.id === "environment" ? (
                <SiteEnvTab site={site} />
              ) : tab.id === "config" ? (
                <SiteConfigTab site={site} />
              ) : tab.id === "deploy" ? (
                <SiteDeployTab site={site} />
              ) : tab.id === "email" ? (
                <SiteEmailTab site={site} />
              ) : tab.id === "metrics" ? (
                <SiteMetricsTab site={site} />
              ) : tab.id === "tinker" ? (
                <SiteTinkerTab site={site} />
              ) : tab.id === "git" ? (
                <GitTab site={site} />
              ) : tab.id === "github" && github ? (
                <GithubTab site={site} repo={github} />
              ) : (
                <SettingsTab site={site} />
              )}
            </div>
          </ScrollArea>
        )}
      </div>
    </div>
  );
}

function ServerTab({ site }: { site: ProjectView }) {
  const info = useAppInfo();
  const [process, setProcess] = useState<ProcessRole>("main");
  const [aux, setAux] = useState<string | null>(null);
  const procs = useAuxProcesses(site.name);
  const hasHelper = !!site.dev_server && site.dev_server.mode !== "ask";
  const role = hasHelper ? process : "main";
  const auxView = procs.data?.find((p) => p.id === aux) ?? null;
  const id = auxView ? `${site.name}:${auxView.id}` : role === "dev" ? `${site.name}-dev` : site.name;
  const running = auxView
    ? auxView.info.state === "running"
    : (role === "dev" ? site.dev_process?.state : site.process.state) === "running";
  const tabs: {
    key: string;
    label: string;
    select: () => void;
    active: boolean;
  }[] = [
    {
      key: "main",
      label: t("server output"),
      select: () => (setAux(null), setProcess("main")),
      active: !auxView && role === "main",
    },
    ...(hasHelper
      ? [
          {
            key: "dev",
            label: `${DEV_TOOL_LABEL[site.dev_server!.tool]} output`,
            select: () => (setAux(null), setProcess("dev")),
            active: !auxView && role === "dev",
          },
        ]
      : []),
    ...(procs.data ?? [])
      .filter((p) => p.mode !== "ask" && p.mode !== "never")
      .map((p) => ({
        key: p.id,
        label: `${p.name.toLowerCase()} output`,
        select: () => setAux(p.id),
        active: aux === p.id,
      })),
  ];

  return (
    <div className="flex h-full flex-col gap-3 overflow-y-auto px-8 pt-4 pb-6">
      <DevServerPrompt site={site} />
      <ProcessFacts site={site} />
      <DevServerControls site={site} />
      <ProcessesPanel site={site} selected={aux} onSelect={setAux} />
      {tabs.length > 1 && (
        <div role="tablist" aria-label={t("Process")} className="flex flex-wrap gap-1 font-mono text-[11.5px]">
          {tabs.map((t) => (
            <button
              key={t.key}
              type="button"
              role="tab"
              aria-selected={t.active}
              onClick={t.select}
              className={cn(
                "rounded-md border px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
                t.active && "border-ember/50 bg-ember/6 text-foreground",
              )}
            >
              {t.label}
            </button>
          ))}
        </div>
      )}
      <Terminal
        key={id}
        className="min-h-80"
        name={site.name}
        process={role}
        aux={auxView?.id ?? null}
        running={running}
        logFile={info.data ? `${info.data.log_dir}/sites/${id}.log` : null}
        label={
          auxView
            ? auxView.name.toLowerCase()
            : role === "dev"
              ? DEV_TOOL_LABEL[site.dev_server!.tool].toLowerCase()
              : "output"
        }
      />
    </div>
  );
}

function Overview({ site }: { site: ProjectView }) {
  const { setSiteTab } = useUi();

  return (
    <>
      <DevServerPrompt site={site} />
      {(site.process.message || !site.path_exists) && (
        <div className="flex items-start gap-2.5 rounded-lg border border-destructive/30 bg-destructive/[0.07] px-3.5 py-2.5 text-sm text-destructive">
          <AlertTriangle className="mt-0.5 size-4 shrink-0" />
          <span className="min-w-0 flex-1">
            {!site.path_exists
              ? t("The folder {path} no longer exists.", { path: tildify(site.path) })
              : site.process.message}
          </span>
          {site.path_exists && (
            <button
              type="button"
              onClick={() => setSiteTab("server")}
              className="shrink-0 underline underline-offset-4"
            >
              {t("see output")}
            </button>
          )}
        </div>
      )}

      {/* Facts: a compact spec sheet. */}
      <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 border-y py-3.5 font-mono text-[12px]">
        <Spec label={t("path")}>
          <button
            type="button"
            onClick={() => revealFolder(site.path)}
            className="hover:text-ember"
            title={t("Show in folder")}
          >
            {tildify(site.path)}
          </button>
        </Spec>
        <Spec label={t("stack")}>
          {site.stack.framework}
          {site.stack.package_manager && <span className="text-muted-foreground"> · {site.stack.package_manager}</span>}
        </Spec>
        <Spec label={t("command")}>
          {site.command ?? (
            <button type="button" onClick={() => setSiteTab("settings")} className="text-amber hover:underline">
              {t("none — set one in Settings")}
            </button>
          )}
        </Spec>
        <ProcessSpec site={site} />
        {site.dev_server && site.dev_server.mode !== "ask" && (
          <Spec label={t("helper")}>
            {DEV_TOOL_LABEL[site.dev_server.tool]}
            <span className="text-muted-foreground">
              {" "}
              · {site.dev_process?.state ?? t("stopped")}
              {site.dev_server.mode === "auto" ? t(" · starts with the site") : ""}
            </span>
          </Spec>
        )}
        {site.command && site.stack.command && site.command !== site.stack.command && (
          <Spec label={t("detected")}>
            <span className="text-muted-foreground" title={t("What LocDev would use for this project today")}>
              {site.stack.command}
            </span>
          </Spec>
        )}
      </dl>

      <StackSection site={site} />

      <Section title={t("runtimes")}>
        {site.runtimes.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("No pins, version files or defaults: the site uses what's first on your PATH.")}
          </p>
        ) : (
          <ul>
            {site.runtimes.map((r) => (
              <li
                key={r.kind}
                className="flex items-baseline gap-3 border-b border-dashed py-2 text-sm last:border-b-0"
              >
                <span className="w-24 shrink-0 text-muted-foreground">{RUNTIME_LABEL[r.kind]}</span>
                <span className="min-w-0 flex-1 truncate font-mono text-[12.5px]">
                  {r.version ? (
                    <>
                      {r.version.version}
                      {r.version.source === "system" && (
                        <span className="text-muted-foreground"> · {r.version.origin}</span>
                      )}
                      {r.version.support?.state === "eol" && (
                        <span className="text-destructive"> {t("· end-of-life")}</span>
                      )}
                      {r.unmet && (
                        <span className="text-amber">
                          {" "}
                          · {r.unmet.file} {t("wants")} {r.unmet.constraint}
                        </span>
                      )}
                    </>
                  ) : (
                    <span className="text-destructive">
                      {t("{requested} not installed", { requested: r.requested })}
                    </span>
                  )}
                </span>
                <span className="shrink-0 font-mono text-[11px] text-muted-foreground">
                  {r.from === "default"
                    ? t("default {value}", { value: r.requested })
                    : r.from === "projects.toml"
                      ? t("pin {value}", { value: r.requested })
                      : r.from}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Section>
    </>
  );
}

function ProcessSpec({ site }: { site: ProjectView }) {
  const { pid, exit_code } = site.process;
  return (
    <Spec label={t("process")}>
      {pid ? (
        <>
          pid {pid}
          {site.auto_start && <span className="text-muted-foreground"> {t("· starts with locdevd")}</span>}
        </>
      ) : (
        <span className="text-muted-foreground">
          {exit_code != null ? t("exited with code {exit_code}", { exit_code }) : t("not running")}
          {site.auto_start && t(" · starts with locdevd")}
        </span>
      )}
    </Spec>
  );
}

function ProcessFacts({ site }: { site: ProjectView }) {
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 font-mono text-[12px]">
      <Spec label={t("command")}>{site.command ?? <span className="text-muted-foreground">{t("none")}</span>}</Spec>
      <ProcessSpec site={site} />
    </dl>
  );
}

function SettingsTab({ site }: { site: ProjectView }) {
  const status = useStatus();
  const { navigate } = useUi();
  const tls = status.data?.tls;

  return (
    <>
      <Section title={t("dev server, runtimes and startup")}>
        <SiteSettingsForm site={site} />
      </Section>
      <HttpsSection site={site} />
      <SubdomainsSection site={site} />
      <MailSection site={site} />
      <PhpExtensionsSection site={site} />
      <Section title={t("domains")}>
        <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 font-mono text-[12px]">
          <Spec label={t("domain")}>{site.domain}</Spec>
          <Spec label={t("http")}>{site.url?.replace("https://", "http://") ?? "—"}</Spec>
          <Spec label={t("https")}>
            {site.https_url ?? "—"}
            {site.https_url && (
              <button
                type="button"
                onClick={() => navigate("settings", "https")}
                className={tls?.trusted ? "ml-2 text-phosphor" : "ml-2 text-amber hover:underline"}
              >
                {tls?.trusted ? t("trusted") : t("browsers will warn: trust the CA")}
              </button>
            )}
          </Spec>
          <Spec label={t("direct")}>{site.direct_url ?? "—"}</Spec>
        </dl>
        <p className="mt-2 text-[12.5px] text-muted-foreground">
          {t("The domain follows the site's name. Folder:")} <FolderLink path={site.path} />
        </p>
      </Section>
    </>
  );
}

/** HTTPS for this site: what it needs (checked now) and HTTPS-only. */
function HttpsSection({ site }: { site: ProjectView }) {
  const [checking, setChecking] = useState(false);
  const checks = useSiteHttps(checking ? site.name : null);
  const secure = useSetSecure();

  return (
    <Section
      title={t("https")}
      aside={
        <Button variant="ghost" size="xs" onClick={() => (checking ? checks.refetch() : setChecking(true))}>
          <ShieldCheck data-icon="inline-start" /> {checking ? t("Check again") : t("Check HTTPS")}
        </Button>
      }
    >
      <SwitchField
        className="flex items-start gap-3 py-1.5"
        checked={!!site.secure}
        disabled={secure.isPending || !site.https_url}
        onCheckedChange={(on) => secure.mutate({ name: site.name, secure: on })}
        switchClassName="mt-0.5 data-checked:bg-ember"
      >
        <span>
          <span className="block text-sm">{t("HTTPS only")}</span>
          <span className="block text-[12.5px] text-muted-foreground">
            {tx("Plain HTTP requests are redirected to {url}. Same as {command}.", {
              url: site.https_url ?? t("the HTTPS address"),
              command: <span className="font-mono text-foreground">locdev secure {site.name}</span>,
            })}
          </span>
        </span>
      </SwitchField>
      {checking && (
        <ul className="mt-2">
          {checks.isPending
            ? [0, 1, 2].map((i) => <Skeleton key={i} className="mb-1.5 h-7" />)
            : checks.data?.map((check) => (
                <li key={check.id} className="flex gap-3 border-b border-dashed py-1.5 text-[13px] last:border-b-0">
                  <Led tone={levelTone(check.level)} className="mt-1.5" />
                  <span className="w-36 shrink-0 text-muted-foreground">{check.label}</span>
                  <span className="min-w-0 flex-1">
                    {check.detail}
                    {check.remedy && <span className="ml-2 font-mono text-[11.5px] text-ember">{check.remedy}</span>}
                  </span>
                </li>
              ))}
        </ul>
      )}
    </Section>
  );
}

function FolderLink({ path }: { path: string }) {
  return (
    <button
      type="button"
      onClick={() => revealFolder(path)}
      className="inline-flex items-center gap-1 font-mono hover:text-ember"
    >
      <FolderOpen className="size-3" />
      {tildify(path)}
    </button>
  );
}

function Spec({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 truncate" data-selectable>
        {children}
      </dd>
    </>
  );
}
