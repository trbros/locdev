import { Section } from "@/components/page";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useSettings } from "@/hooks/use-daemon";
import { useDebugStatus } from "@/hooks/use-debug";
import type { EventSeverity, Settings } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const FROM: { id: EventSeverity; label: string }[] = [
  { id: "info", label: t("Info and up") },
  { id: "warn", label: t("Warnings and up") },
  { id: "error", label: t("Errors and up") },
];

function Stepper({
  label,
  value,
  options,
  unit,
  onChange,
}: {
  label: string;
  value: number;
  options: number[];
  unit: string;
  onChange: (v: number) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3 py-1.5 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="h-7 rounded-md border bg-transparent px-2 text-[12.5px]"
      >
        {[...new Set([...options, value])]
          .sort((a, b) => a - b)
          .map((n) => (
            <option key={n} value={n}>
              {n} {unit}
            </option>
          ))}
      </select>
    </label>
  );
}

/** Settings → Automations: what the Debug Center keeps and for how long. */
export function DebugSettingsSection() {
  const { query, update } = useSettings();
  const status = useDebugStatus();
  const settings = query.data;
  if (!settings) return <Skeleton className="h-32" />;
  const o = settings.observe;
  const set = (patch: Partial<Settings["observe"]>) => update.mutate({ ...settings, observe: { ...o, ...patch } });

  return (
    <Section title={t("Debug Center")}>
      <label className="flex items-start gap-3 py-1.5">
        <Switch checked={o.enabled} onCheckedChange={(enabled) => set({ enabled })} className="mt-0.5" />
        <span>
          <span className="block text-sm">{t("Collect events")}</span>
          <span className="block text-[13px] text-muted-foreground">
            {t("Exceptions, dumps, error logs, 5xx answers and failures from the sites LocDev runs.")}
          </span>
        </span>
      </label>
      <div className="grid grid-cols-2 gap-x-8">
        <Stepper
          label={t("Keep for")}
          value={o.retention_days}
          options={[1, 3, 7, 14, 30]}
          unit={t("days")}
          onChange={(retention_days) => set({ retention_days })}
        />
        <Stepper
          label={t("Use at most")}
          value={o.max_mb}
          options={[25, 50, 100, 250, 500]}
          unit="MB"
          onChange={(max_mb) => set({ max_mb })}
        />
      </div>
      <div role="radiogroup" aria-label={t("Log lines from")} className="mt-2 flex items-center gap-2 text-sm">
        <span className="mr-2 text-muted-foreground">{t("Plain log lines from")}</span>
        {FROM.map((f) => (
          <button
            type="button"
            role="radio"
            key={f.id}
            aria-checked={o.logs_from === f.id}
            onClick={() => set({ logs_from: f.id })}
            className={cn(
              "rounded-full border px-2.5 py-0.5 text-[12.5px] text-muted-foreground",
              o.logs_from === f.id && "border-ember/60 bg-ember/8 text-foreground",
            )}
          >
            {f.label}
          </button>
        ))}
      </div>
      <p className="mt-3 text-[13px] text-muted-foreground">
        {t(
          "Older days are deleted first when either limit is reached. Secrets are hidden before anything is stored: .env values, LocDev's secrets, auth headers, cookies, tokens and passwords in URLs and JSON.",
        )}
        {status.data && ` ${t("Using {mb} MB now.", { mb: (status.data.disk_bytes / 1_048_576).toFixed(1) })}`}
      </p>
    </Section>
  );
}
