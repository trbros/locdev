// Walks every page and subpage of the app against the in-browser mock daemon
// and fails on any page error or missing heading:
//
//   pnpm test:ui
//
// Needs Playwright's Chromium (`pnpm exec playwright install chromium`).

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer } from "vite";

const here = dirname(fileURLToPath(import.meta.url));
const server = await createServer({
  root: resolve(here, ".."),
  server: { port: 1431, strictPort: true },
  logLevel: "error",
});
await server.listen();

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));

let failures = 0;
async function step(name, fn) {
  try {
    await fn();
    console.log(`  ok   ${name}`);
  } catch (error) {
    failures++;
    console.log(`  FAIL ${name}: ${process.env.DEBUG ? error.message : error.message.split("\n")[0]}`);
    // DEBUG=<dir> keeps a screenshot of each failure.
    if (process.env.DEBUG) await page.screenshot({ path: `${process.env.DEBUG}/fail-${failures}.png` }).catch(() => {});
  }
}
const heading = (text) => page.getByRole("heading", { name: text, exact: true }).first().waitFor({ timeout: 3000 });
const press = (keys) => page.keyboard.press(keys);

// English regardless of the machine's language: the checks below look for the source strings.
await page.goto("http://localhost:1431/?lang=en");
await page.waitForTimeout(800);

