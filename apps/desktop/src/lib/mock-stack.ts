// Framework adapters for the mock backend. The real detection is the
// `locdev-frameworks` engine; this only feeds the UI realistic shapes.

import { DaemonError } from "./errors";
import type {
  Component,
  ComponentCategory,
  DefinitionInfo,
  Integration,
  ProjectView,
  RegistryStatus,
  Stack,
  StackReport,
  StackRequest,
  StackResponse,
} from "./protocol";

const c = (
  id: string,
  name: string,
  category: ComponentCategory,
  version: string | null,
  source: string | null,
  evidence: string,
  primary = false,
): Component => ({ id, name, category, version, version_source: source, evidence: [evidence], manual: false, primary });

const COMPONENTS: Partial<Record<ProjectView["kind"], Component[]>> = {
  laravel: [
    c("laravel", "Laravel", "framework", "12.38.1", "vendor/composer/installed.json", "artisan", true),
    c("nodejs", "Node.js", "runtime", null, null, "package.json"),
    c(
      "inertia",
      "Inertia",
      "tool",
      "2.0.10",
      "vendor/composer/installed.json",
      "composer.json: inertiajs/inertia-laravel",
    ),
    c("tailwind", "Tailwind CSS", "tool", "4.3.3", "node_modules", "package.json: tailwindcss"),
    c("vite", "Vite", "tool", "7.2.4", "node_modules", "package.json: vite"),
    c("vue", "Vue", "ui", "3.5.44", "node_modules", "package.json: vue"),
    c("php", "PHP", "language", null, null, "composer.json"),
  ],
  next: [
    c("nextjs", "Next.js", "framework", "16.4.3", "node_modules", "package.json: next", true),
    c("nodejs", "Node.js", "runtime", null, null, "package.json"),
    c("tailwind", "Tailwind CSS", "tool", "4.3.3", "node_modules", "package.json: tailwindcss"),
    c("react", "React", "ui", "19.3.1", "node_modules", "package.json: react"),
    c("typescript", "TypeScript", "language", "5.9.3", "node_modules", "tsconfig.json"),
  ],
  node: [
    c("vite", "Vite", "tool", "8.3.5", "node_modules", "package.json: vite", true),
    c("nodejs", "Node.js", "runtime", null, null, "package.json"),
    c("react", "React", "ui", "19.3.1", "node_modules", "package.json: react"),
    c("typescript", "TypeScript", "language", "5.9.3", "node_modules", "tsconfig.json"),
  ],
  wordpress: [
    c("wordpress", "WordPress", "framework", null, null, "wp-config.php", true),
    c("php", "PHP", "language", null, null, "index.php"),
  ],
  static: [c("html", "HTML5", "language", null, null, "index.html", true)],
};

const ADAPTER: Partial<Record<ProjectView["kind"], [string, Integration]>> = {
  laravel: ["laravel", "native"],
  next: ["nextjs", "script"],
  node: ["vite", "script"],
  wordpress: ["wordpress", "native"],
  static: ["html", "custom"],
};

/** The stack fields the mock projects carry. */
export function mockStackExtras(
  kind: ProjectView["kind"],
): Omit<Stack, "kind" | "framework" | "dev_tool" | "package_manager" | "command" | "dev_server" | "config_files"> {
  const [adapter, integration] = ADAPTER[kind] ?? [null, "none"];
  return {
    adapter,
    integration,
    components: structuredClone(COMPONENTS[kind] ?? []),
    notes:
      kind === "static"
        ? [
            "Served by LocDev's built-in static server (clean URLs, 404.html, no cache); change the command with `locdev edit` to use another one.",
          ]
        : kind === "wordpress"
          ? ["PHP's built-in server ignores .htaccess; pretty permalinks work only through index.php routing."]
          : [],
    port_env: [],
    venv: null,
    requires: [],
    workspaces: [],
    processes:
      kind === "laravel"
        ? [
            {
              id: "queue",
              name: "Queue worker",
              kind: "worker",
              command: "php artisan queue:work",
              port: false,
              conflicts: ["horizon"],
              note: "Restart it after changing job code: a running worker keeps the old code in memory.",
              source: "laravel",
            },
            {
              id: "horizon",
              name: "Horizon",
              kind: "worker",
              command: "php artisan horizon",
              port: false,
              conflicts: ["queue"],
              note: "Horizon runs the Redis queue workers itself; don't run queue:work next to it.",
              source: "laravel",
            },
            {
              id: "scheduler",
              name: "Scheduler",
              kind: "scheduler",
              command: "php artisan schedule:work",
              port: false,
              conflicts: [],
              note: null,
              source: "laravel",
            },
          ]
        : [],
  };
}

const def = (
  id: string,
  name: string,
  category: ComponentCategory,
  integration: Integration,
  dev: string[],
  docs: string,
  extra: Partial<DefinitionInfo> = {},
): DefinitionInfo => ({
  id,
  name,
  category,
  integration,
  docs,
  runtimes: [],
  requires: [],
  dev,
  install: null,
  build: null,
  test: null,
  supported: null,
  config_files: [],
  services: [],
  notes: [],
  ...extra,
});

