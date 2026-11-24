import { existsSync, readFileSync } from "node:fs";
import { withNamedRoutes } from "@triooo/named-routes/next";
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";
import { routing } from "./i18n/routing";

// The version shown is the app's (apps/desktop/package.json), the one the
// GitHub release tag (v<version>) follows. NEXT_PUBLIC_APP_VERSION still
// overrides it. A copy of the site deployed on its own has no apps/desktop:
// it carries that version in generated/version.json instead.
function releaseVersion(): string {
	const packagePath = new URL("../desktop/package.json", import.meta.url);
	if (!existsSync(packagePath)) {
		const copied = JSON.parse(
			readFileSync(
				new URL("./generated/version.json", import.meta.url),
				"utf8",
			),
		)?.app;
		if (!copied) throw new Error("No app version in generated/version.json");
		return copied;
	}
	const version = JSON.parse(readFileSync(packagePath, "utf8"))?.version;
	if (!version) throw new Error("No version in apps/desktop/package.json");
	return version;
}

const nextConfig: NextConfig = {
	reactStrictMode: true,
	poweredByHeader: false,
	env: {
		NEXT_PUBLIC_APP_VERSION:
			process.env.NEXT_PUBLIC_APP_VERSION?.trim() || releaseVersion(),
	},
};

const withNextIntl = createNextIntlPlugin("./i18n/request.ts");
// Typed route names from app/ (docs, docs.$slug…), regenerated on dev and
// build into lib/routes.generated.ts; the [locale] folder is dropped from the
// names and next-intl's Link adds the locale.
export default withNamedRoutes(withNextIntl(nextConfig), {
	i18n: routing,
	output: "lib/routes.generated.ts",
});
