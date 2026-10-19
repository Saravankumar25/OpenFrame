// One registry of typed text that is not stored yet (Windows phase W2, "protect writers' work").
//
// Every editor that waits a moment before saving registers its flush here with usePendingSave.
// Closing the window, switching workspace and switching or closing the project call
// flushPendingSaves() and wait for it, so nothing typed just before is lost. A flush returns false
// when its text is still not stored after trying (the save failed); the caller then asks the user.

import { useEffect, useRef } from "react";

/** Store pending text now. `false` means it could not be stored. */
export type Flush = () => Promise<boolean | void> | boolean | void;

const flushes = new Set<Flush>();

export function registerPendingSave(flush: Flush): () => void {
  flushes.add(flush);
  return () => {
    flushes.delete(flush);
  };
}

/** How long a caller waits for the saves before it treats them as failed (the app never hangs). */
export const FLUSH_TIMEOUT_MS = 4000;

/**
 * Run every registered flush at once and wait for all of them. True when each one finished and
 * none reported unstored text; false when one failed or the wait passed `timeoutMs`.
 */
export async function flushPendingSaves(timeoutMs = FLUSH_TIMEOUT_MS): Promise<boolean> {
  if (flushes.size === 0) return true;
  const all = Promise.allSettled([...flushes].map((f) => Promise.resolve().then(f))).then((results) =>
    results.every((r) => r.status === "fulfilled" && r.value !== false),
  );
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<boolean>((resolve) => {
    timer = setTimeout(() => resolve(false), timeoutMs);
  });
  try {
    return await Promise.race([all, late]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Keep `flush` registered while the component is mounted. When it unmounts (another item opens,
 * the drawer closes, the workspace changes) the flush runs once more, so the last keystrokes are
 * sent even when nobody waited for them.
 */
export function usePendingSave(flush: Flush): void {
  const ref = useRef(flush);
  useEffect(() => {
    ref.current = flush;
  }, [flush]);
  useEffect(() => {
    const run: Flush = () => ref.current();
    const unregister = registerPendingSave(run);
    return () => {
      unregister();
      // Each flush reports its own failure; this only keeps a throw from going unhandled.
      void Promise.resolve().then(run).catch(() => undefined);
    };
  }, []);
}
