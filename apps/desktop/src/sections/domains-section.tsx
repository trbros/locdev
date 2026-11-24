import { Section } from "@/components/page";
import { Led } from "@/components/status";
import { useProjects } from "@/hooks/use-daemon";
import { t } from "@/lib/i18n";

/** Every routed site and the addresses the proxy answers for. */
export function DomainsSection() {
  const projects = useProjects();
  const routed = (projects.data ?? []).filter((p) => p.url);

  return (
    <Section title={t("managed domains")}>
      {routed.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("No site has a port yet, so the proxy routes nothing.")}</p>
      ) : (
        <ul>
          {routed.map((p) => (
            <li
              key={p.name}
              className="flex items-center gap-3 border-b border-dashed py-2 font-mono text-[12.5px] last:border-b-0"
            >
              <Led tone={p.process.state === "running" ? "live" : "off"} />
              <span className="min-w-0 flex-1 truncate">{p.url?.replace(/^https?:\/\//, "")}</span>
              <span className="text-muted-foreground">→ 127.0.0.1:{p.port}</span>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}
