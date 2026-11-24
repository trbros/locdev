// The CLI (`named-routes generate|check`) reads the same options as
// withNamedRoutes in next.config.ts (locales as in i18n/locales.ts; plain
// .mjs so it runs on every supported Node).
import { defineConfig } from "@triooo/named-routes/config";

export default defineConfig({
	output: "lib/routes.generated.ts",
	i18n: {
		locales: ["pt-BR", "en", "es", "ja"],
		defaultLocale: "pt-BR",
		localePrefix: "always",
	},
});
