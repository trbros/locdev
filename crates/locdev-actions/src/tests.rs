use std::collections::BTreeSet;

use locdev_core::actions::FindingLevel;
use locdev_core::model::{ProjectKind, RuntimeKind};

use super::*;
use crate::templates::TEMPLATES;

fn facts() -> Vec<Facts> {
    let scripts: BTreeSet<String> = ["lint", "test", "build", "typecheck"]
        .map(String::from)
        .into();
    vec![
        Facts {
            kind: ProjectKind::Vite,
            node: true,
            manager: Some(RuntimeKind::Pnpm),
            scripts: scripts.clone(),
            node_version_file: Some(".nvmrc".into()),
            output: "dist".into(),
            branch: "main".into(),
            dockerfile: true,
            ..Facts::default()
        },
        Facts {
            kind: ProjectKind::Laravel,
            node: true,
            manager: Some(RuntimeKind::Npm),
            scripts,
            php: true,
            php_version: Some("8.3".into()),
            laravel: true,
            output: "dist".into(),
            branch: "master".into(),
            ..Facts::default()
        },
        Facts {
            kind: ProjectKind::Php,
            php: true,
            pest: true,
            output: "dist".into(),
            branch: "main".into(),
            ..Facts::default()
        },
        Facts {
            kind: ProjectKind::Node,
            node: true,
            manager: Some(RuntimeKind::Bun),
            output: "dist".into(),
            branch: "main".into(),
            ..Facts::default()
        },
    ]
}

/// Every template, for every kind of project: clean output that reads back
/// as the same model.
#[test]
fn templates_render_validate_and_round_trip() {
    let versions = Versions::default();
    for f in facts() {
        for t in TEMPLATES {
            let workflow = (t.build)(&f, &versions);
            let text = yaml::render(&workflow);
            let report = validate::validate(&text, &versions, None);
            let serious: Vec<_> = report
                .findings
                .iter()
                .filter(|f| f.level != FindingLevel::Info)
                .collect();
            assert!(
                serious.is_empty(),
                "{} ({:?}):\n{text}\n{serious:#?}",
                t.id,
                f.kind
            );
            assert_eq!(
                report.workflow.as_ref(),
                Some(&workflow),
                "{} round trip:\n{text}",
                t.id
            );
        }
    }
}

#[test]
fn node_ci_uses_the_projects_tools() {
    let versions = Versions::default();
    let text = yaml::render(&(TEMPLATES[0].build)(&facts()[0], &versions));
    assert!(text.contains("pnpm/action-setup@v6"), "{text}");
    assert!(text.contains("node-version-file: .nvmrc"), "{text}");
    assert!(text.contains("cache: pnpm"), "{text}");
    assert!(
        text.contains("run: pnpm install --frozen-lockfile"),
        "{text}"
    );
    assert!(text.contains("run: pnpm run typecheck"), "{text}");
    assert!(text.contains("permissions:\n  contents: read"), "{text}");
    assert!(
        text.contains("\non:\n  push:\n    branches: [main]\n  pull_request:\n"),
        "{text}"
    );
}

#[test]
fn laravel_ci_prepares_env_and_builds_assets() {
    let versions = Versions::default();
    let (workflow, file) = templates::build("php-ci", &facts()[1], &versions).unwrap();
    let text = yaml::render(&workflow);
    assert_eq!(file, "ci.yml");
    assert!(text.contains("php-version: '8.3'"), "{text}");
    assert!(text.contains("branches: [master]"), "{text}");
    assert!(
        text.contains(
            "run: |\n          cp .env.example .env\n          php artisan key:generate\n"
        ),
        "{text}"
    );
    assert!(text.contains("run: npm run build"), "{text}");
    assert!(text.contains("run: php artisan test"), "{text}");
}

