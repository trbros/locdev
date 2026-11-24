import { Link2, Link2Off, SquareTerminal } from "lucide-react";
import { Fact, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { useCli, useStatus } from "@/hooks/use-daemon";
import { copyText } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

export function CliSection() {
  const { navigate } = useUi();
  const { query, link } = useCli();
  const status = useStatus();
  const cli = query.data;
  const daemon = status.data?.version;
  // A package install (deb/rpm) already puts the bundled CLI on PATH.
  const packaged = !!cli?.bundled && cli.on_path === cli.bundled;
  const found = cli?.on_path ?? null;

  return (
    <Section title={t("command line")}>
      <p className="text-sm text-muted-foreground">
        {tx("The {locdev} CLI talks to the same daemon; anything here works from a terminal too.", {
          locdev: <span className="font-mono text-foreground">locdev</span>,
        })}{" "}
        {tx("Run {cmd} alone to open its terminal UI.", {
          cmd: <span className="font-mono text-foreground">locdev</span>,
        })}
      </p>

      <div className="mt-3">
        <Fact label={t("CLI version")} mono>
          {cli?.version ?? (query.isPending ? "…" : t("not bundled"))}
          {cli?.version && daemon && cli.version !== daemon && t(" (daemon: {version})", { version: daemon })}
        </Fact>
        <Fact label={t("Bundled at")} mono>
          <span onDoubleClick={() => cli?.bundled && copyText(cli.bundled)}>{cli?.bundled ?? "—"}</span>
        </Fact>
        <Fact label={t("In new terminals")} mono>
          {found ?? t("not on PATH")}
        </Fact>
      </div>

      {cli?.bundled && !packaged && (
        <div className="mt-3 flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            {cli.link ? (
              <Button variant="outline" size="sm" disabled={link.isPending} onClick={() => link.mutate(false)}>
                <Link2Off data-icon="inline-start" /> {t("Remove the locdev command")}
              </Button>
            ) : (
              <Button
                size="sm"
                disabled={link.isPending || cli.conflict}
                onClick={() => link.mutate(true)}
                className="bg-ember text-ember-foreground hover:bg-ember/85"
              >
                <Link2 data-icon="inline-start" /> {t("Add locdev to the terminal")}
              </Button>
            )}
            {cli.link_path && (
              <span className="font-mono text-xs text-muted-foreground" data-selectable>
                {cli.link_path}
              </span>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            {cli.conflict
              ? t("Another locdev already exists there and isn't LocDev's; it is left as it is.")
              : t("Per user, no administrator rights: a small script that runs the CLI inside the app.")}
            {cli.link &&
              !cli.link_dir_on_path &&
              " " + t("Its folder isn't on your PATH yet: add it in your shell's profile.")}
          </p>
        </div>
      )}

      <div className="mt-4 flex flex-wrap items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => navigate("tools", "cli")}>
          <SquareTerminal data-icon="inline-start" /> {t("Command reference")}
        </Button>
        <a
          href="https://github.com/trbros/locdev/releases/latest"
          target="_blank"
          rel="noopener"
          className="text-xs text-ember hover:underline"
        >
          {t("Standalone CLI downloads")}
        </a>
      </div>
    </Section>
  );
}
