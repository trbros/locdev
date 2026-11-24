import "server-only";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export type ChangelogEntry = {
	version: string;
	/** As written: `2026-11-03`, or a range for the first version. */
	date: string | null;
	released: boolean;
	body: string;
};

/** The repository's localized changelog (copied to generated/ by `make cli-catalog`;
 *  a test fails when the copy differs). Versions marked "(internal testing)"
 *  were tested internally and with selected users; no public release. */
export async function getChangelog(locale = "en"): Promise<ChangelogEntry[]> {
	const suffix =
		locale === "pt-BR" || locale === "es" || locale === "ja"
			? `.${locale}`
			: "";
	const text = await readFile(
		join(process.cwd(), "generated", `changelog${suffix}.md`),
		"utf8",
	);
	return text
		.split(/^## /m)
		.slice(1)
		.map((chunk) => {
			const [head, ...rest] = chunk.split("\n");
			const version = /^\[([^\]]+)\]/.exec(head)?.[1] ?? head.trim();
			const date = /\] - ([^(]+?)\s*(?:\(|$)/.exec(head)?.[1]?.trim() ?? null;
			return {
				version,
				date,
				released:
					!/internal testing|not released/i.test(head) &&
					version !== "Unreleased",
				body: rest.join("\n").trim(),
			};
		});
}
