// Tunnels for the mock backend: one provider, one shared site. The real
// work (download, checksums, the program, its metrics) is the daemon's.

import { DaemonError } from "./errors";
import type { ProjectView, TunnelProvider, TunnelRequest, TunnelResponse, TunnelView } from "./protocol";

const provider: TunnelProvider = {
  id: "cloudflare",
  name: "Cloudflare Tunnel (Quick Tunnel)",
  description:
    "A random trycloudflare.com address for the site, through Cloudflare's network. No account and no domain.",
  docs: "https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/do-more-with-tunnels/trycloudflare/",
  license: "Apache-2.0 (cloudflared)",
  account: false,
  limits: [
    "For testing and development, with no uptime guarantee; use is subject to Cloudflare's Online Services Terms of Use.",
    "Up to 200 requests at the same time (more get 429); Server-Sent Events don't work.",
    "The address changes every time the tunnel starts and stops working when it stops.",
    "Anyone with the address can open the site, unless you limit it to e-mail addresses (they confirm with a one-time PIN).",
  ],
  program: {
    version: "2026.10.7",
    path: "~/.local/share/locdev/tools/cloudflared/2026.10.7/cloudflared",
    managed: true,
  },
  installable: true,
};

const tunnels = new Map<string, TunnelView>([
  [
    "acme-shop",
    {
      site: "acme-shop",
      provider: "cloudflare",
      state: "online",
      url: "https://calm-fox-river.trycloudflare.com",
      started_at: Date.now() - 18 * 60_000,
      allowed: ["ana@acme.com", "*@agency.studio"],
      target: "http://127.0.0.1:8100",
    },
  ],
]);

const WORDS = ["amber", "brisk", "cedar", "delta", "ember", "frost", "grove", "harbor"];

export function handleTunnels(req: TunnelRequest, projects: () => ProjectView[]): TunnelResponse {
  switch (req.type) {
    case "tunnel_providers":
      return { type: "tunnel_providers", providers: [provider] };
    case "tunnel_install":
      throw new DaemonError("app", "the mock can't download anything", "REMOTE");
    case "tunnel_list":
    case "tunnel_logs":
      return { type: "tunnels", tunnels: [...tunnels.values()] };
    case "tunnel_start": {
      const site = projects().find((p) => p.name === req.site);
      if (!site) throw new DaemonError("app", `no project named '${req.site}'`, "NOT_FOUND");
      if (site.process.state !== "running")
        throw new DaemonError("app", `${req.site} isn't running; start it first`, "NOT_RUNNABLE");
      const bad = (req.allowed ?? []).find((a) => !/^(\*|[\w.%+-]+)@[\w-]+(\.[\w-]+)+$/.test(a));
      if (bad) throw new DaemonError("app", `'${bad}' is not an e-mail address or *@domain`, "INVALID_INPUT");
      const pick = () => WORDS[Math.floor(Math.random() * WORDS.length)];
      const tunnel: TunnelView = {
        site: req.site,
        provider: req.provider ?? "cloudflare",
        state: "online",
        url: `https://${pick()}-${pick()}-${pick()}.trycloudflare.com`,
        started_at: Date.now(),
        allowed: (req.allowed ?? []).map((a) => a.toLowerCase()),
        target: `http://127.0.0.1:${site.port}`,
      };
      tunnels.set(req.site, tunnel);
      return { type: "tunnel", tunnel };
    }
    case "tunnel_stop":
      tunnels.delete(req.site);
      return { type: "tunnels", tunnels: [...tunnels.values()] };
  }
}
