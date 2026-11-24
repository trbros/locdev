import { Page, Section } from "@/components/page";
import type { ViewDef } from "@/lib/nav";
import { t } from "@/lib/i18n";

const PLANS: Record<string, string[]> = {
  services: [
    "PostgreSQL, MySQL/MariaDB and Valkey on demand",
    "Per-instance ports, credentials and data folders",
    t("Mailpit catches every email your sites send"),
    t("Backups with validated restore"),
  ],
};

export function ComingSoonView({ view }: { view: ViewDef }) {
  const Icon = view.icon;

  return (
    <Page view={view}>
      <div className="dots relative flex h-44 items-center justify-center overflow-hidden rounded-xl border">
        <div className="flex size-14 items-center justify-center rounded-2xl border bg-background shadow-[0_18px_40px_-20px_rgb(0_0_0/0.6)]">
          <Icon className="size-6 text-ember" strokeWidth={1.75} />
        </div>
        <span className="absolute top-3 right-3 rounded-md border bg-background px-2 py-0.5 font-mono text-[11px] text-muted-foreground">
          {t("planned · {soon}", { soon: view.soon })}
        </span>
      </div>

      <Section title={t("what's coming")}>
        <ul className="flex flex-col">
          {(PLANS[view.id] ?? []).map((item, i) => (
            <li key={item} className="flex gap-4 border-b border-dashed py-2.5 text-sm last:border-b-0">
              <span className="w-6 shrink-0 font-mono text-[11px] text-ember">{String(i + 1).padStart(2, "0")}</span>
              {item}
            </li>
          ))}
        </ul>
      </Section>
    </Page>
  );
}
