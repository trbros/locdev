import { Languages } from "lucide-react";

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { type LanguagePref, LOCALES, languagePref, locale, setLanguage, systemLocale, t } from "@/lib/i18n";

/** The app's language; changing it reloads the window (see lib/i18n.ts). */
export function LanguageSelector() {
  const system = LOCALES.find((l) => l.id === systemLocale())?.label ?? "English";
  const items = [
    { value: "system", label: t("System ({language})", { language: system }) },
    ...LOCALES.map((l) => ({ value: l.id, label: l.label })),
  ];
  return (
    <div className="flex flex-col gap-2">
      <Select
        items={items}
        value={languagePref}
        onValueChange={(v) => v && v !== languagePref && setLanguage(v as LanguagePref)}
      >
        <SelectTrigger size="sm" aria-label={t("Language")} className="h-8 w-64">
          <Languages className="size-3.5 text-muted-foreground" />
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {items.map((i) => (
            <SelectItem key={i.value} value={i.value}>
              {i.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-[12px] text-muted-foreground">
        {t("The window reloads to apply it. Messages from the daemon (errors, checks) stay in English.")}{" "}
        <span className="font-mono">{locale}</span>
      </p>
    </div>
  );
}
