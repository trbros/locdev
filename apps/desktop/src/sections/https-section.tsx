import { Section } from "@/components/page";
import { useProjects, useStatus } from "@/hooks/use-daemon";
import { openInBrowser } from "@/lib/system";
import { LockOpen, Lock } from "lucide-react";
import { t } from "@/lib/i18n";

/** Per-site HTTPS: whether each site answers over HTTPS and how browsers will see it. */
export function SiteHttpsSection() {
  const projects = useProjects();
  const status = useStatus();
  const trusted = status.data?.tls.trusted ?? false;
  const sites = (projects.data ?? []).filter((p) => p.https_url);

  return (
    <Section title={t("sites over https")}>
      {sites.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          {t("No site is served over HTTPS (HTTPS is off or no site has a port).")}
        </p>
      ) : (
        <ul>
          {sites.map((p) => (
            <li key={p.name} className="flex items-center gap-3 border-b border-dashed py-2 text-sm last:border-b-0">
              {trusted ? <Lock className="size-3.5 text-phosphor" /> : <LockOpen className="size-3.5 text-amber" />}
              <button
                type="button"
                onClick={() => openInBrowser(p.https_url!)}
                disabled={p.process.state !== "running"}
                className="min-w-0 flex-1 truncate text-left font-mono text-[12.5px] enabled:hover:text-ember disabled:text-muted-foreground"
              >
                {p.https_url}
              </button>
              <span className="font-mono text-[11px] text-muted-foreground">
                {p.process.state !== "running"
                  ? t("stopped")
                  : trusted
                    ? t("certificate trusted")
                    : t("browsers will warn")}
              </span>
            </li>
          ))}
        </ul>
      )}
      <p className="mt-3 text-[13px] text-muted-foreground">
        {t(
          "Certificates are minted per site from the local CA when a browser connects; nothing to manage per site. From a terminal:",
        )}{" "}
        <span className="font-mono text-foreground">{t("locdev tls status")}</span>.
      </p>
    </Section>
  );
}
