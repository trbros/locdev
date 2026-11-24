import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { backend, isDaemonDown, request } from "@/lib/daemon";
import {
  type Confirmation,
  DEV_TOOL_LABEL,
  type GitOp,
  type GithubAccount,
  type GithubAction,
  type GithubData,
  type GithubSection,
  type SecurityStatus,
  type DevServerMode,
  type JobAction,
  jobFinished,
  type HostExplain,
  type Settings,
  type Subdomains,
  type LogLine,
  type ProcessRole,
  type Request,
  RUNTIME_LABEL,
  type RuntimeKind,
  type SystemAction,
  type SystemTarget,
  type WorkflowModel,
} from "@/lib/protocol";
import { t } from "@/lib/i18n";

export const keys = {
  status: ["status"] as const,
  projects: ["projects"] as const,
  doctor: ["doctor"] as const,
  info: ["info"] as const,
};

// While the daemon is unreachable, React Query keeps the last successful
// data around; hide it so screens don't show stale numbers as if live.

export function useStatus() {
  const query = useQuery({
    queryKey: keys.status,
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "status" }, "status");
      return status;
    },
    refetchInterval: 3000,
    retry: false,
  });
  return { ...query, data: query.isError ? undefined : query.data };
}

/** `true` once we know the daemon is not reachable. */
export function useDaemonDown() {
  const status = useStatus();
  return status.isError && isDaemonDown(status.error);
}

export function useProjects() {
  const down = useDaemonDown();
  const query = useQuery({
    queryKey: keys.projects,
    queryFn: async () => (await request({ type: "list_projects" }, "projects")).projects,
    refetchInterval: down ? false : 2000,
    enabled: !down,
    retry: false,
  });
  return { ...query, data: down ? undefined : query.data };
}

export function useDoctor() {
  const down = useDaemonDown();
  const query = useQuery({
    queryKey: keys.doctor,
    queryFn: async () => (await request({ type: "doctor" }, "doctor")).checks,
    staleTime: 30_000,
    enabled: !down,
    retry: false,
  });
  return { ...query, data: down ? undefined : query.data };
}

export function useAppInfo() {
  return useQuery({ queryKey: keys.info, queryFn: backend.info, staleTime: Infinity });
}

function useInvalidate() {
  const client = useQueryClient();
  return () => client.invalidateQueries();
}

export const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));

