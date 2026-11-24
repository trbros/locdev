// Shared environment variables: queries and mutations behind Settings →
// Environment and a site's Environment tab. Every rule runs in the daemon.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { ProjectEnvSettings } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

const overviewKey = ["env"] as const;
const projectKey = (name: string) => ["env", "project", name] as const;

export function useEnvOverview() {
  return useQuery({
    queryKey: overviewKey,
    queryFn: async () => {
      const { type: _, ...overview } = await request({ type: "env_overview" }, "env_overview");
      return overview;
    },
    retry: false,
  });
}

export function useEnvProject(name: string | null) {
  return useQuery({
    queryKey: projectKey(name ?? ""),
    queryFn: async () => {
      const { type: _, ...report } = await request({ type: "env_project", name: name! }, "env_project");
      return report;
    },
    enabled: name !== null,
    refetchInterval: 5000,
    retry: false,
  });
}

/** Invalidates everything that depends on the shared variables. */
function useRefresh() {
  const client = useQueryClient();
  return () => client.invalidateQueries({ queryKey: ["env"] });
}

export interface VarInput {
  key: string;
  profile: string;
  value?: string | null;
  secret?: boolean | null;
  description?: string | null;
}

export function useEnvMutations() {
  const refresh = useRefresh();
  const fail = (title: string) => (error: unknown) => toast.error(title, { description: errorMessage(error) });

  const save = useMutation({
    mutationFn: (input: VarInput) => request({ type: "env_set", ...input }, "env_var"),
    onSuccess: ({ var: v }) =>
      toast.success(`Saved ${v.key}`, { description: t("Running sites get it after a restart.") }),
    onSettled: refresh,
  });
  const remove = useMutation({
    mutationFn: (input: { key: string; profile: string }) => request({ type: "env_remove", ...input }, "ok"),
    onSuccess: (_, { key }) => toast.success(`Removed ${key}`),
    onError: fail(t("Could not remove the variable")),
    onSettled: refresh,
  });
  const saveProfile = useMutation({
    mutationFn: (input: { id: string; label: string; description: string }) =>
      request({ type: "env_save_profile", ...input }, "env_profile"),
    onSettled: refresh,
  });
  const removeProfile = useMutation({
    mutationFn: (id: string) => request({ type: "env_remove_profile", id }, "ok"),
    onError: fail(t("Could not remove the profile")),
    onSettled: refresh,
  });
  const setProject = useMutation({
    mutationFn: (input: { name: string; settings: ProjectEnvSettings }) =>
      request({ type: "env_set_project", ...input }, "env_project"),
    onError: fail(t("Could not change the site's variables")),
    onSettled: refresh,
  });
  return { save, remove, saveProfile, removeProfile, setProject };
}

/** A secret's value, fetched only when the user asks for it. */
export async function revealValue(key: string, profile: string, password?: string | null) {
  return (await request({ type: "env_reveal", key, profile, password }, "env_value")).value;
}

export function useSyncPlans(name: string | null, file: string, enabled: boolean) {
  return useQuery({
    queryKey: ["env", "sync", name, file],
    queryFn: async () => (await request({ type: "env_sync_plan", name, file }, "env_sync_plans")).plans,
    enabled,
    retry: false,
  });
}

export function useEnvSync() {
  const refresh = useRefresh();
  return useMutation({
    mutationFn: (input: { name: string; file: string; token: string }) =>
      request({ type: "env_sync", ...input }, "env_synced"),
    onSettled: refresh,
  });
}

export function useEnvBackups(name: string, enabled: boolean) {
  return useQuery({
    queryKey: ["env", "backups", name],
    queryFn: async () => (await request({ type: "env_backups", name }, "env_backups")).backups,
    enabled,
    retry: false,
  });
}

export function useEnvRestore() {
  const refresh = useRefresh();
  return useMutation({
    mutationFn: (input: { name: string; id: string }) => request({ type: "env_restore", ...input }, "ok"),
    onSuccess: () => toast.success(t("Backup restored"), { description: t("The file it replaced was backed up too.") }),
    onError: (error) => toast.error(t("Could not restore the backup"), { description: errorMessage(error) }),
    onSettled: refresh,
  });
}
