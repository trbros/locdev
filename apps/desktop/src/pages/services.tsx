import { Copy, Download, Play, Plus, RotateCcw, RotateCw, Square, Trash2 } from "lucide-react";
import { type FormEvent, useState } from "react";

import { Page, Section, useCrumbs } from "@/components/page";
import { Led, type Tone } from "@/components/status";
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
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { errorMessage } from "@/hooks/use-daemon";
import { useInstanceBackups, useServiceActions, useServiceReleases, useServices } from "@/hooks/use-services";
import { uptime } from "@/lib/format";
import { viewById } from "@/lib/nav";
import { type InstanceView, SERVICE_LABEL, type ServiceInfo, type ServiceKind } from "@/lib/protocol";
import { copyText } from "@/lib/system";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";
import { ServicePackages } from "@/components/insights";

function tone(i: InstanceView): [Tone, string] {
  if (i.info.state === "running") return i.ready ? ["live", t("ready")] : ["warn", t("starting")];
  if (i.info.state === "failed") return ["fail", t("failed")];
  return ["off", t("stopped")];
}

export function ServicesView() {
  const view = viewById("services");
  const crumbs = useCrumbs(view);
  const services = useServices();
  const [creating, setCreating] = useState<ServiceKind | null>(null);
  const [installing, setInstalling] = useState<ServiceKind | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const data = services.data;

  return (
    <Page
      view={view}
      crumbs={crumbs}
      wide
      actions={
        <Button
          size="sm"
          onClick={() => setCreating("postgres")}
          className="bg-ember text-ember-foreground hover:bg-ember/85"
        >
          <Plus data-icon="inline-start" /> {t("New instance")}
        </Button>
      }
    >
      {!data ? (
        <Skeleton className="h-64" />
      ) : (
        <>
          <Section title={t("instances · {length}", { length: data.instances.length })}>
            {data.instances.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t(
                  "No instances yet. Each one gets its own data folder, port, config and logs, and listens on 127.0.0.1 only.",
                )}
              </p>
            ) : (
              <div className="divide-y rounded-lg border">
                {data.instances.map((i) => (
                  <InstanceRow
                    key={i.name}
                    instance={i}
                    open={open === i.name}
                    onToggle={() => setOpen(open === i.name ? null : i.name)}
                  />
                ))}
              </div>
            )}
          </Section>

          <Section title={t("services")}>
            <div className="grid grid-cols-2 gap-3">
              {data.services.map((s) => (
                <ServiceCard
                  key={s.kind}
                  service={s}
                  onInstall={() => setInstalling(s.kind)}
                  onCreate={() => setCreating(s.kind)}
                />
              ))}
            </div>
          </Section>
        </>
      )}
      <CreateDialog kind={creating} services={data?.services ?? []} onClose={() => setCreating(null)} />
      <InstallDialog kind={installing} onClose={() => setInstalling(null)} />
    </Page>
  );
}

