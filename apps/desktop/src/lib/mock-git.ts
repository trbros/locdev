// Git, security and GitHub for the browser mock (`mock.ts`). Mirrors the
// daemon's rules closely enough to exercise the UI: destructive operations
// need the preview's phrase and, when set, the password.

import { DaemonError } from "./errors";
import type {
  Branch,
  Commit,
  DestructivePreview,
  GitOp,
  GitRequest,
  GitResponse,
  GitStatus,
  GithubAccount,
  GithubData,
  RefLogEntry,
  Remote,
  SecurityStatus,
  Stash,
  Worktree,
} from "./protocol";

interface Repo {
  status: GitStatus;
  commits: Commit[];
  branches: Branch[];
  stashes: Stash[];
  remotes: Remote[];
  reflog: RefLogEntry[];
  operations: string[];
  worktrees: Worktree[];
}

const now = () => Math.floor(Date.now() / 1000);
const hex = (seed: number) => (seed * 2654435761).toString(16).padStart(8, "0").slice(-7) + "c0ffee".slice(seed % 3);

function commit(i: number, subject: string, author: string, hoursAgo: number, refs: string[] = []): Commit {
  const sha = hex(i + 11).padEnd(40, "0");
  return {
    sha,
    short: sha.slice(0, 7),
    author,
    email: `${author.toLowerCase()}@acme.dev`,
    time: now() - hoursAgo * 3600,
    refs,
    subject,
    parents: [],
  };
}

function repo(slug: string | null, subjects: [string, string, number][], changes: GitStatus["changes"]): Repo {
  const commits = subjects.map(([s, a, h], i) => commit(i, s, a, h, i === 0 ? ["HEAD -> main", "origin/main"] : []));
  const remotes: Remote[] = slug
    ? [{ name: "origin", fetch_url: `git@github.com:${slug}.git`, push_url: `git@github.com:${slug}.git` }]
    : [];
  const [owner, name] = slug?.split("/") ?? [];
  return {
    status: {
      branch: "main",
      head: commits[0].sha,
      upstream: slug ? "origin/main" : null,
      ahead: 0,
      behind: slug ? 1 : 0,
      state: "clean",
      changes,
      github: slug ? { owner, name } : null,
    },
    commits,
    branches: [
      {
        name: "main",
        remote: false,
        sha: commits[0].short,
        upstream: slug ? "origin/main" : null,
        ahead: 0,
        behind: slug ? 1 : 0,
        current: true,
        time: commits[0].time,
        subject: commits[0].subject,
        merged: true,
      },
      {
        name: "feature/checkout-v2",
        remote: false,
        sha: hex(90),
        upstream: null,
        ahead: 0,
        behind: 0,
        current: false,
        time: now() - 86400 * 2,
        subject: "WIP: new checkout flow",
        merged: false,
      },
      {
        name: "fix/typo",
        remote: false,
        sha: commits[2].short,
        upstream: null,
        ahead: 0,
        behind: 0,
        current: false,
        time: commits[2].time,
        subject: commits[2].subject,
        merged: true,
      },
      ...(slug
        ? [
            {
              name: "origin/main",
              remote: true,
              sha: hex(91),
              upstream: null,
              ahead: 0,
              behind: 0,
              current: false,
              time: now() - 1800,
              subject: "Bump vite to 8.0.3",
              merged: false,
            },
          ]
        : []),
    ],
    stashes: [{ index: 0, message: "On main: half-done pricing table", time: now() - 86400 }],
    remotes,
    reflog: commits.slice(0, 4).map((c, i) => ({
      sha: c.sha,
      message: i === 0 ? `commit: ${c.subject}` : `checkout: moving from fix/typo to main`,
      time: c.time,
    })),
    operations: [],
    worktrees: [
      {
        path: `/home/dev/Projects/${name ?? "app"}`,
        branch: "main",
        head: commits[0].sha,
        main: true,
        locked: false,
        prunable: false,
        site: null,
      },
    ],
  };
}

