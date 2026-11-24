import { Clock, Hand, Play, RotateCw, Sparkles, Square, Zap } from "lucide-react";

import { Led, stateTone } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useDevServer } from "@/hooks/use-daemon";
import { STATE_LABEL, uptime } from "@/lib/format";
import { DEV_TOOL_LABEL, type DevServerMode, type ProjectView } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const MODE_LABEL: Record<DevServerMode, string> = {
  ask: t("Waiting for your decision"),
  manual: t("Started by hand"),
  auto: t("Starts with the site"),
  never: t("Not managed"),
};

/** The helper LocDev detected or manages, if any. */
export const helperOf = (site: ProjectView) => site.dev_server ?? site.stack.dev_server ?? null;

/**
 * Asks before running anything: LocDev never starts a project's scripts on
 * its own. Shown while the decision is pending ("ask").
 */
export function DevServerPrompt({ site }: { site: ProjectView }) {
  const { decide } = useDevServer();
  const helper = helperOf(site);
  if (!helper || (site.dev_server && site.dev_server.mode !== "ask")) return null;
  const tool = DEV_TOOL_LABEL[helper.tool];
  const choose = (mode: DevServerMode, start = false) => decide.mutate({ name: site.name, mode, start });

  return (
    <div className="rounded-lg border border-ember/35 bg-ember/6 px-4 py-3.5">
      <div className="flex items-start gap-3">
        <Sparkles className="mt-0.5 size-4 shrink-0 text-ember" />
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium">{t("{tool} detected in this project.", { tool })}</p>
          <p className="mt-1 text-[13px] text-muted-foreground">
            {tx(
              "Should LocDev run its dev server for you? It would run {command} in {site}'s folder, with the site's runtimes. Nothing runs until you choose, and you can change this later in the Server tab.",
              {
                command: (
                  <span className="font-mono text-foreground">{helper.command.replace("{port}", "<port>")}</span>
                ),
                site: <span className="font-mono">{site.domain}</span>,
              },
            )}
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button size="sm" disabled={decide.isPending} onClick={() => choose("manual", true)}>
              <Play data-icon="inline-start" className="fill-current" /> {t("Start now")}
            </Button>
            <Button size="sm" variant="outline" disabled={decide.isPending} onClick={() => choose("auto", true)}>
              <Zap data-icon="inline-start" /> {t("Start with the site")}
            </Button>
            <Button size="sm" variant="ghost" disabled={decide.isPending} onClick={() => choose("ask")}>
              <Clock data-icon="inline-start" /> {t("Ask me again")}
            </Button>
            <Button size="sm" variant="ghost" disabled={decide.isPending} onClick={() => choose("never")}>
              <Hand data-icon="inline-start" /> {t("Don't manage it")}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

/** State and controls of the helper once the user decided. */
export function DevServerControls({ site }: { site: ProjectView }) {
  const { decide, act } = useDevServer();
  const dev = site.dev_server;
  if (!dev || dev.mode === "ask") return null;
  const state = site.dev_process?.state ?? "stopped";
  const running = state === "running";
  const busy = act.isPending || state === "stopping" || dev.mode === "never";
  const run = (action: "start_dev_server" | "stop_dev_server" | "restart_dev_server") =>
    act.mutate({ action, name: site.name });

  return (
    <div className="flex flex-wrap items-center gap-3 rounded-lg border px-3.5 py-2.5">
      <Led tone={stateTone(state)} live={running} />
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2 text-sm">
          <span className="font-medium">{DEV_TOOL_LABEL[dev.tool]}</span>
          <span className="font-mono text-[11px] text-muted-foreground">
            {STATE_LABEL[state].toLowerCase()}
            {running && site.dev_process?.started_at ? ` · ${uptime(site.dev_process.started_at)}` : ""}
            {dev.port ? ` · :${dev.port}` : ""}
          </span>
        </div>
        <div className="truncate font-mono text-[11px] text-muted-foreground" title={dev.command}>
          {dev.command}
        </div>
        {site.dev_process?.message && (
          <div className="mt-0.5 text-[12px] text-destructive">{site.dev_process.message}</div>
        )}
      </div>

      <DropdownMenu>
        <DropdownMenuTrigger render={<Button variant="ghost" size="sm" className="font-normal" />}>
          {MODE_LABEL[dev.mode]}
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-56">
          {(["manual", "auto", "never"] as const).map((mode) => (
            <DropdownMenuItem
              key={mode}
              onClick={() => decide.mutate({ name: site.name, mode })}
              className={cn(dev.mode === mode && "text-ember")}
            >
              {MODE_LABEL[mode]}
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      {running ? (
        <>
          <Button size="sm" variant="outline" disabled={busy} onClick={() => run("restart_dev_server")}>
            <RotateCw data-icon="inline-start" /> {t("Restart")}
          </Button>
          <Button size="sm" variant="outline" disabled={busy} onClick={() => run("stop_dev_server")}>
            <Square data-icon="inline-start" className="fill-current" /> {t("Stop")}
          </Button>
        </>
      ) : (
        <Button size="sm" variant="outline" disabled={busy} onClick={() => run("start_dev_server")}>
          <Play data-icon="inline-start" className="fill-current" /> {t("Start")}
        </Button>
      )}
    </div>
  );
}
