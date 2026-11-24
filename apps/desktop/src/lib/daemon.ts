import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { mockBackend } from "./mock";
import { DaemonError, type DaemonErrorKind } from "./errors";
import type {
  AppInfo,
  AppUpdate,
  AppUpdateProgress,
  CliStatus,
  ErrorCode,
  Request,
  Response,
  ServiceStatus,
  SystemAction,
  SystemPlan,
  SystemTarget,
} from "./protocol";

export { DaemonError, isDaemonDown } from "./errors";

export interface Backend {
  request(request: Request): Promise<Response>;
  start(): Promise<boolean>;
  stop(): Promise<boolean>;
  /** Waits for the daemon start the shell begins at launch. */
  boot(): Promise<boolean>;
  /** The UI has its first data: the shell closes the splash and shows it. */
  ready(): Promise<void>;
  info(): Promise<AppInfo>;
  /** The privileged change for `target`, to show before applying it. */
  systemPlan(target: SystemTarget, action: SystemAction): Promise<SystemPlan>;
  /** Applies that change; the OS shows its own authorization prompt. */
  systemApply(target: SystemTarget, action: SystemAction): Promise<void>;
  /** Whether the daemon starts at login (per user, no elevation). */
  serviceStatus(): Promise<ServiceStatus>;
  serviceSet(enabled: boolean): Promise<ServiceStatus>;
  /** Saves a message (.eml) or an attachment through the native dialog; the path, or null if cancelled. */
  saveMailFile(id: string, attachment: number | null, fileName: string): Promise<string | null>;
  /** The bundled `locdev` command: version and whether terminals find it. */
  cliStatus(): Promise<CliStatus>;
  /** Adds or removes the per-user link that puts it on PATH. */
  cliLink(enabled: boolean): Promise<CliStatus>;
  /** The app's updater: the latest release, or exactly `tag` (a rollback when older). */
  appUpdateCheck(tag?: string | null): Promise<AppUpdate>;
  /** Downloads, verifies the signature and installs; resolves with the installed version. */
  appUpdateInstall(tag?: string | null): Promise<string>;
  /** Restarts into the installed version (optionally restarting the daemon too). */
  appRestart(restartDaemon: boolean): Promise<void>;
  onAppUpdateProgress(listener: (p: AppUpdateProgress) => void): Promise<() => void>;
  /** Saves text through the native save dialog; the path, or null if cancelled. */
  saveTextFile(fileName: string, text: string): Promise<string | null>;
}

function toDaemonError(error: unknown): DaemonError {
  if (error instanceof DaemonError) return error;
  if (error && typeof error === "object" && "kind" in error) {
    const e = error as { kind: DaemonErrorKind; message: string; code?: ErrorCode };
    return new DaemonError(e.kind, e.message, e.code);
  }
  return new DaemonError("other", String(error));
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toDaemonError(error);
  }
}

const tauriBackend: Backend = {
  request: (request) => call<Response>("daemon_request", { request }),
  start: () => call<boolean>("daemon_start"),
  stop: () => call<boolean>("daemon_stop"),
  boot: () => call<boolean>("daemon_boot"),
  ready: () => call<void>("app_ready"),
  info: () => call<AppInfo>("app_info"),
  systemPlan: (target, action) => call<SystemPlan>("system_plan", { target, action }),
  systemApply: (target, action) => call<void>("system_apply", { target, action }),
  serviceStatus: () => call<ServiceStatus>("service_status"),
  serviceSet: (enabled) => call<ServiceStatus>("service_set", { enabled }),
  saveMailFile: (id, attachment, fileName) => call<string | null>("save_mail_file", { id, attachment, fileName }),
  cliStatus: () => call<CliStatus>("cli_status"),
  cliLink: (enabled) => call<CliStatus>("cli_link", { enabled }),
  appUpdateCheck: (tag) => call<AppUpdate>("app_update_check", { tag: tag ?? null }),
  appUpdateInstall: (tag) => call<string>("app_update_install", { tag: tag ?? null }),
  appRestart: (restartDaemon) => call<void>("app_restart", { restartDaemon }),
  onAppUpdateProgress: (listener) => listen<AppUpdateProgress>("app-update-progress", (e) => listener(e.payload)),
  saveTextFile: (fileName, text) => call<string | null>("save_text_file", { fileName, text }),
};

/** Running inside Tauri talks to the real daemon; a plain browser gets a mock. */
export const backend: Backend = isTauri() ? tauriBackend : mockBackend;
export const isMock = !isTauri();
/** `?shots` hides mock-only chrome so docs screenshots look like the real app. */
export const screenshotMode = isMock && new URLSearchParams(window.location.search).has("shots");

/** Sends a request and narrows the response to the expected variant. */
export async function request<T extends Response["type"]>(
  req: Request,
  expected: T,
): Promise<Extract<Response, { type: T }>> {
  const response = await backend.request(req);
  if (response.type === "error") {
    throw new DaemonError("app", response.message, response.code);
  }
  if (response.type !== expected) {
    throw new DaemonError("other", `unexpected response '${response.type}', wanted '${expected}'`);
  }
  return response as Extract<Response, { type: T }>;
}
