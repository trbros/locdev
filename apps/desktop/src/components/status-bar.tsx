import type { ReactNode } from "react";

import { Led } from "@/components/status";
import { useDaemonDown, useProjects, useStatus } from "@/hooks/use-daemon";
import { isMock, screenshotMode } from "@/lib/daemon";
import { uptime } from "@/lib/format";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

/** Bottom status line in the spirit of a terminal multiplexer. */
export function StatusBar() {
  const status = useStatus();
  const projects = useProjects();
  const down = useDaemonDown();
  const { navigate } = useUi();

  const s = status.data;
  const sites = projects.data ?? [];
  const running = sites.filter((p) => p.process.state === "running").length;
  const proxy = s?.proxy;

  return (
    <footer className="flex h-7 shrink-0 items-stretch border-t bg-chrome font-mono text-[11px] text-muted-foreground">
      <Segment onClick={() => navigate("settings", "daemon")} className="text-foreground/90">
        {s ? <Led tone="live" live /> : <Led tone={down ? "fail" : "off"} />}
        {s ? `locdevd ${s.version}` : down ? t("locdevd offline") : t("connecting…")}
      </Segment>

      {s && (
        <>
          <Segment onClick={() => navigate("sites")}>
            <span>
              <span className="text-foreground/90">{running}</span>/{sites.length} {t("up")}
            </span>
          </Segment>
          <Segment onClick={() => navigate("settings", "proxy")}>
            {proxy?.port ? (
              <>
                proxy <span className="text-foreground/90">:{proxy.port}</span>
                {proxy.error && <span className="text-amber">{t("fallback")}</span>}
              </>
            ) : (
              <span className="text-amber">{t("proxy off")}</span>
            )}
          </Segment>
          <Segment onClick={() => navigate("settings", "dns")}>
            <Led tone={s.dns.routing === "locdev" ? "live" : s.dns.routing === "other" ? "warn" : "off"} />
            {s.dns.routing === "locdev" ? (
              <span className="text-foreground/90">{t("*.test")}</span>
            ) : s.dns.routing === "other" ? (
              <span className="text-amber">{t("*.test via other tool")}</span>
            ) : (
              <span>{t("*.localhost")}</span>
            )}
          </Segment>
          {s.tls.port && (
            <Segment onClick={() => navigate("settings", "https")}>
              <Led tone={s.tls.trusted ? "live" : "warn"} />
              <span className={s.tls.trusted ? "text-foreground/90" : "text-amber"}>
                {s.tls.trusted ? t("https") : t("https untrusted")}
              </span>
            </Segment>
          )}
        </>
      )}

      <div className="ml-auto flex items-stretch">
        {isMock && !screenshotMode && <Segment className="text-amber">{t("mock backend")}</Segment>}
        {s && (
          <Segment>
            {t("up")} {uptime(s.started_at)}
          </Segment>
        )}
        <Segment>{t("locdev")}</Segment>
      </div>
    </footer>
  );
}

function Segment({ children, onClick, className }: { children: ReactNode; onClick?: () => void; className?: string }) {
  const classes = cn("flex items-center gap-1.5 border-r px-3 last:border-r-0", className);
  return onClick ? (
    <button type="button" onClick={onClick} className={cn(classes, "transition-colors hover:bg-foreground/5")}>
      {children}
    </button>
  ) : (
    <div className={classes}>{children}</div>
  );
}
