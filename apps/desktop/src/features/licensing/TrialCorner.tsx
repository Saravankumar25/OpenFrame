// The corner chip and "Buy full version" button in the top bar (owner mocks 183 and 188). Shown on
// every project screen from trial day 1 until the purchase; amber in the last 3 days, always with
// the days in words; gone after the purchase. No price is ever shown.
//
// On the Mac, after the "Checking your purchase" screen was closed with "Not now", the top bar
// shows "Check purchase" to open it again, so the way to check is never lost for the session.

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Zap } from "lucide-react";
import { Button, Chip } from "../../design-system";
import { toast } from "../../app/toast";
import { buyFullVersion, failureText, useCheckScreen, useLicenceNotice } from "./licence";
import { trialChip } from "./rules";

export function TrialCorner() {
  const notice = useLicenceNotice();
  const client = useQueryClient();
  const checkClosed = useCheckScreen((s) => s.closed);
  const reopenCheck = useCheckScreen((s) => s.reopen);
  const [busy, setBusy] = useState(false);
  const chip = trialChip(notice.data?.status?.state);
  // The internal installer build is never sold: no chip and no Buy (ADR-0023).
  if (notice.data?.status.channel === "directInstaller") return null;
  if (checkClosed && notice.data?.notice.kind === "checkNeeded") {
    return (
      <div className="row" role="group" aria-label="Licence" style={{ gap: 6, marginRight: 4 }}>
        <Button size="sm" variant="primary" icon={<RefreshCw size={13} aria-hidden />} onClick={reopenCheck}>
          Check purchase
        </Button>
      </div>
    );
  }
  if (!chip) return null;

  const buy = async () => {
    setBusy(true);
    try {
      const message = await buyFullVersion(client);
      if (message) toast.info(message);
    } catch (e) {
      toast.error(failureText(e, "OpenFrame could not start the purchase right now. Everything keeps working."));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="row" role="group" aria-label="Free trial" style={{ gap: 6, marginRight: 4 }}>
      <Chip tone={chip.ending ? "y" : "default"} title={chip.spoken}>
        <span aria-hidden>{chip.label}</span>
        <span className="sr-only">{chip.spoken}</span>
      </Chip>
      <Button size="sm" variant="primary" icon={<Zap size={13} aria-hidden />} disabled={busy} onClick={() => void buy()}>
        Buy full version
      </Button>
    </div>
  );
}
