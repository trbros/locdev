import { invoke, isTauri } from "@tauri-apps/api/core";
import { locale, t } from "@/lib/i18n";

/** The tray menu's texts (English keys, as in `src-tauri/src/tray.rs`),
 * translated from the app's catalogs: the shell has no catalogs of its own. */
function trayStrings(): Record<string, string> {
  const strings = {
    "LocDev: daemon running · {running}/{total} sites up": t("LocDev: daemon running · {running}/{total} sites up"),
    "LocDev: daemon stopped": t("LocDev: daemon stopped"),
    "1 unread notification": t("1 unread notification"),
    "{n} unread notifications": t("{n} unread notifications"),
    "1 unread email": t("1 unread email"),
    "{n} unread emails": t("{n} unread emails"),
    "blue dot": t("blue dot"),
    "green dot": t("green dot"),
    "violet dot": t("violet dot"),
    "No sites yet": t("No sites yet"),
    "Loading sites…": t("Loading sites…"),
    "Open LocDev": t("Open LocDev"),
    "Restart daemon": t("Restart daemon"),
    "Stop daemon": t("Stop daemon"),
    "Start daemon": t("Start daemon"),
    "Quit LocDev": t("Quit LocDev"),
    "Open in browser": t("Open in browser"),
    Restart: t("Restart"),
    Stop: t("Stop"),
    Start: t("Start"),
  };
  return locale === "en" ? {} : strings;
}

/** Tells the shell the tray's texts; once per load, since changing the
 * language reloads the window. */
export function sendTrayLanguage() {
  if (!isTauri()) return;
  invoke("tray_language", { strings: trayStrings() }).catch(() => {});
}
