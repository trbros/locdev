// Captures the app's screens (running against the in-browser mock daemon)
// for the documentation site, in each language and both themes:
// apps/docs/public/screenshots/<locale>/{dark,light}/*.webp. A docs page
// shows its own language's captures (English when one is missing), and CSS
// picks the one matching the site's theme.
//
//   pnpm screenshots
//   SHOTS_LOCALES=pt-BR pnpm screenshots   (only some languages)
//
// Animations are disabled so captures are stable between runs.

import { mkdirSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import sharp from "sharp";
import { createServer } from "vite";

const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(here, "../../docs/public/screenshots");

const server = await createServer({
  root: resolve(here, ".."),
  server: { port: 1430, strictPort: true },
  logLevel: "warn",
});
await server.listen();
const url = "http://localhost:1430/?shots";

// The app's languages. Selectors go through the same catalogs as the UI, so
// one walk works in every language (data from the mock daemon stays as is).
const ALL_LOCALES = ["en", "pt-BR", "es", "ja"];
// SHOTS_LOCALES=pt-BR,ja redoes only those languages.
const LOCALES = process.env.SHOTS_LOCALES ? process.env.SHOTS_LOCALES.split(",") : ALL_LOCALES;
mkdirSync(out, { recursive: true });
for (const old of readdirSync(out)) {
  // Everything not being regenerated stays (older layouts go on a full run).
  if (LOCALES === ALL_LOCALES || LOCALES.includes(old)) rmSync(resolve(out, old), { recursive: true });
}
const catalogs = { en: {} };
for (const locale of ALL_LOCALES.slice(1)) {
  catalogs[locale] = JSON.parse(readFileSync(resolve(here, `../src/i18n/${locale}.json`), "utf8"));
}
const escapeRegex = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

const browser = await chromium.launch(
  // A Chromium other than Playwright's own (e.g. a preinstalled one).
  process.env.SHOTS_CHROMIUM ? { executablePath: process.env.SHOTS_CHROMIUM } : {},
);

async function session(locale, theme) {
  const page = await browser.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1.5,
    colorScheme: theme,
  });
  await page.addInitScript((t) => {
    // Sandboxed frames (message previews) have no storage; skip them.
    try {
      localStorage.setItem("theme", t);
    } catch {}
  }, theme);
  page.on("pageerror", (error) => console.error(`[${locale}/${theme}] page error:`, error.message));
  await page.goto(`${url}&lang=${locale}`);
  await page.addStyleTag({
    content: "*, *::before, *::after { animation: none !important; transition: none !important; }",
  });
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(1500);
  return page;
}

/** Scrolls a settings section to the top of the view. */
const scrollToSection = (page, title) =>
  page.getByText(title, { exact: true }).evaluate((el) => {
    el.style.scrollMarginTop = "36px";
    el.scrollIntoView({ block: "start" });
  });

const press = (page, keys) => page.keyboard.press(keys);
/** Hides the previous step's confirmation toasts from the next capture
 * (CSS only: React owns those nodes). `showToasts` brings them back. */
const dismissToasts = (page) =>
  page.evaluate(() => {
    const style = document.createElement("style");
    style.id = "shots-no-toasts";
    style.textContent = "[data-sonner-toaster] { display: none !important; }";
    document.head.append(style);
  });
const showToasts = (page) => page.evaluate(() => document.getElementById("shots-no-toasts")?.remove());
const escape = async (page) => {
  await press(page, "Escape");
  await page.waitForTimeout(250);
};