#[test]
fn recommendations_follow_the_stack() {
    let list = templates::list(&facts()[1]);
    let recommended: Vec<&str> = list
        .iter()
        .filter(|t| t.recommended)
        .map(|t| t.id.as_str())
        .collect();
    assert!(recommended.contains(&"php-ci"));
    assert!(
        !recommended.contains(&"node-ci"),
        "Laravel's front-end build lives in PHP CI"
    );
    assert!(list.first().unwrap().recommended, "recommended first");
}

fn findings(text: &str) -> Vec<(FindingLevel, String)> {
    validate::validate(text, &Versions::default(), None)
        .findings
        .into_iter()
        .map(|f| (f.level, f.rule))
        .collect()
}

#[test]
fn flags_script_injection() {
    let text = r#"
on: issues
permissions: {}
jobs:
  greet:
    runs-on: ubuntu-latest
    steps:
      - run: echo "New issue ${{ github.event.issue.title }}"
      - env:
          TITLE: ${{ github.event.issue.title }}
        run: echo "$TITLE"
"#;
    let report = validate::validate(text, &Versions::default(), None);
    let injection: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule == "injection")
        .collect();
    assert_eq!(
        injection.len(),
        1,
        "only the interpolated one: {:#?}",
        report.findings
    );
    assert_eq!(injection[0].level, FindingLevel::Error);
    assert_eq!(injection[0].line, Some(8));
    assert!(
        report.unsupported.is_some(),
        "issues isn't in the visual model"
    );
}

#[test]
fn flags_pull_request_target_checking_out_the_pr() {
    let text = r#"
on: pull_request_target
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          ref: ${{ github.event.pull_request.head.sha }}
      - run: npm test
"#;
    assert!(findings(text).contains(&(FindingLevel::Error, "untrusted-checkout".into())));
}

#[test]
fn flags_unpinned_outdated_and_permissions() {
    let text = r#"
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: someone/thing@main
      - uses: someone/other
      - uses: someone/tagged@v1
"#;
    let found = findings(text);
    assert!(
        found.contains(&(FindingLevel::Warning, "outdated-action".into())),
        "{found:?}"
    );
    assert!(
        found.contains(&(FindingLevel::Warning, "unpinned-action".into())),
        "{found:?}"
    );
    assert!(
        found.contains(&(FindingLevel::Error, "unpinned-action".into())),
        "{found:?}"
    );
    assert!(
        found.contains(&(FindingLevel::Info, "third-party-action".into())),
        "{found:?}"
    );
    assert!(
        found.contains(&(FindingLevel::Warning, "permissions".into())),
        "{found:?}"
    );
}

#[test]
fn flags_structure_problems() {
    let text = r#"
on:
  schedule:
    - cron: "every day"
jobs:
  a:
    runs-on: ubuntu-latest
    needs: [b]
    steps:
      - run: echo
        uses: actions/checkout@v7
  b:
    runs-on: ubuntu-latest
    needs: a
    steps: []
  c:
    needs: [ghost]
    steps:
      - run: echo
"#;
    let found: Vec<String> = findings(text).into_iter().map(|(_, rule)| rule).collect();
    for rule in ["schedule", "step", "needs", "structure"] {
        assert!(
            found.iter().any(|r| r == rule),
            "{rule} missing in {found:?}"
        );
    }
    let report = validate::validate(text, &Versions::default(), None);
    assert!(
        report.findings.iter().any(|f| f.message.contains("loop")),
        "{:#?}",
        report.findings
    );
    assert!(report.findings.iter().any(|f| f.message.contains("ghost")));
}

#[test]
fn syntax_errors_have_a_line() {
    let report = validate::validate("on: push\njobs:\n  a: [\n", &Versions::default(), None);
    assert_eq!(report.findings[0].rule, "syntax");
    assert!(report.findings[0].line.is_some());
    assert!(report.has_errors());
}

