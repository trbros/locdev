import { existsSync, readFileSync } from "node:fs";
import { withNamedRoutes } from "@triooo/named-routes/next";
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";
import { routing } from "./i18n/routing";

// The docs describe the daemon and CLI, so the version shown is the workspace
// version in Cargo.toml. NEXT_PUBLIC_APP_VERSION still overrides it. A copy
// of the site deployed on its own has no Cargo.toml: it carries that version
// in generated/version.json instead.
function workspaceVersion(): string {
	const cargoPath = new URL("../../Cargo.toml", import.meta.url);
	if (!existsSync(cargoPath)) {
		const copied = JSON.parse(
			readFileSync(
				new URL("./generated/version.json", import.meta.url),
				"utf8",
			),
		)?.daemon;
		if (!copied) throw new Error("No daemon version in generated/version.json");
		return copied;
	}
	const cargo = readFileSync(cargoPath, "utf8");
	const version = cargo.match(
		/\[workspace\.package\][^[]*?version = "([^"]+)"/,
	)?.[1];
	if (!version) throw new Error("No [workspace.package] version in Cargo.toml");
	return version;
}

const nextConfig: NextConfig = {
	reactStrictMode: true,
	poweredByHeader: false,
	env: {
		NEXT_PUBLIC_APP_VERSION:
			process.env.NEXT_PUBLIC_APP_VERSION?.trim() || workspaceVersion(),
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
