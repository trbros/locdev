// Third-party software inside LocDev, for Settings → About: the Rust crates
// the app, daemon and CLI link (every platform, no build/dev-only ones) and
// the npm packages the app depends on at runtime, with their licenses as
// their authors declare them. Writes src/generated/third-party.json;
// `--check` fails if that file is out of date (run by `make check`).
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// fileURLToPath: `.pathname` is `/C:/…` on Windows.
const root = fileURLToPath(new URL("../../../", import.meta.url));
const out = fileURLToPath(new URL("../src/generated/third-party.json", import.meta.url));

const tree = execFileSync(
  "cargo",
  [
    "tree",
    "--locked",
    "-e",
    "normal",
    "--target",
    "all",
    "--prefix",
    "none",
    "-f",
    "{p}|{l}|{r}",
    "-p",
    "locdev-desktop",
    "-p",
    "locdev-daemon",
    "-p",
    "locdev",
  ],
  { cwd: root, encoding: "utf8", maxBuffer: 64 << 20 },
);
const rust = new Map();
for (const line of tree.split("\n")) {
  const [p, license, repository] = line.split("|");
  if (!p) continue;
  // Workspace and path crates are LocDev itself.
  if (p.includes("(/") || p.includes("(\\")) continue;
  const m = /^(\S+) v(\S+)/.exec(p);
  if (!m) continue;
  rust.set(`${m[1]}@${m[2]}`, {
    name: m[1],
    version: m[2],
    license: license?.trim() || "unknown",
    url: repository?.replace(/ \(\*\)$/, "").trim() || null,
  });
}

const npmRaw = JSON.parse(
  execFileSync("pnpm", ["licenses", "list", "--prod", "--json"], { encoding: "utf8", maxBuffer: 64 << 20 }),
);
const npm = new Map();
for (const [license, packages] of Object.entries(npmRaw)) {
  for (const p of packages) {
    for (const version of p.versions) {
      npm.set(`${p.name}@${version}`, { name: p.name, version, license, url: p.homepage ?? null });
    }
  }
}

const sorted = (map) => [...map.values()].sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
const text = `${JSON.stringify({ rust: sorted(rust), npm: sorted(npm) }, null, 1)}\n`;

if (process.argv.includes("--check")) {
  let current = "";
  try {
    current = readFileSync(out, "utf8");
  } catch {}
  if (current !== text) {
    console.error("src/generated/third-party.json is out of date: run `make third-party`");
    process.exit(1);
  }
  console.log(`third-party.json: ${rust.size} crates, ${npm.size} npm packages, up to date`);
} else {
  writeFileSync(out, text);
  console.log(`wrote third-party.json: ${rust.size} crates, ${npm.size} npm packages`);
}