await step("dashboard is the first page", () => heading("Dashboard"));
await step("Ctrl+2 opens Sites", async () => {
  await press("Control+2");
  // The list title carries the count: "Sites 5".
  await page.getByRole("heading", { name: /^Sites \d+$/ }).waitFor({ timeout: 3000 });
});
for (const tab of ["Server", "Settings", "Overview"]) {
  await step(`site tab ${tab}`, async () => {
    await page.getByRole("tab", { name: tab }).click();
    await page.getByRole("tab", { name: tab, selected: true }).waitFor({ timeout: 2000 });
  });
}
await step("site tabs: Ctrl+click opens a tab, the address bar switches, new tab, close", async () => {
  await press("Control+2");
  const strip = page.getByRole("tablist", { name: "Open sites" });
  const tabs = strip.getByRole("tab");
  const before = await tabs.count();
  await page.locator("aside li button", { hasText: "blog" }).click({ modifiers: ["Control"] });
  await strip.getByRole("tab", { name: /^blog\.test/, selected: true }).waitFor({ timeout: 3000 });
  if ((await tabs.count()) !== before + 1) throw new Error("Ctrl+click did not open a tab");
  await page.getByRole("button", { name: "Site address" }).click();
  await page.getByRole("textbox", { name: "Site address" }).fill("landing");
  await page.keyboard.press("Enter");
  await strip.getByRole("tab", { name: /^landing\.test/, selected: true }).waitFor({ timeout: 3000 });
  if ((await tabs.count()) !== before + 1) throw new Error("the address bar opened a tab instead of switching");
  if (process.env.SHOT_TABS) await page.screenshot({ path: process.env.SHOT_TABS });
  await page.getByRole("button", { name: "New tab" }).click();
  await page.getByRole("heading", { name: "Open a site" }).waitFor({ timeout: 3000 });
  await strip.locator("[data-tab]:has([role=tab][aria-selected=true]) button[aria-label=\"Close tab\"]").click();
  await strip.getByRole("tab", { name: /^landing\.test/, selected: true }).waitFor({ timeout: 3000 });
  await strip.locator("[data-tab]:has([role=tab][aria-selected=true]) button[aria-label=\"Close tab\"]").click();
  if ((await tabs.count()) !== before) throw new Error("closing tabs left the wrong count");
});
await step("Ctrl+3 opens Runtimes", async () => {
  await press("Control+3");
  await heading("Runtimes");
});
for (const tab of ["Node.js", "PHP", "Package managers", "Overview"]) {
  await step(`runtimes tab ${tab}`, async () => {
    await page.getByRole("tab", { name: tab }).click();
    await page.getByRole("tab", { name: tab, selected: true }).waitFor({ timeout: 2000 });
  });
}
await step("runtimes overview: the defaults in the terminal", async () => {
  const toggle = page.getByRole("switch", { name: /Use the defaults in the terminal/ });
  await toggle.waitFor({ timeout: 3000 });
  if ((await toggle.getAttribute("aria-checked")) === "true") throw new Error("starts on");
  await toggle.click();
  await page.getByText("Open a new terminal to use them.").waitFor({ timeout: 3000 });
  await page.getByText("~/.zshrc", { exact: false }).first().waitFor({ timeout: 3000 });
  await toggle.click();
  await page.getByText("Open a new terminal to use them.").waitFor({ state: "detached", timeout: 3000 });
});
await step("PHP install dialog lists each major.minor line", async () => {
  await page.getByRole("tab", { name: "PHP" }).click();
  await page.getByRole("button", { name: "Install version…" }).first().click();
  await page.getByText("StaticPHP", { exact: false }).first().waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByText("8.4.28").waitFor({ timeout: 3000 });
  const lines = await page.getByRole("dialog").locator("li").count();
  await press("Escape");
  // 8.5, 8.4, 8.3, 8.2: one per major.minor, not one per major.
  if (lines !== 4) throw new Error(`${lines} PHP line(s) listed`);
});
await step("Ctrl+5 opens Tools → Processes & logs", async () => {
  await press("Control+5");
  await heading("Processes & logs");
  await page.getByRole("log").first().waitFor({ timeout: 3000 });
});
await step("console filters by level", async () => {
  await page
    .getByRole("radio", { name: /^error/ })
    .first()
    .click();
  await page.getByText("local.ERROR", { exact: false }).first().waitFor({ timeout: 3000 });
  await page.getByRole("radio", { name: "any level" }).first().click();
});
await step("Tools → Command line lists and searches commands", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Command line" }).click();
  await heading("Command line");
  await page.getByRole("textbox", { name: "Search commands" }).fill("secure");
  await page.getByText("locdev secure", { exact: true }).first().waitFor({ timeout: 2000 });
});
await step("Tools → Doctor", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Doctor" }).click();
  await heading("Doctor");
});
await step("helper dev server asks first, then runs", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Server" }).click();
  await page.getByText("Vite detected in this project.").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Start now" }).click();
  await page.getByRole("tab", { name: "vite output" }).waitFor({ timeout: 3000 });
});
await step("site dependencies: filter, select, plan", async () => {
  await page.getByRole("tab", { name: "Dependencies" }).click();
  await page.getByRole("radio", { name: /^Security/ }).click();
  await page.getByText("axios", { exact: true }).first().waitFor({ timeout: 3000 });
  await page.getByRole("checkbox", { name: "Select axios" }).click();
  await page.getByRole("button", { name: /^Update/ }).click();
  await page.getByText("npm update axios --ignore-scripts", { exact: false }).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Run the update" }).click();
  await page
    .getByText(/updated axios 1\.7\.7/)
    .first()
    .waitFor({ timeout: 5000 });
  await page.keyboard.press("Escape");
});
await step("site git: stage, commit, discard needs the phrase", async () => {
  await page.getByRole("tab", { name: "Git", exact: true }).click();
  await page.getByRole("button", { name: "Stage resources/js/cart.ts" }).click();
  await page.getByRole("textbox", { name: "Commit message" }).fill("Keep coupons on renew");
  await page.getByRole("button", { name: /^Commit 2 files/ }).click();
  await page.getByText("Committed", { exact: false }).first().waitFor({ timeout: 3000 });
  await page.getByRole("radio", { name: /^Changes/ }).click();
  await page.getByRole("button", { name: "Discard notes/todo.md" }).click();
  await page.getByText("notes/todo.md (untracked file, deleted)").waitFor({ timeout: 3000 });
  const confirm = page.getByRole("button", { name: "Confirm" });
  if (await confirm.isEnabled()) throw new Error("confirm enabled before typing the phrase");
  await page.getByRole("textbox", { name: "Confirmation" }).fill("discard");
  await confirm.click();
  await page.getByText("Nothing to commit", { exact: false }).waitFor({ timeout: 3000 });
  await page.getByRole("radio", { name: "Activity" }).click();
  await page.getByText(/discard: Discarded/).waitFor({ timeout: 3000 });
});
await step("Ctrl+6 opens Notifications; mark all read", async () => {
  await press("Control+6");
  await heading("Notifications");
  await page.getByRole("button", { name: "Mark all read" }).click();
  await page.getByText("You're all caught up.").waitFor({ timeout: 3000 });
  await page.getByRole("tab", { name: "History" }).click();
  await page.getByText("GHSA-8hc4-vh64-cxmj in axios", { exact: false }).first().waitFor({ timeout: 3000 });
});
await step("Ctrl+, opens Settings", async () => {
  await press("Control+Comma");
  await heading("Settings");
});
for (const section of [
  "Daemon",
  "Proxy",
  "DNS",
  "Parked folders",
  "HTTPS",
  "Automations",
  "Frameworks",
  "Environment",
  "Security",
  "Integrations",
  "Appearance",
  "About",
  "Overview",
]) {
  await step(`settings → ${section}`, async () => {
    await page
      .getByRole("navigation", { name: "Settings" })
      .getByRole("button", { name: section, exact: true })
      .click();
    await heading(section);
  });
}
await step("Alt+← goes back", async () => {
  await press("Alt+ArrowLeft");
  await heading("About");
});
await step("Ctrl+B collapses the sidebar and remembers it", async () => {
  await press("Control+b");
  await page.locator('nav[aria-label="Main"]:not([data-expanded])').waitFor({ timeout: 2000 });
  const stored = await page.evaluate(() => JSON.parse(localStorage.getItem("locdev.prefs")).sidebar);
  if (stored !== "collapsed") throw new Error(`stored ${stored}`);
  await press("Control+b");
});
await step("palette reaches a subpage", async () => {
  await press("Control+k");
  await page.keyboard.type("Settings DNS");
  await page.keyboard.press("Enter");
  await heading("DNS");
});
await step("security: set a password, then a reset asks for it", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Security" }).click();
  await page.getByRole("textbox", { name: "New password", exact: true }).fill("correct horse");
  await page.getByRole("textbox", { name: "Repeat new password" }).fill("correct horse");
  await page.getByRole("button", { name: "Set password" }).click();
  await page.getByText("Password set").first().waitFor({ timeout: 3000 });
  await press("Control+2");
  await page.getByRole("tab", { name: "Git", exact: true }).click();
  await page.getByRole("radio", { name: "History" }).click();
  await page.getByText("Upgrade to Laravel 12", { exact: true }).click();
  await page.getByRole("button", { name: "Reset…" }).click();
  await page.getByRole("textbox", { name: "LocDev password" }).waitFor({ timeout: 3000 });
  await page.keyboard.press("Escape");
});
await step("environment: a new secret stays masked, reaches the site, syncs masked", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Environment" }).click();
  await page.getByRole("button", { name: "Add variable" }).click();
  await page.getByLabel("Name").fill("API_TOKEN");
  await page.getByLabel("Value").fill("tok_live_smoke");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await page.getByText("API_TOKEN", { exact: true }).first().waitFor({ timeout: 3000 });
  if (await page.getByText("tok_live_smoke").count()) throw new Error("secret value shown in the list");
  await press("Control+2");
  await page.getByRole("tab", { name: "Environment" }).click();
  await page.getByText("API_TOKEN", { exact: true }).first().waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Sync to .env…" }).click();
  await page.getByText("+ API_TOKEN=********").waitFor({ timeout: 3000 });
  await page.keyboard.press("Escape");
});
await step("stack: the overview lists components; a hand-added one shows as added", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Overview" }).click();
  await page.getByText("Inertia", { exact: true }).first().waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Edit…" }).click();
  await page.getByLabel("Search definitions").fill("react");
  await page.getByRole("dialog").getByRole("button", { name: "Add" }).first().click();
  await page.getByRole("dialog").getByRole("button", { name: "Hide" }).first().waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Done" }).click();
  await page.getByText("added", { exact: true }).first().waitFor({ timeout: 3000 });
});
await step("processes: decide, start, see the output; an alternative is refused", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Server" }).click();
  await page
    .getByRole("radiogroup", { name: "How LocDev runs Queue worker" })
    .getByRole("radio", { name: "manual" })
    .click();
  await page.getByRole("button", { name: "Start Queue worker" }).click();
  await page.getByRole("tab", { name: "queue worker output" }).click();
  await page.getByText("Processing jobs from the [default] queue.").first().waitFor({ timeout: 3000 });
  await page
    .getByRole("radiogroup", { name: "How LocDev runs Horizon" })
    .getByRole("radio", { name: "manual" })
    .click();
  await page.getByRole("button", { name: "Start Horizon" }).click();
  await page
    .getByText(/would consume the same jobs/)
    .first()
    .waitFor({ timeout: 3000 });
});
await step("services: create an instance; deleting its data needs the name", async () => {
  await page.getByRole("button", { name: "Services" }).first().click();
  await page.getByText("db", { exact: true }).first().waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "New instance" }).click();
  await page.getByRole("dialog").getByRole("radio", { name: "Valkey" }).click();
  await page.getByLabel("Name").fill("kv");
  await page.getByRole("dialog").getByRole("button", { name: "Create" }).click();
  await page.getByText("kv", { exact: true }).first().click();
  await page.getByRole("button", { name: "Remove instance…" }).click();
  await page.getByRole("dialog").getByRole("switch").click();
  const remove = page.getByRole("dialog").getByRole("button", { name: "Remove" });
  if (await remove.isEnabled()) throw new Error("remove enabled without the name");
  await page.getByLabel("Type kv to confirm").fill("kv");
  await remove.click();
  await page.getByText("kv", { exact: true }).waitFor({ state: "detached", timeout: 3000 });
});
await step("github: connect via gh, re-run a failed run after confirming", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Integrations" }).click();
  await page.getByRole("button", { name: "Use the token of gh" }).click();
  await page.getByText("acme-dev", { exact: false }).first().waitFor({ timeout: 3000 });
  await press("Control+2");
  await page.getByRole("tab", { name: "GitHub" }).click();
  await page.getByRole("radio", { name: "Runs" }).click();
  await page.getByRole("button", { name: "Re-run failed" }).click();
  await page.getByText("This happens on GitHub as", { exact: false }).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Confirm" }).click();
  await page.getByText("Re-running the failed jobs").first().waitFor({ timeout: 3000 });
});
await step("actions builder: template, injection blocks saving, create", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Actions builder" }).click();
  await heading("Actions builder");
  await page.getByRole("button", { name: /^PHP \/ Laravel CI/ }).click();
  await page.getByText("runs-on: ubuntu-latest").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: /Install dependencies/ }).click();
  await page.getByRole("textbox", { name: "Commands" }).fill('echo "${{ github.event.pull_request.title }}"');
  await page
    .getByText(/is pasted into a shell script/)
    .first()
    .waitFor({ timeout: 8000 });
  await page.getByRole("button", { name: "Review & save…" }).click();
  await page.getByText(/^Fix 1 error first/).waitFor({ timeout: 3000 });
  await page.keyboard.press("Escape");
  await page.getByRole("textbox", { name: "Commands" }).fill("npm ci");
  await page
    .getByText("No problems found.", { exact: false })
    .or(page.getByText("checks · 0 errors"))
    .first()
    .waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Review & save…" }).click();
  await page.getByRole("button", { name: "Create file" }).click();
  await page.getByText("Saved .github/workflows/ci.yml").first().waitFor({ timeout: 3000 });
  await page.getByRole("radio", { name: "YAML" }).click();
  await page.getByRole("textbox", { name: "Workflow YAML" }).waitFor({ timeout: 3000 });
});
await step("database: rows, cell edit, destructive SQL needs the name, remote and missing db gated", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Database" }).click();
  await heading("Databases");
  await page.getByRole("button", { name: "PostgreSQL · acme_shop" }).click();
  await page
    .getByRole("navigation", { name: "Tables" })
    .getByRole("button", { name: /^users/ })
    .click();
  await page.getByText("user1@acme.test", { exact: true }).waitFor({ timeout: 3000 });
  await page.getByText("user1@acme.test", { exact: true }).dblclick();
  await page.getByLabel("New value for email").fill("ana@acme.test");
  await press("Enter");
  await page.getByText("ana@acme.test", { exact: true }).waitFor({ timeout: 3000 });
  await page.getByRole("tab", { name: "sql" }).click();
  await page.getByRole("textbox", { name: "SQL" }).fill("drop table users");
  await press("Control+Enter");
  const run = page.getByRole("dialog").getByRole("button", { name: "Run" });
  await run.waitFor({ timeout: 3000 });
  if (await run.isEnabled()) throw new Error("destructive SQL runnable without the name");
  await press("Escape");
  await page.getByRole("button", { name: "PostgreSQL · blog" }).click();
  await page.getByRole("button", { name: "Open it anyway" }).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "SQLite · dev.db" }).click();
  await page.getByRole("button", { name: /^Create / }).click();
  await page.getByText("No tables yet.").or(page.getByText("Choose a table.")).first().waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Redis/Valkey" }).click();
  await page.getByText("laravel_cache:settings").click();
  await page.getByText("expires in", { exact: false }).waitFor({ timeout: 3000 });
});
await step("config files: options, review diff, save, history, analysis fix", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Config" }).click();
  await page.getByText("vite.config.ts").first().waitFor({ timeout: 3000 });
  await page.getByText("The dev server listens on every network interface").waitFor({ timeout: 3000 });
  const port = page.getByLabel("Port", { exact: true });
  await port.fill("5180");
  await press("Enter");
  await page
    .getByRole("dialog")
    .getByText("+    port: 5180, // LocDev proxies acme-shop.test here")
    .waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByRole("button", { name: "Save" }).click();
  await page.getByText("Saved vite.config.ts").first().waitFor({ timeout: 3000 });
  await page.getByRole("tab", { name: "history" }).click();
  await page.getByText("replaced by: set server.port = 5180").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Review fix" }).click();
  await page.getByRole("dialog").getByText('+    host: "localhost",').waitFor({ timeout: 3000 });
  await press("Escape");
  await page.getByRole("button", { name: /^tsconfig\.json/ }).click();
  const erasable = page.getByRole("switch", { name: "Erasable syntax only" });
  if (await erasable.isEnabled()) throw new Error("an option the installed version lacks is editable");
});
await step("deploy: platforms ranked by fit, readiness, names only", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Deploy" }).click();
  await page.getByText("Recommendations only; nothing is sent to any platform.").waitFor({ timeout: 3000 });
  const cards = page
    .getByRole("region", { name: "Platforms" })
    .getByRole("listitem")
    .filter({ has: page.locator("button[aria-expanded]") });
  const first = await cards.first().textContent({ timeout: 3000 });
  if (!first?.includes("Railway") || !first.includes("fits")) throw new Error(`first platform: ${first}`);
  await page.getByText("APP_DEBUG is true locally").waitFor({ timeout: 3000 });
  await page.getByText("APP_KEY", { exact: true }).waitFor({ timeout: 3000 });
});
await step("subdomains: add a rule, turn the wildcard off, test a host", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Settings" }).click();
  await page.getByLabel("Subdomain", { exact: true }).fill("api");
  await page.getByLabel("Rule port").fill("9000");
  await page.getByRole("button", { name: "Add rule" }).click();
  await page.getByText("→ 127.0.0.1:9000").waitFor({ timeout: 3000 });
  await page.getByRole("switch", { name: "Wildcard" }).click();
  await page.getByText("is off: only listed subdomains answer").waitFor({ timeout: 3000 });
  await page.getByLabel("Hostname to test").fill("other.acme-shop.test");
  await page.getByRole("button", { name: "Test", exact: true }).click();
  await page.getByText("the wildcard is off and no rule matches").waitFor({ timeout: 3000 });
  await page.getByLabel("Hostname to test").fill("api.acme-shop.test");
  await page.getByRole("button", { name: "Test", exact: true }).click();
  await page.getByText("(rule api)").waitFor({ timeout: 3000 });
});
await step("mailbox: list, sandboxed HTML, debug, test send; site mail toggle", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Mailbox" }).click();
  await heading("Mailbox");
  await page
    .getByRole("list", { name: "Messages" })
    .getByText("Welcome to Acme Shop")
    .first()
    .waitFor({ timeout: 3000 });
  const frame = page.locator('iframe[title="Message HTML"]');
  if ((await frame.getAttribute("sandbox")) !== "") throw new Error("message HTML isn't sandboxed");
  // The message's script would replace the body; its own content must still be there.
  await frame.contentFrame().getByRole("heading", { name: "Welcome to Acme Shop, Ana!" }).waitFor({ timeout: 3000 });
  await page.getByText(/remote resource.* not loaded/).waitFor({ timeout: 3000 });
  await page.getByRole("tab", { name: /^Debug/ }).click();
  await page.getByText("a line is longer than 998 characters").waitFor({ timeout: 3000 });
  await page.getByLabel("Search mail").fill("invoice");
  await page.getByRole("list", { name: "Messages" }).getByText("Invoice #1042").waitFor({ timeout: 3000 });
  await page.getByLabel("Search mail").fill("");
  await page.getByRole("button", { name: "Send a test" }).click();
  await page.getByRole("list", { name: "Messages" }).getByText("LocDev test message").waitFor({ timeout: 5000 });
  await press("Control+2");
  await page.getByRole("tab", { name: "Settings" }).click();
  await page.getByRole("switch", { name: "Send mail to the Mailbox" }).waitFor({ timeout: 3000 });
  await page.getByText("sets: MAIL_MAILER=smtp").waitFor({ timeout: 3000 });
});
await step("email studio: edit a block live, add one, undo, review the diff, save, send a test", async () => {
  await press("Control+2");
  await page.getByRole("tab", { name: "Email Studio" }).click({ timeout: 5000 });
  const structure = page.getByRole("region", { name: "Structure" });
  await structure.getByText("Confirm e-mail").first().click({ timeout: 5000 });
  await page.getByLabel("Label", { exact: true }).fill("Verify my address");
  const frame = page.locator('iframe[title="Email preview"]').contentFrame();
  await frame.getByText("Verify my address").waitFor({ timeout: 3000 });
  await frame.getByText("Welcome, Ana!").waitFor({ timeout: 3000 });
  await page.getByRole("region", { name: "Blocks" }).getByRole("button", { name: "Divider" }).click({ timeout: 5000 });
  await page.getByRole("button", { name: "Undo" }).click({ timeout: 5000 });
  await page.getByRole("button", { name: "Review & save" }).click({ timeout: 5000 });
  const dialog = page.getByRole("dialog");
  await dialog.getByText("Verify my address").first().waitFor({ timeout: 3000 });
  await dialog.getByRole("button", { name: "Save" }).click({ timeout: 5000 });
  await page.getByText("Saved resources/views/emails/welcome.blade.php").first().waitFor({ timeout: 3000 });
  // Blade logic outside the blocks is still there, as code.
  await structure.getByText("@if ($user->isAdmin())").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Send test" }).click({ timeout: 5000 });
  await page.getByText("Sent to the Mailbox").first().waitFor({ timeout: 3000 });
});
await step("parked folders: park one, change rules, un-ignore, stop parking keeps files", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Parked folders" }).click();
  await heading("Parked folders");
  await page.getByText("~/Code", { exact: true }).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Park a folder" }).click();
  await page.getByPlaceholder("/home/you/Development").fill("/home/you/Experiments");
  await page.getByRole("radio", { name: "Suggest it" }).click();
  await page.getByRole("button", { name: "Park", exact: true }).click();
  await page.getByText("~/Experiments", { exact: true }).waitFor({ timeout: 3000 });
  await page.getByText("new projects are suggested").nth(1).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Stop ignoring" }).click();
  await page.getByRole("button", { name: "Stop ignoring" }).waitFor({ state: "detached", timeout: 3000 });
  await page.getByRole("button", { name: "Stop parking" }).nth(1).click();
  await page.getByText("Its 2 sites are stopped and removed from LocDev.").waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByRole("button", { name: "Stop parking" }).click();
  await page.getByText("Removed sites: wp-agency, docs-clients").waitFor({ timeout: 3000 });
  if (process.env.SHOT) await page.screenshot({ path: process.env.SHOT });
});
await step("new project: Flask shows LocDev's files; Laravel picks a PHP version", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "New project" }).click();
  await page.getByRole("button", { name: /^Flask/ }).click();
  await page.getByLabel("Project name").fill("api");
  await page.getByText("LocDev writes its starter files first:").waitFor({ timeout: 3000 });
  await page.locator("summary", { hasText: "app.py" }).waitFor({ timeout: 3000 });
  await page.locator("summary", { hasText: "requirements.txt" }).click();
  await page.getByText("python3 -m venv .venv").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: /^Laravel/ }).click();
  await page.getByLabel("Project name").fill("shop");
  await page.getByRole("combobox", { name: "PHP" }).click();
  await page.getByRole("option", { name: "8.3", exact: true }).click();
  await page.getByText("Created with PHP 8.3, and the site is pinned to it.").waitFor({ timeout: 3000 });
});
await step("php settings: change a value, review the diff, apply, sites restart", async () => {
  await press("Control+3");
  await page.getByRole("tab", { name: "PHP", exact: true }).click();
  await page.getByText("LocDev's sites run on PHP's built-in server", { exact: false }).waitFor({ timeout: 3000 });
  await page.getByRole("radio", { name: "PHP 8.4" }).click();
  await page.getByLabel("memory_limit", { exact: true }).fill("1G");
  await page.getByRole("button", { name: "Review 1 change" }).click();
  await page.getByText("+ memory_limit = 1G").waitFor({ timeout: 3000 });
  await page.getByText("Restarts: acme-shop").waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByRole("button", { name: "Apply", exact: true }).click();
  await page.getByText("Restarted: acme-shop").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Advanced configuration" }).click();
  await page.getByRole("dialog").getByText("Advanced configuration · PHP 8.4").waitFor({ timeout: 3000 });
  await press("Escape");
});
await step("tunnels: share a running site with e-mail protection, see the address, stop", async () => {
  await page.getByRole("navigation", { name: "Main" }).getByRole("button", { name: "Tunnels" }).click();
  await page.getByText("https://calm-fox-river.trycloudflare.com").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Share a site" }).click();
  await page.getByRole("combobox", { name: "Site" }).selectOption("blog");
  await page.getByLabel("Allowed e-mail addresses").fill("bad-email");
  await page.getByRole("dialog").getByRole("button", { name: "Share", exact: true }).click();
  await page.getByText("'bad-email' is not an e-mail address").waitFor({ timeout: 3000 });
  await page.getByLabel("Allowed e-mail addresses").fill("Ana@Example.com");
  await page.getByRole("dialog").getByRole("button", { name: "Share", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByText(/^https:\/\/.*\.trycloudflare\.com$/)
    .waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByRole("button", { name: "Done" }).click();
  await page.getByText("Only ana@example.com, after an e-mail PIN").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Stop sharing" }).first().click();
  await page.getByText("acme-shop is no longer shared").waitFor({ timeout: 3000 });
});
await step("deploy platforms: connect, linked project, deploy, roll back, logs", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Integrations" }).click();
  await page.getByText("connected as ana").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Connect…" }).click();
  await page.getByLabel("Token", { exact: true }).fill("short");
  await page.getByRole("dialog").getByRole("button", { name: "Connect", exact: true }).click();
  await page.getByText("the platform refused the token").waitFor({ timeout: 3000 });
  await page.getByLabel("Token", { exact: true }).fill("nfp_a-long-enough-token");
  await page.getByRole("dialog").getByRole("button", { name: "Connect", exact: true }).click();
  await page.getByText("connected as ana").nth(1).waitFor({ timeout: 3000 });
  await press("Control+2");
  await page.locator("aside li button", { hasText: "acme-shop" }).click();
  await page.getByRole("tab", { name: "Deploy", exact: true }).click();
  await page.getByText("Fix the cart total with coupons").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Deploy to production" }).click();
  await page.getByText("builds main at 4be1c02d9a").waitFor({ timeout: 3000 });
  await page.getByRole("dialog").getByRole("button", { name: "Deploy", exact: true }).click();
  await page.getByText("Deploy started").waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: "Build log" }).first().click();
  await page.getByText('Running "vercel build"').waitFor({ timeout: 3000 });
  await press("Escape");
});
await step("updates: install with progress, restart prompt, earlier version", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Updates" }).click();
  await page.getByText(/LocDev \d+\.\d+\.\d+ is available/).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: /^Install \d/ }).first().click();
  await page.getByText("Restart LocDev").waitFor({ timeout: 6000 });
  await page.getByRole("button", { name: "Go back to this version" }).first().waitFor({ timeout: 3000 });
  await page.getByText("this app ships").waitFor({ state: "detached", timeout: 1000 });
});
await step("shortcuts: rebind one, it fires, conflicts are caught, defaults come back", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Shortcuts" }).click();
  await page.getByRole("button", { name: "Add a shortcut for Go to Tools" }).click();
  await page.getByText("Press the keys…").waitFor({ timeout: 2000 });
  await press("Control+K");
  await page.getByText("is already used by Open the command palette.").waitFor({ timeout: 2000 });
  await page.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: "Add a shortcut for Go to Tools" }).click();
  await press("Control+Alt+G");
  await press("Control+Alt+G");
  await page.getByRole("navigation", { name: "Settings" }).waitFor({ state: "detached", timeout: 2000 });
  await press("Control+Slash");
  await page.getByRole("button", { name: "Restore all defaults" }).click();
  await page.getByRole("button", { name: "Restore all defaults" }).isDisabled();
});
await step("editor: chosen in Integrations, opens a site from its menu", async () => {
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "Integrations" }).click();
  await page.getByRole("radio", { name: /Zed/ }).click();
  await page.getByRole("radio", { name: /Zed/, checked: true }).waitFor({ timeout: 2000 });
  await press("Control+2");
  await page.locator("aside li button", { hasText: "acme-shop" }).click({ button: "right" });
  await page.getByRole("menuitem", { name: "Open in Zed" }).click();
});
await step("about: versions, history and third-party software", async () => {
  await press("Control+Comma");
  await page.getByRole("navigation", { name: "Settings" }).getByRole("button", { name: "About" }).click();
  await page.getByRole("button", { name: "Version history" }).click();
  await page.getByRole("dialog").getByText("this app").waitFor({ timeout: 2000 });
  await press("Escape");
  await page.getByRole("button", { name: "Third-party software" }).click();
  await page.getByRole("dialog").getByText("tokio", { exact: true }).waitFor({ timeout: 3000 });
  await press("Escape");
});
await step("debug center: timeline, detail with cause and stack, dumps, problems", async () => {
  await press("Control+Shift+D");
  await page.getByRole("heading", { name: "Debug Center" }).waitFor({ timeout: 3000 });
  await page.getByRole("button", { name: /QueryException/ }).first().click();
  await page.getByText("Likely cause").waitFor({ timeout: 2000 });
  await page.getByText("4 frames in dependencies").click();
  await page.getByText("Illuminate\\Routing\\Route->runController()").waitFor({ timeout: 2000 });
  await page.getByRole("button", { name: /App\\Models\\Order/ }).first().click();
  await page.getByText('"customer_id"').waitFor({ timeout: 2000 });
  await page.getByRole("tab", { name: "Problems" }).click();
  await page.getByRole("list", { name: "Problems" }).waitFor({ timeout: 2000 });
  await page.getByRole("navigation", { name: "Sites" }).getByRole("button", { name: "acme-shop" }).click();
  await page.getByRole("switch").last().click();
  await page.getByText("Dumps from acme-shop now come here (restarted)").waitFor({ timeout: 3000 });
});
await step("no page errors", async () => {
  if (errors.length) throw new Error(errors.join(" | "));
});

await browser.close();
await server.close();
if (failures) {
  console.log(`${failures} step(s) failed`);
  process.exit(1);
}
console.log("all steps passed");
