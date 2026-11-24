// Shared environment variables for the mock backend (`make app-mock`, the UI
// smoke test and screenshots). Mirrors the daemon's rules closely enough to
// exercise every state the UI shows; the real logic is `locdev-env`.

import { DaemonError } from "./errors";
import {
  type ConflictSource,
  type EnvBackup,
  type EnvConflict,
  type EnvEntry,
  type EnvFileInfo,
  type EnvHistoryEntry,
  type EnvProfile,
  type EnvRequest,
  type EnvResponse,
  type EnvVar,
  GLOBAL_PROFILE,
  type ProjectEnvSettings,
  type ProjectKind,
  type SyncPlan,
  publicPrefix,
} from "./protocol";

interface Stored extends EnvVar {
  /** The real value, secret or not (the daemon keeps secrets in the vault). */
  secretValue: string;
}

const now = Date.now();

const profiles: EnvProfile[] = [
  { id: GLOBAL_PROFILE, label: "Global", description: "Every project gets these unless it opts out." },
  { id: "laravel", label: "Laravel", description: "Queue, cache and mail defaults for Laravel sites." },
  { id: "staging", label: "Staging", description: "" },
];

const stored: Stored[] = [
  v("STRIPE_SECRET_KEY", GLOBAL_PROFILE, true, "sk_test_51Mock000000000000", "Stripe test mode", 3),
  v("STRIPE_PUBLIC_KEY", GLOBAL_PROFILE, false, "pk_test_51Mock000000000000", "", 3),
  v("SENTRY_DSN", GLOBAL_PROFILE, true, "https://0000@o0.ingest.sentry.io/0", "", 30),
  v("NEXT_PUBLIC_POSTHOG_HOST", GLOBAL_PROFILE, false, "https://eu.posthog.com", "", 48),
  v("QUEUE_CONNECTION", "laravel", false, "redis", "", 72),
  v("MAIL_MAILER", "laravel", false, "smtp", "", 72),
];

function v(
  key: string,
  profile: string,
  secret: boolean,
  value: string,
  description: string,
  hoursAgo: number,
): Stored {
  return {
    key,
    profile,
    secret,
    value: secret ? null : value,
    description,
    updated_at: now - hoursAgo * 3600_000,
    secretValue: value,
  };
}

const projectSettings: Record<string, ProjectEnvSettings> = {
  "acme-shop": { inherit_global: true, profiles: ["laravel"], disabled: [] },
};

/** Files each mock project has, with the keys they define. */
const projectFiles: Record<string, Record<string, string[]>> = {
  "acme-shop": {
    ".env": ["APP_NAME", "APP_KEY", "DB_CONNECTION", "MAIL_MAILER"],
    ".env.example": ["APP_NAME", "APP_KEY", "DB_CONNECTION", "MAIL_MAILER", "STRIPE_SECRET_KEY"],
    ".env.testing": ["DB_CONNECTION"],
  },
  blog: {
    ".env.local": ["SENTRY_DSN", "DATABASE_URL"],
    ".env": ["NEXT_PUBLIC_SITE_URL"],
    ".env.production": ["NEXT_PUBLIC_SITE_URL", "STRIPE_SECRET_KEY"],
  },
  landing: {},
};

const ACTIVE: Partial<Record<ProjectKind, string[]>> = {
  laravel: [".env"],
  next: [".env.development.local", ".env.local", ".env.development", ".env"],
  vite: [".env.development.local", ".env.development", ".env.local", ".env"],
  unknown: [".env.local", ".env"],
};
const LOADER: Partial<Record<ProjectKind, string>> = { laravel: "phpdotenv", next: "@next/env", vite: "Vite" };
const NOTES: Partial<Record<ProjectKind, string[]>> = {
  laravel: [
    "Laravel reads .env through phpdotenv; a variable already in the process environment wins over the file.",
    "Changes reach a running process only after it restarts.",
  ],
  next: [
    "NEXT_PUBLIC_ variables are inlined when the code is compiled: restart the dev server after changing one, and rebuild for production.",
    "Changes reach a running process only after it restarts.",
  ],
  node: [
    "Node.js doesn't read .env files by itself (no dotenv dependency, no --env-file).",
    "Changes reach a running process only after it restarts.",
  ],
};

const history: EnvHistoryEntry[] = stored.map((s) => ({
  at: s.updated_at,
  action: "set",
  key: s.key,
  profile: s.profile,
  project: null,
  detail: "created",
}));
const backups: EnvBackup[] = [];
/** What each mock file looks like after syncs (only the appended keys). */
const appended: Record<string, Record<string, string[]>> = {};
let changedAt = 0;

