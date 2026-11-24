import { RefreshCw, ShieldCheck, Undo2 } from "lucide-react";
import { useState } from "react";

import { Fact, Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { SystemDialog } from "@/components/system-dialog";
import { Button } from "@/components/ui/button";
import { useCheckDns, useStatus } from "@/hooks/use-daemon";
import type { Routing, SystemAction } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const ROUTING: Record<Routing, { tone: Tone; label: string }> = {
  locdev: { tone: "live", label: t("*.test → LocDev") },
  other: { tone: "warn", label: t("*.test → another tool") },
  none: { tone: "off", label: t("*.test not resolving") },
};

export function DnsSection() {
  const status = useStatus();
  const check = useCheckDns();
  const [action, setAction] = useState<SystemAction | null>(null);
  const dns = status.data?.dns;

  if (!dns) {
    return (
      <Section title={t("dns")}>
        <p className="text-sm text-muted-foreground">{t("Start the daemon to manage DNS.")}</p>
      </Section>
    );
  }

  const routing = ROUTING[dns.routing];
  const supported = dns.backend !== "unsupported";

  return (
    <Section
      title={t("dns")}
      aside={
        <span className="flex items-center gap-2 font-mono text-[11px]">
          <Led tone={routing.tone} live={dns.routing === "locdev"} />
          {routing.label}
        </span>
      }
    >
      <Fact label={t("Server")} mono>
        {dns.port ? `127.0.0.1:${dns.port}` : <span className="text-destructive">{dns.error ?? t("off")}</span>}
      </Fact>
      <Fact label={t("System resolver")} mono>
        {dns.backend}
        {dns.installed !== null && (
          <span className="text-muted-foreground"> · {dns.installed ? t("configured") : t("not configured")}</span>
        )}
      </Fact>

      {dns.conflicts.length > 0 && (
        <div className="mt-3 rounded-md border border-amber/30 bg-amber/[0.07] px-3 py-2.5 text-[13px]">
          <div className="text-amber">{t("Another tool also claims *.test:")}</div>
          <ul className="mt-1 font-mono text-xs text-foreground/80">
            {dns.conflicts.map((file) => (
              <li key={file} data-selectable>
                {file}
              </li>
            ))}
          </ul>
          <p className="mt-1.5 text-xs text-muted-foreground">
            {t(
              "LocDev never edits other tools' files. With both in place, lookups can be slow or reach the wrong tool; remove that file if you no longer use it.",
            )}
          </p>
        </div>
      )}

      {!supported && (
        <p className="mt-3 text-[13px] text-muted-foreground">
          {tx(
            "This system's resolver can't be configured automatically. Point {test} at {address} yourself, or keep using {localhost} URLs.",
            {
              test: <span className="font-mono">*.test</span>,
              address: <span className="font-mono">127.0.0.1:{dns.port}</span>,
              localhost: <span className="font-mono">.localhost</span>,
            },
          )}
        </p>
      )}

      <div className="mt-4 flex flex-wrap items-center gap-2">
        {supported && dns.installed !== true && (
          <Button
            size="sm"
            onClick={() => setAction("install")}
            disabled={!dns.port}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <ShieldCheck data-icon="inline-start" /> {t("Route *.test to LocDev")}
          </Button>
        )}
        {supported && dns.installed !== false && (
          <Button variant="outline" size="sm" onClick={() => setAction("uninstall")}>
            <Undo2 data-icon="inline-start" /> {t("Remove DNS changes")}
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={() => check.mutate()} disabled={check.isPending}>
          <RefreshCw data-icon="inline-start" className={cn(check.isPending && "animate-spin")} /> {t("Re-check")}
        </Button>
      </div>

      <SystemDialog change={action && { target: "dns", action }} onClose={() => setAction(null)} />
    </Section>
  );
}
