import {
  Archive,
  ArrowDown,
  ArrowUp,
  GitBranch,
  GitCommitHorizontal,
  GitMerge,
  Minus,
  MoreHorizontal,
  Plus,
  RefreshCw,
  ShieldAlert,
  Trash2,
  Undo2,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Section } from "@/components/page";
import { Led } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Textarea } from "@/components/ui/textarea";
import {
  useGit,
  useGitBranches,
  useGitDiff,
  useGitHistory,
  useGitLog,
  useGitRemotes,
  useGitShow,
  useGitStashes,
  useGitStatus,
} from "@/hooks/use-daemon";
import { request } from "@/lib/daemon";
import { uptime } from "@/lib/format";
import type { ChangeKind, Commit, DestructivePreview, FileChange, GitOp, ProjectView, ResetMode } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { intlLocale, t, tn } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";
import { Worktrees } from "@/components/insights";

type View = "changes" | "history" | "branches" | "worktrees" | "stashes" | "remotes" | "activity";

const KIND: Record<ChangeKind, string> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  copied: "C",
  type_changed: "T",
};

const ago = (secs: number) => uptime(secs * 1000).split(" ")[0];
const message = (error: unknown) => (error instanceof Error ? error.message : String(error));

/**
 * Runs Git operations for one site. Destructive ones (the daemon decides,
 * through the preview) open the confirmation dialog instead of running.
 */
function useRunner(name: string) {
  const git = useGit(name);
  const [confirm, setConfirm] = useState<{
    op: GitOp;
    preview: DestructivePreview;
  } | null>(null);
  const run = async (op: GitOp, done?: () => void) => {
    try {
      const { preview } = await request({ type: "git_preview", name, op }, "git_preview");
      if (preview) {
        setConfirm({ op, preview });
        return;
      }
      const result = await git.mutateAsync({ op });
      toast.success(result.message);
      done?.();
    } catch (error) {
      toast.error(t("Git refused"), { description: message(error) });
    }
  };
  const dialog = confirm && (
    <DestructiveDialog name={name} op={confirm.op} preview={confirm.preview} onClose={() => setConfirm(null)} />
  );
  return { run, busy: git.isPending, dialog };
}

