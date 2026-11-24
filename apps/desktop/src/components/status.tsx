import type { CheckLevel, ProcessState } from "@/lib/protocol";
import { cn } from "@/lib/utils";

export type Tone = "live" | "warn" | "fail" | "ember" | "info" | "off";

const TONE_TEXT: Record<Tone, string> = {
  live: "text-phosphor",
  warn: "text-amber",
  fail: "text-destructive",
  ember: "text-ember",
  info: "text-signal",
  off: "text-muted-foreground",
};

export const toneText = (tone: Tone) => TONE_TEXT[tone];

export const stateTone = (state: ProcessState): Tone =>
  ({ running: "live", stopping: "warn", failed: "fail", stopped: "off", exited: "off" })[state] as Tone;

export const levelTone = (level: CheckLevel): Tone =>
  ({ ok: "live", info: "info", warn: "warn", error: "fail" })[level] as Tone;

/** A status diode. `off` renders as an unlit ring. */
export function Led({ tone, live, className }: { tone: Tone; live?: boolean; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn("led", TONE_TEXT[tone], className)}
      data-off={tone === "off" ? "" : undefined}
      data-live={live ? "" : undefined}
    />
  );
}

/** Mono "LED + label" chip, e.g. `● running`. */
export function StateChip({ tone, live, children }: { tone: Tone; live?: boolean; children: React.ReactNode }) {
  return (
    <span
      className={cn(
        "inline-flex h-6 items-center gap-2 rounded-md border px-2 font-mono text-[11px] tracking-wide lowercase",
        TONE_TEXT[tone],
      )}
    >
      <Led tone={tone} live={live} />
      <span className="text-foreground/85">{children}</span>
    </span>
  );
}
