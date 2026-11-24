// Services for the mock backend; the real logic is `locdev-services` and
// `daemon/services.rs`.

import { DaemonError } from "./errors";
import type {
  InstanceBackup,
  InstanceView,
  ServiceInfo,
  ServiceKind,
  ServicesRequest,
  ServicesResponse,
} from "./protocol";
import { SERVICE_LABEL } from "./protocol";

const now = Date.now();

/** What Homebrew offers (macOS, the mock's platform). */
const PACKAGES: Partial<Record<ServiceKind, [string, string][]>> = {
  mysql: [
    ["mysql", "latest"],
    ["mysql@8.4", "8.4 LTS"],
    ["mysql@8.0", "8.0 (deprecated by Homebrew)"],
  ],
  redis: [
    ["redis", "latest"],
    ["redis@8.2", "8.2"],
  ],
  postgres: [
    ["postgresql@17", "17"],
    ["postgresql@16", "16"],
  ],
};

const NO_DOWNLOAD: Partial<Record<ServiceKind, string>> = {
  mysql: "Oracle publishes MD5 sums only, which LocDev doesn't accept; use a system MySQL or MariaDB",
  redis: "Redis publishes no prebuilt binaries; use a system Redis, or Valkey (a compatible fork LocDev can install)",
  sqlite: "SQLite is a file; there is nothing to install or run",
};

const installs: ServiceInfo["installs"] = [
  { kind: "postgres", version: "17.4", source: "system", bin_dir: "/usr/lib/postgresql/17/bin" },
  {
    kind: "postgres",
    version: "18.6.1",
    source: "managed",
    bin_dir: "~/.local/share/locdev/services/bin/postgres/18.6.1/bin",
  },
  {
    kind: "valkey",
    version: "9.1.3",
    source: "managed",
    bin_dir: "~/.local/share/locdev/services/bin/valkey/9.1.3/bin",
  },
];

const stopped = { state: "stopped", pid: null, started_at: null, exit_code: null, message: null, restarts: 0 } as const;

function connection(kind: ServiceKind, port: number): [string, string][] {
  if (kind === "postgres")
    return [
      ["DATABASE_URL", `postgresql://locdev@127.0.0.1:${port}/postgres`],
      ["DB_CONNECTION", "pgsql"],
      ["DB_HOST", "127.0.0.1"],
      ["DB_PORT", String(port)],
      ["DB_USERNAME", "locdev"],
    ];
  if (kind === "valkey" || kind === "redis")
    return [
      ["REDIS_URL", `redis://127.0.0.1:${port}`],
      ["REDIS_HOST", "127.0.0.1"],
      ["REDIS_PORT", String(port)],
    ];
  return [["DATABASE_URL", `${kind}://127.0.0.1:${port}`]];
}

const instance = (name: string, kind: ServiceKind, version: string, port: number, running: boolean): InstanceView => ({
  name,
  kind,
  version,
  source: "managed",
  port,
  autostart: running,
  created_at: now - 86_400_000,
  info: running
    ? { state: "running", pid: 61234, started_at: now - 3_600_000, exit_code: null, message: null, restarts: 0 }
    : { ...stopped },
  data_dir: `~/.local/share/locdev/services/instances/${name}/data`,
  ready: running,
  connection: connection(kind, port),
  log_file: null,
});

const instances: InstanceView[] = [
  instance("db", "postgres", "18.6.1", 5433, true),
  instance("cache", "valkey", "9.1.3", 6380, false),
];
const backups: InstanceBackup[] = [];

