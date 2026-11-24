import { readFile, writeFile } from "node:fs/promises";

// Root files are authoritative; both products ship byte-identical copies.
const root = new URL("../../../", import.meta.url);
const check = process.argv.includes("--check");
for (const suffix of ["", ".pt-BR", ".es", ".ja"]) {
	const source = await readFile(new URL(`CHANGELOG${suffix}.md`, root), "utf8");
	for (const target of [
		`apps/docs/generated/changelog${suffix}.md`,
		`apps/desktop/src/generated/changelog${suffix}.md`,
	]) {
		const url = new URL(target, root);
		if (check) {
			const current = await readFile(url, "utf8").catch((error) => {
				if (error.code === "ENOENT") return null;
				throw error;
			});
			if (current !== source) {
				console.error(
					`${target} is stale: node apps/docs/scripts/sync-changelogs.mjs`,
				);
				process.exitCode = 1;
			}
		} else {
			await writeFile(url, source, "utf8");
		}
	}
}
