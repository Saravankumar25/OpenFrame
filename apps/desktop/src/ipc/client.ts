// The single typed boundary between React and the Rust application core.
// React never touches files, SQLite or the network directly: every read and
// every mutation is an allow-listed operation executed by Rust.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppEvent } from "./generated/AppEvent";

export type { AppEvent };

/** Mirrors Rust `AppError`. UI logic must branch on `code`, never on `message`. */
export interface AppErrorShape {
  code: string;
  message: string;
  detail?: string;
  retryable: boolean;
}

export class OpError extends Error implements AppErrorShape {
  code: string;
  detail?: string;
  retryable: boolean;
  op: string;
  constructor(op: string, e: AppErrorShape) {
    super(e.message);
    this.name = "OpError";
    this.op = op;
    this.code = e.code;
    this.detail = e.detail;
    this.retryable = e.retryable;
  }
  /** True when the code equals `prefix` or is nested under it (`storage` matches `storage.disk_full`). */
  is(prefix: string): boolean {
    return this.code === prefix || this.code.startsWith(prefix + ".");
  }
}

function toOpError(op: string, raw: unknown): OpError {
  if (raw && typeof raw === "object" && "code" in raw && "message" in raw) {
    const r = raw as { code: unknown; message: unknown; detail?: unknown; retryable?: unknown };
    const code = typeof r.code === "string" ? r.code : String((r.code as { 0?: string })?.[0] ?? r.code);
    return new OpError(op, {
      code,
      message: String(r.message),
      detail: r.detail ? String(r.detail) : undefined,
      retryable: Boolean(r.retryable),
    });
  }
  return new OpError(op, {
    code: "internal.unexpected",
    message: "Something went wrong inside OpenFrame. Your project was not changed.",
    detail: raw instanceof Error ? raw.message : String(raw),
    retryable: false,
  });
}

export const inTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Invoke an application operation (`module.action`). */
export async function call<T = unknown>(op: string, args: object = {}): Promise<T> {
  try {
    return await invoke<T>("of_invoke", { op, args });
  } catch (e) {
    throw toOpError(op, e);
  }
}

/** Open a project/global asset with the OS default app, or reveal it in Explorer. */
export async function openAsset(assetId: string, opts: { reveal?: boolean; global?: boolean } = {}): Promise<void> {
  try {
    await invoke("of_open_asset", { assetId, reveal: opts.reveal ?? false, global: opts.global ?? false });
  } catch (e) {
    throw toOpError("open_asset", e);
  }
}

export async function revealLocation(kind: "project" | "globalVault" | "logs" | "exported", id?: string, path?: string): Promise<void> {
  try {
    await invoke("of_reveal_path", { kind, id, path });
  } catch (e) {
    throw toOpError("reveal", e);
  }
}

export async function openUrl(url: string): Promise<void> {
  try {
    await invoke("of_open_url", { url });
  } catch (e) {
    throw toOpError("open_url", e);
  }
}

/** Subscribe to application events pushed from Rust. */
export function onAppEvent(handler: (e: AppEvent) => void): Promise<UnlistenFn> {
  return listen<AppEvent>("of://event", (msg) => handler(msg.payload));
}