export function useDaemonControl() {
  const invalidate = useInvalidate();

  const start = useMutation({
    mutationFn: backend.start,
    onSuccess: (started) => toast.success(started ? t("Daemon started") : t("Daemon already running")),
    onError: (error) => toast.error(t("Could not start the daemon"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });

  const stop = useMutation({
    mutationFn: backend.stop,
    onSuccess: () => toast(t("Daemon stopped")),
    onError: (error) => toast.error(t("Could not stop the daemon"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });

  const restart = useMutation({
    mutationFn: async () => {
      await backend.stop();
      return backend.start();
    },
    onSuccess: () => toast.success(t("Daemon restarted")),
    onError: (error) => toast.error(t("Could not restart the daemon"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });

  return { start, stop, restart, pending: start.isPending || stop.isPending || restart.isPending };
}

type ProjectAction = "start_project" | "stop_project" | "restart_project" | "remove_project";

const ACTION_LABEL: Record<ProjectAction, string> = {
  start_project: t("start"),
  stop_project: t("stop"),
  restart_project: t("restart"),
  remove_project: t("remove"),
};

export function useProjectAction() {
  const invalidate = useInvalidate();

  return useMutation({
    mutationFn: async ({ action, name }: { action: ProjectAction; name: string }) => {
      const response = await backend.request({ type: action, name });
      if (response.type === "error") throw new Error(response.message);
      return response;
    },
    onSuccess: (_, { action, name }) => {
      if (action === "remove_project") toast(`Removed ${name}`, { description: t("Its files were not touched.") });
    },
    onError: (error, { action, name }) =>
      toast.error(t("Could not {action} {name}", { action: ACTION_LABEL[action], name }), {
        description: errorMessage(error),
      }),
    onSettled: invalidate,
  });
}

export function useSaveProject() {
  const invalidate = useInvalidate();

  return useMutation({
    mutationFn: async (req: Extract<Request, { type: "add_project" | "update_project" }>) =>
      (await request(req, "project")).project,
    onSettled: invalidate,
  });
}

/**
 * Streams a process's output by polling with a sequence cursor. `paused`
 * stops polling without losing what was already read.
 */
export function useLogs(
  name: string | undefined,
  process: ProcessRole = "main",
  enabled = true,
  keep = 2000,
  aux: string | null = null,
) {
  const [lines, setLines] = useState<LogLine[]>([]);
  const cursor = useRef<number | undefined>(undefined);

  useEffect(() => {
    setLines([]);
    cursor.current = undefined;
    if (!name || !enabled) return;

    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;

    const poll = async () => {
      try {
        const { lines: fresh } = await request(
          { type: "project_logs", name, process, aux, after: cursor.current, limit: cursor.current ? 1000 : keep },
          "logs",
        );
        if (cancelled) return;
        if (fresh.length) {
          cursor.current = fresh[fresh.length - 1].seq;
          setLines((prev) => [...prev, ...fresh].slice(-keep));
        }
      } catch {
        // Daemon hiccups are shown elsewhere; keep polling.
      }
      if (!cancelled) timer = setTimeout(poll, 700);
    };
    poll();

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [name, process, enabled, keep, aux]);

  return lines;
}

/** Every supervised process, with CPU and memory (sampled by the daemon). */
export function useProcesses() {
  const down = useDaemonDown();
  return useQuery({
    queryKey: ["processes"],
    queryFn: async () => (await request({ type: "list_processes" }, "processes")).processes,
    refetchInterval: down ? false : 3000,
    enabled: !down,
    retry: false,
  });
}

type DevAction = "start_dev_server" | "stop_dev_server" | "restart_dev_server";

/** The helper dev server: decide how it runs, start, stop, restart. */
export function useDevServer() {
  const invalidate = useInvalidate();
  const decide = useMutation({
    mutationFn: async (req: { name: string; mode: DevServerMode; start?: boolean; command?: string }) =>
      (await request({ type: "set_dev_server", ...req }, "project")).project,
    onSuccess: (project, { mode, start }) =>
      toast.success(
        start
          ? t("Starting {name}", {
              name: project.stack.dev_server ? DEV_TOOL_LABEL[project.stack.dev_server.tool] : t("the dev server"),
            })
          : mode === "never"
            ? t("LocDev won't manage it")
            : mode === "auto"
              ? t("It will start with the site")
              : t("Saved"),
      ),
    onError: (error) => toast.error(t("Could not save the decision"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
  const act = useMutation({
    mutationFn: async ({ action, name }: { action: DevAction; name: string }) =>
      (await request({ type: action, name }, "project")).project,
    onError: (error) => toast.error(t("Dev server"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
  return { decide, act };
}

/** What HTTPS needs for one site, checked now. */
export function useSiteHttps(name: string | null) {
  return useQuery({
    queryKey: ["site-https", name],
    queryFn: async () => (await request({ type: "check_site_https", name: name! }, "doctor")).checks,
    enabled: name !== null,
    staleTime: 10_000,
    retry: false,
  });
}

export function useSetSecure() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async ({ name, secure }: { name: string; secure: boolean }) =>
      (await request({ type: "set_secure", name, secure }, "project")).project,
    onSuccess: (project) =>
      toast.success(
        project.secure
          ? t("{domain} is HTTPS-only", { domain: project.domain })
          : t("{domain} serves HTTP again", { domain: project.domain }),
      ),
    onError: (error) => toast.error(t("Could not change HTTPS"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
}

export function useSetSubdomains() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async ({ name, subdomains }: { name: string; subdomains: Subdomains }) =>
      (await request({ type: "set_subdomains", name, subdomains }, "project")).project,
    onError: (error) => toast.error(t("Could not change subdomains"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
}

/** How one hostname resolves, routes and is certified. */
export async function explainHost(host: string): Promise<HostExplain> {
  const { type: _, ...explain } = await request({ type: "explain_host", host }, "host_explain");
  return explain;
}

/** Re-probes how the OS resolves *.test and refreshes everything that shows it. */
export function useCheckDns() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const { type: _, ...dns } = await request({ type: "check_dns" }, "dns");
      return dns;
    },
    onSettled: () => client.invalidateQueries(),
  });
}

/** Re-reads whether the OS and browsers trust the local CA. */
export function useCheckTls() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const { type: _, ...tls } = await request({ type: "check_tls" }, "tls");
      return tls;
    },
    onSettled: () => client.invalidateQueries(),
  });
}

export interface SystemChange {
  target: SystemTarget;
  action: SystemAction;
}

export function useSystemPlan(change: SystemChange | null) {
  return useQuery({
    queryKey: ["system-plan", change?.target, change?.action],
    queryFn: () => backend.systemPlan(change!.target, change!.action),
    enabled: change !== null,
    retry: false,
    staleTime: 0,
  });
}

/** Applies a privileged change, then re-checks so every view updates. */
export function useSystemApply() {
  const checkDns = useCheckDns();
  const checkTls = useCheckTls();

  return useMutation({
    mutationFn: async ({ target, action }: SystemChange) => {
      await backend.systemApply(target, action);
      if (target === "dns") {
        const dns = await checkDns.mutateAsync();
        return action === "uninstall"
          ? { ok: true, message: "*.test is no longer routed to LocDev" }
          : dns.routing === "locdev"
            ? { ok: true, message: "*.test now resolves through LocDev" }
            : { ok: false, message: "Installed, but *.test doesn't reach LocDev yet" };
      }
      const tls = await checkTls.mutateAsync();
      return action === "uninstall"
        ? { ok: true, message: t("The LocDev CA is no longer trusted") }
        : tls.trusted
          ? { ok: true, message: t("The LocDev CA is trusted; sites now open over HTTPS") }
          : { ok: false, message: "Installed, but the system doesn't report the CA as trusted yet" };
    },
    onSuccess: ({ ok, message }) =>
      ok ? toast.success(message) : toast.warning(message, { description: t("Open Doctor to see why.") }),
    onError: (error) => toast.error(t("Change not applied"), { description: errorMessage(error) }),
  });
}

export function useService() {
  const client = useQueryClient();
  const query = useQuery({ queryKey: ["service"], queryFn: backend.serviceStatus, retry: false });
  const set = useMutation({
    mutationFn: backend.serviceSet,
    onSuccess: (status) => {
      client.setQueryData(["service"], status);
      toast.success(status.installed ? "LocDev will start at login" : "LocDev won't start at login");
    },
    onError: (error) => toast.error(t("Could not change the login item"), { description: errorMessage(error) }),
  });
  return { query, set };
}

/** The `locdev` command bundled with the app, and its PATH link. */
export function useCli() {
  const client = useQueryClient();
  const query = useQuery({ queryKey: ["cli"], queryFn: backend.cliStatus, retry: false });
  const link = useMutation({
    mutationFn: backend.cliLink,
    onSuccess: (status) => {
      client.setQueryData(["cli"], status);
      toast.success(status.link ? t("locdev is ready in new terminals") : t("Removed the locdev command"));
    },
    onError: (error) => toast.error(t("Could not change the locdev command"), { description: errorMessage(error) }),
  });
  return { query, link };
}

export function useRuntimes() {
  const query = useQuery({
    queryKey: ["runtimes"],
    queryFn: async () => (await request({ type: "list_runtimes" }, "runtimes")).runtimes,
    // Poll fast while an install runs, so the progress bar moves.
    refetchInterval: (q) =>
      q.state.data?.some((r) => r.job && !["done", "failed"].includes(r.job.state)) ? 400 : 5000,
    retry: false,
  });
  return query;
}

export function useCatalog(kind: RuntimeKind | null) {
  return useQuery({
    queryKey: ["catalog", kind],
    queryFn: async () => (await request({ type: "runtime_catalog", kind: kind! }, "catalog")).releases,
    enabled: kind !== null,
    staleTime: 60 * 60 * 1000,
    retry: false,
  });
}

/** The default versions in new terminals, and the switch for them. */
export function useTerminalRuntimes() {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["terminal-runtimes"],
    queryFn: async () => {
      const { type: _, ...state } = await request({ type: "terminal_runtimes" }, "terminal_runtimes");
      return state;
    },
    retry: false,
  });
  const toggle = useMutation({
    mutationFn: async (enable: boolean) => {
      const { type: _, ...state } = await request({ type: "terminal_runtimes", enable }, "terminal_runtimes");
      return state;
    },
    onSuccess: (state) => {
      client.setQueryData(["terminal-runtimes"], state);
      toast.success(
        state.enabled
          ? t("New terminals use the default versions")
          : t("New terminals no longer use LocDev's versions"),
      );
    },
    onError: (error) => toast.error(t("Could not change the terminal setup"), { description: errorMessage(error) }),
  });
  return { query, toggle };
}

/** Install, uninstall, set default and re-scan, with toasts. */
export function useRuntimeActions() {
  const client = useQueryClient();
  const settle = () => client.invalidateQueries();
  const fail = (what: string) => (error: unknown) => toast.error(what, { description: errorMessage(error) });

  const install = useMutation({
    mutationFn: async ({ kind, version }: { kind: RuntimeKind; version: string }) =>
      (await request({ type: "install_runtime", kind, version }, "runtimes")).runtimes,
    onSuccess: (runtimes) => client.setQueryData(["runtimes"], runtimes),
    onError: fail(t("Could not start the install")),
  });
  const uninstall = useMutation({
    mutationFn: async ({ kind, version, force }: { kind: RuntimeKind; version: string; force?: boolean }) =>
      (await request({ type: "uninstall_runtime", kind, version, force }, "runtimes")).runtimes,
    onSuccess: (_, { kind, version }) => toast(`Removed ${RUNTIME_LABEL[kind]} ${version}`),
    onError: fail(t("Could not remove the version")),
    onSettled: settle,
  });
  const setDefault = useMutation({
    mutationFn: async ({ kind, version }: { kind: RuntimeKind; version: string | null }) =>
      (await request({ type: "set_default_runtime", kind, version }, "runtimes")).runtimes,
    onSuccess: (_, { kind, version }) =>
      toast.success(
        version
          ? t("Default {kind} is now {version}", { kind: RUNTIME_LABEL[kind], version })
          : t("No default {kind}", { kind: RUNTIME_LABEL[kind] }),
      ),
    onError: fail(t("Could not change the default")),
    onSettled: settle,
  });
  const rescan = useMutation({
    mutationFn: async () => (await request({ type: "list_runtimes", refresh: true }, "runtimes")).runtimes,
    onSuccess: (runtimes) => client.setQueryData(["runtimes"], runtimes),
    onError: fail(t("Could not scan for runtimes")),
  });

  return { install, uninstall, setDefault, rescan };
}

// ── Dependencies, jobs, notifications, settings ──────────────────────────────

/** A site's dependencies from disk plus the last registry check (no network). */
export function useDeps(name: string | null) {
  return useQuery({
    queryKey: ["deps", name],
    queryFn: async () => {
      const { type: _, ...report } = await request({ type: "list_deps", name: name! }, "deps");
      return report;
    },
    enabled: name !== null,
    refetchInterval: 15_000,
    retry: false,
  });
}

export function useJobs(project?: string | null) {
  const down = useDaemonDown();
  return useQuery({
    queryKey: ["jobs", project ?? null],
    queryFn: async () => (await request({ type: "list_jobs", project: project ?? null }, "jobs")).jobs,
    // Faster while something is moving.
    refetchInterval: (q) => (down ? false : q.state.data?.some((j) => !jobFinished(j.status)) ? 800 : 5000),
    enabled: !down,
    retry: false,
  });
}

export function useJob(id: number | null) {
  return useQuery({
    queryKey: ["job", id],
    queryFn: async () => (await request({ type: "get_job", id: id! }, "job")).job,
    enabled: id !== null,
    refetchInterval: (q) => (q.state.data && jobFinished(q.state.data.status) ? false : 600),
    retry: false,
  });
}

export function useJobControl() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async ({ id, action }: { id: number; action: JobAction }) =>
      (await request({ type: "control_job", id, action }, "job")).job,
    onError: (error) => toast.error(t("Could not change the job"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
}

export function useCheckDeps() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async (name: string) => (await request({ type: "check_deps", name }, "job")).job,
    onError: (error) => toast.error(t("Could not start the check"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
}

export interface UpdateChoice {
  name: string;
  packages: string[];
  allow_major: boolean;
  run_scripts: boolean;
}

export function usePlanUpdate(choice: UpdateChoice | null) {
  return useQuery({
    queryKey: ["deps-plan", choice],
    queryFn: async () => {
      const { type: _, ...plan } = await request({ type: "plan_deps_update", ...choice! }, "deps_plan");
      return plan;
    },
    enabled: choice !== null && choice.packages.length > 0,
    staleTime: 0,
    retry: false,
  });
}

export function useUpdateDeps() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async (choice: UpdateChoice) => (await request({ type: "update_deps", ...choice }, "job")).job,
    onError: (error) => toast.error(t("Could not start the update"), { description: errorMessage(error) }),
    onSettled: invalidate,
  });
}

export function useNotifications(all = false) {
  const down = useDaemonDown();
  return useQuery({
    queryKey: ["notifications", all],
    queryFn: async () => {
      const { notifications, unread } = await request({ type: "list_notifications", all }, "notifications");
      return { notifications, unread };
    },
    refetchInterval: down ? false : 5000,
    enabled: !down,
    retry: false,
  });
}

export function useMarkNotifications() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (req: { ids?: number[]; read?: boolean; dismissed?: boolean }) =>
      request({ type: "mark_notifications", ...req }, "ok"),
    onSettled: () => client.invalidateQueries({ queryKey: ["notifications"] }),
  });
}

export function useTestNotification() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => request({ type: "test_notification" }, "ok"),
    onSettled: () => client.invalidateQueries({ queryKey: ["notifications"] }),
  });
}

