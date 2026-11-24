// Tools → Tunnels and the "Share on the internet" dialog. The daemon runs
// the provider's own program for one site and reads the address back.

import { useEffect, useState } from "react";
import { Copy, Download, ExternalLink, Globe, Lock, ScrollText, Share2, Square } from "lucide-react";

import { Page, Section, useCrumbs } from "@/components/page";
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
import { useJobs, useProjects } from "@/hooks/use-daemon";
import {
  useTunnelInstall,
  useTunnelLogs,
  useTunnelProviders,
  useTunnels,
  useTunnelStart,
  useTunnelStop,
} from "@/hooks/use-tunnels";
import { tildify, uptime } from "@/lib/format";
import { t } from "@/lib/i18n";
import { subById, viewById } from "@/lib/nav";
import type { TunnelProvider, TunnelState, TunnelView } from "@/lib/protocol";
import { jobFinished } from "@/lib/protocol";
import { copyText, openInBrowser } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";

const STATE: Record<TunnelState, { label: string; tone: Tone }> = {
  online: { label: t("online"), tone: "live" },
  starting: { label: t("connecting"), tone: "warn" },
  failed: { label: t("failed"), tone: "fail" },
  stopped: { label: t("stopped"), tone: "off" },
};

/** The daemon's provider facts (English), as the UI shows them. */
const LIMITS: Record<string, string> = {
  "For testing and development, with no uptime guarantee; use is subject to Cloudflare's Online Services Terms of Use.":
    t(
      "For testing and development, with no uptime guarantee; use is subject to Cloudflare's Online Services Terms of Use.",
    ),
  "Up to 200 requests at the same time (more get 429); Server-Sent Events don't work.": t(
    "Up to 200 requests at the same time (more get 429); Server-Sent Events don't work.",
  ),
  "The address changes every time the tunnel starts and stops working when it stops.": t(
    "The address changes every time the tunnel starts and stops working when it stops.",
  ),
  "Anyone with the address can open the site, unless you limit it to e-mail addresses (they confirm with a one-time PIN).":
    t(
      "Anyone with the address can open the site, unless you limit it to e-mail addresses (they confirm with a one-time PIN).",
    ),
};

export function TunnelsView() {
  const view = viewById("tools");
  const sub = subById(view, "tunnels")!;
  const crumbs = useCrumbs(view, sub);
  const tunnels = useTunnels();
  const providers = useTunnelProviders();
  const { setSharing } = useUi();
  const [logs, setLogs] = useState<string | null>(null);

  return (
    <Page view={view} title={sub.label} description={sub.description} crumbs={crumbs} wide>
      <div className="flex flex-col gap-8">
        <p className="max-w-3xl text-[13px] text-muted-foreground">
          {t(
            "Share a running site on the internet for a demo or a webhook. LocDev runs the provider's own program and shares only that site's server: never the proxy, a database or LocDev itself. Stop it when you're done; it also ends when LocDev stops.",
          )}
        </p>

        <Section
          title={t("shared sites")}
          aside={
            <Button size="sm" variant="outline" onClick={() => setSharing("")}>
              <Share2 data-icon="inline-start" /> {t("Share a site")}
            </Button>
          }
        >
          {tunnels.isPending ? (
            <Skeleton className="h-24" />
          ) : (tunnels.data ?? []).length === 0 ? (
            <div className="flex flex-col items-center gap-2 rounded-lg border border-dashed px-6 py-10 text-center">
              <Globe className="size-6 text-muted-foreground" />
              <p className="text-sm">{t("No site is shared")}</p>
              <p className="max-w-md text-[13px] text-muted-foreground">
                {t("Use Share on the internet in a site's menu, or the button above.")}
              </p>
            </div>
          ) : (
            <ul className="flex flex-col gap-3 pt-1">
              {tunnels.data!.map((tunnel) => (
                <TunnelCard key={tunnel.site} tunnel={tunnel} onLogs={() => setLogs(tunnel.site)} />
              ))}
            </ul>
          )}
        </Section>

        <Section title={t("providers")}>
          {providers.isPending ? (
            <Skeleton className="h-32" />
          ) : (
            (providers.data ?? []).map((p) => <ProviderCard key={p.id} provider={p} />)
          )}
        </Section>
      </div>
      <LogsDialog site={logs} onClose={() => setLogs(null)} />
    </Page>
  );
}

