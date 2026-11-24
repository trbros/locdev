import { getDocHeadings } from "./doc-headings.ts";
import { route } from "./routes.generated.ts";

export type DocSearchEntry = {
	id: string;
	title: string;
	page: string;
	group: string;
	description: string;
	href: string;
};

export function createDocSearchEntries(
	content: string,
	page: { slug: string; title: string; group: string; description: string },
): DocSearchEntry[] {
	const href = route("docs.$slug", { slug: page.slug });
	return [
		{
			id: page.slug,
			title: page.title,
			page: page.title,
			group: page.group,
			description: page.description,
			href,
		},
		...getDocHeadings(content).map((heading) => ({
			id: `${page.slug}#${heading.id}`,
			title: heading.title,
			page: page.title,
			group: page.group,
			description: page.description,
			href: `${href}#${heading.id}`,
		})),
	];
}

export function normalizeSearch(value: string) {
	return value.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase();
}