export function useSettings() {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["settings"],
    queryFn: async () => {
      const { type: _, ...settings } = await request({ type: "get_settings" }, "settings");
      return settings;
    },
    retry: false,
  });
  const update = useMutation({
    mutationFn: async (settings: Settings) => {
      const { type: _, ...saved } = await request({ type: "update_settings", settings }, "settings");
      return saved;
    },
    onSuccess: (saved) => client.setQueryData(["settings"], saved),
    onError: (error) => toast.error(t("Could not save the setting"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["jobs"] }),
  });
  return { query, update };
}

// ── Git, security, GitHub ────────────────────────────────────────────────────

const gitKey = (name: string, ...rest: unknown[]) => ["git", name, ...rest];

export function useGitStatus(name: string | null) {
  return useQuery({
    queryKey: gitKey(name ?? "", "status"),
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "git_status", name: name! }, "git_status");
      return status;
    },
    enabled: name !== null,
    refetchInterval: 4000,
    retry: false,
  });
}

export function useGitDiff(name: string, path: string | null, staged: boolean) {
  return useQuery({
    queryKey: gitKey(name, "diff", path, staged),
    queryFn: async () => {
      const { text, truncated } = await request({ type: "git_diff", name, path: path!, staged }, "text");
      return { text, truncated };
    },
    enabled: path !== null,
    retry: false,
  });
}

