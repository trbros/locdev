import { AlertTriangle, Bug, Play, RotateCw, Square } from "lucide-react";
import { useEffect, useState } from "react";

import { Page, Section, useCrumbs } from "@/components/page";
import { Led, stateTone } from "@/components/status";
import { Terminal } from "@/components/terminal";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useDaemonDown, useDevServer, useProcesses, useProjectAction } from "@/hooks/use-daemon";
import { STATE_LABEL, uptime } from "@/lib/format";
import { subById, viewById } from "@/lib/nav";
import type { ProcessView } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const keyOf = (p: ProcessView) => `${p.project}:${p.role}`;

/**
 * Every process LocDev supervises, with a live console. Works whether or not
 * this window was open when things happened: the daemon keeps the output.
 */
export function ProcessesView() {
  const view = viewById("tools");
  const sub = subById(view, "processes")!;
  const crumbs = useCrumbs(view, sub);
  const processes = useProcesses();
  const down = useDaemonDown();
  const [selected, setSelected] = useState<string | null>(null);

  const all = processes.data ?? [];
  // Running first, then failures, then the rest.
  const order = { running: 0, stopping: 0, failed: 1, exited: 2, stopped: 3 } as const;
  const sorted = [...all].sort((a, b) => order[a.info.state] - order[b.info.state] || keyOf(a).localeCompare(keyOf(b)));
  const current = sorted.find((p) => keyOf(p) === selected) ?? sorted[0];

  useEffect(() => {
    if (!selected && sorted[0]) setSelected(keyOf(sorted[0]));
  }, [selected, sorted]);

  const running = all.filter((p) => p.info.state === "running").length;
  const failed = all.filter((p) => p.info.state === "failed").length;

  return (
    <Page
      view={view}
      title={sub.label}
      description={sub.description}
      crumbs={crumbs}
      wide
      actions={
        <span className="font-mono text-[11px] text-muted-foreground">
          {running} {t("running")}
          {failed ? t(" · {failed} failed", { failed }) : ""}
        </span>
      }
    >
      {down ? (
        <p className="text-sm text-muted-foreground">{t("Start the daemon to see its processes.")}</p>
      ) : processes.isPending ? (
        <div className="flex flex-col gap-2">
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} className="h-10" />
          ))}
        </div>
      ) : !all.length ? (
        <p className="text-sm text-muted-foreground">{t("No sites yet, so no processes.")}</p>
      ) : (
        <>
          <ProcessTable processes={sorted} selected={current ? keyOf(current) : null} onSelect={setSelected} />
          {current && <ProcessDetail process={current} />}
        </>
      )}
    </Page>
  );
}

