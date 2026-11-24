import { ChevronLeft, ChevronRight } from "lucide-react";
import { Fragment, type ReactNode } from "react";

import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { ScrollArea } from "@/components/ui/scroll-area";
import { type SubDef, type ViewDef } from "@/lib/nav";
import { useShortcutLabel } from "@/lib/shortcuts";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

export interface Crumb {
  label: string;
  onClick?: () => void;
}

/** Back/forward and the trail to the current page. */
export function Breadcrumbs({ crumbs }: { crumbs: Crumb[] }) {
  const { back, forward, canBack, canForward } = useUi();

  return (
    <div className="flex items-center gap-2">
      <div className="flex items-center">
        <HistoryButton label={t("Back · Alt ←")} disabled={!canBack} onClick={back}>
          <ChevronLeft />
        </HistoryButton>
        <HistoryButton label={t("Forward · Alt →")} disabled={!canForward} onClick={forward}>
          <ChevronRight />
        </HistoryButton>
      </div>
      <Breadcrumb>
        <BreadcrumbList className="font-mono text-[11px]">
          {crumbs.map((crumb, i) => (
            <Fragment key={crumb.label}>
              {i > 0 && <BreadcrumbSeparator />}
              <BreadcrumbItem>
                {i === crumbs.length - 1 || !crumb.onClick ? (
                  <BreadcrumbPage className={cn("font-mono", i === crumbs.length - 1 && "text-ember")}>
                    {crumb.label}
                  </BreadcrumbPage>
                ) : (
                  <BreadcrumbLink render={<button type="button" onClick={crumb.onClick} />}>
                    {crumb.label}
                  </BreadcrumbLink>
                )}
              </BreadcrumbItem>
            </Fragment>
          ))}
        </BreadcrumbList>
      </Breadcrumb>
    </div>
  );
}

function HistoryButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className="flex size-6 items-center justify-center rounded text-muted-foreground transition-colors enabled:hover:bg-foreground/6 enabled:hover:text-foreground disabled:opacity-35 [&_svg]:size-3.5"
    >
      {children}
    </button>
  );
}

/** The trail for a page (and its subpage), each step clickable. */
export function useCrumbs(view: ViewDef, sub?: SubDef, extra: Crumb[] = []): Crumb[] {
  const { navigate } = useUi();
  const crumbs: Crumb[] = [{ label: view.label, onClick: () => navigate(view.id) }];
  if (sub) crumbs.push({ label: sub.label, onClick: () => navigate(view.id, sub.id) });
  return [...crumbs, ...extra];
}

/** Title block shared by every page: breadcrumbs, title, description, actions. */
export function ViewHeading({
  view,
  title,
  description,
  crumbs,
  actions,
}: {
  view: ViewDef;
  title?: ReactNode;
  description?: ReactNode;
  crumbs?: Crumb[];
  actions?: ReactNode;
}) {
  const fallback = useCrumbs(view);
  return (
    <header className="flex flex-col gap-3">
      <Breadcrumbs crumbs={crumbs ?? fallback} />
      <div className="flex items-end justify-between gap-4">
        <div className="min-w-0">
          <h1 className="text-[26px] leading-none font-semibold tracking-[-0.02em]">{title ?? view.label}</h1>
          <p className="mt-2 text-sm text-muted-foreground">{description ?? view.description}</p>
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </div>
    </header>
  );
}

export function Page({
  view,
  title,
  description,
  crumbs,
  actions,
  tabs,
  wide,
  children,
}: {
  view: ViewDef;
  title?: ReactNode;
  description?: ReactNode;
  crumbs?: Crumb[];
  actions?: ReactNode;
  /** Secondary navigation under the title. */
  tabs?: ReactNode;
  /** `full` for workspaces like the Actions builder. */
  wide?: boolean | "full";
  children: ReactNode;
}) {
  return (
    <ScrollArea className="h-full">
      <div
        className={cn(
          "mx-auto flex flex-col gap-6 px-10 pt-6 pb-12",
          wide === "full" ? "max-w-360" : wide ? "max-w-5xl" : "max-w-3xl",
        )}
      >
        <ViewHeading view={view} title={title} description={description} crumbs={crumbs} actions={actions} />
        {tabs}
        {children}
      </div>
    </ScrollArea>
  );
}