const repos: Record<string, Repo> = {
  "acme-shop": repo(
    "acme/shop",
    [
      ["Cart: keep coupons when the session renews", "Ana", 2],
      ["Add order export to CSV", "Ana", 20],
      ["Fix typo in checkout copy", "Rui", 30],
      ["Upgrade to Laravel 12", "Rui", 72],
      ["Initial import", "Ana", 400],
    ],
    [
      {
        path: "app/Http/Controllers/CartController.php",
        orig_path: null,
        staged: "modified",
        unstaged: null,
        untracked: false,
        conflicted: false,
        added: 12,
        removed: 3,
      },
      {
        path: "resources/js/cart.ts",
        orig_path: null,
        staged: null,
        unstaged: "modified",
        untracked: false,
        conflicted: false,
        added: 4,
        removed: 1,
      },
      {
        path: "notes/todo.md",
        orig_path: null,
        staged: null,
        unstaged: null,
        untracked: true,
        conflicted: false,
        added: 9,
        removed: 0,
      },
    ],
  ),
  blog: repo(
    "acme/blog",
    [
      ["Post: shipping a local dev environment", "Rui", 5],
      ["RSS feed with full content", "Rui", 50],
      ["Fix dark mode code blocks", "Ana", 80],
    ],
    [],
  ),
  landing: repo(
    null,
    [
      ["Hero copy", "Ana", 10],
      ["Pricing section", "Ana", 30],
      ["Initial", "Ana", 90],
    ],
    [],
  ),
};

let security: SecurityStatus = { password_set: false, require_password: true, backups: true, locked_until: null };
let password: string | null = null;
let account: GithubAccount | null = null;
let devicePolls = 0;

function find(name: string): Repo {
  const r = repos[name];
  if (!r) throw new DaemonError("app", `${name}: not a Git repository`, "CONFLICT");
  return r;
}

const unauthorized = (message: string) => new DaemonError("app", message, "UNAUTHORIZED");

function checkPassword(given: string | null | undefined) {
  if (!security.password_set) return;
  if (!given) throw unauthorized("this operation needs your LocDev password");
  if (given !== password) throw unauthorized("wrong password");
}

function preview(r: Repo, op: GitOp): DestructivePreview | null {
  const make = (
    summary: string,
    lost: string[],
    phrase: string,
    backup: string | null,
    blocked: string | null = null,
  ) => ({
    summary,
    lost,
    phrase,
    password: security.password_set && security.require_password,
    backup: security.backups ? backup : null,
    blocked,
  });
  switch (op.op) {
    case "discard":
      return make(
        `Discarding throws away uncommitted changes in ${op.paths.length} file(s).`,
        op.paths.map((p) =>
          r.status.changes.find((c) => c.path === p)?.untracked
            ? `${p} (untracked file, deleted)`
            : `${p} (uncommitted changes)`,
        ),
        "discard",
        "a snapshot of tracked changes, and copies of untracked files",
      );
    case "delete_branch": {
      const b = r.branches.find((b) => !b.remote && b.name === op.name);
      if (!b) return null;
      if (b.current)
        return make("You can't delete the branch you're on.", [], op.name, null, "switch to another branch first");
      if (b.merged && !op.force) return null;
      return make(
        `${op.name} has 1 commit(s) that aren't in your current branch.`,
        [`${b.sha} ${b.subject}`],
        op.name,
        `a ref to ${op.name}'s last commit`,
      );
    }
    case "push":
      if (!op.force) return null;
      return make(
        "Force-pushing rewrites origin/main: commits there that you don't have are removed for everyone.",
        [`${hex(91)} Bump vite to 8.0.3 (on origin/main)`],
        r.status.branch ?? "",
        "a ref to origin/main's current commit",
      );
    case "stash_drop":
      return make(
        `Dropping stash@{${op.index}} deletes the changes saved in it.`,
        [r.stashes[op.index]?.message ?? "", "resources/views/pricing.blade.php"],
        "drop",
        "a ref to the stash's commit",
      );
    case "reset": {
      const at = r.commits.findIndex((c) => c.sha.startsWith(op.target) || c.short === op.target);
      const target = at >= 0 ? r.commits[at] : r.commits[1];
      const lost = r.commits
        .slice(0, Math.max(at, 1))
        .map((c) => `${c.short} ${c.subject} (off the branch; still in the reflog)`);
      if (op.mode === "hard")
        lost.push(...r.status.changes.filter((c) => !c.untracked).map((c) => `${c.path} (uncommitted changes)`));
      return make(
        `Resetting moves ${r.status.branch} to ${target.short}${op.mode === "hard" ? " and throws uncommitted changes away" : ""}.`,
        lost,
        target.short,
        "a ref to the current commit, plus a snapshot of uncommitted changes",
      );
    }
    case "restore_files":
      return make(
        `Restoring overwrites ${op.paths.length} file(s) with their version from ${op.sha.slice(0, 7)}.`,
        op.paths.filter((p) => r.status.changes.some((c) => c.path === p)).map((p) => `${p} (uncommitted changes)`),
        "restore",
        "a snapshot of uncommitted changes",
      );
    case "worktree_remove": {
      const tree = r.worktrees.find((w) => w.path === op.path);
      return make(
        "Removing a working tree deletes its folder; its branch and commits stay in the repository.",
        [`the folder ${op.path}`],
        op.path.split("/").pop() ?? "remove",
        "a snapshot of its tracked changes under refs/locdev/backups/",
        !tree
          ? `${op.path} is not a working tree of this repository`
          : tree.main
            ? "the main working tree can't be removed"
            : null,
      );
    }
    case "remote_remove":
      return make(
        `Removing ${op.name} forgets its URL and remote-tracking branches.`,
        r.branches.filter((b) => b.remote).map((b) => b.name),
        op.name,
        null,
      );
    default:
      return null;
  }
}

