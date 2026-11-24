// Parked folders: their projects become sites on their own. Discovery,
// naming and watching happen in the daemon; this only asks and shows.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { ParkOptions, ParkResponse } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useParked() {
  return useQuery({
    queryKey: ["parked"],
    queryFn: async () => (await request({ type: "park_list" }, "parked")).dirs,
    // Scans also happen on file events; keep the counts current.
    refetchInterval: 5000,
    retry: false,
  });
}

/** Says what a change did to the site list. */
function summarize({ added, removed }: ParkResponse) {
  if (added.length > 0) toast.success(t("New sites: {names}", { names: added.join(", ") }));
  if (removed.length > 0) {
    toast(t("Removed sites: {names}", { names: removed.join(", ") }), {
      description: t("Their files were not touched."),
    });
  }
}

function useParkMutation<I>(run: (input: I) => Promise<ParkResponse>, failure: string) {
  const client = useQueryClient();
  return useMutation({
    mutationFn: run,
    onSuccess: (response) => {
      client.setQueryData(["parked"], response.dirs);
      summarize(response);
    },
    onError: (error) => toast.error(failure, { description: errorMessage(error) }),
    // Sites come and go with these.
    onSettled: () => client.invalidateQueries(),
  });
}

export function useParkSet() {
  return useParkMutation(
    (input: { path: string; options?: ParkOptions }) => request({ type: "park_set", ...input }, "parked"),
    t("Could not save the parked folder"),
  );
}

export function useParkRemove() {
  return useParkMutation(
    (input: { path: string; keep_sites: boolean }) => request({ type: "park_remove", ...input }, "parked"),
    t("Could not remove the parked folder"),
  );
}

export function useParkScan() {
  return useParkMutation((path: string | null) => request({ type: "park_scan", path }, "parked"), t("Could not scan"));
}

export function useParkIgnore() {
  return useParkMutation(
    (input: { path: string; ignore: boolean }) => request({ type: "park_ignore", ...input }, "parked"),
    t("Could not change the project"),
  );
}