/** Subpages as a row of tabs (underline style), e.g. a site's or Runtimes' sections. */
export function SubTabs({
  items,
  value,
  onChange,
  className,
}: {
  items: (SubDef & { badge?: ReactNode })[];
  value: string;
  onChange: (id: string) => void;
  className?: string;
}) {
  return (
    // Scrolls sideways when the tabs don't fit (a site has eight), without a visible bar.
    <div
      role="tablist"
      className={cn("flex items-center gap-1 overflow-x-auto border-b [scrollbar-width:none]", className)}
      onKeyDown={arrowKeys}
    >
      {items.map((item) => {
        const active = item.id === value;
        return (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={active}
            tabIndex={active ? 0 : -1}
            onClick={(e) => {
              onChange(item.id);
              e.currentTarget.scrollIntoView({ block: "nearest", inline: "nearest" });
            }}
            title={item.description}
            className={cn(
              "relative flex h-9 shrink-0 items-center gap-1.5 px-2.5 text-[13px] whitespace-nowrap text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring",
              active &&
                "text-foreground after:absolute after:inset-x-2 after:bottom-0 after:h-0.5 after:rounded-full after:bg-ember",
            )}
          >
            <item.icon className="size-3.5" strokeWidth={1.75} />
            {item.label}
            {item.badge}
          </button>
        );
      })}
    </div>
  );
}

/** Left/right arrows move between tabs, like native tab strips. */
function arrowKeys(e: React.KeyboardEvent<HTMLElement>) {
  if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
  const tabs = [...e.currentTarget.querySelectorAll<HTMLElement>('[role="tab"]')];
  const at = tabs.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  const next = tabs[(at + (e.key === "ArrowRight" ? 1 : tabs.length - 1)) % tabs.length];
  next?.focus();
  next?.click();
}

/** Subpages as a vertical list beside the content (Settings). */
export function SideNav({
  items,
  value,
  onChange,
  label,
}: {
  items: SubDef[];
  value: string;
  onChange: (id: string) => void;
  label: string;
}) {
  const settingsKeys = useShortcutLabel("settings");
  return (
    <nav aria-label={label} className="flex w-44 shrink-0 flex-col gap-0.5">
      {items.map((item) => {
        const active = item.id === value;
        return (
          <button
            key={item.id}
            type="button"
            onClick={() => onChange(item.id)}
            aria-current={active ? "page" : undefined}
            className={cn(
              "flex h-8 items-center gap-2.5 rounded-md px-2.5 text-left text-[13px] text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground",
              active && "bg-foreground/[0.07] text-foreground",
            )}
          >
            <item.icon className={cn("size-3.5 shrink-0", active && "text-ember")} strokeWidth={1.75} />
            <span className="truncate">{item.label}</span>
          </button>
        );
      })}
      <p className="mt-3 px-2.5 font-mono text-[10px] text-muted-foreground/70">
        {settingsKeys && t("{keys} opens settings", { keys: settingsKeys })}
      </p>
    </nav>
  );
}

/** A titled group of rows, separated by hairlines rather than boxed cards. */
export function Section({
  title,
  aside,
  className,
  children,
}: {
  title: string;
  aside?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <section className={cn("flex flex-col", className)}>
      <div className="mb-2 flex items-center justify-between gap-3 border-b pb-2">
        <h2 className="font-mono text-[11px] tracking-[0.08em] text-muted-foreground uppercase">{title}</h2>
        {aside}
      </div>
      {children}
    </section>
  );
}

/** `label ........ value` row used for facts and settings. */
export function Fact({ label, children, mono }: { label: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className="flex items-baseline gap-3 py-1.5 text-sm">
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <span aria-hidden className="mb-1 min-w-4 flex-1 border-b border-dotted border-foreground/15" />
      <span className={cn("min-w-0 truncate text-right", mono && "font-mono text-[12.5px]")} data-selectable>
        {children}
      </span>
    </div>
  );
}
