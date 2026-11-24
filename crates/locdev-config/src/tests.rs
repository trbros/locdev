use locdev_core::config::OptionState;
use serde_json::json;

use crate::{js, json as jsonc};

fn set(state: OptionState) -> serde_json::Value {
    match state {
        OptionState::Set { value } => value,
        other => panic!("expected a value, got {other:?}"),
    }
}

const VITE: &str = r#"import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173, // keep in sync with LocDev
    host: process.env.HOST,
  },
})
"#;

#[test]
fn js_reads_literals_and_code_without_running_it() {
    let read = js::read(
        VITE,
        "vite.config.ts",
        &["server.port", "server.host", "build.outDir"],
    )
    .unwrap();
    assert_eq!(set(read.values[0].1.clone()), json!(5173));
    assert_eq!(
        read.values[1].1,
        OptionState::Code {
            code: "process.env.HOST".into()
        }
    );
    assert_eq!(read.values[2].1, OptionState::Missing);
    assert_eq!(read.notes, vec!["plugins: react()".to_string()]);
}

#[test]
fn js_replaces_only_the_literal_and_keeps_comments() {
    let out = js::set(VITE, "vite.config.ts", "server.port", &json!(5180)).unwrap();
    assert_eq!(out, VITE.replace("port: 5173,", "port: 5180,"));
    assert!(
        js::set(VITE, "vite.config.ts", "server.host", &json!("0.0.0.0"))
            .unwrap_err()
            .contains("computed in code")
    );
}

#[test]
fn js_inserts_missing_keys_and_objects_in_the_file_style() {
    let out = js::set(VITE, "vite.config.ts", "server.strictPort", &json!(true)).unwrap();
    assert!(
        out.contains("    host: process.env.HOST,\n    strictPort: true,\n  },"),
        "{out}"
    );
    let out = js::set(VITE, "vite.config.ts", "build.sourcemap", &json!("hidden")).unwrap();
    assert!(
        out.contains("  },\n  build: {\n    sourcemap: 'hidden',\n  },\n})"),
        "{out}"
    );
    let read = js::read(&out, "vite.config.ts", &["build.sourcemap"]).unwrap();
    assert_eq!(set(read.values[0].1.clone()), json!("hidden"));
}

#[test]
fn js_finds_the_config_through_constants_arrows_and_module_exports() {
    let next = "/** @type {import('next').NextConfig} */\nconst nextConfig = {\n  reactStrictMode: true,\n};\n\nexport default nextConfig;\n";
    let out = js::set(next, "next.config.mjs", "poweredByHeader", &json!(false)).unwrap();
    assert_eq!(
        out,
        "/** @type {import('next').NextConfig} */\nconst nextConfig = {\n  reactStrictMode: true,\n  poweredByHeader: false,\n};\n\nexport default nextConfig;\n"
    );
    let cjs = "module.exports = { images: { unoptimized: false } }\n";
    assert_eq!(
        js::set(cjs, "next.config.js", "images.unoptimized", &json!(true)).unwrap(),
        "module.exports = { images: { unoptimized: true } }\n"
    );
    let arrow = "export default defineConfig(({ mode }) => {\n  const port = 3000\n  return {\n    server: { port },\n  }\n})\n";
    let read = js::read(arrow, "vite.config.js", &["server.port"]).unwrap();
    assert_eq!(
        read.values[0].1,
        OptionState::Code {
            code: "port".into()
        }
    );
    let ts = "import type { NextConfig } from \"next\";\n\nconst config: NextConfig = {} satisfies NextConfig;\nexport default config;\n";
    let out = js::set(ts, "next.config.ts", "typedRoutes", &json!(true)).unwrap();
    assert!(
        out.contains("const config: NextConfig = {\n  typedRoutes: true,\n} satisfies NextConfig;"),
        "{out}"
    );
}

