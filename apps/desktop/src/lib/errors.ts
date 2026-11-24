import type { ErrorCode } from "./protocol";

export type DaemonErrorKind = "daemon_down" | "incompatible" | "app" | "other";

export class DaemonError extends Error {
  readonly kind: DaemonErrorKind;
  readonly code?: ErrorCode;

  constructor(kind: DaemonErrorKind, message: string, code?: ErrorCode) {
    super(message);
    this.kind = kind;
    this.code = code;
  }
}

export const isDaemonDown = (error: unknown) => error instanceof DaemonError && error.kind === "daemon_down";