async function capture(locale, theme) {
  const catalog = catalogs[locale];
  /** The UI's text for an English key, as `t()` gives it. */
  const L = (key, vars = {}) =>
    Object.entries(vars).reduce((text, [k, v]) => text.replaceAll(`{${k}}`, v), catalog[key] ?? key);
  const startsWith = (key) => new RegExp(`^${escapeRegex(L(key))}`);
  const page = await session(locale, theme);

  async function shot(name) {
    await page.waitForTimeout(350);
    const png = await page.screenshot();
    // WebP keeps text crisp at a fraction of the PNG size (the grain texture
    // makes PNGs heavy).
    mkdirSync(`${out}/${locale}/${theme}`, { recursive: true });
    await sharp(png).webp({ quality: 88, effort: 6 }).toFile(`${out}/${locale}/${theme}/${name}.webp`);
  }

  /** Opens a Settings subpage from its side navigation. */
  const settings = async (page, name) => {
    await press(page, "Control+Comma");
    await page
      .getByRole("navigation", { name: L("Settings") })
      .getByRole("button", { name: L(name), exact: true })
      .click();
    await page.waitForTimeout(300);
  };

  await shot("dashboard");
  await press(page, "Control+2");
  await shot("sites");

  await page.locator("aside li button").nth(3).click();
  await shot("site-failed");
  await page.locator("aside li button").first().click();
  await page.getByRole("tab", { name: L("Settings"), exact: true }).click();
  await shot("site-settings");
  await page.getByRole("tab", { name: L("Server"), exact: true }).click();
  await shot("dev-prompt");
  await page.getByRole("tab", { name: L("Dependencies"), exact: true }).click();
  await shot("dependencies");
  await page.getByRole("checkbox", { name: L("Select {name}", { name: "axios" }) }).click();
  await page.getByRole("button", { name: startsWith("Update") }).click();
  await page.waitForTimeout(900);
  await shot("dependency-update");
  await escape(page);
  await page.getByRole("tab", { name: L("Git"), exact: true }).click();
  await page.getByText("resources/js/cart.ts", { exact: true }).click();
  await shot("git");
  await page.getByRole("radio", { name: L("History") }).click();
  await page.getByText("Fix typo in checkout copy", { exact: true }).click();
  await page.getByRole("radio", { name: L("hard") }).click();
  await page.getByRole("button", { name: L("Reset…") }).click();
  await page.waitForTimeout(500);
  await shot("git-confirm");
  await escape(page);
  await settings(page, "Security");
  await shot("settings-security");
  await settings(page, "Integrations");
  await page.getByRole("button", { name: L("Use the token of gh") }).click();
  await page.waitForTimeout(500);
  await press(page, "Control+2");
  await page.getByRole("tab", { name: L("GitHub"), exact: true }).click();
  await page.getByRole("radio", { name: L("Runs") }).click();
  await shot("github");
  await page.getByRole("tab", { name: L("Overview"), exact: true }).click();

  await press(page, "Control+n");
  await shot("add-site");
  await escape(page);

  await press(page, "Control+k");
  await shot("palette");
  await escape(page);

  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Actions builder") })
    .click();
  await page.waitForTimeout(500);
  await shot("actions-builder");
  await page.getByRole("button", { name: /^PHP \/ Laravel CI/ }).click();
  await page.waitForTimeout(800);
  await shot("actions-editor");

  await press(page, "Control+5");
  await page.waitForTimeout(900);
  await shot("processes");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Command line") })
    .click();
  await shot("cli");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Doctor") })
    .click();
  await page.waitForTimeout(900);
  await shot("doctor");

  await press(page, "Control+6");
  await shot("notifications");

  await settings(page, "Overview");
  await shot("settings");
  await settings(page, "Automations");
  await shot("automations");

  await settings(page, "DNS");
  await shot("settings-dns");
  await page.getByRole("button", { name: L("Route *.test to LocDev") }).click();
  await page.waitForTimeout(500);
  await shot("authorize-dns");
  await escape(page);

  await settings(page, "Integrations");
  await shot("integrations");

  await settings(page, "Updates");
  await page.getByRole("button", { name: L("Install {version}").replace("{version}", "") }).first().waitFor();
  await shot("updates");

  await settings(page, "Shortcuts");
  await shot("shortcuts");

  await press(page, "Control+Shift+D");
  await page.getByRole("button", { name: /QueryException/ }).first().click();
  await shot("debug-center");
  await page.getByRole("button", { name: /App\\Models\\Order/ }).first().click();
  await shot("debug-dump");
  await page.getByRole("tab", { name: L("Problems") }).click();
  await shot("debug-problems");

  await settings(page, "Integrations");
  await page.getByText(L("code editor"), { exact: true }).scrollIntoViewIfNeeded();
  await page.mouse.wheel(0, 120);
  await shot("editor-settings");

  await settings(page, "Parked folders");
  await shot("parked-folders");
  await page.getByRole("button", { name: L("Park a folder") }).click();
  await page.waitForTimeout(400);
  await shot("park-folder");
  await escape(page);

  await settings(page, "HTTPS");
  await shot("settings-https");
  await page.getByRole("button", { name: L("Trust LocDev CA") }).click();
  await page.waitForTimeout(500);
  await shot("authorize-tls");
  await escape(page);

  await press(page, "Control+3");
  await page.waitForTimeout(800);
  await shot("runtimes");
  // The defaults in new terminals.
  const terminal = page.getByRole("switch", { name: startsWith("Use the defaults in the terminal") });
  await terminal.click();
  await page.waitForTimeout(500);
  await terminal.evaluate((el) => {
    el.style.scrollMarginTop = "90px";
    el.scrollIntoView({ block: "start" });
  });
  await dismissToasts(page);
  await shot("runtimes-terminal");
  await showToasts(page);
  await terminal.click();
  await page.getByRole("tab", { name: L("Node.js"), exact: true }).click();
  await shot("runtimes-node");
  await page.getByRole("tab", { name: L("PHP"), exact: true }).click();
  await page.waitForTimeout(400);
  // The PHP catalog: one release per line, from StaticPHP / windows.php.net.
  await page.getByRole("button", { name: L("Install version…") }).first().click();
  await page.getByRole("dialog").getByText("8.4.28").waitFor();
  await dismissToasts(page);
  await shot("runtimes-install");
  await showToasts(page);
  await escape(page);
  await page.getByRole("radio", { name: "PHP 8.4" }).click();
  await page.waitForTimeout(300);
  // The section's title is "設定" in Japanese, like Settings in the sidebar.
  await page.getByRole("radiogroup", { name: L("PHP version") }).evaluate((el) => {
    el.style.scrollMarginTop = "36px";
    el.scrollIntoView({ block: "start" });
  });
  await shot("php-settings");
  await page.getByRole("button", { name: L("Advanced configuration") }).click();
  await page.waitForTimeout(400);
  await shot("php-advanced");
  await escape(page);

  await press(page, "Control+4");
  await shot("services");

  // The compact sidebar.
  await press(page, "Control+b");
  await press(page, "Control+1");
  await shot("sidebar-collapsed");
  await press(page, "Control+b");
  // Tools added in v0.3.
  await press(page, "Control+1");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Database") })
    .click();
  await page.waitForTimeout(600);
  await page.getByRole("button", { name: L("PostgreSQL · acme_shop") }).click();
  await page.waitForTimeout(400);
  await page
    .getByRole("navigation", { name: L("Tables") })
    .getByRole("button", { name: /^users/ })
    .click();
  await page.waitForTimeout(700);
  await shot("database");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Mailbox") })
    .click();
  await page.waitForTimeout(900);
  await shot("mailbox");
  await press(page, "Control+2");
  await page.getByRole("tab", { name: L("Environment"), exact: true }).click();
  await page.waitForTimeout(500);
  await shot("site-environment");
  await page.getByRole("tab", { name: L("Config"), exact: true }).click();
  await page.waitForTimeout(700);
  await shot("config-files");
  await page.getByRole("tab", { name: L("Email Studio"), exact: true }).click();
  await page.waitForTimeout(900);
  await page
    .getByRole("region", { name: L("Structure") })
    .getByText("Confirm e-mail")
    .first()
    .click();
  await page.waitForTimeout(700);
  await shot("email-studio");
  await page.getByRole("tab", { name: L("Deploy"), exact: true }).click();
  await page.waitForTimeout(500);
  await shot("deploy-linked");
  // The assistant, below the linked project.
  await page.getByRole("region", { name: L("Stack") }).evaluate((el) => el.scrollIntoView({ block: "start" }));
  await page.waitForTimeout(400);
  await shot("deploy");
  await page.getByRole("tab", { name: L("Overview"), exact: true }).click();
  // New project (v0.3, stage 8): the form with its plan, then the run.
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("New project") })
    .click();
  await page.waitForTimeout(600);
  await page.getByRole("button", { name: /^Laravel/ }).click();
  await page.getByLabel(L("Project name")).fill("shop");
  await page.getByRole("combobox", { name: L("Starter kit") }).click();
  await page.getByRole("option", { name: L("React") }).click();
  await page.getByRole("combobox", { name: L("LocDev database") }).click();
  await page.getByRole("option", { name: L("PostgreSQL") }).click();
  await page.getByLabel(L("Search {label}", { label: "Packagist" })).fill("horizon");
  await page.waitForTimeout(900);
  await page
    .getByRole("button", { name: L("Add") })
    .first()
    .click();
  await page.waitForTimeout(500);
  // The form and the plan, from the project's name down.
  await page.evaluate(
    (title) => {
      const heading = [...document.querySelectorAll("h2")].find((h) => h.textContent === title);
      heading?.scrollIntoView({ block: "start" });
    },
    L("{name} project", { name: "Laravel" }),
  );
  await page.waitForTimeout(300);
  await shot("new-project");
  await page.getByRole("button", { name: L("Create project") }).click();
  await page.waitForTimeout(3600);
  await shot("new-project-run");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Installers") })
    .click();
  await page.waitForTimeout(700);
  await shot("installers");
  await page
    .getByRole("navigation", { name: L("Main") })
    .getByRole("button", { name: L("Tunnels") })
    .click();
  await page.waitForTimeout(500);
  await shot("tunnels");
  await page.getByRole("button", { name: L("Share a site") }).click();
  await page.waitForTimeout(400);
  await shot("tunnel-share");
  await escape(page);
  await page.close();
  console.log(`  ${locale}/${theme}`);
}

// Two at a time: each walk is its own page and mock daemon.
const runs = LOCALES.flatMap((locale) => ["dark", "light"].map((theme) => [locale, theme]));
while (runs.length) await Promise.all(runs.splice(0, 2).map(([locale, theme]) => capture(locale, theme)));

await browser.close();
await server.close();