export function handleServices(req: ServicesRequest): ServicesResponse | { type: "ok" } {
  const find = (name: string) => {
    const i = instances.find((i) => i.name === name);
    if (!i) throw new DaemonError("app", `no service instance named ${name}`, "NOT_FOUND");
    return i;
  };
  const list = (): ServicesResponse => ({
    type: "services",
    services: (["postgres", "mariadb", "mysql", "mongodb", "valkey", "redis", "sqlite"] as ServiceKind[]).map(
      (kind) => ({
        kind,
        name: SERVICE_LABEL[kind],
        default_port: {
          postgres: 5432,
          mariadb: 3306,
          mysql: 3306,
          mongodb: 27017,
          valkey: 6379,
          redis: 6379,
          sqlite: null,
        }[kind],
        installs: installs.filter((i) => i.kind === kind),
        no_download: NO_DOWNLOAD[kind] ?? null,
        notes: [],
        packages: (PACKAGES[kind] ?? []).map(([id, version]) => ({
          id,
          version,
          manager: "Homebrew",
          commands: [["brew", "install", id]],
        })),
      }),
    ),
    instances: structuredClone(instances),
  });
  switch (req.type) {
    case "list_services":
      return list();
    case "service_releases":
      if (NO_DOWNLOAD[req.kind]) throw new DaemonError("app", NO_DOWNLOAD[req.kind]!, "REMOTE");
      return {
        type: "service_releases",
        releases: (req.kind === "postgres"
          ? ["18.6.1", "17.10.1", "16.14.1"]
          : req.kind === "valkey"
            ? ["9.1.3", "8.1.6"]
            : ["8.0.17"]
        ).map((version) => ({
          kind: req.kind,
          version,
          publisher:
            req.kind === "postgres"
              ? "PostgreSQL source, built by theseus-rs/postgresql-binaries"
              : SERVICE_LABEL[req.kind],
          url: "",
        })),
      };
    case "install_service":
      installs.push({ kind: req.kind, version: req.version, source: "managed", bin_dir: "" });
      throw new DaemonError("app", "The mock backend doesn't download; the version was added to the list.", "REMOTE");
    case "uninstall_service":
      return list();
    case "create_instance": {
      if (instances.some((i) => i.name === req.name)) {
        throw new DaemonError("app", `an instance named ${req.name} exists`, "ALREADY_EXISTS");
      }
      const install = installs.find((i) => i.kind === req.kind);
      if (!install) throw new DaemonError("app", `no ${SERVICE_LABEL[req.kind]} installed`, "NOT_RUNNABLE");
      const port =
        req.port ??
        { postgres: 5434, mariadb: 3306, mysql: 3306, mongodb: 27017, valkey: 6381, redis: 6381, sqlite: 0 }[req.kind];
      const created = instance(req.name, req.kind, install.version, port, !!req.autostart);
      instances.push(created);
      return { type: "instance", ...created };
    }
    case "control_instance": {
      const i = find(req.name);
      const on = req.action !== "stop";
      i.info = on
        ? { state: "running", pid: 62000, started_at: Date.now(), exit_code: null, message: null, restarts: 0 }
        : { ...stopped };
      i.ready = on;
      return { type: "instance", ...structuredClone(i) };
    }
    case "set_instance": {
      const i = find(req.name);
      i.autostart = req.autostart;
      return { type: "instance", ...structuredClone(i) };
    }
    case "remove_instance": {
      find(req.name);
      if (req.delete_data && req.confirm !== req.name) {
        throw new DaemonError(
          "app",
          `deleting the data can't be undone: type the instance's name (${req.name}) to confirm`,
          "UNAUTHORIZED",
        );
      }
      instances.splice(
        instances.findIndex((i) => i.name === req.name),
        1,
      );
      return list();
    }
    case "backup_instance": {
      const b = { id: String(Date.now()), instance: req.name, at: Date.now(), size: 6_436_537, path: "" };
      backups.unshift(b);
      return { type: "instance_backup", ...b };
    }
    case "instance_backups":
      return { type: "instance_backups", backups: backups.filter((b) => b.instance === req.name) };
    case "restore_instance":
      return { type: "instance", ...structuredClone(find(req.name)) };
    case "instance_logs":
      return { type: "ok" };
    default:
      throw new DaemonError("app", `mock-services: ${req.type}`, "NOT_FOUND");
  }
}