export function useGitLog(name: string, limit: number) {
  return useQuery({
    queryKey: gitKey(name, "log", limit),
    queryFn: async () => (await request({ type: "git_log", name, limit }, "commits")).commits,
    placeholderData: (previous) => previous,
    retry: false,
  });
}

export function useGitShow(name: string, sha: string | null) {
  return useQuery({
    queryKey: gitKey(name, "show", sha),
    queryFn: async () => {
      const { type: _, ...detail } = await request({ type: "git_show", name, sha: sha! }, "commit");
      return detail;
    },
    enabled: sha !== null,
    staleTime: Infinity,
    retry: false,
  });
}

export function useGitBranches(name: string) {
  return useQuery({
    queryKey: gitKey(name, "branches"),
    queryFn: async () => (await request({ type: "git_branches", name }, "branches")).branches,
    retry: false,
  });
}

export function useGitStashes(name: string) {
  return useQuery({
    queryKey: gitKey(name, "stashes"),
    queryFn: async () => (await request({ type: "git_stashes", name }, "stashes")).stashes,
    retry: false,
  });
}

export function useGitRemotes(name: string) {
  return useQuery({
    queryKey: gitKey(name, "remotes"),
    queryFn: async () => (await request({ type: "git_remotes", name }, "remotes")).remotes,
    retry: false,
  });
}

