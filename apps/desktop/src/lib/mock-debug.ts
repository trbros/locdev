import type {
  DebugEvent,
  DebugRequest,
  DumpNode,
  EventFilter,
  EventGroup,
  ObserveSettings,
  ProjectView,
  Response,
} from "./protocol";
import { SEVERITY_ORDER } from "./protocol";

const MIN = 60_000;
const now = Date.now();
const str = (v: string): DumpNode => ({ t: "str", v });

const ORDER_DUMP: DumpNode = {
  t: "object",
  class: "App\\Models\\Order",
  handle: 1842,
  items: [
    { key: "connection", kind: "protected", value: str("mysql") },
    { key: "table", kind: "protected", value: str("orders") },
    { key: "exists", kind: "public", value: { t: "bool", v: true } },
    {
      key: "attributes",
      kind: "protected",
      value: {
        t: "array",
        count: 6,
        items: [
          { key: "id", kind: "key", value: { t: "int", v: 42 } },
          { key: "customer_id", kind: "key", value: { t: "int", v: 7 } },
          { key: "total", kind: "key", value: str("189.90") },
          { key: "status", kind: "key", value: str("pending") },
          { key: "coupon", kind: "key", value: { t: "null" } },
          { key: "created_at", kind: "key", value: str("2026-11-05 09:58:12") },
        ],
      },
    },
    {
      key: "relations",
      kind: "protected",
      value: {
        t: "array",
        count: 1,
        items: [
          {
            key: "customer",
            kind: "key",
            value: {
              t: "object",
              class: "App\\Models\\Customer",
              handle: 1850,
              items: [
                { key: "name", kind: "key", value: str("Ana Souza") },
                { key: "api_token", kind: "key", value: { t: "redacted" } },
              ],
            },
          },
        ],
      },
    },
  ],
};

function ev(
  id: number,
  ago: number,
  e: Partial<DebugEvent> & Pick<DebugEvent, "kind" | "severity" | "message">,
): DebugEvent {
  return {
    id,
    at: now - ago,
    source: "output",
    fingerprint: `${e.kind}:${e.code ?? ""}:${e.message.replace(/\d+/g, "#")}`,
    ...e,
  };
}

const laravelStack = [
  {
    function: "App\\Http\\Controllers\\CheckoutController->store()",
    file: "~/Code/acme-shop/app/Http/Controllers/CheckoutController.php",
    line: 58,
    vendor: false,
  },
  {
    function: "Illuminate\\Routing\\Controller->callAction()",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Routing/Controller.php",
    line: 54,
    vendor: true,
  },
  {
    function: "Illuminate\\Routing\\ControllerDispatcher->dispatch()",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Routing/ControllerDispatcher.php",
    line: 43,
    vendor: true,
  },
  {
    function: "Illuminate\\Routing\\Route->runController()",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Routing/Route.php",
    line: 265,
    vendor: true,
  },
  {
    function: "Illuminate\\Pipeline\\Pipeline->then()",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Pipeline/Pipeline.php",
    line: 126,
    vendor: true,
  },
  { function: "require_once()", file: "~/Code/acme-shop/public/index.php", line: 20, vendor: false },
];

