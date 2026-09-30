// Debounced autosave for text buffers. The last change is flushed when the
// editor closes, so no keystrokes are lost; errors are reported, never swallowed.
// Backend edits use a coalesce key, so a burst of saves is one undo step.

import { useCallback, useEffect, useRef } from "react";
import { reportError } from "../../ipc/query";

export function useDebouncedSave<T>(save: (value: T) => Promise<unknown>, delay = 600) {
  const timer = useRef<number | undefined>(undefined);
  const pending = useRef<{ value: T } | null>(null);
  const saveRef = useRef(save);
  useEffect(() => {
    saveRef.current = save;
  }, [save]);

  const flush = useCallback(() => {
    window.clearTimeout(timer.current);
    const p = pending.current;
    pending.current = null;
    if (p) void saveRef.current(p.value).catch(reportError);
  }, []);

  const schedule = useCallback(
    (value: T) => {
      pending.current = { value };
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(flush, delay);
    },
    [delay, flush],
  );

  useEffect(() => () => flush(), [flush]);
  return { schedule, flush };
}
