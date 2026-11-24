// Deploy platforms through their APIs. The daemon keeps the token, checks
// the local repository before deploying and makes every call; this only
// asks and shows.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useRemotePlatforms() {
  return useQuery({
    queryKey: ["remote", "platforms"],
    queryFn: async () => (await request({ type: "deploy_platforms" }, "deploy_platforms")).platforms,
    retry: false,
  });
}

export function useRemoteConnect() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { platform: string; token: string; team?: string | null }) =>
      (await request({ type: "deploy_connect", ...input }, "deploy_platforms")).platforms,
    onSuccess: (platforms) => client.setQueryData(["remote", "platforms"], platforms),
  });
}

export function useRemoteDisconnect() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (platform: string) =>
      (await request({ type: "deploy_disconnect", platform }, "deploy_platforms")).platforms,
    onSuccess: (platforms) => client.setQueryData(["remote", "platforms"], platforms),
    onError: (error) => toast.error(t("Could not disconnect"), { description: errorMessage(error) }),
  });
}

export function useRemoteProjects(platform: string | null) {
  return useQuery({
    queryKey: ["remote", "projects", platform],
    queryFn: async () =>
      (await request({ type: "deploy_remote_projects", platform: platform! }, "remote_projects")).projects,
    enabled: !!platform,
    retry: false,
  });
}

export function useRemoteLink() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { site: string; platform: string | null; project: string | null }) =>
      request({ type: "deploy_link_site", ...input }, "project"),
    onError: (error) => toast.error(t("Could not link the site"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries(),
  });
}

export function useRemoteStatus(site: string, linked: boolean) {
  return useQuery({
    queryKey: ["remote", "status", site],
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "deploy_remote_status", site }, "remote_status");
      return status;
    },
    enabled: linked,
    // Faster while something is building.
    refetchInterval: (q) =>
      q.state.data?.deployments.some((d) => d.state === "queued" || d.state === "building") ? 4000 : 30_000,
    retry: false,
  });
}

export function useRemoteAction() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (
      input:
        | { kind: "run"; site: string; production: boolean }
        | { kind: "cancel" | "rollback"; site: string; deployment: string },
    ) => {
      if (input.kind === "run")
        return request({ type: "deploy_run", site: input.site, production: input.production }, "remote_deployment");
      if (input.kind === "cancel")
        return request({ type: "deploy_cancel", site: input.site, deployment: input.deployment }, "ok");
      return request({ type: "deploy_rollback", site: input.site, deployment: input.deployment }, "ok");
    },
    onSuccess: (_, input) =>
      toast.success(
        input.kind === "run"
          ? t("Deploy started")
          : input.kind === "cancel"
            ? t("Deployment cancelled")
            : t("Production rolled back"),
      ),
    onError: (error) => toast.error(t("The platform refused it"), { description: errorMessage(error) }),
    onSettled: (_, __, input) => client.invalidateQueries({ queryKey: ["remote", "status", input.site] }),
  });
}

export function useRemoteLogs(site: string, deployment: string | null) {
  return useQuery({
    queryKey: ["remote", "logs", site, deployment],
    queryFn: async () => {
      const { type: _, ...log } = await request({ type: "deploy_logs", site, deployment: deployment! }, "remote_log");
      return log;
    },
    enabled: !!deployment,
    retry: false,
  });
}
