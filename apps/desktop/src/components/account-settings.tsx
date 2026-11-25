import { Copy, ExternalLink, KeyRound, LogOut, Terminal } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Fact, Section } from "@/components/page";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { SwitchField } from "@/components/ui/switch-field";
import { useGithubAccount, useGithubConnect, useSecurity, useSettings } from "@/hooks/use-daemon";
import type { DeviceCode } from "@/lib/protocol";
import { copyText, openInBrowser } from "@/lib/system";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

// ── Security ───────────────────────────────────────────────────────────────

export function SecuritySection() {
  const { query, setPassword, setOptions } = useSecurity();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [again, setAgain] = useState("");
  const status = query.data;
  if (!status) return <Skeleton className="h-48" />;

  const reset = () =>
    [setCurrent, setNext, setAgain].forEach((f) => {
      f("");
    });
  const save = (remove: boolean) =>
    setPassword.mutate(
      { current: status.password_set ? current : null, new: remove ? null : next },
      {
        onSuccess: () => {
          toast.success(
            remove ? t("Password removed") : status.password_set ? t("Password changed") : t("Password set"),
          );
          reset();
        },
        onError: (error) => toast.error(t("Not saved"), { description: message(error) }),
      },
    );
  const option = (patch: { require_password?: boolean; backups?: boolean }) =>
    setOptions.mutate(
      { password: current || null, require_password: status.require_password, backups: status.backups, ...patch },
      { onError: (error) => toast.error(t("Not saved"), { description: message(error) }) },
    );
  const mismatch = again.length > 0 && again !== next;
  const locked = status.locked_until && status.locked_until > Date.now();

  return (
    <>
      <p className="rounded-md border px-3 py-2.5 text-[13px] text-muted-foreground">
        {tx(
          "The password guards destructive operations {started_in_locdev} (the app and the {locdev} command): discarding changes, resetting a branch, force-pushing, deleting an unmerged branch or a stash. It doesn't protect the repository itself: Git in a terminal, your editor or any other tool works as before. Only an Argon2id hash is stored, readable by your user only.",
          {
            started_in_locdev: <strong className="text-foreground">{t("started in LocDev")}</strong>,
            locdev: <span className="font-mono">locdev</span>,
          },
        )}
      </p>

      {locked && (
        <p className="rounded-md border border-destructive/30 bg-destructive/[0.07] px-3 py-2 text-[13px] text-destructive">
          {t("Too many wrong passwords. Try again in")} {Math.ceil((status.locked_until! - Date.now()) / 1000)}s.
        </p>
      )}

      <Section title={t("password")}>
        <Fact label={t("State")}>{status.password_set ? t("set") : t("not set")}</Fact>
        <form
          className="mt-3 grid max-w-md gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (next.length >= 8 && !mismatch) save(false);
          }}
        >
          {status.password_set && (
            <Input
              type="password"
              value={current}
              onChange={(e) => setCurrent(e.target.value)}
              placeholder={t("Current password")}
              aria-label={t("Current password")}
              autoComplete="current-password"
            />
          )}
          <Input
            type="password"
            value={next}
            onChange={(e) => setNext(e.target.value)}
            placeholder={t("New password (8+ characters)")}
            aria-label={t("New password")}
            autoComplete="new-password"
          />
          <Input
            type="password"
            value={again}
            onChange={(e) => setAgain(e.target.value)}
            placeholder={t("New password again")}
            aria-label={t("Repeat new password")}
            autoComplete="new-password"
            aria-invalid={mismatch}
          />
          <div className="flex gap-2 pt-1">
            <Button
              type="submit"
              size="sm"
              disabled={next.length < 8 || mismatch || again !== next || setPassword.isPending}
            >
              <KeyRound data-icon="inline-start" /> {status.password_set ? t("Change password") : t("Set password")}
            </Button>
            {status.password_set && (
              <Button
                type="button"
                size="sm"
                variant="destructive"
                disabled={!current || setPassword.isPending}
                onClick={() => save(true)}
              >
                {t("Remove password")}
              </Button>
            )}
          </div>
          <p className="text-[12px] text-muted-foreground">
            {tx(
              "There's no recovery: if you forget it, delete {security_toml} in the LocDev config folder (that removes the protection, which is the point: it guards against slips, not against you).",
              { security_toml: <span className="font-mono">security.toml</span> },
            )}
          </p>
        </form>
      </Section>

      <Section title={t("options")}>
        {status.password_set && (
          <p className="pb-2 text-[12.5px] text-muted-foreground">
            {t(
              "Changing these needs the current password (type it above), so the protection can't be switched off around it.",
            )}
          </p>
        )}
        <SwitchField
          className="flex items-start gap-3 py-1.5"
          checked={status.require_password}
          disabled={!status.password_set}
          onCheckedChange={(v) => option({ require_password: v })}
          switchClassName="mt-0.5 data-checked:bg-ember"
        >
          <span>
            <span className="block text-sm">{t("Ask for the password")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {t("On every destructive operation, after typing the confirmation phrase.")}
            </span>
          </span>
        </SwitchField>
        <SwitchField
          className="flex items-start gap-3 py-1.5"
          checked={status.backups}
          onCheckedChange={(v) => option({ backups: v })}
          switchClassName="mt-0.5 data-checked:bg-ember"
        >
          <span>
            <span className="block text-sm">{t("Back up first")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {tx(
                "Before a destructive operation, save uncommitted changes as a snapshot and keep a ref to the commits it moves away from (under {refs_locdev_backup}), and copy untracked files into the LocDev data folder.",
                { refs_locdev_backup: <span className="font-mono">refs/locdev/backups/</span> },
              )}
            </span>
          </span>
        </SwitchField>
      </Section>
    </>
  );
}

