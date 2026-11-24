import { Pause, Play, RotateCcw, X } from "lucide-react";
import { useState } from "react";

import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { useJobControl } from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import { JOB_LABEL, type Job, type JobStatus } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const TONE: Record<JobStatus, Tone> = {
  pending: "off",
  running: "info",
  completed: "live",
  failed: "fail",
  cancelled: "warn",
  paused: "off",
};

/** Jobs with their state, progress and controls; click one for its log. */
export function JobList({ jobs, empty = t("No jobs yet.") }: { jobs: Job[]; empty?: string }) {
  const control = useJobControl();
  const [open, setOpen] = useState<number | null>(null);
  if (!jobs.length) return <p className="py-2 text-sm text-muted-foreground">{empty}</p>;

  return (
    <ul>
      {jobs.map((job) => {
        const when = job.finished_at ?? job.started_at ?? job.created_at;
        const due = job.status === "pending" && job.scheduled_at > Date.now();
        return (
          <li key={job.id} className="border-b border-dashed last:border-b-0">
            <div className="group flex items-center gap-3 py-2">
              <Led tone={TONE[job.status]} live={job.status === "running"} />
              <button
                type="button"
                onClick={() => setOpen(open === job.id ? null : job.id)}
                className="min-w-0 flex-1 text-left"
              >
                <span className="text-[13.5px]">
                  {JOB_LABEL[job.kind]}
                  {job.project && <span className="font-mono text-[12.5px]"> · {job.project}</span>}
                </span>
                <span className="block truncate font-mono text-[11px] text-muted-foreground">
                  #{job.id} · {job.status}
                  {job.automatic && t(" · automatic")}
                  {job.progress && ` · ${job.progress.done}/${job.progress.total ?? "?"} ${job.progress.label}`}
                  {due
                    ? t(" · in {time}", { time: uptime(Date.now(), job.scheduled_at) })
                    : t(" · {when} ago", { when: uptime(when) })}
                  {job.attempts > 1 &&
                    t(" · attempt {attempts}/{max_attempts}", {
                      attempts: job.attempts,
                      max_attempts: job.max_attempts,
                    })}
                  {job.result && ` · ${job.result}`}
                  {job.error && ` · ${job.error}`}
                </span>
              </button>
              <span className="flex shrink-0 gap-0.5 opacity-60 transition-opacity group-hover:opacity-100">
                {job.status === "pending" && (
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("Pause")}
                    onClick={() => control.mutate({ id: job.id, action: "pause" })}
                  >
                    <Pause />
                  </Button>
                )}
                {job.status === "paused" && (
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("Resume")}
                    onClick={() => control.mutate({ id: job.id, action: "resume" })}
                  >
                    <Play />
                  </Button>
                )}
                {(job.status === "failed" || job.status === "cancelled") && (
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("Retry")}
                    onClick={() => control.mutate({ id: job.id, action: "retry" })}
                  >
                    <RotateCcw />
                  </Button>
                )}
                {["pending", "paused", "running"].includes(job.status) && (
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label={t("Cancel")}
                    onClick={() => control.mutate({ id: job.id, action: "cancel" })}
                  >
                    <X />
                  </Button>
                )}
              </span>
            </div>
            {open === job.id && (
              <pre
                className={cn(
                  "mb-2 max-h-56 overflow-auto rounded-md bg-well px-3 py-2 font-mono text-[11.5px] leading-relaxed",
                )}
                data-selectable
              >
                {job.log.length ? job.log.join("\n") : t("no log")}
              </pre>
            )}
          </li>
        );
      })}
    </ul>
  );
}