#[test]
fn js_refuses_what_it_cant_read() {
    assert!(js::read("export default makeConfig()\n", "vite.config.js", &[]).is_err());
    assert!(
        js::check("export default {", "vite.config.js")
            .unwrap()
            .contains("line")
    );
    assert!(js::set("export default { a: 1 }", "x.config.js", "a", &json!([1])).is_err());
}

const TSCONFIG: &str = r#"{
  // Shared by the app and the tests
  "compilerOptions": {
    "target": "ES2022",
    "strict": false, /* TODO */
  },
  "include": ["src"],
}
"#;

#[test]
fn jsonc_edits_keep_comments_and_trailing_commas() {
    assert_eq!(
        set(jsonc::read(TSCONFIG, false, "compilerOptions.strict").unwrap()),
        json!(false)
    );
    let out = jsonc::set(TSCONFIG, false, "compilerOptions.strict", &json!(true)).unwrap();
    assert_eq!(
        out,
        TSCONFIG.replace("\"strict\": false", "\"strict\": true")
    );
    let out = jsonc::set(TSCONFIG, false, "compilerOptions.noEmit", &json!(true)).unwrap();
    assert!(
        out.contains("// Shared by the app")
            && out.contains("/* TODO */")
            && out.contains("\"noEmit\": true"),
        "{out}"
    );
    assert_eq!(
        set(jsonc::read(&out, false, "compilerOptions.noEmit").unwrap()),
        json!(true)
    );
}

#[test]
fn strict_json_stays_strict() {
    let package = "{\n  \"name\": \"app\",\n  \"private\": true\n}\n";
    assert!(jsonc::check(TSCONFIG, true).is_some());
    let out = jsonc::set(package, true, "engines.node", &json!(">=20")).unwrap();
    assert!(jsonc::check(&out, true).is_none(), "{out}");
    assert_eq!(
        set(jsonc::read(&out, true, "engines.node").unwrap()),
        json!(">=20")
    );
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in files {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

fn installed(name: &str, version: &str) -> (String, String) {
    (
        format!("node_modules/{name}/package.json"),
        format!("{{\"version\":\"{version}\"}}"),
    )
}

fn with_packages(files: &[(&str, &str)], packages: &[(&str, &str)]) -> tempfile::TempDir {
    let deps: Vec<String> = packages
        .iter()
        .map(|(n, v)| format!("\"{n}\": \"^{v}\""))
        .collect();
    let manifest = format!(
        "{{\n  \"name\": \"app\",\n  \"devDependencies\": {{ {} }}\n}}\n",
        deps.join(", ")
    );
    let mut all: Vec<(String, String)> = files
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    all.push(("package.json".into(), manifest));
    all.extend(packages.iter().map(|(n, v)| installed(n, v)));
    let refs: Vec<(&str, &str)> = all.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    project(&refs)
}

#[test]
fn lists_only_existing_files_and_tailwind_css() {
    let dir = with_packages(
        &[
            ("vite.config.ts", VITE),
            ("tsconfig.json", TSCONFIG),
            ("tsconfig.node.json", "{}"),
            ("src/index.css", "@import \"tailwindcss\";\n"),
            ("node_modules/x/y.css", "@import \"tailwindcss\";"),
        ],
        &[("vite", "7.1.0"), ("tailwindcss", "4.1.0")],
    );
    let files = crate::list(dir.path());
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "vite.config.ts",
            "tsconfig.json",
            "tsconfig.node.json",
            "package.json",
            "src/index.css"
        ]
    );
    let vite = &files[0];
    assert_eq!(
        (vite.kind.as_str(), vite.version.as_deref(), vite.visual),
        ("vite", Some("7.1.0"), true)
    );
    assert!(crate::open(dir.path(), "../etc/passwd").is_err());
    assert!(crate::open(dir.path(), "node_modules/x/y.css").is_err());
}