#[test]
fn reads_actionlint_json() {
    let json = r#"[{"message":"property \"foo\" is not defined","filepath":"<stdin>","line":12,"column":5,"kind":"expression","snippet":"","end_column":9}]"#;
    let found = validate::parse_actionlint(json).unwrap();
    assert_eq!(found[0].line, Some(12));
    assert_eq!(found[0].rule, "expression");
    assert_eq!(found[0].source, "actionlint");
    assert!(validate::parse_actionlint("").unwrap().is_empty());
}

#[test]
fn file_names_stay_inside_the_folder() {
    for ok in ["ci.yml", "deploy-pages.yaml", "a_b.c.yml"] {
        assert!(check_file_name(ok).is_ok(), "{ok}");
    }
    for bad in [
        "../ci.yml",
        "ci",
        ".hidden.yml",
        "a/b.yml",
        "a\\b.yml",
        "-x.yml",
        ".yml",
        "ci.yml.bak",
    ] {
        assert!(check_file_name(bad).is_err(), "{bad}");
    }
}

#[test]
fn saving_never_overwrites_by_surprise() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path();
    assert_eq!(
        save(project, "ci.yml", "one\n", None).unwrap(),
        ".github/workflows/ci.yml"
    );
    let error = save(project, "ci.yml", "two\n", None).unwrap_err();
    assert!(error.0.contains("already exists"), "{error}");

    let plan = plan(project, "ci.yml", "two\n", Validation::default()).unwrap();
    assert!(plan.exists);
    assert!(
        plan.diff.contains("-one") && plan.diff.contains("+two"),
        "{}",
        plan.diff
    );
    let stale = save(project, "ci.yml", "two\n", Some(&hash(b"something else")));
    assert!(stale.unwrap_err().0.contains("changed since"));
    save(project, "ci.yml", "two\n", plan.current_hash.as_deref()).unwrap();
    assert_eq!(
        std::fs::read_to_string(project.join(".github/workflows/ci.yml")).unwrap(),
        "two\n"
    );

    assert_eq!(existing(project)[0].file, "ci.yml");
    let new = super::plan(project, "new.yml", "x: 1\n", Validation::default()).unwrap();
    assert!(!new.exists && new.diff.contains("+x: 1"));
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_out_of_the_project() {
    let outside = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".github")).unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join(".github/workflows")).unwrap();
    let error = save(dir.path(), "ci.yml", "x\n", None).unwrap_err();
    assert!(error.0.contains("outside the project"), "{error}");
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".github/workflows")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("x.yml"),
        dir.path().join(".github/workflows/ci.yml"),
    )
    .unwrap();
    assert!(
        save(dir.path(), "ci.yml", "x\n", None)
            .unwrap_err()
            .0
            .contains("symlink")
    );
}

/// LocDev's own templates follow the practices it suggests.
#[test]
fn templates_follow_the_suggested_practices() {
    let versions = Versions::default();
    for f in facts() {
        for t in TEMPLATES {
            let text = yaml::render(&(t.build)(&f, &versions));
            let report = validate::validate(&text, &versions, None);
            let practices: Vec<_> = report
                .findings
                .iter()
                .filter(|x| x.rule.starts_with("practice-") || x.rule == "missing-setup")
                .collect();
            assert!(practices.is_empty(), "{}: {practices:#?}", t.id);
        }
    }
}

#[test]
fn suggests_setup_timeouts_and_caches() {
    let text = "on: pull_request
jobs:
  t:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions/setup-node@v5
      - run: pnpm install && pnpm test
";
    let report = validate::validate(text, &Versions::default(), None);
    let rules: Vec<&str> = report.findings.iter().map(|f| f.rule.as_str()).collect();
    for want in [
        "practice-concurrency",
        "practice-timeout",
        "practice-cache",
        "practice-credentials",
        "missing-setup",
    ] {
        assert!(rules.contains(&want), "{want} in {rules:?}");
    }
    let setup = report
        .findings
        .iter()
        .find(|f| f.rule == "missing-setup")
        .unwrap();
    assert!(setup.fix.as_deref().unwrap().contains("pnpm/action-setup"));
    assert_eq!(setup.level, FindingLevel::Warning);
}
