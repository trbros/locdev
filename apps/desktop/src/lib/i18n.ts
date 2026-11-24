// The app's languages. English is the source: `t("Add site")` looks the
// English text up in the active language's catalog (src/i18n/<locale>.json)
// and falls back to it, so a missing translation shows English, never a key.
// The language is fixed when the app loads; changing it reloads the window,
// so lists built at import time (navigation, labels) come out translated too.
// `pnpm test:i18n` fails when a catalog misses a string the code uses.

import es from "@/i18n/es.json";
import ja from "@/i18n/ja.json";
import ptBR from "@/i18n/pt-BR.json";

export type Locale = "en" | "pt-BR" | "es" | "ja";
export type LanguagePref = "system" | Locale;

export const LOCALES: { id: Locale; label: string }[] = [
  { id: "en", label: "English" },
  { id: "pt-BR", label: "Português (Brasil)" },
  { id: "es", label: "Español" },
  { id: "ja", label: "日本語" },
];

const CATALOGS: Record<Locale, Record<string, string>> = { en: {}, "pt-BR": ptBR, es, ja };
const KEY = "locdev.language";

function stored(): LanguagePref {
  try {
    const value = localStorage.getItem(KEY);
    if (value === "system" || LOCALES.some((l) => l.id === value)) return value as LanguagePref;
  } catch {
    // Storage blocked: follow the system.
  }
  return "system";
}

/** The system's language, when it's one the app speaks. */
export function systemLocale(): Locale {
  const wanted = typeof navigator === "undefined" ? [] : (navigator.languages ?? [navigator.language]);
  for (const tag of wanted) {
    const lower = tag.toLowerCase();
    if (lower.startsWith("pt")) return "pt-BR";
    if (lower.startsWith("es")) return "es";
    if (lower.startsWith("ja")) return "ja";
    if (lower.startsWith("en")) return "en";
  }
  return "en";
}

function fromQuery(): Locale | null {
  // `?lang=ja` (screenshots of the docs in each language).
  if (typeof location === "undefined") return null;
  const value = new URLSearchParams(location.search).get("lang");
  return LOCALES.some((l) => l.id === value) ? (value as Locale) : null;
}

export const languagePref: LanguagePref = stored();
export const locale: Locale = fromQuery() ?? (languagePref === "system" ? systemLocale() : languagePref);
const catalog = CATALOGS[locale];

if (typeof document !== "undefined") document.documentElement.lang = locale;

/** The text in the app's language; `{name}` placeholders are filled from `vars`. */
export type Vars = Record<string, string | number | null | undefined>;

export function t(text: string, vars?: Vars): string {
  const out = catalog[text] || text;
  if (!vars) return out;
  // null/undefined render as nothing, as they would in JSX.
  return out.replace(/\{(\w+)\}/g, (whole, key: string) => (key in vars ? String(vars[key] ?? "") : whole));
}

/** Singular or plural by count: `tn("{n} site", "{n} sites", n)`. */
export function tn(one: string, many: string, n: number, vars?: Vars): string {
  const plural = new Intl.PluralRules(locale).select(n) === "one" ? one : many;
  return t(plural, { n, ...vars });
}

/** Dates, numbers and relative times in the app's language. */
export const intlLocale = locale;

export function setLanguage(pref: LanguagePref) {
  try {
    localStorage.setItem(KEY, pref);
  } catch {
    // Nothing to keep it in; it still applies after the reload below.
  }
  location.reload();
}
