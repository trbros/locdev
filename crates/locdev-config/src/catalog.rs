//! The configuration files LocDev knows and the options it can edit in a
//! form. Only files that exist are ever shown; nothing here is created.
//!
//! Options carry the package versions they exist in, the documented default
//! and the official page, so the form can refuse what the installed version
//! doesn't support. Adding an option: one entry here plus a test that reads
//! and writes it in `tests.rs`.

use locdev_core::config::{ConfigFormat, OptionKind};

pub struct Kind {
    pub id: &'static str,
    pub label: &'static str,
    /// Exact file names at the project root.
    pub names: &'static [&'static str],
    /// `(prefix, suffix)` for families like `tsconfig.app.json`.
    pub family: Option<(&'static str, &'static str)>,
    /// Format when the extension alone would say otherwise (`.json` with comments).
    pub format: Option<ConfigFormat>,
    pub package: Option<&'static str>,
    pub docs: &'static str,
    pub options: &'static [Opt],
}

pub struct Opt {
    pub key: &'static str,
    pub section: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: OptionKind,
    /// JSON literals.
    pub choices: &'static [&'static str],
    /// JSON literal of the documented default.
    pub default: Option<&'static str>,
    pub example: Option<&'static str>,
    pub docs: Option<&'static str>,
    /// Lowest package version with the option (inclusive).
    pub since: Option<&'static str>,
    /// First package version without it (exclusive).
    pub until: Option<&'static str>,
}

const fn opt(
    key: &'static str,
    section: &'static str,
    label: &'static str,
    description: &'static str,
    kind: OptionKind,
) -> Opt {
    Opt {
        key,
        section,
        label,
        description,
        kind,
        choices: &[],
        default: None,
        example: None,
        docs: None,
        since: None,
        until: None,
    }
}

impl Opt {
    const fn default(mut self, value: &'static str) -> Self {
        self.default = Some(value);
        self
    }
    const fn choices(mut self, choices: &'static [&'static str]) -> Self {
        self.choices = choices;
        self
    }
    const fn example(mut self, example: &'static str) -> Self {
        self.example = Some(example);
        self
    }
    const fn docs(mut self, docs: &'static str) -> Self {
        self.docs = Some(docs);
        self
    }
    const fn since(mut self, version: &'static str) -> Self {
        self.since = Some(version);
        self
    }
}

use OptionKind::{Bool, Choice, Number, String as Text};

const VITE: &[Opt] = &[
    opt("server.port", "Server", "Port", "Port of the dev server. LocDev proxies the site's domain to it.", Number)
        .default("5173")
        .docs("https://vite.dev/config/server-options#server-port"),
    opt("server.host", "Server", "Host", "Address the dev server listens on. `true` or `0.0.0.0` exposes it to the network.", Text)
        .default("\"localhost\"")
        .docs("https://vite.dev/config/server-options#server-host"),
    opt("server.strictPort", "Server", "Strict port", "Fail instead of trying the next port when the port is taken.", Bool)
        .default("false")
        .docs("https://vite.dev/config/server-options#server-strictport"),
    opt("server.open", "Server", "Open browser", "Open the app in the browser when the server starts.", Bool)
        .default("false")
        .docs("https://vite.dev/config/server-options#server-open"),
    opt("preview.port", "Preview", "Port", "Port of `vite preview`.", Number)
        .default("4173")
        .docs("https://vite.dev/config/preview-options#preview-port"),
    opt("build.outDir", "Build", "Output folder", "Where the production build is written, relative to the root.", Text)
        .default("\"dist\"")
        .docs("https://vite.dev/config/build-options#build-outdir"),
    opt("build.sourcemap", "Build", "Source maps", "Source maps for the production build. `hidden` writes them without the comment that links them.", Choice)
        .choices(&["false", "true", "\"inline\"", "\"hidden\""])
        .default("false")
        .docs("https://vite.dev/config/build-options#build-sourcemap"),
    opt("build.emptyOutDir", "Build", "Empty output folder", "Empty the output folder before building (by default only when it's inside the root).", Bool)
        .docs("https://vite.dev/config/build-options#build-emptyoutdir"),
];

