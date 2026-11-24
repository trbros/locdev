import { existsSync } from "node:fs";
import { join } from "node:path";

export type ShotTheme = "dark" | "light";

const DIR = join(process.cwd(), "public", "screenshots");

/** A capture's URL in the page's language (apps/desktop/scripts/screenshots.mjs
 *  writes public/screenshots/<locale>/<theme>/<name>.webp), English when that
 *  language has none, or null when it doesn't exist at all. */
export function shotUrl(
	locale: string,
	theme: ShotTheme,
	name: string,
): string | null {
	for (const dir of [locale, "en"]) {
		if (existsSync(join(DIR, dir, theme, `${name}.webp`)))
			return `/screenshots/${dir}/${theme}/${name}.webp`;
	}
	return null;
}
