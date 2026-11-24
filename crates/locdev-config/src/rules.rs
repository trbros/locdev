//! Config analysis: rules that can be checked against the files and the
//! installed versions, each with the versions it's about and the official
//! source. Findings never change files; a `fix` is only offered.
//!
//! A new rule: a function here, listed in `check`, plus a test in `tests.rs`.

use std::path::Path;

use locdev_core::config::{ConfigFinding, FindingKind, OptionState};
use locdev_core::semver::Version;
use serde_json::{Value, json};

use crate::files::{self, Found, Versions};
use crate::{js, json as jsonc};

struct Project<'a> {
    root: &'a Path,
    files: Vec<Found>,
    versions: Versions,
}

impl Project<'_> {
    fn file(&self, kind: &str) -> Option<&Found> {
        self.files.iter().find(|f| f.kind.id == kind)
    }

    fn text(&self, found: &Found) -> Option<String> {
        std::fs::read_to_string(self.root.join(&found.rel)).ok()
    }

    /// What `key` is in the first file of `kind`.
    fn value(&self, kind: &str, key: &str) -> Option<(String, OptionState)> {
        let found = self.file(kind)?;
        let text = self.text(found)?;
        let state = match files::format(found.kind, &found.rel, &text) {
            locdev_core::config::ConfigFormat::Js => {
                js::read(&text, &found.rel, &[key]).ok()?.values.pop()?.1
            }
            locdev_core::config::ConfigFormat::Json => jsonc::read(&text, true, key).ok()?,
            locdev_core::config::ConfigFormat::Jsonc => jsonc::read(&text, false, key).ok()?,
            _ => return None,
        };
        Some((found.rel.clone(), state))
    }

    /// `(file, value)` when `key` is set to a literal.
    fn literal(&self, kind: &str, key: &str) -> Option<(String, Value)> {
        match self.value(kind, key)? {
            (file, OptionState::Set { value }) => Some((file, value)),
            _ => None,
        }
    }

    /// The key is present in any form (literal or code).
    fn present(&self, kind: &str, key: &str) -> Option<String> {
        match self.value(kind, key)? {
            (_, OptionState::Missing) => None,
            (file, _) => Some(file),
        }
    }

    fn at_least(&self, package: &str, version: &str) -> Option<&Version> {
        let installed = self.versions.get(package)?;
        (installed >= &Version::parse(version)?).then_some(installed)
    }
}

struct Rule {
    rule: &'static str,
    kind: FindingKind,
    title: &'static str,
    detail: &'static str,
    source: &'static str,
}

fn finding(
    r: Rule,
    file: String,
    key: Option<&str>,
    applies: String,
    fix: Option<(&str, Value)>,
) -> ConfigFinding {
    ConfigFinding {
        rule: r.rule.into(),
        kind: r.kind,
        file,
        key: key.map(str::to_string),
        title: r.title.into(),
        detail: r.detail.into(),
        applies,
        source: r.source.into(),
        fix: fix.map(|(k, v)| (k.to_string(), v)),
    }
}

pub fn check(root: &Path) -> Vec<ConfigFinding> {
    let project = Project {
        root,
        files: files::discover(root),
        versions: Versions::read(root),
    };
    let rules: &[fn(&Project) -> Vec<ConfigFinding>] = &[
        vite_https_boolean,
        vite_host_exposed,
        next_powered_by,
        next_images_domains,
        next_experimental_turbo,
        next_typed_routes_moved,
        next_typed_routes_available,
        next_cache_components,
        next_ignore_build_errors,
        tailwind4_js_config,
        tailwind4_directives,
        tailwind4_postcss_plugin,
        eslintrc_with_eslint9,
        ts_import_flags,
        ts_bundler_resolution,
        composer_unstable,
        package_manager_mismatch,
    ];
    rules.iter().flat_map(|rule| rule(&project)).collect()
}

fn vite_https_boolean(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("vite", "5.0.0") else {
        return vec![];
    };
    match p.literal("vite", "server.https") {
        Some((file, Value::Bool(true))) => vec![finding(
            Rule {
                rule: "vite.https-boolean",
                kind: FindingKind::Incompatible,
                title: "`server.https: true` was removed in Vite 5",
                detail: "Vite 5 dropped the self-signed certificate behind `https: true`; it needs certificate options or @vitejs/plugin-basic-ssl. Behind LocDev, HTTPS comes from the proxy, so the dev server can stay on HTTP.",
                source: "https://v5.vite.dev/guide/migration.html",
            },
            file,
            Some("server.https"),
            format!("vite >= 5 (installed {v})"),
            None,
        )],
        _ => vec![],
    }
}

