// The Mailbox for the mock backend: a few captured messages. The SMTP
// server, parsing and wiring live in `locdev-mail`.

import { DaemonError } from "./errors";
import type { MailDetail, MailRequest, MailResponse, MailSetup, MailSummary } from "./protocol";

const now = Date.now();
const html = `<!doctype html><html><body style="font-family:Arial,sans-serif;background:#f4f4f5;padding:24px">
<table width="560" style="background:#fff;border-radius:8px;padding:24px;margin:auto"><tr><td>
<h1 style="font-size:20px;color:#111">Welcome to Acme Shop, Ana!</h1>
<p style="color:#333">Confirm your e-mail to start ordering.</p>
<p><a href="https://acme-shop.test/confirm?token=6f1c" style="background:#ea580c;color:#fff;padding:10px 18px;border-radius:6px;text-decoration:none">Confirm e-mail</a></p>
<img src="https://tracker.example.com/pixel.gif" width="1" height="1" alt="">
<p style="color:#666;font-size:12px">Questions? <a href="https://acme-shop.test/help">Help center</a></p>
<script>document.body.innerHTML = "scripts must not run"</script>
</td></tr></table></body></html>`;

const messages: (MailDetail & { raw: string })[] = [
  {
    raw: "",
    summary: {
      id: "1793700000300",
      site: "acme-shop",
      from: "Acme Shop <hello@acme-shop.test>",
      to: ["ana@example.com"],
      subject: "Welcome to Acme Shop",
      date: now - 120_000,
      received_at: now - 120_000,
      size: 4812,
      attachments: 0,
      has_html: true,
      preview: "Welcome to Acme Shop, Ana! Confirm your e-mail to start ordering.",
      read: false,
    },
    envelope: {
      from: "hello@acme-shop.test",
      to: ["ana@example.com"],
      helo: "acme-shop.test",
      auth_user: "acme-shop",
      protocol: "ESMTPA",
      peer: "127.0.0.1:52144",
      received_at: now - 120_000,
    },
    headers: [
      { name: "From", value: "Acme Shop <hello@acme-shop.test>" },
      { name: "To", value: "ana@example.com" },
      { name: "Subject", value: "Welcome to Acme Shop" },
      { name: "Message-ID", value: "<b2c1@acme-shop.test>" },
      { name: "MIME-Version", value: "1.0" },
      { name: "Content-Type", value: 'multipart/alternative; boundary="=_x"' },
    ],
    cc: [],
    reply_to: [],
    html,
    text: "Welcome to Acme Shop, Ana!\n\nConfirm your e-mail to start ordering:\nhttps://acme-shop.test/confirm?token=6f1c\n",
    text_generated: false,
    attachments: [],
    links: [
      { url: "https://acme-shop.test/confirm?token=6f1c", text: "Confirm e-mail" },
      { url: "https://acme-shop.test/help", text: "Help center" },
    ],
    problems: ["a line is longer than 998 characters (RFC 5322 §2.1.1)"],
    framework: "Laravel + Vue + Vite",
  },
  {
    raw: "",
    summary: {
      id: "1793700000200",
      site: "acme-shop",
      from: "Acme Shop <billing@acme-shop.test>",
      to: ["ana@example.com"],
      subject: "Invoice #1042",
      date: now - 3_600_000,
      received_at: now - 3_600_000,
      size: 38_211,
      attachments: 1,
      has_html: false,
      preview: "Your invoice for October is attached.",
      read: true,
    },
    envelope: {
      from: "billing@acme-shop.test",
      to: ["ana@example.com"],
      helo: "acme-shop.test",
      auth_user: "acme-shop",
      protocol: "ESMTPA",
      peer: "127.0.0.1:52090",
      received_at: now - 3_600_000,
    },
    headers: [
      { name: "From", value: "Acme Shop <billing@acme-shop.test>" },
      { name: "Subject", value: "Invoice #1042" },
    ],
    cc: ["finance@acme-shop.test"],
    reply_to: [],
    html: null,
    text: "Your invoice for October is attached.\n",
    text_generated: false,
    attachments: [
      {
        index: 0,
        name: "invoice-1042.pdf",
        content_type: "application/pdf",
        size: 36_120,
        content_id: null,
        inline: false,
      },
    ],
    links: [],
    problems: [],
    framework: "Laravel + Vue + Vite",
  },
  {
    raw: "",
    summary: {
      id: "1793700000100",
      site: "blog",
      from: "blog@blog.test",
      to: ["me@example.com"],
      subject: "New comment on “Hello world”",
      date: now - 86_400_000,
      received_at: now - 86_400_000,
      size: 902,
      attachments: 0,
      has_html: false,
      preview: "Someone left a comment on your post.",
      read: true,
    },
    envelope: {
      from: "blog@blog.test",
      to: ["me@example.com"],
      helo: "localhost",
      auth_user: "blog",
      protocol: "ESMTPA",
      peer: "127.0.0.1:51001",
      received_at: now - 86_400_000,
    },
    headers: [{ name: "Subject", value: "New comment on “Hello world”" }],
    cc: [],
    reply_to: [],
    html: null,
    text: "Someone left a comment on your post.\nhttps://blog.test/hello-world#comments\n",
    text_generated: false,
    attachments: [],
    links: [{ url: "https://blog.test/hello-world#comments", text: null }],
    problems: [],
    framework: "Next.js",
  },
];

