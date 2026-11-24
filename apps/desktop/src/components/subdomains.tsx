import { Plus, Search, Trash2 } from "lucide-react";
import { useState } from "react";

import { Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { errorMessage, explainHost, useSetSubdomains } from "@/hooks/use-daemon";
import type { HostExplain, ProjectView, SubdomainRule, SubdomainTarget, Subdomains } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

type Kind = SubdomainTarget["type"];

function describe(target: SubdomainTarget, site: ProjectView) {
  switch (target.type) {
    case "site":
      return t("the site");
    case "port":
      return `127.0.0.1:${target.port}`;
    case "process": {
      const port = site.processes?.[target.id]?.port;
      return port
        ? t("the {id} process (:{port})", { id: target.id, port })
        : t("the {id} process (no port yet)", { id: target.id });
    }
  }
}

/** Which subdomains reach the site, rules to other ports or processes, and a host tester. */
export function SubdomainsSection({ site }: { site: ProjectView }) {
  const current: Subdomains = site.subdomains ?? { wildcard: true, rules: [] };
  const set = useSetSubdomains();
  const [pattern, setPattern] = useState("");
  const [kind, setKind] = useState<Kind>("port");
  const [value, setValue] = useState("");
  const processes = Object.entries(site.processes ?? {}).map(([id]) => id);

  const save = (next: Subdomains, done?: () => void) =>
    set.mutate({ name: site.name, subdomains: next }, { onSuccess: () => done?.() });

  const add = () => {
    const target: SubdomainTarget =
      kind === "port"
        ? { type: "port", port: Number(value) }
        : kind === "process"
          ? { type: "process", id: value }
          : { type: "site" };
    const rule: SubdomainRule = { pattern: pattern.trim(), target };
    save({ ...current, rules: [...current.rules.filter((r) => r.pattern !== rule.pattern), rule] }, () => {
      setPattern("");
      setValue("");
    });
  };

  const ready = pattern.trim() !== "" && (kind === "site" || (kind === "port" ? Number(value) > 0 : value !== ""));

  return (
    <Section title={t("subdomains")}>
      <div className="flex items-center gap-3 py-1.5 text-sm">
        <Switch
          aria-label={t("Wildcard")}
          checked={current.wildcard}
          onCheckedChange={(wildcard) => save({ ...current, wildcard })}
          className="data-checked:bg-ember"
        />
        <span>
          <span className="font-mono text-[12.5px]">*.{site.domain}</span>{" "}
          <span className="text-muted-foreground">
            {current.wildcard
              ? t("reaches the site (multi-tenant apps read the Host header)")
              : t("is off: only listed subdomains answer")}
          </span>
        </span>
      </div>
      {current.rules.length > 0 && (
        <ul className="mt-1 divide-y rounded-lg border">
          {current.rules.map((r) => (
            <li key={r.pattern} className="flex items-center gap-3 px-3 py-1.5 font-mono text-[12.5px]">
              <span>
                {r.pattern}.{site.domain}
              </span>
              <span className="text-muted-foreground">→ {describe(r.target, site)}</span>
              <Button
                size="icon-sm"
                variant="ghost"
                className="ml-auto"
                aria-label={t("Remove {pattern}", { pattern: r.pattern })}
                onClick={() => save({ ...current, rules: current.rules.filter((x) => x.pattern !== r.pattern) })}
              >
                <Trash2 />
              </Button>
            </li>
          ))}
        </ul>
      )}
      <form
        className="mt-2 flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (ready) add();
        }}
      >
        <Input
          aria-label={t("Subdomain")}
          value={pattern}
          onChange={(e) => setPattern(e.target.value)}
          placeholder={t("api, admin, *.tenants")}
          className="h-7 w-44 font-mono text-[12px]"
        />
        <span className="font-mono text-[12px] text-muted-foreground">.{site.domain} →</span>
        <Select
          items={[
            { value: "port", label: t("a port") },
            { value: "process", label: t("a process") },
            { value: "site", label: t("the site") },
          ]}
          value={kind}
          onValueChange={(v) => v && setKind(v as Kind)}
        >
          <SelectTrigger size="sm" aria-label={t("Target")} className="h-7 w-28 text-[12px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="port">{t("a port")}</SelectItem>
            <SelectItem value="process" disabled={processes.length === 0}>
              {t("a process")}
            </SelectItem>
            <SelectItem value="site">{t("the site")}</SelectItem>
          </SelectContent>
        </Select>
        {kind === "port" && (
          <Input
            aria-label={t("Rule port")}
            inputMode="numeric"
            value={value}
            onChange={(e) => setValue(e.target.value.replace(/\D/g, ""))}
            placeholder="9000"
            className="h-7 w-20 font-mono text-[12px]"
          />
        )}
        {kind === "process" && (
          <Select
            items={processes.map((p) => ({ value: p, label: p }))}
            value={value || null}
            onValueChange={(v) => setValue(String(v ?? ""))}
          >
            <SelectTrigger size="sm" aria-label={t("Process")} className="h-7 w-32 font-mono text-[12px]">
              <SelectValue placeholder={t("process")} />
            </SelectTrigger>
            <SelectContent>
              {processes.map((p) => (
                <SelectItem key={p} value={p} className="font-mono text-[12px]">
                  {p}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
        <Button type="submit" size="xs" variant="outline" disabled={!ready || set.isPending}>
          <Plus data-icon="inline-start" /> {t("Add rule")}
        </Button>
      </form>
      <p className="mt-1.5 text-[11.5px] text-muted-foreground">
        {tx(
          "Exact rules win over wildcard ones, then the longest; {star} stands for one or more labels. HTTPS certificates cover every level.",
          { star: <code>*</code> },
        )}
      </p>
      <HostTester site={site} />
    </Section>
  );
}

function Row({ ok, label, children }: { ok: boolean | null; label: string; children: React.ReactNode }) {
  return (
    <li className="flex gap-2">
      <span
        className={cn(
          "w-10 shrink-0 font-mono text-[11px]",
          ok === null ? "text-amber" : ok ? "text-phosphor" : "text-destructive",
        )}
      >
        {ok === null ? t("warn") : ok ? "ok" : t("fail")}
      </span>
      <span className="w-12 shrink-0 font-mono text-[11px] text-muted-foreground">{label}</span>
      <span className="min-w-0 text-[12.5px]">{children}</span>
    </li>
  );
}

/** DNS and routing are separate: shows both for one hostname. */
function HostTester({ site }: { site: ProjectView }) {
  const [host, setHost] = useState(`api.${site.domain}`);
  const [result, setResult] = useState<HostExplain | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      setResult(await explainHost(host));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const local = result?.dns.answers.some((a) => a === "127.0.0.1" || a === "::1") ?? false;
  return (
    <div className="mt-4">
      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void run();
        }}
      >
        <Input
          aria-label={t("Hostname to test")}
          value={host}
          onChange={(e) => setHost(e.target.value)}
          className="h-7 w-72 font-mono text-[12px]"
        />
        <Button type="submit" size="xs" variant="outline" disabled={busy || !host.trim()}>
          <Search data-icon="inline-start" /> {t("Test")}
        </Button>
      </form>
      {error && <p className="mt-1 text-[12px] text-destructive">{error}</p>}
      {result && (
        <ul className="mt-2 flex flex-col gap-1" aria-label={t("Host test")}>
          <Row ok={local} label={t("dns")}>
            {result.dns.error
              ? t("doesn't resolve: {error}", { error: result.dns.error })
              : `${result.dns.answers.join(", ")}${result.dns.via_locdev ? t(" through LocDev's DNS") : local ? t(" (another resolver answered)") : ""}`}
          </Row>
          <Row
            ok={
              result.route.problem === null && result.route.reachable !== false
                ? true
                : result.route.problem
                  ? false
                  : null
            }
            label={t("route")}
          >
            {result.route.problem ??
              `${result.route.target}${result.route.port && !result.route.target.startsWith("port ") ? t(" on :{port}", { port: result.route.port }) : ""}${result.route.rule ? t(" (rule {rule})", { rule: result.route.rule }) : ""}${result.route.reachable === false ? t(", nothing answers there now") : ""}`}
          </Row>
          <Row
            ok={
              result.tls.https_port !== null && result.tls.trusted
                ? true
                : result.tls.https_port === null
                  ? false
                  : null
            }
            label={t("https")}
          >
            {result.tls.https_port === null
              ? t("HTTPS is off")
              : `${t("certificate for {names}", { names: result.tls.names.join(", ") })}${result.tls.trusted ? "" : t(" (CA not trusted yet)")}`}
          </Row>
        </ul>
      )}
    </div>
  );
}
