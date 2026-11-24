import { Download, ExternalLink, Inbox, Mail, Paperclip, Search, Send, ShieldAlert, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";

import { useCrumbs, ViewHeading } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { errorMessage, useProjects } from "@/hooks/use-daemon";
import { type MailFilter, useMailActions, useMailbox, useMailDetail, useMailList } from "@/hooks/use-mail";
import { subById, viewById } from "@/lib/nav";
import type { MailDetail, MailSummary } from "@/lib/protocol";
import { openInBrowser } from "@/lib/system";
import { cn } from "@/lib/utils";
import { intlLocale, t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

/** Nothing in a message runs or loads: no scripts, no network, no navigation. */
const CSP = "default-src 'none'; img-src data:; style-src 'unsafe-inline'; font-src data:";

/** Remote resources a message would load (blocked here, listed for the user). */
function remoteResources(html: string): string[] {
  const found = new Set<string>();
  for (const m of html.matchAll(/\b(?:src|background)\s*=\s*["']?(https?:\/\/[^"'\s>]+)/gi)) found.add(m[1]);
  for (const m of html.matchAll(/url\(\s*["']?(https?:\/\/[^"')\s]+)/gi)) found.add(m[1]);
  return [...found];
}

function srcdoc(html: string) {
  const meta = `<meta http-equiv="Content-Security-Policy" content="${CSP}"><base target="_blank">`;
  return /<head[^>]*>/i.test(html) ? html.replace(/<head[^>]*>/i, (h) => `${h}${meta}`) : `${meta}${html}`;
}

function time(ms: number) {
  const d = new Date(ms);
  const today = new Date().toDateString() === d.toDateString();
  return today
    ? d.toLocaleTimeString(intlLocale, { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(intlLocale, { month: "short", day: "numeric" });
}

function size(bytes: number) {
  return bytes < 1024
    ? `${bytes} B`
    : bytes < 1048576
      ? `${(bytes / 1024).toFixed(1)} KB`
      : `${(bytes / 1048576).toFixed(1)} MB`;
}

export function MailboxView() {
  const view = viewById("tools");
  const crumbs = useCrumbs(view, subById(view, "mailbox")!);
  const projects = useProjects();
  const status = useMailbox();
  const actions = useMailActions();
  const [filter, setFilter] = useState<MailFilter>({ site: null, query: "", unread: false, attachments: false });
  const [selected, setSelected] = useState<string | null>(null);
  const list = useMailList(filter);
  const messages = list.data ?? [];
  const current = selected && messages.some((m) => m.id === selected) ? selected : (messages[0]?.id ?? null);
  const sites = useMemo(() => (projects.data ?? []).map((p) => p.name), [projects.data]);

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-col gap-3 border-b px-8 pt-5 pb-3">
        <ViewHeading
          view={view}
          title={t("Mailbox")}
          crumbs={crumbs}
          description={
            status.data?.port
              ? t("SMTP on 127.0.0.1:{port}. Capture only: nothing is delivered to real recipients.", {
                  port: status.data.port,
                })
              : (status.data?.error ?? "starting…")
          }
          actions={
            <span className="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={() => actions.test.mutate(filter.site)}
                disabled={!status.data?.port}
              >
                <Send data-icon="inline-start" /> {t("Send a test")}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={messages.length === 0}
                onClick={() => {
                  if (
                    window.confirm(
                      filter.site
                        ? t("Delete every message from {site}?", { site: filter.site })
                        : t("Delete every captured message?"),
                    )
                  )
                    actions.clear.mutate(filter.site);
                }}
              >
                <Trash2 data-icon="inline-start" /> {t("Clear")}
              </Button>
            </span>
          }
        />
        <div className="flex flex-wrap items-center gap-3">
          <div className="relative">
            <Search className="pointer-events-none absolute top-1/2 left-2 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              aria-label={t("Search mail")}
              placeholder={t("subject, sender, recipient…")}
              value={filter.query}
              onChange={(e) => setFilter({ ...filter, query: e.target.value })}
              className="h-7 w-72 pl-7 text-[12.5px]"
            />
          </div>
          <label className="flex items-center gap-1.5 text-[12.5px]">
            <Switch
              aria-label={t("Unread only")}
              checked={filter.unread}
              onCheckedChange={(unread) => setFilter({ ...filter, unread })}
              className="data-checked:bg-ember"
            />
            {t("unread")}
          </label>
          <label className="flex items-center gap-1.5 text-[12.5px]">
            <Switch
              aria-label={t("With attachments")}
              checked={filter.attachments}
              onCheckedChange={(attachments) => setFilter({ ...filter, attachments })}
              className="data-checked:bg-ember"
            />
            {t("with attachments")}
          </label>
        </div>
      </div>
      <div className="flex min-h-0 flex-1">
        <nav aria-label={t("Inboxes")} className="flex w-44 shrink-0 flex-col gap-0.5 border-r p-2">
          {[null, ...sites].map((site) => (
            <button
              type="button"
              key={site ?? "*"}
              onClick={() => setFilter({ ...filter, site })}
              className={cn(
                "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-[13px] text-muted-foreground hover:bg-foreground/5 hover:text-foreground",
                filter.site === site && "bg-foreground/[0.07] text-foreground",
              )}
            >
              <Inbox className="size-3.5 shrink-0" />
              <span className="truncate">{site ?? t("All sites")}</span>
            </button>
          ))}
        </nav>
        <ScrollArea className="w-80 shrink-0 border-r">
          {list.isPending ? (
            <Skeleton className="m-3 h-40" />
          ) : list.isError ? (
            <p className="p-4 text-sm text-destructive">{errorMessage(list.error)}</p>
          ) : messages.length === 0 ? (
            <p className="p-4 text-[12.5px] text-muted-foreground">
              {tx(
                "No messages. Turn the Mailbox on for a site (its Settings tab, or {locdev_mail_enable}) and the mail it sends lands here.",
                { locdev_mail_enable: <code>locdev mail enable</code> },
              )}
            </p>
          ) : (
            <ul aria-label={t("Messages")} className="divide-y">
              {messages.map((m) => (
                <MessageRow key={m.id} m={m} active={m.id === current} onClick={() => setSelected(m.id)} />
              ))}
            </ul>
          )}
        </ScrollArea>
        <div className="min-w-0 flex-1">{current ? <MessagePane id={current} /> : null}</div>
      </div>
    </div>
  );
}

function MessageRow({ m, active, onClick }: { m: MailSummary; active: boolean; onClick: () => void }) {
  return (
    <li>
      <button
        type="button"
        onClick={onClick}
        className={cn(
          "flex w-full flex-col gap-0.5 px-3 py-2 text-left hover:bg-foreground/[0.03]",
          active && "bg-foreground/[0.06]",
        )}
      >
        <span className="flex items-center gap-1.5 text-[12px]">
          {!m.read && <span className="size-1.5 rounded-full bg-ember" aria-label={t("unread")} />}
          <span className="truncate text-muted-foreground">{m.to.join(", ")}</span>
          <span className="ml-auto shrink-0 font-mono text-[10.5px] text-muted-foreground">{time(m.received_at)}</span>
        </span>
        <span className={cn("truncate text-[13px]", !m.read && "font-medium")}>{m.subject || t("(no subject)")}</span>
        <span className="flex items-center gap-1.5 text-[11.5px] text-muted-foreground">
          {m.site && <span className="font-mono text-[10.5px] text-ember">{m.site}</span>}
          {m.attachments > 0 && (
            <Paperclip className="size-3" aria-label={tn("{n} attachment", "{n} attachments", m.attachments)} />
          )}
          <span className="truncate">{m.preview}</span>
        </span>
      </button>
    </li>
  );
}

type Tab = "html" | "text" | "headers" | "links" | "attachments" | "debug";

function MessagePane({ id }: { id: string }) {
  const detail = useMailDetail(id);
  const actions = useMailActions();
  const [tab, setTab] = useState<Tab | null>(null);
  const [width, setWidth] = useState<"desktop" | "mobile">("desktop");
  if (detail.isPending) return <Skeleton className="m-6 h-64" />;
  if (detail.isError || !detail.data)
    return <p className="p-6 text-sm text-destructive">{errorMessage(detail.error)}</p>;
  const d = detail.data;
  const s = d.summary;
  const active: Tab = tab ?? (d.html ? "html" : "text");
  const tabs: [Tab, string][] = [
    ["html", "HTML"],
    ["text", t("Text")],
    ["headers", t("Headers ({length})", { length: d.headers.length })],
    ["links", t("Links ({length})", { length: d.links.length })],
    ["attachments", t("Attachments ({length})", { length: d.attachments.length })],
    ["debug", d.problems.length ? t("Debug · {length} problems", { length: d.problems.length }) : "Debug"],
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="flex flex-col gap-1 border-b px-5 py-3">
        <div className="flex items-start gap-2">
          <h3 className="text-[15px] font-medium">{s.subject || t("(no subject)")}</h3>
          <span className="ml-auto flex shrink-0 gap-1">
            <Button size="xs" variant="ghost" onClick={() => actions.save(s.id, null, `${s.id}.eml`)}>
              <Download data-icon="inline-start" /> {t(".eml")}
            </Button>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t("Delete message")}
              onClick={() => actions.remove.mutate([s.id])}
            >
              <Trash2 />
            </Button>
          </span>
        </div>
        <dl className="grid grid-cols-[4rem_1fr] gap-x-2 text-[12.5px]">
          <dt className="text-muted-foreground">{t("From")}</dt>
          <dd className="truncate">{s.from}</dd>
          <dt className="text-muted-foreground">{t("To")}</dt>
          <dd className="truncate">{s.to.join(", ")}</dd>
          {d.cc.length > 0 && (
            <>
              <dt className="text-muted-foreground">{t("Cc")}</dt>
              <dd className="truncate">{d.cc.join(", ")}</dd>
            </>
          )}
          <dt className="text-muted-foreground">{t("Date")}</dt>
          <dd>
            {s.date ? (
              new Date(s.date).toLocaleString(intlLocale)
            ) : (
              <span className="text-muted-foreground">{t("no Date header")}</span>
            )}
          </dd>
          {s.site && (
            <>
              <dt className="text-muted-foreground">{t("Site")}</dt>
              <dd>
                <span className="font-mono text-[12px] text-ember">{s.site}</span>
                {d.framework && <span className="text-muted-foreground"> · {d.framework}</span>}
              </dd>
            </>
          )}
        </dl>
      </div>
      <div className="flex gap-1 overflow-x-auto border-b px-4 py-1.5 [scrollbar-width:none]">
        <div role="tablist" aria-label={t("Message view")} className="flex gap-1">
          {tabs.map(([t, label]) => (
            <button
              type="button"
              role="tab"
              key={t}
              aria-selected={active === t}
              disabled={t === "html" && !d.html}
              onClick={() => setTab(t)}
              className={cn(
                "shrink-0 rounded-md px-2 py-0.5 font-mono text-[11.5px] whitespace-nowrap text-muted-foreground hover:text-foreground disabled:opacity-40",
                active === t && "bg-foreground/[0.07] text-foreground",
                t === "debug" && d.problems.length > 0 && "text-amber",
              )}
            >
              {label}
            </button>
          ))}
        </div>
        {active === "html" && (
          <span className="ml-auto flex gap-1">
            {(["desktop", "mobile"] as const).map((w) => (
              <button
                type="button"
                key={w}
                onClick={() => setWidth(w)}
                aria-pressed={width === w}
                className={cn(
                  "rounded px-1.5 font-mono text-[11px] text-muted-foreground",
                  width === w && "bg-foreground/[0.07] text-foreground",
                )}
              >
                {w}
              </button>
            ))}
          </span>
        )}
      </div>
      {/* biome-ignore lint/a11y/noNoninteractiveTabindex: a scrollable region must take focus to scroll by keyboard */}
      <div
        tabIndex={0}
        aria-label={t("Message")}
        className="min-h-0 flex-1 overflow-auto outline-none focus-visible:ring-2 focus-visible:ring-ring/50"
      >
        {active === "html" && d.html ? (
          <HtmlView html={d.html} width={width} />
        ) : active === "text" ? (
          <div className="p-5">
            {d.text_generated && (
              <p className="mb-2 text-[11.5px] text-muted-foreground">
                {t("Generated from the HTML: the message has no text part.")}
              </p>
            )}
            <pre className="font-mono text-[12.5px] whitespace-pre-wrap" data-selectable>
              {d.text ?? "(no text)"}
            </pre>
          </div>
        ) : active === "headers" ? (
          <table className="m-5 font-mono text-[12px]" data-selectable>
            <tbody>
              {d.headers.map((h, i) => (
                <tr key={`${h.name}-${i}`} className="align-top">
                  <td className="pr-3 pb-1 whitespace-nowrap text-signal">{h.name}</td>
                  <td className="pb-1 break-all">{h.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : active === "links" ? (
          <ul className="flex flex-col gap-1 p-5 text-[12.5px]">
            {d.links.length === 0 && <li className="text-muted-foreground">{t("No links.")}</li>}
            {d.links.map((l) => (
              <li key={l.url} className="flex items-baseline gap-2">
                <button
                  type="button"
                  onClick={() => openInBrowser(l.url)}
                  className="truncate font-mono text-ember hover:underline"
                  title={t("Open in the browser")}
                >
                  {l.url}
                </button>
                {l.text && <span className="shrink-0 text-muted-foreground">“{l.text}”</span>}
                <ExternalLink className="size-3 shrink-0 text-muted-foreground" />
              </li>
            ))}
          </ul>
        ) : active === "attachments" ? (
          <ul className="flex flex-col gap-1 p-5 text-[12.5px]">
            {d.attachments.length === 0 && <li className="text-muted-foreground">{t("No attachments.")}</li>}
            {d.attachments.map((a) => (
              <li key={a.index} className="flex items-center gap-2">
                <Paperclip className="size-3.5 text-muted-foreground" />
                <span>{a.name ?? t("(unnamed)")}</span>
                <span className="font-mono text-[11px] text-muted-foreground">
                  {a.content_type} · {size(a.size)}
                  {a.inline ? t(" · inline") : ""}
                </span>
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => actions.save(s.id, a.index, a.name ?? `attachment-${a.index}`)}
                >
                  <Download data-icon="inline-start" /> {t("Save")}
                </Button>
              </li>
            ))}
          </ul>
        ) : (
          <DebugView d={d} />
        )}
      </div>
    </div>
  );
}

function HtmlView({ html, width }: { html: string; width: "desktop" | "mobile" }) {
  const blocked = remoteResources(html);
  return (
    <div className="flex h-full flex-col">
      {blocked.length > 0 && (
        <p
          className="flex items-center gap-1.5 border-b px-5 py-1.5 text-[11.5px] text-muted-foreground"
          title={blocked.join("\n")}
        >
          <ShieldAlert className="size-3.5" />{" "}
          {tn(
            "{n} remote resource not loaded (trackers can tell a message was opened). Listed in Debug.",
            "{n} remote resources not loaded (trackers can tell a message was opened). Listed in Debug.",
            blocked.length,
          )}
        </p>
      )}
      <div className="flex-1 bg-white">
        {/* sandbox="": no scripts, forms, navigation or popups; the CSP blocks every network load. */}
        <iframe
          title={t("Message HTML")}
          sandbox=""
          srcDoc={srcdoc(html)}
          className={cn("mx-auto h-full min-h-[480px] border-0", width === "mobile" ? "w-[375px] border-x" : "w-full")}
        />
      </div>
    </div>
  );
}

function DebugView({ d }: { d: MailDetail }) {
  const e = d.envelope;
  const rows: [string, string][] = [
    [t("Project"), d.summary.site ?? t("not identified (the session didn't authenticate as a site)")],
    [t("Framework"), d.framework ?? "—"],
    [t("Received"), new Date(e.received_at).toLocaleString(intlLocale)],
    [
      t("Sent (Date header)"),
      d.summary.date ? new Date(d.summary.date).toLocaleString(intlLocale) : t("no Date header"),
    ],
    [t("Size"), size(d.summary.size)],
    [t("Transport"), t("{protocol} from {peer}", { protocol: e.protocol, peer: e.peer })],
    ["HELO", e.helo ?? "—"],
    [t("AUTH user"), e.auth_user ?? t("none")],
    ["MAIL FROM", e.from || t("(empty)")],
    ["RCPT TO", e.to.join(", ")],
  ];
  const remote = d.html ? remoteResources(d.html) : [];
  return (
    <div className="flex flex-col gap-4 p-5 text-[12.5px]">
      <dl className="grid grid-cols-[10rem_1fr] gap-x-3 gap-y-1">
        {rows.map(([k, v]) => (
          <div key={k} className="contents">
            <dt className="text-muted-foreground">{k}</dt>
            <dd className="font-mono text-[12px] break-all">{v}</dd>
          </div>
        ))}
      </dl>
      <section>
        <h4 className="mb-1 flex items-center gap-1.5 font-medium">
          <Mail className="size-3.5" /> {t("Formatting")}
        </h4>
        {d.problems.length === 0 ? (
          <p className="text-muted-foreground">
            {t("No problems found by LocDev's checks (RFC 5322 headers, MIME decoding, cid references, line length).")}
          </p>
        ) : (
          <ul className="list-disc pl-5 text-amber">
            {d.problems.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        )}
      </section>
      {remote.length > 0 && (
        <section>
          <h4 className="mb-1 font-medium">{t("Remote resources (not loaded)")}</h4>
          <ul className="font-mono text-[11.5px] break-all text-muted-foreground">
            {remote.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
