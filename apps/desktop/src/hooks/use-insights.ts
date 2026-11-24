// Request metrics, Tinker, PHP extensions, worktrees, workflow audit and
// history, and service packages. The daemon does the work; this asks.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import { t } from "@/lib/i18n";
import type { ServiceKind } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";

const strip = <T extends { type: string }>({ type: _, ...rest }: T) => rest;

export function useRequestMetrics(name: string) {
  return useQuery({
    queryKey: ["metrics", name],
    queryFn: async () => strip(await request({ type: "request_metrics", name }, "request_metrics")),
    refetchInterval: 3_000,
  });
}

export function useResetMetrics(name: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => request({ type: "reset_request_metrics", name }, "ok"),
    onSettled: () => client.invalidateQueries({ queryKey: ["metrics", name] }),
  });
}

export function useTinkerRun(name: string) {
  return useMutation({
    mutationFn: async (code: string) => strip(await request({ type: "tinker_run", name, code }, "tinker")),
    onError: (e) => toast.error(t("Tinker didn't run"), { description: errorMessage(e) }),
  });
}

export function useTinkerOpen(name: string) {
  return useMutation({
    mutationFn: async () => strip(await request({ type: "tinker_open", name }, "terminal")),
    onError: (e) => toast.error(t("Tinker didn't start"), { description: errorMessage(e) }),
  });
}

export function usePhpExtensions(name: string, enabled: boolean) {
  return useQuery({
    queryKey: ["php-extensions", name],
    queryFn: async () => strip(await request({ type: "php_extensions", name }, "php_extensions")),
    enabled,
    retry: false,
  });
}

export function usePhpModules(executable: string | null) {
  return useQuery({
    queryKey: ["php-modules", executable],
    queryFn: async () => strip(await request({ type: "php_modules", executable: executable! }, "php_modules")),
    enabled: !!executable,
    retry: false,
  });
}

export function useWorktrees(name: string) {
  return useQuery({
    queryKey: ["git", name, "worktrees"],
    queryFn: async () => (await request({ type: "git_worktrees", name }, "worktrees")).worktrees,
    retry: false,
  });
}

export function useActionsAudit(name: string) {
  return useQuery({
    queryKey: ["actions", name, "audit"],
    queryFn: async () => strip(await request({ type: "actions_audit", name }, "actions_audit")),
    retry: false,
  });
}

export function useActionsHistory(name: string, file: string | null) {
  return useQuery({
    queryKey: ["actions", name, "history", file],
    queryFn: async () => (await request({ type: "actions_history", name, file: file! }, "actions_history")).versions,
    enabled: !!file,
  });
}

export function useActionsHistoryVersion(name: string) {
  return useMutation({
    mutationFn: async ({ file, id }: { file: string; id: string }) =>
      strip(await request({ type: "actions_history_version", name, file, id }, "actions_yaml")),
    onError: (e) => toast.error(t("Could not open that version"), { description: errorMessage(e) }),
  });
}

export function useActionlintInstall() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => request({ type: "actionlint_install" }, "ok"),
    onSuccess: () => toast.success(t("actionlint installed")),
    onError: (e) => toast.error(t("Could not install actionlint"), { description: errorMessage(e) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["actions"] }),
  });
}

export function useInstallServicePackage() {
  return useMutation({
    mutationFn: async ({ kind, id }: { kind: ServiceKind; id: string }) =>
      strip(await request({ type: "install_service_package", kind, id }, "terminal")),
    onError: (e) => toast.error(t("Could not install"), { description: errorMessage(e) }),
  });
}