function TunnelCard({ tunnel, onLogs }: { tunnel: TunnelView; onLogs: () => void }) {
  const stop = useTunnelStop();
  const { openSite } = useUi();
  const state = STATE[tunnel.state];

  return (
    <li className="rounded-lg border">
      <div className="flex flex-wrap items-center gap-3 px-4 py-3">
        <Led tone={state.tone} live={tunnel.state === "online"} />
        <button
          type="button"
          onClick={() => openSite(tunnel.site)}
          className="font-mono text-sm hover:text-ember hover:underline"
        >
          {tunnel.site}.test
        </button>
        <span className="font-mono text-[11px] text-muted-foreground">
          {state.label}
          {tunnel.state === "online" && tunnel.started_at ? ` · ${uptime(tunnel.started_at)}` : ""}
        </span>
        <span className="ml-auto flex gap-1">
          <Button size="sm" variant="ghost" onClick={onLogs}>
            <ScrollText data-icon="inline-start" /> {t("Log")}
          </Button>
          <Button size="sm" variant="outline" disabled={stop.isPending} onClick={() => stop.mutate(tunnel.site)}>
            <Square data-icon="inline-start" className="fill-current" /> {t("Stop sharing")}
          </Button>
        </span>
      </div>
      <div className="flex flex-col gap-1.5 border-t px-4 py-3 text-[13px]">
        {tunnel.url ? (
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-mono text-[13px] text-ember" data-selectable>
              {tunnel.url}
            </span>
            <Button size="icon-xs" variant="ghost" aria-label={t("Copy address")} onClick={() => copyText(tunnel.url!)}>
              <Copy />
            </Button>
            <Button size="icon-xs" variant="ghost" aria-label={t("Open")} onClick={() => openInBrowser(tunnel.url!)}>
              <ExternalLink />
            </Button>
          </div>
        ) : tunnel.state === "starting" ? (
          <span className="text-muted-foreground">{t("Waiting for the address…")}</span>
        ) : null}
        <span className="font-mono text-[11.5px] text-muted-foreground">
          → {tunnel.target} · {tunnel.provider}
        </span>
        {tunnel.allowed.length > 0 ? (
          <span className="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
            <Lock className="size-3.5 text-phosphor" />
            {t("Only {emails}, after an e-mail PIN", { emails: tunnel.allowed.join(", ") })}
          </span>
        ) : (
          <span className="text-[12.5px] text-amber">{t("Anyone with the address can open it.")}</span>
        )}
        {tunnel.error && <span className="text-[12.5px] text-destructive">{tunnel.error}</span>}
      </div>
    </li>
  );
}

function ProviderCard({ provider }: { provider: TunnelProvider }) {
  const install = useTunnelInstall();
  const jobs = useJobs();
  const installing = (jobs.data ?? []).some((j) => j.kind === "tunnel_install" && !jobFinished(j.status));
  const program = provider.program;

  return (
    <div className="flex flex-col gap-3 rounded-lg border px-4 py-3">
      <div className="flex flex-wrap items-center gap-3">
        <Led tone={program ? "live" : "off"} />
        <span className="text-sm font-medium">{provider.name}</span>
        <span className="font-mono text-[11px] text-muted-foreground">{provider.license}</span>
        <span className="ml-auto flex gap-1">
          <Button size="sm" variant="ghost" onClick={() => openInBrowser(provider.docs)}>
            <ExternalLink data-icon="inline-start" /> {t("Docs")}
          </Button>
          {provider.installable && (
            <Button
              size="sm"
              variant="outline"
              disabled={installing || install.isPending}
              onClick={() => install.mutate(provider.id)}
            >
              <Download data-icon="inline-start" />
              {installing ? t("Installing…") : program?.managed ? t("Update") : t("Install")}
            </Button>
          )}
        </span>
      </div>
      <p className="text-[13px] text-muted-foreground">
        {t("A random trycloudflare.com address for the site, through Cloudflare's network. No account and no domain.")}
      </p>
      <p className="font-mono text-[12px]">
        {program ? (
          <>
            cloudflared {program.version} ·{" "}
            <span className="text-muted-foreground">
              {program.managed ? t("downloaded and verified by LocDev") : tildify(program.path)}
            </span>
            {program.outdated && <span className="text-amber"> · {program.outdated}</span>}
          </>
        ) : provider.installable ? (
          <span className="text-muted-foreground">{t("cloudflared isn't installed.")}</span>
        ) : (
          <span className="text-amber">{provider.install_note}</span>
        )}
      </p>
      <ul className="flex flex-col gap-1 text-[12.5px] text-muted-foreground">
        {provider.limits.map((limit) => (
          <li key={limit}>· {LIMITS[limit] ?? limit}</li>
        ))}
      </ul>
    </div>
  );
}