const enabled: Record<string, boolean> = { "acme-shop": true };

function setup(name: string): MailSetup {
  const laravel = name === "acme-shop";
  return {
    site: name,
    framework: laravel ? "Laravel + Vue + Vite" : "Next.js",
    enabled: enabled[name] ?? false,
    wiring: laravel ? "automatic" : "manual",
    current: laravel ? ["MAIL_MAILER=log", "MAIL_HOST=127.0.0.1", "MAIL_PORT=2525"] : [],
    real_smtp: false,
    injects: laravel
      ? [
          { name: "MAIL_MAILER", value: "smtp" },
          { name: "MAIL_HOST", value: "127.0.0.1" },
          { name: "MAIL_PORT", value: "1025" },
          { name: "MAIL_USERNAME", value: name },
          { name: "MAIL_PASSWORD", value: "locdev" },
          { name: "MAIL_SCHEME", value: "smtp" },
        ]
      : [],
    snippet: laravel
      ? null
      : 'const transport = nodemailer.createTransport({\n  host: "127.0.0.1",\n  port: 1025,\n  secure: false,\n});\n',
    notes: laravel
      ? ["Queue workers and the scheduler get the same variables, so queued mail is captured too."]
      : ["Next.js reads its SMTP settings from code; add the snippet (development only)."],
  };
}

const summary = (m: (typeof messages)[number]): MailSummary => m.summary;

export function handleMail(req: MailRequest): MailResponse {
  switch (req.type) {
    case "mailbox_status":
      return {
        type: "mailbox_status",
        enabled: true,
        port: 1025,
        error: null,
        messages: messages.length,
        unread: messages.filter((m) => !m.summary.read).length,
      };
    case "mail_list": {
      const q = req.query?.toLowerCase();
      return {
        type: "mail_list",
        messages: messages
          .filter((m) => !req.site || m.summary.site === req.site)
          .filter((m) => !req.unread || !m.summary.read)
          .filter((m) => !req.attachments || m.summary.attachments > 0)
          .filter(
            (m) =>
              !q ||
              [m.summary.subject, m.summary.from, m.summary.preview, ...m.summary.to].some((f) =>
                f.toLowerCase().includes(q),
              ),
          )
          .map(summary),
      };
    }
    case "mail_get": {
      const m = messages.find((x) => x.summary.id === req.id);
      if (!m) throw new DaemonError("app", `no message ${req.id}`, "NOT_FOUND");
      m.summary.read = true;
      const { raw: _, ...detail } = m;
      return { type: "mail_detail", ...structuredClone(detail) };
    }
    case "mail_raw":
      return { type: "mail_raw", base64: btoa("Subject: mock\r\n\r\nmock\r\n") };
    case "mail_attachment":
      return {
        type: "mail_attachment",
        name: "invoice-1042.pdf",
        content_type: "application/pdf",
        base64: btoa("%PDF-1.4"),
      };
    case "mail_delete": {
      let count = 0;
      for (const id of req.ids) {
        const i = messages.findIndex((m) => m.summary.id === id);
        if (i >= 0) {
          messages.splice(i, 1);
          count++;
        }
      }
      return { type: "mail_deleted", count };
    }
    case "mail_clear": {
      const before = messages.length;
      for (let i = messages.length - 1; i >= 0; i--)
        if (!req.site || messages[i].summary.site === req.site) messages.splice(i, 1);
      return { type: "mail_deleted", count: before - messages.length };
    }
    case "mail_setup":
      return { type: "mail_setup", ...setup(req.name) };
    case "mail_enable":
      enabled[req.name] = req.enabled;
      return { type: "mail_setup", ...setup(req.name) };
    case "mail_send_test": {
      const id = String(Date.now());
      messages.unshift({
        raw: "",
        summary: {
          id,
          site: req.name ?? null,
          from: "locdev@locdev.test",
          to: ["you@example.test"],
          subject: "LocDev test message",
          date: Date.now(),
          received_at: Date.now(),
          size: 640,
          attachments: 0,
          has_html: true,
          preview: "This message was sent by LocDev to its own Mailbox.",
          read: false,
        },
        envelope: {
          from: "locdev@locdev.test",
          to: ["you@example.test"],
          helo: "locdev",
          auth_user: req.name ?? null,
          protocol: "ESMTPA",
          peer: "127.0.0.1:50000",
          received_at: Date.now(),
        },
        headers: [{ name: "Subject", value: "LocDev test message" }],
        cc: [],
        reply_to: [],
        html: "<p>This message was sent by <strong>LocDev</strong> to its own Mailbox.</p>",
        text: "This message was sent by LocDev to its own Mailbox. Nothing left this machine.",
        text_generated: false,
        attachments: [],
        links: [],
        problems: [],
        framework: null,
      });
      return { type: "ok" } as unknown as MailResponse;
    }
  }
}
