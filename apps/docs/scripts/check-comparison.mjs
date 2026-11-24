// Checks the LocDev column of the /compare matrix against the repository:
// every LocDev source link must point to a file that exists, and features the
// code already ships can't be listed as planned or missing. It reads files
// only; competitors still need a manual review of their official sources.
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { groups } from "../lib/comparison-matrix.ts";

const blob = "https://github.com/trbros/locdev/blob/main/";

// Feature id → a file whose presence proves LocDev ships it.
export const shippedBy = {
	git: "crates/locdev-git/src/lib.rs",
	github: "crates/locdev-daemon/src/daemon/github.rs",
	actions: "crates/locdev-actions/src/lib.rs",
	emailstudio: "crates/locdev-email/src/lib.rs",
	debugcenter: "crates/locdev-daemon/src/daemon/debug.rs",
	redaction: "crates/locdev-observe/src/redact.rs",
	dumps: "crates/locdev-daemon/src/daemon/debug.rs",
	dbbrowser: "crates/locdev-db/src/lib.rs",
	tunnels: "crates/locdev-daemon/src/daemon/tunnels.rs",
	deploy: "crates/locdev-deploy/src/remote/mod.rs",
	mysql: "crates/locdev-services/src/releases.rs",
	postgres: "crates/locdev-services/src/releases.rs",
	redis: "crates/locdev-services/src/releases.rs",
	mail: "crates/locdev-mail/src/smtp.rs",
	env: "crates/locdev-env/src/vault.rs",
	websocket: "crates/locdev-proxy/src/lib.rs",
	subdomains: "crates/locdev-core/src/subdomains.rs",
	diagnostics: "crates/locdev-daemon/src/doctor.rs",
	creator: "crates/locdev-scaffold/src/catalog.rs",
	static: "crates/locdev-proxy/src/files.rs",
	worktrees: "crates/locdev-daemon/src/daemon/git.rs",
	metrics: "crates/locdev-proxy/src/metrics.rs",
	tinker: "crates/locdev-daemon/src/daemon/tinker.rs",
	phpext: "crates/locdev-daemon/src/daemon/extensions.rs",
	workflowaudit: "crates/locdev-actions/src/validate.rs",
	installers: "crates/locdev-scaffold/src/tools.rs",
	opensource: "LICENSE",
};

export function verifyComparisonEvidence(repoRoot) {
	const issues = [];
	const features = groups.flatMap((group) => group.features);
	for (const feature of features) {
		const cell = feature.cells.locdev;
		if (cell.source?.startsWith(blob)) {
			const file = cell.source.slice(blob.length);
			if (!existsSync(resolve(repoRoot, file)))
				issues.push(`${feature.id}: source ${file} does not exist`);
		}
		const proof = shippedBy[feature.id];
		if (
			proof &&
			existsSync(resolve(repoRoot, proof)) &&
			["planned", "no", "unknown"].includes(cell.status)
		)
			issues.push(
				`${feature.id}: LocDev ships it (${proof}) but the matrix says ${cell.status}`,
			);
	}
	const frameworks = readFileSync(
		resolve(repoRoot, "crates/locdev-frameworks/definitions/frameworks.toml"),
		"utf8",
	);
	const python = features.find((f) => f.id === "python")?.cells.locdev;
	if (/^id = "django"$/m.test(frameworks) && python?.status === "no")
		issues.push(
			"python: Django is a framework definition but the matrix says no",
		);
	return issues;
}

if (
	process.argv[1] &&
	resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
	const issues = verifyComparisonEvidence(resolve(process.argv[2] || "../.."));
	for (const issue of issues) console.error(`REVIEW REQUIRED: ${issue}`);
	if (issues.length) {
		console.error(
			"Update lib/comparison-matrix.ts and the notes in messages/*.json from the real code, in every language.",
		);
		process.exitCode = 1;
	} else
		console.log(
			"The LocDev column matches the repository. Competitors still need a manual review of their sources.",
		);
}
