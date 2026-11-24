// Sharing sites on the internet. The daemon runs the provider's program
// and reads the address back; this only asks and shows.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

import { request } from "@/lib/daemon";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

export function useTunnelProviders() {
  return useQuery({
    queryKey: ["tunnels", "providers"],
    queryFn: async () => (await request({ type: "tunnel_providers" }, "tunnel_providers")).providers,
    retry: false,
  });
}

export function useTunnels() {
  return useQuery({
    queryKey: ["tunnels", "list"],
    queryFn: async () => (await request({ type: "tunnel_list" }, "tunnels")).tunnels,
    // Fast while one is waiting for its address.
    refetchInterval: (q) => (q.state.data?.some((t) => t.state === "starting") ? 1000 : 4000),
    retry: false,
  });
}

export function useTunnelStart() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (input: { site: string; provider?: string; allowed: string[] }) =>
      (await request({ type: "tunnel_start", ...input }, "tunnel")).tunnel,
    onError: (error) => toast.error(t("Could not share the site"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["tunnels"] }),
  });
}

export function useTunnelStop() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (site: string) => request({ type: "tunnel_stop", site }, "tunnels"),
    onSuccess: (_, site) => toast(t("{site} is no longer shared", { site })),
    onError: (error) => toast.error(t("Could not stop sharing"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["tunnels"] }),
  });
}

export function useTunnelInstall() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async (provider: string) => (await request({ type: "tunnel_install", provider }, "job")).job,
    onSuccess: () => toast(t("Downloading cloudflared"), { description: t("It shows up here once it is verified.") }),
    onError: (error) => toast.error(t("Could not install it"), { description: errorMessage(error) }),
    onSettled: () => {
      client.invalidateQueries({ queryKey: ["tunnels"] });
      client.invalidateQueries({ queryKey: ["jobs"] });
    },
  });
}

export function useTunnelLogs(site: string | null) {
  return useQuery({
    queryKey: ["tunnels", "logs", site],
    queryFn: async () => (await request({ type: "tunnel_logs", site: site!, limit: 300 }, "logs")).lines,
    enabled: !!site,
    refetchInterval: 2000,
    retry: false,
  });
}