#[test]
fn options_follow_the_installed_version() {
    let next = "const nextConfig = {\n  reactStrictMode: true,\n};\n\nexport default nextConfig;\n";
    let dir = with_packages(&[("next.config.mjs", next)], &[("next", "15.2.4")]);
    let view = crate::open(dir.path(), "next.config.mjs").unwrap();
    let typed = view
        .options
        .iter()
        .find(|o| o.key == "typedRoutes")
        .unwrap();
    assert!(!typed.editable);
    assert!(
        typed
            .blocked
            .as_deref()
            .unwrap()
            .contains("needs next 15.5.0 or newer (this project has 15.2.4)")
    );
    let strict = view
        .options
        .iter()
        .find(|o| o.key == "reactStrictMode")
        .unwrap();
    assert_eq!(strict.state, OptionState::Set { value: json!(true) });
    assert!(strict.editable);
    let refused = crate::apply(
        dir.path(),
        "next.config.mjs",
        next,
        &locdev_core::config::ConfigChange::Set {
            key: "typedRoutes".into(),
            value: json!(true),
        },
    );
    assert!(refused.unwrap_err().contains("needs next 15.5.0"));
    let wrong = crate::apply(
        dir.path(),
        "next.config.mjs",
        next,
        &locdev_core::config::ConfigChange::Set {
            key: "output".into(),
            value: json!("server"),
        },
    );
    assert!(wrong.is_err());
}

#[test]
fn saves_with_history_revert_and_stale_hash() {
    use locdev_core::config::ConfigChange;
    let dir = with_packages(&[("vite.config.ts", VITE)], &[("vite", "7.1.0")]);
    let state = tempfile::tempdir().unwrap();
    let history = crate::history::History::new(state.path(), "app", "vite.config.ts");
    let view = crate::open(dir.path(), "vite.config.ts").unwrap();
    let change = ConfigChange::Set {
        key: "server.port".into(),
        value: json!(5180),
    };
    let preview = crate::preview(dir.path(), "vite.config.ts", &change).unwrap();
    assert!(
        preview
            .diff
            .contains("-    port: 5173, // keep in sync with LocDev")
    );
    assert!(
        preview
            .diff
            .contains("+    port: 5180, // keep in sync with LocDev")
    );
    assert!(preview.removed_comments.is_empty() && preview.syntax_error.is_none());

    let hash = crate::save(dir.path(), "vite.config.ts", &change, &view.hash, &history).unwrap();
    assert!(
        std::fs::read_to_string(dir.path().join("vite.config.ts"))
            .unwrap()
            .contains("port: 5180")
    );
    // The same base hash again: the file changed since.
    assert!(
        crate::save(dir.path(), "vite.config.ts", &change, &view.hash, &history)
            .unwrap_err()
            .contains("changed since")
    );

    let broken = ConfigChange::Text {
        content: "export default {".into(),
    };
    assert!(
        crate::save(dir.path(), "vite.config.ts", &broken, &hash, &history)
            .unwrap_err()
            .starts_with("not saved")
    );

    let entries = history.list();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].reason, "set server.port = 5180");
    crate::revert(
        dir.path(),
        "vite.config.ts",
        &entries[0].id,
        &hash,
        &history,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vite.config.ts")).unwrap(),
        VITE
    );
    assert_eq!(
        history.list()[0].reason,
        format!("revert to {}", entries[0].id)
    );
    assert!(history.load("../../x").is_err());
}

#[test]
fn text_preview_lists_removed_comments() {
    use locdev_core::config::ConfigChange;
    let dir = with_packages(&[("tsconfig.json", TSCONFIG)], &[("typescript", "5.9.2")]);
    let content = TSCONFIG.replace("  // Shared by the app and the tests\n", "");
    let preview =
        crate::preview(dir.path(), "tsconfig.json", &ConfigChange::Text { content }).unwrap();
    assert_eq!(
        preview.removed_comments,
        vec!["  // Shared by the app and the tests".to_string()]
    );
}

fn rules(dir: &tempfile::TempDir) -> Vec<String> {
    crate::rules::check(dir.path())
        .into_iter()
        .map(|f| f.rule)
        .collect()
}

