// Debounced autosave for text buffers. The last change is flushed when the
// editor closes, so no keystrokes are lost; errors are reported, never swallowed.
// Backend edits use a coalesce key, so a burst of saves is one undo step.

import { useCallback, useEffect, useRef } from "react";
import { usePendingSave } from "../../app/pendingSaves";
import { reportError } from "../../ipc/query";

export function useDebouncedSave<T>(save: (value: T) => Promise<unknown>, delay = 600) {
  const timer = useRef<number | undefined>(undefined);
  const pending = useRef<{ value: T } | null>(null);
  const inflight = useRef<Promise<boolean> | null>(null);
  const saveRef = useRef(save);
  useEffect(() => {
    saveRef.current = save;
  }, [save]);

  /**
   * Save the pending value now and wait for it, after any save already running. Resolves false
   * when the text is still not stored; a failed value stays pending so the next flush retries it.
   */
  const flush = useCallback(async (): Promise<boolean> => {
    window.clearTimeout(timer.current);
    if (inflight.current) await inflight.current;
    const p = pending.current;
    if (!p) return true;
    pending.current = null;
    const run = saveRef.current(p.value).then(
      () => true,
      (e: unknown) => {
        reportError(e);
        // Keep the failed value unless something newer was typed meanwhile.
        pending.current ??= p;
        return false;
      },
    );
    inflight.current = run;
    try {
      return await run;
    } finally {
      if (inflight.current === run) inflight.current = null;
    }
  }, []);

  const schedule = useCallback(
    (value: T) => {
      pending.current = { value };
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => void flush(), delay);
    },
    [delay, flush],
  );

  usePendingSave(flush);
  return { schedule, flush };
}
