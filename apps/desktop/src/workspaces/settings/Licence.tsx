// Settings > Project settings > Licence (build plan area "Payment and free trial in all three
// channels", items "Shared entitlement interface" and "No data loss on expiry"; ADR-0018, ADR-0022
// and ADR-0023). Shows the free-trial or licence state the Rust core computed, the release channel
// in plain words and why a channel cannot answer yet, with a "Restore purchase" action. The
// internal installer build says "Internal build, not for sale." and has nothing to restore. No
// price, currency or pricing model is ever shown.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { Banner, Button, Skeleton } from "../../design-system";
import { call, OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { Channel } from "../../ipc/generated/Channel";
import type { EntitlementStatus } from "../../ipc/generated/EntitlementStatus";

export const CHANNEL_LABEL: Record<Channel, string> = {
  microsoftStore: "Microsoft Store",
  macAppStore: "Mac App Store",
  directInstaller: "Internal build",
};

const LOAD_FAILED = "OpenFrame could not show your licence right now. Everything keeps working.";
const RESTORE_FAILED = "OpenFrame could not restore your purchase right now. Everything keeps working. Try again later.";

function plainMessage(e: unknown, fallback: string): string {
  if (e instanceof OpError) return e.message;
  reportError(e);
  return fallback;
}

export function LicenceSection() {
  const [status, setStatus] = useState<EntitlementStatus | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [checked, setChecked] = useState(false);
  const client = useQueryClient();

  useEffect(() => {
    let live = true;
    call<EntitlementStatus>("entitlement.status", {})
      .then((s) => live && setStatus(s))
      .catch((e: unknown) => live && setLoadError(plainMessage(e, LOAD_FAILED)));
    return () => {
      live = false;
    };
  }, []);

  const restore = async () => {
    setBusy(true);
    setError(null);
    setChecked(false);
    try {
      const s = await call<EntitlementStatus>("entitlement.restore_purchase", {});
      setStatus(s);
      setLoadError(null);
      setChecked(true);
      // A purchase found here lifts a store lock and starts the screens that follow a purchase.
      void client.invalidateQueries();
    } catch (e) {
      setError(plainMessage(e, RESTORE_FAILED));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="card pad" style={{ maxWidth: 720, marginTop: 16 }} aria-labelledby="licence-heading">
      <h2 id="licence-heading" className="h4">Licence</h2>
      {status ? (
        <>
          <p className="sm" style={{ margin: "0 0 6px", lineHeight: 1.5 }}>{status.summary}</p>
          <p className="xs muted" style={{ margin: "0 0 10px" }}>
            Installed from: {CHANNEL_LABEL[status.channel]}
          </p>
          {status.limitation && (
            <p className="xs muted" style={{ margin: "0 0 10px", lineHeight: 1.5 }}>{status.limitation}</p>
          )}
        </>
      ) : (
        !loadError && (
          <div role="status" aria-label="Checking your licence" style={{ marginBottom: 10 }}>
            <Skeleton h={14} w="60%" />
          </div>
        )
      )}
      {loadError && (
        <div style={{ marginBottom: 10 }}>
          <Banner tone="err">{loadError}</Banner>
        </div>
      )}
      {error && (
        <div style={{ marginBottom: 10 }}>
          <Banner tone="err">{error}</Banner>
        </div>
      )}
      {status?.channel !== "directInstaller" && (
        <Button icon={<RefreshCw size={14} />} disabled={busy} onClick={() => void restore()}>
          {busy ? "Checking…" : "Restore purchase"}
        </Button>
      )}
      {checked && !busy && (
        <span role="status" className="xs muted" style={{ marginLeft: 10 }}>
          Checked again.
        </span>
      )}
    </section>
  );
}