function ProcessTable({
  processes,
  selected,
  onSelect,
}: {
  processes: ProcessView[];
  selected: string | null;
  onSelect: (key: string) => void;
}) {
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{t("Site")}</TableHead>
          <TableHead>{t("Process")}</TableHead>
          <TableHead className="text-right">{t("PID")}</TableHead>
          <TableHead className="text-right">{t("Port")}</TableHead>
          <TableHead>{t("Runtime")}</TableHead>
          <TableHead>{t("Status")}</TableHead>
          <TableHead className="text-right">{t("Restarts")}</TableHead>
          <TableHead className="text-right">{t("CPU")}</TableHead>
          <TableHead className="text-right">{t("Memory")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {processes.map((p) => {
          const live = p.info.state === "running";
          return (
            <TableRow
              key={keyOf(p)}
              onClick={() => onSelect(keyOf(p))}
              data-state={selected === keyOf(p) ? "selected" : undefined}
              className="cursor-pointer font-mono text-[12px] data-[state=selected]:bg-ember/[0.07]"
            >
              <TableCell>
                <span className="flex items-center gap-2">
                  <Led tone={stateTone(p.info.state)} live={live} />
                  {p.project}
                </span>
              </TableCell>
              <TableCell>
                {p.label}
                {p.role === "dev" && <span className="text-muted-foreground"> {t("· helper")}</span>}
              </TableCell>
              <TableCell className="text-right tabular-nums">{live ? p.info.pid : "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{p.port ?? "—"}</TableCell>
              <TableCell className="max-w-40 truncate text-muted-foreground" title={p.runtimes.join(", ")}>
                {p.runtimes.join(" · ") || t("PATH")}
              </TableCell>
              <TableCell className={cn(p.info.state === "failed" && "text-destructive")}>
                {live && p.info.started_at
                  ? t("up {time}", { time: uptime(p.info.started_at) })
                  : STATE_LABEL[p.info.state].toLowerCase()}
              </TableCell>
              <TableCell className="text-right tabular-nums">{p.info.restarts || "—"}</TableCell>
              <TableCell className="text-right tabular-nums">{p.cpu != null ? `${p.cpu.toFixed(1)}%` : "—"}</TableCell>
              <TableCell className="text-right tabular-nums">
                {p.memory != null ? `${Math.round(p.memory / 1e6)} MB` : "—"}
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}

function ProcessDetail({ process }: { process: ProcessView }) {
  const action = useProjectAction();
  const { act } = useDevServer();
  const { openSite } = useUi();
  const live = process.info.state === "running";
  const busy = action.isPending || act.isPending || process.info.state === "stopping";
  const run = (kind: "start" | "stop" | "restart") =>
    process.role === "main"
      ? action.mutate({ action: `${kind}_project`, name: process.project })
      : act.mutate({ action: `${kind}_dev_server`, name: process.project });

  return (
    <Section
      title={`${process.project} · ${process.label}${process.role === "dev" ? t(" (helper)") : ""}`}
      aside={
        <span className="flex items-center gap-1.5">
          <Button variant="ghost" size="xs" onClick={() => openSite(process.project, "server")}>
            {t("open site")}
          </Button>
          {live ? (
            <>
              <Button variant="outline" size="xs" disabled={busy} onClick={() => run("restart")}>
                <RotateCw data-icon="inline-start" /> {t("Restart")}
              </Button>
              <Button variant="outline" size="xs" disabled={busy} onClick={() => run("stop")}>
                <Square data-icon="inline-start" className="fill-current" /> {t("Stop")}
              </Button>
            </>
          ) : (
            <Button variant="outline" size="xs" disabled={busy || !process.command} onClick={() => run("start")}>
              <Play data-icon="inline-start" className="fill-current" /> {t("Start")}
            </Button>
          )}
        </span>
      }
    >
      {process.command && (
        <p
          className="mb-2 truncate font-mono text-[12px] text-muted-foreground"
          data-selectable
          title={process.command}
        >
          <span className="text-ember">$</span> {process.command}
        </p>
      )}
      <Diagnosis process={process} />
      <div className="flex h-105 min-h-0 flex-col">
        <Terminal
          name={process.project}
          process={process.role}
          running={live}
          logFile={process.log_file}
          label={process.role === "dev" ? process.label.toLowerCase() : "output"}
        />
      </div>
      <p className="mt-2 flex items-center gap-1.5 text-[12px] text-muted-foreground">
        <Bug className="size-3.5" />{" "}
        {t(
          "This is the process's output, kept by the daemon even with this window closed. Interactive debugging (Xdebug, the Node inspector) isn't part of LocDev yet.",
        )}
      </p>
    </Section>
  );
}

/** Explains a failure from what the daemon recorded; no guessing beyond it. */
function Diagnosis({ process }: { process: ProcessView }) {
  const { state, exit_code, message } = process.info;
  if (state !== "failed") return null;
  const hints: string[] = [];
  if (message?.includes("already in use")) {
    hints.push(t("Another program holds the port. Stop it, or give the site another port in its Settings."));
  }
  if (exit_code === 127) {
    hints.push(
      t("The command wasn't found. Are the project's dependencies installed (npm install, composer install)?"),
    );
  }
  if (message?.startsWith("gave up")) {
    hints.push(t("Restart on failure stopped trying; fix the error in the output, then start it again."));
  }
  return (
    <div className="mb-3 flex gap-2.5 rounded-lg border border-destructive/30 bg-destructive/[0.07] px-3.5 py-2.5 text-sm">
      <AlertTriangle className="mt-0.5 size-4 shrink-0 text-destructive" />
      <div className="min-w-0">
        <div className="text-destructive">
          {message ?? t("The process failed")}
          {exit_code != null && (
            <span className="font-mono text-[12px]"> {t("(exit code {exit_code})", { exit_code })}</span>
          )}
        </div>
        {hints.map((hint) => (
          <div key={hint} className="mt-0.5 text-[13px] text-muted-foreground">
            {hint}
          </div>
        ))}
      </div>
    </div>
  );
}
