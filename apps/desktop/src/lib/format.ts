import type { ProcessState } from "./protocol";
import { intlLocale, t } from "@/lib/i18n";

export function uptime(since: number, now = Date.now()): string {
  const secs = Math.max(0, Math.floor((now - since) / 1000));
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ${mins % 60}m`;
  return `${Math.floor(hours / 24)}d ${hours % 24}h`;
}

export const STATE_LABEL: Record<ProcessState, string> = {
  running: t("Running"),
  stopping: t("Stopping"),
  stopped: t("Stopped"),
  exited: t("Exited"),
  failed: t("Failed"),
};

/** Shortens /home/<user>/... and C:\Users\<user>\... to ~/... for display. */
export function tildify(path: string): string {
  return path.replace(/^\/home\/[^/]+|^\/Users\/[^/]+|^[A-Za-z]:\\Users\\[^\\]+/, "~");
}

// eslint-disable-next-line no-control-regex
const ANSI = /\x1b\[[0-9;?]*[A-Za-z]|\x1b\][^\x07]*\x07/g;

export const stripAnsi = (text: string) => text.replace(ANSI, "");

export function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString(intlLocale, { hour12: false });
}