function run(r: Repo, op: GitOp): string {
  const s = r.status;
  switch (op.op) {
    case "stage":
      for (const c of s.changes)
        if (op.paths.includes(c.path))
          Object.assign(c, { staged: c.staged ?? c.unstaged ?? "added", unstaged: null, untracked: false });
      return `Staged ${op.paths.length} file(s)`;
    case "unstage":
      for (const c of s.changes)
        if (op.paths.includes(c.path))
          Object.assign(c, {
            unstaged: c.staged === "added" ? null : c.staged,
            untracked: c.staged === "added",
            staged: null,
          });
      return `Unstaged ${op.paths.length} file(s)`;
    case "discard":
      s.changes = s.changes.filter((c) => !op.paths.includes(c.path) || c.staged);
      for (const c of s.changes) if (op.paths.includes(c.path)) c.unstaged = null;
      return `Discarded changes in ${op.paths.length} file(s)`;
    case "commit": {
      if (!s.changes.some((c) => c.staged)) throw new DaemonError("app", "nothing staged to commit", "CONFLICT");
      const c = commit(r.commits.length + 50, op.message.split("\n")[0], "You", 0, ["HEAD -> main"]);
      r.commits[0].refs = r.commits[0].refs.filter((x) => !x.startsWith("HEAD"));
      r.commits.unshift(c);
      s.changes = s.changes.filter((x) => !x.staged).map((x) => x);
      s.ahead += 1;
      r.reflog.unshift({ sha: c.sha, message: `commit: ${c.subject}`, time: c.time });
      return `Committed ${c.short}`;
    }
    case "create_branch":
      r.branches.push({ ...r.branches[0], name: op.name, current: false, upstream: null, ahead: 0, behind: 0 });
      if (op.switch) return run(r, { op: "switch", name: op.name });
      return `Created ${op.name}`;
    case "switch":
      for (const b of r.branches) b.current = b.name === op.name;
      s.branch = op.name;
      r.reflog.unshift({ sha: s.head ?? "", message: `checkout: moving to ${op.name}`, time: now() });
      return `Switched to ${op.name}`;
    case "delete_branch":
      r.branches = r.branches.filter((b) => b.remote || b.name !== op.name);
      return `Deleted ${op.name}`;
    case "fetch":
      return "Fetched origin";
    case "pull":
      s.behind = 0;
      return "Fast-forwarded to origin/main";
    case "push":
      s.ahead = 0;
      return op.force ? "Force-pushed main (with lease)" : "Pushed main";
    case "stash_push":
      r.stashes.unshift({ index: 0, message: `On ${s.branch}: ${op.message || "WIP"}`, time: now() });
      r.stashes.forEach((st, i) => (st.index = i));
      s.changes = op.untracked ? [] : s.changes.filter((c) => c.untracked);
      return "Stashed the changes";
    case "stash_apply":
    case "stash_pop":
      if (op.op === "stash_pop") r.stashes.splice(op.index, 1);
      r.stashes.forEach((st, i) => (st.index = i));
      return op.op === "stash_pop" ? "Applied and dropped the stash" : "Applied the stash";
    case "stash_drop":
      r.stashes.splice(op.index, 1);
      r.stashes.forEach((st, i) => (st.index = i));
      return `Dropped stash@{${op.index}}`;
    case "remote_add":
      r.remotes.push({ name: op.name, fetch_url: op.url, push_url: op.url });
      return `Added ${op.name}`;
    case "remote_remove":
      r.remotes = r.remotes.filter((x) => x.name !== op.name);
      return `Removed ${op.name}`;
    case "remote_set_url":
      for (const x of r.remotes) if (x.name === op.name) x.fetch_url = x.push_url = op.url;
      return `Changed ${op.name}'s URL`;
    case "revert": {
      const target = r.commits.find((c) => c.sha === op.sha);
      const c = commit(r.commits.length + 60, `Revert "${target?.subject}"`, "You", 0, ["HEAD -> main"]);
      r.commits[0].refs = [];
      r.commits.unshift(c);
      return `Reverted in a new commit ${c.short}`;
    }
    case "restore_files":
      return `Restored ${op.paths.length} file(s) from ${op.sha.slice(0, 7)}`;
    case "reset": {
      const at = Math.max(
        r.commits.findIndex((c) => c.sha === op.target || c.short === op.target),
        1,
      );
      r.commits = r.commits.slice(at);
      r.commits[0].refs = ["HEAD -> main"];
      if (op.mode === "hard") s.changes = s.changes.filter((c) => c.untracked);
      return `Reset main to ${r.commits[0].short} (${op.mode})`;
    }
    case "merge":
      return `Merged ${op.branch}`;
    case "abort_merge":
      return "Aborted the merge";
    case "worktree_add": {
      const main = r.worktrees[0].path;
      const slug = op.branch
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-|-$/g, "");
      const path = op.path ?? `${main}-${slug}`;
      if (r.worktrees.some((w) => w.path === path)) throw new DaemonError("app", `${path} already exists`, "CONFLICT");
      r.worktrees.push({
        path,
        branch: op.branch,
        head: r.commits[0].sha,
        main: false,
        locked: false,
        prunable: false,
        site: op.site ? (path.split("/").pop() ?? null) : null,
      });
      return `Preparing worktree (new branch '${op.branch}')${op.site ? " and registered as a site" : ""}`;
    }
    case "worktree_remove":
      r.worktrees = r.worktrees.filter((w) => w.path !== op.path);
      return "worktree remove done";
    case "worktree_prune":
      r.worktrees = r.worktrees.filter((w) => !w.prunable);
      return "worktree prune done";
  }
}

