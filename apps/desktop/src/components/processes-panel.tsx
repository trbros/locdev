import { Play, Plus, RotateCw, Square, Trash2 } from "lucide-react";
import { type FormEvent, useState } from "react";

import { Led, stateTone } from "@/components/status";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { errorMessage } from "@/hooks/use-daemon";
import { useAuxActions, useAuxProcesses } from "@/hooks/use-procs";
import type { AuxProcessView, DevServerMode, ProjectView } from "@/lib/protocol";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { tx } from "@/lib/i18n-jsx";

const MODES: { id: DevServerMode; label: string; title: string }[] = [
  { id: "manual", label: t("manual"), title: t("Started and stopped by hand") },
  { id: "auto", label: t("with site"), title: t("Starts together with the site") },
  { id: "never", label: t("never"), title: t("Not managed; no more questions") },
];

/**
 * A site's auxiliary processes (queue workers, schedulers, WebSocket
 * servers). Offered ones wait for a decision; nothing runs before it.
 */
export function ProcessesPanel({
  site,
  selected,
  onSelect,
}: {
  site: ProjectView;
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const procs = useAuxProcesses(site.name);
  const { control, set, remove } = useAuxActions(site.name);
  const [adding, setAdding] = useState(false);
  const list = procs.data ?? [];
  if (list.length === 0 && !adding) {
    return (
      <div className="flex items-center justify-between text-[12.5px] text-muted-foreground">
        <span>{t("No auxiliary processes offered for this stack.")}</span>
        <Button size="xs" variant="ghost" onClick={() => setAdding(true)}>
          <Plus data-icon="inline-start" /> {t("Add a process")}
        </Button>
      </div>
    );
  }

  return (
    <div className="rounded-lg border">
      <div className="flex items-center justify-between border-b px-3 py-1.5">
        <span className="font-mono text-[11px] tracking-[0.08em] text-muted-foreground uppercase">
          {t("processes")}
        </span>
        <Button size="xs" variant="ghost" onClick={() => setAdding(true)}>
          <Plus data-icon="inline-start" /> {t("Add")}
        </Button>
      </div>
      <ul className="divide-y">
        {list.map((p) => (
          <ProcessRow
            key={p.id}
            process={p}
            selected={selected === p.id}
            onSelect={() => onSelect(p.id)}
            onMode={(mode) => set.mutate({ process: p.id, mode })}
            onAction={(action) => control.mutate({ process: p.id, action })}
            onRemove={p.custom ? () => remove.mutate(p.id) : undefined}
          />
        ))}
      </ul>
      <AddProcessDialog site={site} open={adding} onClose={() => setAdding(false)} />
    </div>
  );
}

function ProcessRow({
  process: p,
  selected,
  onSelect,
  onMode,
  onAction,
  onRemove,
}: {
  process: AuxProcessView;
  selected: boolean;
  onSelect: () => void;
  onMode: (mode: DevServerMode) => void;
  onAction: (action: "start" | "stop" | "restart") => void;
  onRemove?: () => void;
}) {
  const running = p.info.state === "running";
  const decided = p.mode === "manual" || p.mode === "auto";
  return (
    <li className={cn("flex flex-col gap-1 px-3 py-2", selected && "bg-ember/4")}>
      <div className="flex items-center gap-3">
        <button type="button" onClick={onSelect} className="flex min-w-0 flex-1 items-center gap-2 text-left">
          <Led tone={stateTone(p.info.state)} live={running} />
          <span className="text-sm">{p.name}</span>
          <span className="font-mono text-[11px] text-muted-foreground">{p.kind}</span>
          {p.custom && <span className="font-mono text-[10.5px] text-amber">{t("yours")}</span>}
          <span className="truncate font-mono text-[11px] text-muted-foreground">{p.command}</span>
        </button>
        <div
          role="radiogroup"
          aria-label={t("How LocDev runs {name}", { name: p.name })}
          className="flex rounded-md border p-0.5 text-[11.5px]"
        >
          {MODES.map((m) => (
            <button
              type="button"
              role="radio"
              key={m.id}
              title={m.title}
              aria-checked={p.mode === m.id}
              onClick={() => onMode(m.id)}
              className={cn(
                "rounded px-2 py-0.5",
                p.mode === m.id ? "bg-foreground/8 text-foreground" : "text-muted-foreground hover:text-foreground",
              )}
            >
              {m.label}
            </button>
          ))}
        </div>
        {running ? (
          <>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t("Restart {name}", { name: p.name })}
              onClick={() => onAction("restart")}
            >
              <RotateCw />
            </Button>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t("Stop {name}", { name: p.name })}
              onClick={() => onAction("stop")}
            >
              <Square className="fill-current" />
            </Button>
          </>
        ) : (
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Start {name}", { name: p.name })}
            disabled={!decided}
            title={decided ? undefined : t("Choose manual or with site first")}
            onClick={() => onAction("start")}
          >
            <Play className="fill-current" />
          </Button>
        )}
        {onRemove && (
          <Button size="icon-sm" variant="ghost" aria-label={t("Remove {name}", { name: p.name })} onClick={onRemove}>
            <Trash2 />
          </Button>
        )}
      </div>
      {p.mode === "ask" && (
        <p className="pl-5 text-[12px] text-muted-foreground">
          {t("Offered by {source}; LocDev runs nothing until you choose manual or with site.", { source: p.source })}
        </p>
      )}
      {p.note && p.mode !== "never" && <p className="pl-5 text-[12px] text-muted-foreground">{p.note}</p>}
      {p.info.message && <p className="pl-5 text-[12px] text-destructive">{p.info.message}</p>}
    </li>
  );
}

function AddProcessDialog({ site, open, onClose }: { site: ProjectView; open: boolean; onClose: () => void }) {
  const { set } = useAuxActions(site.name);
  const [label, setLabel] = useState("");
  const [command, setCommand] = useState("");
  const [error, setError] = useState<string | null>(null);
  const id = label
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    try {
      await set.mutateAsync({ process: id, mode: "manual", command: command.trim(), label: label.trim() });
      setLabel("");
      setCommand("");
      setError(null);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-lg">
        <form onSubmit={submit} className="flex flex-col gap-4">
          <DialogHeader>
            <DialogTitle>{t("Add a process")}</DialogTitle>
            <DialogDescription>
              {tx("Runs in {site}'s folder with its runtimes and shared variables. {port} gets a port of its own.", {
                site: site.name,
                port: "{port}",
              })}
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="proc-label">{t("Name")}</Label>
            <Input
              id="proc-label"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              autoFocus
              placeholder={t("Mail queue")}
            />
            {id && <span className="font-mono text-[11.5px] text-muted-foreground">{t("id: {id}", { id })}</span>}
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="proc-command">{t("Command")}</Label>
            <Input
              id="proc-command"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder={t("php artisan queue:work --queue=mail")}
              className="font-mono text-xs"
            />
          </div>
          {error && <p className="text-[13px] text-destructive">{error}</p>}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onClose}>
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={!id || !command.trim() || set.isPending}>
              {t("Add")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
