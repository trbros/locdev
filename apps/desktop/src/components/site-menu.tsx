import {
  AppWindow,
  Code2,
  Copy,
  ExternalLink,
  FolderOpen,
  Link2,
  type LucideIcon,
  Pencil,
  Play,
  RotateCw,
  Share2,
  Square,
  Trash2,
} from "lucide-react";
import { Fragment, type ReactNode } from "react";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuGroup,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { DropdownMenuItem, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { useProjectAction } from "@/hooks/use-daemon";
import { useDefaultEditorName, useOpenInEditor } from "@/hooks/use-editors";
import type { ProjectView } from "@/lib/protocol";
import { copyText, openInBrowser, revealFolder } from "@/lib/system";
import { useUi } from "@/lib/ui-state";
import { t } from "@/lib/i18n";

interface SiteAction {
  id: string;
  label: string;
  icon: LucideIcon;
  run: () => void;
  disabled?: boolean;
  destructive?: boolean;
}

/**
 * Every action available on a site, grouped. The single source for the ⋯
 * dropdown and the right-click menu, so both always offer the same things.
 */
export function useSiteActions(site: ProjectView): SiteAction[][] {
  const action = useProjectAction();
  const editor = useDefaultEditorName();
  const openEditor = useOpenInEditor();
  const { setEditing, setRemoving, tabs, openInNewTab, setSharing } = useUi();
  const state = site.process.state;
  const running = state === "running";
  const busy = action.isPending || state === "stopping";
  const run = (kind: "start_project" | "stop_project" | "restart_project") =>
    action.mutate({ action: kind, name: site.name });

  return [
    [
      {
        id: "open",
        label: t("Open in browser"),
        icon: ExternalLink,
        run: () => site.url && openInBrowser(site.url),
        disabled: !running || !site.url,
      },
      running
        ? { id: "stop", label: t("Stop"), icon: Square, run: () => run("stop_project"), disabled: busy }
        : {
            id: "start",
            label: t("Start"),
            icon: Play,
            run: () => run("start_project"),
            disabled: busy || !site.command,
          },
      {
        id: "restart",
        label: t("Restart"),
        icon: RotateCw,
        run: () => run("restart_project"),
        disabled: busy || !running,
      },
      {
        id: "share",
        label: t("Share on the internet…"),
        icon: Share2,
        run: () => setSharing(site.name),
        disabled: !running || !site.port,
      },
    ],
    [
      {
        id: "new-tab",
        label: t("Open in new tab"),
        icon: AppWindow,
        run: () => openInNewTab(site.name),
        disabled: tabs.some((tab) => tab.site === site.name),
      },
      { id: "edit", label: t("Edit settings…"), icon: Pencil, run: () => setEditing(site.name) },
      {
        id: "editor",
        label: editor ? t("Open in {editor}", { editor }) : t("Open in editor"),
        icon: Code2,
        run: () => openEditor.mutate({ site: site.name }),
        disabled: !editor,
      },
      { id: "reveal", label: t("Reveal folder"), icon: FolderOpen, run: () => revealFolder(site.path) },
    ],
    [
      {
        id: "copy-url",
        label: t("Copy URL"),
        icon: Link2,
        run: () => site.url && copyText(site.url, "URL copied"),
        disabled: !site.url,
      },
      { id: "copy-path", label: t("Copy path"), icon: Copy, run: () => copyText(site.path, t("Path copied")) },
    ],
    [
      {
        id: "remove",
        label: t("Remove from LocDev…"),
        icon: Trash2,
        run: () => setRemoving(site.name),
        destructive: true,
      },
    ],
  ];
}

/** Renders the site actions as items of the ⋯ dropdown. */
export function SiteDropdownItems({ site }: { site: ProjectView }) {
  const groups = useSiteActions(site);
  return groups.map((group, i) => (
    <Fragment key={group[0].id}>
      {i > 0 && <DropdownMenuSeparator />}
      {group.map((item) => (
        <DropdownMenuItem
          key={item.id}
          disabled={item.disabled}
          variant={item.destructive ? "destructive" : "default"}
          onClick={item.run}
        >
          <item.icon /> {item.label}
        </DropdownMenuItem>
      ))}
    </Fragment>
  ));
}

/** Wraps an element so right-clicking it opens the site's actions. */
export function SiteContextMenu({ site, children }: { site: ProjectView; children: ReactNode }) {
  return (
    <ContextMenu>
      {/* `group/ctx` lets the wrapped element highlight while its menu is open. */}
      <ContextMenuTrigger className="group/ctx">{children}</ContextMenuTrigger>
      <ContextMenuContent className="w-56">
        <SiteContextItems site={site} />
      </ContextMenuContent>
    </ContextMenu>
  );
}

function SiteContextItems({ site }: { site: ProjectView }) {
  const groups = useSiteActions(site);
  return (
    <>
      {/* Base UI labels must sit inside a group. */}
      <ContextMenuGroup>
        <ContextMenuLabel className="truncate font-mono">{site.domain}</ContextMenuLabel>
      </ContextMenuGroup>
      {groups.map((group) => (
        <Fragment key={group[0].id}>
          <ContextMenuSeparator />
          {group.map((item) => (
            <ContextMenuItem
              key={item.id}
              disabled={item.disabled}
              variant={item.destructive ? "destructive" : "default"}
              onClick={item.run}
            >
              <item.icon /> {item.label}
            </ContextMenuItem>
          ))}
        </Fragment>
      ))}
    </>
  );
}
