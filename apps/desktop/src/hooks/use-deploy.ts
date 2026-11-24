// Deploy assistant: the analysis and config files behind a site's Deploy tab.
// The daemon builds everything; nothing is sent to any platform.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { request } from "@/lib/daemon";

export function useDeployReport(site: string) {
  return useQuery({
    queryKey: ["deploy", site],
    queryFn: async () => {
      const { type: _, ...report } = await request({ type: "deploy_analysis", name: site }, "deploy_report");
      return report;
    },
    retry: false,
  });
}

export function useDeployActions(site: string) {
  const client = useQueryClient();
  return {
    config: async (provider: string) => {
      const { type: _, ...config } = await request({ type: "deploy_config", name: site, provider }, "deploy_config");
      return config;
    },
    write: useMutation({
      mutationFn: (v: { provider: string; hash: string }) =>
        request(
          { type: "deploy_write_config", name: site, provider: v.provider, hash: v.hash },
          "deploy_config_written",
        ),
      onSettled: () => client.invalidateQueries({ queryKey: ["deploy", site] }),
    }),
  };
}
