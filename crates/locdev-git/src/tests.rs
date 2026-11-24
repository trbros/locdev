use super::*;

/// A fresh repository with an identity of its own (no global config needed).
fn repo() -> (tempfile::TempDir, Repo) {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        &["init", "--quiet", "--initial-branch=main"][..],
        &["config", "user.name", "Test"],
        &["config", "user.email", "test@example.com"],
        &["config", "commit.gpgsign", "false"],
        // A global core.autocrlf (Git for Windows' default) would turn the
        // line endings into CRLF on checkout.
        &["config", "core.autocrlf", "false"],
    ] {
        assert!(raw(dir.path(), args, None, None).unwrap().success);
    }
    let repo = Repo::open(dir.path()).unwrap();
    (dir, repo)
}

fn write(repo: &Repo, file: &str, body: &str) {
    let path = repo.root().join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn commit_all(repo: &Repo, message: &str) {
    repo.run(&["add", "-A"]).unwrap();
    repo.commit(message).unwrap();
}

#[test]
fn not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Repo::open(dir.path()).is_err());
}

#[test]
fn status_tracks_every_kind_of_change() {
    let (_dir, repo) = repo();
    let status = repo.status().unwrap();
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.head.is_none(), "no commits yet");

    write(&repo, "a.txt", "one\ntwo\n");
    write(&repo, "gone.txt", "x\n");
    write(&repo, "old name.txt", "keep\n");
    commit_all(&repo, "first");

    write(&repo, "a.txt", "one\nTWO\nthree\n");
    std::fs::remove_file(repo.root().join("gone.txt")).unwrap();
    repo.run(&["mv", "old name.txt", "new name.txt"]).unwrap();
    write(&repo, "src/new file.rs", "fn main() {}\n");
    repo.stage(&["a.txt".into()]).unwrap();

    let status = repo.status().unwrap();
    let by_path = |p: &str| status.changes.iter().find(|c| c.path == p).unwrap();
    let a = by_path("a.txt");
    assert_eq!((a.staged, a.unstaged), (Some(ChangeKind::Modified), None));
    assert_eq!((a.added, a.removed), (Some(2), Some(1)));
    assert_eq!(by_path("gone.txt").unstaged, Some(ChangeKind::Deleted));
    let renamed = by_path("new name.txt");
    assert_eq!(renamed.staged, Some(ChangeKind::Renamed));
    assert_eq!(renamed.orig_path.as_deref(), Some("old name.txt"));
    assert!(
        by_path("src/new file.rs").untracked,
        "paths with spaces survive -z parsing"
    );
}

#[test]
fn stage_unstage_discard_and_commit() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");

    write(&repo, "a.txt", "changed\n");
    write(&repo, "scratch.txt", "temp\n");
    repo.stage(&["a.txt".into()]).unwrap();
    repo.unstage(&["a.txt".into()]).unwrap();
    assert_eq!(
        repo.status()
            .unwrap()
            .changes
            .iter()
            .find(|c| c.path == "a.txt")
            .unwrap()
            .staged,
        None
    );

    repo.discard(&["a.txt".into(), "scratch.txt".into()])
        .unwrap();
    assert!(repo.status().unwrap().changes.is_empty());
    assert_eq!(
        std::fs::read_to_string(repo.root().join("a.txt")).unwrap(),
        "one\n"
    );
    assert!(!repo.root().join("scratch.txt").exists());

    write(&repo, "b.txt", "b\n");
    repo.stage(&["b.txt".into()]).unwrap();
    // The message goes through stdin: quotes and dashes are just text.
    repo.commit("--amend \"quoted\" $(rm -rf /)\n\nbody")
        .unwrap();
    let log = repo.log(None, 0, 10).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].subject, "--amend \"quoted\" $(rm -rf /)");
    assert!(repo.commit("   ").is_err());
}

#[test]
fn rejects_paths_and_names_that_could_be_options() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "a\n");
    commit_all(&repo, "first");
    for bad in ["../outside", "/etc/passwd", ""] {
        assert!(repo.stage(&[bad.into()]).is_err(), "{bad}");
    }
    for bad in ["-D", "--force", "has space", "a..b", ""] {
        assert!(repo.create_branch(bad, None, false).is_err(), "{bad}");
    }
    assert!(repo.check_rev("--all").is_err());
    assert!(
        repo.remote_add("origin", "ext::sh -c touch% /tmp/pwned")
            .is_err()
    );
    assert!(repo.remote_add("origin", "--upload-pack=x").is_err());
    // A file literally named like an option is still just a file.
    write(&repo, "-rf", "x\n");
    repo.stage(&["-rf".into()]).unwrap();
}

