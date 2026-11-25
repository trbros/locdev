import { useQueryClient } from "@tanstack/react-query";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { toast } from "sonner";

import { AppContextMenu } from "@/components/app-context-menu";
import { CommandPalette } from "@/components/command-palette";
import { ShareSiteDialog } from "@/pages/tunnels";
import { RemoveSiteDialog } from "@/components/remove-site-dialog";
import { Sidebar } from "@/components/sidebar";
import { AddSiteDialog, EditSiteDialog } from "@/components/site-dialog";
import { StatusBar } from "@/components/status-bar";
import { TitleBar } from "@/components/title-bar";
import { useNotifications, useProjectAction, useProjects } from "@/hooks/use-daemon";
import { useOpenInEditor } from "@/hooks/use-editors";
import { useShortcutMap } from "@/lib/shortcuts";
import { openInBrowser, revealFolder } from "@/lib/system";
import { backend, request } from "@/lib/daemon";
import { TOOLS, VIEWS, type ViewId, viewById } from "@/lib/nav";
import { usePrefs } from "@/lib/prefs";
import type { Notification as DaemonNotification } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";
import { ComingSoonView } from "@/pages/coming-soon";
import { DashboardView } from "@/pages/dashboard";
import { NotificationsView, useOpenNotification } from "@/pages/notifications";
import { RuntimesView } from "@/pages/runtimes";
import { ServicesView } from "@/pages/services";
import { useAutoUpdate } from "@/hooks/use-updates";
import { SettingsView } from "@/pages/settings";
import { SitesView } from "@/pages/sites";
import { ToolsView } from "@/pages/tools";
import { t } from "@/lib/i18n";
import { sendTrayLanguage } from "@/lib/tray-language";

export default function App() {
  const { view } = useUi();
  useAppShortcuts();
  useBoot();
  useNotificationToasts();
  usePresence();
  useTrayNavigation();
  useNotificationLaunch();
  useAutoUpdate();

  const def = viewById(view);

  return (
    <>
      <AppContextMenu>
        <div className="grain flex h-full min-h-0 flex-col">
          <TitleBar />
          <div className="flex min-h-0 flex-1">
            <Sidebar />
            <main key={view} className="min-w-0 flex-1 animate-in fade-in-0 duration-200">
              {def.soon ? (
                <ComingSoonView view={def} />
              ) : view === "dashboard" ? (
                <DashboardView />
              ) : view === "sites" ? (
                <SitesView />
              ) : view === "runtimes" ? (
                <RuntimesView />
              ) : view === "services" ? (
                <ServicesView />
              ) : view === "tools" ? (
                <ToolsView />
              ) : view === "notifications" ? (
                <NotificationsView />
              ) : (
                <SettingsView />
              )}
            </main>
          </div>
          <StatusBar />
        </div>
      </AppContextMenu>

      {/* Dialogs live outside the app menu's tree: React bubbles events through
        portals, and a right-click inside a dialog shouldn't open the app menu. */}
      <CommandPalette />
      <AddSiteDialog />
      <EditSiteDialog />
      <RemoveSiteDialog />
      <ShareSiteDialog />
    </>
  );
}

function useAppShortcuts() {
  const {
    view,
    navigate,
    back,
    forward,
    setPaletteOpen,
    setAddSiteOpen,
    tabs,
    activeTab,
    activateTab,
    newTab,
    closeTab,
    selected,
    setSharing,
  } = useUi();
  const { sidebar, set } = usePrefs();
  const projects = useProjects();
  const action = useProjectAction();
  const openEditor = useOpenInEditor();
  const onSites = view === "sites";
  const site = onSites ? projects.data?.find((p) => p.name === selected) : undefined;

  const step = (by: number) => {
    const at = tabs.findIndex((tab) => tab.id === activeTab);
    activateTab(tabs[(at + by + tabs.length) % tabs.length].id);
  };
  const running = site?.process.state === "running";

  useShortcutMap({
    palette: () => setPaletteOpen(true),
    settings: () => navigate("settings"),
    shortcuts: () => navigate("settings", "shortcuts"),
    sidebar: () => set("sidebar", sidebar === "expanded" ? "collapsed" : "expanded"),
    back,
    forward,
    "add-site": () => setAddSiteOpen(true),
    "new-project": () => navigate("tools", "new-project"),
    ...Object.fromEntries(VIEWS.map((v) => [`go:${v.id}`, () => navigate(v.id)])),
    ...Object.fromEntries(TOOLS.map((tool) => [`tool:${tool.id}`, () => navigate("tools", tool.id)])),
    // Site tabs, as in a browser (only where they are).
    "new-tab": onSites && (() => newTab()),
    "close-tab": onSites && (() => closeTab(activeTab)),
    "next-tab": onSites && tabs.length > 1 && (() => step(1)),
    "previous-tab": onSites && tabs.length > 1 && (() => step(-1)),
    // The site in the front tab.
    "site-browser": !!site?.url && (() => openInBrowser(site.url!)),
    "site-editor": !!site && (() => openEditor.mutate({ site: site.name })),
    "site-folder": !!site && (() => revealFolder(site.path)),
    "site-share": !!site && running && (() => setSharing(site.name)),
    "site-start-stop":
      !!site?.command && (() => action.mutate({ action: running ? "stop_project" : "start_project", name: site.name })),
    "site-restart": !!site?.command && (() => action.mutate({ action: "restart_project", name: site.name })),
  });

  // Mouse back/forward buttons.
  useEffect(() => {
    const onMouse = (e: MouseEvent) => {
      if (e.button === 3) back();
      if (e.button === 4) forward();
    };
    window.addEventListener("mouseup", onMouse);
    return () => window.removeEventListener("mouseup", onMouse);
  }, [back, forward]);
}

