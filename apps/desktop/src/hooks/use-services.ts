// Databases and caches: installs, instances, backups.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { ProcessAction, ServiceKind } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useServices() {
  return useQuery({
    queryKey: ["services"],
    queryFn: async () => {
      const { services, instances } = await request({ type: "list_services" }, "services");
      return { services, instances };
    },
    refetchInterval: 2000,
    retry: false,
  });
}

export function useServiceReleases(kind: ServiceKind | null) {
  return useQuery({
    queryKey: ["service-releases", kind],
    queryFn: async () => (await request({ type: "service_releases", kind: kind! }, "service_releases")).releases,
    enabled: kind !== null,
    retry: false,
    staleTime: 10 * 60_000,
  });
}

export function useInstanceBackups(name: string | null) {
  return useQuery({
    queryKey: ["instance-backups", name],
    queryFn: async () => (await request({ type: "instance_backups", name: name! }, "instance_backups")).backups,
    enabled: name !== null,
    retry: false,
  });
}

export function useServiceActions() {
  const client = useQueryClient();
  const refresh = () => {
    client.invalidateQueries({ queryKey: ["services"] });
    client.invalidateQueries({ queryKey: ["jobs"] });
    client.invalidateQueries({ queryKey: ["instance-backups"] });
  };
  const fail = (title: string) => (error: unknown) => toast.error(title, { description: errorMessage(error) });
  const install = useMutation({
    mutationFn: (input: { kind: ServiceKind; version: string }) =>
      request({ type: "install_service", ...input }, "job"),
    onSuccess: () =>
      toast.success(t("Download queued"), { description: t("It's installed only if its SHA-256 matches.") }),
    onError: fail(t("Could not install")),
    onSettled: refresh,
  });
  const uninstall = useMutation({
    mutationFn: (input: { kind: ServiceKind; version: string }) =>
      request({ type: "uninstall_service", ...input }, "services"),
    onError: fail(t("Could not remove the version")),
    onSettled: refresh,
  });
  const create = useMutation({
    mutationFn: (input: {
      name: string;
      kind: ServiceKind;
      version?: string | null;
      port?: number | null;
      autostart: boolean;
    }) => request({ type: "create_instance", ...input }, "instance"),
    onSettled: refresh,
  });
  const control = useMutation({
    mutationFn: (input: { name: string; action: ProcessAction }) =>
      request({ type: "control_instance", ...input }, "instance"),
    onError: fail(t("The instance didn't change")),
    onSettled: refresh,
  });
  const autostart = useMutation({
    mutationFn: (input: { name: string; autostart: boolean }) =>
      request({ type: "set_instance", ...input }, "instance"),
    onError: fail(t("Could not save")),
    onSettled: refresh,
  });
  const remove = useMutation({
    mutationFn: (input: { name: string; delete_data: boolean; confirm: string | null }) =>
      request({ type: "remove_instance", ...input }, "services"),
    onSettled: refresh,
  });
  const backup = useMutation({
    mutationFn: (name: string) => request({ type: "backup_instance", name }, "instance_backup"),
    onSuccess: () => toast.success(t("Backup taken")),
    onError: fail(t("Backup failed")),
    onSettled: refresh,
  });
  const restore = useMutation({
    mutationFn: (input: { name: string; id: string }) =>
      request({ type: "restore_instance", ...input, confirm: input.name }, "instance"),
    onSuccess: () =>
      toast.success(t("Restored"), { description: t("The previous data was moved aside, not deleted.") }),
    onError: fail(t("Restore failed")),
    onSettled: refresh,
  });
  return { install, uninstall, create, control, autostart, remove, backup, restore };
}
