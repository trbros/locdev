// Deploy platforms through their APIs: accounts (Settings → Integrations)
// and the site's linked project (Site → Deploy). The daemon keeps tokens,
// checks the local repository and makes every call.

import { useState } from "react";
import { ExternalLink, Link2, Rocket, RotateCcw, ScrollText, TriangleAlert, Unlink, X } from "lucide-react";

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
import { errorMessage } from "@/hooks/use-daemon";
import {
  useRemoteAction,
  useRemoteConnect,
  useRemoteDisconnect,
  useRemoteLink,
  useRemoteLogs,
  useRemotePlatforms,
  useRemoteProjects,
  useRemoteStatus,
} from "@/hooks/use-remote";
import { uptime } from "@/lib/format";
import { t } from "@/lib/i18n";
import type { ProjectView, RemoteDeployment, RemotePlatform, RemoteState } from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";

const STATE: Record<RemoteState, { label: string; tone: Tone }> = {
  queued: { label: t("queued"), tone: "warn" },
  building: { label: t("building"), tone: "warn" },
  ready: { label: t("ready"), tone: "live" },
  error: { label: t("error"), tone: "fail" },
  canceled: { label: t("canceled"), tone: "off" },
  unknown: { label: "?", tone: "off" },
};

/** The daemon's token instructions (English), as the UI shows them. */
const TOKEN_HELP: Record<string, string> = {
  vercel: t(
    "Account Settings → Tokens → Create Token. Scope it to the team that owns the projects (and give its slug or ID here), with an expiration you're comfortable with.",
  ),
  netlify: t(
    "User settings → Applications → Personal access tokens → New access token. It acts as your user on every team you're in.",
  ),
};