/**
 * New notifications pop up as toasts while the app is open (the daemon keeps
 * them either way). Only ones newer than what was already there at launch.
 */
function useNotificationToasts() {
  const list = useNotifications(false);
  const open = useOpenNotification();
  const seen = useRef<number | null>(null);

  useEffect(() => {
    const items = list.data?.notifications;
    if (!items) return;
    const newest = items.reduce((max, n) => Math.max(max, n.id), 0);
    if (seen.current === null) {
      seen.current = newest;
      return;
    }
    for (const n of items.filter((n) => n.id > seen.current! && !n.read).slice(0, 3)) {
      const show = n.severity === "error" ? toast.error : n.severity === "warn" ? toast.warning : toast;
      show(n.title, {
        description: n.body.split("\n")[0],
        action: n.link ? { label: t("Open"), onClick: () => open(n) } : undefined,
      });
    }
    seen.current = Math.max(seen.current, newest);
  }, [list.data, open]);
}

/**
 * The shell starts the daemon at launch, behind the native splash, while this
 * window loads hidden. Once that start settles and the first data is in, the
 * shell swaps the splash for this window. Stopping the daemon later is respected.
 */
/**
 * Tells the daemon whether this window is in front, so it shows a native
 * notification only when nobody would see the toast. Repeated while focused
 * because the daemon forgets it after 20 s (a crashed app stops counting).
 */
function usePresence() {
  useEffect(() => {
    const send = () => {
      const focused = document.visibilityState === "visible" && document.hasFocus();
      void backend.request({ type: "client_presence", focused }).catch(() => {});
    };
    send();
    const timer = window.setInterval(() => {
      if (document.hasFocus()) send();
    }, 8_000);
    window.addEventListener("focus", send);
    window.addEventListener("blur", send);
    document.addEventListener("visibilitychange", send);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", send);
      window.removeEventListener("blur", send);
      document.removeEventListener("visibilitychange", send);
    };
  }, []);
}

/**
 * A click on an OS notification opens the app (or brings it forward) with
 * the notification it was: open what it's about, like a click in the list.
 */
function useNotificationLaunch() {
  const open = useOpenNotification();
  const { navigate } = useUi();
  const latest = useRef({ open, navigate });
  latest.current = { open, navigate };
  useEffect(() => {
    if (!isTauri()) return;
    const take = async () => {
      const target = await invoke<string | null>("take_notification_to_open").catch(() => null);
      if (!target) return;
      const { open, navigate } = latest.current;
      if (target === "list") return navigate("notifications");
      const { notifications } = await request({ type: "list_notifications", all: true }, "notifications").catch(() => ({
        notifications: [] as DaemonNotification[],
      }));
      const found = notifications.find((n) => n.id === Number(target));
      if (found?.link) open(found);
      else navigate("notifications");
    };
    void take();
    const off = listen<string>("open-notification", () => void take());
    return () => {
      off.then((fn) => fn());
    };
  }, []);
}

/** The tray's "unread notifications/emails" items open the app on that page. */
function useTrayNavigation() {
  const { navigate } = useUi();
  useEffect(sendTrayLanguage, []);
  useEffect(() => {
    if (!isTauri()) return;
    const off = listen<[ViewId, string | null]>("navigate", ({ payload: [view, sub] }) =>
      navigate(view, sub ?? undefined),
    );
    return () => {
      off.then((fn) => fn());
    };
  }, [navigate]);
}

function useBoot() {
  const client = useQueryClient();
  const booted = useRef(false);

  useEffect(() => {
    if (booted.current) return;
    booted.current = true;
    void (async () => {
      try {
        await backend.boot();
      } catch (error) {
        toast.error(t("Could not start the daemon"), {
          description: error instanceof Error ? error.message : String(error),
        });
      }
      await client.invalidateQueries();
      // Let the first frame with data paint before the window swaps in.
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      await backend.ready();
    })();
  }, [client]);
}
