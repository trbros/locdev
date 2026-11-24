import { ExternalLink, Play, RotateCw, Square } from "lucide-react";
import { useState } from "react";

import { Fact, Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { useGithub, useGithubAccount, useGithubAction } from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import type { GithubAction, GithubItem, GithubRepo, GithubSection, ProjectView, WorkflowRun } from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

type View = Extract<GithubSection, "overview" | "issues" | "pulls" | "runs" | "workflows" | "releases">;

const since = (iso: string) => uptime(Date.parse(iso)).split(" ")[0];
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function runTone(run: Pick<WorkflowRun, "status" | "conclusion">): Tone {
  if (run.status !== "completed") return "info";
  if (run.conclusion === "success") return "live";
  if (run.conclusion === "failure" || run.conclusion === "timed_out") return "fail";
  return "off";
}

export function GithubTab({ site, repo }: { site: ProjectView; repo: GithubRepo }) {
  const account = useGithubAccount();
  const { navigate } = useUi();
  const [view, setView] = useState<View>("overview");

  if (account.isPending) return <Skeleton className="h-32" />;
  if (!account.data) {
    return (
      <div className="flex flex-col items-start gap-3 text-sm text-muted-foreground">
        <p>
          {tx(
            "This site's remote is {repo}. Connect a GitHub account to see its issues, pull requests and workflow runs here.",
            {
              repo: (
                <span className="font-mono text-foreground">
                  {repo.owner}/{repo.name}
                </span>
              ),
            },
          )}
        </p>
        <Button size="sm" variant="outline" onClick={() => navigate("settings", "integrations")}>
          {t("Connect GitHub…")}
        </Button>
      </div>
    );
  }

  return (
    <>
      <div className="flex flex-wrap items-center gap-3">
        <div role="radiogroup" aria-label={t("GitHub section")} className="flex rounded-md border p-0.5 text-[12.5px]">
          {(
            [
              ["overview", t("Overview")],
              ["issues", t("Issues")],
              ["pulls", t("Pull requests")],
              ["runs", t("Runs")],
              ["workflows", t("Workflows")],
              ["releases", t("Releases")],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              role="radio"
              aria-checked={view === id}
              onClick={() => setView(id)}
              className={cn(
                "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
                view === id && "bg-foreground/[0.07] text-foreground",
              )}
            >
              {label}
            </button>
          ))}
        </div>
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto"
          onClick={() => openInBrowser(`https://github.com/${repo.owner}/${repo.name}`)}
        >
          <ExternalLink data-icon="inline-start" /> {repo.owner}/{repo.name}
        </Button>
      </div>
      {view === "overview" ? (
        <Overview name={site.name} />
      ) : view === "issues" || view === "pulls" ? (
        <Items name={site.name} section={view} />
      ) : view === "runs" ? (
        <Runs name={site.name} login={account.data.login} />
      ) : view === "workflows" ? (
        <Workflows name={site.name} login={account.data.login} />
      ) : (
        <Releases name={site.name} />
      )}
    </>
  );
}

function Failed({ error }: { error: unknown }) {
  return <p className="text-sm text-destructive">{message(error)}</p>;
}

function Overview({ name }: { name: string }) {
  const overview = useGithub(name, "overview");
  const checks = useGithub(name, "checks");
  if (overview.isPending) return <Skeleton className="h-32" />;
  if (overview.isError) return <Failed error={overview.error} />;
  const o = overview.data;
  return (
    <div className="flex max-w-3xl flex-col gap-6">
      <Section title={t("Repository")}>
        {o.description && <p className="pb-2 text-sm text-muted-foreground">{o.description}</p>}
        <Fact label={t("Visibility")}>{o.private ? t("private") : t("public")}</Fact>
        <Fact label={t("Default branch")} mono>
          {o.default_branch}
        </Fact>
        <Fact label={t("Open issues and PRs")}>{o.open_issues}</Fact>
        <Fact label={t("Stars")}>{o.stars}</Fact>
        {o.rate_remaining != null && <Fact label={t("API requests left this hour")}>{o.rate_remaining}</Fact>}
      </Section>
      <Section title={t("Checks on the current commit")}>
        {checks.isPending ? (
          <Skeleton className="h-16" />
        ) : checks.isError ? (
          <Failed error={checks.error} />
        ) : checks.data.checks.length === 0 ? (
          <p className="px-1 text-[12.5px] text-muted-foreground">
            {tx("No checks for {sha} (not pushed yet, or no workflows ran on it).", {
              sha: <span className="font-mono">{checks.data.sha.slice(0, 7)}</span>,
            })}
          </p>
        ) : (
          checks.data.checks.map((c) => (
            <button
              key={c.name}
              type="button"
              onClick={() => openInBrowser(c.url)}
              className="flex h-8 items-center gap-3 px-1 text-left text-[13px] hover:text-foreground"
            >
              <Led tone={runTone(c)} live={c.status !== "completed"} />
              <span className="flex-1">{c.name}</span>
              <span className="font-mono text-[11.5px] text-muted-foreground">{c.conclusion ?? c.status}</span>
            </button>
          ))
        )}
      </Section>
    </div>
  );
}

function Items({ name, section }: { name: string; section: "issues" | "pulls" }) {
  const items = useGithub(name, section);
  if (items.isPending) return <Skeleton className="h-32" />;
  if (items.isError) return <Failed error={items.error} />;
  const list: GithubItem[] = items.data.items;
  if (list.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("No open")} {section === "pulls" ? t("pull requests") : t("issues")}.
      </p>
    );
  }
  return (
    <div className="flex max-w-4xl flex-col">
      {list.map((item) => (
        <button
          key={item.number}
          type="button"
          onClick={() => openInBrowser(item.url)}
          className="flex flex-col gap-0.5 rounded px-2 py-2 text-left hover:bg-foreground/4"
        >
          <span className="flex items-center gap-2 text-[13.5px]">
            <span className="font-mono text-[12px] text-muted-foreground">#{item.number}</span>
            <span className="truncate">{item.title}</span>
            {item.draft && <span className="rounded bg-foreground/6 px-1.5 font-mono text-[10.5px]">{t("draft")}</span>}
            {item.labels.map((l) => (
              <span key={l} className="rounded bg-ember/10 px-1.5 font-mono text-[10.5px] text-ember">
                {l}
              </span>
            ))}
          </span>
          <span className="font-mono text-[11px] text-muted-foreground">
            {tx("{author} · updated {since} ago{branch}", {
              author: item.author,
              since: since(item.updated_at),
              branch: item.branch && ` · ${item.branch}`,
            })}
          </span>
        </button>
      ))}
    </div>
  );
}