export function useGitHistory(name: string) {
  return useQuery({
    queryKey: gitKey(name, "history"),
    queryFn: async () => {
      const { reflog, operations } = await request({ type: "git_history", name }, "git_history");
      return { reflog, operations };
    },
    retry: false,
  });
}

/** What a Git operation would lose; `null` when it isn't destructive. */
export function useGitPreview(name: string, op: GitOp | null) {
  return useQuery({
    queryKey: gitKey(name, "preview", op),
    queryFn: async () => (await request({ type: "git_preview", name, op: op! }, "git_preview")).preview,
    enabled: op !== null,
    staleTime: 0,
    gcTime: 0,
    retry: false,
  });
}

export function useGit(name: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async ({ op, confirm }: { op: GitOp; confirm?: Confirmation }) => {
      const { type: _, ...result } = await request({ type: "git", name, op, confirm: confirm ?? null }, "git_result");
      return result;
    },
    onSettled: () => client.invalidateQueries({ queryKey: ["git", name] }),
  });
}

export function useSecurity() {
  const client = useQueryClient();
  const query = useQuery({
    queryKey: ["security"],
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "get_security" }, "security");
      return status;
    },
    retry: false,
  });
  const save = (status: SecurityStatus) => client.setQueryData(["security"], status);
  const setPassword = useMutation({
    mutationFn: async (req: { current?: string | null; new?: string | null }) => {
      const { type: _, ...status } = await request({ type: "set_password", ...req }, "security");
      return status;
    },
    onSuccess: save,
  });
  const setOptions = useMutation({
    mutationFn: async (req: { password?: string | null; require_password: boolean; backups: boolean }) => {
      const { type: _, ...status } = await request({ type: "set_security_options", ...req }, "security");
      return status;
    },
    onSuccess: save,
  });
  return { query, setPassword, setOptions };
}

