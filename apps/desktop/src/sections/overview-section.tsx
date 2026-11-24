import { Fact, Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { useDaemonDown, useRuntimes, useService, useSettings, useStatus } from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import { RUNTIME_LABEL, type AuditInterval } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { ChevronRight } from "lucide-react";
import { useTheme } from "next-themes";
import type { ReactNode } from "react";
import { t } from "@/lib/i18n";

const AUDIT_LABEL: Record<AuditInterval, string> = {
  off: t("off"),
  manual: t("only when asked"),
  "6h": t("every 6 hours"),
  "12h": t("every 12 hours"),
  daily: t("daily"),
  weekly: t("weekly"),
};

/** One line per area: its state and a way in. */
export function OverviewSection() {
  const status = useStatus();
  const down = useDaemonDown();
  const { query: service } = useService();
  const runtimes = useRuntimes();
  const { theme } = useTheme();
  const { navigate } = useUi();
  const automation = useSettings().query.data;
  const s = status.data;

  const rows: { id: string; label: string; tone: Tone; value: ReactNode }[] = [
    {
      id: "daemon",
      label: t("Daemon"),
      tone: s ? "live" : down ? "fail" : "off",
      value: s
        ? t("running · up {started_at} · {running}/{projects} sites", {
            started_at: uptime(s.started_at),
            running: s.running,
            projects: s.projects,
          })
        : down
          ? "stopped"
          : "…",
    },
    {
      id: "daemon",
      label: t("Start at login"),
      tone: service.data?.installed ? "live" : "off",
      value: !service.data?.manager ? t("not supported here") : service.data.installed ? t("on") : t("off"),
    },
    {
      id: "proxy",
      label: t("Proxy"),
      tone: !s ? "off" : s.proxy.port ? (s.proxy.error ? "warn" : "live") : "fail",
      value: !s
        ? "—"
        : s.proxy.port
          ? `127.0.0.1:${s.proxy.port}${s.proxy.error ? t(" (fallback)") : ""}`
          : t("not listening"),
    },
    {
      id: "dns",
      label: t("DNS"),
      tone: !s ? "off" : s.dns.routing === "locdev" ? "live" : s.dns.routing === "other" ? "warn" : "off",
      value: !s
        ? "—"
        : s.dns.routing === "locdev"
          ? "*.test → LocDev"
          : s.dns.routing === "other"
            ? "*.test → another tool"
            : "*.test not set up (sites use .localhost)",
    },
    {
      id: "https",
      label: t("HTTPS"),
      tone: !s?.tls.port ? "off" : s.tls.trusted ? "live" : "warn",
      value: !s
        ? "—"
        : !s.tls.port
          ? (s.tls.error ?? t("off"))
          : s.tls.trusted
            ? t("CA trusted")
            : t("CA not trusted yet"),
    },
    {
      id: "automations",
      label: t("Dependency checks"),
      tone: !automation
        ? "off"
        : automation.network.offline
          ? "warn"
          : automation.automation.audit === "off"
            ? "off"
            : "live",
      value: !automation
        ? "—"
        : automation.network.offline
          ? t("offline mode: paused")
          : AUDIT_LABEL[automation.automation.audit],
    },
    {
      id: "appearance",
      label: t("Theme"),
      tone: "info",
      value: theme === "system" ? t("follows the system") : theme === "light" ? t("Paper (light)") : t("Carbon (dark)"),
    },
  ];

  const defaults = runtimes.data?.filter((r) => r.default) ?? [];

  return (
    <>
      <Section title={t("state")}>
        <ul>
          {rows.map((row) => (
            <li key={row.label}>
              <button
                type="button"
                onClick={() => navigate("settings", row.id)}
                className="group flex w-full items-center gap-3 border-b border-dashed p-2.5 text-left text-sm last:border-b-0 hover:bg-foreground/2"
              >
                <Led tone={row.tone} live={row.tone === "live"} />
                <span className="w-32 shrink-0">{row.label}</span>
                <span className="min-w-0 flex-1 truncate font-mono text-[12.5px] text-muted-foreground">
                  {row.value}
                </span>
                <ChevronRight className="size-3.5 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
              </button>
            </li>
          ))}
        </ul>
      </Section>
      <Section
        title={t("runtime defaults")}
        aside={
          <button
            type="button"
            onClick={() => navigate("runtimes")}
            className="font-mono text-[11px] text-muted-foreground hover:text-foreground"
          >
            {t("manage →")}
          </button>
        }
      >
        {defaults.length ? (
          defaults.map((r) => (
            <Fact key={r.kind} label={RUNTIME_LABEL[r.kind]} mono>
              {r.default}
            </Fact>
          ))
        ) : (
          <p className="text-sm text-muted-foreground">{t("No defaults: sites without a pin use the system PATH.")}</p>
        )}
      </Section>
    </>
  );
}