const NEXT: &[Opt] = &[
    opt("reactStrictMode", "React", "Strict mode", "React Strict Mode in development: double-invokes renders and effects to surface side effects.", Bool)
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/reactStrictMode"),
    opt("poweredByHeader", "Security", "X-Powered-By header", "Next.js sends `X-Powered-By: Next.js` unless this is `false`.", Bool)
        .default("true")
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/poweredByHeader"),
    opt("output", "Build", "Output", "`standalone` copies only the files needed to run with Node; `export` builds a static site.", Choice)
        .choices(&["\"standalone\"", "\"export\""])
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/output")
        .since("13.3.0"),
    opt("trailingSlash", "Routing", "Trailing slash", "Redirect URLs to the version ending in `/`.", Bool)
        .default("false")
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/trailingSlash"),
    opt("basePath", "Routing", "Base path", "Serve the app under a sub-path, like `/docs`.", Text)
        .example("\"/docs\"")
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/basePath"),
    opt("compress", "Performance", "gzip compression", "gzip responses from `next start`. Turn off when a proxy already compresses.", Bool)
        .default("true")
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/compress"),
    opt("images.unoptimized", "Images", "Unoptimized images", "Serve images as they are, without the optimization API.", Bool)
        .default("false")
        .docs("https://nextjs.org/docs/app/api-reference/components/image#unoptimized"),
    opt("typedRoutes", "TypeScript", "Typed routes", "Type-check `href`s against the app's routes.", Bool)
        .default("false")
        .docs("https://nextjs.org/docs/app/api-reference/config/typescript#statically-typed-links")
        .since("15.5.0"),
    opt("typescript.ignoreBuildErrors", "TypeScript", "Ignore type errors in builds", "Let `next build` succeed with TypeScript errors.", Bool)
        .default("false")
        .docs("https://nextjs.org/docs/app/api-reference/config/next-config-js/typescript"),
];

const TSCONFIG: &[Opt] = &[
    opt("compilerOptions.strict", "Type checking", "Strict", "Turns on the whole family of strict checks.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#strict"),
    opt("compilerOptions.noUncheckedIndexedAccess", "Type checking", "Checked index access", "Index access (`obj[key]`, `arr[i]`) includes `undefined` in its type.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#noUncheckedIndexedAccess")
        .since("4.1.0"),
    opt("compilerOptions.target", "Emit", "Target", "JavaScript version of the output and of the built-in library types.", Text)
        .example("\"ES2022\"")
        .docs("https://www.typescriptlang.org/tsconfig/#target"),
    opt("compilerOptions.module", "Modules", "Module", "Module system of the output.", Text)
        .example("\"ESNext\"")
        .docs("https://www.typescriptlang.org/tsconfig/#module"),
    opt("compilerOptions.moduleResolution", "Modules", "Module resolution", "How imports are resolved. `bundler` matches Vite, Next.js and other bundlers.", Choice)
        .choices(&["\"bundler\"", "\"node16\"", "\"nodenext\"", "\"node10\""])
        .docs("https://www.typescriptlang.org/tsconfig/#moduleResolution")
        .since("5.0.0"),
    opt("compilerOptions.verbatimModuleSyntax", "Modules", "Verbatim module syntax", "Imports and exports without `type` are kept as written; type-only ones must say `type`.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#verbatimModuleSyntax")
        .since("5.0.0"),
    opt("compilerOptions.erasableSyntaxOnly", "Modules", "Erasable syntax only", "Refuse syntax that type stripping (Node's `--experimental-strip-types`) can't remove: enums, namespaces, parameter properties.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#erasableSyntaxOnly")
        .since("5.8.0"),
    opt("compilerOptions.jsx", "Emit", "JSX", "How JSX is emitted.", Choice)
        .choices(&["\"preserve\"", "\"react-jsx\"", "\"react-jsxdev\"", "\"react\"", "\"react-native\""])
        .docs("https://www.typescriptlang.org/tsconfig/#jsx"),
    opt("compilerOptions.noEmit", "Emit", "No emit", "Type-check only; a bundler produces the JavaScript.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#noEmit"),
    opt("compilerOptions.skipLibCheck", "Performance", "Skip lib check", "Don't type-check `.d.ts` files; faster builds.", Bool)
        .default("false")
        .docs("https://www.typescriptlang.org/tsconfig/#skipLibCheck"),
];

