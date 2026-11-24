// The Debug Center: the daemon collects and stores events; this asks for
// them (polling while the page is open) and sends the user's actions.

import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { EventFilter } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useDebugEvents(filter: EventFilter, live: boolean) {
  return useQuery({
    queryKey: ["debug", "events", filter],
    queryFn: async () => request({ type: "observe_events", filter, limit: 300 }, "observe_events"),
    refetchInterval: live ? 1500 : false,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useDebugGroups(filter: EventFilter, live: boolean) {
  return useQuery({
    queryKey: ["debug", "groups", filter],
    queryFn: async () => (await request({ type: "observe_groups", filter }, "observe_groups")).groups,
    refetchInterval: live ? 4000 : false,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useDebugEvent(id: number | null) {
  return useQuery({
    queryKey: ["debug", "event", id],
    queryFn: async () => (await request({ type: "observe_event", id: id! }, "observe_event")).event,
    enabled: id != null,
    staleTime: Infinity,
    retry: false,
  });
}

export function useDebugStatus() {
  return useQuery({
    queryKey: ["debug", "status"],
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "observe_status" }, "observe_status");
      return status;
    },
    refetchInterval: 10_000,
    retry: false,
  });
}

export function useDebugActions() {
  const client = useQueryClient();
  const clear = useMutation({
    mutationFn: (site: string | null) => request({ type: "observe_clear", site }, "ok"),
    onSuccess: () => toast.success(t("Events cleared")),
    onError: (error) => toast.error(t("Could not clear the events"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["debug"] }),
  });
  const dumps = useMutation({
    mutationFn: (input: { site: string; enabled: boolean }) =>
      request({ type: "observe_dumps", ...input }, "observe_dumps"),
    onSuccess: (r) =>
      toast.success(
        r.enabled
          ? r.restarted
            ? t("Dumps from {site} now come here (restarted)", { site: r.site })
            : t("Dumps from {site} will come here once it starts", { site: r.site })
          : t("Dumps from {site} go back to the page", { site: r.site }),
      ),
    onError: (error) => toast.error(t("Could not change dumps"), { description: errorMessage(error) }),
    onSettled: () => {
      client.invalidateQueries({ queryKey: ["debug", "status"] });
      client.invalidateQueries({ queryKey: ["projects"] });
    },
  });
  return { clear, dumps };
}
