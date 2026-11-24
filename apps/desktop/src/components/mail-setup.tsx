import { TriangleAlert } from "lucide-react";
import { useState } from "react";

import { type CodeLanguage, CodeView } from "@/components/code-editor";
import { Section } from "@/components/page";
import { Checkbox } from "@/components/ui/checkbox";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { errorMessage } from "@/hooks/use-daemon";
import { useMailActions, useMailSetup } from "@/hooks/use-mail";
import type { ProjectView } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";

/** The site's mail to LocDev's Mailbox: what changes, and consent for a real SMTP. */
export function MailSection({ site }: { site: ProjectView }) {
  const setup = useMailSetup(site.name);
  const { enable } = useMailActions();
  const { navigate } = useUi();
  const [allow, setAllow] = useState(site.mail?.override_real ?? false);

  return (
    <Section title={t("mail")}>
      {setup.isPending ? (
        <Skeleton className="h-16" />
      ) : setup.isError || !setup.data ? (
        <p className="text-[12.5px] text-muted-foreground">{errorMessage(setup.error)}</p>
      ) : (
        <div className="flex flex-col gap-2 text-[12.5px]">
          <div className="flex items-center gap-3">
            <Switch
              aria-label={t("Send mail to the Mailbox")}
              checked={setup.data.enabled}
              disabled={enable.isPending || (!setup.data.enabled && setup.data.real_smtp && !allow)}
              onCheckedChange={(enabled) => enable.mutate({ name: site.name, enabled, override_real: allow })}
              className="data-checked:bg-ember"
            />
            <span>
              {setup.data.enabled ? t("Mail this site sends goes to the ") : t("Send this site's mail to the ")}
              <button type="button" onClick={() => navigate("tools", "mailbox")} className="text-ember hover:underline">
                {t("Mailbox")}
              </button>
              {setup.data.enabled ? t(", not to real recipients.") : t(" instead of real recipients.")}
            </span>
          </div>
          {setup.data.current.length > 0 && (
            <p className="font-mono text-[11.5px] text-muted-foreground">
              {t("now:")} {setup.data.current.join("  ")}
            </p>
          )}
          {setup.data.real_smtp && (
            <label className="flex items-start gap-2 text-amber">
              <TriangleAlert className="mt-0.5 size-3.5 shrink-0" />
              <span className="flex flex-col gap-1">
                <span>
                  {t(
                    "This project is set to send through a real mail server. Turning the Mailbox on replaces that while it's on.",
                  )}
                </span>
                <span className="flex items-center gap-2 text-foreground">
                  <Checkbox
                    checked={allow}
                    onCheckedChange={(v) => setAllow(v === true)}
                    aria-label={t("Allow replacing the real mail server")}
                  />
                  {t("Allow replacing it (only in this site's processes; .env stays as it is)")}
                </span>
              </span>
            </label>
          )}
          {setup.data.wiring === "automatic" ? (
            setup.data.injects.length > 0 && (
              <p className="font-mono text-[11.5px] text-muted-foreground">
                {t("sets:")} {setup.data.injects.map((h) => `${h.name}=${h.value}`).join("  ")}
              </p>
            )
          ) : (
            <div>
              <p className="text-muted-foreground">
                {t("This stack reads SMTP settings from code. Add this for development:")}
              </p>
              <CodeView
                value={setup.data.snippet ?? ""}
                language={snippetLanguage(setup.data.snippet ?? "")}
                lineNumbers={false}
                ariaLabel={t("Mail settings snippet")}
                className="mt-1"
              />
            </div>
          )}
          {setup.data.notes.map((n) => (
            <p key={n} className="text-[11.5px] text-muted-foreground">
              {n}
            </p>
          ))}
        </div>
      )}
    </Section>
  );
}

/** The snippets are Python (Django), Ruby (Rails) or JavaScript (Nodemailer). */
function snippetLanguage(snippet: string): CodeLanguage {
  if (/^\s*EMAIL_[A-Z_]+\s*=/m.test(snippet)) return "python";
  if (/action_mailer|^\s*config\./m.test(snippet)) return "ruby";
  if (/createTransport|require\(|import /.test(snippet)) return "javascript";
  if (/^\s*[a-z_.]+\s*=/m.test(snippet)) return "dotenv"; // php.ini lines
  return "text";
}
