/** Public/file identifier `br`; valid Portuguese language tag for Intl and SEO. */
export const locales = ["pt-BR", "en", "es", "ja"] as const;
export type AppLocale = (typeof locales)[number];
export const languageTags: Record<AppLocale, string> = {
	"pt-BR": "pt-BR",
	en: "en",
	es: "es",
	ja: "ja",
};
export function localeFileKey(locale: string) {
	return locale === "pt-BR" ? "br" : locale;
}
/** Every locale is prefixed, Brazilian Portuguese included (`/pt-BR/docs`). */
export function localizedPathname(pathname: string, locale: AppLocale) {
	const path = pathname === "/" ? "" : pathname;
	return `/${locale}${path}`;
}