let events: DebugEvent[] = [
  ev(1, 95 * MIN, {
    kind: "job",
    severity: "error",
    source: "jobs",
    message: "registry.npmjs.org: connection timed out",
    code: "Dependency check",
    site: "blog",
  }),
  ev(2, 70 * MIN, {
    kind: "build",
    severity: "error",
    site: "blog",
    process: "blog",
    runtime: "node",
    framework: "Next.js",
    message: "Module not found: Can't resolve '@/components/pricing-table'",
    file: "~/Code/blog/app/pricing/page.tsx",
    line: 3,
    hint: "A dependency is missing or the import path is wrong.",
    raw: " ⨯ ./app/pricing/page.tsx:3:1\nModule not found: Can't resolve '@/components/pricing-table'",
  }),
  ev(3, 42 * MIN, {
    kind: "process",
    severity: "error",
    source: "supervisor",
    site: "wp-agency",
    process: "wp-agency",
    code: "exit 255",
    message: "process exited with exit status: 255",
    raw: "PHP Fatal error:  Uncaught Error: Call to undefined function mysql_connect() in ~/Sites/wp-agency/wp-includes/load.php:1902",
  }),
  ev(4, 31 * MIN, {
    kind: "exception",
    severity: "error",
    site: "blog",
    process: "blog",
    runtime: "node",
    framework: "Next.js",
    code: "TypeError",
    message: "Cannot read properties of undefined (reading 'slug')",
    file: "~/Code/blog/app/posts/[slug]/page.tsx",
    line: 14,
    stack: [
      { function: "PostPage", file: "~/Code/blog/app/posts/[slug]/page.tsx", line: 14, column: 32, vendor: false },
      {
        function: "renderWithHooks",
        file: "~/Code/blog/node_modules/next/dist/compiled/react-dom/cjs/react-dom-server.node.development.js",
        line: 5658,
        column: 18,
        vendor: true,
      },
    ],
  }),
  ev(5, 12 * MIN, {
    kind: "http",
    severity: "error",
    source: "proxy",
    site: "acme-shop",
    code: "500",
    message: "500 POST /checkout",
    request: { method: "POST", url: "acme-shop.test/checkout", status: 500, duration_ms: 184 },
  }),
  ev(6, 12 * MIN, {
    kind: "exception",
    severity: "error",
    source: "log_file",
    site: "acme-shop",
    runtime: "php",
    framework: "Laravel + Vite",
    code: "Illuminate\\Database\\QueryException",
    message:
      "SQLSTATE[HY000] [2002] Connection refused (Connection: mysql, SQL: select * from `coupons` where `code` = ? limit 1)",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Database/Connection.php",
    line: 825,
    stack: laravelStack,
    hint: "The database isn't running or listens elsewhere: check Services and the site's DB_HOST/DB_PORT.",
    context: { channel: "local", log: "storage/logs/laravel.log" },
  }),
  ev(7, 6 * MIN, {
    kind: "log",
    severity: "warn",
    site: "acme-shop",
    process: "acme-shop",
    runtime: "php",
    framework: "Laravel + Vite",
    code: "Deprecated",
    message: "strlen(): Passing null to parameter #1 ($string) of type string is deprecated",
    file: "~/Code/acme-shop/app/Support/Money.php",
    line: 31,
  }),
  ev(8, 3 * MIN, {
    kind: "dump",
    severity: "info",
    source: "dumper",
    site: "acme-shop",
    runtime: "php",
    framework: "Laravel + Vite",
    message: "App\\Models\\Order {#1842}",
    file: "~/Code/acme-shop/app/Http/Controllers/CheckoutController.php",
    line: 51,
    request: {
      method: "POST",
      url: "https://acme-shop.test/checkout",
      controller: "App\\Http\\Controllers\\CheckoutController@store",
    },
    dump: ORDER_DUMP,
  }),
  ev(9, 2 * MIN, {
    kind: "http",
    severity: "error",
    source: "proxy",
    site: "acme-shop",
    code: "500",
    message: "500 POST /checkout",
    request: { method: "POST", url: "acme-shop.test/checkout", status: 500, duration_ms: 171 },
  }),
  ev(10, 2 * MIN, {
    kind: "exception",
    severity: "error",
    source: "log_file",
    site: "acme-shop",
    runtime: "php",
    framework: "Laravel + Vite",
    code: "Illuminate\\Database\\QueryException",
    message:
      "SQLSTATE[HY000] [2002] Connection refused (Connection: mysql, SQL: select * from `coupons` where `code` = ? limit 1)",
    file: "~/Code/acme-shop/vendor/laravel/framework/src/Illuminate/Database/Connection.php",
    line: 825,
    stack: laravelStack,
    hint: "The database isn't running or listens elsewhere: check Services and the site's DB_HOST/DB_PORT.",
    context: { channel: "local", log: "storage/logs/laravel.log" },
    raw: '[2026-11-05 10:01:12] local.ERROR: SQLSTATE[HY000] [2002] Connection refused … {"exception":"[object] (Illuminate\\\\Database\\\\QueryException(code: 2002): …"}',
  }),
];
events[5].fingerprint = events[9].fingerprint;
events[4].fingerprint = events[8].fingerprint;

