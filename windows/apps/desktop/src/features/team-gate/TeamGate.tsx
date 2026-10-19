// Team password for the internal builds (ADR-0024; build plan area 20 box "Team password gate").
// The internal Windows installer and Mac disk image ask the team password on every launch, before
// anything opens. The Rust core holds the check and refuses every other operation until the
// password is right, so this screen is not the only gate. Store packages have no gate: the status
// says it is not required and the app opens straight away.

import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { LockKeyhole } from "lucide-react";
import { Banner, Button, Dialog, Field, TextInput } from "../../design-system";
import { call } from "../../ipc/client";
import type { TeamGateAnswer } from "../../ipc/generated/TeamGateAnswer";
import type { TeamGateStatus } from "../../ipc/generated/TeamGateStatus";
import { BrandLogo } from "../../theme/BrandLogo";

const LOAD_FAILED = "OpenFrame could not start the password check. Close OpenFrame and open it again.";
const TRY_FAILED = "OpenFrame could not check the password right now. Try again.";

/** "1:05", "0:09": the wait in minutes and seconds. */
export function waitClock(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Shows `children` once the team password is right, or straight away when there is no gate. */
export function TeamGate({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<TeamGateStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const load = useCallback(() => {
    setFailed(false);
    call<TeamGateStatus>("team_gate.status", {})
      .then(setStatus)
      .catch(() => setFailed(true));
  }, []);
  useEffect(load, [load]);

  if (status && (!status.required || status.unlocked)) return <>{children}</>;
  return (
    <div className="app-root splash" role={status ? undefined : "status"}>
      <BrandLogo size="lg" />
      {failed ? (
        <div className="col" style={{ alignItems: "center", gap: 10 }}>
          <div className="splash-sub" role="alert">{LOAD_FAILED}</div>
          <Button onClick={load}>Try again</Button>
        </div>
      ) : status ? (
        <PasswordDialog status={status} onStatus={setStatus} />
      ) : (
        <div className="splash-sub">Starting OpenFrame Studio…</div>
      )}
    </div>
  );
}

function PasswordDialog({ status, onStatus }: { status: TeamGateStatus; onStatus: (s: TeamGateStatus) => void }) {
  const client = useQueryClient();
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [wait, setWait] = useState(status.waitSeconds);

  // Count the wait down on screen; ask the core again when it ends.
  useEffect(() => setWait(status.waitSeconds), [status.waitSeconds]);
  useEffect(() => {
    if (wait <= 0) return;
    const t = window.setTimeout(() => {
      if (wait <= 1) {
        call<TeamGateStatus>("team_gate.status", {})
          .then(onStatus)
          .catch(() => setWait(0));
        setMessage(null);
      }
      setWait((w) => Math.max(0, w - 1));
    }, 1000);
    return () => window.clearTimeout(t);
  }, [wait, onStatus]);

  const waiting = wait > 0;
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!password || busy || waiting) return;
    setBusy(true);
    try {
      const answer = await call<TeamGateAnswer>("team_gate.unlock", { password });
      setMessage(answer.message);
      if (answer.status.unlocked) void client.invalidateQueries();
      onStatus(answer.status);
    } catch {
      setMessage(TRY_FAILED);
    } finally {
      setPassword("");
      setBusy(false);
    }
  };

  return (
    <Dialog open onOpenChange={() => undefined} dismissible={false} title="Enter the team password" size="md">
      <form onSubmit={(e) => void submit(e)} className="col" style={{ gap: 12 }}>
        <p style={{ margin: 0 }}>
          This is an internal build of OpenFrame Studio for the team. It is not for sale. Enter the team password to open it.
        </p>
        <Field label="Team password" htmlFor="team-password">
          <TextInput
            id="team-password"
            type="password"
            autoComplete="off"
            autoFocus
            value={password}
            disabled={busy || waiting}
            onChange={(e) => setPassword(e.target.value)}
            aria-describedby={message ? "team-password-message" : undefined}
          />
        </Field>
        {message && (
          <div id="team-password-message" role="alert">
            <Banner tone="warn">{message}</Banner>
          </div>
        )}
        {waiting && (
          <p className="sm muted" style={{ margin: 0 }} aria-live="polite">
            Next try in {waitClock(wait)}.
          </p>
        )}
        <Button type="submit" variant="primary" icon={<LockKeyhole size={14} aria-hidden />} disabled={!password || busy || waiting}>
          {busy ? "Checking…" : "Open OpenFrame Studio"}
        </Button>
      </form>
    </Dialog>
  );
}