export function useGithubAccount() {
  const down = useDaemonDown();
  return useQuery({
    queryKey: ["github-account"],
    queryFn: async () => (await request({ type: "github_account" }, "github")).account,
    enabled: !down,
    staleTime: 60_000,
    retry: false,
  });
}

export function useGithubConnect() {
  const client = useQueryClient();
  const done = (account: GithubAccount | null) => {
    client.setQueryData(["github-account"], account);
    client.invalidateQueries({ queryKey: ["github"] });
  };
  const token = useMutation({
    mutationFn: async (token: string) => (await request({ type: "github_connect_token", token }, "github")).account,
    onSuccess: done,
  });
  const ghCli = useMutation({
    mutationFn: async () => (await request({ type: "github_connect_gh_cli" }, "github")).account,
    onSuccess: done,
  });
  const deviceStart = useMutation({
    mutationFn: async () => {
      const { type: _, ...code } = await request({ type: "github_device_start" }, "device_code");
      return code;
    },
  });
  const devicePoll = useMutation({
    mutationFn: async () => (await request({ type: "github_device_poll" }, "github")).account,
    onSuccess: (account) => {
      if (account) done(account);
    },
  });
  const disconnect = useMutation({
    mutationFn: async () => (await request({ type: "github_disconnect" }, "github")).account,
    onSuccess: done,
    onError: (error) => toast.error(t("Could not disconnect"), { description: errorMessage(error) }),
  });
  return { token, ghCli, deviceStart, devicePoll, disconnect };
}

export function useGithub<S extends GithubSection>(name: string, section: S, enabled = true) {
  return useQuery({
    queryKey: ["github", name, section],
    queryFn: async () => {
      const { type: _, ...data } = await request({ type: "github", name, section }, "github_data");
      return data as Extract<GithubData, { section: S }>;
    },
    enabled,
    staleTime: 30_000,
    refetchInterval: section === "runs" ? 20_000 : false,
    retry: false,
  });
}

export function useGithubAction(name: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (action: GithubAction) => {
      const { type: _, ...result } = await request({ type: "github_action", name, action }, "git_result");
      return result;
    },
    onSuccess: (result) => toast.success(result.message),
    onError: (error) => toast.error(t("GitHub refused the change"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["github", name] }),
  });
}

// ── GitHub Actions builder ───────────────────────────────────────────────────

export function useActionsOverview(name: string | null) {
  return useQuery({
    queryKey: ["actions", name],
    queryFn: async () => {
      const { type: _, ...overview } = await request({ type: "actions_overview", name: name! }, "actions_overview");
      return overview;
    },
    enabled: name !== null,
    retry: false,
  });
}

/** The YAML for a visual-editor workflow, with its checks. */
export function useActionsRender(workflow: WorkflowModel | null) {
  return useQuery({
    queryKey: ["actions-render", workflow],
    queryFn: async () => {
      const { yaml, validation } = await request({ type: "actions_render", workflow: workflow! }, "actions_yaml");
      return { yaml, validation };
    },
    enabled: workflow !== null,
    placeholderData: (previous) => previous,
    retry: false,
  });
}

export function useActionsValidate(yaml: string | null) {
  return useQuery({
    queryKey: ["actions-validate", yaml],
    queryFn: async () => {
      const { type: _, ...validation } = await request({ type: "actions_validate", yaml: yaml! }, "actions_validation");
      return validation;
    },
    enabled: yaml !== null,
    placeholderData: (previous) => previous,
    retry: false,
  });
}

export function useRefreshActionVersions() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => request({ type: "refresh_action_versions" }, "action_versions"),
    onSuccess: () => {
      toast.success(t("Action versions checked against GitHub"));
      client.invalidateQueries({ queryKey: ["actions"] });
    },
    onError: (error) => toast.error(t("Could not check the versions"), { description: errorMessage(error) }),
  });
}

/** `value`, once it has stopped changing for `ms`. */
export function useDebounced<T>(value: T, ms = 300): T {
  const [settled, setSettled] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setSettled(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return settled;
}
