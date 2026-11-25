import { Eye, EyeOff, KeyRound, Lock, Pencil, Plus, RefreshCw, RotateCcw, Search, Trash2, Upload } from "lucide-react";
import { type SyntheticEvent, useMemo, useState } from "react";
import { toast } from "sonner";

import { CodeView } from "@/components/code-editor";
import { Section } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { SwitchField } from "@/components/ui/switch-field";
import { Textarea } from "@/components/ui/textarea";
import { errorMessage, useProjectAction, useSettings } from "@/hooks/use-daemon";
import {
  revealValue,
  useEnvBackups,
  useEnvMutations,
  useEnvOverview,
  useEnvProject,
  useEnvRestore,
  useEnvSync,
  useSyncPlans,
} from "@/hooks/use-env";
import { DaemonError } from "@/lib/errors";
import { uptime } from "@/lib/format";
import {
  type EntryStatus,
  type EnvConflict,
  type EnvConflictPolicy,
  type EnvProfile,
  type EnvVar,
  GLOBAL_PROFILE,
  type ProjectView,
  publicPrefix,
} from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const MASK = "••••••••";

const STATUS: Record<EntryStatus, { tone: Tone; label: string; title: string }> = {
  injected: { tone: "live", label: "injected", title: t("The site's processes get it") },
  skipped: { tone: "warn", label: "skipped", title: t("The site (or the system) already defines it") },
  overridden: { tone: "off", label: "overridden", title: t("A later profile sets the same key") },
  disabled: { tone: "off", label: "off", title: t("Turned off for this site") },
  blocked: { tone: "fail", label: "blocked", title: t("Refused for safety") },
};

const POLICIES: { id: EnvConflictPolicy; label: string; detail: string }[] = [
  {
    id: "active_files",
    label: t("Files the dev server reads"),
    detail: t("Skip a shared variable when .env, .env.local… (whatever the framework loads) already has it."),
  },
  {
    id: "any_file",
    label: t("Any .env file"),
    detail: t("Also skip it when only .env.example or .env.production mention it."),
  },
];

/** Reveals a secret, asking for the LocDev password when the daemon wants it. */
function useReveal() {
  const [asking, setAsking] = useState<{ key: string; profile: string } | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const id = (key: string, profile: string) => `${profile}/${key}`;

  const reveal = async (key: string, profile: string, password?: string) => {
    try {
      const value = await revealValue(key, profile, password);
      setValues((v) => ({ ...v, [id(key, profile)]: value }));
      setAsking(null);
    } catch (error) {
      if (error instanceof DaemonError && error.code === "UNAUTHORIZED" && !password) {
        setAsking({ key, profile });
      } else {
        toast.error(t("Could not reveal {key}", { key }), { description: errorMessage(error) });
      }
    }
  };
  const hide = (key: string, profile: string) =>
    setValues((v) => {
      const next = { ...v };
      delete next[id(key, profile)];
      return next;
    });
  const shown = (key: string, profile: string) => values[id(key, profile)];

  const prompt = (
    <PasswordDialog
      open={asking !== null}
      what={asking?.key ?? ""}
      onCancel={() => setAsking(null)}
      onSubmit={(password) => asking && reveal(asking.key, asking.profile, password)}
    />
  );
  return { reveal, hide, shown, prompt };
}