const DIFF = `diff --git a/resources/js/cart.ts b/resources/js/cart.ts
--- a/resources/js/cart.ts
+++ b/resources/js/cart.ts
@@ -12,7 +12,10 @@ export function renew(session: Session) {
   const cart = load(session.id);
-  cart.coupons = [];
+  // Coupons survive a renewed session.
+  if (session.renewed) {
+    cart.coupons = keepValid(cart.coupons);
+  }
   return save(cart);
 }
`;

function github(name: string, section: string): GithubData {
  if (!account) throw unauthorized("no GitHub account connected (Settings → Integrations)");
  const r = find(name);
  if (!r.status.github) throw new DaemonError("app", `${name} has no GitHub remote`, "NOT_FOUND");
  const { owner, name: repoName } = r.status.github;
  const url = `https://github.com/${owner}/${repoName}`;
  const iso = (h: number) => new Date(Date.now() - h * 3600_000).toISOString();
  switch (section) {
    case "overview":
      return {
        section: "overview",
        repo: r.status.github,
        description: "The Acme storefront",
        private: true,
        default_branch: "main",
        html_url: url,
        open_issues: 3,
        stars: 12,
        rate_remaining: 4987,
      };
    case "issues":
      return {
        section: "issues",
        items: [
          {
            number: 41,
            title: "Coupons vanish after the session renews",
            state: "open",
            author: "ana",
            url: `${url}/issues/41`,
            created_at: iso(30),
            updated_at: iso(2),
            labels: ["bug"],
            branch: null,
            draft: false,
          },
          {
            number: 38,
            title: "Export orders as CSV",
            state: "open",
            author: "rui",
            url: `${url}/issues/38`,
            created_at: iso(200),
            updated_at: iso(20),
            labels: ["feature"],
            branch: null,
            draft: false,
          },
        ],
      };
    case "pulls":
      return {
        section: "pulls",
        items: [
          {
            number: 42,
            title: "Keep coupons when the session renews",
            state: "open",
            author: "ana",
            url: `${url}/pull/42`,
            created_at: iso(3),
            updated_at: iso(1),
            labels: [],
            branch: "fix/coupons",
            draft: false,
          },
        ],
      };
    case "workflows":
      return {
        section: "workflows",
        workflows: [
          {
            id: 1,
            name: "CI",
            path: ".github/workflows/ci.yml",
            state: "active",
            url: `${url}/actions/workflows/ci.yml`,
          },
          {
            id: 2,
            name: "Deploy",
            path: ".github/workflows/deploy.yml",
            state: "active",
            url: `${url}/actions/workflows/deploy.yml`,
          },
        ],
      };
    case "runs":
      return {
        section: "runs",
        runs: [
          {
            id: 9003,
            name: "CI",
            branch: "fix/coupons",
            event: "pull_request",
            status: "in_progress",
            conclusion: null,
            sha: hex(7),
            url: `${url}/actions/runs/9003`,
            created_at: iso(0.1),
            workflow_id: 1,
          },
          {
            id: 9002,
            name: "CI",
            branch: "main",
            event: "push",
            status: "completed",
            conclusion: "failure",
            sha: hex(8),
            url: `${url}/actions/runs/9002`,
            created_at: iso(2),
            workflow_id: 1,
          },
          {
            id: 9001,
            name: "Deploy",
            branch: "main",
            event: "workflow_dispatch",
            status: "completed",
            conclusion: "success",
            sha: hex(9),
            url: `${url}/actions/runs/9001`,
            created_at: iso(26),
            workflow_id: 2,
          },
        ],
      };
    case "releases":
      return {
        section: "releases",
        releases: [
          {
            tag: "v2.4.0",
            name: "2.4.0",
            draft: false,
            prerelease: false,
            published_at: iso(100),
            url: `${url}/releases/tag/v2.4.0`,
          },
        ],
      };
    case "branches":
      return { section: "branches", names: ["main", "fix/coupons"] };
    case "commits":
      return { section: "commits", commits: r.commits };
    default:
      return {
        section: "checks",
        sha: r.status.head ?? "",
        checks: [
          { name: "test (php 8.4)", status: "completed", conclusion: "failure", url: `${url}/actions/runs/9002` },
          { name: "lint", status: "completed", conclusion: "success", url: `${url}/actions/runs/9002` },
        ],
      };
  }
}

