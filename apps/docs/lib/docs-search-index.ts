import "server-only";
import { cache } from "react";
import { docGroups, getDoc } from "./docs";
import { getDocLabels } from "./docs-labels";
import { createDocSearchEntries } from "./docs-search";

export const getDocsSearchIndex = cache(async (locale: string) => {
	const labels = getDocLabels(locale);
	const groups = await Promise.all(
		docGroups.map(async (group) =>
			Promise.all(
				group.slugs.map(async (slug) =>
					createDocSearchEntries(await getDoc(locale, slug), {
						slug,
						title: labels.titles[slug],
						group: labels.groups[group.id],
						description: labels.descriptions[slug],
					}),
				),
			),
		),
	);
	return groups.flat(2);
});