function record(action: string, rest: Partial<EnvHistoryEntry> = {}) {
  history.unshift({ at: Date.now(), action, key: null, profile: null, project: null, detail: null, ...rest });
}

const view = (s: Stored): EnvVar => ({
  key: s.key,
  profile: s.profile,
  secret: s.secret,
  value: s.secret ? null : s.secretValue,
  description: s.description,
  updated_at: s.updated_at,
});

function settingsOf(name: string): ProjectEnvSettings {
  return projectSettings[name] ?? { inherit_global: true, profiles: [], disabled: [] };
}

function filesOf(name: string): Record<string, string[]> {
  const files = structuredClone(projectFiles[name] ?? {});
  for (const [file, keys] of Object.entries(appended[name] ?? {})) files[file] = [...(files[file] ?? []), ...keys];
  return files;
}

function scan(name: string, kind: ProjectKind): EnvFileInfo[] {
  const active = ACTIVE[kind] ?? [];
  const loader = LOADER[kind] ?? "none";
  return Object.entries(filesOf(name))
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([file, keys]) => {
      const rank = active.indexOf(file);
      const template = file === ".env.example";
      const mode =
        file.startsWith(".env.") && !template && file !== ".env.local" ? file.slice(5).replace(/\.local$/, "") : null;
      return {
        file,
        role:
          file === ".env"
            ? "base"
            : file === ".env.local"
              ? "local"
              : template
                ? "template"
                : file.endsWith(".local")
                  ? "mode_local"
                  : "mode",
        mode,
        active: rank >= 0,
        rank: rank >= 0 ? rank + 1 : null,
        note:
          rank >= 0
            ? `read by ${loader}`
            : template
              ? "a template; no loader reads it"
              : mode
                ? `only read in ${mode} mode, not by the dev server`
                : `not read by ${loader}`,
        keys: keys.length,
        duplicates: [],
        error: null,
      };
    });
}

function report(name: string, kind: ProjectKind, framework: string, running: boolean) {
  const settings = settingsOf(name);
  const chain = [...(settings.inherit_global ? [GLOBAL_PROFILE] : []), ...settings.profiles];
  const files = scan(name, kind);
  const fileKeys = filesOf(name);
  const entries: EnvEntry[] = [];
  const conflicts: EnvConflict[] = [];
  const keys = [...new Set(stored.filter((s) => chain.includes(s.profile)).map((s) => s.key))].sort();
  for (const key of keys) {
    const vars = chain.flatMap((p) => stored.filter((s) => s.profile === p && s.key === key));
    const winner = vars[vars.length - 1];
    for (const earlier of vars.slice(0, -1)) {
      entries.push({
        ...base(earlier),
        status: "overridden",
        reason: `profile \`${winner.profile}\` sets it too and comes later`,
        value: null,
      });
      conflicts.push({
        key,
        profile: earlier.profile,
        source: { kind: "profile", profile: winner.profile },
        injected: false,
        effect: `\`${winner.profile}\`'s value is used instead`,
      });
    }
    const entry: EnvEntry = { ...base(winner), status: "injected", reason: null };
    if (settings.disabled.includes(key)) {
      entries.push({ ...entry, status: "disabled", reason: "turned off for this project" });
      continue;
    }
    const prefix = publicPrefix(key);
    if (winner.secret && prefix) {
      entries.push({
        ...entry,
        status: "blocked",
        reason: `a secret with the ${prefix} prefix would be bundled into browser code`,
      });
      continue;
    }
    const defining = files.filter((f) => fileKeys[f.file]?.includes(key));
    for (const f of defining) {
      const source: ConflictSource = { kind: "file", file: f.file, active: f.active };
      conflicts.push({
        key,
        profile: winner.profile,
        source,
        injected: !f.active,
        effect: f.active
          ? `${f.file} defines it and ${LOADER[kind] ?? "the app"} reads that file; LocDev doesn't inject it, so the project's value is kept`
          : `${f.file} defines it but isn't read in development; the shared value is injected`,
      });
    }
    const active = defining.find((f) => f.active);
    entries.push(active ? { ...entry, status: "skipped", reason: `already in ${active.file}` } : entry);
  }
  return {
    project: name,
    framework,
    settings,
    entries,
    files,
    conflicts,
    notes: NOTES[kind] ?? ["Changes reach a running process only after it restarts."],
    restart_needed: running && changedAt > now,
  };
}

