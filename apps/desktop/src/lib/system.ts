import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { t } from "@/lib/i18n";

export async function openInBrowser(url: string) {
  if (!isTauri()) {
    window.open(url, "_blank");
    return;
  }
  await openUrl(url).catch((e) => toast.error(t("Could not open the browser"), { description: String(e) }));
}

export async function revealFolder(path: string) {
  if (!isTauri()) {
    toast(t("Reveal folder"), { description: path });
    return;
  }
  await revealItemInDir(path).catch((e) => toast.error(t("Could not open the folder"), { description: String(e) }));
}

/** Native folder picker; `null` when cancelled or unavailable. */
export async function pickFolder(title = t("Choose a project folder")): Promise<string | null> {
  if (!isTauri()) return null;
  const selected = await open({ directory: true, multiple: false, title });
  return typeof selected === "string" ? selected : null;
}

export async function copyText(text: string, label = t("Copied to clipboard")) {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(label);
  } catch {
    toast.error(t("Could not copy"));
  }
}
