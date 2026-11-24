// Accessibility check of every page against the in-browser mock daemon,
// in both themes, with axe-core and the WCAG 2.0/2.1/2.2 A and AA rules
// (what EN 301 549, the European Accessibility Act and Section 508 refer to):
//
//   pnpm test:a11y
//   A11Y_DETAIL=1 pnpm test:a11y      (every element, not just the first)
//
// Fails when any page has a violation. Needs Playwright's Chromium.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const here = dirname(fileURLToPath(import.meta.url));
const axeSource = readFileSync(createRequire(import.meta.url).resolve("axe-core/axe.min.js"), "utf8");
const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

const server = await createServer({
  root: resolve(here, ".."),
  server: { port: 1434, strictPort: true },
  logLevel: "error",
});
await server.listen();
const browser = await chromium.launch();

// Every page and subpage (src/lib/nav.ts), plus each site tab.
const ROUTES = [
  ["dashboard"],
  ["sites"],
  ...["overview", "node", "php", "package-managers"].map((s) => ["runtimes", s]),
  ["services"],
  ...[
    "new-project",
    "installers",
    "processes",
    "debug",
    "database",
    "mailbox",
    "tunnels",
    "actions",
    "cli",
    "doctor",
  ].map((s) => ["tools", s]),
  ...["pending", "history", "preferences"].map((s) => ["notifications", s]),
  ...[
    "overview",
    "daemon",
    "proxy",
    "dns",
    "parked",
    "https",
    "automations",
    "frameworks",
    "environment",
    "security",
    "integrations",
    "updates",
    "appearance",
    "shortcuts",
    "about",
  ].map((s) => ["settings", s]),
];
const SITE_TABS = [
  "Server",
  "Dependencies",
  "Environment",
  "Config",
  "Email Studio",
  "Deploy",
  "Git",
  "GitHub",
  "Settings",
];

const found = new Map(); // rule id → { impact, help, url, pages: Set, nodes: [] }
let checked = 0;

async function audit(page, where) {
  await page.waitForTimeout(500);
  const result = await page.evaluate(
    async (tags) =>
      // biome-ignore lint: axe is injected into the page
      await window.axe.run(document, { runOnly: { type: "tag", values: tags }, resultTypes: ["violations"] }),
    TAGS,
  );
  checked++;
  for (const v of result.violations) {
    const entry = found.get(v.id) ?? { impact: v.impact, help: v.help, url: v.helpUrl, pages: new Set(), nodes: [] };
    entry.pages.add(where);
    for (const node of v.nodes) {
      if (entry.nodes.length < (process.env.A11Y_DETAIL ? 50 : 3))
        entry.nodes.push(`${where}: ${node.target.join(" ")} — ${node.failureSummary?.split("\n")[1]?.trim() ?? ""}`);
    }
    found.set(v.id, entry);
  }
}

for (const theme of ["dark", "light"]) {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 }, colorScheme: theme });
  await page.addInitScript((t) => {
    try {
      localStorage.setItem("theme", t);
    } catch {}
  }, theme);
  for (const [view, sub] of ROUTES) {
    await page.addInitScript(
      (route) => {
        try {
          sessionStorage.setItem("locdev.route", JSON.stringify(route));
        } catch {}
      },
      { view, sub },
    );
    await page.goto("http://localhost:1434/?lang=en&shots");
    await page.addScriptTag({ content: axeSource });
    await page.waitForTimeout(700);
    await audit(page, `${theme} ${view}${sub ? `/${sub}` : ""}`);
    if (view === "sites") {
      await page.locator("aside li button").first().click();
      for (const tab of SITE_TABS) {
        const button = page.getByRole("tab", { name: tab, exact: true });
        if ((await button.count()) === 0) continue;
        await button.first().click();
        await audit(page, `${theme} site/${tab}`);
      }
    }
  }
  // Keyboard only (WCAG 2.4.7): every element Tab reaches must show focus.
  for (const [view, sub] of [
    ["dashboard"],
    ["runtimes", "overview"],
    ["settings", "appearance"],
    ["tools", "doctor"],
  ]) {
    await page.addInitScript(
      (route) => {
        try {
          sessionStorage.setItem("locdev.route", JSON.stringify(route));
        } catch {}
      },
      { view, sub },
    );
    await page.goto("http://localhost:1434/?lang=en&shots");
    await page.waitForTimeout(700);
    for (let i = 0; i < 40; i++) {
      await page.keyboard.press("Tab");
      const hidden = await page.evaluate(() => {
        const el = document.activeElement;
        if (!el || el === document.body) return null;
        const shows = (e) => {
          const s = getComputedStyle(e);
          return (
            (s.outlineStyle !== "none" && Number.parseFloat(s.outlineWidth) > 0) ||
            (s.boxShadow && s.boxShadow !== "none")
          );
        };
        // The ring may be drawn by the element or by a wrapper (focus-within).
        for (let e = el, depth = 0; e && depth < 3; e = e.parentElement, depth++) if (shows(e)) return null;
        return `${el.tagName.toLowerCase()}${el.getAttribute("aria-label") ? `[${el.getAttribute("aria-label")}]` : ""} "${(el.textContent ?? "").trim().slice(0, 30)}"`;
      });
      if (hidden) {
        const where = `${theme} ${view}${sub ? `/${sub}` : ""}`;
        const entry = found.get("focus-visible") ?? {
          impact: "serious",
          help: "Focused elements must show a visible focus indicator (WCAG 2.4.7)",
          url: "https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html",
          pages: new Set(),
          nodes: [],
        };
        entry.pages.add(where);
        if (entry.nodes.length < 40 && !entry.nodes.some((n) => n.endsWith(hidden)))
          entry.nodes.push(`${where}: ${hidden}`);
        found.set("focus-visible", entry);
      }
    }
    checked++;
  }

  // Dialogs: add site, the command palette.
  await page.addScriptTag({ content: axeSource });
  await page.keyboard.press("Control+n");
  await audit(page, `${theme} dialog add-site`);
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+k");
  await audit(page, `${theme} dialog palette`);
  await page.keyboard.press("Escape");
  await page.close();
}

await browser.close();
await server.close();

const order = { critical: 0, serious: 1, moderate: 2, minor: 3 };
const rules = [...found.entries()].sort((a, b) => order[a[1].impact] - order[b[1].impact]);
console.log(`${checked} views checked against ${TAGS.join(", ")}`);
for (const [id, v] of rules) {
  console.log(`\n✗ ${id} (${v.impact}) — ${v.help}\n  ${v.url}\n  on ${v.pages.size} view(s)`);
  for (const n of v.nodes) console.log(`    ${n}`);
}
if (rules.length) {
  console.log(`\n${rules.length} rule(s) failed`);
  process.exit(1);
}
console.log("no violations");