export function GitTab({ site }: { site: ProjectView }) {
  const status = useGitStatus(site.name);
  const [view, setView] = useState<View>("changes");
  const { run, busy, dialog } = useRunner(site.name);
  const stashCount = useGitStashes(site.name).data?.length ?? 0;

  if (status.isPending) return <Skeleton className="h-40" />;
  if (status.isError || !status.data) {
    return (
      <p className="text-sm text-muted-foreground">
        {tx("{error}. Run {git_init} in the site's folder to track it here.", {
          error: message(status.error),
          git_init: <span className="font-mono">git init</span>,
        })}
      </p>
    );
  }
  const s = status.data;

  return (
    <>
      <div className="flex flex-wrap items-center gap-3 text-[13px]">
        <span className="flex items-center gap-1.5 font-mono font-medium">
          <GitBranch className="size-3.5 text-ember" />
          {s.branch ?? t("detached HEAD")}
        </span>
        {s.upstream && (
          <span className="font-mono text-[12px] text-muted-foreground">
            → {s.upstream}
            {(s.ahead > 0 || s.behind > 0) && (
              <>
                {" "}
                <ArrowUp className="inline size-3" />
                {s.ahead} <ArrowDown className="inline size-3" />
                {s.behind}
              </>
            )}
          </span>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {s.upstream && (
            <>
              <Button size="sm" variant="outline" disabled={busy} onClick={() => run({ op: "fetch" })}>
                <RefreshCw data-icon="inline-start" /> {t("Fetch")}
              </Button>
              <Button size="sm" variant="outline" disabled={busy || s.behind === 0} onClick={() => run({ op: "pull" })}>
                <ArrowDown data-icon="inline-start" /> {t("Pull")}
              </Button>
              <Button size="sm" variant="outline" disabled={busy} onClick={() => run({ op: "push" })}>
                <ArrowUp data-icon="inline-start" /> {t("Push")}
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger
                  render={<Button size="icon-sm" variant="ghost" aria-label={t("More Git actions")} />}
                >
                  <MoreHorizontal />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem variant="destructive" onClick={() => run({ op: "push", force: true })}>
                    {t("Force push (with lease)…")}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )}
        </div>
      </div>

      {s.state !== "clean" && (
        <div className="flex items-center gap-3 rounded-md border border-amber/30 bg-amber/[0.07] px-3 py-2 text-[13px] text-amber">
          <GitMerge className="size-4" />A {s.state.replace("_", "-")} {t("is in progress")}
          {s.changes.some((c) => c.conflicted) && t(" with conflicts to resolve")}.
          {s.state === "merging" && (
            <Button size="sm" variant="outline" className="ml-auto" onClick={() => run({ op: "abort_merge" })}>
              {t("Abort merge…")}
            </Button>
          )}
        </div>
      )}

      <div role="radiogroup" aria-label={t("Git section")} className="flex w-fit rounded-md border p-0.5 text-[12.5px]">
        {(
          [
            ["changes", `${t("Changes")} ${s.changes.length || ""}`],
            ["history", t("History")],
            ["branches", t("Branches")],
            ["worktrees", t("Worktrees")],
            ["stashes", `${t("Stashes")} ${stashCount || ""}`],
            ["remotes", t("Remotes")],
            ["activity", t("Activity")],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="radio"
            aria-checked={view === id}
            onClick={() => setView(id)}
            className={cn(
              "rounded px-2.5 py-1 text-muted-foreground transition-colors hover:text-foreground",
              view === id && "bg-foreground/[0.07] text-foreground",
            )}
          >
            {label}
          </button>
        ))}
      </div>

      {view === "changes" ? (
        <Changes name={site.name} changes={s.changes} run={run} busy={busy} />
      ) : view === "history" ? (
        <History name={site.name} run={run} />
      ) : view === "branches" ? (
        <Branches name={site.name} run={run} />
      ) : view === "worktrees" ? (
        <Worktrees name={site.name} run={run} busy={busy} />
      ) : view === "stashes" ? (
        <Stashes name={site.name} run={run} hasChanges={s.changes.length > 0} />
      ) : view === "remotes" ? (
        <Remotes name={site.name} run={run} />
      ) : (
        <Activity name={site.name} />
      )}
      {dialog}
    </>
  );
}

type Run = (op: GitOp, done?: () => void) => Promise<void>;

// ── Changes ────────────────────────────────────────────────────────────────

function Changes({ name, changes, run, busy }: { name: string; changes: FileChange[]; run: Run; busy: boolean }) {
  const [open, setOpen] = useState<{ path: string; staged: boolean } | null>(null);
  const [text, setText] = useState("");
  const staged = changes.filter((c) => c.staged);
  const unstaged = changes.filter((c) => c.unstaged || c.untracked || c.conflicted);
  const diff = useGitDiff(name, open?.path ?? null, open?.staged ?? false);

  if (changes.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("Nothing to commit: the working tree matches the last commit.")}
      </p>
    );
  }

  const row = (c: FileChange, inIndex: boolean) => {
    const kind = inIndex ? c.staged : c.unstaged;
    const active = open?.path === c.path && open.staged === inIndex;
    return (
      <div
        key={`${inIndex}:${c.path}`}
        className={cn(
          "group flex h-8 items-center gap-2 rounded px-2 text-[12.5px] hover:bg-foreground/4",
          active && "bg-foreground/[0.07]",
        )}
      >
        <span
          className={cn(
            "w-4 shrink-0 text-center font-mono font-semibold",
            c.conflicted
              ? "text-destructive"
              : inIndex
                ? "text-phosphor"
                : c.untracked
                  ? "text-muted-foreground"
                  : "text-amber",
          )}
        >
          {c.conflicted ? "U" : c.untracked ? "?" : kind ? KIND[kind] : ""}
        </span>
        <button
          type="button"
          className="min-w-0 flex-1 truncate text-left font-mono"
          title={c.orig_path ? `${c.orig_path} → ${c.path}` : c.path}
          onClick={() => setOpen({ path: c.path, staged: inIndex })}
        >
          {c.path}
        </button>
        {c.added != null && (
          <span className="font-mono text-[11px] text-muted-foreground">
            <span className="text-phosphor">+{c.added}</span> <span className="text-destructive">-{c.removed}</span>
          </span>
        )}
        <span className="flex opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
          {inIndex ? (
            <Button
              size="icon-xs"
              variant="ghost"
              aria-label={t("Unstage {path}", { path: c.path })}
              title={t("Unstage")}
              onClick={() => run({ op: "unstage", paths: [c.path] })}
            >
              <Minus />
            </Button>
          ) : (
            <>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label={t("Discard {path}", { path: c.path })}
                title={t("Discard changes…")}
                onClick={() => run({ op: "discard", paths: [c.path] })}
              >
                <Undo2 />
              </Button>
              <Button
                size="icon-xs"
                variant="ghost"
                aria-label={t("Stage {path}", { path: c.path })}
                title={t("Stage")}
                onClick={() => run({ op: "stage", paths: [c.path] })}
              >
                <Plus />
              </Button>
            </>
          )}
        </span>
      </div>
    );
  };

  return (
    <div className="@container">
      <div className="grid min-h-0 gap-5 @3xl:grid-cols-[minmax(0,20rem)_minmax(0,1fr)]">
        <div className="flex flex-col gap-4">
          <Section
            title={t("Staged {length}", { length: staged.length })}
            aside={
              staged.length > 0 && (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => run({ op: "unstage", paths: staged.map((c) => c.path) })}
                >
                  {t("Unstage all")}
                </Button>
              )
            }
          >
            {staged.length === 0 ? (
              <p className="px-2 py-1 text-[12.5px] text-muted-foreground">
                {t("Stage files to include them in the commit.")}
              </p>
            ) : (
              staged.map((c) => row(c, true))
            )}
          </Section>
          <Section
            title={t("Changes {length}", { length: unstaged.length })}
            aside={
              unstaged.length > 0 && (
                <Button
                  size="xs"
                  variant="ghost"
                  onClick={() => run({ op: "stage", paths: unstaged.map((c) => c.path) })}
                >
                  {t("Stage all")}
                </Button>
              )
            }
          >
            {unstaged.map((c) => row(c, false))}
          </Section>
          <div className="flex flex-col gap-2">
            <Textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={t("Commit message (first line: a short summary)")}
              aria-label={t("Commit message")}
              className="min-h-20 font-mono text-[12.5px]"
            />
            <Button
              disabled={busy || staged.length === 0 || !text.trim()}
              onClick={() => run({ op: "commit", message: text.trim() }, () => setText(""))}
              className="bg-ember text-ember-foreground hover:bg-ember/85"
            >
              <GitCommitHorizontal data-icon="inline-start" />{" "}
              {tn("Commit {n} file", "Commit {n} files", staged.length)}
            </Button>
          </div>
        </div>
        <div className="min-w-0">
          {open ? (
            diff.isPending ? (
              <Skeleton className="h-40" />
            ) : diff.isError ? (
              <p className="text-sm text-destructive">{message(diff.error)}</p>
            ) : (
              <DiffView text={diff.data.text} truncated={diff.data.truncated} />
            )
          ) : (
            <p className="pt-8 text-center text-sm text-muted-foreground">{t("Select a file to see what changed.")}</p>
          )}
        </div>
      </div>
    </div>
  );
}