export function handleGit(req: GitRequest): GitResponse {
  switch (req.type) {
    case "git_status":
      return { type: "git_status", ...structuredClone(find(req.name).status) };
    case "git_diff": {
      const r = find(req.name);
      const change = r.status.changes.find((c) => c.path === req.path);
      const text = change?.untracked
        ? `+ (new file)\n+${"\n".repeat(0)}# TODO\n+- pricing table\n`
        : DIFF.replaceAll("resources/js/cart.ts", req.path);
      return { type: "text", text, truncated: false };
    }
    case "git_log": {
      const r = find(req.name);
      return {
        type: "commits",
        commits: structuredClone(r.commits.slice(req.skip ?? 0, (req.skip ?? 0) + (req.limit ?? 50))),
      };
    }
    case "git_show": {
      const c = find(req.name).commits.find((c) => c.sha === req.sha);
      if (!c) throw new DaemonError("app", "unknown commit", "NOT_FOUND");
      return {
        type: "commit",
        commit: c,
        body: "",
        files: [{ path: "resources/js/cart.ts", added: 4, removed: 1 }],
        diff: DIFF,
        truncated: false,
      };
    }
    case "git_branches":
      return { type: "branches", branches: structuredClone(find(req.name).branches) };
    case "git_stashes":
      return { type: "stashes", stashes: structuredClone(find(req.name).stashes) };
    case "git_remotes":
      return { type: "remotes", remotes: structuredClone(find(req.name).remotes) };
    case "git_worktrees": {
      const r = find(req.name);
      r.worktrees[0].site ??= req.name;
      return { type: "worktrees", worktrees: structuredClone(r.worktrees) };
    }
    case "git_history": {
      const r = find(req.name);
      return { type: "git_history", reflog: structuredClone(r.reflog), operations: [...r.operations] };
    }
    case "git_preview":
      return { type: "git_preview", preview: preview(find(req.name), req.op) };
    case "git": {
      const r = find(req.name);
      const p = preview(r, req.op);
      const stamp = new Date().toISOString().slice(0, 19).replace("T", " ");
      if (p) {
        try {
          if (p.blocked) throw new DaemonError("app", p.blocked, "CONFLICT");
          if (req.confirm?.phrase.trim() !== p.phrase)
            throw new DaemonError("app", `type "${p.phrase}" to confirm`, "INVALID_INPUT");
          if (p.password) checkPassword(req.confirm?.password);
        } catch (error) {
          r.operations.unshift(`${stamp} refused ${req.op.op}: ${(error as Error).message}`);
          throw error;
        }
      }
      const message = run(r, req.op);
      const backup = p?.backup ? `refs/locdev/backups/${Date.now()}` : null;
      r.operations.unshift(`${stamp} ${req.op.op}: ${message}${backup ? ` (backup ${backup})` : ""}`);
      return { type: "git_result", message, output: "", backup };
    }
    case "get_security":
      return { type: "security", ...security };
    case "set_password":
      if (security.password_set) checkPassword(req.current);
      if (req.new != null && req.new.length < 8)
        throw new DaemonError("app", "use at least 8 characters", "INVALID_INPUT");
      password = req.new ?? null;
      security = { ...security, password_set: password !== null };
      return { type: "security", ...security };
    case "set_security_options":
      checkPassword(req.password);
      security = { ...security, require_password: req.require_password, backups: req.backups };
      return { type: "security", ...security };
    case "github_account":
      return { type: "github", account };
    case "github_connect_token":
      if (!/^(gh[pousr]_|github_pat_)\w{20,}$/.test(req.token.trim()))
        throw unauthorized("GitHub didn't accept the token");
      account = {
        login: "acme-dev",
        name: "Acme Developer",
        method: "token",
        scopes: ["repo", "workflow"],
        storage: "system keyring",
      };
      return { type: "github", account };
    case "github_connect_gh_cli":
      account = {
        login: "acme-dev",
        name: "Acme Developer",
        method: "gh_cli",
        scopes: ["repo", "workflow", "read:org"],
        storage: "system keyring",
      };
      return { type: "github", account };
    case "github_device_start":
      devicePolls = 0;
      return {
        type: "device_code",
        user_code: "WDJB-MJHT",
        verification_uri: "https://github.com/login/device",
        expires_in: 900,
        interval: 5,
      };
    case "github_device_poll":
      if (++devicePolls < 2) return { type: "github", account: null };
      account = {
        login: "acme-dev",
        name: "Acme Developer",
        method: "device",
        scopes: ["repo", "workflow"],
        storage: "system keyring",
      };
      return { type: "github", account };
    case "github_disconnect":
      account = null;
      return { type: "github", account };
    case "github":
      return { type: "github_data", ...github(req.name, req.section) };
    case "github_action":
      github(req.name, "overview");
      return {
        type: "git_result",
        message:
          req.action.action === "dispatch"
            ? `Started the workflow on ${req.action.git_ref}`
            : req.action.action === "cancel_run"
              ? "Asked GitHub to cancel the run"
              : "Re-running the failed jobs",
        output: "",
        backup: null,
      };
  }
}