// ── Integrations ───────────────────────────────────────────────────────────

export function IntegrationsSection() {
  const account = useGithubAccount();
  const connect = useGithubConnect();
  const { query, update } = useSettings();
  const [token, setToken] = useState("");
  const [device, setDevice] = useState<DeviceCode | null>(null);
  const settings = query.data;

  // Poll at the interval GitHub asked for until the user approves (or the code expires).
  useEffect(() => {
    if (!device) return;
    const expires = Date.now() + device.expires_in * 1000;
    const timer = setInterval(() => {
      if (Date.now() > expires) {
        setDevice(null);
        toast.error(t("The code expired; start again"));
        return;
      }
      connect.devicePoll.mutate(undefined, {
        onSuccess: (account) => {
          if (account) {
            setDevice(null);
            toast.success(t("Connected as {login}", { login: account.login }));
          }
        },
        onError: (error) => {
          setDevice(null);
          toast.error(t("Not connected"), { description: message(error) });
        },
      });
    }, device.interval * 1000);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device, connect.devicePoll]);

  if (account.isPending || !settings) return <Skeleton className="h-48" />;
  const connected = (label: string) => ({
    onSuccess: () => {
      setToken("");
      toast.success(label);
    },
    onError: (error: unknown) => toast.error(t("Not connected"), { description: message(error) }),
  });
  const integrations = (patch: Partial<typeof settings.integrations>) =>
    update.mutate({ ...settings, integrations: { ...settings.integrations, ...patch } });

  return (
    <>
      <Section title={t("github")}>
        {account.data ? (
          <>
            <Fact label={t("Account")} mono>
              {account.data.login}
              {account.data.name && ` (${account.data.name})`}
            </Fact>
            <Fact label={t("Connected with")}>
              {account.data.method === "device"
                ? t("browser sign-in")
                : account.data.method === "gh_cli"
                  ? t("GitHub CLI token")
                  : t("personal access token")}
            </Fact>
            <Fact label={t("Token kept in")}>{account.data.storage}</Fact>
            {account.data.scopes.length > 0 && (
              <Fact label={t("Scopes")} mono>
                {account.data.scopes.join(", ")}
              </Fact>
            )}
            <div className="pt-2">
              <Button
                size="sm"
                variant="outline"
                disabled={connect.disconnect.isPending}
                onClick={() => connect.disconnect.mutate()}
              >
                <LogOut data-icon="inline-start" /> {t("Disconnect")}
              </Button>
            </div>
          </>
        ) : (
          <div className="flex flex-col gap-5 pt-1">
            <p className="text-[13px] text-muted-foreground">
              {t(
                "The token is checked against GitHub and kept in your system keyring (Secret Service, Keychain or Windows Credential Manager), never in a file or a log. LocDev only calls api.github.com, and anything that changes something on GitHub asks you first.",
              )}
            </p>

            <div className="flex flex-col gap-2">
              <div className="text-sm font-medium">{t("Personal access token")}</div>
              <form
                className="flex gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  connect.token.mutate(token.trim(), connected(t("Connected to GitHub")));
                }}
              >
                <Input
                  type="password"
                  value={token}
                  onChange={(e) => setToken(e.target.value)}
                  placeholder={t("github_pat_… or ghp_…")}
                  aria-label={t("GitHub token")}
                  autoComplete="off"
                  className="max-w-sm font-mono"
                />
                <Button size="sm" type="submit" disabled={!token.trim() || connect.token.isPending}>
                  {t("Connect")}
                </Button>
              </form>
              <p className="text-[12px] text-muted-foreground">
                {t(
                  "Fine-grained, limited to the repositories you want, with read access to metadata, issues, pull requests and actions (and write to actions, to re-run or start workflows).",
                )}{" "}
                <button
                  type="button"
                  className="text-ember hover:underline"
                  onClick={() => openInBrowser("https://github.com/settings/personal-access-tokens/new")}
                >
                  {t("Create one")}
                </button>
              </p>
            </div>

            <div className="flex flex-col gap-2">
              <div className="text-sm font-medium">{t("GitHub CLI")}</div>
              <div>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={connect.ghCli.isPending}
                  onClick={() => connect.ghCli.mutate(undefined, connected(t("Connected with the GitHub CLI's token")))}
                >
                  <Terminal data-icon="inline-start" /> {t("Use the token of gh")}
                </Button>
              </div>
              <p className="text-[12px] text-muted-foreground">
                {tx(
                  "Copies the token of a logged-in {gh} once ({gh_auth_token}). Logging out of gh later doesn't disconnect LocDev.",
                  {
                    gh: <span className="font-mono">gh</span>,
                    gh_auth_token: <span className="font-mono">gh auth token</span>,
                  },
                )}
              </p>
            </div>

            <div className="flex flex-col gap-2">
              <div className="text-sm font-medium">{t("Sign in with the browser")}</div>
              {device ? (
                <div className="flex flex-wrap items-center gap-3 rounded-md border px-3 py-2.5">
                  <span className="font-mono text-[18px] font-semibold tracking-[0.15em]" data-selectable>
                    {device.user_code}
                  </span>
                  <Button size="xs" variant="ghost" onClick={() => copyText(device.user_code)}>
                    <Copy data-icon="inline-start" /> {t("Copy")}
                  </Button>
                  <Button size="xs" variant="outline" onClick={() => openInBrowser(device.verification_uri)}>
                    <ExternalLink data-icon="inline-start" /> {t("Open")}{" "}
                    {device.verification_uri.replace("https://", "")}
                  </Button>
                  <span className="text-[12px] text-muted-foreground">{t("waiting for you to approve…")}</span>
                  <Button size="xs" variant="ghost" className="ml-auto" onClick={() => setDevice(null)}>
                    {t("Cancel")}
                  </Button>
                </div>
              ) : (
                <div>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={!settings.integrations.github_client_id || connect.deviceStart.isPending}
                    onClick={() =>
                      connect.deviceStart.mutate(undefined, {
                        onSuccess: setDevice,
                        onError: (error) => toast.error(t("Could not start"), { description: message(error) }),
                      })
                    }
                  >
                    {t("Get a code")}
                  </Button>
                </div>
              )}
              <p className="text-[12px] text-muted-foreground">
                {t("Needs the client id of a GitHub OAuth App with device flow enabled (LocDev doesn't ship one).")}
              </p>
              <Input
                defaultValue={settings.integrations.github_client_id}
                onBlur={(e) =>
                  e.target.value.trim() !== settings.integrations.github_client_id &&
                  integrations({ github_client_id: e.target.value.trim() })
                }
                placeholder={t("OAuth App client id")}
                aria-label={t("OAuth App client id")}
                className="max-w-sm font-mono text-[12.5px]"
              />
            </div>
          </div>
        )}
      </Section>

      <Section title={t("workflow checks")}>
        <SwitchField
          className="flex items-start gap-3 py-1.5"
          checked={settings.integrations.github_watch_runs}
          onCheckedChange={(v) => integrations({ github_watch_runs: v })}
          switchClassName="mt-0.5 data-checked:bg-ember"
        >
          <span>
            <span className="block text-sm">{t("Notify about failed workflow runs")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {t(
                "Every 15 minutes, for sites with a GitHub remote, while connected and not offline. The first check only records what's there, so you're not flooded with old failures.",
              )}
            </span>
          </span>
        </SwitchField>
      </Section>
    </>
  );
}
