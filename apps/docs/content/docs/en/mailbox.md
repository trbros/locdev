# Mailbox

LocDev has its own test inbox: an SMTP server that **only captures**. It listens on `127.0.0.1:1025` (or the next free port, if Mailpit is already there) and keeps everything it receives, delivering nothing to anyone. There's no extra tool to install.

![Tools → Mailbox: per-site inboxes, the isolated HTML and the debug tab.](screenshot:mailbox)

```bash
locdev mail enable shop          # shop's email goes to the Mailbox
locdev mail list                 # messages, newest first
locdev mail show <id>            # envelope, text, links, attachments and problems
locdev mail raw <id> -o msg.eml  # the message exactly as it arrived
locdev mail test                 # send a test message to the Mailbox
```

In the app: **Tools → Mailbox**. To turn a site on, use **Site → Settings → mail**.

## How each framework is wired

LocDev doesn't intercept traffic. It sets the SMTP transport explicitly, **only in the environment of the site's processes** (server, workers, scheduler). The project's `.env` is never edited. After turning it on or off, restart the site.

| Stack                             | How                                                                                                                                                                                                                                                                                                                                                                                      |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Laravel                           | `MAIL_MAILER=smtp`, `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME` (the site's name), `MAIL_PASSWORD`, plus `MAIL_SCHEME` (Laravel 11+) or `MAIL_ENCRYPTION=null` (10 and earlier), depending on what the project's `config/mail.php` reads. An existing `MAIL_URL` is cleared, because it wins over `MAIL_HOST`.                                                                             |
| WordPress and PHP                 | PHP's `mail()` goes through `sendmail_path`, which comes from a LocDev `.ini` added with `PHP_INI_SCAN_DIR` (the system's `.ini` files still apply). On Windows, the `.ini` uses `SMTP`/`smtp_port`. SMTP plugins (WP Mail SMTP and the like) don't go through `mail()` and need to point to `127.0.0.1` as well.                                                                        |
| Node.js, Django, Rails and others | If the project's `.env`/`.env.example` already has `SMTP_*`, `MAIL_*`, `EMAIL_*` or `MAILER_*` variables (`HOST`, `PORT`, `USER`, `PASS`, `SECURE`, `URL`…), LocDev sets those same variables. If the framework reads SMTP straight from code (`settings.py`, `development.rb`, `nodemailer.createTransport`), the screen shows the exact snippet to add to the development environment. |

Each site's SMTP user is its name, and that's how the Mailbox knows which project each message came from. The server accepts any password and doesn't offer STARTTLS (it's loopback).

### Real SMTP: only with your authorization

If the project points to a real server, turning the Mailbox on **needs explicit consent**: `locdev mail enable --override-real`, or the "Allow replacing it" box in the app. That applies to a host that isn't local or to an API transport such as SES, Postmark, Mailgun or Resend. Passwords from `.env` never show on screen.

## The inbox

- **Organization:** a general inbox and one per site, with search by subject, sender, recipient or snippet, and filters for unread and with attachments.
- **Views:** HTML, text, headers, links, attachments (with download) and the original `.eml`.
- **Debug:** shows what the SMTP session actually reported:
  - the envelope (`MAIL FROM`, `RCPT TO`) and `HELO`;
  - the protocol (`ESMTPA` when it authenticated) and the `AUTH` user;
  - the source address, the arrival time, the message's `Date`, the size and the site's framework.
- **Formatting problems:** objective checks.
  - Missing `From`, `Date`, `Message-ID` or `Subject`, and an invalid `Date` (RFC 5322).
  - Lines over 998 characters and 8-bit content without MIME.
  - Decoding errors (base64, quoted-printable, charset).
  - HTML without a text alternative and `cid:` without the matching part.

### Isolated HTML

The email's HTML is shown in an `iframe` with an empty `sandbox`, with no scripts, forms, navigation or pop-ups. On top of that, a security policy blocks every network load, so tracking pixels and remote images don't load. The screen says how many remote resources were blocked and the Debug tab lists them. Embedded images (`cid:`) show normally. Links don't open by clicking inside the email; use the Links tab, which opens the address in the browser.

Each captured message also becomes a notification (the _Captured emails_ category), shown by the system when the app isn't in front, and lights the green dot on the tray icon ([App tour](/docs/app)). Opening or deleting the message marks the notification read.

## Storage

Messages are kept in `data/mail/messages` (a `0700` folder), each as an `.eml` plus a `.json` with the envelope. The last 1,000 are kept and, besides that, messages over 25 MB are refused. To delete: `locdev mail delete`, `locdev mail clear [site]` or the app's buttons.

> [!WARNING]
> On Laravel, LocDev runs `artisan serve` with `--no-reload`. Without it, `serve` drops the variables LocDev injects before starting the PHP server (the PHP builds ship with `variables_order=EGPCS`). After editing `.env`, restart the site.
