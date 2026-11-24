// Framework adapters: a site's stack, manual choices and the definitions.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";

import { errorMessage, keys } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useStackReport(name: string | null) {
  return useQuery({
    queryKey: ["stack", name],
    queryFn: async () => {
      const { type: _, ...report } = await request({ type: "stack_detect", name }, "stack_report");
      return report;
    },
    enabled: name !== null,
    retry: false,
  });
}

/** What a folder would be detected as, before it's added. */
export function useDetectFolder(path: string | null) {
  return useQuery({
    queryKey: ["stack-folder", path],
    queryFn: async () => {
      const { type: _, ...report } = await request({ type: "stack_detect", path }, "stack_report");
      return report;
    },
    enabled: !!path,
    retry: false,
  });
}

export function useDefinitions() {
  return useQuery({
    queryKey: ["definitions"],
    queryFn: async () => {
      const { definitions, registry } = await request({ type: "stack_definitions" }, "stack_definitions");
      return { definitions, registry };
    },
    retry: false,
    staleTime: 60_000,
  });
}

export function useStackSet() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (input: {
      name: string;
      add?: string[];
      remove?: string[];
      adapter?: string | null;
      apply_command?: boolean;
    }) => request({ type: "stack_set", ...input }, "stack_report"),
    onError: (error) => toast.error(t("Could not change the stack"), { description: errorMessage(error) }),
    onSettled: (_, __, { name }) => {
      client.invalidateQueries({ queryKey: ["stack", name] });
      client.invalidateQueries({ queryKey: keys.projects });
    },
  });
}

export function useDefinitionsActions() {
  const client = useQueryClient();
  const done = () => {
    client.invalidateQueries({ queryKey: ["definitions"] });
    client.invalidateQueries({ queryKey: ["stack"] });
    client.invalidateQueries({ queryKey: keys.projects });
  };
  const refresh = useMutation({
    mutationFn: (force: boolean) => request({ type: "refresh_definitions", force }, "stack_definitions"),
    onSuccess: ({ registry }) =>
      toast.success(t("Definitions at revision {revision}", { revision: registry.revision })),
    onError: (error) => toast.error(t("Could not update the definitions"), { description: errorMessage(error) }),
    onSettled: done,
  });
  const rollback = useMutation({
    mutationFn: () => request({ type: "rollback_definitions" }, "stack_definitions"),
    onSuccess: ({ registry }) => toast.success(t("Back to revision {revision}", { revision: registry.revision })),
    onError: (error) => toast.error(t("Could not roll back"), { description: errorMessage(error) }),
    onSettled: done,
  });
  return { refresh, rollback };
}
