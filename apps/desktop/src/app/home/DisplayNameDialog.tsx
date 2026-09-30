// First-run name prompt and "change your name" (Security §5: a stable local
// identity with a visible display name — no account). The name appears on
// notes, tasks and activity. Stored via app.set_display_name.

import { useState } from "react";
import { call } from "../../ipc/client";
import { queryClient } from "../../ipc/query";
import { Button, Dialog, Field, TextInput } from "../../design-system";
import { toast } from "../toast";

export function DisplayNameDialog({ firstRun, current, onClose }: { firstRun: boolean; current: string; onClose: () => void }) {
  const [name, setName] = useState(current);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const invalid = name.trim().length === 0;
  const save = async () => {
    if (invalid || busy) return;
    setBusy(true);
    try {
      await call("app.set_display_name", { displayName: name.trim() });
      await queryClient.invalidateQueries({ queryKey: ["app.info"] });
      if (!firstRun) toast.success("Your name was updated.");
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Your name could not be saved.");
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      dismissible={!firstRun}
      onOpenChange={(v) => !v && onClose()}
      title={firstRun ? "Welcome to OpenFrame Studio" : "Your name"}
      sub={firstRun ? "What should we call you?" : undefined}
      size="sm"
      footer={
        <>
          {!firstRun && <Button onClick={onClose}>Cancel</Button>}
          <Button variant="primary" disabled={invalid || busy} onClick={() => void save()}>
            {firstRun ? "Continue" : "Save"}
          </Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); void save(); }}>
        <Field label="Your name" required htmlFor="dn-name" error={invalid ? "Enter your name." : error}>
          <TextInput id="dn-name" autoFocus value={name} maxLength={80} invalid={invalid} onChange={(e) => { setName(e.target.value); setError(null); }} onFocus={(e) => e.currentTarget.select()} />
        </Field>
        <div className="hint">
          Your name appears on notes, tasks and project activity. There is no account — it stays on this computer and you can change it any time from the avatar on the Home screen.
        </div>
      </form>
    </Dialog>
  );
}
