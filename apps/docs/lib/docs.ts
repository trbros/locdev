import "server-only";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { notFound } from "next/navigation";
import { localeFileKey } from "@/i18n/locales";
import { routing } from "@/i18n/routing";

export const docGroups = [
	{
		id: "start",
		slugs: [
			"introduction",
			"installation",
			"getting-started",
			"app",
			"shortcuts",
		],
	},
	{
		id: "tutorials",
		slugs: ["tutorial-laravel", "tutorial-node", "tutorial-versions"],
	},
	{
		id: "core",
		slugs: [
			"sites",
			"parked-folders",
			"domains",
			"subdomains",
			"https",
			"runtimes",
			"php-settings",
			"frameworks",
			"servers",
		],
	},
	{
		id: "tools",
		slugs: [
			"new-project",
			"editors",
			"debug-center",
			"metrics",
			"tinker",
			"environment",
			"services",
			"database",
			"mailbox",
			"email-studio",
			"config-files",
			"deploy",
			"tunnels",
			"dependencies",
			"git",
			"actions",
		],
	},
	{
		id: "maintenance",
		slugs: ["doctor", "troubleshooting", "updating", "uninstalling"],
	},
	{
		id: "reference",
		slugs: ["cli", "configuration", "architecture", "security", "service"],
	},
] as const;

export const docSlugs = docGroups.flatMap((group) => [...group.slugs]);
export type DocSlug = (typeof docSlugs)[number];
export type DocLocale = (typeof routing.locales)[number];
export function isDocSlug(slug: string): slug is DocSlug {
	return (docSlugs as string[]).includes(slug);
}
export async function getDoc(locale: string, slug: string): Promise<string> {
	if (!routing.locales.includes(locale as DocLocale) || !isDocSlug(slug))
		notFound();
	// Read only from a checked-in list; never interpolate arbitrary user file paths.
	return readFile(
		join(process.cwd(), "content", "docs", localeFileKey(locale), `${slug}.md`),
		"utf8",
	);
}
export function getDocTitle(source: string): string {
	return source.match(/^# (.+)$/m)?.[1] ?? "LocDev";
}
