import { Monitor, Moon, Sun } from "lucide-react";
import { useId } from "react";
import { t } from "@/lib/i18n";

const THEMES = [
  { id: "system", label: t("System"), icon: Monitor },
  { id: "light", label: t("Paper"), icon: Sun },
  { id: "dark", label: t("Carbon"), icon: Moon },
] as const;

type ThemeId = (typeof THEMES)[number]["id"];

type InterfaceThemeSelectorProps = {
  value: string | undefined;
  onChange: (theme: ThemeId) => void;
};

// Tokens reais de apps/desktop/src/index.css.
// São fixos para que os previews não mudem de aparência
// quando o usuário troca o tema do aplicativo.

const PREVIEW_TOKENS = {
  light: {
    background: "oklch(0.975 0.006 85)",
    card: "oklch(0.993 0.004 85)",
    chrome: "oklch(0.935 0.01 78)",
    well: "oklch(0.955 0.008 82)",
    foreground: "oklch(0.24 0.012 60)",
    muted: "oklch(0.5 0.016 65)",
    border: "oklch(0.24 0.012 60 / 11%)",
    ember: "oklch(0.55 0.17 45)",
    phosphor: "oklch(0.58 0.15 150)",
  },
  dark: {
    background: "oklch(0.175 0.006 60)",
    card: "oklch(0.205 0.007 60)",
    chrome: "oklch(0.155 0.006 60)",
    well: "oklch(0.145 0.005 60)",
    foreground: "oklch(0.93 0.01 80)",
    muted: "oklch(0.66 0.016 70)",
    border: "oklch(0.93 0.01 80 / 8%)",
    ember: "oklch(0.73 0.16 48)",
    phosphor: "oklch(0.8 0.17 148)",
  },
} as const;

type PreviewMode = keyof typeof PREVIEW_TOKENS;

function ThemePreview({ mode }: { mode: PreviewMode }) {
  const colors = PREVIEW_TOKENS[mode];

  return (
    <div
      className="relative h-full min-w-0 flex-1 overflow-hidden"
      style={{ backgroundColor: colors.background }}
      aria-hidden="true"
    >
      {/* Barra superior */}
      <div
        className="absolute inset-x-0 top-0 flex h-2.75 items-center gap-0.75 border-b px-1.25"
        style={{
          backgroundColor: colors.chrome,
          borderColor: colors.border,
        }}
      >
        <span className="size-0.75 rounded-full" style={{ backgroundColor: colors.muted }} />
        <span className="size-0.75 rounded-full" style={{ backgroundColor: colors.muted }} />
        <span className="size-0.75 rounded-full" style={{ backgroundColor: colors.phosphor }} />
      </div>

      {/* Sidebar */}
      <div
        className="absolute bottom-0 left-0 top-2.75 w-[26%] border-r px-1 py-1.5"
        style={{
          backgroundColor: colors.chrome,
          borderColor: colors.border,
        }}
      >
        <div className="mb-1.5 h-0.75 w-[75%] rounded-full" style={{ backgroundColor: colors.muted }} />

        <div className="mb-1.25 h-1.25 w-full rounded-xs" style={{ backgroundColor: colors.ember }} />

        <div className="mb-1.25 h-0.75 w-[85%] rounded-full" style={{ backgroundColor: colors.muted, opacity: 0.5 }} />

        <div className="h-0.75 w-[65%] rounded-full" style={{ backgroundColor: colors.muted, opacity: 0.5 }} />
      </div>

      {/* Conteúdo principal */}
      <div className="absolute bottom-1.25 left-[31%] right-1.25 top-4.25">
        <div
          className="mb-1.25 h-1 w-[54%] rounded-full"
          style={{ backgroundColor: colors.foreground, opacity: 0.7 }}
        />

        <div className="mb-2 h-0.75 w-[85%] rounded-full" style={{ backgroundColor: colors.muted, opacity: 0.5 }} />

        {/* Card simulado */}
        <div
          className="rounded-[3px] border p-1"
          style={{
            backgroundColor: colors.card,
            borderColor: colors.border,
          }}
        >
          <div className="mb-1 h-0.75 w-[65%] rounded-full" style={{ backgroundColor: colors.muted, opacity: 0.6 }} />

          <div className="h-1.5 w-[80%] rounded-xs" style={{ backgroundColor: colors.ember }} />
        </div>
      </div>
    </div>
  );
}

export function InterfaceThemeSelector({ value, onChange }: InterfaceThemeSelectorProps) {
  const groupId = useId();
  const selectedTheme = value ?? "system";

  return (
    <div className="space-y-3">
      <div className="space-y-1">
        <h3 id={groupId} className="text-[13px] font-medium text-foreground">
          {t("Interface theme")}
        </h3>

        <p className="text-[12px] text-muted-foreground">{t("Customize your workspace appearance.")}</p>
      </div>

      <div role="radiogroup" aria-labelledby={groupId} className="grid max-w-105 grid-cols-3 gap-3">
        {THEMES.map(({ id, label, icon: Icon }) => {
          const selected = selectedTheme === id;

          return (
            <label key={id} className="group relative flex min-w-0 cursor-pointer flex-col gap-2">
              <input
                type="radio"
                name={groupId}
                value={id}
                checked={selected}
                onChange={() => onChange(id)}
                className="peer sr-only"
              />

              {/* Miniatura */}
              <span
                className={[
                  "relative flex h-18 w-full overflow-hidden rounded-[6px] border transition-all duration-150",
                  "peer-focus-visible:outline-2 peer-focus-visible:outline-offset-3 peer-focus-visible:outline-ember",
                  selected ? "border-ember ring-1 ring-ember/50" : "border-border group-hover:border-ember/50",
                ].join(" ")}
              >
                {id === "system" ? (
                  <>
                    <ThemePreview mode="light" />
                    <ThemePreview mode="dark" />
                    <div className="pointer-events-none absolute inset-y-0 left-1/2 w-px bg-foreground/10" />
                  </>
                ) : (
                  <ThemePreview mode={id} />
                )}

                {selected && (
                  <span className="absolute bottom-1 right-1 flex size-3.5 items-center justify-center rounded-full bg-ember text-ember-foreground shadow-sm">
                    <svg viewBox="0 0 16 16" fill="none" className="size-2.5" aria-hidden="true">
                      <path
                        d="m3.5 8 3 3 6-6"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  </span>
                )}
              </span>

              {/* Nome abaixo da miniatura */}
              <span
                className={[
                  "flex items-center justify-center gap-1.5 text-center text-[11px] transition-colors",
                  selected ? "font-medium text-foreground" : "text-muted-foreground group-hover:text-foreground",
                ].join(" ")}
              >
                <Icon className="size-3" aria-hidden="true" />
                {label}
              </span>
            </label>
          );
        })}
      </div>
    </div>
  );
}