export function DiffView({ text, truncated }: { text: string; truncated?: boolean }) {
  if (!text.trim())
    return <p className="text-sm text-muted-foreground">{t("No textual changes (binary or mode only).")}</p>;
  return (
    <div className="flex flex-col gap-2">
      <pre
        className="max-h-128 overflow-auto rounded-md bg-well py-2 font-mono text-[11.5px] leading-relaxed"
        data-selectable
      >
        {text.split("\n").map((line, i) => (
          <div
            key={i}
            className={cn(
              "px-3 whitespace-pre",
              line.startsWith("+") && !line.startsWith("+++") && "bg-phosphor/10 text-phosphor",
              line.startsWith("-") && !line.startsWith("---") && "bg-destructive/10 text-destructive",
              line.startsWith("@@") && "text-ember",
              (line.startsWith("diff ") || line.startsWith("+++") || line.startsWith("---")) && "text-muted-foreground",
            )}
          >
            {line || " "}
          </div>
        ))}
      </pre>
      {truncated && (
        <p className="text-[12px] text-amber">{t("The diff is too large; only the beginning is shown.")}</p>
      )}
    </div>
  );
}

// ── History ────────────────────────────────────────────────────────────────

function History({ name, run }: { name: string; run: Run }) {
  const [limit, setLimit] = useState(50);
  const log = useGitLog(name, limit);
  const [open, setOpen] = useState<string | null>(null);

  if (log.isPending) return <Skeleton className="h-40" />;
  const commits = log.data ?? [];
  return (
    // Side by side when there's room; otherwise the open commit replaces the list.
    <div className="@container">
      <div className="grid gap-5 @4xl:grid-cols-[minmax(0,24rem)_minmax(0,1fr)]">
        <div className={cn("flex flex-col", open && "hidden @4xl:flex")}>
          {commits.map((c) => (
            <button
              key={c.sha}
              type="button"
              onClick={() => setOpen(c.sha)}
              className={cn(
                "flex flex-col gap-0.5 rounded px-2 py-1.5 text-left hover:bg-foreground/4",
                open === c.sha && "bg-foreground/[0.07]",
              )}
            >
              <span className="flex items-center gap-2 text-[13px]">
                <span className="font-mono text-[11.5px] text-ember">{c.short}</span>
                <span className="truncate">{c.subject}</span>
              </span>
              <span className="flex gap-2 font-mono text-[11px] text-muted-foreground">
                {c.author} · {ago(c.time)} {t("ago")}
                {c.refs.map((r) => (
                  <span key={r} className="rounded bg-foreground/6 px-1">
                    {r}
                  </span>
                ))}
              </span>
            </button>
          ))}
          {commits.length >= limit && (
            <Button size="sm" variant="ghost" className="mt-2 self-start" onClick={() => setLimit(limit + 50)}>
              {t("Load more")}
            </Button>
          )}
        </div>
        <div className={cn("min-w-0", !open && "hidden @4xl:block")}>
          {open ? (
            <>
              <Button size="xs" variant="ghost" className="mb-2 @4xl:hidden" onClick={() => setOpen(null)}>
                {t("← All commits")}
              </Button>
              <CommitView key={open} name={name} sha={open} commit={commits.find((c) => c.sha === open)} run={run} />
            </>
          ) : (
            <p className="pt-8 text-center text-sm text-muted-foreground">{t("Select a commit to see its changes.")}</p>
          )}
        </div>
      </div>
    </div>
  );
}

