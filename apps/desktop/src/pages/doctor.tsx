import { Copy, RefreshCw } from "lucide-react";

import { Page, Section, useCrumbs } from "@/components/page";
import { Led, levelTone, toneText } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useDaemonControl, useDaemonDown, useDoctor } from "@/hooks/use-daemon";
import { subById, viewById } from "@/lib/nav";
import type { Check, CheckLevel } from "@/lib/protocol";
import { copyText } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t, tn } from "@/lib/i18n";

const ORDER: Record<CheckLevel, number> = { error: 0, warn: 1, info: 2, ok: 3 };
const VERDICT: Record<CheckLevel, string> = { ok: "pass", info: "note", warn: "warn", error: "fail" };

export function DoctorView() {
  const view = viewById("tools");
  const sub = subById(view, "doctor")!;
  const crumbs = useCrumbs(view, sub);
  const doctor = useDoctor();
  const down = useDaemonDown();
  const { start } = useDaemonControl();

  const checks = [...(doctor.data ?? [])].sort((a, b) => ORDER[a.level] - ORDER[b.level]);
  const count = (level: CheckLevel) => checks.filter((c) => c.level === level).length;
  const attention = count("error") + count("warn");

  return (
    <Page
      view={view}
      title={sub.label}
      description={sub.description}
      crumbs={crumbs}
      actions={
        <Button variant="outline" size="sm" onClick={() => doctor.refetch()} disabled={doctor.isFetching || down}>
          <RefreshCw data-icon="inline-start" className={cn(doctor.isFetching && "animate-spin")} /> {t("Re-run")}
        </Button>
      }
    >
      {down ? (
        <div className="rounded-lg border border-destructive/30 bg-destructive/6 p-5">
          <div className="flex items-center gap-2 font-mono text-sm text-destructive">
            <Led tone="fail" /> {t("locdevd is offline")}
          </div>
          <p className="mt-1.5 text-sm text-muted-foreground">
            {t("Checks run inside the daemon. Start it to diagnose.")}
          </p>
          <Button size="sm" className="mt-4" onClick={() => start.mutate()} disabled={start.isPending}>
            {t("Start locdevd")}
          </Button>
        </div>
      ) : doctor.isPending ? (
        <div className="flex flex-col gap-2">
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <Skeleton key={i} className="h-9" />
          ))}
        </div>
      ) : (
        <>
          {/* Summary as a single scoreboard line. */}
          <div className="flex items-center gap-5 font-mono text-[12px]">
            <span className={cn("text-sm", attention ? "text-amber" : "text-phosphor")}>
              {attention ? tn("{n} needs attention", "{n} need attention", attention) : t("all clear")}
            </span>
            <span className="h-4 w-px bg-border" />
            {(["ok", "info", "warn", "error"] as const).map((level) => (
              <span key={level} className="flex items-center gap-1.5 text-muted-foreground">
                <Led tone={levelTone(level)} />
                {count(level)} {VERDICT[level]}
              </span>
            ))}
          </div>

          <Section title={t("checks")}>
            <ol>
              {checks.map((check, i) => (
                <CheckRow key={check.id} check={check} index={i} />
              ))}
            </ol>
          </Section>
        </>
      )}
    </Page>
  );
}

function CheckRow({ check, index }: { check: Check; index: number }) {
  const tone = levelTone(check.level);

  return (
    <li
      className="rise flex gap-4 border-b border-dashed py-3 last:border-b-0"
      style={{ "--i": index } as React.CSSProperties}
    >
      <span className={cn("mt-0.5 w-10 shrink-0 font-mono text-[10.5px] tracking-wide uppercase", toneText(tone))}>
        {VERDICT[check.level]}
      </span>
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{check.label}</div>
        <div className="mt-0.5 text-[13px] wrap-break-word text-muted-foreground" data-selectable>
          {check.detail}
        </div>
        {check.remedy && (
          <button
            type="button"
            onClick={() => copyText(check.remedy!)}
            className="group mt-2 flex max-w-full items-center gap-2 rounded-md border bg-well px-2.5 py-1.5 text-left font-mono text-[11.5px] transition-colors hover:border-ember/40"
          >
            <span className="text-ember">$</span>
            <span className="truncate" data-selectable>
              {check.remedy}
            </span>
            <Copy className="size-3 shrink-0 text-muted-foreground group-hover:text-foreground" />
          </button>
        )}
      </div>
    </li>
  );
}