const PACKAGE_JSON: &[Opt] = &[
    opt(
        "type",
        "Package",
        "Module type",
        "How Node treats `.js` files in this package.",
        Choice,
    )
    .choices(&["\"module\"", "\"commonjs\""])
    .default("\"commonjs\"")
    .docs("https://nodejs.org/api/packages.html#type"),
    opt(
        "private",
        "Package",
        "Private",
        "npm refuses to publish a private package.",
        Bool,
    )
    .docs("https://docs.npmjs.com/cli/configuring-npm/package-json#private"),
    opt(
        "engines.node",
        "Package",
        "Node.js versions",
        "Node.js versions the project supports.",
        Text,
    )
    .example("\">=20\"")
    .docs("https://docs.npmjs.com/cli/configuring-npm/package-json#engines"),
];

const COMPOSER_JSON: &[Opt] = &[
    opt(
        "minimum-stability",
        "Dependencies",
        "Minimum stability",
        "Least stable releases Composer accepts.",
        Choice,
    )
    .choices(&["\"stable\"", "\"RC\"", "\"beta\"", "\"alpha\"", "\"dev\""])
    .default("\"stable\"")
    .docs("https://getcomposer.org/doc/04-schema.md#minimum-stability"),
    opt(
        "prefer-stable",
        "Dependencies",
        "Prefer stable",
        "Prefer stable releases when both stable and unstable ones match.",
        Bool,
    )
    .default("false")
    .docs("https://getcomposer.org/doc/04-schema.md#prefer-stable"),
    opt(
        "config.sort-packages",
        "Config",
        "Sort packages",
        "Keep `require` sorted when adding packages.",
        Bool,
    )
    .default("false")
    .docs("https://getcomposer.org/doc/06-config.md#sort-packages"),
    opt(
        "config.optimize-autoloader",
        "Config",
        "Optimize autoloader",
        "Build a class map on every dump: faster, slower to dump.",
        Bool,
    )
    .default("false")
    .docs("https://getcomposer.org/doc/06-config.md#optimize-autoloader"),
    opt(
        "config.platform.php",
        "Config",
        "Platform PHP",
        "Resolve dependencies as if this PHP version were installed (match production).",
        Text,
    )
    .example("\"8.3.0\"")
    .docs("https://getcomposer.org/doc/06-config.md#platform"),
];

const PRETTIER: &[Opt] = &[
    opt(
        "printWidth",
        "Format",
        "Print width",
        "Line length Prettier wraps at.",
        Number,
    )
    .default("80")
    .docs("https://prettier.io/docs/options#print-width"),
    opt(
        "tabWidth",
        "Format",
        "Tab width",
        "Spaces per indentation level.",
        Number,
    )
    .default("2")
    .docs("https://prettier.io/docs/options#tab-width"),
    opt("useTabs", "Format", "Tabs", "Indent with tabs.", Bool)
        .default("false")
        .docs("https://prettier.io/docs/options#tabs"),
    opt(
        "semi",
        "Format",
        "Semicolons",
        "Semicolons at the end of statements.",
        Bool,
    )
    .default("true")
    .docs("https://prettier.io/docs/options#semicolons"),
    opt(
        "singleQuote",
        "Format",
        "Single quotes",
        "Single quotes instead of double.",
        Bool,
    )
    .default("false")
    .docs("https://prettier.io/docs/options#quotes"),
    opt(
        "trailingComma",
        "Format",
        "Trailing commas",
        "Where trailing commas go. The default became `all` in Prettier 3.",
        Choice,
    )
    .choices(&["\"all\"", "\"es5\"", "\"none\""])
    .default("\"all\"")
    .docs("https://prettier.io/docs/options#trailing-commas"),
    opt(
        "endOfLine",
        "Format",
        "End of line",
        "Line endings.",
        Choice,
    )
    .choices(&["\"lf\"", "\"crlf\"", "\"cr\"", "\"auto\""])
    .default("\"lf\"")
    .docs("https://prettier.io/docs/options#end-of-line"),
];