const base = (s: Stored) => ({
  key: s.key,
  profile: s.profile,
  secret: s.secret,
  value: s.secret ? null : s.secretValue,
});

function plan(name: string, kind: ProjectKind, file: string, running: boolean): SyncPlan {
  const r = report(name, kind, "", running);
  const files = filesOf(name);
  const target = files[file] ?? [];
  const out: SyncPlan = {
    project: name,
    file,
    exists: file in files,
    add: [],
    skip: [],
    preview: "",
    token: "",
    refused: null,
  };
  if (!/^\.env(\.[a-z0-9.-]+)?$/.test(file)) {
    return { ...out, refused: `${file} is not a .env file (.env, .env.local, .env.<mode>…)` };
  }
  const template = file === ".env.example";
  const deployed = /^\.env\.(production|prod|staging|stage)(\.local)?$/.test(file);
  const lines = ["# Shared variables added by LocDev (locdev env sync)"];
  for (const e of r.entries.filter((e) => e.status === "injected" || e.status === "skipped")) {
    if (target.includes(e.key)) {
      out.skip.push({ key: e.key, reason: `already in ${file}` });
      continue;
    }
    const other = r.files.find((f) => f.active && f.file !== file && files[f.file]?.includes(e.key));
    if (other) {
      out.skip.push({ key: e.key, reason: `defined in ${other.file}, which the dev server reads` });
      continue;
    }
    if (e.secret && deployed) {
      out.skip.push({ key: e.key, reason: `secret; development secrets aren't copied into ${file}` });
      continue;
    }
    const value = template ? "" : e.secret ? "********" : (e.value ?? "");
    lines.push(`${e.key}=${value}`);
    out.add.push({ key: e.key, profile: e.profile, secret: e.secret, placeholder: template });
  }
  for (const e of r.entries.filter((e) => e.status === "disabled" || e.status === "blocked")) {
    out.skip.push({ key: e.key, reason: e.reason ?? "not used by this project" });
  }
  if (out.add.length) out.preview = lines.map((l) => `+ ${l}\n`).join("");
  out.token = `${name}:${file}:${target.length}:${out.add.map((a) => a.key).join(",")}`;
  return out;
}

export interface MockSite {
  name: string;
  kind: ProjectKind;
  framework: string;
  running: boolean;
}

