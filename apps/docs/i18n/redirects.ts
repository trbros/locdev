import { legacyDocSlugs, legacyPaths } from "../lib/doc-redirects.ts";

const prefixed = ["pt-BR", "en", "es", "ja"];

/**
 * Permanent redirects for URLs published before every locale had a prefix.
 * Unprefixed paths and `/br`, `/pt` were Brazilian Portuguese, so they go to
 * `/pt-BR/...`; old docs slugs and pages map to their current names. The
 * bare root is left to next-intl, which picks the saved or browser language.
 */
export function canonicalRedirect(pathname: string): string | null {
	const parts = pathname.split("/").filter(Boolean);
	if (parts.length === 0) return null;
	let changed = false;
	if (["pt", "br"].includes(parts[0])) {
		parts[0] = "pt-BR";
		changed = true;
	} else if (!prefixed.includes(parts[0])) {
		parts.unshift("pt-BR");
		changed = true;
	}
	const rest = `/${parts.slice(1).join("/")}`;
	const page = legacyPaths[rest as keyof typeof legacyPaths];
	if (page) {
		parts.splice(1, parts.length - 1, ...page.split("/").filter(Boolean));
		changed = true;
	}
	if (parts.length === 3 && parts[1] === "docs") {
		const legacy = parts[2] as keyof typeof legacyDocSlugs;
		if (Object.hasOwn(legacyDocSlugs, legacy)) {
			parts[2] = legacyDocSlugs[legacy];
			changed = true;
		}
	}
	return changed ? `/${parts.join("/")}` : null;
}