function ServiceCard({
  service: s,
  onInstall,
  onCreate,
}: {
  service: ServiceInfo;
  onInstall: () => void;
  onCreate: () => void;
}) {
  const { uninstall } = useServiceActions();
  return (
    <div className="flex flex-col gap-2 rounded-lg border p-3">
      <div className="flex items-center gap-2">
        <span className="font-medium">{s.name}</span>
        {s.default_port && <span className="font-mono text-[11px] text-muted-foreground">:{s.default_port}</span>}
        <span className="ml-auto flex gap-1">
          {s.kind !== "sqlite" && !s.no_download && (
            <Button size="xs" variant="ghost" onClick={onInstall}>
              <Download data-icon="inline-start" /> {t("Install")}
            </Button>
          )}
          {s.installs.length > 0 && (
            <Button size="xs" variant="outline" onClick={onCreate}>
              <Plus data-icon="inline-start" /> {t("Instance")}
            </Button>
          )}
        </span>
      </div>
      {s.installs.length === 0 ? (
        <p className="text-[12.5px] text-muted-foreground">{t("Not installed.")}</p>
      ) : (
        <ul className="font-mono text-[12px]">
          {s.installs.map((i) => (
            <li key={`${i.version}-${i.source}`} className="flex items-center gap-2">
              <span>{i.version}</span>
              <span className="text-muted-foreground">{i.source === "managed" ? t("LocDev") : t("system")}</span>
              {i.source === "managed" && (
                <button
                  type="button"
                  className="ml-auto text-muted-foreground hover:text-destructive"
                  aria-label={t("Remove {name} {version}", { name: s.name, version: i.version })}
                  onClick={() => uninstall.mutate({ kind: s.kind, version: i.version })}
                >
                  <Trash2 className="size-3.5" />
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {s.no_download && <p className="text-[12px] text-muted-foreground">{s.no_download}</p>}
      <ServicePackages service={s} />
      {s.notes.map((n) => (
        <p key={n} className="text-[12px] text-muted-foreground">
          {n}
        </p>
      ))}
    </div>
  );
}

function InstanceRow({ instance: i, open, onToggle }: { instance: InstanceView; open: boolean; onToggle: () => void }) {
  const { control, autostart, backup, restore } = useServiceActions();
  const backups = useInstanceBackups(open ? i.name : null);
  const [removing, setRemoving] = useState(false);
  const [ledTone, label] = tone(i);
  const running = i.info.state === "running";
  return (
    <div className="flex flex-col">
      <div className="flex items-center gap-3 px-3 py-2">
        <button type="button" onClick={onToggle} className="flex min-w-0 flex-1 items-center gap-3 text-left">
          <Led tone={ledTone} live={running} />
          <span className="font-mono text-[13px] font-medium">{i.name}</span>
          <span className="text-sm">
            {SERVICE_LABEL[i.kind]} {i.version}
          </span>
          <span className="font-mono text-[11.5px] text-muted-foreground">127.0.0.1:{i.port}</span>
          <span className="font-mono text-[11px] text-muted-foreground">{label}</span>
        </button>
        <label className="flex items-center gap-2 text-[12px] text-muted-foreground" title={t("Start with LocDev")}>
          <Switch
            checked={i.autostart}
            onCheckedChange={(on) => autostart.mutate({ name: i.name, autostart: on })}
            aria-label={t("Start {name} with LocDev", { name: i.name })}
            className="data-checked:bg-ember"
          />
          {t("with LocDev")}
        </label>
        {running ? (
          <>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t("Restart {name}", { name: i.name })}
              onClick={() => control.mutate({ name: i.name, action: "restart" })}
            >
              <RotateCw />
            </Button>
            <Button
              size="icon-sm"
              variant="ghost"
              aria-label={t("Stop {name}", { name: i.name })}
              onClick={() => control.mutate({ name: i.name, action: "stop" })}
            >
              <Square className="fill-current" />
            </Button>
          </>
        ) : (
          <Button
            size="icon-sm"
            variant="ghost"
            aria-label={t("Start {name}", { name: i.name })}
            onClick={() => control.mutate({ name: i.name, action: "start" })}
          >
            <Play className="fill-current" />
          </Button>
        )}
      </div>
      {i.info.message && !running && <p className="px-3 pb-2 pl-9 text-[12px] text-destructive">{i.info.message}</p>}
      {open && (
        <div className="flex flex-col gap-3 border-t bg-foreground/1.5 px-3 py-3 pl-9">
          <div>
            <p className="mb-1 font-mono text-[11px] text-muted-foreground uppercase">{t("connection")}</p>
            <ul className="font-mono text-[12px]">
              {i.connection.map(([k, v]) => (
                <li key={k} className="flex items-center gap-2">
                  <span data-selectable>
                    {k}={v}
                  </span>
                  <button
                    type="button"
                    aria-label={t("Copy {k}", { k: k })}
                    className="text-muted-foreground hover:text-foreground"
                    onClick={() => copyText(`${k}=${v}`)}
                  >
                    <Copy className="size-3" />
                  </button>
                </li>
              ))}
            </ul>
            <p className="mt-1 text-[12px] text-muted-foreground">
              {t("Local development only: listens on 127.0.0.1 and its users have no password. Data in {data_dir}.", {
                data_dir: i.data_dir,
              })}
            </p>
          </div>
          <div>
            <div className="mb-1 flex items-center gap-2">
              <p className="font-mono text-[11px] text-muted-foreground uppercase">{t("backups")}</p>
              <Button size="xs" variant="ghost" disabled={backup.isPending} onClick={() => backup.mutate(i.name)}>
                {t("Back up now")}
              </Button>
            </div>
            {(backups.data ?? []).length === 0 ? (
              <p className="text-[12px] text-muted-foreground">
                {t("None yet. A backup is a cold copy: the instance stops for a moment and starts again.")}
              </p>
            ) : (
              <ul className="font-mono text-[12px]">
                {backups.data!.map((b) => (
                  <li key={b.id} className="flex items-center gap-3">
                    <span>
                      {uptime(b.at)} {t("ago")}
                    </span>
                    <span className="text-muted-foreground">
                      {(b.size / 1_048_576).toFixed(1)} {t("MB")}
                    </span>
                    <Button size="xs" variant="ghost" onClick={() => restore.mutate({ name: i.name, id: b.id })}>
                      <RotateCcw data-icon="inline-start" /> {t("Restore")}
                    </Button>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div>
            <Button size="xs" variant="ghost" className="text-destructive" onClick={() => setRemoving(true)}>
              <Trash2 data-icon="inline-start" /> {t("Remove instance…")}
            </Button>
          </div>
        </div>
      )}
      <RemoveDialog instance={i} open={removing} onClose={() => setRemoving(false)} />
    </div>
  );
}

function RemoveDialog({ instance: i, open, onClose }: { instance: InstanceView; open: boolean; onClose: () => void }) {
  const { remove } = useServiceActions();
  const [deleteData, setDeleteData] = useState(false);
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const submit = async () => {
    try {
      await remove.mutateAsync({ name: i.name, delete_data: deleteData, confirm: deleteData ? confirm : null });
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    }
  };
  return (
    <Dialog open={open} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("Remove {name}?", { name: i.name })}</DialogTitle>
          <DialogDescription>
            {t(
              "It stops and leaves the list. Its data stays on disk unless you delete it too; backups are always kept.",
            )}
          </DialogDescription>
        </DialogHeader>
        <label className="flex items-center gap-3 text-sm">
          <Switch checked={deleteData} onCheckedChange={setDeleteData} className="data-checked:bg-destructive" />
          {t("Also delete its data (can't be undone)")}
        </label>
        {deleteData && (
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="confirm-name">{t("Type {name} to confirm", { name: i.name })}</Label>
            <Input
              id="confirm-name"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              className="font-mono"
              autoFocus
            />
          </div>
        )}
        {error && <p className="text-[13px] text-destructive">{error}</p>}
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            {t("Cancel")}
          </Button>
          <Button variant="destructive" disabled={deleteData && confirm !== i.name} onClick={submit}>
            {t("Remove")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function CreateDialog({
  kind,
  services,
  onClose,
}: {
  kind: ServiceKind | null;
  services: ServiceInfo[];
  onClose: () => void;
}) {
  const { create } = useServiceActions();
  const [service, setService] = useState<ServiceKind>(kind ?? "postgres");
  const [name, setName] = useState("");
  const [autostart, setAutostart] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const installed = services.filter((s) => s.installs.length > 0 && s.kind !== "sqlite");
  const current = kind && service !== kind && !name ? kind : service;

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    try {
      await create.mutateAsync({ name: name.trim(), kind: current, autostart });
      setName("");
      setError(null);
      onClose();
    } catch (err) {
      setError(errorMessage(err));
    }
  };

  return (
    <Dialog open={kind !== null} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <form onSubmit={submit} className="flex flex-col gap-4">
          <DialogHeader>
            <DialogTitle>{t("New instance")}</DialogTitle>
            <DialogDescription>
              {t("Its own data folder, port, config and logs. The newest installed version is used.")}
            </DialogDescription>
          </DialogHeader>
          <div role="radiogroup" aria-label={t("Service")} className="flex flex-wrap gap-1.5">
            {installed.map((s) => (
              <button
                type="button"
                role="radio"
                key={s.kind}
                aria-checked={current === s.kind}
                onClick={() => setService(s.kind)}
                className={cn(
                  "rounded-md border px-2.5 py-1 text-[12.5px]",
                  current === s.kind && "border-ember/60 bg-ember/6",
                )}
              >
                {s.name}
              </button>
            ))}
            {installed.length === 0 && <p className="text-sm text-muted-foreground">{t("Install a service first.")}</p>}
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="instance-name">{t("Name")}</Label>
            <Input
              id="instance-name"
              value={name}
              onChange={(e) => setName(e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-"))}
              placeholder={t("db")}
              className="font-mono"
              autoFocus
            />
          </div>
          <label className="flex items-center gap-3 text-sm">
            <Switch checked={autostart} onCheckedChange={setAutostart} className="data-checked:bg-ember" />
            {t("Start it now and whenever LocDev starts")}
          </label>
          {error && <p className="text-[13px] text-destructive">{error}</p>}
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={onClose}>
              {t("Cancel")}
            </Button>
            <Button type="submit" disabled={!name.trim() || create.isPending || installed.length === 0}>
              {t("Create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function InstallDialog({ kind, onClose }: { kind: ServiceKind | null; onClose: () => void }) {
  const releases = useServiceReleases(kind);
  const { install } = useServiceActions();
  return (
    <Dialog open={kind !== null} onOpenChange={(next) => !next && onClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("Install {name}", { name: kind ? SERVICE_LABEL[kind] : "" })}</DialogTitle>
          <DialogDescription>
            {t("Downloaded in the background and installed only when its SHA-256 matches the publisher's.")}
          </DialogDescription>
        </DialogHeader>
        {releases.isPending ? (
          <Skeleton className="h-24" />
        ) : releases.isError ? (
          <p className="text-sm text-destructive">{errorMessage(releases.error)}</p>
        ) : (
          <ul className="divide-y rounded-lg border">
            {releases.data!.map((r) => (
              <li key={r.version} className="flex items-center gap-3 px-3 py-2 text-sm">
                <span className="font-mono">{r.version}</span>
                <span className="truncate text-[12px] text-muted-foreground">{r.publisher}</span>
                <Button
                  size="xs"
                  variant="outline"
                  className="ml-auto"
                  onClick={() => {
                    install.mutate({ kind: r.kind, version: r.version });
                    onClose();
                  }}
                >
                  {t("Install")}
                </Button>
              </li>
            ))}
          </ul>
        )}
      </DialogContent>
    </Dialog>
  );
}
