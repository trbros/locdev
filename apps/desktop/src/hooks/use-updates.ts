// Updates. The app's own come from the Tauri shell (signed latest.json);
// the daemon/CLI ones are read by the daemon from the published releases.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { backend, request, screenshotMode } from "@/lib/daemon";
import { usePrefs } from "@/lib/prefs";
import type { AppUpdateProgress } from "@/lib/protocol";
import { useUi } from "@/lib/ui-state";

import { errorMessage } from "./use-daemon";
import { t } from "@/lib/i18n";

/** Releases and how this daemon/CLI compares (cached by the daemon for 6 h). */
export function useReleases() {
  return useQuery({
    queryKey: ["updates", "releases"],
    queryFn: async () => {
      const { type: _, ...status } = await request({ type: "update_check" }, "updates");
      return status;
    },
    retry: false,
    staleTime: 60_000,
  });
}

export function useReleasesRefresh() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: async () => {
      const { type: _, ...status } = await request({ type: "update_check", refresh: true }, "updates");
      return status;
    },
    onSuccess: (status) => client.setQueryData(["updates", "releases"], status),
    onError: (error) => toast.error(t("Could not check for updates"), { description: errorMessage(error) }),
  });
}

/** What the app's updater offers: the latest release, or `tag` exactly. */
export function useAppUpdate(tag: string | null = null, enabled = true) {
  return useQuery({
    queryKey: ["updates", "app", tag],
    queryFn: () => backend.appUpdateCheck(tag),
    enabled,
    retry: false,
    staleTime: 5 * 60_000,
  });
}

/** Install progress, 0–1 (null while unknown). */
export function useAppInstall() {
  const client = useQueryClient();
  const [progress, setProgress] = useState<AppUpdateProgress | null>(null);
  useEffect(() => {
    let off: (() => void) | undefined;
    let gone = false;
    backend.onAppUpdateProgress(setProgress).then((unlisten) => (gone ? unlisten() : (off = unlisten)));
    return () => {
      gone = true;
      off?.();
    };
  }, []);
  const install = useMutation({
    mutationFn: (tag: string | null) => backend.appUpdateInstall(tag),
    onMutate: () => setProgress(null),
    onError: (error) => toast.error(t("The update failed"), { description: errorMessage(error) }),
    onSettled: () => client.invalidateQueries({ queryKey: ["updates", "app"] }),
  });
  return { install, progress };
}

export function useAppRestart() {
  return useMutation({
    mutationFn: (restartDaemon: boolean) => backend.appRestart(restartDaemon),
    onError: (error) => toast.error(t("Could not restart"), { description: errorMessage(error) }),
  });
}

const LAST_CHECK = "locdev.updates.lastCheck";
const DAY = 24 * 3_600_000;

/** At launch (after a short wait) and once a day while open: tells about a
 * newer app, or installs it when the user chose that. Never restarts. */
export function useAutoUpdate() {
  const { updateCheck, updateInstall } = usePrefs();
  const { navigate } = useUi();
  const busy = useRef(false);

  useEffect(() => {
    if (!updateCheck || screenshotMode) return;
    const run = async () => {
      let last = 0;
      try {
        last = Number(localStorage.getItem(LAST_CHECK) ?? 0);
      } catch {
        // No storage: check every launch.
      }
      if (busy.current || Date.now() - last < DAY) return;
      busy.current = true;
      try {
        const update = await backend.appUpdateCheck(null);
        try {
          localStorage.setItem(LAST_CHECK, String(Date.now()));
        } catch {
          // As above.
        }
        if (!update.version) return;
        const open = { label: t("Open"), onClick: () => navigate("settings", "updates") };
        if (updateInstall && update.can_install) {
          const version = await backend.appUpdateInstall(null);
          toast.success(t("LocDev {version} is installed", { version }), {
            description: t("It runs after a restart."),
            action: { label: t("Restart"), onClick: () => backend.appRestart(true) },
            duration: Infinity,
          });
        } else {
          toast(t("LocDev {version} is available", { version: update.version }), { action: open, duration: 15_000 });
        }
      } catch {
        // Offline, or no release yet: Settings → Updates shows why.
      } finally {
        busy.current = false;
      }
    };
    const first = setTimeout(run, 10_000);
    const daily = setInterval(run, 60 * 60_000);
    return () => {
      clearTimeout(first);
      clearInterval(daily);
    };
  }, [updateCheck, updateInstall, navigate]);
}
