import { ArrowDownToLine, BookOpen, ExternalLink, FileText, GitBranch, History, Package, Scale } from "lucide-react";
import { lazy, Suspense, useState } from "react";

import { LogoMark } from "@/components/logo";
import { Fact, Section } from "@/components/page";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { useAppInfo, useCli, useStatus } from "@/hooks/use-daemon";
import { useAppUpdate } from "@/hooks/use-updates";
import changelogEn from "@/generated/changelog.md?raw";
import changelogPtBr from "@/generated/changelog.pt-BR.md?raw";
import changelogEs from "@/generated/changelog.es.md?raw";
import changelogJa from "@/generated/changelog.ja.md?raw";
import { PROTOCOL_VERSION } from "@/lib/protocol";
import { copyText, openInBrowser } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { locale, t } from "@/lib/i18n";

const REPO = "https://github.com/trbros/locdev";
const LICENSE = "https://www.apache.org/licenses/LICENSE-2.0";
const docs = () => `https://locdev.app/${locale}/docs`;

const ThirdParty = lazy(() => import("@/components/third-party"));

/** Changelog sections: `## [0.19.0] - 2026-11-23`. */
function releases() {
  const changelog = { en: changelogEn, "pt-BR": changelogPtBr, es: changelogEs, ja: changelogJa }[locale];
  const out: { version: string; date: string; body: string }[] = [];
  const parts = changelog.split(/^## \[/m).slice(1);
  for (const part of parts) {
    const head = /^([^\]]+)\]\s*-\s*(.*)$/m.exec(part);
    if (!head) continue;
    out.push({ version: head[1], date: head[2].trim(), body: part.slice(part.indexOf("\n") + 1).trim() });
  }
  return out;
}

/** The little Markdown the changelog uses: ### headings, - bullets, `code`. */
function ChangelogBody({ text }: { text: string }) {
  const blocks: { kind: "h" | "li" | "p"; text: string }[] = [];
  for (const line of text.split("\n")) {
    // `###` sections and the `####` groups inside them.
    if (/^#{3,4} /.test(line)) blocks.push({ kind: "h", text: line.replace(/^#+ /, "") });
    else if (line.startsWith("- ")) blocks.push({ kind: "li", text: line.slice(2) });
    else if (/^\s+\S/.test(line) && blocks.length && blocks[blocks.length - 1].kind !== "h")
      blocks[blocks.length - 1].text += ` ${line.trim()}`;
    else if (line.trim()) blocks.push({ kind: "p", text: line.trim() });
  }
  const inline = (s: string) =>
    s.split(/(`[^`]+`)/).map((part, i) =>
      part.startsWith("`") ? (
        <code key={i} className="rounded bg-foreground/6 px-1 font-mono text-[12px]">
          {part.slice(1, -1)}
        </code>
      ) : (
        part
      ),
    );
  return (
    <div className="flex flex-col gap-1.5 text-[13px] leading-relaxed">
      {blocks.map((b, i) =>
        b.kind === "h" ? (
          <h4 key={i} className="mt-2 font-mono text-[11px] tracking-wide text-muted-foreground uppercase">
            {b.text}
          </h4>
        ) : b.kind === "li" ? (
          <p key={i} className="relative pl-3.5 before:absolute before:left-0 before:text-ember before:content-['·']">
            {inline(b.text)}
          </p>
        ) : (
          <p key={i} className="text-muted-foreground">
            {inline(b.text)}
          </p>
        ),
      )}
    </div>
  );
}

function VersionHistory({ open, onClose, current }: { open: boolean; onClose: () => void; current?: string }) {
  const list = releases();
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("Version history")}</DialogTitle>
        </DialogHeader>
        <div className="-mx-1 max-h-[65vh] overflow-y-auto px-1" data-selectable>
          {list.map((r) => (
            <section key={r.version} className="border-b py-3 last:border-0">
              <h3 className="flex items-baseline gap-2">
                <span className="font-mono text-sm font-medium">{r.version}</span>
                <span className="text-[12px] text-muted-foreground">{r.date}</span>
                {r.version === current && (
                  <span className="rounded-full bg-ember/12 px-2 text-[11px] text-ember">{t("this app")}</span>
                )}
              </h3>
              <div className="mt-1.5">
                <ChangelogBody text={r.body} />
              </div>
            </section>
          ))}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function UpdateCheck() {
  const [asked, setAsked] = useState(false);
  const update = useAppUpdate(null, asked);
  const { navigate } = useUi();
  if (!asked) {
    return (
      <Button variant="outline" size="sm" onClick={() => setAsked(true)}>
        <ArrowDownToLine /> {t("Check for updates")}
      </Button>
    );
  }
  return (
    <span className="flex items-center gap-2 text-[13px]">
      {update.isFetching ? (
        <span className="text-muted-foreground">{t("Checking…")}</span>
      ) : update.isError ? (
        <span className="text-muted-foreground">
          <Led tone="fail" /> {t("Could not check for updates")}
        </span>
      ) : update.data?.version ? (
        <>
          <Led tone="warn" /> {t("LocDev {version} is available", { version: update.data.version })}
        </>
      ) : (
        <>
          <Led tone="live" /> {t("Up to date")}
        </>
      )}
      <Button variant="ghost" size="sm" onClick={() => navigate("settings", "updates")}>
        {t("Updates")}
      </Button>
    </span>
  );
}