#[test]
fn diffs_tracked_staged_and_untracked() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");
    write(&repo, "a.txt", "two\n");
    write(&repo, "new.txt", "fresh\n");
    let (diff, truncated) = repo.diff("a.txt", false).unwrap();
    assert!(diff.contains("-one") && diff.contains("+two") && !truncated);
    assert!(
        repo.diff("a.txt", true).unwrap().0.is_empty(),
        "nothing staged"
    );
    assert!(repo.diff("new.txt", false).unwrap().0.contains("+fresh"));
}

#[test]
fn big_diffs_neither_hang_nor_overflow() {
    let (_dir, repo) = repo();
    write(&repo, "big.txt", "");
    commit_all(&repo, "first");
    // Well past a pipe buffer and past the diff limit.
    write(
        &repo,
        "big.txt",
        &"a line of text that repeats\n".repeat(40_000),
    );
    let (diff, truncated) = repo.diff("big.txt", false).unwrap();
    assert!(truncated);
    assert!(diff.len() <= MAX_DIFF);
}

#[test]
fn history_show_and_reflog() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");
    write(&repo, "a.txt", "one\ntwo\n");
    commit_all(&repo, "second");

    let log = repo.log(None, 0, 1).unwrap();
    assert_eq!(log[0].subject, "second");
    assert!(log[0].refs.iter().any(|r| r.contains("main")));
    assert_eq!(
        repo.log(None, 1, 10).unwrap()[0].subject,
        "first",
        "paginated"
    );

    let detail = repo.show(&log[0].sha).unwrap();
    assert_eq!(
        detail.files,
        vec![FileStat {
            path: "a.txt".into(),
            added: Some(1),
            removed: Some(0)
        }]
    );
    assert!(detail.diff.contains("+two"));
    assert!(
        repo.reflog(10)
            .unwrap()
            .iter()
            .any(|e| e.message.contains("second"))
    );
}

#[test]
fn branches_switch_merge_and_delete() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "base\n");
    commit_all(&repo, "base");

    repo.create_branch("feature/x", None, true).unwrap();
    write(&repo, "b.txt", "feature\n");
    commit_all(&repo, "feature work");
    repo.switch("main").unwrap();

    let branches = repo.branches().unwrap();
    let feature = branches.iter().find(|b| b.name == "feature/x").unwrap();
    assert!(!feature.merged, "not merged yet");
    // A plain delete refuses an unmerged branch.
    assert!(repo.delete_branch("feature/x", false).is_err());

    repo.merge("feature/x").unwrap();
    assert!(
        repo.branches()
            .unwrap()
            .iter()
            .find(|b| b.name == "feature/x")
            .unwrap()
            .merged
    );
    repo.delete_branch("feature/x", false).unwrap();
}

#[test]
fn merge_conflicts_stay_for_the_user() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "base\n");
    commit_all(&repo, "base");
    repo.create_branch("other", None, true).unwrap();
    write(&repo, "a.txt", "theirs\n");
    commit_all(&repo, "theirs");
    repo.switch("main").unwrap();
    write(&repo, "a.txt", "ours\n");
    commit_all(&repo, "ours");

    let error = repo.merge("other").unwrap_err();
    assert!(error.0.contains("conflicts"), "{error}");
    let status = repo.status().unwrap();
    assert_eq!(status.state, RepoState::Merging);
    assert!(
        status
            .changes
            .iter()
            .any(|c| c.path == "a.txt" && c.conflicted)
    );
    repo.abort_merge().unwrap();
    assert_eq!(repo.status().unwrap().state, RepoState::Clean);
}

#[test]
fn stashes_round_trip() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");
    write(&repo, "a.txt", "wip\n");
    write(&repo, "new.txt", "untracked\n");
    repo.stash_push(Some("my wip"), true).unwrap();
    assert!(repo.status().unwrap().changes.is_empty());
    let stashes = repo.stashes().unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("my wip"));
    repo.stash_apply(0, false).unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.root().join("a.txt")).unwrap(),
        "wip\n"
    );
    repo.stash_drop(0).unwrap();
    assert!(repo.stashes().unwrap().is_empty());
    assert!(repo.stash_drop(0).is_err());
}

#[test]
fn revert_restore_and_reset_are_different_things() {
    let (_dir, repo) = repo();
    write(&repo, "a.txt", "v1\n");
    commit_all(&repo, "v1");
    let v1 = repo.head().unwrap();
    write(&repo, "a.txt", "v2\n");
    commit_all(&repo, "v2");
    let v2 = repo.head().unwrap();

    // revert: a new commit that undoes v2; history keeps both.
    repo.revert(&v2).unwrap();
    assert_eq!(repo.log(None, 0, 10).unwrap().len(), 3);
    assert_eq!(
        std::fs::read_to_string(repo.root().join("a.txt")).unwrap(),
        "v1\n"
    );

    // restore: only the working tree changes; HEAD stays.
    let head = repo.head().unwrap();
    repo.restore_files(&v2, &["a.txt".into()]).unwrap();
    assert_eq!(repo.head().unwrap(), head);
    assert_eq!(
        std::fs::read_to_string(repo.root().join("a.txt")).unwrap(),
        "v2\n"
    );

    // reset --hard: the branch moves; a backup keeps what would be lost.
    let backup = repo
        .backup("test")
        .unwrap()
        .expect("there are changes to save");
    assert!(backup.starts_with("refs/locdev/backups/test"));
    assert_eq!(repo.commits_between(&v1).unwrap().len(), 2);
    repo.reset(&v1, ResetMode::Hard).unwrap();
    assert_eq!(repo.head().unwrap(), v1);
    assert!(repo.status().unwrap().changes.is_empty());
    // The saved state is still reachable.
    assert!(repo.check_rev("refs/locdev/backups/test").is_ok());
}

