// Auxiliary processes for the mock backend; the real rules live in the
// daemon (`daemon/procs.rs`).

import { DaemonError } from "./errors";
import type { AuxProcessView, ProcsRequest, ProjectView } from "./protocol";

type Log = (key: string, text: string, stream?: "stdout" | "stderr" | "system") => void;

const running = new Map<string, number>();

function views(p: ProjectView): AuxProcessView[] {
  const offers = p.stack.processes.map((o) => {
    const choice = p.processes?.[o.id];
    return {
      id: o.id,
      name: o.name,
      kind: o.kind,
      command: choice?.command ?? o.command,
      offered: o.command,
      custom: false,
      source: o.source,
      mode: choice?.mode ?? "ask",
      port: choice?.port ?? null,
      conflicts: o.conflicts,
      note: o.note,
      info: info(p.name, o.id),
    } satisfies AuxProcessView;
  });
  const custom = Object.entries(p.processes ?? {})
    .filter(([id, c]) => !p.stack.processes.some((o) => o.id === id) && c.command)
    .map(
      ([id, c]) =>
        ({
          id,
          name: c.label ?? id,
          kind: "other",
          command: c.command!,
          offered: null,
          custom: true,
          source: null,
          mode: c.mode,
          port: c.port ?? null,
          conflicts: [],
          note: null,
          info: info(p.name, id),
        }) satisfies AuxProcessView,
    );
  return [...offers, ...custom];
}

function info(site: string, id: string): AuxProcessView["info"] {
  const at = running.get(`${site}:${id}`);
  return at
    ? { state: "running", pid: 51000 + (at % 999), started_at: at, exit_code: null, message: null, restarts: 0 }
    : { state: "stopped", pid: null, started_at: null, exit_code: null, message: null, restarts: 0 };
}

export function auxRunning(site: string): string[] {
  return [...running.keys()].filter((k) => k.startsWith(`${site}:`)).map((k) => k.slice(site.length + 1));
}

export function stopAux(site: string) {
  for (const key of [...running.keys()]) if (key.startsWith(`${site}:`)) running.delete(key);
}

export function handleProcs(req: ProcsRequest, find: (name: string) => ProjectView, log: Log) {
  const p = find(req.name);
  const start = (id: string) => {
    const view = views(p).find((v) => v.id === id);
    if (!view) throw new DaemonError("app", `${p.name} has no process \`${id}\``, "NOT_FOUND");
    if (view.mode === "ask" || view.mode === "never") {
      throw new DaemonError("app", `LocDev doesn't run ${view.name} for ${p.name} until you decide`, "NOT_RUNNABLE");
    }
    const other = view.conflicts.find((c) => running.has(`${p.name}:${c}`));
    if (other) {
      throw new DaemonError("app", `${other} is running and would consume the same jobs; stop it first`, "CONFLICT");
    }
    running.set(`${p.name}:${id}`, Date.now());
    log(`${p.name}:${id}`, `$ ${view.command}`, "system");
    log(
      `${p.name}:${id}`,
      id === "scheduler"
        ? "INFO  Running scheduled tasks every minute."
        : "INFO  Processing jobs from the [default] queue.",
    );
  };
  switch (req.type) {
    case "project_processes":
      break;
    case "control_process":
      if (req.action !== "start") running.delete(`${p.name}:${req.process}`);
      if (req.action !== "stop") start(req.process);
      break;
    case "set_process": {
      p.processes ??= {};
      const entry = (p.processes[req.process] ??= { mode: "ask" });
      if (req.mode) entry.mode = req.mode;
      if (req.command !== undefined && req.command !== null) entry.command = req.command || undefined;
      if (req.label) entry.label = req.label;
      break;
    }
    case "remove_process":
      running.delete(`${p.name}:${req.process}`);
      delete p.processes?.[req.process];
      break;
  }
  return { type: "project_processes" as const, processes: views(p) };
}