function CommitView({ name, sha, commit, run }: { name: string; sha: string; commit?: Commit; run: Run }) {
  const detail = useGitShow(name, sha);
  const [branch, setBranch] = useState("");
  const [files, setFiles] = useState<Set<string>>(new Set());
  const [mode, setMode] = useState<ResetMode>("mixed");
  if (detail.isPending) return <Skeleton className="h-40" />;
  if (detail.isError) return <p className="text-sm text-destructive">{message(detail.error)}</p>;
  const d = detail.data;
  const toggle = (path: string) =>
    setFiles((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h3 className="text-[15px] font-medium" data-selectable>
          {d.commit.subject}
        </h3>
        <p className="font-mono text-[11.5px] break-all text-muted-foreground" data-selectable>
          {d.commit.sha} · {d.commit.author} &lt;{d.commit.email}&gt; ·{" "}
          {new Date(d.commit.time * 1000).toLocaleString(intlLocale)}
        </p>
        {d.body && (
          <pre className="mt-2 font-sans text-[13px] whitespace-pre-wrap text-muted-foreground" data-selectable>
            {d.body}
          </pre>
        )}
      </div>

      {/* Three different ways back, explained side by side so they aren't confused. */}
      <Section title={t("From this commit")}>
        <div className="flex flex-col gap-3 text-[13px]">
          <Action
            title={t("New branch here")}
            text={t("Starts a branch at this commit. Nothing changes on the current branch.")}
          >
            <Input
              value={branch}
              onChange={(e) => setBranch(e.target.value)}
              placeholder={t("branch name")}
              aria-label={t("Branch name")}
              className="h-7 w-44 font-mono text-[12px]"
            />
            <Button
              size="xs"
              variant="outline"
              disabled={!branch.trim()}
              onClick={() =>
                run(
                  {
                    op: "create_branch",
                    name: branch.trim(),
                    start: sha,
                    switch: true,
                  },
                  () => setBranch(""),
                )
              }
            >
              {t("Create and switch")}
            </Button>
          </Action>
          <Action
            title={t("Revert")}
            text={t("Adds a new commit that undoes this one. History is kept: safe on shared branches.")}
          >
            <Button size="xs" variant="outline" onClick={() => run({ op: "revert", sha })}>
              {t("Revert")} {commit?.short ?? sha.slice(0, 7)}
            </Button>
          </Action>
          <Action
            title={t("Restore files")}
            text={t(
              "Puts the files you tick below back as they were in this commit, as uncommitted changes. Other files and history are untouched.",
            )}
          >
            <Button
              size="xs"
              variant="outline"
              disabled={files.size === 0}
              onClick={() => run({ op: "restore_files", sha, paths: [...files] }, () => setFiles(new Set()))}
            >
              {files.size ? tn("Restore {n} file…", "Restore {n} files…", files.size) : t("Restore files…")}
            </Button>
          </Action>
          <Action
            title={t("Reset branch here")}
            text={t(
              "Moves the current branch to this commit. Later commits leave the branch (the reflog keeps them). Hard also throws away uncommitted changes.",
            )}
            danger
          >
            <div role="radiogroup" aria-label={t("Reset mode")} className="flex rounded-md border p-0.5 text-[11.5px]">
              {(["soft", "mixed", "hard"] as const).map((m) => (
                <button
                  key={m}
                  type="button"
                  role="radio"
                  aria-checked={mode === m}
                  onClick={() => setMode(m)}
                  title={
                    m === "soft"
                      ? t("Changes stay staged")
                      : m === "mixed"
                        ? t("Changes stay in the working tree")
                        : t("Changes are thrown away")
                  }
                  className={cn(
                    "rounded px-2 py-0.5 text-muted-foreground",
                    mode === m && "bg-foreground/[0.07] text-foreground",
                  )}
                >
                  {m}
                </button>
              ))}
            </div>
            <Button size="xs" variant="destructive" onClick={() => run({ op: "reset", target: sha, mode })}>
              {t("Reset…")}
            </Button>
          </Action>
        </div>
      </Section>

      <Section title={t("Files {length}", { length: d.files.length })}>
        {d.files.map((f, i) => (
          <label key={f.path} className="flex h-7 items-center gap-2 px-1 text-[12.5px]" htmlFor={`commit-file-${i}`}>
            <Checkbox
              id={`commit-file-${i}`}
              checked={files.has(f.path)}
              onCheckedChange={() => toggle(f.path)}
              aria-label={t("Select {path}", { path: f.path })}
            />
            <span className="min-w-0 flex-1 truncate font-mono">{f.path}</span>
            {f.added != null && (
              <span className="font-mono text-[11px]">
                <span className="text-phosphor">+{f.added}</span> <span className="text-destructive">-{f.removed}</span>
              </span>
            )}
          </label>
        ))}
      </Section>
      <DiffView text={d.diff} truncated={d.truncated} />
    </div>
  );
}

function Action({
  title,
  text,
  danger,
  children,
}: {
  title: string;
  text: string;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
      <div className="min-w-56 flex-1">
        <div className={cn("font-medium", danger && "text-destructive")}>{title}</div>
        <div className="text-[12.5px] text-muted-foreground">{text}</div>
      </div>
      <div className="flex items-center gap-2">{children}</div>
    </div>
  );
}

// ── Branches, stashes, remotes, activity ────────────────────────────────────

function Branches({ name, run }: { name: string; run: Run }) {
  const branches = useGitBranches(name);
  const [newName, setNewName] = useState("");
  if (branches.isPending) return <Skeleton className="h-32" />;
  const all = branches.data ?? [];
  const groups = [
    [t("Local"), all.filter((b) => !b.remote)],
    [t("Remote"), all.filter((b) => b.remote)],
  ] as const;

  return (
    <div className="flex max-w-3xl flex-col gap-5">
      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          run({ op: "create_branch", name: newName.trim(), switch: true }, () => setNewName(""));
        }}
      >
        <Input
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder={t("new-branch")}
          aria-label={t("New branch name")}
          className="h-8 w-56 font-mono text-[12.5px]"
        />
        <Button size="sm" variant="outline" type="submit" disabled={!newName.trim()}>
          <Plus data-icon="inline-start" /> {t("New branch from HEAD")}
        </Button>
      </form>
      {groups.map(
        ([label, list]) =>
          list.length > 0 && (
            <Section key={label} title={`${label} ${list.length}`}>
              {list.map((b) => (
                <div key={b.name} className="group flex h-9 items-center gap-3 px-1 text-[13px]">
                  <Led tone={b.current ? "live" : "off"} />
                  <span className={cn("font-mono", b.current && "font-semibold")}>{b.name}</span>
                  {b.upstream && (
                    <span className="font-mono text-[11px] text-muted-foreground">
                      → {b.upstream}
                      {(b.ahead > 0 || b.behind > 0) && ` ↑${b.ahead} ↓${b.behind}`}
                    </span>
                  )}
                  <span className="min-w-0 flex-1 truncate text-[12.5px] text-muted-foreground">
                    {b.subject} · {ago(b.time)} {t("ago")}
                  </span>
                  {!b.current && (
                    <span className="flex gap-1 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
                      <Button size="xs" variant="ghost" onClick={() => run({ op: "switch", name: b.name })}>
                        {t("Switch")}
                      </Button>
                      <Button size="xs" variant="ghost" onClick={() => run({ op: "merge", branch: b.name })}>
                        {t("Merge into current")}
                      </Button>
                      {!b.remote && (
                        <Button
                          size="icon-xs"
                          variant="ghost"
                          aria-label={t("Delete {name}", { name: b.name })}
                          title={b.merged ? t("Delete (merged)") : t("Delete (not merged)…")}
                          onClick={() =>
                            run({
                              op: "delete_branch",
                              name: b.name,
                              force: !b.merged,
                            })
                          }
                        >
                          <Trash2 />
                        </Button>
                      )}
                    </span>
                  )}
                </div>
              ))}
            </Section>
          ),
      )}
    </div>
  );
}