export function handleEnv(req: EnvRequest, sites: () => MockSite[]): EnvResponse | { type: "ok" } {
  const site = (name: string) => {
    const s = sites().find((s) => s.name === name);
    if (!s) throw new DaemonError("app", `no project named ${name}`, "NOT_FOUND");
    return s;
  };
  const invalid = (message: string) => new DaemonError("app", message, "INVALID_INPUT");
  switch (req.type) {
    case "env_overview":
      return {
        type: "env_overview",
        profiles: structuredClone(profiles),
        vars: stored.map(view),
        vault: {
          available: true,
          backend: "system keyring",
          error: null,
          secrets: stored.filter((s) => s.secret).length,
        },
        history: history.slice(0, 100),
      };
    case "env_set": {
      const key = req.key.trim();
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key))
        throw invalid(`\`${key}\` is not a valid variable name (letters, digits and _, not starting with a digit)`);
      if (["PATH", "HOME", "PORT", "HOST"].includes(key) || /^(LD_|DYLD_|LOCDEV)/.test(key)) {
        throw invalid(`${key} is set by LocDev or the system and can't be shared`);
      }
      if (!profiles.some((p) => p.id === req.profile)) throw invalid(`no profile \`${req.profile}\``);
      const existing = stored.find((s) => s.key === key && s.profile === req.profile);
      const secret = req.secret ?? existing?.secret ?? publicPrefix(key) === null;
      const prefix = publicPrefix(key);
      if (secret && prefix) {
        throw invalid(
          `${key} can't be secret: frameworks bundle ${prefix} variables into browser code. Rename it or store it as a plain value`,
        );
      }
      if (!existing && req.value == null) throw invalid(`${key} needs a value`);
      const next: Stored = {
        key,
        profile: req.profile,
        secret,
        value: null,
        description: req.description ?? existing?.description ?? "",
        updated_at: Date.now(),
        secretValue: req.value ?? existing!.secretValue,
      };
      if (existing) Object.assign(existing, next);
      else stored.push(next);
      changedAt = Date.now();
      record("set", { key, profile: req.profile, detail: existing ? null : "created" });
      return { type: "env_var", var: view(next) };
    }
    case "env_remove": {
      const i = stored.findIndex((s) => s.key === req.key && s.profile === req.profile);
      if (i < 0) throw new DaemonError("app", `no variable ${req.key} in profile \`${req.profile}\``, "INVALID_INPUT");
      stored.splice(i, 1);
      changedAt = Date.now();
      record("removed", { key: req.key, profile: req.profile });
      return { type: "ok" };
    }
    case "env_reveal": {
      const s = stored.find((s) => s.key === req.key && s.profile === req.profile);
      if (!s) throw invalid(`no variable ${req.key} in profile \`${req.profile}\``);
      if (s.secret) record("revealed", { key: s.key, profile: s.profile });
      return { type: "env_value", value: s.secretValue };
    }
    case "env_save_profile": {
      if (!/^[a-z0-9][a-z0-9-]{0,39}$/.test(req.id))
        throw invalid(`\`${req.id}\` is not a valid profile id (lowercase letters, digits and -)`);
      const profile = { id: req.id, label: req.label.trim() || req.id, description: req.description.trim() };
      const i = profiles.findIndex((p) => p.id === req.id);
      if (i >= 0) profiles[i] = profile;
      else profiles.push(profile);
      record("profile_saved", { profile: req.id });
      return { type: "env_profile", profile };
    }
    case "env_remove_profile": {
      if (req.id === GLOBAL_PROFILE) throw invalid("the global profile can't be removed");
      const count = stored.filter((s) => s.profile === req.id).length;
      if (count) throw invalid(`profile \`${req.id}\` still has ${count} variable(s); remove them first`);
      profiles.splice(
        profiles.findIndex((p) => p.id === req.id),
        1,
      );
      for (const s of Object.values(projectSettings)) s.profiles = s.profiles.filter((p) => p !== req.id);
      record("profile_removed", { profile: req.id });
      return { type: "ok" };
    }
    case "env_project": {
      const s = site(req.name);
      return { type: "env_project", ...report(s.name, s.kind, s.framework, s.running) };
    }
    case "env_set_project": {
      const s = site(req.name);
      const missing = req.settings.profiles.find((p) => !profiles.some((x) => x.id === p));
      if (missing) throw invalid(`no profile \`${missing}\``);
      projectSettings[s.name] = {
        inherit_global: req.settings.inherit_global,
        profiles: [...new Set(req.settings.profiles.filter((p) => p !== GLOBAL_PROFILE))],
        disabled: [...new Set(req.settings.disabled)].sort(),
      };
      changedAt = Date.now();
      record("project_changed", { project: s.name });
      return { type: "env_project", ...report(s.name, s.kind, s.framework, s.running) };
    }
    case "env_sync_plan": {
      const file = req.file ?? ".env";
      const list = req.name ? [site(req.name)] : sites();
      return {
        type: "env_sync_plans",
        plans: list.filter((s) => s.kind !== "static").map((s) => plan(s.name, s.kind, file, s.running)),
      };
    }
    case "env_sync": {
      const s = site(req.name);
      const file = req.file ?? ".env";
      const p = plan(s.name, s.kind, file, s.running);
      if (p.refused) throw invalid(p.refused);
      if (p.token !== req.token) {
        throw new DaemonError(
          "app",
          `${file} or the shared variables changed since the preview; review it again`,
          "CONFLICT",
        );
      }
      let backup: string | null = null;
      if (p.exists) {
        backup = `${Date.now()}-${file.slice(1).replace(/\./g, "-")}`;
        backups.unshift({ id: backup, project: s.name, file, at: Date.now(), size: 120 });
      }
      appended[s.name] ??= {};
      appended[s.name][file] = [...(appended[s.name][file] ?? []), ...p.add.map((a) => a.key)];
      record("synced", { project: s.name, detail: `${file}: ${p.add.map((a) => a.key).join(", ")}` });
      return { type: "env_synced", project: s.name, file, added: p.add.map((a) => a.key), backup };
    }
    case "env_backups":
      return { type: "env_backups", backups: backups.filter((b) => b.project === site(req.name).name) };
    case "env_restore": {
      const s = site(req.name);
      const b = backups.find((b) => b.id === req.id && b.project === s.name);
      if (!b) throw new DaemonError("app", `no backup ${req.id} for ${s.name}`, "NOT_FOUND");
      delete appended[s.name]?.[b.file];
      record("restored", { project: s.name, detail: `${b.file} from ${b.id}` });
      return { type: "ok" };
    }
  }
}