const DEFINITIONS: DefinitionInfo[] = [
  def(
    "laravel",
    "Laravel",
    "framework",
    "native",
    ["php artisan serve --host=127.0.0.1 --port={port} --no-reload"],
    "https://laravel.com/docs",
    {
      runtimes: ["php", "composer"],
      install: "composer install",
      test: "php artisan test",
      supported: ">=10",
      services: ["postgres", "mysql", "redis", "queue", "scheduler", "reverb", "mailpit"],
    },
  ),
  def(
    "nextjs",
    "Next.js",
    "framework",
    "script",
    ["<pm> run dev --hostname 127.0.0.1 --port {port}"],
    "https://nextjs.org/docs/app/api-reference/cli/next",
    { runtimes: ["node"], supported: ">=13" },
  ),
  def(
    "django",
    "Django",
    "framework",
    "native",
    ["python3 manage.py runserver 127.0.0.1:{port}"],
    "https://docs.djangoproject.com/en/stable/ref/django-admin/#runserver",
    { runtimes: ["python"], requires: ["python3"] },
  ),
  def(
    "rails",
    "Ruby on Rails",
    "framework",
    "native",
    ["bin/rails server -b 127.0.0.1 -p {port}"],
    "https://guides.rubyonrails.org/command_line.html#bin-rails-server",
    { runtimes: ["ruby"], requires: ["ruby", "bundle"] },
  ),
  def(
    "spring",
    "Spring Boot",
    "framework",
    "native",
    ["./mvnw spring-boot:run"],
    "https://docs.spring.io/spring-boot/maven-plugin/run.html",
    { runtimes: ["java"] },
  ),
  def(
    "vite",
    "Vite",
    "tool",
    "script",
    ["<pm> run dev --port {port} --strictPort --host 127.0.0.1"],
    "https://vite.dev/guide/",
  ),
  def("react", "React", "ui", "none", [], "https://react.dev/learn/build-a-react-app-from-scratch"),
  def("tailwind", "Tailwind CSS", "tool", "none", [], "https://tailwindcss.com/docs/installation"),
  def("python", "Python", "language", "custom", [], "https://docs.python.org/3/library/venv.html"),
  def("go", "Go", "language", "native", ["go run ."], "https://go.dev/doc/tutorial/", { requires: ["go"] }),
];

let registry: RegistryStatus = {
  origin: "bundled",
  revision: 2026110201,
  bundled_revision: 2026110201,
  format: 1,
  count: 46,
  url: "https://github.com/trbros/locdev/releases/download/definitions/frameworks.json",
  checked_at: null,
  error: null,
  previous: null,
  skipped: null,
};

export function handleStack(req: StackRequest, sites: () => ProjectView[]): StackResponse {
  const site = (name: string) => {
    const s = sites().find((s) => s.name === name);
    if (!s) throw new DaemonError("app", `no project named ${name}`, "NOT_FOUND");
    return s;
  };
  const report = (s: ProjectView): StackReport => ({
    target: s.name,
    path: s.path,
    stack: s.stack,
    command: s.command ?? null,
    prerequisites: s.stack.requires.map((program) => ({ program, found: `/usr/bin/${program}` })),
    definition: DEFINITIONS.find((d) => d.id === s.stack.adapter) ?? null,
    install:
      s.kind === "laravel" ? "composer install" : s.stack.package_manager ? `${s.stack.package_manager} install` : null,
    build: s.stack.package_manager ? `${s.stack.package_manager} run build` : null,
    test: s.kind === "laravel" ? "php artisan test" : null,
    adapter_override: s.adapter ?? null,
    added: s.stacks ?? [],
    hidden: s.hidden_stacks ?? [],
  });
  switch (req.type) {
    case "stack_definitions":
      return { type: "stack_definitions", definitions: DEFINITIONS, registry };
    case "refresh_definitions":
      registry = {
        ...registry,
        checked_at: Date.now(),
        error: "https://github.com/trbros/locdev/releases/download/definitions/frameworks.json: http status: 404",
      };
      throw new DaemonError("app", registry.error!, "REMOTE");
    case "rollback_definitions":
      throw new DaemonError("app", "the bundled definitions are in use; there is nothing to roll back", "CONFLICT");
    case "stack_detect":
      if (req.name) return { type: "stack_report", ...report(site(req.name)) };
      return {
        type: "stack_report",
        target: req.path ?? "",
        path: req.path ?? "",
        stack: { ...site(sites()[0].name).stack },
        command: null,
        prerequisites: [],
        definition: null,
        install: null,
        build: null,
        test: null,
        adapter_override: null,
        added: [],
        hidden: [],
      };
    case "stack_set": {
      const s = site(req.name);
      for (const id of [...(req.add ?? []), ...(req.remove ?? [])]) {
        if (!DEFINITIONS.some((d) => d.id === id) && !s.stack.components.some((c) => c.id === id)) {
          throw new DaemonError("app", `no framework definition \`${id}\` (see \`locdev stack list\`)`, "NOT_FOUND");
        }
      }
      s.stacks ??= [];
      s.hidden_stacks ??= [];
      for (const id of req.add ?? []) {
        s.hidden_stacks = s.hidden_stacks.filter((h) => h !== id);
        if (!s.stacks.includes(id)) s.stacks.push(id);
        const d = DEFINITIONS.find((d) => d.id === id);
        if (d && !s.stack.components.some((c) => c.id === id)) {
          s.stack.components.push({ ...c(id, d.name, d.category, null, null, "added by hand"), manual: true });
        }
      }
      for (const id of req.remove ?? []) {
        if (s.stacks.includes(id)) s.stacks = s.stacks.filter((x) => x !== id);
        else if (!s.hidden_stacks.includes(id)) s.hidden_stacks.push(id);
        s.stack.components = s.stack.components.filter((x) => x.id !== id || x.primary);
      }
      if (req.adapter !== undefined && req.adapter !== null) s.adapter = req.adapter || undefined;
      return { type: "stack_report", ...report(s) };
    }
  }
}
