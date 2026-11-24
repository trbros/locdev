#!/usr/bin/env node
/**
 * Sincroniza logos reais das definições do LocDev com o catálogo do SVGL.
 * Execução a partir de apps/desktop: node scripts/sync-stack-icons.mjs
 * Dependências de desenvolvimento: @svgr/core e @svgr/plugin-jsx.
 * Nenhum acesso à rede é realizado pelo app em tempo de execução.
 */
import { readFile, writeFile, mkdir, rm, rename, mkdtemp } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { matchLogo, componentName } from './lib/svgl-match.mjs';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = resolve(scriptDir, '..');
const outputDir = resolve(appDir, 'src/components/stack-icons/generated');
const definitionsPath = resolve(appDir, 'src/generated/frameworks.json');
const args = new Set(process.argv.slice(2));
const dryRun = args.has('--dry-run');
const strict = args.has('--strict');
const catalogFile = [...args].find((arg) => arg.startsWith('--catalog='))?.slice('--catalog='.length);

const ALIASES = { next: 'nextjs', node: 'nodejs', static: 'html' };
const unique = (xs) => [...new Set(xs)];

async function request(url, type = 'text') {
  let lastError;
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const response = await fetch(url, {
        headers: { 'User-Agent': 'LocDev-SVGL-Generator/1.0', Accept: type === 'json' ? 'application/json' : 'image/svg+xml' },
        signal: AbortSignal.timeout(20_000),
      });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText} (${url})`);
      return type === 'json' ? await response.json() : await response.text();
    } catch (error) {
      lastError = error;
      if (attempt < 2) await new Promise((r) => setTimeout(r, 500 * (attempt + 1)));
    }
  }
  throw lastError;
}

function assertSvg(text, url) {
  if (!/<svg[\s>]/i.test(text)) throw new Error(`Not an SVG: ${url}`);
  if (/<script\b|<foreignObject\b|\bon[a-z]+\s*=|javascript:/i.test(text)) {
    throw new Error(`Unsafe SVG attributes/elements: ${url}`);
  }
  if (text.length > 250_000) throw new Error(`SVG unexpectedly large: ${url}`);
  return text;
}

async function main() {
  const snapshot = JSON.parse(await readFile(definitionsPath, 'utf8'));
  const definitions = snapshot.definitions;
  if (!Array.isArray(definitions) || !definitions.every((d) => d.id && d.name)) {
    throw new Error('Invalid frameworks.json: expected definitions[] with id and name');
  }
  const catalog = catalogFile
    ? JSON.parse(await readFile(resolve(process.cwd(), catalogFile), 'utf8'))
    : await request('https://api.svgl.app', 'json');
  if (!Array.isArray(catalog)) throw new Error('SVGL catalog response is not an array');

  const selection = definitions.map((definition) => ({
    ...definition,
    match: matchLogo(definition, catalog),
  }));
  const missing = selection.filter((entry) => !entry.match).map((entry) => `${entry.id} (${entry.name})`);
  console.log(`LocDev: ${definitions.length} definitions · SVGL matches: ${definitions.length - missing.length} · missing: ${missing.length}`);
  if (missing.length) console.log('Sem correspondência oficial no SVGL:\n - ' + missing.join('\n - '));
  if (dryRun) return;
  if (strict && missing.length) throw new Error('Strict mode: some brand icons are not in SVGL; no files changed');

  const { transform } = await import('@svgr/core');
  const jsxPlugin = (await import('@svgr/plugin-jsx')).default;
  const stagingRoot = await mkdtemp(join(dirname(outputDir), '.locdev-svgl-'));
  const stage = join(stagingRoot, 'generated');
  await mkdir(stage, { recursive: true });
  const iconImports = [];
  const light = [];
  const dark = [];
  const entriesReport = [];

  try {
    const svgCache = new Map();
    const toComponent = async (name, url) => {
      if (!svgCache.has(url)) {
        svgCache.set(url, await request(url));
      }
      const svg = assertSvg(svgCache.get(url), url);
      const tsx = await transform(svg, {
        plugins: [jsxPlugin],
        typescript: true,
        expandProps: 'end',
        dimensions: false,
        jsxRuntime: 'automatic',
        exportType: 'default',
        icon: false,
      }, { componentName: name });
      await writeFile(join(stage, `${name}.tsx`), `/* Source: ${url} — trademark of its owner. */\n${tsx}\n`);
      await writeFile(join(stage, `${name}.svg`), svg);
    };

    for (const entry of selection) {
      const name = componentName(entry.id);
      if (!entry.match) {
        const defaultIcon = entry.id === 'laravel' ? 'ManualLaravel' : 'MissingStackIcon';
        light.push([entry.id, defaultIcon]); dark.push([entry.id, defaultIcon]);
        entriesReport.push({ id: entry.id, name: entry.name, status: entry.id === 'laravel' ? 'user-provided' : 'missing', source: null });
        continue;
      }
      const { routes } = entry.match;
      const isDual = routes.light !== routes.dark;
      const nameLight = isDual ? `${name}Light` : name;
      const nameDark = isDual ? `${name}Dark` : name;
      // On each candidate error, keep a neutral fallback and report it, never guess another logo.
      try {
        await toComponent(nameLight, routes.light);
        if (isDual) await toComponent(nameDark, routes.dark);
        iconImports.push([nameLight, `./${nameLight}`]);
        if (isDual) iconImports.push([nameDark, `./${nameDark}`]);
        light.push([entry.id, nameLight]); dark.push([entry.id, nameDark]);
        entriesReport.push({ id: entry.id, name: entry.name, status: 'svgl', svglTitle: entry.match.entry.title, source: routes });
      } catch (error) {
        console.warn(`Não foi possível gerar ${entry.id}: ${error.message}`);
        light.push([entry.id, 'MissingStackIcon']); dark.push([entry.id, 'MissingStackIcon']);
        entriesReport.push({ id: entry.id, name: entry.name, status: 'failed', source: routes, error: String(error) });
      }
    }

    const has = new Set(light.map(([id]) => id));
    const missingRef = ['unknown', 'MissingStackIcon'];
    light.push(missingRef); dark.push(missingRef);
    for (const [alias, canonical] of Object.entries(ALIASES)) {
      if (has.has(alias)) continue;
      const lightIcon = light.find(([id]) => id === canonical)?.[1] ?? 'MissingStackIcon';
      const darkIcon = dark.find(([id]) => id === canonical)?.[1] ?? 'MissingStackIcon';
      light.push([alias, lightIcon]); dark.push([alias, darkIcon]);
    }
    const uniqueImports = [...new Map(iconImports.map(([name, path]) => [name, path])).entries()];
    const record = (pairs) => pairs.map(([id, comp]) => `  ${JSON.stringify(id)}: ${comp},`).join('\n');
    const names = definitions.map((def) => `  ${JSON.stringify(def.id)}: ${JSON.stringify(def.name)},`).concat([
      `  next: 'Next.js',`, `  node: 'Node.js',`, `  static: 'HTML5',`, `  unknown: 'Unknown',`
    ]).join('\n');
    const index = `/* AUTO-GENERATED by scripts/sync-stack-icons.mjs — do not edit by hand. */\n` +
      `import type { ComponentType, SVGProps } from 'react';\n` +
      (light.some(([,component]) => component === 'ManualLaravel') ? `import ManualLaravel from '../manual/laravel';\n` : '') +
      `import MissingStackIcon from '../manual/missing';\n` +
      uniqueImports.map(([name, path]) => `import ${name} from '${path}';`).join('\n') + '\n\n' +
      `type SvgComponent = ComponentType<SVGProps<SVGSVGElement>>;\n\n` +
      `export const stacks: Record<string, SvgComponent> = {\n${record(light)}\n};\n` +
      `export const stacksDark: Record<string, SvgComponent> = {\n${record(dark)}\n};\n` +
      `export const stackNames: Record<string, string> = {\n${names}\n};\n`;
    await writeFile(join(stage, 'index.tsx'), index);
    await writeFile(join(stage, 'manifest.json'), JSON.stringify({
      source: 'https://api.svgl.app', generatedAt: new Date().toISOString(),
      total: definitions.length, matched: entriesReport.filter((e) => e.status === 'svgl').length,
      missing: entriesReport.filter((e) => e.status === 'missing').length,
      entries: entriesReport,
    }, null, 2));
    // Avoid partially changing old generated icons until all files are ready.
    const previous = `${outputDir}.previous`;
    await rm(previous, { recursive: true, force: true });
    let movedOld = false;
    try {
      try { await rename(outputDir, previous); movedOld = true; } catch (error) { if (error.code !== 'ENOENT') throw error; }
      await rename(stage, outputDir);
      await rm(previous, { recursive: true, force: true });
    } catch (error) {
      if (movedOld) await rename(previous, outputDir).catch(() => {});
      throw error;
    }
    console.log(`Gerados em ${outputDir}. Consulte generated/manifest.json para o relatório.`);
  } finally {
    await rm(stagingRoot, { recursive: true, force: true });
  }
}

main().catch((error) => { console.error(error); process.exitCode = 1; });
