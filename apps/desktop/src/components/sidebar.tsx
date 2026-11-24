import { ChevronDown, Command, type LucideIcon, Moon, PanelLeftClose, PanelLeftOpen, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { type KeyboardEvent, type ReactNode, useState } from "react";

import { LogoMark } from "@/components/logo";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useNotifications, useProjects } from "@/hooks/use-daemon";
import { VIEWS, type ViewDef } from "@/lib/nav";
import { useShortcutLabel } from "@/lib/shortcuts";
import { useMedia, usePrefs } from "@/lib/prefs";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const GROUPS: { id: ViewDef["group"]; label: string }[] = [
  { id: "workspace", label: t("Workspace") },
  { id: "tools", label: t("Tools") },
  { id: "system", label: t("System") },
];

/**
 * The window's left edge: icons only, or icons and names. The choice is
 * remembered; narrow windows always get the compact form.
 */
export function Sidebar() {
  const { sidebar, set } = usePrefs();
  const narrow = useMedia("(max-width: 1000px)");
  const expanded = sidebar === "expanded" && !narrow;
  const { setPaletteOpen } = useUi();
  const { resolvedTheme, setTheme } = useTheme();
  const paletteKeys = useShortcutLabel("palette");
  const sidebarKeys = useShortcutLabel("sidebar");

  return (
    <nav
      aria-label={t("Main")}
      data-expanded={expanded || undefined}
      onKeyDown={moveFocus}
      className={cn(
        "flex shrink-0 flex-col border-r bg-chrome py-3 transition-[width] duration-200 ease-out motion-reduce:transition-none",
        expanded ? "w-56 px-2.5" : "w-15 items-center px-0",
      )}
    >
      <div className={cn("mb-3 flex h-9 items-center gap-2.5", expanded && "px-1.5")}>
        <LogoMark className="size-8 shrink-0" />
        {expanded && <span className="truncate text-[15px] font-semibold tracking-[-0.02em]">{t("LocDev")}</span>}
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-y-auto">
        {GROUPS.map((group, gi) => {
          const views = VIEWS.filter((v) => v.group === group.id);
          return (
            <div key={group.id} role="group" aria-label={group.label} className="flex flex-col gap-0.5">
              {gi > 0 && <span aria-hidden className={cn("my-2 h-px bg-border", expanded ? "mx-2" : "mx-auto w-6")} />}
              {views.map((view) => (
                <NavEntry key={view.id} view={view} expanded={expanded} />
              ))}
            </div>
          );
        })}
      </div>

      <div className={cn("mt-2 flex flex-col gap-0.5", !expanded && "items-center")}>
        <Utility
          expanded={expanded}
          icon={Command}
          label={t("Commands")}
          shortcut={paletteKeys ?? undefined}
          onClick={() => setPaletteOpen(true)}
        />
        <Utility
          expanded={expanded}
          icon={resolvedTheme === "dark" ? Sun : Moon}
          label={resolvedTheme === "dark" ? t("Light theme") : t("Dark theme")}
          onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
        />
        {!narrow && (
          <Utility
            expanded={expanded}
            icon={expanded ? PanelLeftClose : PanelLeftOpen}
            label={expanded ? t("Collapse sidebar") : t("Expand sidebar")}
            shortcut={sidebarKeys ?? undefined}
            onClick={() => set("sidebar", expanded ? "collapsed" : "expanded")}
          />
        )}
      </div>
    </nav>
  );
}

