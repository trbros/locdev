// The database manager for the mock backend: a tiny in-memory database.
// The real drivers and SQL checks live in `locdev-db`.

import { DaemonError } from "./errors";
import type { DbColumn, DbConnection, DbRequest, DbResponse, DbRows, RedisKey } from "./protocol";

const conn = (c: Partial<DbConnection> & Pick<DbConnection, "id" | "label" | "engine">): DbConnection => ({
  host: "127.0.0.1",
  port: null,
  database: null,
  username: null,
  has_password: false,
  source: "LocDev instance",
  project: null,
  remote: false,
  ...c,
});

const connections: DbConnection[] = [
  conn({
    id: "project:acme-shop:0",
    label: "PostgreSQL · acme_shop",
    engine: "postgres",
    port: 5433,
    database: "acme_shop",
    username: "locdev",
    source: ".env (DB_*)",
    project: "acme-shop",
  }),
  conn({
    id: "project:acme-shop:1",
    label: "Redis/Valkey",
    engine: "redis",
    port: 6380,
    source: ".env (REDIS_*)",
    project: "acme-shop",
  }),
  conn({
    id: "project:blog:0",
    label: "PostgreSQL · blog",
    engine: "postgres",
    host: "db.blog-prod.example.com",
    port: 5432,
    database: "blog",
    username: "app",
    has_password: true,
    source: ".env (DATABASE_URL)",
    project: "blog",
    remote: true,
  }),
  conn({
    id: "project:landing:0",
    label: "SQLite · dev.db",
    engine: "sqlite",
    host: null,
    source: ".env (DATABASE_URL)",
    project: "landing",
  }),
  conn({
    id: "instance:db",
    label: "db · PostgreSQL",
    engine: "postgres",
    port: 5433,
    database: "postgres",
    username: "locdev",
  }),
];

const columns: DbColumn[] = [
  { name: "id", data_type: "bigint", nullable: false, default: "nextval('users_id_seq'::regclass)", primary_key: true },
  { name: "name", data_type: "character varying(255)", nullable: false, default: null, primary_key: false },
  { name: "email", data_type: "character varying(255)", nullable: false, default: null, primary_key: false },
  { name: "role", data_type: "character varying(32)", nullable: false, default: "'member'", primary_key: false },
  {
    name: "created_at",
    data_type: "timestamp(0) without time zone",
    nullable: true,
    default: null,
    primary_key: false,
  },
];
const names = ["Ana", "Bruno", "Carla", "Davi", "Elisa", "Fábio", "Gabi", "Hugo", "Iara", "João", "Kátia", "Lucas"];
const users: (string | null)[][] = Array.from({ length: 137 }, (_, i) => [
  String(i + 1),
  `${names[i % names.length]} ${String.fromCharCode(65 + (i % 26))}.`,
  `user${i + 1}@acme.test`,
  i % 9 === 0 ? "admin" : "member",
  i % 7 === 0 ? null : `2026-10-${String((i % 28) + 1).padStart(2, "0")} 12:${String(i % 60).padStart(2, "0")}:00`,
]);
let landingCreated = false;

const redisData: Record<string, { kind: string; ttl: number; items: [string, string | null][] }> = {
  "laravel_cache:settings": { kind: "string", ttl: 3412, items: [['{"theme":"dark","locale":"pt_BR"}', null]] },
  "laravel_database_queues:default": {
    kind: "list",
    ttl: -1,
    items: [
      ['{"job":"SendWelcomeMail","id":"9f2"}', null],
      ['{"job":"ResizeImage","id":"a01"}', null],
    ],
  },
  "laravel_session:7f3c": {
    kind: "hash",
    ttl: 7104,
    items: [
      ["user_id", "12"],
      ["_token", "Xq4…"],
    ],
  },
  leaderboard: {
    kind: "zset",
    ttl: -1,
    items: [
      ["ana", "1520"],
      ["bruno", "1290"],
    ],
  },
};

