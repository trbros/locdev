import { Bell, BellOff, Check, CheckCheck, ExternalLink, Send, X } from "lucide-react";

import { Page, Section, SubTabs, useCrumbs } from "@/components/page";
import { Led, type Tone } from "@/components/status";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import {
  useDaemonDown,
  useMarkNotifications,
  useNotifications,
  useSettings,
  useTestNotification,
} from "@/hooks/use-daemon";
import { uptime } from "@/lib/format";
import { NOTIFICATION_TABS, type SiteTab, subById, type ViewId, viewById } from "@/lib/nav";
import { CATEGORY_LABEL, type Notification, type NotificationCategory } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { cn } from "@/lib/utils";
import { t } from "@/lib/i18n";

const TONE: Record<Notification["severity"], Tone> = { info: "info", warn: "warn", error: "fail" };

/** Opens what a notification is about. */
export function useOpenNotification() {
  const { navigate, openSite } = useUi();
  const mark = useMarkNotifications();
  return (n: Notification) => {
    if (!n.read) mark.mutate({ ids: [n.id], read: true });
    if (!n.link) return;
    if (n.link.view === "sites" && n.link.project) openSite(n.link.project, (n.link.sub ?? "overview") as SiteTab);
    else navigate(n.link.view as ViewId, n.link.sub ?? undefined);
  };
}

export function NotificationsView() {
  const view = viewById("notifications");
  const { sub: current, navigate } = useUi();
  const sub = subById(view, current)!;
  const crumbs = useCrumbs(view, sub);
  const down = useDaemonDown();
  const unread = useNotifications(false).data?.unread ?? 0;
  const mark = useMarkNotifications();

  return (
    <Page
      view={view}
      crumbs={crumbs}
      actions={
        sub.id === "pending" &&
        unread > 0 && (
          <Button variant="outline" size="sm" onClick={() => mark.mutate({ read: true })}>
            <CheckCheck data-icon="inline-start" /> {t("Mark all read")}
          </Button>
        )
      }
      tabs={
        <SubTabs
          items={NOTIFICATION_TABS.map((t) => ({
            ...t,
            badge:
              t.id === "pending" && unread > 0 ? (
                <span className="rounded-full bg-ember/15 px-1.5 font-mono text-[10px] text-ember">{unread}</span>
              ) : undefined,
          }))}
          value={sub.id}
          onChange={(id) => navigate("notifications", id)}
        />
      }
    >
      {down ? (
        <p className="text-sm text-muted-foreground">{t("Start the daemon to see notifications.")}</p>
      ) : sub.id === "preferences" ? (
        <Preferences />
      ) : (
        <List history={sub.id === "history"} />
      )}
    </Page>
  );
}

function List({ history }: { history: boolean }) {
  const list = useNotifications(history);
  const mark = useMarkNotifications();
  const open = useOpenNotification();
  const items = (list.data?.notifications ?? []).filter((n) => history || !n.read);

  if (list.isPending) {
    return (
      <div className="flex flex-col gap-2">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-14" />
        ))}
      </div>
    );
  }
  if (!items.length) {
    return (
      <div className="dots flex flex-col items-center gap-2 rounded-xl border py-12 text-center">
        <Bell className="size-5 text-muted-foreground" />
        <p className="text-sm">{history ? t("No notifications yet.") : t("You're all caught up.")}</p>
        <p className="max-w-sm text-[13px] text-muted-foreground">
          {t(
            "LocDev notifies about dependency updates, security advisories, dev servers that fail and the result of things you start. They stay here, even after restarts, until you dismiss them.",
          )}
        </p>
      </div>
    );
  }

  return (
    <ul className="flex flex-col">
      {items.map((n) => (
        <li
          key={n.id}
          className={cn("group flex gap-3 border-b border-dashed py-3 last:border-b-0", n.dismissed && "opacity-60")}
        >
          <span className="mt-1.5">
            <Led tone={n.read ? "off" : TONE[n.severity]} />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex items-baseline gap-2">
              <span className={cn("text-sm", !n.read && "font-medium")}>{n.title}</span>
              <span className="font-mono text-[10.5px] text-muted-foreground">
                {CATEGORY_LABEL[n.category].toLowerCase()} · {uptime(n.created_at)} {t("ago")}
                {n.dismissed && t(" · dismissed")}
              </span>
            </div>
            {n.body && <p className="mt-0.5 text-[13px] whitespace-pre-line text-muted-foreground">{n.body}</p>}
          </div>
          <div className="flex shrink-0 items-start gap-1 opacity-60 transition-opacity group-hover:opacity-100">
            {n.link && (
              <Button variant="ghost" size="xs" onClick={() => open(n)}>
                <ExternalLink data-icon="inline-start" /> {t("Open")}
              </Button>
            )}
            {!n.read && (
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={t("Mark read")}
                onClick={() => mark.mutate({ ids: [n.id], read: true })}
              >
                <Check />
              </Button>
            )}
            {!n.dismissed && (
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={t("Dismiss")}
                onClick={() => mark.mutate({ ids: [n.id], dismissed: true })}
              >
                <X />
              </Button>
            )}
          </div>
        </li>
      ))}
    </ul>
  );
}

function Preferences() {
  const { query, update } = useSettings();
  const test = useTestNotification();
  const settings = query.data;
  if (!settings) return <Skeleton className="h-40" />;
  const set = (patch: Partial<typeof settings.notifications>) =>
    update.mutate({ ...settings, notifications: { ...settings.notifications, ...patch } });

  return (
    <>
      <Section title={t("desktop notifications")}>
        <label className="flex items-start gap-3 py-1.5">
          <Switch
            checked={settings.notifications.native}
            onCheckedChange={(native) => set({ native })}
            className="mt-0.5 data-checked:bg-ember"
          />
          <span>
            <span className="block text-sm">{t("Also show them through the system")}</span>
            <span className="block text-[12.5px] text-muted-foreground">
              {t(
                "Delivered by the daemon whenever no LocDev window is in front: hidden in the tray, closed or behind other windows. While this window is in front you get toasts instead, never both. What comes in while the computer is off or locked shows when it's back (several at once become one summary).",
              )}
            </span>
          </span>
        </label>
        <div className="mt-2">
          <Button variant="outline" size="sm" disabled={test.isPending} onClick={() => test.mutate()}>
            <Send data-icon="inline-start" /> {t("Send a test notification")}
          </Button>
        </div>
      </Section>
      <Section title={t("categories")}>
        <ul>
          {(Object.keys(CATEGORY_LABEL) as NotificationCategory[]).map((category) => {
            const on = settings.notifications.categories[category] ?? true;
            return (
              <li key={category} className="flex items-center gap-3 border-b border-dashed py-2 last:border-b-0">
                {on ? (
                  <Bell className="size-3.5 text-muted-foreground" />
                ) : (
                  <BellOff className="size-3.5 text-muted-foreground" />
                )}
                <span className="flex-1 text-sm">{CATEGORY_LABEL[category]}</span>
                <Switch
                  checked={on}
                  onCheckedChange={(next) =>
                    set({ categories: { ...settings.notifications.categories, [category]: next } })
                  }
                  className="data-checked:bg-ember"
                  aria-label={CATEGORY_LABEL[category]}
                />
              </li>
            );
          })}
        </ul>
        <p className="mt-2 text-[12.5px] text-muted-foreground">
          {t(
            "A turned-off category creates no notifications at all. How often dependencies are checked is in Settings → Automations.",
          )}
        </p>
      </Section>
    </>
  );
}
