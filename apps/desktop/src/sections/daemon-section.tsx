import { Fact, Section } from "@/components/page";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useAppInfo, useDaemonControl, useDaemonDown, useService, useStatus } from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import { Play, RotateCw, Square } from "lucide-react";
import { t } from "@/lib/i18n";

function LoginItem() {
  const { query, set } = useService();
  const service = query.data;
  if (!service?.manager) return null;

  return (
    <label className="mt-4 flex items-start gap-3 rounded-lg border p-3">
      <Switch
        checked={service.installed}
        disabled={set.isPending}
        onCheckedChange={(enabled) => set.mutate(enabled)}
        className="mt-0.5 data-checked:bg-ember"
      />
      <span className="min-w-0">
        <span className="block text-sm">{t("Start LocDev at login")}</span>
        <span className="block truncate font-mono text-[11px] text-muted-foreground" title={service.location ?? ""}>
          {t("{manager} · per user, no administrator rights", { manager: service.manager })}
        </span>
      </span>
    </label>
  );
}

export function DaemonSection() {
  const status = useStatus();
  const down = useDaemonDown();
  const info = useAppInfo();
  const { start, stop, restart, pending } = useDaemonControl();
  const s = status.data;

  return (
    <Section
      title={t("daemon")}
      aside={
        <span className="flex items-center gap-2 font-mono text-[11px]">
          <Led tone={s ? "live" : down ? "fail" : "off"} live={!!s} />
          {s ? t("running") : down ? t("stopped") : t("checking")}
        </span>
      }
    >
      <Fact label={t("Process")} mono>
        {s ? `pid ${s.pid}` : "—"}
      </Fact>
      <Fact label={t("Uptime")} mono>
        {s ? uptime(s.started_at) : "—"}
      </Fact>
      <Fact label={t("Sites")} mono>
        {s ? t("{running} running / {projects}", { running: s.running, projects: s.projects }) : "—"}
      </Fact>
      <Fact label={t("IPC endpoint")} mono>
        {info.data?.endpoint ?? "—"}
      </Fact>

      <div className="mt-4 flex items-center gap-2">
        {s ? (
          <>
            <Button variant="outline" size="sm" onClick={() => restart.mutate()} disabled={pending}>
              <RotateCw data-icon="inline-start" /> {t("Restart")}
            </Button>
            <Button variant="destructive" size="sm" onClick={() => stop.mutate()} disabled={pending}>
              <Square data-icon="inline-start" className="fill-current" /> {t("Stop")}
            </Button>
          </>
        ) : (
          <Button
            size="sm"
            onClick={() => start.mutate()}
            disabled={pending}
            className="bg-ember text-ember-foreground hover:bg-ember/85"
          >
            <Play data-icon="inline-start" className="fill-current" /> {t("Start locdevd")}
          </Button>
        )}
        <span className="ml-2 text-xs text-muted-foreground">{t("Stopping it also stops every running site.")}</span>
      </div>

      <LoginItem />
    </Section>
  );
}