function check(sql: string) {
  const destructive = sql
    .split(";")
    .map((s) => s.trim())
    .filter(
      (s) =>
        /^(drop|truncate)\b/i.test(s) ||
        (/^(delete|update)\b/i.test(s) && (!/\bwhere\b/i.test(s) || /\bwhere\s*\(?\s*(1\s*=\s*1|true)\b/i.test(s))),
    );
  return {
    destructive,
    writes: !/^\s*(select|with|show|explain)\b/i.test(sql),
    confirm: destructive.length ? "acme_shop" : null,
  };
}

export function handleDb(req: DbRequest): DbResponse | { type: "ok" } {
  const c = "connection" in req ? connections.find((x) => x.id === req.connection) : undefined;
  if ("connection" in req && !c) throw new DaemonError("app", `no database connection ${req.connection}`, "NOT_FOUND");
  if (c?.remote && "allow_remote" in req && !req.allow_remote) {
    throw new DaemonError(
      "app",
      `${c.label} is on ${c.host}, not this machine (it may be production); open it explicitly to continue`,
      "UNAUTHORIZED",
    );
  }
  if (c?.engine === "sqlite" && !landingCreated && req.type !== "db_create_database") {
    throw new DaemonError("app", "missing database: prisma/dev.db doesn't exist", "NOT_FOUND");
  }
  switch (req.type) {
    case "db_connections":
      return { type: "db_connections", connections };
    case "db_schema":
      return {
        type: "db_schema",
        databases: ["acme_shop", "postgres"],
        tables: [
          { name: "users", schema: "public", kind: "table", rows_estimate: users.length },
          { name: "migrations", schema: "public", kind: "table", rows_estimate: 14 },
          { name: "active_users", schema: "public", kind: "view", rows_estimate: null },
        ],
      };
    case "db_table_info":
      return {
        type: "db_table_info",
        table: { name: req.table, schema: "public", kind: "table", rows_estimate: users.length },
        columns,
        indexes: [
          { name: "users_pkey", columns: ["id"], unique: true },
          { name: "users_email_unique", columns: ["email"], unique: true },
        ],
        foreign_keys: [],
      };
    case "db_rows": {
      let rows = users.slice();
      for (const f of req.filters ?? []) {
        const i = columns.findIndex((col) => col.name === f.column);
        rows = rows.filter((r) => {
          const v = r[i];
          if (f.op === "is_null") return v === null;
          if (f.op === "not_null") return v !== null;
          if (f.op === "like") return v !== null && new RegExp(`^${(f.value ?? "").replace(/%/g, ".*")}$`, "i").test(v);
          return f.op === "=" ? v === f.value : v !== f.value;
        });
      }
      if (req.order_by) {
        const i = columns.findIndex((col) => col.name === req.order_by);
        rows.sort(
          (a, b) =>
            String(a[i] ?? "").localeCompare(String(b[i] ?? ""), undefined, { numeric: true }) * (req.desc ? -1 : 1),
        );
      }
      const size = req.page_size ?? 100;
      const page = req.page ?? 0;
      const out: DbRows = {
        columns: columns.map((col) => col.name),
        rows: rows.slice(page * size, page * size + size),
        total: rows.length,
        affected: null,
        truncated: false,
        elapsed_ms: 3,
      };
      return { type: "db_rows", ...out };
    }
    case "db_check":
      return { type: "db_check", ...check(req.sql) };
    case "db_query": {
      const c2 = check(req.sql);
      if (c2.confirm && req.confirm !== c2.confirm) {
        throw new DaemonError(
          "app",
          `this would destroy data; type the database's name (${c2.confirm}) to run it`,
          "UNAUTHORIZED",
        );
      }
      if (/^\s*select/i.test(req.sql)) {
        return {
          type: "db_rows",
          columns: columns.map((col) => col.name),
          rows: users.slice(0, 20),
          total: null,
          affected: null,
          truncated: false,
          elapsed_ms: 2,
        };
      }
      return { type: "db_rows", columns: [], rows: [], total: null, affected: 1, truncated: false, elapsed_ms: 1 };
    }
    case "db_edit_row": {
      const e = req.edit;
      if (e.op === "update") {
        const row = users.find((r) => r[0] === e.key[0][1]);
        for (const [col, v] of e.changes) if (row) row[columns.findIndex((x) => x.name === col)] = v;
      } else if (e.op === "insert") {
        const row: (string | null)[] = columns.map(() => null);
        row[0] = String(users.length + 1);
        for (const [col, v] of e.values) row[columns.findIndex((x) => x.name === col)] = v;
        users.push(row);
      } else {
        users.splice(
          users.findIndex((r) => r[0] === e.key[0][1]),
          1,
        );
      }
      return { type: "db_affected", rows: 1 };
    }
    case "db_create_database":
      landingCreated = true;
      return { type: "ok" };
    case "redis_keys": {
      const pattern = new RegExp(`^${(req.pattern ?? "*").replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*/g, ".*")}$`);
      const keys: RedisKey[] = Object.entries(redisData)
        .filter(([k]) => pattern.test(k))
        .map(([key, v]) => ({ key, kind: v.kind, ttl: v.ttl }));
      return { type: "redis_keys", cursor: "0", keys };
    }
    case "redis_get": {
      const v = redisData[req.key];
      if (!v) throw new DaemonError("app", `no key ${req.key}`, "NOT_FOUND");
      return { type: "redis_value", key: req.key, kind: v.kind, ttl: v.ttl, items: v.items, truncated: false };
    }
    case "redis_edit": {
      const e = req.edit;
      if (e.op === "delete") delete redisData[e.key];
      else if (e.op === "set")
        redisData[e.key] = { kind: "string", ttl: redisData[e.key]?.ttl ?? -1, items: [[e.value, null]] };
      else if (redisData[e.key]) redisData[e.key].ttl = e.seconds ?? -1;
      return { type: "ok" };
    }
  }
}
