import { Check, PanelLeft, PanelLeftClose } from "lucide-react";
import { useId } from "react";
import type { Prefs } from "@/lib/prefs";
import { t } from "@/lib/i18n";

type SidebarMode = Prefs["sidebar"];

type SidebarLayoutSelectorProps = {
  value: SidebarMode | undefined;
  onChange: (value: SidebarMode) => void;
};

const OPTIONS = [
  {
    id: "expanded",
    label: t("Icons and names"),
    icon: PanelLeft,
  },
  {
    id: "collapsed",
    label: t("Icons only"),
    icon: PanelLeftClose,
  },
] as const;

function SidebarPreview({ mode }: { mode: SidebarMode }) {
  const expanded = mode === "expanded";

  return (
    <div aria-hidden="true" className="flex h-19 w-full overflow-hidden bg-background">
      {/* Sidebar */}
      <div
        className="flex shrink-0 flex-col gap-1.25 border-r border-border bg-chrome px-1.25 py-1.75"
        style={{ width: expanded ? "38%" : "20%" }}
      >
        {/* Logo */}
        <div className="mb-0.75 flex items-center gap-1">
          <span className="size-1.75 shrink-0 rounded-xs bg-ember" />

          {expanded && <span className="h-1 w-[55%] rounded-full bg-foreground/70" />}
        </div>

        {/* Itens da navegação */}
        {[0, 1, 2, 3].map((index) => (
          <div
            key={index}
            className={`flex h-2.25 items-center gap-1 rounded-xs px-0.5 ${index === 0 ? "bg-ember/15" : ""}`}
          >
            <span
              className={`size-1.25 shrink-0 rounded-[1px] ${index === 0 ? "bg-ember" : "bg-muted-foreground/45"}`}
            />

            {expanded && (
              <span
                className={`h-0.75 rounded-full ${index === 0 ? "bg-ember/80" : "bg-muted-foreground/35"}`}
                style={{
                  width: `${75 - index * 9}%`,
                }}
              />
            )}
          </div>
        ))}
      </div>

      {/* Área principal */}
      <div className="min-w-0 flex-1 px-1.75 py-2.25">
        <div className="mb-1.25 h-1 w-[58%] rounded-full bg-foreground/70" />

        <div className="mb-2.25 h-0.75 w-[82%] rounded-full bg-muted-foreground/35" />

        <div className="rounded-[3px] border border-border bg-card p-1.25">
          <div className="mb-1.25 h-0.75 w-[66%] rounded-full bg-muted-foreground/45" />

          <div className="mb-1 h-1.5 w-[75%] rounded-xs bg-ember" />

          <div className="h-0.75 w-[90%] rounded-full bg-muted-foreground/25" />
        </div>
      </div>
    </div>
  );
}

export function SidebarLayoutSelector({ value, onChange }: SidebarLayoutSelectorProps) {
  const id = useId();
  const selected = value ?? "expanded";

  return (
    <fieldset className="space-y-3">
      <legend id={`${id}-label`} className="text-[13px] font-medium text-foreground">
        {t("Sidebar layout")}
      </legend>

      <p className="text-[12px] text-muted-foreground">{t("Choose how navigation appears in your workspace.")}</p>

      <div className="grid max-w-72.5 grid-cols-2 gap-3">
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

              <span
                className={`relative block overflow-hidden
                  rounded-[6px] border transition-colors
                  peer-focus-visible:outline-2
                  peer-focus-visible:outline-offset-2
                  peer-focus-visible:outline-ember ${
                    active ? "border-ember ring-1 ring-ember/30" : "border-border group-hover:border-ember/50"
                  }`}
              >
                <SidebarPreview mode={option.id} />

                {active && (
                  <span
                    className="absolute bottom-1 right-1 flex size-3.5
                      items-center justify-center rounded-full
                      bg-ember text-ember-foreground"
                  >
                    <Check className="size-2.5" />
                  </span>
                )}
              </span>

              <span
                className={`flex items-center justify-center gap-1.5
                  text-center text-[11px] ${
                    active ? "font-medium text-foreground" : "text-muted-foreground group-hover:text-foreground"
                  }`}
              >
                <Icon className="size-3" aria-hidden="true" />
                {option.label}
              </span>
            </label>
          );
        })}
      </div>
    </fieldset>
  );
}