#[test]
fn rules_depend_on_versions_and_values() {
    let next = "export default {\n  images: { domains: ['cdn.example.com'] },\n  experimental: { turbo: {} },\n  typescript: { ignoreBuildErrors: true },\n}\n";
    let dir = with_packages(
        &[("next.config.mjs", next), ("tsconfig.json", "{}")],
        &[("next", "15.5.0")],
    );
    let found = rules(&dir);
    for rule in [
        "next.powered-by",
        "next.images-domains",
        "next.experimental-turbo",
        "next.ignore-build-errors",
        "next.typed-routes-available",
    ] {
        assert!(
            found.contains(&rule.to_string()),
            "{rule} missing in {found:?}"
        );
    }
    // Older Next: version-bound rules stay quiet.
    let dir = with_packages(&[("next.config.mjs", next)], &[("next", "13.5.0")]);
    let found = rules(&dir);
    assert!(
        !found.contains(&"next.images-domains".to_string())
            && !found.contains(&"next.experimental-turbo".to_string()),
        "{found:?}"
    );

    let vite = "export default { server: { https: true, host: true } }\n";
    let found = rules(&with_packages(
        &[("vite.config.js", vite)],
        &[("vite", "6.0.0")],
    ));
    assert!(
        found.contains(&"vite.https-boolean".into()) && found.contains(&"vite.host-exposed".into()),
        "{found:?}"
    );
    let found = rules(&with_packages(
        &[("vite.config.js", vite)],
        &[("vite", "4.5.0")],
    ));
    assert!(!found.contains(&"vite.https-boolean".into()), "{found:?}");
}

#[test]
fn tailwind_eslint_typescript_and_composer_rules() {
    let dir = with_packages(
        &[
            ("tailwind.config.js", "export default {}\n"),
            ("src/app.css", "@tailwind base;\n@tailwind utilities;\n"),
            (
                "postcss.config.js",
                "export default { plugins: { tailwindcss: {}, autoprefixer: {} } }\n",
            ),
            (".eslintrc.json", "{}"),
            ("vite.config.ts", "export default {}\n"),
            (
                "tsconfig.json",
                "{ \"compilerOptions\": { \"moduleResolution\": \"node\", \"importsNotUsedAsValues\": \"remove\" } }",
            ),
        ],
        &[
            ("tailwindcss", "4.1.0"),
            ("eslint", "9.10.0"),
            ("typescript", "5.6.0"),
            ("vite", "7.0.0"),
        ],
    );
    let found = rules(&dir);
    for rule in [
        "tailwind.v4-js-config",
        "tailwind.v4-directives",
        "tailwind.v4-postcss-plugin",
        "eslint.eslintrc-ignored",
        "typescript.bundler-resolution",
        "typescript.import-flags",
    ] {
        assert!(
            found.contains(&rule.to_string()),
            "{rule} missing in {found:?}"
        );
    }
    let composer = project(&[("composer.json", "{\n  \"minimum-stability\": \"dev\"\n}\n")]);
    assert_eq!(rules(&composer), vec!["composer.unstable".to_string()]);
    let fine = project(&[(
        "composer.json",
        "{\n  \"minimum-stability\": \"dev\",\n  \"prefer-stable\": true\n}\n",
    )]);
    assert!(rules(&fine).is_empty());
}

#[test]
fn findings_carry_a_source_and_what_they_apply_to() {
    let dir = with_packages(
        &[(
            "vite.config.js",
            "export default { server: { host: '0.0.0.0' } }\n",
        )],
        &[("vite", "7.0.0")],
    );
    let finding = crate::rules::check(dir.path())
        .into_iter()
        .find(|f| f.rule == "vite.host-exposed")
        .unwrap();
    assert!(finding.source.starts_with("https://vite.dev/"));
    assert_eq!(
        finding.fix,
        Some(("server.host".to_string(), json!("localhost")))
    );
}