const next = events.length;

function matches(e: DebugEvent, f: EventFilter = {}) {
  if (f.site && e.site !== f.site) return false;
  if (f.kinds?.length && !f.kinds.includes(e.kind)) return false;
  if (f.min_severity && SEVERITY_ORDER.indexOf(e.severity) < SEVERITY_ORDER.indexOf(f.min_severity)) return false;
  if (f.after != null && e.id <= f.after) return false;
  if (f.before != null && e.id >= f.before) return false;
  if (f.fingerprint && e.fingerprint !== f.fingerprint) return false;
  const q = f.query?.toLowerCase();
  if (q && ![e.message, e.code, e.file, e.request?.url].some((v) => v?.toLowerCase().includes(q))) return false;
  return true;
}

function groups(f?: EventFilter): EventGroup[] {
  const map = new Map<string, EventGroup>();
  for (const e of events.filter((e) => matches(e, f))) {
    const g = map.get(e.fingerprint) ?? {
      fingerprint: e.fingerprint,
      kind: e.kind,
      severity: e.severity,
      site: e.site,
      message: e.message,
      code: e.code,
      count: 0,
      first: e.at,
      last: e.at,
      last_id: e.id,
      hourly: Array(24).fill(0),
    };
    g.count++;
    g.first = Math.min(g.first, e.at);
    if (e.id >= g.last_id) Object.assign(g, { last: e.at, last_id: e.id });
    const age = Math.floor((Date.now() - e.at) / 3_600_000);
    if (age < 24) g.hourly[23 - age]++;
    map.set(e.fingerprint, g);
  }
  // A busier history for the sparkline of the repeated database error.
  for (const g of map.values())
    if (g.count > 1 && g.kind === "exception")
      g.hourly = g.hourly.map(
        (n, i) => n + [0, 0, 1, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 1, 2, 0][i],
      );
  return [...map.values()].sort((a, b) => b.last_id - a.last_id);
}

export function handleDebug(
  req: DebugRequest,
  projects: () => ProjectView[],
  settings: () => ObserveSettings,
): Response {
  switch (req.type) {
    case "observe_events": {
      const list = events
        .filter((e) => matches(e, req.filter))
        .sort((a, b) => b.id - a.id)
        .slice(0, req.limit ?? 200);
      return { type: "observe_events", events: list, newest: next } as Response;
    }
    case "observe_event": {
      const event = events.find((e) => e.id === req.id);
      return (
        event ? { type: "observe_event", event } : { type: "error", code: "NOT_FOUND", message: `no event ${req.id}` }
      ) as Response;
    }
    case "observe_groups":
      return { type: "observe_groups", groups: groups(req.filter) } as Response;
    case "observe_status":
      return {
        type: "observe_status",
        settings: settings(),
        events: events.length,
        disk_bytes: 48_213,
        dump_server: "127.0.0.1:41873",
        dump_sites: projects()
          .filter((p) => p.dumps)
          .map((p) => p.name),
      } as Response;
    case "observe_clear":
      events = req.site ? events.filter((e) => e.site !== req.site) : [];
      return { type: "ok" } as Response;
    case "observe_dumps": {
      const p = projects().find((p) => p.name === req.site);
      if (p) p.dumps = req.enabled;
      return {
        type: "observe_dumps",
        site: req.site,
        enabled: req.enabled,
        restarted: p?.process.state === "running",
      } as Response;
    }
  }
}