export function AboutSection() {
  const info = useAppInfo();
  const status = useStatus();
  const { query: cli } = useCli();
  const [history, setHistory] = useState(false);
  const [thirdParty, setThirdParty] = useState(false);
  const app = info.data;
  const built = app?.built_at ? new Date(app.built_at * 1000).toLocaleString() : null;

  return (
    <>
      <div className="relative overflow-hidden rounded-xl border bg-chrome px-6 py-6">
        <div
          aria-hidden
          className="pointer-events-none absolute -top-24 -right-16 size-72 rounded-full bg-ember/10 blur-3xl"
        />
        <div className="relative flex items-center gap-5">
          <LogoMark className="size-16 shrink-0" />
          <div className="min-w-0">
            <h2 className="text-[24px] leading-none font-semibold tracking-[-0.02em]">{t("LocDev")}</h2>
            <p className="mt-2 max-w-lg text-[13px] text-muted-foreground">
              {t(
                "LocDev brings your projects and development servers together in one place. Start and stop projects, follow their logs, and access each site through a .test domain with local HTTPS, from the app or CLI. Open source for Linux, macOS and Windows.",
              )}
            </p>
            <p className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 font-mono text-[12px]">
              <span>
                {t("app")} <span className="text-ember">{app?.version ?? "—"}</span>
              </span>
              <span className="text-muted-foreground/50">·</span>
              <span>
                {t("daemon")} <span className="text-ember">{status.data?.version ?? "—"}</span>
              </span>
              <span className="text-muted-foreground/50">·</span>
              <span>
                {t("CLI")} <span className="text-ember">{cli.data?.version ?? app?.bundled_daemon ?? "—"}</span>
              </span>
            </p>
          </div>
        </div>
        <div className="relative mt-5 flex flex-wrap items-center gap-2">
          <UpdateCheck />
          <Button variant="ghost" size="sm" onClick={() => setHistory(true)}>
            <History /> {t("Version history")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => openInBrowser(docs())}>
            <BookOpen /> {t("Documentation")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => openInBrowser(REPO)}>
            <GitBranch /> {t("GitHub")}
          </Button>
        </div>
      </div>

      <Section title={t("build")}>
        <Fact label={t("App")} mono>
          {app?.version ?? "—"}
        </Fact>
        <Fact label={t("Daemon running")} mono>
          {status.data?.version ?? t("not running")}
        </Fact>
        <Fact label={t("Daemon and CLI in this app")} mono>
          {app?.bundled_daemon ?? "—"}
        </Fact>
        <Fact label={t("Commit")} mono>
          {app?.commit ? (
            <button
              type="button"
              className="hover:text-ember"
              title={t("Copy")}
              onClick={() => app.commit && copyText(app.commit)}
            >
              {app.commit}
            </button>
          ) : (
            t("unknown")
          )}
        </Fact>
        <Fact label={t("Built")} mono>
          {built ? `${built} · ${app?.profile}` : (app?.profile ?? "—")}
        </Fact>
        <Fact label={t("System")} mono>
          {app ? `${app.os} · ${app.arch}` : "—"}
        </Fact>
        <Fact label={t("Update channel")} mono>
          <button type="button" className="hover:text-ember" onClick={() => app && openInBrowser(app.releases)}>
            {t("stable · GitHub releases")}
          </button>
        </Fact>
        <Fact label={t("IPC protocol")} mono>
          {PROTOCOL_VERSION}
          {status.data &&
            status.data.protocol !== PROTOCOL_VERSION &&
            t(" (daemon: {protocol})", { protocol: status.data.protocol })}
        </Fact>
        <Fact label={t("Config")} mono>
          <span onDoubleClick={() => app && copyText(app.config_dir)}>{app?.config_dir ?? "—"}</span>
        </Fact>
      </Section>

      <Section title={t("license and credits")}>
        <p className="text-[13px] text-muted-foreground">
          {t(
            "LocDev is open source under the Apache License 2.0. It's built on open-source software: Tauri, React, Rust's tokio, hyper and rustls, and hundreds more, each under its own license.",
          )}
        </p>
        <div className="mt-3 flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={() => openInBrowser(LICENSE)}>
            <Scale /> {t("Apache License 2.0")}
          </Button>
          <Button variant="outline" size="sm" onClick={() => setThirdParty(true)}>
            <Package /> {t("Third-party software")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => openInBrowser("https://locdev.app")}>
            <ExternalLink /> {t("App homepage")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => openInBrowser(`${REPO}/blob/main/CHANGELOG.md`)}>
            <FileText /> {t("CHANGELOG.md")}
          </Button>
        </div>
      </Section>

      <VersionHistory open={history} onClose={() => setHistory(false)} current={app?.version} />
      {thirdParty && (
        <Suspense fallback={<Skeleton className="h-10" />}>
          <ThirdParty onClose={() => setThirdParty(false)} />
        </Suspense>
      )}
    </>
  );
}