function Stashes({ name, run, hasChanges }: { name: string; run: Run; hasChanges: boolean }) {
  const stashes = useGitStashes(name);
  const [text, setText] = useState("");
  const [untracked, setUntracked] = useState(true);
  return (
    <div className="flex max-w-3xl flex-col gap-5">
      <form
        className="flex flex-wrap items-center gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          run({ op: "stash_push", message: text.trim() || null, untracked }, () => setText(""));
        }}
      >
        <Input
          value={text}
          onChange={(e) => setText(e.target.value)}
          placeholder={t("What's in it (optional)")}
          aria-label={t("Stash message")}
          className="h-8 w-64 text-[13px]"
        />
        <label className="flex items-center gap-2 text-[12.5px] text-muted-foreground" htmlFor="untracked">
          <Checkbox id="untracked" checked={untracked} onCheckedChange={(v) => setUntracked(v === true)} />{" "}
          {t("include untracked files")}
        </label>
        <Button size="sm" variant="outline" type="submit" disabled={!hasChanges}>
          <Archive data-icon="inline-start" /> {t("Stash changes")}
        </Button>
      </form>
      <Section title={t("Saved")}>
        {(stashes.data ?? []).length === 0 ? (
          <p className="px-1 py-1 text-[12.5px] text-muted-foreground">{t("No stashes.")}</p>
        ) : (
          stashes.data!.map((s) => (
            <div key={s.index} className="group flex h-9 items-center gap-3 px-1 text-[13px]">
              <span className="font-mono text-[11.5px] text-ember">
                {t("stash@")}
                {`{${s.index}}`}
              </span>
              <span className="min-w-0 flex-1 truncate">{s.message}</span>
              <span className="font-mono text-[11px] text-muted-foreground">
                {ago(s.time)} {t("ago")}
              </span>
              <span className="flex gap-1">
                <Button size="xs" variant="ghost" onClick={() => run({ op: "stash_apply", index: s.index })}>
                  {t("Apply")}
                </Button>
                <Button size="xs" variant="ghost" onClick={() => run({ op: "stash_pop", index: s.index })}>
                  {t("Pop")}
                </Button>
                <Button
                  size="icon-xs"
                  variant="ghost"
                  aria-label={t("Drop stash {index}", { index: s.index })}
                  title={t("Drop…")}
                  onClick={() => run({ op: "stash_drop", index: s.index })}
                >
                  <Trash2 />
                </Button>
              </span>
            </div>
          ))
        )}
      </Section>
    </div>
  );
}

