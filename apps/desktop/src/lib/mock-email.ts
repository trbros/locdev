// Email Studio for the mock backend: one Laravel template kept as segments,
// a simplified renderer and the same rules (hash on save, no overwrite).
// The real parser, emitter and previews live in `locdev-email`.

import { DaemonError } from "./errors";
import type { EmailCatalog, EmailRequest, EmailResponse, JsonValue, Segment } from "./protocol";

const theme: Record<string, JsonValue> = {
  width: 600,
  background: "#f4f4f5",
  card: "#ffffff",
  text: "#18181b",
  muted: "#71717a",
  accent: "#ea580c",
  font: "Arial, Helvetica, sans-serif",
};

const head = `{{-- locdev:email ${JSON.stringify(theme)} --}}\n<!DOCTYPE html>\n<html><body style="margin:0;background:#f4f4f5">\n<table role="presentation" width="100%"><tr><td align="center" style="padding:24px 12px">\n<table role="presentation" width="600" style="max-width:600px;background:#fff;border-radius:8px;font-family:Arial,sans-serif;color:#18181b">\n`;
const tail = `</table>\n</td></tr></table>\n</body></html>\n`;

interface MockFile {
  segments: Segment[];
  hash: string;
  format: "blade" | "markdown";
}

const files: Record<string, MockFile> = {
  "resources/views/emails/welcome.blade.php": {
    format: "blade",
    hash: "h1",
    segments: [
      { kind: "code", id: "c0", code: head },
      {
        kind: "block",
        id: "b0",
        block: "header",
        props: { title: "{{ config('app.name') }}", align: "center" },
        edited: false,
      },
      {
        kind: "block",
        id: "b1",
        block: "heading",
        props: { text: "Welcome, {{ $user->name }}!", level: 1 },
        edited: false,
      },
      {
        kind: "block",
        id: "b2",
        block: "text",
        props: { text: "Thanks for signing up. Confirm your e-mail to start ordering." },
        edited: false,
      },
      {
        kind: "code",
        id: "c1",
        code: '@if ($user->isAdmin())\n<tr><td style="padding:8px 32px"><p>You have admin access.</p></td></tr>\n@endif\n',
      },
      { kind: "block", id: "b3", block: "button", props: { text: "Confirm e-mail", url: "{{ $url }}" }, edited: false },
      {
        kind: "block",
        id: "b4",
        block: "footer",
        props: { text: "© {{ date('Y') }} {{ config('app.name') }}" },
        edited: false,
      },
      { kind: "code", id: "c2", code: tail },
    ],
  },
};

const catalog = (): EmailCatalog => ({
  framework: "Laravel + Vue + Vite",
  templates: [
    {
      path: "resources/views/emails/welcome.blade.php",
      name: "emails.welcome",
      format: "blade",
      mailables: [
        {
          class: "App\\Mail\\WelcomeMail",
          path: "app/Mail/WelcomeMail.php",
          variables: ["user", "url"],
          subject: "Welcome",
        },
      ],
      has_blocks: true,
    },
    {
      path: "resources/views/mail/invitation.blade.php",
      name: "mail.invitation",
      format: "text",
      mailables: [
        {
          class: "App\\Mail\\InvitationMail",
          path: "app/Mail/InvitationMail.php",
          variables: ["organization", "url"],
          subject: null,
        },
      ],
      has_blocks: false,
    },
    ...Object.keys(files)
      .filter((p) => p !== "resources/views/emails/welcome.blade.php")
      .map((path) => ({
        path,
        name: path.replace("resources/views/", "").replace(".blade.php", "").replaceAll("/", "."),
        format: files[path].format,
        mailables: [],
        has_blocks: true,
      })),
  ],
  components: [
    {
      name: "button",
      path: "resources/views/components/mail/button.blade.php",
      usage: "x-mail.button",
      props: [
        { name: "url", default: null },
        { name: "color", default: "'#ea580c'" },
      ],
      slot: true,
      description: "A branded button",
      builtin: false,
    },
    {
      name: "button",
      path: "",
      usage: "x-mail::button",
      props: [
        { name: "url", default: null },
        { name: "color", default: null },
      ],
      slot: true,
      description: "Laravel Markdown mail component",
      builtin: true,
    },
  ],
  conventions: [
    "Templates: resources/views/emails/<name>.blade.php, with a Mailable in app/Mail",
    "Components: resources/views/components/mail/<name>.blade.php, used as <x-mail.name>, props from @props([...])",
  ],
  native: ["php artisan make:mail <Name>Mail --view=emails.<name> (used by `locdev new email`)"],
  framework_preview: true,
});