const BIOME: &[Opt] = &[
    opt(
        "formatter.enabled",
        "Formatter",
        "Enabled",
        "Biome's formatter.",
        Bool,
    )
    .default("true")
    .docs("https://biomejs.dev/reference/configuration/#formatterenabled"),
    opt(
        "formatter.indentStyle",
        "Formatter",
        "Indent style",
        "Tabs or spaces.",
        Choice,
    )
    .choices(&["\"tab\"", "\"space\""])
    .default("\"tab\"")
    .docs("https://biomejs.dev/reference/configuration/#formatterindentstyle"),
    opt(
        "formatter.lineWidth",
        "Formatter",
        "Line width",
        "Line length the formatter wraps at.",
        Number,
    )
    .default("80")
    .docs("https://biomejs.dev/reference/configuration/#formatterlinewidth"),
    opt(
        "linter.enabled",
        "Linter",
        "Enabled",
        "Biome's linter.",
        Bool,
    )
    .default("true")
    .docs("https://biomejs.dev/reference/configuration/#linterenabled"),
];

const NUXT: &[Opt] = &[
    opt(
        "ssr",
        "Rendering",
        "Server-side rendering",
        "Render pages on the server; `false` builds a single-page app.",
        Bool,
    )
    .default("true")
    .docs("https://nuxt.com/docs/api/nuxt-config#ssr"),
    opt(
        "devtools.enabled",
        "Development",
        "Nuxt DevTools",
        "The DevTools panel in development.",
        Bool,
    )
    .docs("https://nuxt.com/docs/api/nuxt-config#devtools"),
    opt(
        "compatibilityDate",
        "Development",
        "Compatibility date",
        "Pins the behavior of Nitro and presets to a date.",
        Text,
    )
    .example("\"2025-07-15\"")
    .docs("https://nuxt.com/docs/api/nuxt-config#compatibilitydate"),
];

const ASTRO: &[Opt] = &[
    opt(
        "output",
        "Rendering",
        "Output",
        "`static` prerenders every page; `server` renders on demand (needs an adapter).",
        Choice,
    )
    .choices(&["\"static\"", "\"server\""])
    .default("\"static\"")
    .docs("https://docs.astro.build/en/reference/configuration-reference/#output"),
    opt(
        "site",
        "Site",
        "Site URL",
        "Final deployed URL, used for sitemaps and canonical links.",
        Text,
    )
    .example("\"https://example.com\"")
    .docs("https://docs.astro.build/en/reference/configuration-reference/#site"),
    opt(
        "server.port",
        "Server",
        "Port",
        "Port of the dev server.",
        Number,
    )
    .default("4321")
    .docs("https://docs.astro.build/en/reference/configuration-reference/#serverport"),
    opt(
        "trailingSlash",
        "Site",
        "Trailing slash",
        "How URLs ending in `/` are matched.",
        Choice,
    )
    .choices(&["\"always\"", "\"never\"", "\"ignore\""])
    .default("\"ignore\"")
    .docs("https://docs.astro.build/en/reference/configuration-reference/#trailingslash"),
];

const REACT_ROUTER: &[Opt] = &[opt(
    "ssr",
    "Rendering",
    "Server-side rendering",
    "Render on the server; `false` builds a single-page app.",
    Bool,
)
.default("true")
.docs("https://reactrouter.com/api/framework-conventions/react-router.config.ts#ssr")];

const JS: &[&str] = &["js", "mjs", "cjs", "ts", "mts", "cts"];

macro_rules! js_names {
    ($base:literal) => {
        &[
            concat!($base, ".js"),
            concat!($base, ".mjs"),
            concat!($base, ".cjs"),
            concat!($base, ".ts"),
            concat!($base, ".mts"),
            concat!($base, ".cts"),
        ]
    };
}