function Remotes({ name, run }: { name: string; run: Run }) {
  const remotes = useGitRemotes(name);
  const [form, setForm] = useState({ name: "", url: "" });
  const [editing, setEditing] = useState<{ name: string; url: string } | null>(null);
  return (
    <div className="flex max-w-3xl flex-col gap-5">
      <Section title={t("Remotes")}>
        {(remotes.data ?? []).length === 0 && (
          <p className="px-1 py-1 text-[12.5px] text-muted-foreground">{t("No remotes.")}</p>
        )}
        {(remotes.data ?? []).map((r) => (
          <div key={r.name} className="group flex min-h-9 items-center gap-3 px-1 text-[13px]">
            <span className="w-20 shrink-0 font-mono font-medium">{r.name}</span>
            {editing?.name === r.name ? (
              <form
                className="flex flex-1 gap-2"
                onSubmit={(e) => {
                  e.preventDefault();
                  run(
                    {
                      op: "remote_set_url",
                      name: r.name,
                      url: editing.url.trim(),
                    },
                    () => setEditing(null),
                  );
                }}
              >
                <Input
                  autoFocus
                  value={editing.url}
                  onChange={(e) => setEditing({ ...editing, url: e.target.value })}
                  aria-label={t("Remote URL")}
                  className="h-7 flex-1 font-mono text-[12px]"
                />
                <Button size="xs" type="submit">
                  {t("Save")}
                </Button>
                <Button size="xs" variant="ghost" type="button" onClick={() => setEditing(null)}>
                  {t("Cancel")}
                </Button>
              </form>
            ) : (
              <>
                <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-muted-foreground" data-selectable>
                  {r.fetch_url}
                  {r.push_url !== r.fetch_url && t(" (push: {push_url})", { push_url: r.push_url })}
                </span>
                <span className="flex gap-1 opacity-0 group-focus-within:opacity-100 group-hover:opacity-100">
                  <Button size="xs" variant="ghost" onClick={() => setEditing({ name: r.name, url: r.fetch_url })}>
                    {t("Change URL")}
                  </Button>
                  <Button
                    size="icon-xs"
                    variant="ghost"
                    aria-label={t("Remove {name}", { name: r.name })}
                    onClick={() => run({ op: "remote_remove", name: r.name })}
                  >
                    <Trash2 />
                  </Button>
                </span>
              </>
            )}
          </div>
        ))}
      </Section>
      <form
        className="flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          run({ op: "remote_add", name: form.name.trim(), url: form.url.trim() }, () => setForm({ name: "", url: "" }));
        }}
      >
        <Input
          value={form.name}
          onChange={(e) => setForm({ ...form, name: e.target.value })}
          placeholder={t("upstream")}
          aria-label={t("Remote name")}
          className="h-8 w-32 font-mono text-[12.5px]"
        />
        <Input
          value={form.url}
          onChange={(e) => setForm({ ...form, url: e.target.value })}
          placeholder={t("git@github.com:owner/repo.git")}
          aria-label={t("Remote URL")}
          className="h-8 w-80 font-mono text-[12.5px]"
        />
        <Button size="sm" variant="outline" type="submit" disabled={!form.name.trim() || !form.url.trim()}>
          <Plus data-icon="inline-start" /> {t("Add remote")}
        </Button>
      </form>
    </div>
  );
}