function PasswordDialog({
  open,
  what,
  onCancel,
  onSubmit,
}: {
  open: boolean;
  what: string;
  onCancel: () => void;
  onSubmit: (password: string) => void;
}) {
  const [password, setPassword] = useState("");
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onCancel()}>
      <DialogContent className="sm:max-w-sm">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            onSubmit(password);
            setPassword("");
          }}
          className="flex flex-col gap-4"
        >
          <DialogHeader>
            <DialogTitle>{t("LocDev password")}</DialogTitle>
            <DialogDescription>
              {tx(
                "Showing {what} needs your LocDev password. It guards what goes through LocDev, not the vault file itself.",
                { what: <span className="font-mono">{what}</span> },
              )}
            </DialogDescription>
          </DialogHeader>
          <Input type="password" autoFocus value={password} onChange={(e) => setPassword(e.target.value)} />
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onCancel}>
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={!password}>
              {t("Show")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ── Settings → Environment ───────────────────────────────────────────────────

export function EnvironmentSettings() {
  const overview = useEnvOverview();
  const { remove, removeProfile } = useEnvMutations();
  const settings = useSettings();
  const reveal = useReveal();
  const [query, setQuery] = useState("");
  const [profile, setProfile] = useState<string | null>(null);
  const [editing, setEditing] = useState<EnvVar | "new" | null>(null);
  const [newProfile, setNewProfile] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [removing, setRemoving] = useState<EnvVar | null>(null);

  const data = overview.data;
  const needle = query.trim().toLowerCase();
  const vars = useMemo(
    () =>
      (data?.vars ?? []).filter(
        (v) =>
          (profile === null || v.profile === profile) &&
          (!needle || v.key.toLowerCase().includes(needle) || v.description.toLowerCase().includes(needle)),
      ),
    [data, profile, needle],
  );

  if (overview.isPending) return <Skeleton className="h-64" />;
  if (overview.isError || !data) {
    return <p className="text-sm text-destructive">{errorMessage(overview.error)}</p>;
  }
  const label = (id: string) => data.profiles.find((p) => p.id === id)?.label ?? id;
  const policy = settings.query.data?.env.conflicts;

  return (
    <>
      <div
        className={cn(
          "flex items-start gap-3 rounded-lg border px-3 py-2.5 text-[13px]",
          data.vault.available ? "text-muted-foreground" : "border-destructive/40 bg-destructive/6 text-destructive",
        )}
      >
        <Lock className="mt-0.5 size-4 shrink-0" />
        <span>
          {data.vault.available
            ? tx(
                "Secret values are encrypted (ChaCha20-Poly1305) with a key kept in the {backend}. Sites get them only in their process environment; nothing is written to your project files unless you sync.",
                { backend: <span className="text-foreground">{data.vault.backend}</span> },
              )
            : t("The vault can't be opened: {error}. Plain variables still work; secrets are not injected.", {
                error: data.vault.error,
              })}
        </span>
      </div>

      <Section
        title={t("variables · {length}", { length: data.vars.length })}
        aside={
          <div className="flex gap-1.5">
            <Button size="xs" variant="outline" onClick={() => setSyncing(true)} disabled={data.vars.length === 0}>
              <Upload data-icon="inline-start" /> {t("Sync to .env…")}
            </Button>
            <Button
              size="xs"
              onClick={() => setEditing("new")}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
            >
              <Plus data-icon="inline-start" /> {t("Add variable")}
            </Button>
          </div>
        }
      >
        <div className="flex flex-wrap items-center gap-2 pb-2">
          <div className="relative min-w-48 flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("Search keys and descriptions")}
              className="h-8 pl-8 text-[13px]"
              aria-label={t("Search variables")}
            />
          </div>
          <div
            role="radiogroup"
            aria-label={t("Profile")}
            className="flex flex-wrap rounded-md border p-0.5 text-[12.5px]"
          >
            {[null, ...data.profiles.map((p) => p.id)].map((id) => (
              <button
                type="button"
                role="radio"
                key={id ?? "all"}
                aria-checked={profile === id}
                onClick={() => setProfile(id)}
                className={cn(
                  "rounded px-2.5 py-1 transition-colors",
                  profile === id
                    ? "bg-foreground/[0.07] text-foreground"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {id === null ? t("All") : label(id)}
              </button>
            ))}
          </div>
        </div>

        {vars.length === 0 ? (
          <p className="py-6 text-center text-sm text-muted-foreground">
            {data.vars.length === 0
              ? t("No shared variables yet. Add API keys or DSNs once and every site can use them.")
              : t("Nothing matches.")}
          </p>
        ) : (
          <div className="divide-y rounded-lg border">
            {vars.map((v) => {
              const shown = reveal.shown(v.key, v.profile);
              return (
                <div key={`${v.profile}/${v.key}`} className="group flex items-center gap-3 px-3 py-2">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate font-mono text-[13px] font-medium" data-selectable>
                        {v.key}
                      </span>
                      {v.secret && (
                        <span className="inline-flex items-center gap-1 rounded border border-amber/30 px-1 font-mono text-[10.5px] text-amber">
                          <KeyRound className="size-3" /> {t("secret")}
                        </span>
                      )}
                      {publicPrefix(v.key) && (
                        <span
                          className="rounded border px-1 font-mono text-[10.5px] text-signal"
                          title={t("Frameworks bundle this into browser code")}
                        >
                          {t("public")}
                        </span>
                      )}
                      <span className="font-mono text-[11px] text-muted-foreground">{label(v.profile)}</span>
                    </div>
                    <div
                      className="truncate font-mono text-[12px] text-muted-foreground"
                      data-selectable={!v.secret || !!shown}
                    >
                      {v.secret ? (shown ?? MASK) : v.value}
                      {v.description && <span className="ml-3 font-sans">{v.description}</span>}
                    </div>
                  </div>
                  <span className="hidden font-mono text-[11px] text-muted-foreground sm:block">
                    {uptime(v.updated_at)} {t("ago")}
                  </span>
                  <div className="flex gap-0.5 opacity-70 group-hover:opacity-100">
                    {v.secret && (
                      <Button
                        size="icon-sm"
                        variant="ghost"
                        aria-label={shown ? t("Hide {key}", { key: v.key }) : t("Show {key}", { key: v.key })}
                        onClick={() => (shown ? reveal.hide(v.key, v.profile) : reveal.reveal(v.key, v.profile))}
                      >
                        {shown ? <EyeOff /> : <Eye />}
                      </Button>
                    )}
                    <Button
                      size="icon-sm"
                      variant="ghost"
                      aria-label={t("Edit {key}", { key: v.key })}
                      onClick={() => setEditing(v)}
                    >
                      <Pencil />
                    </Button>
                    <Button
                      size="icon-sm"
                      variant="ghost"
                      aria-label={t("Remove {key}", { key: v.key })}
                      onClick={() => setRemoving(v)}
                    >
                      <Trash2 />
                    </Button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </Section>

      <Section
        title={t("profiles")}
        aside={
          <Button size="xs" variant="outline" onClick={() => setNewProfile(true)}>
            <Plus data-icon="inline-start" /> {t("New profile")}
          </Button>
        }
      >
        <p className="pb-2 text-[13px] text-muted-foreground">
          {tx(
            "Every site takes {global} unless it opts out. Other profiles apply to the sites that enable them (in the site's Environment tab); a later profile wins over an earlier one.",
            { global: <span className="font-mono">global</span> },
          )}
        </p>
        <div className="flex flex-col">
          {data.profiles.map((p) => {
            const count = data.vars.filter((v) => v.profile === p.id).length;
            return (
              <div key={p.id} className="flex items-center gap-3 py-1.5 text-sm">
                <span className="w-28 font-mono text-[12.5px]">{p.id}</span>
                <span>{p.label}</span>
                <span className="truncate text-[12.5px] text-muted-foreground">{p.description}</span>
                <span className="ml-auto font-mono text-[11.5px] text-muted-foreground">
                  {t("{count} var(s)", { count })}
                </span>
                {p.id !== GLOBAL_PROFILE && (
                  <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label={t("Remove profile {id}", { id: p.id })}
                    disabled={count > 0 || removeProfile.isPending}
                    title={count > 0 ? t("Remove its variables first") : undefined}
                    onClick={() => removeProfile.mutate(p.id)}
                  >
                    <Trash2 />
                  </Button>
                )}
              </div>
            );
          })}
        </div>
      </Section>

      <Section title={t("conflicts")}>
        <div
          role="radiogroup"
          aria-label={t("When a site already has the key")}
          className="grid grid-cols-2 gap-2 pt-1"
        >
          {POLICIES.map((p) => (
            <button
              type="button"
              role="radio"
              key={p.id}
              aria-checked={policy === p.id}
              disabled={!settings.query.data}
              onClick={() =>
                settings.query.data && settings.update.mutate({ ...settings.query.data, env: { conflicts: p.id } })
              }
              className={cn(
                "rounded-lg border px-3 py-2 text-left transition-colors hover:bg-foreground/3",
                policy === p.id && "border-ember/60 bg-ember/6",
              )}
            >
              <span className="block text-sm">{p.label}</span>
              <span className="block text-[12px] text-muted-foreground">{p.detail}</span>
            </button>
          ))}
        </div>
        <p className="mt-2 text-[12.5px] text-muted-foreground">
          {t(
            "A variable already in the process environment wins over .env files in Laravel, Next.js, Vite, Nuxt and dotenv, so LocDev never injects a key the site defines itself: the site's own value is kept, and the conflict is shown here and in the site's log.",
          )}
        </p>
      </Section>

      <Section title={t("history")}>
        {data.history.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("No changes yet.")}</p>
        ) : (
          <div className="flex max-h-64 flex-col overflow-y-auto font-mono text-[12px]">
            {data.history.slice(0, 50).map((h, i) => (
              <div key={`${h.at}-${i}`} className="flex gap-3 py-0.5">
                <span className="w-16 shrink-0 text-right text-muted-foreground">{uptime(h.at)}</span>
                <span className="w-28 shrink-0">{h.action.replace("_", " ")}</span>
                <span className="truncate text-muted-foreground">
                  {[h.key, h.profile && `(${h.profile})`, h.project && `site ${h.project}`, h.detail]
                    .filter(Boolean)
                    .join(" ")}
                </span>
              </div>
            ))}
          </div>
        )}
        <p className="mt-1 text-[12px] text-muted-foreground">{t("Values never appear in the history.")}</p>
      </Section>

      <VarDialog
        open={editing !== null}
        initial={editing === "new" ? null : editing}
        profiles={data.profiles}
        defaultProfile={profile ?? GLOBAL_PROFILE}
        onClose={() => setEditing(null)}
      />
      <ProfileDialog open={newProfile} onClose={() => setNewProfile(false)} />
      <SyncDialog name={null} open={syncing} onClose={() => setSyncing(false)} />
      <Dialog open={removing !== null} onOpenChange={(next) => !next && setRemoving(null)}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t("Remove")} {removing?.key}?
            </DialogTitle>
            <DialogDescription>
              {t("Sites stop getting it at their next restart. Files you already synced keep their copy.")}
              {removing?.secret && t(" Its encrypted value is deleted from the vault.")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRemoving(null)}>
              {t("Cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={() => {
                if (removing) remove.mutate({ key: removing.key, profile: removing.profile });
                setRemoving(null);
              }}
            >
              {t("Remove")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      {reveal.prompt}
    </>
  );
}

function VarDialog({
  open,
  initial,
  profiles,
  defaultProfile,
  onClose,
}: {
  open: boolean;
  initial: EnvVar | null;
  profiles: EnvProfile[];
  defaultProfile: string;
  onClose: () => void;
}) {
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg">
        {open && <VarForm initial={initial} profiles={profiles} defaultProfile={defaultProfile} onClose={onClose} />}
      </DialogContent>
    </Dialog>
  );
}

function VarForm({
  initial,
  profiles,
  defaultProfile,
  onClose,
}: {
  initial: EnvVar | null;
  profiles: EnvProfile[];
  defaultProfile: string;
  onClose: () => void;
}) {
  const { save } = useEnvMutations();
  const [key, setKey] = useState(initial?.key ?? "");
  const [profile, setProfile] = useState(initial?.profile ?? defaultProfile);
  // A secret's value isn't loaded: leaving it empty keeps the stored one.
  const [value, setValue] = useState(initial && !initial.secret ? (initial.value ?? "") : "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const prefix = publicPrefix(key.trim());
  const [secretChoice, setSecretChoice] = useState<boolean | null>(initial?.secret ?? null);
  const secret = prefix ? false : (secretChoice ?? true);
  const [error, setError] = useState<string | null>(null);
  const keepsValue = initial !== null && value === "" && initial.secret;

  const submit = async (e: SyntheticEvent) => {
    e.preventDefault();
    setError(null);
    try {
      await save.mutateAsync({
        key: key.trim(),
        profile,
        value: keepsValue ? null : value,
        secret,
        description,
      });
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <DialogHeader>
        <DialogTitle>{initial ? t("Edit {key}", { key: initial.key }) : t("New shared variable")}</DialogTitle>
        <DialogDescription>
          {t("Sites get it in their process environment at start. Their .env files are not touched.")}
        </DialogDescription>
      </DialogHeader>
      <div className="grid grid-cols-[1fr_auto] gap-3">
        <div className="flex flex-col gap-1.5">
          <Label htmlFor="env-key">{t("Name")}</Label>
          <Input
            id="env-key"
            value={key}
            onChange={(e) => setKey(e.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, "_"))}
            disabled={initial !== null}
            placeholder={t("STRIPE_SECRET_KEY")}
            className="font-mono"
            autoFocus={!initial}
          />
        </div>
        <div className="flex flex-col gap-1.5">
          <Label>{t("Profile")}</Label>
          <div role="radiogroup" className="flex flex-wrap gap-1">
            {profiles.map((p) => (
              <button
                type="button"
                role="radio"
                key={p.id}
                aria-checked={profile === p.id}
                disabled={initial !== null}
                onClick={() => setProfile(p.id)}
                className={cn(
                  "rounded-md border px-2 py-1.5 text-[12.5px] disabled:opacity-60",
                  profile === p.id && "border-ember/60 bg-ember/6",
                )}
              >
                {p.label}
              </button>
            ))}
          </div>
        </div>
      </div>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="env-value">{t("Value")}</Label>
        {secret ? (
          <Input
            id="env-value"
            type="password"
            autoComplete="off"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder={initial?.secret ? t("Leave empty to keep the stored value") : ""}
            className="font-mono"
            autoFocus={!!initial}
          />
        ) : (
          <Textarea
            id="env-value"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            rows={2}
            className="font-mono text-[13px]"
          />
        )}
      </div>
      <SwitchField
        className="flex items-start gap-3"
        checked={secret}
        disabled={!!prefix}
        onCheckedChange={setSecretChoice}
        switchClassName="mt-0.5 data-checked:bg-ember"
      >
        <span>
          <span className="block text-sm">{t("Secret")}</span>
          <span className="block text-[12.5px] text-muted-foreground">
            {prefix
              ? t("{prefix} variables are bundled into browser code by frameworks, so they can't be secret.", {
                  prefix,
                })
              : t(
                  "Encrypted in the vault, hidden in lists and logs, never copied into .env.production or .env.staging.",
                )}
          </span>
        </span>
      </SwitchField>
      <div className="flex flex-col gap-1.5">
        <Label htmlFor="env-description">{t("Description (optional)")}</Label>
        <Input id="env-description" value={description} onChange={(e) => setDescription(e.target.value)} />
      </div>
      {error && <p className="text-[13px] text-destructive">{error}</p>}
      <DialogFooter>
        <Button type="button" variant="ghost" onClick={onClose}>
          {t("Cancel")}
        </Button>
        <Button type="submit" disabled={!key.trim() || save.isPending}>
          {t("Save")}
        </Button>
      </DialogFooter>
    </form>
  );
}

function ProfileDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { saveProfile } = useEnvMutations();
  const [label, setLabel] = useState("");
  const [description, setDescription] = useState("");
  const [error, setError] = useState<string | null>(null);
  const id = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

  const submit = async (e: SyntheticEvent) => {
    e.preventDefault();
    try {
      await saveProfile.mutateAsync({ id, label: label.trim(), description });
      setLabel("");
      setDescription("");
      setError(null);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={submit} className="flex flex-col gap-4">
          <DialogHeader>
            <DialogTitle>{t("New profile")}</DialogTitle>
            <DialogDescription>
              {t("A group of variables sites can take: Development, Staging, Laravel, a client's account…")}
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="profile-label">{t("Name")}</Label>
            <Input id="profile-label" value={label} onChange={(e) => setLabel(e.target.value)} autoFocus />
            {id && <span className="font-mono text-[11.5px] text-muted-foreground">{t("id: {id}", { id })}</span>}
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="profile-description">{t("Description (optional)")}</Label>
            <Input id="profile-description" value={description} onChange={(e) => setDescription(e.target.value)} />
          </div>
          {error && <p className="text-[13px] text-destructive">{error}</p>}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onClose}>
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={!id || saveProfile.isPending}>
              {t("Create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ── Sync ─────────────────────────────────────────────────────────────────────

const FILES = [".env", ".env.local", ".env.development", ".env.example"];

/** Previews and applies `env sync` for one site (`name`) or every site. */
function SyncDialog({ name, open, onClose }: { name: string | null; open: boolean; onClose: () => void }) {
  const [file, setFile] = useState(".env");
  const plans = useSyncPlans(name, file, open);
  const sync = useEnvSync();
  const [running, setRunning] = useState(false);
  const todo = (plans.data ?? []).filter((p) => !p.refused && p.add.length > 0);

  const apply = async () => {
    setRunning(true);
    let added = 0;
    for (const plan of todo) {
      try {
        const result = await sync.mutateAsync({ name: plan.project, file: plan.file, token: plan.token });
        added += result.added.length;
      } catch (error) {
        toast.error(`${plan.project}: ${plan.file}`, { description: errorMessage(error) });
      }
    }
    setRunning(false);
    if (added)
      toast.success(t("Added {added} line(s)", { added }), {
        description: t("A backup of each changed file was kept."),
      });
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && !running && onClose()}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("Sync shared variables")} {name ? t("into {name}", { name }) : t("into every site")}
          </DialogTitle>
          <DialogDescription>
            {t(
              "Only appends the keys a file is missing; comments, order and formatting stay. An encrypted backup is taken first. Secrets never go into .env.production or .env.staging, and .env.example gets empty placeholders.",
            )}
          </DialogDescription>
        </DialogHeader>
        <div role="radiogroup" aria-label={t("Target file")} className="flex flex-wrap gap-1.5">
          {FILES.map((f) => (
            <button
              type="button"
              role="radio"
              key={f}
              aria-checked={file === f}
              onClick={() => setFile(f)}
              className={cn(
                "rounded-md border px-2.5 py-1 font-mono text-[12.5px]",
                file === f && "border-ember/60 bg-ember/6",
              )}
            >
              {f}
            </button>
          ))}
        </div>
        <div className="max-h-[50vh] overflow-y-auto">
          {plans.isPending ? (
            <Skeleton className="h-32" />
          ) : plans.isError ? (
            <p className="text-sm text-destructive">{errorMessage(plans.error)}</p>
          ) : (plans.data ?? []).length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("No sites to sync.")}</p>
          ) : (
            <div className="flex flex-col gap-3">
              {plans.data!.map((plan) => (
                <div key={plan.project} className="rounded-lg border px-3 py-2.5">
                  <div className="flex items-baseline gap-2 font-mono text-[13px]">
                    <span className="font-semibold">{plan.project}</span>
                    <span className="text-muted-foreground">
                      {plan.file}
                      {!plan.exists && t(" (new file)")}
                    </span>
                  </div>
                  {plan.refused ? (
                    <p className="mt-1 text-[12.5px] text-destructive">{plan.refused}</p>
                  ) : plan.add.length === 0 ? (
                    <p className="mt-1 text-[12.5px] text-muted-foreground">{t("Nothing to add.")}</p>
                  ) : (
                    <CodeView
                      value={plan.preview}
                      language="dotenv"
                      lineNumbers={false}
                      ariaLabel={t("Lines to add")}
                      className="mt-1.5"
                    />
                  )}
                  {plan.skip.length > 0 && (
                    <ul className="mt-1.5 text-[12px] text-muted-foreground">
                      {plan.skip.map((s) => (
                        <li key={s.key}>
                          <span className="font-mono">{s.key}</span>: {s.reason}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={running}>
            {t("Cancel")}
          </Button>
          <Button onClick={apply} disabled={todo.length === 0 || running}>
            {running ? t("Writing…") : t("Append to {length} file(s)", { length: todo.length })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Site → Environment ───────────────────────────────────────────────────────

export function SiteEnvTab({ site }: { site: ProjectView }) {
  const report = useEnvProject(site.name);
  const overview = useEnvOverview();
  const { setProject } = useEnvMutations();
  const action = useProjectAction();
  const reveal = useReveal();
  const [syncing, setSyncing] = useState(false);
  const [showBackups, setShowBackups] = useState(false);

  if (report.isPending) return <Skeleton className="h-64" />;
  if (report.isError || !report.data) {
    return <p className="text-sm text-destructive">{errorMessage(report.error)}</p>;
  }
  const r = report.data;
  const profiles = overview.data?.profiles ?? [];
  const change = (patch: Partial<typeof r.settings>) =>
    setProject.mutate({ name: site.name, settings: { ...r.settings, ...patch } });
  const toggleProfile = (id: string) =>
    change({
      profiles: r.settings.profiles.includes(id)
        ? r.settings.profiles.filter((p) => p !== id)
        : [...r.settings.profiles, id],
    });
  const toggleKey = (key: string) =>
    change({
      disabled: r.settings.disabled.includes(key)
        ? r.settings.disabled.filter((k) => k !== key)
        : [...r.settings.disabled, key],
    });
  const counts = r.entries.reduce<Partial<Record<EntryStatus, number>>>(
    (acc, e) => ({ ...acc, [e.status]: (acc[e.status] ?? 0) + 1 }),
    {},
  );

  return (
    <>
      {r.restart_needed && (
        <div className="flex items-center gap-3 rounded-lg border border-amber/40 bg-amber/[0.07] px-3 py-2 text-[13px] text-amber">
          <span>{t("The site is running with an older set of variables.")}</span>
          <Button
            size="xs"
            variant="outline"
            className="ml-auto"
            disabled={action.isPending}
            onClick={() => action.mutate({ action: "restart_project", name: site.name })}
          >
            <RefreshCw data-icon="inline-start" /> {t("Restart to apply")}
          </Button>
        </div>
      )}

      <Section
        title={t("profiles")}
        aside={
          <span className="font-mono text-[11px] text-muted-foreground">
            {t("{framework} rules", { framework: r.framework })}
          </span>
        }
      >
        <SwitchField
          className="flex items-center gap-3 py-1.5 text-sm"
          checked={r.settings.inherit_global}
          onCheckedChange={(inherit_global) => change({ inherit_global })}
          switchClassName="data-checked:bg-ember"
        >
          {t("Take the global variables")}
        </SwitchField>
        <div className="flex flex-wrap gap-1.5 pt-1">
          {profiles
            .filter((p) => p.id !== GLOBAL_PROFILE)
            .map((p) => {
              const on = r.settings.profiles.includes(p.id);
              const order = r.settings.profiles.indexOf(p.id);
              return (
                <button
                  type="button"
                  key={p.id}
                  aria-pressed={on}
                  onClick={() => toggleProfile(p.id)}
                  className={cn(
                    "rounded-md border px-2.5 py-1 text-[12.5px] transition-colors hover:bg-foreground/3",
                    on && "border-ember/60 bg-ember/6",
                  )}
                >
                  {on && <span className="mr-1.5 font-mono text-[11px] text-ember">{order + 1}</span>}
                  {p.label}
                </button>
              );
            })}
        </div>
        <p className="mt-2 text-[12.5px] text-muted-foreground">
          {t("Order: global first, then the profiles in the order you turned them on; a later one wins.")}
        </p>
      </Section>

      <Section
        title={`${t("variables · {n} injected", { n: counts.injected ?? 0 })}${counts.skipped ? t(" · {n} skipped", { n: counts.skipped }) : ""}`}
        aside={
          <Button size="xs" variant="outline" onClick={() => setSyncing(true)} disabled={r.entries.length === 0}>
            <Upload data-icon="inline-start" /> {t("Sync to .env…")}
          </Button>
        }
      >
        {r.entries.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            {t("No shared variables apply to this site. Add some in Settings → Environment.")}
          </p>
        ) : (
          <div className="divide-y rounded-lg border">
            {r.entries.map((e) => {
              const shown = reveal.shown(e.key, e.profile);
              const off = r.settings.disabled.includes(e.key);
              return (
                <div key={`${e.profile}/${e.key}`} className="flex items-center gap-3 px-3 py-2">
                  <span
                    className="flex w-24 shrink-0 items-center gap-2 font-mono text-[11.5px]"
                    title={STATUS[e.status].title}
                  >
                    <Led tone={STATUS[e.status].tone} /> {STATUS[e.status].label}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <span className="truncate font-mono text-[13px]" data-selectable>
                        {e.key}
                      </span>
                      <span className="font-mono text-[11px] text-muted-foreground">{e.profile}</span>
                    </div>
                    <div className="truncate text-[12px] text-muted-foreground">
                      <span className="font-mono">{e.secret ? (shown ?? MASK) : e.value}</span>
                      {e.reason && <span className="ml-3">{e.reason}</span>}
                    </div>
                  </div>
                  {e.secret && e.status !== "overridden" && (
                    <Button
                      size="icon-sm"
                      variant="ghost"
                      aria-label={shown ? t("Hide {key}", { key: e.key }) : t("Show {key}", { key: e.key })}
                      onClick={() => (shown ? reveal.hide(e.key, e.profile) : reveal.reveal(e.key, e.profile))}
                    >
                      {shown ? <EyeOff /> : <Eye />}
                    </Button>
                  )}
                  {e.status !== "overridden" && (
                    <Switch
                      checked={!off}
                      onCheckedChange={() => toggleKey(e.key)}
                      aria-label={
                        off
                          ? t("Give {key} to this site", { key: e.key })
                          : t("Turn {key} off for this site", { key: e.key })
                      }
                      className="data-checked:bg-ember"
                    />
                  )}
                </div>
              );
            })}
          </div>
        )}
      </Section>

      {r.conflicts.length > 0 && (
        <Section title={t("conflicts · {length}", { length: r.conflicts.length })}>
          <div className="flex flex-col gap-1.5">
            {r.conflicts.map((c, i) => (
              <ConflictRow key={`${c.key}-${i}`} conflict={c} />
            ))}
          </div>
        </Section>
      )}

      <Section title={t(".env files")}>
        {r.files.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("No .env files in this site's folder.")}</p>
        ) : (
          <div className="flex flex-col">
            {r.files.map((f) => (
              <div key={f.file} className="flex items-baseline gap-3 py-1 text-[13px]">
                <span className="w-44 shrink-0 font-mono">{f.file}</span>
                <span
                  className={cn(
                    "w-16 shrink-0 font-mono text-[11.5px]",
                    f.active ? "text-phosphor" : "text-muted-foreground",
                  )}
                >
                  {f.active ? t("read #{rank}", { rank: f.rank }) : t("not read")}
                </span>
                <span className="w-14 shrink-0 font-mono text-[11.5px] text-muted-foreground">
                  {t("{keys} keys", { keys: f.keys })}
                </span>
                <span className="truncate text-[12.5px] text-muted-foreground">
                  {f.error ?? f.note}
                  {f.duplicates.length > 0 && (
                    <span className="ml-2 text-amber">
                      {t("duplicated:")} {f.duplicates.join(", ")}
                    </span>
                  )}
                </span>
              </div>
            ))}
          </div>
        )}
        <ul className="mt-2 flex flex-col gap-1 text-[12.5px] text-muted-foreground">
          {r.notes.map((n) => (
            <li key={n}>· {n}</li>
          ))}
        </ul>
      </Section>

      <Section
        title={t("backups")}
        aside={
          <Button size="xs" variant="ghost" onClick={() => setShowBackups((s) => !s)}>
            {showBackups ? t("Hide") : t("Show")}
          </Button>
        }
      >
        {showBackups ? (
          <BackupList name={site.name} />
        ) : (
          <p className="text-[12.5px] text-muted-foreground">
            {t("LocDev keeps an encrypted copy of a .env file before it writes to it.")}
          </p>
        )}
      </Section>

      <SyncDialog name={site.name} open={syncing} onClose={() => setSyncing(false)} />
      {reveal.prompt}
    </>
  );
}

function ConflictRow({ conflict: c }: { conflict: EnvConflict }) {
  const source =
    c.source.kind === "file"
      ? `${c.source.file}${c.source.active ? "" : t(" (not read)")}`
      : c.source.kind === "profile"
        ? t("profile {profile}", { profile: c.source.profile })
        : c.source.kind === "system"
          ? t("system environment")
          : "set by LocDev";
  return (
    <div className="flex items-start gap-2 text-[13px]">
      <Led tone={c.injected ? "info" : "warn"} className="mt-1.5" />
      <span>
        <span className="font-mono">{c.key}</span> <span className="text-muted-foreground">{t("vs")}</span>{" "}
        <span className="font-mono">{source}</span>
        <span className="block text-[12.5px] text-muted-foreground">{c.effect}</span>
      </span>
    </div>
  );
}

function BackupList({ name }: { name: string }) {
  const backups = useEnvBackups(name, true);
  const restore = useEnvRestore();
  const [confirming, setConfirming] = useState<string | null>(null);
  if (backups.isPending) return <Skeleton className="h-16" />;
  if (!backups.data?.length) return <p className="text-sm text-muted-foreground">{t("No backups yet.")}</p>;
  return (
    <div className="flex flex-col">
      {backups.data.map((b) => (
        <div key={b.id} className="flex items-center gap-3 py-1 text-[13px]">
          <span className="w-40 font-mono">{b.file}</span>
          <span className="font-mono text-[11.5px] text-muted-foreground">
            {uptime(b.at)} {t("ago")}
          </span>
          <span className="font-mono text-[11.5px] text-muted-foreground">{b.size} B</span>
          {confirming === b.id ? (
            <span className="ml-auto flex items-center gap-1.5">
              <span className="text-[12px] text-muted-foreground">{t("Replace the current file?")}</span>
              <Button size="xs" variant="ghost" onClick={() => setConfirming(null)}>
                {t("No")}
              </Button>
              <Button
                size="xs"
                variant="destructive"
                onClick={() => {
                  restore.mutate({ name, id: b.id });
                  setConfirming(null);
                }}
              >
                {t("Restore")}
              </Button>
            </span>
          ) : (
            <Button size="xs" variant="ghost" className="ml-auto" onClick={() => setConfirming(b.id)}>
              <RotateCcw data-icon="inline-start" /> {t("Restore")}
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}