function LogsDialog({ site, onClose }: { site: string | null; onClose: () => void }) {
  const logs = useTunnelLogs(site);
  return (
    <Dialog open={!!site} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("Tunnel log · {site}", { site: site ?? "" })}</DialogTitle>
          <DialogDescription>{t("The tunnel program's own output.")}</DialogDescription>
        </DialogHeader>
        <pre className="max-h-96 overflow-auto rounded-md bg-well px-3 py-2 font-mono text-[11.5px] whitespace-pre-wrap">
          {(logs.data ?? []).map((l) => l.text).join("\n") || t("Nothing yet.")}
        </pre>
      </DialogContent>
    </Dialog>
  );
}

/**
 * "Share on the internet" for one site (`sharing` in the UI state; "" lets
 * the user pick a running site). Shows what sharing means before starting.
 */
export function ShareSiteDialog() {
  const { sharing, setSharing } = useUi();
  const projects = useProjects();
  const providers = useTunnelProviders();
  const tunnels = useTunnels();
  const start = useTunnelStart();
  const [site, setSite] = useState("");
  const [emails, setEmails] = useState("");
  const open = sharing !== null;
  useEffect(() => {
    if (open) {
      setSite(sharing ?? "");
      setEmails("");
      start.reset();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, sharing]);

  const running = (projects.data ?? []).filter((p) => p.process.state === "running" && p.port);
  const provider = providers.data?.[0];
  const current = tunnels.data?.find((tunnel) => tunnel.site === site);
  const shared = start.data ?? current;
  const close = () => setSharing(null);
  const allowed = emails
    .split(",")
    .map((e) => e.trim())
    .filter(Boolean);

  return (
    <Dialog open={open} onOpenChange={(next) => !next && close()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Share2 className="size-4 text-ember" /> {t("Share on the internet")}
          </DialogTitle>
          <DialogDescription>
            {t("The site gets a public address while the tunnel runs. Only this site's server is shared.")}
          </DialogDescription>
        </DialogHeader>

        {shared?.url ? (
          <div className="flex flex-col gap-2">
            <span className="text-[12.5px] text-muted-foreground">{t("Shared at")}</span>
            <div className="flex items-center gap-2 rounded-md border bg-well/50 px-3 py-2">
              <span className="min-w-0 flex-1 truncate font-mono text-[13px] text-ember" data-selectable>
                {shared.url}
              </span>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label={t("Copy address")}
                onClick={() => copyText(shared.url!)}
              >
                <Copy />
              </Button>
              <Button size="icon-xs" variant="ghost" aria-label={t("Open")} onClick={() => openInBrowser(shared.url!)}>
                <ExternalLink />
              </Button>
            </div>
            {shared.allowed.length === 0 && (
              <p className="text-[12.5px] text-amber">{t("Anyone with the address can open it.")}</p>
            )}
          </div>
        ) : (
          <div className="flex flex-col gap-3">
            {!sharing && (
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-muted-foreground">{t("Site")}</span>
                <select
                  value={site}
                  onChange={(e) => setSite(e.target.value)}
                  aria-label={t("Site")}
                  className="h-8 rounded-md border bg-background px-2 font-mono text-[13px]"
                >
                  <option value="">{t("Pick a running site")}</option>
                  {running.map((p) => (
                    <option key={p.name} value={p.name}>
                      {p.domain}
                    </option>
                  ))}
                </select>
              </label>
            )}
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-muted-foreground">{t("Who can open it")}</span>
              <Input
                value={emails}
                onChange={(e) => setEmails(e.target.value)}
                placeholder={t("Anyone with the address, or: ana@example.com, *@acme.com")}
                aria-label={t("Allowed e-mail addresses")}
                className="font-mono text-[12.5px]"
              />
              <span className="text-[12px] text-muted-foreground">
                {t("With e-mail addresses, visitors confirm theirs with a one-time PIN before seeing the site.")}
              </span>
            </label>
            {provider && (
              <ul
                className={cn(
                  "flex flex-col gap-1 rounded-md border px-3 py-2 text-[12px] text-muted-foreground",
                  !provider.program && "opacity-70",
                )}
              >
                <li className="font-medium text-foreground">{provider.name}</li>
                {provider.limits.map((limit) => (
                  <li key={limit}>· {LIMITS[limit] ?? limit}</li>
                ))}
              </ul>
            )}
            {provider && !provider.program && (
              <p className="text-[12.5px] text-amber">
                {t("cloudflared isn't installed: install it in Tools → Tunnels first.")}
              </p>
            )}
            {start.error && <p className="text-[12.5px] text-destructive">{start.error.message}</p>}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={close}>
            {shared?.url ? t("Done") : t("Cancel")}
          </Button>
          {!shared?.url && (
            <Button
              disabled={!site || !provider?.program || start.isPending}
              onClick={() => start.mutate({ site, provider: provider?.id, allowed })}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
            >
              <Share2 data-icon="inline-start" />
              {start.isPending ? t("Connecting…") : t("Share")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
