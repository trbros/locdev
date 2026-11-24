import { defineRouting } from "next-intl/routing";
import { locales } from "./locales";

// Every language has its prefix (/pt-BR, /en, /es, /ja). The root picks one
// from the saved choice (NEXT_LOCALE cookie, set when the user switches),
// then the browser, then Brazilian Portuguese.
export const routing = defineRouting({
	locales,
	defaultLocale: "pt-BR",
	localePrefix: "always",
	localeDetection: true,
});