fn vite_host_exposed(p: &Project) -> Vec<ConfigFinding> {
    let exposed = |v: &Value| {
        v == &Value::Bool(true) || v.as_str().is_some_and(|s| s == "0.0.0.0" || s == "::")
    };
    match p.literal("vite", "server.host") {
        Some((file, value)) if exposed(&value) => vec![finding(
            Rule {
                rule: "vite.host-exposed",
                kind: FindingKind::Security,
                title: "The dev server listens on every network interface",
                detail: "`server.host` set to `true`/`0.0.0.0` makes the dev server reachable from other machines on the network. LocDev reaches it on 127.0.0.1, so exposing it isn't needed.",
                source: "https://vite.dev/config/server-options#server-host",
            },
            file,
            Some("server.host"),
            "vite (any version)".into(),
            Some(("server.host", json!("localhost"))),
        )],
        _ => vec![],
    }
}

fn next_powered_by(p: &Project) -> Vec<ConfigFinding> {
    let Some(found) = p.file("next") else {
        return vec![];
    };
    if p.versions.get("next").is_none() {
        return vec![];
    }
    match p.value("next", "poweredByHeader") {
        Some((
            _,
            OptionState::Set {
                value: Value::Bool(false),
            },
        ))
        | Some((_, OptionState::Code { .. }))
        | None => vec![],
        Some(_) => vec![finding(
            Rule {
                rule: "next.powered-by",
                kind: FindingKind::Security,
                title: "Responses announce `X-Powered-By: Next.js`",
                detail: "Next.js adds the header unless `poweredByHeader` is `false`. It tells anyone which framework serves the site; turning it off is a one-line change.",
                source: "https://nextjs.org/docs/app/api-reference/config/next-config-js/poweredByHeader",
            },
            found.rel.clone(),
            Some("poweredByHeader"),
            "next (any version)".into(),
            Some(("poweredByHeader", json!(false))),
        )],
    }
}

fn next_images_domains(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("next", "14.0.0") else {
        return vec![];
    };
    match p.present("next", "images.domains") {
        Some(file) => vec![finding(
            Rule {
                rule: "next.images-domains",
                kind: FindingKind::Deprecated,
                title: "`images.domains` is deprecated",
                detail: "Since Next.js 14, `images.remotePatterns` replaces `images.domains`; patterns can also restrict protocol, port and path.",
                source: "https://nextjs.org/docs/app/api-reference/components/image#domains",
            },
            file,
            Some("images.domains"),
            format!("next >= 14 (installed {v})"),
            None,
        )],
        None => vec![],
    }
}

fn next_experimental_turbo(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("next", "15.3.0") else {
        return vec![];
    };
    match p.present("next", "experimental.turbo") {
        Some(file) => vec![finding(
            Rule {
                rule: "next.experimental-turbo",
                kind: FindingKind::Deprecated,
                title: "`experimental.turbo` moved to `turbopack`",
                detail: "Since Next.js 15.3 the Turbopack options live in the top-level `turbopack` key.",
                source: "https://nextjs.org/docs/app/api-reference/config/next-config-js/turbopack",
            },
            file,
            Some("experimental.turbo"),
            format!("next >= 15.3 (installed {v})"),
            None,
        )],
        None => vec![],
    }
}

fn next_typed_routes_moved(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("next", "15.5.0") else {
        return vec![];
    };
    match p.present("next", "experimental.typedRoutes") {
        Some(file) => vec![finding(
            Rule {
                rule: "next.typed-routes-moved",
                kind: FindingKind::Deprecated,
                title: "`experimental.typedRoutes` is now `typedRoutes`",
                detail: "Typed routes became stable in Next.js 15.5 and moved out of `experimental`.",
                source: "https://nextjs.org/blog/next-15-5",
            },
            file,
            Some("experimental.typedRoutes"),
            format!("next >= 15.5 (installed {v})"),
            Some(("typedRoutes", json!(true))),
        )],
        None => vec![],
    }
}

fn next_typed_routes_available(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("next", "15.5.0") else {
        return vec![];
    };
    let typescript = p.file("tsconfig").is_some();
    let Some(found) = p.file("next") else {
        return vec![];
    };
    if !typescript
        || p.present("next", "typedRoutes").is_some()
        || p.present("next", "experimental.typedRoutes").is_some()
    {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "next.typed-routes-available",
            kind: FindingKind::Available,
            title: "Typed routes are available",
            detail: "With `typedRoutes: true`, TypeScript checks every `<Link href>` and `router.push` against the app's real routes.",
            source: "https://nextjs.org/docs/app/api-reference/config/typescript#statically-typed-links",
        },
        found.rel.clone(),
        Some("typedRoutes"),
        format!("next >= 15.5 (installed {v})"),
        Some(("typedRoutes", json!(true))),
    )]
}