pub const KINDS: &[Kind] = &[
    Kind {
        id: "vite",
        label: "Vite",
        names: js_names!("vite.config"),
        family: None,
        format: None,
        package: Some("vite"),
        docs: "https://vite.dev/config/",
        options: VITE,
    },
    Kind {
        id: "next",
        label: "Next.js",
        names: js_names!("next.config"),
        family: None,
        format: None,
        package: Some("next"),
        docs: "https://nextjs.org/docs/app/api-reference/config/next-config-js",
        options: NEXT,
    },
    Kind {
        id: "nuxt",
        label: "Nuxt",
        names: js_names!("nuxt.config"),
        family: None,
        format: None,
        package: Some("nuxt"),
        docs: "https://nuxt.com/docs/api/nuxt-config",
        options: NUXT,
    },
    Kind {
        id: "astro",
        label: "Astro",
        names: js_names!("astro.config"),
        family: None,
        format: None,
        package: Some("astro"),
        docs: "https://docs.astro.build/en/reference/configuration-reference/",
        options: ASTRO,
    },
    Kind {
        id: "svelte",
        label: "Svelte / SvelteKit",
        names: js_names!("svelte.config"),
        family: None,
        format: None,
        package: Some("@sveltejs/kit"),
        docs: "https://svelte.dev/docs/kit/configuration",
        options: &[],
    },
    Kind {
        id: "react-router",
        label: "React Router",
        names: js_names!("react-router.config"),
        family: None,
        format: None,
        package: Some("@react-router/dev"),
        docs: "https://reactrouter.com/api/framework-conventions/react-router.config.ts",
        options: REACT_ROUTER,
    },
    Kind {
        id: "angular",
        label: "Angular workspace",
        names: &["angular.json"],
        family: None,
        format: None,
        package: Some("@angular/core"),
        docs: "https://angular.dev/reference/configs/workspace-config",
        options: &[],
    },
    Kind {
        id: "nest",
        label: "Nest CLI",
        names: &["nest-cli.json"],
        family: None,
        format: None,
        package: Some("@nestjs/core"),
        docs: "https://docs.nestjs.com/cli/monorepo#cli-properties",
        options: &[],
    },
    Kind {
        id: "tailwind",
        label: "Tailwind CSS (JavaScript config)",
        names: js_names!("tailwind.config"),
        family: None,
        format: None,
        package: Some("tailwindcss"),
        docs: "https://v3.tailwindcss.com/docs/configuration",
        options: &[],
    },
    Kind {
        id: "postcss",
        label: "PostCSS",
        names: &[
            "postcss.config.js",
            "postcss.config.mjs",
            "postcss.config.cjs",
            "postcss.config.ts",
            ".postcssrc",
            ".postcssrc.json",
        ],
        family: None,
        format: None,
        package: Some("postcss"),
        docs: "https://github.com/postcss/postcss-load-config#usage",
        options: &[],
    },
    Kind {
        id: "eslint",
        label: "ESLint (flat config)",
        names: js_names!("eslint.config"),
        family: None,
        format: None,
        package: Some("eslint"),
        docs: "https://eslint.org/docs/latest/use/configure/configuration-files",
        options: &[],
    },
    Kind {
        id: "eslintrc",
        label: "ESLint (legacy eslintrc)",
        names: &[
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.js",
            ".eslintrc.cjs",
            ".eslintrc.yml",
            ".eslintrc.yaml",
        ],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: Some("eslint"),
        docs: "https://eslint.org/docs/v8.x/use/configure/configuration-files",
        options: &[],
    },
    Kind {
        id: "babel",
        label: "Babel",
        names: &[
            ".babelrc",
            ".babelrc.json",
            "babel.config.json",
            "babel.config.js",
            "babel.config.cjs",
            "babel.config.mjs",
        ],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: Some("@babel/core"),
        docs: "https://babeljs.io/docs/configuration",
        options: &[],
    },
    Kind {
        id: "biome",
        label: "Biome",
        names: &["biome.json", "biome.jsonc"],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: Some("@biomejs/biome"),
        docs: "https://biomejs.dev/reference/configuration/",
        options: BIOME,
    },
    Kind {
        id: "prettier",
        label: "Prettier",
        names: &[
            ".prettierrc",
            ".prettierrc.json",
            "prettier.config.js",
            "prettier.config.mjs",
            "prettier.config.cjs",
            "prettier.config.ts",
            ".prettierrc.js",
            ".prettierrc.mjs",
            ".prettierrc.cjs",
        ],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: Some("prettier"),
        docs: "https://prettier.io/docs/configuration",
        options: PRETTIER,
    },
    Kind {
        id: "tsconfig",
        label: "TypeScript",
        names: &["tsconfig.json"],
        family: Some(("tsconfig.", ".json")),
        format: Some(ConfigFormat::Jsonc),
        package: Some("typescript"),
        docs: "https://www.typescriptlang.org/tsconfig/",
        options: TSCONFIG,
    },
    Kind {
        id: "jsconfig",
        label: "jsconfig",
        names: &["jsconfig.json"],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: None,
        docs: "https://code.visualstudio.com/docs/languages/jsconfig",
        options: TSCONFIG,
    },
    Kind {
        id: "package",
        label: "package.json",
        names: &["package.json"],
        family: None,
        format: None,
        package: None,
        docs: "https://docs.npmjs.com/cli/configuring-npm/package-json",
        options: PACKAGE_JSON,
    },
    Kind {
        id: "composer",
        label: "composer.json",
        names: &["composer.json"],
        family: None,
        format: None,
        package: None,
        docs: "https://getcomposer.org/doc/04-schema.md",
        options: COMPOSER_JSON,
    },
    Kind {
        id: "phpunit",
        label: "PHPUnit",
        names: &["phpunit.xml", "phpunit.xml.dist"],
        family: None,
        format: Some(ConfigFormat::Xml),
        package: Some("phpunit/phpunit"),
        docs: "https://docs.phpunit.de/en/main/configuration.html",
        options: &[],
    },
    Kind {
        id: "vitest",
        label: "Vitest",
        names: js_names!("vitest.config"),
        family: None,
        format: None,
        package: Some("vitest"),
        docs: "https://vitest.dev/config/",
        options: &[],
    },
    Kind {
        id: "playwright",
        label: "Playwright",
        names: js_names!("playwright.config"),
        family: None,
        format: None,
        package: Some("@playwright/test"),
        docs: "https://playwright.dev/docs/test-configuration",
        options: &[],
    },
    Kind {
        id: "deno",
        label: "Deno",
        names: &["deno.json", "deno.jsonc"],
        family: None,
        format: Some(ConfigFormat::Jsonc),
        package: None,
        docs: "https://docs.deno.com/runtime/fundamentals/configuration/",
        options: &[],
    },
    Kind {
        id: "wrangler",
        label: "Cloudflare Wrangler",
        names: &["wrangler.toml", "wrangler.json", "wrangler.jsonc"],
        family: None,
        format: None,
        package: Some("wrangler"),
        docs: "https://developers.cloudflare.com/workers/wrangler/configuration/",
        options: &[],
    },
    Kind {
        id: "tailwind-css",
        label: "Tailwind CSS (CSS-first config)",
        // Found by content, not by name (see `files::tailwind_css`).
        names: &[],
        family: None,
        format: Some(ConfigFormat::Css),
        package: Some("tailwindcss"),
        docs: "https://tailwindcss.com/docs/theme",
        options: &[],
    },
];

pub fn kind(id: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.id == id)
}

/// The kind a root-level file name belongs to.
pub fn kind_of(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| {
        k.names.contains(&name)
            || k.family
                .is_some_and(|(prefix, suffix)| name.starts_with(prefix) && name.ends_with(suffix))
    })
}

/// The format of a file: the kind's override, else its extension.
pub fn format_of(kind: &Kind, name: &str) -> ConfigFormat {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    if JS.contains(&ext) {
        return ConfigFormat::Js;
    }
    match ext {
        "toml" => ConfigFormat::Toml,
        "yml" | "yaml" => ConfigFormat::Yaml,
        "xml" | "dist" => ConfigFormat::Xml,
        "css" => ConfigFormat::Css,
        "jsonc" => ConfigFormat::Jsonc,
        _ => kind.format.unwrap_or(ConfigFormat::Json),
    }
}