const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

function sub(text: string, data: Record<string, JsonValue>) {
  return text.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (_, expr: string) => {
    const m = /^\$(\w+)(?:->(\w+))?/.exec(expr);
    if (m) {
      const v = data[m[1]] as JsonValue;
      const value =
        m[2] && v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, JsonValue>)[m[2]] : v;
      if (value !== undefined) return esc(String(value));
    }
    if (expr.startsWith("config(")) return "Acme Shop";
    if (expr.startsWith("date(")) return String(new Date().getFullYear());
    return `‹${esc(expr)}›`;
  });
}

function block(s: Extract<Segment, { kind: "block" }>, tagged: boolean) {
  const p = s.props as Record<string, string>;
  const data = tagged ? ` data-locdev-block="${s.id}"` : "";
  const td = (pad: string, body: string) => `<tr${data}><td style="padding:${pad}">${body}</td></tr>`;
  switch (s.block) {
    case "heading":
      return td("24px 32px 8px", `<h1 style="margin:0;font-size:24px">${p.text ?? ""}</h1>`);
    case "text":
      return td(
        "8px 32px",
        `<p style="margin:0;font-size:15px;line-height:1.6">${(p.text ?? "").replace(/\n/g, "<br>")}</p>`,
      );
    case "button":
      return td(
        "16px 32px",
        `<a href="${p.url ?? "#"}" style="display:inline-block;padding:12px 22px;background:${p.background ?? "#ea580c"};color:#fff;border-radius:6px;text-decoration:none;font-weight:bold">${p.text ?? "Button"}</a>`,
      );
    case "header":
      return td(
        "24px 32px 8px",
        `<div style="text-align:center;font-size:18px;font-weight:bold">${p.title ?? ""}</div>`,
      );
    case "footer":
      return td("24px 32px", `<p style="margin:0;font-size:12px;color:#71717a;text-align:center">${p.text ?? ""}</p>`);
    case "divider":
      return td("16px 32px", `<hr style="border:0;border-top:1px solid #e4e4e7;margin:0">`);
    case "spacer":
      return `<tr${data}><td style="height:${p.height ?? 24}px"></td></tr>`;
    case "image":
      return td(
        "8px 32px",
        `<img src="${p.src ?? ""}" alt="${p.alt ?? ""}" width="${p.width ?? 536}" style="max-width:100%">`,
      );
    case "component":
      return td(
        "8px 32px",
        `<a href="#" style="display:inline-block;padding:12px 22px;background:#ea580c;color:#fff;border-radius:6px">${(s.props.slot as string) ?? ""}</a>`,
      );
    default:
      return td("8px 32px", `<p>${esc(s.block)}</p>`);
  }
}

function emit(segments: Segment[], tagged = false) {
  return segments
    .map((s) =>
      s.kind === "code"
        ? s.code
        : `{{-- locdev:block ${JSON.stringify({ type: s.block, props: s.props })} --}}\n${block(s, tagged)}\n{{-- /locdev:block --}}\n`,
    )
    .join("");
}

function render(segments: Segment[], data: JsonValue) {
  const content = emit(segments, true)
    .replace(/\{\{--[\s\S]*?--\}\}/g, "")
    .split("\n")
    .filter((l) => !/^\s*@(if|else|endif|foreach|endforeach)\b/.test(l))
    .join("\n");
  return sub(content, (data ?? {}) as Record<string, JsonValue>);
}

const doc = (path: string) => {
  const f = files[path];
  if (!f) throw new DaemonError("app", `${path} isn't an email template LocDev found in this project`, "NOT_FOUND");
  return f;
};

