import { ExternalLink, MousePointerClick, Play, Plus, Power, RotateCw, Sparkles, Square } from "lucide-react";

import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from "@/components/ui/command";
import { useDaemonControl, useProjectAction, useProjects, useStatus } from "@/hooks/use-daemon";
import { VIEWS } from "@/lib/nav";
import { ShortcutText } from "@/lib/shortcuts";
import { openInBrowser } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";

export function CommandPalette() {
  const { paletteOpen, setPaletteOpen, navigate, setAddSiteOpen, openSite } = useUi();
  const projects = useProjects();
  const status = useStatus();
  const daemon = useDaemonControl();
  const action = useProjectAction();

  const run = (fn: () => void) => {
    setPaletteOpen(false);
    fn();
  };

  return (
    <CommandDialog open={paletteOpen} onOpenChange={setPaletteOpen} className="sm:max-w-xl">
      <Command>
        <CommandInput placeholder={t("Search pages, sites and actions…")} />
        <CommandList className="max-h-96">
          <CommandEmpty>{t("Nothing found.")}</CommandEmpty>

          <CommandGroup heading="Actions">
            <CommandItem onSelect={() => run(() => setAddSiteOpen(true))}>
              <Plus /> {t("Add site")}
              <CommandShortcut>
                <ShortcutText id="add-site" />
              </CommandShortcut>
            </CommandItem>
            <CommandItem onSelect={() => run(() => navigate("tools", "new-project"))}>
              <Sparkles /> {t("New project…")}
              <CommandShortcut>
                <ShortcutText id="new-project" />
              </CommandShortcut>
            </CommandItem>
            {status.isSuccess ? (
              <>
                <CommandItem onSelect={() => run(() => daemon.restart.mutate())}>
                  <RotateCw /> {t("Restart daemon")}
                </CommandItem>
                <CommandItem onSelect={() => run(() => daemon.stop.mutate())}>
                  <Power /> {t("Stop daemon")}
                </CommandItem>
              </>
            ) : (
              <CommandItem onSelect={() => run(() => daemon.start.mutate())}>
                <Power /> {t("Start daemon")}
              </CommandItem>
            )}
          </CommandGroup>

          {!!projects.data?.length && (
            <>
              <Divider />
              <CommandGroup heading="Sites">
                {projects.data.flatMap((site) => {
                  const running = site.process.state === "running";
                  const items = [
                    running ? (
                      <CommandItem
                        key={`${site.name}-stop`}
                        value={`stop ${site.domain}`}
                        onSelect={() => run(() => action.mutate({ action: "stop_project", name: site.name }))}
                      >
                        <Square /> {t("Stop")} <span className="font-mono">{site.domain}</span>
                      </CommandItem>
                    ) : (
                      <CommandItem
                        key={`${site.name}-start`}
                        value={`start ${site.domain}`}
                        disabled={!site.command}
                        onSelect={() => run(() => action.mutate({ action: "start_project", name: site.name }))}
                      >
                        <Play /> {t("Start")} <span className="font-mono">{site.domain}</span>
                      </CommandItem>
                    ),
                    <CommandItem
                      key={`${site.name}-select`}
                      value={`show ${site.domain}`}
                      onSelect={() => run(() => openSite(site.name))}
                    >
                      <MousePointerClick /> {t("Show")} <span className="font-mono">{site.domain}</span>
                    </CommandItem>,
                  ];
                  if (running && site.url) {
                    items.unshift(
                      <CommandItem
                        key={`${site.name}-open`}
                        value={`open ${site.domain}`}
                        onSelect={() => run(() => openInBrowser(site.url!))}
                      >
                        <ExternalLink /> {t("Open")} <span className="font-mono">{site.domain}</span>
                      </CommandItem>,
                    );
                  }
                  return items;
                })}
              </CommandGroup>
            </>
          )}

          <Divider />
          <CommandGroup heading={t("Go to")}>
            {VIEWS.map((page) => (
              <CommandItem key={page.id} value={`go ${page.label}`} onSelect={() => run(() => navigate(page.id))}>
                <page.icon /> {page.label}
                <CommandShortcut>
                  <ShortcutText id={`go:${page.id}`} />
                </CommandShortcut>
              </CommandItem>
            ))}
            {VIEWS.flatMap((page) =>
              (page.subs ?? [])
                .filter((sub) => sub.id !== page.defaultSub || page.id === "tools")
                .map((sub) => (
                  <CommandItem
                    key={`${page.id}-${sub.id}`}
                    value={`go ${page.label} ${sub.label} ${sub.description}`}
                    onSelect={() => run(() => navigate(page.id, sub.id))}
                  >
                    <sub.icon /> {page.label} <span className="text-muted-foreground">›</span> {sub.label}
                  </CommandItem>
                )),
            )}
          </CommandGroup>
        </CommandList>
      </Command>
    </CommandDialog>
  );
}

/** A line between groups. Not cmdk's separator: role="separator" isn't allowed inside a listbox. */
function Divider() {
  return <div role="none" className="-mx-1 h-px bg-border" />;
}
