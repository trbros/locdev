// Builds the daemon and CLI and places them where Tauri expects sidecars:
// src-tauri/binaries/<name>-<target-triple>[.exe]. Installers then ship them
// next to the app executable, where the app (and the launcher) look first.
//
//   node scripts/prepare-sidecars.mjs            # release, host target
//   node scripts/prepare-sidecars.mjs --debug    # debug build (for `tauri dev`)
//
// Tauri sets TAURI_ENV_TARGET_TRIPLE when it runs this as a build hook, so
// `tauri build --target <triple>` cross-builds the sidecars too.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const out = resolve(root, "apps/desktop/src-tauri/binaries");
const debug = process.argv.includes("--debug") || process.env.TAURI_ENV_DEBUG === "true";

const hostTriple = () =>
  execFileSync("rustc", ["-vV"], { encoding: "utf8" })
    .split("\n")
    .find((line) => line.startsWith("host:"))
    .slice("host:".length)
    .trim();

const triple = process.env.TAURI_ENV_TARGET_TRIPLE || hostTriple();
// With an explicit triple (always the case under `tauri build`), build into
// target/<triple>/ like the app itself, so release jobs find everything there.
const withTarget = Boolean(process.env.TAURI_ENV_TARGET_TRIPLE) || triple !== hostTriple();
const exe = triple.includes("windows") ? ".exe" : "";

const args = ["build", "-p", "locdev-daemon", "-p", "locdev"];
if (!debug) args.push("--release");
if (withTarget) args.push("--target", triple);

console.log(`[sidecars] cargo ${args.join(" ")}`);
execFileSync("cargo", args, { cwd: root, stdio: "inherit" });

const built = join(root, "target", withTarget ? triple : "", debug ? "debug" : "release");
mkdirSync(out, { recursive: true });
for (const bin of ["locdevd", "locdev"]) {
  const from = join(built, `${bin}${exe}`);
  const to = join(out, `${bin}-${triple}${exe}`);
  copyFileSync(from, to);
  console.log(`[sidecars] ${to}`);
}