/** Arrow keys move between the sidebar's buttons. */
function moveFocus(e: KeyboardEvent<HTMLElement>) {
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
  const items = [...e.currentTarget.querySelectorAll<HTMLElement>("button:not(:disabled)")];
  const at = items.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  e.preventDefault();
  items[(at + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length]?.focus();
}

function NavEntry({ view, expanded }: { view: ViewDef; expanded: boolean }) {
  const { view: current, sub, navigate } = useUi();
  const projects = useProjects();
  const unread = useNotifications(false).data?.unread ?? 0;
  const active = current === view.id;
  const running = projects.data?.filter((p) => p.process.state === "running").length ?? 0;
  const badge =
    view.id === "sites" && running > 0
      ? String(running)
      : view.id === "notifications" && unread > 0
        ? String(unread)
        : undefined;
  const shortcut = useShortcutLabel(`go:${view.id}`) ?? undefined;
  // Only Tools lists its subpages here: it is a toolbox, not one page.
  const submenu = view.id === "tools" ? view.subs : undefined;
  const [open, setOpen] = useState(true);

  if (submenu && !expanded) {
    return (
      <DropdownMenu>
        <Tooltip>
          <TooltipTrigger
            render={
              <DropdownMenuTrigger
                render={<NavButton active={active} expanded={false} label={view.label} icon={view.icon} />}
              />
            }
          />
          <TooltipContent side="right">{view.label}</TooltipContent>
        </Tooltip>
        <DropdownMenuContent side="right" align="start" className="w-52">
          <DropdownMenuGroup>
            <DropdownMenuLabel>{view.label}</DropdownMenuLabel>
            {submenu.map((s) => (
              <DropdownMenuItem key={s.id} onClick={() => navigate(view.id, s.id)}>
                <s.icon /> {s.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuGroup>
        </DropdownMenuContent>
      </DropdownMenu>
    );
  }

  const button = (
    <NavButton
      active={active && !(submenu && expanded)}
      expanded={expanded}
      label={view.label}
      icon={view.icon}
      badge={badge}
      soon={view.soon}
      trailing={
        submenu ? (
          <ChevronDown className={cn("size-3.5 transition-transform", !open && "-rotate-90")} />
        ) : expanded && !view.soon ? (
          <span className="font-mono text-[10px] text-muted-foreground/60 opacity-0 transition-opacity group-hover/nav:opacity-100">
            {shortcut}
          </span>
        ) : undefined
      }
      onClick={() => (submenu ? setOpen(!open) : navigate(view.id))}
      ariaExpanded={submenu ? open : undefined}
    />
  );

  return (
    <>
      {expanded ? (
        button
      ) : (
        <Tooltip>
          <TooltipTrigger render={button} />
          <TooltipContent side="right" className="flex items-center gap-2">
            {view.label}
            <span className="font-mono text-[10px] opacity-70">{view.soon ?? shortcut}</span>
          </TooltipContent>
        </Tooltip>
      )}
      {submenu && expanded && open && (
        <div className="ml-4.75 flex flex-col gap-0.5 border-l pl-2">
          {submenu.map((s) => (
            <NavButton
              key={s.id}
              small
              expanded
              active={active && (sub ?? view.defaultSub) === s.id}
              label={s.label}
              icon={s.icon}
              onClick={() => navigate(view.id, s.id)}
            />
          ))}
        </div>
      )}
    </>
  );
}

function NavButton({
  active,
  expanded,
  label,
  icon: Icon,
  badge,
  soon,
  trailing,
  small,
  onClick,
  ariaExpanded,
  ...rest
}: {
  active: boolean;
  expanded: boolean;
  label: string;
  icon: LucideIcon;
  badge?: string;
  soon?: string;
  trailing?: ReactNode;
  small?: boolean;
  onClick?: () => void;
  ariaExpanded?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={expanded ? undefined : label}
      aria-current={active ? "page" : undefined}
      aria-expanded={ariaExpanded}
      {...rest}
      className={cn(
        "group/nav relative flex shrink-0 items-center rounded-lg text-muted-foreground transition-colors hover:bg-foreground/6 hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring",
        expanded ? "w-full gap-2.5 px-2" : "size-10 justify-center",
        expanded && (small ? "h-7 text-[13px]" : "h-9 text-sm"),
        active && "bg-foreground/[0.07] text-foreground",
        soon && "opacity-60",
      )}
    >
      {/* The active marker sits on the window edge, like a slot indicator. */}
      {active && !small && (
        <span
          className={cn("absolute top-2 bottom-2 w-0.75 rounded-r bg-ember", expanded ? "-left-2.5" : "-left-2.5")}
        />
      )}
      <Icon className={cn("shrink-0", small ? "size-3.75" : "size-4.5")} strokeWidth={1.75} />
      {expanded && <span className="min-w-0 flex-1 truncate text-left">{label}</span>}
      {expanded && soon && <span className="font-mono text-[10px]">{soon}</span>}
      {badge &&
        (expanded ? (
          <span
            className={cn(
              "rounded-full px-1.5 font-mono text-[10px] tabular-nums",
              label === t("Notifications") ? "bg-ember/15 text-ember" : "bg-phosphor/15 text-phosphor",
            )}
          >
            {badge}
          </span>
        ) : (
          <span
            className={cn(
              "absolute top-1.5 right-1.5 size-1.5 rounded-full",
              label === t("Notifications") ? "bg-ember" : "bg-phosphor",
            )}
          />
        ))}
      {expanded && trailing}
    </button>
  );
}

function Utility({
  expanded,
  icon: Icon,
  label,
  shortcut,
  onClick,
}: {
  expanded: boolean;
  icon: LucideIcon;
  label: string;
  shortcut?: string;
  onClick: () => void;
}) {
  const button = (
    <button
      type="button"
      onClick={onClick}
      // Collapsed, the icon needs a name; expanded, the visible text is it.
      aria-label={expanded ? undefined : label}
      aria-keyshortcuts={shortcut}
      className={cn(
        "flex shrink-0 items-center rounded-lg text-muted-foreground transition-colors hover:bg-foreground/6 hover:text-foreground [&_svg]:size-4",
        expanded ? "h-8 w-full gap-2.5 px-2 text-[13px]" : "size-9 justify-center",
      )}
    >
      <Icon />
      {expanded && <span className="flex-1 truncate text-left">{label}</span>}
      {expanded && shortcut && (
        <span aria-hidden className="font-mono text-[10px] text-muted-foreground/60">
          {shortcut}
        </span>
      )}
    </button>
  );
  if (expanded) return button;
  return (
    <Tooltip>
      <TooltipTrigger render={button} />
      <TooltipContent side="right">
        {label}
        {shortcut && <span className="ml-2 font-mono text-[10px] opacity-70">{shortcut}</span>}
      </TooltipContent>
    </Tooltip>
  );
}