function Activity({ name }: { name: string }) {
  const history = useGitHistory(name);
  if (history.isPending) return <Skeleton className="h-32" />;
  const { operations, reflog } = history.data ?? { operations: [], reflog: [] };
  return (
    <div className="flex max-w-4xl flex-col gap-6">
      <Section title={t("Run through LocDev")}>
        {operations.length === 0 ? (
          <p className="px-1 py-1 text-[12.5px] text-muted-foreground">
            {t("Nothing yet. Operations and refusals show up here.")}
          </p>
        ) : (
          <pre className="max-h-72 overflow-auto font-mono text-[11.5px] leading-relaxed" data-selectable>
            {operations.map((line, i) => (
              <div key={i} className={cn(line.includes(" refused ") && "text-destructive")}>
                {line}
              </div>
            ))}
          </pre>
        )}
      </Section>
      <Section title={t("HEAD history (reflog)")}>
        <p className="pb-2 text-[12.5px] text-muted-foreground">
          {t(
            "Every position HEAD had, including after resets and deleted branches. Create a branch at one of these to get its commits back.",
          )}
        </p>
        {reflog.map((e, i) => (
          <div key={i} className="flex h-7 items-center gap-3 px-1 text-[12.5px]">
            <span className="font-mono text-[11.5px] text-ember" data-selectable>
              {e.sha.slice(0, 7)}
            </span>
            <span className="min-w-0 flex-1 truncate">{e.message}</span>
            <span className="font-mono text-[11px] text-muted-foreground">
              {ago(e.time)} {t("ago")}
            </span>
          </div>
        ))}
      </Section>
    </div>
  );
}