/** Settings → Integrations: connect deploy platforms. */
export function DeployPlatformsSection() {
  const platforms = useRemotePlatforms();
  const [connecting, setConnecting] = useState<RemotePlatform | null>(null);
  const disconnect = useRemoteDisconnect();

  return (
    <Section title={t("deploy platforms")}>
      <p className="pt-1 pb-3 text-[13px] text-muted-foreground">
        {t(
          "Connect a platform to see a site's deployments and deploy, cancel or roll back from LocDev. The token is checked with the platform first and kept in your system keyring.",
        )}
      </p>
      {platforms.isPending ? (
        <Skeleton className="h-24" />
      ) : platforms.error ? (
        <p className="text-[12.5px] text-destructive">{errorMessage(platforms.error)}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {(platforms.data ?? []).map((p) => (
            <li key={p.id} className="flex flex-wrap items-center gap-3 rounded-lg border px-4 py-3">
              <Led tone={p.account ? "live" : "off"} />
              <span className="text-sm font-medium">{p.name}</span>
              <span className="text-[12.5px] text-muted-foreground">
                {p.account
                  ? t("connected as {user}", { user: p.account.user }) +
                    (p.account.team ? ` · ${t("team {team}", { team: p.account.team })}` : "")
                  : t("not connected")}
              </span>
              {!p.capabilities.logs && (
                <span className="text-[12px] text-muted-foreground">· {t("no build logs in its API")}</span>
              )}
              <span className="ml-auto flex gap-1">
                {p.account ? (
                  <Button size="sm" variant="ghost" onClick={() => disconnect.mutate(p.id)}>
                    {t("Disconnect")}
                  </Button>
                ) : (
                  <Button size="sm" variant="outline" onClick={() => setConnecting(p)}>
                    {t("Connect…")}
                  </Button>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
      <ConnectDialog platform={connecting} onClose={() => setConnecting(null)} />
    </Section>
  );
}

function ConnectDialog({ platform, onClose }: { platform: RemotePlatform | null; onClose: () => void }) {
  const connect = useRemoteConnect();
  const [token, setToken] = useState("");
  const [team, setTeam] = useState("");
  const close = () => {
    setToken("");
    setTeam("");
    connect.reset();
    onClose();
  };

  return (
    <Dialog open={!!platform} onOpenChange={(open) => !open && close()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("Connect {name}", { name: platform?.name ?? "" })}</DialogTitle>
          <DialogDescription>{platform ? (TOKEN_HELP[platform.id] ?? platform.token_help) : ""}</DialogDescription>
        </DialogHeader>
        {platform && (
          <div className="flex flex-col gap-3">
            <button
              type="button"
              onClick={() => openInBrowser(platform.token_url)}
              className="flex items-center gap-1.5 self-start text-[13px] text-ember hover:underline"
            >
              <ExternalLink className="size-3.5" /> {t("Create a token")}
            </button>
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-muted-foreground">{t("Token")}</span>
              <Input
                type="password"
                value={token}
                onChange={(e) => setToken(e.target.value)}
                autoComplete="off"
                aria-label={t("Token")}
                className="font-mono"
              />
            </label>
            {platform.id === "vercel" && (
              <label className="flex flex-col gap-1.5 text-sm">
                <span className="text-muted-foreground">{t("Team (optional)")}</span>
                <Input
                  value={team}
                  onChange={(e) => setTeam(e.target.value)}
                  placeholder="acme"
                  aria-label={t("Team")}
                  className="font-mono"
                />
              </label>
            )}
            {connect.error && <p className="text-[12.5px] text-destructive">{errorMessage(connect.error)}</p>}
          </div>
        )}
        <DialogFooter>
          <Button variant="outline" onClick={close}>
            {t("Cancel")}
          </Button>
          <Button
            disabled={!token.trim() || connect.isPending}
            onClick={() =>
              platform &&
              connect.mutate(
                { platform: platform.id, token: token.trim(), team: team.trim() || null },
                { onSuccess: close },
              )
            }
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            {connect.isPending ? t("Checking…") : t("Connect")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Site → Deploy: the linked project, its deployments and the actions. */
export function RemoteDeploySection({ site }: { site: ProjectView }) {
  const linked = !!site.deploy;
  return (
    <Section title={t("linked project")}>{linked ? <LinkedProject site={site} /> : <LinkPicker site={site} />}</Section>
  );
}

function LinkPicker({ site }: { site: ProjectView }) {
  const platforms = useRemotePlatforms();
  const connected = (platforms.data ?? []).filter((p) => p.account);
  const [platform, setPlatform] = useState<string | null>(null);
  const chosen = platform ?? connected[0]?.id ?? null;
  const projects = useRemoteProjects(chosen);
  const link = useRemoteLink();

  if (platforms.isPending) return <Skeleton className="h-16" />;
  if (connected.length === 0)
    return (
      <p className="pt-1 text-[13px] text-muted-foreground">
        {t("Connect Vercel or Netlify in Settings → Integrations to deploy this site from LocDev.")}
      </p>
    );
  return (
    <div className="flex flex-col gap-3 pt-1">
      {connected.length > 1 && (
        <div role="radiogroup" aria-label={t("Platform")} className="flex gap-1">
          {connected.map((p) => (
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={p.id === chosen}
              onClick={() => setPlatform(p.id)}
              className={cn(
                "rounded-md border px-2.5 py-1 text-[12.5px]",
                p.id === chosen && "border-ember/60 bg-ember/6",
              )}
            >
              {p.name}
            </button>
          ))}
        </div>
      )}
      {projects.isPending ? (
        <Skeleton className="h-16" />
      ) : projects.error ? (
        <p className="text-[12.5px] text-destructive">{errorMessage(projects.error)}</p>
      ) : (
        <ul className="flex flex-col">
          {(projects.data ?? []).map((p) => (
            <li key={p.id} className="flex items-center gap-3 border-b py-2 text-[13px] last:border-0">
              <span className="font-medium">{p.name}</span>
              <span className="font-mono text-[11.5px] text-muted-foreground">{p.repo ?? ""}</span>
              <Button
                size="xs"
                variant="outline"
                className="ml-auto"
                disabled={link.isPending}
                onClick={() => link.mutate({ site: site.name, platform: chosen, project: p.id })}
              >
                <Link2 data-icon="inline-start" /> {t("Link")}
              </Button>
            </li>
          ))}
          {(projects.data ?? []).length === 0 && (
            <li className="text-[13px] text-muted-foreground">{t("No projects on this account.")}</li>
          )}
        </ul>
      )}
    </div>
  );
}

function LinkedProject({ site }: { site: ProjectView }) {
  const status = useRemoteStatus(site.name, true);
  const action = useRemoteAction();
  const link = useRemoteLink();
  const [confirm, setConfirm] = useState<{
    kind: "run" | "rollback";
    production?: boolean;
    deployment?: RemoteDeployment;
  } | null>(null);
  const [logs, setLogs] = useState<string | null>(null);

  if (status.isPending) return <Skeleton className="h-48" />;
  if (status.error)
    return (
      <div className="flex flex-col gap-2 pt-1">
        <p className="text-[12.5px] text-destructive">{errorMessage(status.error)}</p>
        <Button
          size="sm"
          variant="ghost"
          className="self-start"
          onClick={() => link.mutate({ site: site.name, platform: null, project: null })}
        >
          <Unlink data-icon="inline-start" /> {t("Unlink")}
        </Button>
      </div>
    );
  const s = status.data!;
  const blocked = s.git.problems.length > 0;

  return (
    <div className="flex flex-col gap-4 pt-1">
      <div className="flex flex-wrap items-center gap-3">
        <span className="text-sm font-medium">{s.project.name}</span>
        <span className="text-[12.5px] text-muted-foreground">
          {s.platform.name}
          {s.project.repo && ` · ${s.project.repo}`}
          {s.project.branch && ` · ${t("production: {branch}", { branch: s.project.branch })}`}
        </span>
        <span className="ml-auto flex gap-1">
          <Button size="sm" variant="ghost" onClick={() => openInBrowser(s.project.dashboard)}>
            <ExternalLink data-icon="inline-start" /> {t("Dashboard")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => link.mutate({ site: site.name, platform: null, project: null })}
          >
            <Unlink data-icon="inline-start" /> {t("Unlink")}
          </Button>
        </span>
      </div>

      <div className="flex flex-wrap items-center gap-2 rounded-lg border bg-well/40 px-3.5 py-2.5 text-[12.5px]">
        <span className="text-muted-foreground">{t("From here:")}</span>
        <span className="font-mono">
          {s.git.branch ?? "?"} @ {s.git.commit?.slice(0, 10) ?? "?"}
        </span>
        <span className="ml-auto flex gap-1.5">
          <Button
            size="sm"
            variant="outline"
            disabled={blocked || action.isPending}
            onClick={() => setConfirm({ kind: "run", production: false })}
          >
            <Rocket data-icon="inline-start" /> {t("Deploy preview")}
          </Button>
          <Button
            size="sm"
            disabled={blocked || action.isPending}
            onClick={() => setConfirm({ kind: "run", production: true })}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <Rocket data-icon="inline-start" /> {t("Deploy to production")}
          </Button>
        </span>
        {s.git.problems.map((p) => (
          <p key={p} className="flex w-full gap-2 text-amber">
            <TriangleAlert className="mt-0.5 size-3.5 shrink-0" /> {p}
          </p>
        ))}
      </div>

      <ul className="flex flex-col rounded-lg border">
        {s.deployments.map((d) => {
          const state = STATE[d.state];
          return (
            <li
              key={d.id}
              className="flex flex-wrap items-center gap-x-3 gap-y-1 border-b px-3.5 py-2.5 text-[13px] last:border-0"
            >
              <Led tone={state.tone} live={d.state === "building"} />
              <span className="w-20 font-mono text-[11.5px]">{state.label}</span>
              <span className="min-w-0 flex-1 truncate">
                {d.message ?? d.id}
                <span className="ml-2 font-mono text-[11.5px] text-muted-foreground">
                  {d.branch ?? ""} {d.commit?.slice(0, 7) ?? ""}
                </span>
              </span>
              {d.current && (
                <span className="rounded border border-phosphor/40 px-1.5 font-mono text-[10.5px] text-phosphor">
                  {t("live")}
                </span>
              )}
              {d.production && !d.current && (
                <span className="font-mono text-[10.5px] text-muted-foreground">{t("production")}</span>
              )}
              <span className="font-mono text-[11px] text-muted-foreground">
                {d.created_at ? t("{time} ago", { time: uptime(d.created_at) }) : ""}
              </span>
              <span className="flex gap-0.5">
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Build log")}
                  title={t("Build log")}
                  onClick={() => setLogs(d.id)}
                >
                  <ScrollText />
                </Button>
                {(d.state === "queued" || d.state === "building") && (
                  <Button
                    size="icon-xs"
                    variant="ghost"
                    aria-label={t("Cancel deployment")}
                    title={t("Cancel deployment")}
                    onClick={() => action.mutate({ kind: "cancel", site: site.name, deployment: d.id })}
                  >
                    <X />
                  </Button>
                )}
                {d.state === "ready" && d.production && !d.current && s.platform.capabilities.rollback && (
                  <Button
                    size="icon-xs"
                    variant="ghost"
                    aria-label={t("Roll back to this one")}
                    title={t("Roll back to this one")}
                    onClick={() => setConfirm({ kind: "rollback", deployment: d })}
                  >
                    <RotateCcw />
                  </Button>
                )}
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Open")}
                  onClick={() => openInBrowser(d.url ?? d.dashboard)}
                >
                  <ExternalLink />
                </Button>
              </span>
              {d.error && <p className="w-full pl-6 text-[12px] text-destructive">{d.error}</p>}
            </li>
          );
        })}
      </ul>

      <div className="grid gap-1 text-[12.5px] text-muted-foreground">
        {s.domains.length > 0 && <span>{t("Domains: {list}", { list: s.domains.join(", ") })}</span>}
        {s.env.length > 0 && (
          <span>{t("Variables (names only): {list}", { list: s.env.map((e) => e.key).join(", ") })}</span>
        )}
        {s.notes.map((n) => (
          <span key={n}>{n}</span>
        ))}
      </div>

      <Dialog open={!!confirm} onOpenChange={(open) => !open && setConfirm(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {confirm?.kind === "rollback"
                ? t("Roll production back?")
                : confirm?.production
                  ? t("Deploy to production?")
                  : t("Deploy a preview?")}
            </DialogTitle>
            <DialogDescription>
              {confirm?.kind === "rollback"
                ? t("Production traffic of {project} goes back to {id}.", {
                    project: s.project.name,
                    id: confirm.deployment?.id ?? "",
                  })
                : t("{platform} builds {branch} at {commit} from {repo}: what this site has pushed.", {
                    platform: s.platform.name,
                    branch: s.git.branch ?? "?",
                    commit: s.git.commit?.slice(0, 10) ?? "?",
                    repo: s.project.repo ?? "?",
                  })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setConfirm(null)}>
              {t("Cancel")}
            </Button>
            <Button
              disabled={action.isPending}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
              onClick={() => {
                if (!confirm) return;
                if (confirm.kind === "run")
                  action.mutate(
                    { kind: "run", site: site.name, production: !!confirm.production },
                    { onSettled: () => setConfirm(null) },
                  );
                else
                  action.mutate(
                    { kind: "rollback", site: site.name, deployment: confirm.deployment!.id },
                    { onSettled: () => setConfirm(null) },
                  );
              }}
            >
              {confirm?.kind === "rollback" ? t("Roll back production") : t("Deploy")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <LogsDialog site={site.name} deployment={logs} onClose={() => setLogs(null)} />
    </div>
  );
}

function LogsDialog({ site, deployment, onClose }: { site: string; deployment: string | null; onClose: () => void }) {
  const logs = useRemoteLogs(site, deployment);
  return (
    <Dialog open={!!deployment} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("Build log · {id}", { id: deployment ?? "" })}</DialogTitle>
          <DialogDescription>{t("From the platform's API.")}</DialogDescription>
        </DialogHeader>
        {logs.isPending ? (
          <Skeleton className="h-64" />
        ) : logs.error ? (
          <p className="text-[12.5px] text-destructive">{errorMessage(logs.error)}</p>
        ) : logs.data?.link ? (
          <div className="flex flex-col gap-2 text-[13px]">
            <p className="text-muted-foreground">
              {t("This platform's API has no build logs. They're on its dashboard:")}
            </p>
            <Button variant="outline" className="self-start" onClick={() => openInBrowser(logs.data!.link!)}>
              <ExternalLink data-icon="inline-start" /> {t("Open the deploy's page")}
            </Button>
          </div>
        ) : (
          <pre className="max-h-96 overflow-auto rounded-md bg-well px-3 py-2 font-mono text-[11.5px] whitespace-pre-wrap">
            {(logs.data?.lines ?? []).join("\n") || t("Nothing yet.")}
          </pre>
        )}
      </DialogContent>
    </Dialog>
  );
}
