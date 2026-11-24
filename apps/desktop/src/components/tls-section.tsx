import { Copy, FolderOpen, RefreshCw, ShieldCheck, Undo2 } from "lucide-react";
import { useState } from "react";

import { Fact, Section } from "@/components/page";
import { Led } from "@/components/status";
import { SystemDialog } from "@/components/system-dialog";
import { Button } from "@/components/ui/button";
import { useCheckTls, useStatus } from "@/hooks/use-daemon";
import type { SystemAction } from "@/lib/protocol";
import { copyText, revealFolder } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function TlsSection() {
  const status = useStatus();
  const check = useCheckTls();
  const [action, setAction] = useState<SystemAction | null>(null);
  const tls = status.data?.tls;

  if (!tls) {
    return (
      <Section title={t("https")}>
        <p className="text-sm text-muted-foreground">{t("Start the daemon to manage HTTPS.")}</p>
      </Section>
    );
  }

  const fullyTrusted = tls.trusted && tls.browsers !== false;

  return (
    <Section
      title={t("https")}
      aside={
        <span className="flex items-center gap-2 font-mono text-[11px]">
          <Led tone={!tls.port ? "off" : fullyTrusted ? "live" : "warn"} live={fullyTrusted} />
          {!tls.port ? t("off") : fullyTrusted ? t("trusted") : t("not trusted")}
        </span>
      }
    >
      <Fact label={t("Listening on")} mono>
        {tls.port ? `127.0.0.1:${tls.port}` : <span className="text-destructive">{tls.error ?? t("off")}</span>}
      </Fact>
      <Fact label={t("System store")} mono>
        <span className={tls.trusted ? "text-phosphor" : "text-amber"}>
          {tls.trusted ? t("trusted") : t("not trusted")}
        </span>
      </Fact>
      {tls.browsers !== null && (
        <Fact label={t("Chrome / Firefox")} mono>
          <span className={tls.browsers ? "text-phosphor" : "text-amber"}>
            {tls.browsers ? t("trusted") : t("not trusted")}
          </span>
        </Fact>
      )}

      {tls.fingerprint && (
        <button
          type="button"
          onClick={() => copyText(tls.fingerprint!, t("Fingerprint copied"))}
          className="group mt-3 w-full rounded-md border bg-well px-3 py-2 text-left transition-colors hover:border-ember/40"
          title={t("Compare with what your browser shows for the issuer")}
        >
          <div className="flex items-center justify-between font-mono text-[10.5px] text-muted-foreground">
            {t("local CA · sha-256")}
            <Copy className="size-3 opacity-60 group-hover:opacity-100" />
          </div>
          <div className="mt-1 font-mono text-[11px] leading-relaxed" data-selectable>
            {/* 32 bytes as two rows of 16, so it never wraps mid-byte. */}
            {[0, 16].map((start) => (
              <div key={start}>
                {tls
                  .fingerprint!.split(":")
                  .slice(start, start + 16)
                  .join(":")}
              </div>
            ))}
          </div>
        </button>
      )}

      <p className="mt-3 text-[13px] text-muted-foreground">
        {tx(
          "The CA is restricted to {test} and {localhost} (X.509 name constraints): browsers reject anything it signs for real domains, so trusting it can't be used to intercept other sites.",
          {
            test: <span className="font-mono text-foreground">.test</span>,
            localhost: <span className="font-mono text-foreground">.localhost</span>,
          },
        )}
      </p>

      <div className="mt-4 flex flex-wrap items-center gap-2">
        {!fullyTrusted && (
          <Button
            size="sm"
            onClick={() => setAction("install")}
            disabled={!tls.ca_path}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <ShieldCheck data-icon="inline-start" /> {t("Trust LocDev CA")}
          </Button>
        )}
        {(tls.trusted || tls.browsers) && (
          <Button variant="outline" size="sm" onClick={() => setAction("uninstall")}>
            <Undo2 data-icon="inline-start" /> {t("Remove trust")}
          </Button>
        )}
        {tls.ca_path && (
          <Button variant="ghost" size="sm" onClick={() => revealFolder(tls.ca_path!)}>
            <FolderOpen data-icon="inline-start" /> {t("CA file")}
          </Button>
        )}
        <Button variant="ghost" size="sm" onClick={() => check.mutate()} disabled={check.isPending}>
          <RefreshCw data-icon="inline-start" className={cn(check.isPending && "animate-spin")} /> {t("Re-check")}
        </Button>
      </div>

      <SystemDialog change={action && { target: "tls", action }} onClose={() => setAction(null)} />
    </Section>
  );
}
