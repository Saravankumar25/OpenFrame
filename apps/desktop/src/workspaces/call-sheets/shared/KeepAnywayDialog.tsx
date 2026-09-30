// "Keep Anyway" confirmation for moves refused by strict validation (FSD §36.5–36.6).

import { useState } from "react";
import { ConfirmDialog } from "../../../design-system";
import { useKeepAnyway } from "../../../api/schedule";

export function KeepAnywayDialog() {
  const pending = useKeepAnyway((s) => s.pending);
  const set = useKeepAnyway((s) => s.set);
  const [busy, setBusy] = useState(false);
  return (
    <ConfirmDialog
      open={pending !== null}
      onOpenChange={(v) => !v && set(null)}
      title="This change creates a schedule warning"
      confirmLabel="Keep Anyway"
      cancelLabel="Change the schedule instead"
      busy={busy}
      onConfirm={async () => {
        if (!pending) return;
        setBusy(true);
        await pending.retry();
        setBusy(false);
        set(null);
      }}
    >
      <p style={{ marginTop: 0 }}>{pending?.message}</p>
      <p className="hint">Strict validation is on for this schedule. Nothing was changed yet; the choice is yours.</p>
    </ConfirmDialog>
  );
}
