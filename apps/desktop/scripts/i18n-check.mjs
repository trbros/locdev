// Every string the app passes to t()/tn() must be in each catalog
// (src/i18n/<locale>.json). A value equal to the English text is fine
// (names, commands); a missing one fails. Also lists keys nobody uses.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

// fileURLToPath: `.pathname` is `/C:/…` on Windows.
const root = fileURLToPath(new URL("../src/", import.meta.url));
const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path);
    else if (/\.(tsx?|mts)$/.test(name) && !/mock/.test(name) && name !== "i18n.ts") files.push(path);
  }
})(root);

const used = new Set();
// "…" or '…' (prettier picks single quotes when the text has double ones).
const literal = String.raw`(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)')`;
const call = new RegExp(String.raw`\btx?\(\s*${literal}`, "g");
const plural = new RegExp(String.raw`\btn\(\s*${literal}\s*,\s*${literal}`, "g");
const text = (double, single) =>
  double !== undefined
    ? JSON.parse(`"${double}"`)
    : JSON.parse(`"${single.replace(/\\'/g, "'").replace(/"/g, '\\"')}"`);
for (const file of files) {
  const src = readFileSync(file, "utf8");
  for (const m of src.matchAll(call)) used.add(text(m[1], m[2]));
  for (const m of src.matchAll(plural)) {
    used.add(text(m[1], m[2]));
    used.add(text(m[3], m[4]));
  }
}

// Texts the daemon sends from its fixed catalogs (frameworks and tools),
// which the app shows through t(): they need translations too.
const generated = (name) => JSON.parse(readFileSync(new URL(`../src/generated/${name}`, import.meta.url), "utf8"));
const TEXT_FIELDS = new Set(["description", "label", "note", "help"]);
(function collect(value, key) {
  if (Array.isArray(value)) value.forEach((v) => collect(v, key));
  else if (value && typeof value === "object") for (const [k, v] of Object.entries(value)) collect(v, k);
  else if (typeof value === "string" && TEXT_FIELDS.has(key) && value.trim()) used.add(value);
})([generated("starters.json"), generated("tools.json")]);

let failed = false;
for (const locale of ["pt-BR", "es", "ja"]) {
  const catalog = JSON.parse(readFileSync(new URL(`../src/i18n/${locale}.json`, import.meta.url), "utf8"));
  const missing = [...used].filter((k) => !(k in catalog));
  const unused = Object.keys(catalog).filter((k) => !used.has(k));
  const placeholders = (s) =>
    [...s.matchAll(/\{(\w+)\}/g)]
      .map((m) => m[1])
      .sort()
      .join(",");
  const broken = [...used].filter((k) => k in catalog && placeholders(k) !== placeholders(catalog[k]));
  if (missing.length || broken.length) failed = true;
  console.log(
    `${locale}: ${used.size} strings, ${missing.length} missing, ${broken.length} with other placeholders, ${unused.length} unused`,
  );
  for (const k of process.argv.includes("--all") ? missing : missing.slice(0, 30))
    console.log(`  missing: ${JSON.stringify(k)}`);
  for (const k of broken.slice(0, 30))
    console.log(`  placeholders differ: ${JSON.stringify(k)} → ${JSON.stringify(catalog[k])}`);
  if (process.argv.includes("--unused")) for (const k of unused) console.log(`  unused: ${JSON.stringify(k)}`);
}
process.exitCode = failed ? 1 : 0;
