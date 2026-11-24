// Config Files: queries and mutations behind a site's Config tab. The
// daemon parses (never runs) the files and checks the reviewed hash.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { request } from "@/lib/daemon";
import type { ConfigChange } from "@/lib/protocol";

const filesKey = (site: string) => ["config", site, "files"] as const;
const fileKey = (site: string, path: string) => ["config", site, "file", path] as const;

export function useConfigFiles(site: string) {
  return useQuery({
    queryKey: filesKey(site),
    queryFn: async () => (await request({ type: "config_files", name: site }, "config_files")).files,
    retry: false,
  });
}

export function useConfigFile(site: string, path: string | null) {
  return useQuery({
    queryKey: fileKey(site, path ?? ""),
    queryFn: async () => {
      const { type: _, ...view } = await request({ type: "config_file", name: site, path: path! }, "config_file");
      return view;
    },
    enabled: path !== null,
    retry: false,
  });
}

export function useConfigFindings(site: string) {
  return useQuery({
    queryKey: ["config", site, "findings"],
    queryFn: async () => (await request({ type: "config_check", name: site }, "config_findings")).findings,
    retry: false,
  });
}

export function useConfigHistory(site: string, path: string) {
  return useQuery({
    queryKey: ["config", site, "history", path],
    queryFn: async () => (await request({ type: "config_history", name: site, path }, "config_history")).entries,
    retry: false,
  });
}

export function useConfigActions(site: string) {
  const client = useQueryClient();
  const refresh = () => client.invalidateQueries({ queryKey: ["config", site] });
  return {
    preview: async (path: string, change: ConfigChange) => {
      const { type: _, ...preview } = await request(
        { type: "config_preview", name: site, path, change },
        "config_preview",
      );
      return preview;
    },
    version: async (path: string, id: string) =>
      (await request({ type: "config_history_version", name: site, path, id }, "config_history_version")).content,
    save: useMutation({
      mutationFn: (v: { path: string; change: ConfigChange; baseHash: string }) =>
        request(
          { type: "config_save", name: site, path: v.path, change: v.change, base_hash: v.baseHash },
          "config_saved",
        ),
      onSettled: refresh,
    }),
    revert: useMutation({
      mutationFn: (v: { path: string; id: string; baseHash: string }) =>
        request({ type: "config_revert", name: site, path: v.path, id: v.id, base_hash: v.baseHash }, "config_saved"),
      onSettled: refresh,
    }),
  };
}
