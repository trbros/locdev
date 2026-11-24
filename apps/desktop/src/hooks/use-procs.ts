// A site's auxiliary processes (queue workers, schedulers, servers).

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { DevServerMode, ProcessAction } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

const key = (name: string) => ["procs", name] as const;

export function useAuxProcesses(name: string) {
  return useQuery({
    queryKey: key(name),
    queryFn: async () => (await request({ type: "project_processes", name }, "project_processes")).processes,
    refetchInterval: 2000,
    retry: false,
  });
}

export function useAuxActions(name: string) {
  const client = useQueryClient();
  const done = (processes: Awaited<ReturnType<typeof request<"project_processes">>>["processes"]) =>
    client.setQueryData(key(name), processes);
  const fail = (title: string) => (error: unknown) => toast.error(title, { description: errorMessage(error) });
  const control = useMutation({
    mutationFn: async (input: { process: string; action: ProcessAction }) =>
      (await request({ type: "control_process", name, ...input }, "project_processes")).processes,
    onSuccess: done,
    onError: fail(t("The process didn't change")),
  });
  const set = useMutation({
    mutationFn: async (input: {
      process: string;
      mode?: DevServerMode;
      command?: string | null;
      label?: string | null;
    }) => (await request({ type: "set_process", name, ...input }, "project_processes")).processes,
    onSuccess: done,
    onError: fail(t("Could not save the process")),
  });
  const remove = useMutation({
    mutationFn: async (process: string) =>
      (await request({ type: "remove_process", name, process }, "project_processes")).processes,
    onSuccess: done,
    onError: fail(t("Could not remove the process")),
  });
  return { control, set, remove };
}
