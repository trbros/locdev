import { Fact, Section } from "@/components/page";
import { Led } from "@/components/status";
import { useStatus } from "@/hooks/use-daemon";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function ProxySection() {
  const status = useStatus();
  const proxy = status.data?.proxy;

  return (
    <Section
      title={t("proxy")}
      aside={
        proxy && (
          <span className="flex items-center gap-2 font-mono text-[11px]">
            <Led tone={proxy.port ? (proxy.error ? "warn" : "live") : "off"} />
            {proxy.port ? `127.0.0.1:${proxy.port}` : proxy.enabled ? t("not listening") : t("disabled")}
          </span>
        )
      }
    >
      <Fact label={t("Requested port")} mono>
        {proxy ? proxy.requested_port : "—"}
      </Fact>
      <Fact label={t("Listening on")} mono>
        {proxy?.port ? `127.0.0.1:${proxy.port}` : "—"}
      </Fact>
      <Fact label={t("Site URLs")} mono>
        {proxy ? `http://<site>.${proxy.suffix}${proxy.port && proxy.port !== 80 ? `:${proxy.port}` : ""}` : "—"}
      </Fact>
      {proxy?.error && (
        <p className="mt-3 rounded-md border border-amber/30 bg-amber/[0.07] px-3 py-2 text-[13px] text-amber">
          {t("{error}. Run Doctor for the fix.", { error: proxy.error })}
        </p>
      )}
      {status.data?.dns.routing === "none" && (
        <p className="mt-3 text-[13px] text-muted-foreground">
          {tx(
            "Until {test} resolves on this machine, {localhost} works too, since browsers resolve it on their own. Set it up below.",
            {
              test: <span className="font-mono">*.test</span>,
              localhost: <span className="font-mono">.localhost</span>,
            },
          )}
        </p>
      )}
    </Section>
  );
}
