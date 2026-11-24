import { Check, Monitor, Sparkles, Turtle } from "lucide-react";
import { useId } from "react";
import type { Prefs } from "@/lib/prefs";
import { t } from "@/lib/i18n";

type MotionMode = Prefs["motion"];

type MotionPreferenceSelectorProps = {
  value: MotionMode | undefined;
  onChange: (value: MotionMode) => void;
};

const OPTIONS = [
  {
    id: "system",
    label: t("System"),
    icon: Monitor,
    description: t("Follow system preference"),
  },
  {
    id: "reduce",
    label: t("Reduced"),
    icon: Turtle,
    description: t("Minimal animation"),
  },
  {
    id: "full",
    label: t("Full"),
    icon: Sparkles,
    description: t("All animations"),
  },
] as const;

function MotionPreview({ mode }: { mode: MotionMode }) {
  const positions = mode === "reduce" ? [50] : mode === "system" ? [26, 74] : [18, 50, 82];

  return (
    <div aria-hidden="true" className="relative h-19 w-full overflow-hidden bg-background">
      {/* Barra superior */}
      <div className="absolute inset-x-0 top-0 flex h-2.75 items-center gap-0.75 border-b border-border bg-chrome px-1.25">
        <span className="size-0.75 rounded-full bg-muted-foreground/40" />
        <span className="size-0.75 rounded-full bg-muted-foreground/40" />
        <span className="size-0.75 rounded-full bg-phosphor" />
      </div>

      {/* Área de conteúdo */}
      <div className="absolute inset-x-2 bottom-1.75 top-4.5">
        <div className="mb-1.5 h-0.75 w-[52%] rounded-full bg-foreground/60" />

        <div className="mb-2 h-0.75 w-[75%] rounded-full bg-muted-foreground/30" />

        {/* Linha da trajetória */}
        <div className="relative h-7.5">
          {mode !== "reduce" && (
            <div className="absolute inset-x-[10%] top-1/2 border-t border-dashed border-muted-foreground/25" />
          )}

          {/* Divisão do modo System */}
          {mode === "system" && <div className="absolute bottom-0 left-1/2 top-0 border-l border-border" />}

          {positions.map((position, index) => {
            const last = index === positions.length - 1;

            return (
              <div
                key={`${mode}-${position}`}
                className={`absolute top-0 flex h-6.25 w-5 flex-col gap-0.75 rounded-[3px] border p-1 ${
                  last ? "border-ember/50 bg-ember/15" : "border-border bg-card"
                }`}
                style={{
                  left: `${position}%`,
                  transform: "translateX(-50%)",
                  opacity: mode === "full" ? [0.3, 0.6, 1][index] : mode === "system" && index === 0 ? 0.55 : 1,
                }}
              >
                <span className={`h-0.75 w-[80%] rounded-full ${last ? "bg-ember" : "bg-muted-foreground/50"}`} />

                <span className="h-0.75 w-full rounded-full bg-muted-foreground/25" />

                <span className="h-0.75 w-[65%] rounded-full bg-muted-foreground/25" />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

export function MotionPreferenceSelector({ value, onChange }: MotionPreferenceSelectorProps) {
  const id = useId();
  const selected = value ?? "system";

  return (
    <fieldset className="space-y-3">
      <legend className="text-[13px] font-medium text-foreground">{t("Motion preference")}</legend>

      <p className="text-[12px] text-muted-foreground">{t("Choose how animations behave throughout LocDev.")}</p>

      <div className="grid max-w-105 grid-cols-3 gap-3">
        {OPTIONS.map((option) => {
          const Icon = option.icon;
          const active = selected === option.id;

          return (
            <label key={option.id} className="group flex min-w-0 cursor-pointer flex-col gap-2">
              <input
                type="radio"
                name={id}
                value={option.id}
                checked={active}
                onChange={() => onChange(option.id)}
                className="peer sr-only"
              />

              {/* Miniatura */}
              <span
                className={`relative block overflow-hidden rounded-[6px]
                  border transition-colors
                  peer-focus-visible:outline-2
                  peer-focus-visible:outline-offset-2
                  peer-focus-visible:outline-ember ${
                    active ? "border-ember ring-1 ring-ember/30" : "border-border group-hover:border-ember/50"
                  }`}
              >
                <MotionPreview mode={option.id} />

                {active && (
                  <span className="absolute bottom-1 right-1 flex size-3.5 items-center justify-center rounded-full bg-ember text-ember-foreground">
                    <Check className="size-2.5" />
                  </span>
                )}
              </span>

              {/* Rótulo */}
              <span
                className={`flex items-center justify-center gap-1
                  text-center text-[11px] ${
                    active ? "font-medium text-foreground" : "text-muted-foreground group-hover:text-foreground"
                  }`}
              >
                <Icon className="size-3" aria-hidden="true" />
                {option.label}
              </span>

              <span className="text-center text-[10px] leading-4 text-muted-foreground">{option.description}</span>
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}