export function handleEmail(req: EmailRequest): EmailResponse {
  switch (req.type) {
    case "email_catalog":
      return { type: "email_catalog", ...catalog() };
    case "email_open": {
      if (req.path === "resources/views/mail/invitation.blade.php") {
        const code = "{{ __('invite.subject', ['organization' => $organization]) }}\n\n{{ $url }}\n";
        return {
          type: "email_doc",
          path: req.path,
          format: "text",
          content: code,
          hash: "t1",
          segments: [{ kind: "code", id: "c0", code }],
          theme,
          variables: ["organization", "url"],
          sample: { organization: "Acme", url: "https://example.test/accept" },
          notes: [
            "Text templates are edited as code here; LocDev's visual blocks work in Blade, Markdown mailables and HTML.",
          ],
        };
      }
      const f = doc(req.path);
      return {
        type: "email_doc",
        path: req.path,
        format: f.format,
        content: emit(f.segments),
        hash: f.hash,
        segments: structuredClone(f.segments),
        theme,
        variables: ["user", "url"],
        sample: { user: { name: "Ana" }, url: "https://acme-shop.test/confirm" },
        notes: [],
      };
    }
    case "email_stat":
      return { type: "email_stat", hash: files[req.path]?.hash ?? "t1" };
    case "email_preview": {
      const segs = req.segments ?? files[req.path]?.segments ?? [];
      const html = render(segs, req.data);
      return {
        type: "email_preview",
        html: req.mode === "framework" ? html.replace(/data-locdev-block="[^"]*"/g, "") : html,
        text: html
          .replace(/<[^>]+>/g, " ")
          .replace(/\s+/g, " ")
          .trim(),
        content: emit(segs),
        notes:
          req.mode === "framework"
            ? ["Rendered by Laravel in a separate process with the sample data."]
            : [
                "Static preview: 2 Blade directive line(s) (@if, @foreach…) aren't evaluated, so every branch shows. The framework preview runs them.",
              ],
        error: null,
      };
    }
    case "email_build": {
      const before = emit(doc(req.path).segments);
      const after = emit(req.segments);
      const a = before.split("\n");
      const b = after.split("\n");
      const diff = [
        `--- ${req.path}`,
        `+++ ${req.path}`,
        "@@ @@",
        ...a.filter((l) => !b.includes(l)).map((l) => `-${l}`),
        ...b.filter((l) => !a.includes(l)).map((l) => `+${l}`),
      ].join("\n");
      return { type: "email_built", content: after, diff: before === after ? "" : `${diff}\n` };
    }
    case "email_save": {
      const f = doc(req.path);
      if (req.base_hash !== f.hash)
        throw new DaemonError(
          "app",
          `${req.path} changed since you opened it (another editor?); review it again`,
          "CONFLICT",
        );
      // The mock keeps segments; parse back what was saved by matching markers.
      const kept: Segment[] = [];
      const re = /\{\{-- locdev:block (.*?) --\}\}\n[\s\S]*?\n\{\{-- \/locdev:block --\}\}\n/g;
      let last = 0;
      let m: RegExpExecArray | null;
      let n = 0;
      while ((m = re.exec(req.content))) {
        if (m.index > last) kept.push({ kind: "code", id: `c${n++}`, code: req.content.slice(last, m.index) });
        const meta = JSON.parse(m[1]);
        kept.push({ kind: "block", id: `b${n++}`, block: meta.type, props: meta.props, edited: false });
        last = m.index + m[0].length;
      }
      if (last < req.content.length) kept.push({ kind: "code", id: `c${n++}`, code: req.content.slice(last) });
      f.segments = kept;
      f.hash = `h${Date.now()}`;
      return { type: "email_saved", hash: f.hash };
    }
    case "email_sample":
      return { type: "ok" } as unknown as EmailResponse;
    case "email_new": {
      const slug = req.item.toLowerCase().replace(/[^a-z0-9]+/g, "-");
      const path = req.component
        ? `resources/views/components/mail/${slug}.blade.php`
        : `resources/views/emails/${slug}.blade.php`;
      if (files[path] || (!req.component && slug === "welcome"))
        throw new DaemonError("app", `${path} already exists; LocDev doesn't overwrite it`, "CONFLICT");
      if (!req.component) {
        files[path] = {
          format: req.markdown ? "markdown" : "blade",
          hash: "n1",
          segments: [
            { kind: "code", id: "c0", code: head },
            { kind: "block", id: "b0", block: "heading", props: { text: req.item, level: 1 }, edited: false },
            {
              kind: "block",
              id: "b1",
              block: "text",
              props: { text: "Hello {{ $name ?? 'there' }},\n\nWrite your message here." },
              edited: false,
            },
            { kind: "code", id: "c1", code: tail },
          ],
        };
      }
      const cls = slug.replace(/(^|-)(\w)/g, (_: string, __: string, c: string) => c.toUpperCase()) + "Mail";
      return {
        type: "email_created",
        files: req.component ? [path] : [`app/Mail/${cls}.php`, path],
        ran: req.component ? null : `php artisan make:mail ${cls} --view=emails.${slug} --no-interaction`,
        notes: [],
      };
    }
    case "email_send_test":
      return { type: "ok" } as unknown as EmailResponse;
    case "email_history":
      return { type: "config_history", entries: [] } as unknown as EmailResponse;
    case "email_history_version":
      return { type: "config_history_version", content: "" } as unknown as EmailResponse;
  }
}