/** Every change on GitHub goes through this: what happens, as whom, then a click. */
function ConfirmRemote({
  name,
  login,
  title,
  text,
  action,
  onClose,
}: {
  name: string;
  login: string;
  title: string;
  text: string;
  action: GithubAction;
  onClose: () => void;
}) {
  const run = useGithubAction(name);
  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>
            {text} {t("This happens on GitHub as")} <span className="font-mono">{login}</span>.
          </DialogDescription>
        </DialogHeader>
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button disabled={run.isPending} onClick={() => run.mutate(action, { onSuccess: onClose })}>
            {t("Confirm")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function Runs({ name, login }: { name: string; login: string }) {
  const runs = useGithub(name, "runs");
  const [confirm, setConfirm] = useState<{ title: string; text: string; action: GithubAction } | null>(null);
  if (runs.isPending) return <Skeleton className="h-32" />;
  if (runs.isError) return <Failed error={runs.error} />;
  return (
    <div className="flex max-w-5xl flex-col">
      {runs.data.runs.length === 0 && <p className="text-sm text-muted-foreground">{t("No workflow runs yet.")}</p>}
      {runs.data.runs.map((run) => (
        <div key={run.id} className="group flex h-11 items-center gap-3 px-1 text-[13px]">
          <Led tone={runTone(run)} live={run.status !== "completed"} />
          <button type="button" className="min-w-0 flex-1 truncate text-left" onClick={() => openInBrowser(run.url)}>
            <span className="font-medium">{run.name}</span>{" "}
            <span className="font-mono text-[11.5px] text-muted-foreground">
              {run.branch} · {run.event} · {run.sha.slice(0, 7)} · {since(run.created_at)} {t("ago")}
            </span>
          </button>
          <span className="w-24 text-right font-mono text-[11.5px] text-muted-foreground">
            {run.conclusion ?? run.status.replace("_", " ")}
          </span>
          <span className="flex w-28 justify-end gap-1">
            {run.status !== "completed" ? (
              <Button
                size="xs"
                variant="ghost"
                onClick={() =>
                  setConfirm({
                    title: t("Cancel {name}?", { name: run.name }),
                    text: "GitHub stops the jobs still running in this run.",
                    action: { action: "cancel_run", run_id: run.id },
                  })
                }
              >
                <Square data-icon="inline-start" /> {t("Cancel")}
              </Button>
            ) : (
              run.conclusion !== "success" && (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() =>
                    setConfirm({
                      title: t("Re-run failed jobs of {name}?", { name: run.name }),
                      text: "GitHub runs the failed jobs again, on the same commit.",
                      action: { action: "rerun_failed", run_id: run.id },
                    })
                  }
                >
                  <RotateCw data-icon="inline-start" /> {t("Re-run failed")}
                </Button>
              )
            )}
          </span>
        </div>
      ))}
      {confirm && <ConfirmRemote name={name} login={login} {...confirm} onClose={() => setConfirm(null)} />}
    </div>
  );
}

function Workflows({ name, login }: { name: string; login: string }) {
  const workflows = useGithub(name, "workflows");
  const overview = useGithub(name, "overview");
  const [dispatch, setDispatch] = useState<{ id: number; name: string; ref: string } | null>(null);
  const [confirm, setConfirm] = useState<{ title: string; text: string; action: GithubAction } | null>(null);
  if (workflows.isPending) return <Skeleton className="h-32" />;
  if (workflows.isError) return <Failed error={workflows.error} />;
  return (
    <div className="flex max-w-4xl flex-col">
      {workflows.data.workflows.map((w) => (
        <div key={w.id} className="flex min-h-10 flex-wrap items-center gap-3 px-1 text-[13px]">
          <Led tone={w.state === "active" ? "live" : "off"} />
          <button type="button" className="text-left font-medium" onClick={() => openInBrowser(w.url)}>
            {w.name}
          </button>
          <span className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-muted-foreground">{w.path}</span>
          {dispatch?.id === w.id ? (
            <form
              className="flex items-center gap-2"
              onSubmit={(e) => {
                e.preventDefault();
                setConfirm({
                  title: t("Run {name} on {ref}?", { name: w.name, ref: dispatch.ref }),
                  text: t("Starts the workflow by hand (it needs a workflow_dispatch trigger)."),
                  action: { action: "dispatch", workflow_id: w.id, git_ref: dispatch.ref.trim() },
                });
                setDispatch(null);
              }}
            >
              <Input
                autoFocus
                value={dispatch.ref}
                onChange={(e) => setDispatch({ ...dispatch, ref: e.target.value })}
                aria-label={t("Branch or tag")}
                className="h-7 w-36 font-mono text-[12px]"
              />
              <Button size="xs" type="submit" disabled={!dispatch.ref.trim()}>
                {t("Run…")}
              </Button>
            </form>
          ) : (
            <Button
              size="xs"
              variant="ghost"
              onClick={() => setDispatch({ id: w.id, name: w.name, ref: overview.data?.default_branch ?? "main" })}
            >
              <Play data-icon="inline-start" /> {t("Run workflow")}
            </Button>
          )}
        </div>
      ))}
      {confirm && <ConfirmRemote name={name} login={login} {...confirm} onClose={() => setConfirm(null)} />}
    </div>
  );
}

function Releases({ name }: { name: string }) {
  const releases = useGithub(name, "releases");
  if (releases.isPending) return <Skeleton className="h-32" />;
  if (releases.isError) return <Failed error={releases.error} />;
  if (releases.data.releases.length === 0) return <p className="text-sm text-muted-foreground">{t("No releases.")}</p>;
  return (
    <div className="flex max-w-3xl flex-col">
      {releases.data.releases.map((r) => (
        <button
          key={r.tag}
          type="button"
          onClick={() => openInBrowser(r.url)}
          className="flex h-9 items-center gap-3 px-1 text-left text-[13px] hover:text-foreground"
        >
          <span className="font-mono font-medium">{r.tag}</span>
          <span className="flex-1 truncate text-muted-foreground">{r.name}</span>
          {(r.draft || r.prerelease) && (
            <span className="rounded bg-foreground/6 px-1.5 font-mono text-[10.5px]">
              {r.draft ? t("draft") : t("pre-release")}
            </span>
          )}
          <span className="font-mono text-[11px] text-muted-foreground">
            {r.published_at ? t("{time} ago", { time: since(r.published_at) }) : ""}
          </span>
        </button>
      ))}
    </div>
  );
}