fn next_cache_components(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("next", "16.0.0") else {
        return vec![];
    };
    ["experimental.dynamicIO", "experimental.ppr"]
        .iter()
        .filter_map(|key| {
            let file = p.present("next", key)?;
            Some(finding(
                Rule {
                    rule: "next.cache-components",
                    kind: FindingKind::Incompatible,
                    title: "Replaced by `cacheComponents` in Next.js 16",
                    detail: "Next.js 16 folds `experimental.dynamicIO` and `experimental.ppr` into the `cacheComponents` option.",
                    source: "https://nextjs.org/blog/next-16",
                },
                file,
                Some(key),
                format!("next >= 16 (installed {v})"),
                None,
            ))
        })
        .collect()
}

fn next_ignore_build_errors(p: &Project) -> Vec<ConfigFinding> {
    match p.literal("next", "typescript.ignoreBuildErrors") {
        Some((file, Value::Bool(true))) => vec![finding(
            Rule {
                rule: "next.ignore-build-errors",
                kind: FindingKind::Development,
                title: "Builds succeed with TypeScript errors",
                detail: "`typescript.ignoreBuildErrors: true` lets `next build` ship code that doesn't type-check. Run `tsc` elsewhere (CI) if you keep it.",
                source: "https://nextjs.org/docs/app/api-reference/config/next-config-js/typescript",
            },
            file,
            Some("typescript.ignoreBuildErrors"),
            "next (any version)".into(),
            Some(("typescript.ignoreBuildErrors", json!(false))),
        )],
        _ => vec![],
    }
}

fn css_files(p: &Project) -> Vec<(String, String)> {
    p.files
        .iter()
        .filter(|f| f.kind.id == "tailwind-css")
        .filter_map(|f| Some((f.rel.clone(), p.text(f)?)))
        .collect()
}

fn tailwind4_js_config(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("tailwindcss", "4.0.0") else {
        return vec![];
    };
    let Some(config) = p.file("tailwind") else {
        return vec![];
    };
    if css_files(p).iter().any(|(_, css)| css.contains("@config")) {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "tailwind.v4-js-config",
            kind: FindingKind::Incompatible,
            title: "Tailwind CSS 4 doesn't load this JavaScript config",
            detail: "v4 is configured in CSS (`@theme`). A `tailwind.config.js` only applies when a stylesheet loads it with `@config \"./tailwind.config.js\"`; no stylesheet here does.",
            source: "https://tailwindcss.com/docs/upgrade-guide#using-a-javascript-config-file",
        },
        config.rel.clone(),
        None,
        format!("tailwindcss >= 4 (installed {v})"),
        None,
    )]
}

fn tailwind4_directives(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("tailwindcss", "4.0.0") else {
        return vec![];
    };
    css_files(p)
        .into_iter()
        .filter(|(_, css)| css.contains("@tailwind base") || css.contains("@tailwind utilities"))
        .map(|(file, _)| {
            finding(
                Rule {
                    rule: "tailwind.v4-directives",
                    kind: FindingKind::Incompatible,
                    title: "`@tailwind` directives are v3 syntax",
                    detail: "Tailwind CSS 4 replaces `@tailwind base/components/utilities` with a single `@import \"tailwindcss\";`.",
                    source: "https://tailwindcss.com/docs/upgrade-guide#removed-tailwind-directives",
                },
                file,
                None,
                format!("tailwindcss >= 4 (installed {v})"),
                None,
            )
        })
        .collect()
}

fn tailwind4_postcss_plugin(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("tailwindcss", "4.0.0") else {
        return vec![];
    };
    let Some(found) = p.file("postcss") else {
        return vec![];
    };
    let Some(text) = p.text(found) else {
        return vec![];
    };
    let mentions_old = text.contains("tailwindcss:")
        || text.contains("'tailwindcss'")
        || text.contains("\"tailwindcss\"");
    if !mentions_old || text.contains("@tailwindcss/postcss") {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "tailwind.v4-postcss-plugin",
            kind: FindingKind::Incompatible,
            title: "The PostCSS plugin moved to `@tailwindcss/postcss`",
            detail: "In Tailwind CSS 4 the `tailwindcss` package is no longer a PostCSS plugin; the config must use `@tailwindcss/postcss` (and no longer needs autoprefixer).",
            source: "https://tailwindcss.com/docs/upgrade-guide#using-postcss",
        },
        found.rel.clone(),
        None,
        format!("tailwindcss >= 4 (installed {v})"),
        None,
    )]
}