// ── Destructive confirmation ────────────────────────────────────────────────

/**
 * Shows what an operation loses and asks for the phrase (and the password,
 * when one is set). The daemon checks both again; this is only the form.
 */
export function DestructiveDialog({
  name,
  op,
  preview,
  onClose,
}: {
  name: string;
  op: GitOp;
  preview: DestructivePreview;
  onClose: () => void;
}) {
  const git = useGit(name);
  const [phrase, setPhrase] = useState("");
  const [password, setPassword] = useState("");
  const ready = !preview.blocked && phrase.trim() === preview.phrase && (!preview.password || password.length > 0);

  const submit = async () => {
    try {
      const result = await git.mutateAsync({
        op,
        confirm: { phrase: phrase.trim(), password: password || null },
      });
      toast.success(result.message, {
        description: result.backup ? t("Backup: {backup}", { backup: result.backup }) : undefined,
      });
      onClose();
    } catch (error) {
      setPassword("");
      toast.error(t("Not done"), { description: message(error) });
    }
  };

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldAlert className="size-4 text-destructive" /> {t("This can't be undone from here")}
          </DialogTitle>
          <DialogDescription>{preview.summary}</DialogDescription>
        </DialogHeader>
        {preview.blocked ? (
          <p className="rounded-md border border-amber/30 bg-amber/[0.07] px-3 py-2 text-[13px] text-amber">
            {t("Blocked: {blocked}.", { blocked: preview.blocked })}
          </p>
        ) : (
          <form
            className="flex flex-col gap-4"
            onSubmit={(e) => {
              e.preventDefault();
              if (ready) submit();
            }}
          >
            {preview.lost.length > 0 && (
              <div>
                <div className="mb-1 text-[12.5px] font-medium">{t("What goes away")}</div>
                <ul
                  className="max-h-40 overflow-auto rounded-md bg-well px-3 py-2 font-mono text-[11.5px] leading-relaxed"
                  data-selectable
                >
                  {preview.lost.map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
              </div>
            )}
            <p className="text-[12.5px] text-muted-foreground">
              {preview.backup
                ? t("LocDev saves {backup} first.", { backup: preview.backup })
                : t("No backup is made for this operation.")}
            </p>
            <label className="flex flex-col gap-1.5 text-[13px]" htmlFor="phrase">
              <span>
                {tx("Type {phrase} to confirm", {
                  phrase: <span className="rounded bg-foreground/[0.07] px-1 font-mono">{preview.phrase}</span>,
                })}
              </span>
              <Input
                autoFocus
                value={phrase}
                onChange={(e) => setPhrase(e.target.value)}
                aria-label={t("Confirmation")}
                className="font-mono"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            {preview.password && (
              <label className="flex flex-col gap-1.5 text-[13px]" htmlFor="locdev_password">
                {t("LocDev password")}
                <Input
                  id="locdev_password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  aria-label={t("LocDev password")}
                  autoComplete="current-password"
                />
              </label>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="outline" type="button" onClick={onClose}>
                {t("Cancel")}
              </Button>
              <Button variant="destructive" type="submit" disabled={!ready || git.isPending}>
                {t("Confirm")}
              </Button>
            </div>
          </form>
        )}
        {preview.blocked && (
          <div className="flex justify-end">
            <Button variant="outline" onClick={onClose}>
              {t("Close")}
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
