import { JobList } from "@/components/jobs";
import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useJobs, useSettings } from "@/hooks/use-daemon";
import { jobFinished, type AuditInterval, type Settings } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const AUDIT: { id: AuditInterval; label: string }[] = [
  { id: "off", label: t("Off") },
  { id: "manual", label: t("Only when I ask") },
  { id: "6h", label: t("Every 6 hours") },
  { id: "12h", label: t("Every 12 hours") },
  { id: "daily", label: t("Daily") },
  { id: "weekly", label: t("Weekly") },
];

function NumberSetting({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (value: number) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3 py-1.5 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <span className="flex items-center gap-1">
        <Button
          variant="outline"
          size="icon-xs"
          disabled={value <= min}
          onClick={() => onChange(value - 1)}
          aria-label={t("Less")}
        >
          −
        </Button>
        <span className="w-8 text-center font-mono tabular-nums">{value}</span>
        <Button
          variant="outline"
          size="icon-xs"
          disabled={value >= max}
          onClick={() => onChange(value + 1)}
          aria-label={t("More")}
        >
          +
        </Button>
      </span>
    </label>
  );
}

export function AutomationsSection() {
  const { query, update } = useSettings();
  const jobs = useJobs();
  const settings = query.data;
  if (!settings) return <Skeleton className="h-48" />;
  const set = (patch: Partial<Settings>) => update.mutate({ ...settings, ...patch });
  const automation = (patch: Partial<Settings["automation"]>) =>
    set({ automation: { ...settings.automation, ...patch } });
  const active = (jobs.data ?? []).filter((j) => !jobFinished(j.status));
  const recent = (jobs.data ?? []).filter((j) => jobFinished(j.status)).slice(0, 15);

  return (
    <>
      <Section title={t("dependency checks")}>
        <div role="radiogroup" aria-label={t("How often")} className="grid grid-cols-3 gap-2 pt-1">
          {AUDIT.map(({ id, label }) => (
            <button
              type="button"
              role="radio"
              key={id}
              aria-checked={settings.automation.audit === id}
              onClick={() => automation({ audit: id })}
              className={cn(
                "rounded-lg border px-3 py-2 text-left text-sm transition-colors hover:bg-foreground/3",
                settings.automation.audit === id && "border-ember/60 bg-ember/6",
              )}
            >
              {label}
            </button>
          ))}
        </div>
        <p className="mt-2 text-[13px] text-muted-foreground">
          {t(
            "Checks ask npm, Packagist and osv.dev about each site's packages, one at a time, through a queue that survives restarts; a check that was due while the computer was off runs when it's back. Turning this off (or going offline) cancels the queued checks and stops running ones at the next package.",
          )}
        </p>
      </Section>
      <Section title={t("limits")}>
        <div className="grid grid-cols-2 gap-x-8">
          <NumberSetting
            label={t("Jobs at the same time")}
            value={settings.automation.concurrency}
            min={1}
            max={8}
            onChange={(v) => automation({ concurrency: v })}
          />
          <NumberSetting
            label={t("Registry requests per second")}
            value={settings.automation.requests_per_second}
            min={1}
            max={50}
            onChange={(v) => automation({ requests_per_second: v })}
          />
        </div>
      </Section>
      <Section title={t("network")}>
        <label className="flex items-start gap-3 py-1.5">
          <Switch
            checked={settings.network.offline}
            onCheckedChange={(offline) => set({ network: { offline } })}
            className="mt-0.5 data-checked:bg-ember"
          />
          <span>
            <span className="block text-sm">{t("Offline mode")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {t(
                'No background network use: no automatic checks, release catalogs or support schedules. What you ask for explicitly (an install, "Check now") still goes out.',
              )}
            </span>
          </span>
        </label>
      </Section>
      <Section title={t("queue · {length} active", { length: active.length })}>
        <JobList jobs={active} empty={t("Nothing queued.")} />
      </Section>
      <Section title={t("recent")}>
        <JobList jobs={recent} empty={t("No finished jobs yet.")} />
      </Section>
    </>
  );
}