fn eslintrc_with_eslint9(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("eslint", "9.0.0") else {
        return vec![];
    };
    let Some(found) = p.file("eslintrc") else {
        return vec![];
    };
    if p.file("eslint").is_some() {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "eslint.eslintrc-ignored",
            kind: FindingKind::Incompatible,
            title: "ESLint 9 ignores `.eslintrc` by default",
            detail: "ESLint 9 reads `eslint.config.js` (flat config). An eslintrc file only applies with `ESLINT_USE_FLAT_CONFIG=false`; `npx @eslint/migrate-config` converts it.",
            source: "https://eslint.org/docs/latest/use/configure/migration-guide",
        },
        found.rel.clone(),
        None,
        format!("eslint >= 9 (installed {v})"),
        None,
    )]
}

fn ts_import_flags(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("typescript", "5.0.0") else {
        return vec![];
    };
    ["compilerOptions.importsNotUsedAsValues", "compilerOptions.preserveValueImports"]
        .iter()
        .filter_map(|key| {
            let file = p.present("tsconfig", key)?;
            Some(finding(
                Rule {
                    rule: "typescript.import-flags",
                    kind: FindingKind::Deprecated,
                    title: "Deprecated in TypeScript 5.0, an error since 5.5",
                    detail: "`importsNotUsedAsValues` and `preserveValueImports` were replaced by `verbatimModuleSyntax`.",
                    source: "https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-0.html#deprecations-and-default-changes",
                },
                file,
                Some(key),
                format!("typescript >= 5.0 (installed {v})"),
                Some(("compilerOptions.verbatimModuleSyntax", json!(true))),
            ))
        })
        .collect()
}

fn ts_bundler_resolution(p: &Project) -> Vec<ConfigFinding> {
    let Some(v) = p.at_least("typescript", "5.0.0") else {
        return vec![];
    };
    let bundled = ["vite", "next", "nuxt", "astro"]
        .iter()
        .any(|k| p.file(k).is_some());
    match p.literal("tsconfig", "compilerOptions.moduleResolution") {
        Some((file, Value::String(s)))
            if bundled && matches!(s.to_lowercase().as_str(), "node" | "node10" | "classic") =>
        {
            vec![finding(
                Rule {
                    rule: "typescript.bundler-resolution",
                    kind: FindingKind::Development,
                    title: "Module resolution doesn't match the bundler",
                    detail: "`node10` (`node`) resolution ignores `exports` in package.json, which the bundler honors, so types can differ from what runs. TypeScript 5.0 added `bundler` for this case.",
                    source: "https://www.typescriptlang.org/tsconfig/#moduleResolution",
                },
                file,
                Some("compilerOptions.moduleResolution"),
                format!("typescript >= 5.0 (installed {v})"),
                Some(("compilerOptions.moduleResolution", json!("bundler"))),
            )]
        }
        _ => vec![],
    }
}

fn composer_unstable(p: &Project) -> Vec<ConfigFinding> {
    let Some((file, Value::String(stability))) = p.literal("composer", "minimum-stability") else {
        return vec![];
    };
    if stability.eq_ignore_ascii_case("stable") {
        return vec![];
    }
    if matches!(
        p.literal("composer", "prefer-stable"),
        Some((_, Value::Bool(true)))
    ) {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "composer.unstable",
            kind: FindingKind::Development,
            title: "Unstable releases can be installed everywhere",
            detail: "With `minimum-stability` below `stable` and no `prefer-stable: true`, Composer may pick dev/alpha/beta releases for every package, not just the one that needed it.",
            source: "https://getcomposer.org/doc/04-schema.md#minimum-stability",
        },
        file,
        Some("prefer-stable"),
        "composer (any version)".into(),
        Some(("prefer-stable", json!(true))),
    )]
}

fn package_manager_mismatch(p: &Project) -> Vec<ConfigFinding> {
    let Some((file, Value::String(field))) = p.literal("package", "packageManager") else {
        return vec![];
    };
    let declared = field.split('@').next().unwrap_or("").to_string();
    let locks = [
        ("npm", "package-lock.json"),
        ("pnpm", "pnpm-lock.yaml"),
        ("yarn", "yarn.lock"),
        ("bun", "bun.lock"),
        ("bun", "bun.lockb"),
    ];
    let present: Vec<&str> = locks
        .iter()
        .filter(|(_, f)| p.root.join(f).exists())
        .map(|(m, _)| *m)
        .collect();
    if present.is_empty() || present.contains(&declared.as_str()) {
        return vec![];
    }
    vec![finding(
        Rule {
            rule: "package.manager-mismatch",
            kind: FindingKind::Development,
            title: "`packageManager` disagrees with the lockfile",
            detail: "package.json names one package manager (Corepack enforces it) while the lockfile belongs to another, so installs can resolve different versions.",
            source: "https://nodejs.org/api/packages.html#packagemanager",
        },
        file,
        Some("packageManager"),
        format!("declared {declared}, lockfile from {}", present.join(", ")),
        None,
    )]
}
