// New projects: the starters, the tools they need, the plan and the terminal.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { NewProject, PackageRegistry } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

const startersQuery = {
  queryKey: ["starters"],
  queryFn: () => request({ type: "list_starters" }, "starters"),
  staleTime: Number.POSITIVE_INFINITY,
  retry: false,
} as const;

export function useStarters() {
  return useQuery({ ...startersQuery, select: (r) => r.starters });
}

/** Where the app creates new projects by default (`~/LocDev/Sites`). */
export function useSitesDir() {
  return useQuery({ ...startersQuery, select: (r) => r.sites_dir ?? null });
}

/** Runs each tool's `--version`, so it isn't polled. */
export function useTools() {
  return useQuery({
    queryKey: ["tools"],
    queryFn: async () => (await request({ type: "list_tools" }, "tools")).tools,
    staleTime: 60_000,
    retry: false,
  });
}

/** The plan for what's in the form; recomputed by the daemon when creating. */
export function usePlan(project: NewProject | null) {
  return useQuery({
    queryKey: ["project-plan", project],
    queryFn: async () => {
      const { type: _, ...plan } = await request({ type: "plan_project", project: project! }, "project_plan");
      return plan;
    },
    enabled: project !== null,
    retry: false,
    placeholderData: (previous) => previous,
  });
}

export function useCreateProject() {
  return useMutation({
    mutationFn: async (project: NewProject) => {
      const { type: _, ...info } = await request({ type: "create_project", project }, "terminal");
      return info;
    },
    onError: (e) => toast.error(t("Could not start"), { description: errorMessage(e) }),
  });
}

export function useInstallTool() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      const { type: _, ...info } = await request({ type: "install_tool", id }, "terminal");
      return info;
    },
    onError: (e) => toast.error(t("Could not install"), { description: errorMessage(e) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["tools"] }),
  });
}

export function usePackageSearch(registry: PackageRegistry | null | undefined, query: string) {
  return useQuery({
    queryKey: ["package-search", registry, query],
    queryFn: async () => (await request({ type: "search_packages", registry: registry!, query }, "packages")).packages,
    enabled: !!registry && query.trim().length >= 2,
    retry: false,
    staleTime: 5 * 60_000,
  });
}
