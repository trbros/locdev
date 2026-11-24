// PHP settings per version line. The daemon validates, backs up, checks
// what PHP loads and restarts the sites; this only asks and shows.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";
import type { PhpIniChange } from "@/lib/protocol";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function usePhpLines() {
  return useQuery({
    queryKey: ["php", "lines"],
    queryFn: async () => (await request({ type: "php_config_lines" }, "php_lines")).lines,
    retry: false,
  });
}

export function usePhpConfig(line: string | null) {
  return useQuery({
    queryKey: ["php", "config", line],
    queryFn: async () => (await request({ type: "php_config", line: line! }, "php_config")).config,
    enabled: !!line,
    retry: false,
  });
}

/** What a change would do (nothing is written). */
export function usePhpPreview() {
  return useMutation({
    mutationFn: async (input: { line: string; change: PhpIniChange }) =>
      (await request({ type: "php_config_change", ...input, dry_run: true }, "php_config")).preview!,
  });
}

export function usePhpApply() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { line: string; change: PhpIniChange }) =>
      request({ type: "php_config_change", ...input, dry_run: false }, "php_config"),
    onSuccess: ({ config, restarted }) => {
      client.setQueryData(["php", "config", config.line], config);
      toast.success(t("PHP {line} settings applied", { line: config.line }), {
        description: restarted.length
          ? t("Restarted: {sites}", { sites: restarted.join(", ") })
          : t("No site on this line was running."),
      });
    },
    onError: (error) => toast.error(t("The settings were not applied"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["php", "lines"] }),
  });
}

export function usePhpFile(line: string, path: string | null) {
  return useQuery({
    queryKey: ["php", "file", line, path],
    queryFn: async () => (await request({ type: "php_ini_read", line, path: path! }, "php_ini_file")).text,
    enabled: !!path,
    retry: false,
  });
}