#[test]
fn remotes_and_github_detection() {
    let (_dir, repo) = repo();
    repo.remote_add("origin", "git@github.com:trbros/locdev.git")
        .unwrap();
    repo.remote_add("mirror", "https://gitlab.com/x/y.git")
        .unwrap();
    let remotes = repo.remotes().unwrap();
    assert_eq!(remotes.len(), 2);
    assert_eq!(
        repo.status().unwrap().github.unwrap().slug(),
        "trbros/locdev"
    );
    repo.remote_set_url("origin", "https://github.com/bnacci/other")
        .unwrap();
    repo.remote_remove("mirror").unwrap();
    assert_eq!(repo.remotes().unwrap().len(), 1);

    for (url, slug) in [
        ("https://github.com/a/b.git", Some("a/b")),
        ("ssh://git@github.com/a/b", Some("a/b")),
        ("https://gitlab.com/a/b", None),
        ("https://github.com/a", None),
    ] {
        assert_eq!(github_repo(url).map(|r| r.slug()).as_deref(), slug, "{url}");
    }
}

#[test]
fn push_and_pull_against_a_local_remote() {
    let (_dir, repo) = repo();
    let bare = tempfile::tempdir().unwrap();
    assert!(
        raw(
            bare.path(),
            &["init", "--bare", "--quiet", "--initial-branch=main"],
            None,
            None
        )
        .unwrap()
        .success
    );
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");
    repo.remote_add("origin", &format!("https://example.invalid/{}", "x"))
        .unwrap();
    // A file URL is refused for remotes added through LocDev; use git itself here.
    repo.run(&["remote", "set-url", "origin", bare.path().to_str().unwrap()])
        .unwrap();
    repo.run(&["push", "--set-upstream", "origin", "main"])
        .unwrap();
    write(&repo, "a.txt", "two\n");
    commit_all(&repo, "second");
    assert_eq!(repo.status().unwrap().ahead, 1);
    repo.push(false).unwrap();
    repo.fetch(Some("origin")).unwrap();
    assert_eq!(repo.status().unwrap().ahead, 0);
    repo.pull().unwrap();
    assert!(repo.fetch(Some("nope")).is_err());
}

#[test]
fn worktrees_add_list_and_remove() {
    let (dir, repo) = repo();
    write(&repo, "a.txt", "one\n");
    commit_all(&repo, "first");
    let outside = tempfile::tempdir().unwrap();
    let tree = outside.path().join("shop-feature");

    repo.worktree_add(&tree, "feature", true, None).unwrap();
    assert!(tree.join("a.txt").is_file());
    assert!(
        repo.worktree_add(&tree, "other", true, None).is_err(),
        "folder exists"
    );
    assert!(
        repo.worktree_add(&outside.path().join("x"), "nope", false, None)
            .is_err()
    );

    let trees = repo.worktrees().unwrap();
    assert_eq!(trees.len(), 2);
    assert!(trees[0].main && same_path(&trees[0].path, dir.path()));
    assert_eq!(trees[1].branch.as_deref(), Some("feature"));
    assert!(same_path(&trees[1].path, &tree));

    assert!(
        repo.worktree_remove(dir.path(), false).is_err(),
        "main tree"
    );
    std::fs::write(tree.join("b.txt"), "new").unwrap();
    assert!(
        repo.worktree_remove(&tree, false).is_err(),
        "untracked file"
    );
    repo.worktree_remove(&tree, true).unwrap();
    assert!(!tree.exists());
    assert_eq!(repo.worktrees().unwrap().len(), 1);
}

#[test]
fn worktree_porcelain() {
    let out = "worktree /src/shop\nHEAD abc\nbranch refs/heads/main\n\nworktree /src/shop-x\nHEAD def\ndetached\nlocked\nprunable gitdir file points to non-existent location\n\n";
    let trees = parse_worktrees(out);
    assert_eq!(trees.len(), 2);
    assert_eq!(trees[0].branch.as_deref(), Some("main"));
    assert!(trees[0].main && !trees[1].main);
    assert!(trees[1].branch.is_none() && trees[1].locked && trees[1].prunable);
}
