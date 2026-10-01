// "Proposed Changes" review card (agentic spec §7, §36; FSD §42.7–§42.15).
//
// Every AI-prepared project change arrives here as ONE Change Set: summary,
// affected areas and an exact preview, then the user's explicit choice
// [Reject] [Apply Changes]. Nothing is applied by the assistant itself.
// A stale or conflicting proposal can never be applied from this card — it must
// be re-checked against the current project first. Destructive changes also
// need the product's usual confirmation.

import { useState } from "react";
import { call } from "../ipc/client";
import { reportError } from "../ipc/query";
import { Button, ConfirmDialog } from "../design-system";
import { toast } from "../app/toast";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import { STALE_HINT, changeCount, changeSetActions, changeSetStateText } from "./model";

export function ProposedChanges({ cs, onChanged }: { cs: AiChangeSetDto; onChanged: () => void }) {
  const [busy, setBusy] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const actions = changeSetActions(cs);
  const stateText = changeSetStateText(cs);
  const stale = cs.state === "Stale" || cs.state === "Conflict";

  const run = (op: string, args: Record<string, unknown>, done?: (r: AiChangeSetDto) => void) => {
    setBusy(true);
    call<AiChangeSetDto>(op, { id: cs.id, ...args })
      .then((r) => {
        done?.(r);
        onChanged();
      })
      .catch(reportError)
      .finally(() => setBusy(false));
  };

  const apply = (confirmDestructive: boolean) =>
    run("ai.change_set.accept", confirmDestructive ? { confirmDestructive: true } : {}, (r) => {
      if (r.state === "Applied") toast.undoable(`Applied: ${r.title}`);
    });

  return (
    <>
      <section className={`cs ai-review${stale ? " ai-stale" : ""}`} aria-label="Proposed Changes" role={stale ? "alert" : undefined}>
        <div className="csh">{stale ? "Proposed Changes — out of date" : "Proposed Changes"}</div>
        <div className="csr ai-block">
          <b>{cs.title}</b>
          {cs.summary && <div className="sm">{cs.summary}</div>}
          {stale && <div className="sm">{cs.staleReason ?? "The project changed after this suggestion was prepared. Review is required before applying it."}</div>}
        </div>
        <div className="csr">
          <span>Affected areas</span>
          <b>{cs.affectedModules.join(", ") || "—"}</b>
        </div>
        {cs.rows.map((row, i) =>
          row.tone === "section" ? (
            <div key={`r${i}`} className="csr ai-section">
              <b>{row.label}</b>
              <span className="xs muted">{row.value}</span>
            </div>
          ) : (
            <div key={`r${i}`} className={`csr${row.tone === "destructive" ? " ai-destructive" : ""}`}>
              <span>{row.label}</span>
              <b>{row.value}</b>
            </div>
          ),
        )}
        {cs.exclusions.map((row, i) => (
          <div key={`x${i}`} className={`csr ${row.tone === "locked" ? "ai-locked" : "ai-excluded"}`}>
            <span>{row.label}</span>
            <b>{row.value}</b>
          </div>
        ))}
        {cs.targets.length > 0 && (
          <div className="csr ai-block">
            <span className="xs muted">Items: {cs.targets.map((t) => t.label).join(", ")}</span>
          </div>
        )}
        {cs.state === "Pending" && (
          <div className="csr ai-block">
            <span className="xs muted">
              {changeCount(cs.operationCount)}
              {cs.partCount > 1 ? ` in ${cs.partCount} parts` : ""}, applied together as one step you can undo. If any part fails, nothing is changed.
            </span>
          </div>
        )}
      </section>
      {stateText && (
        <div className={`xs ${cs.state === "Failed" ? "" : "muted"}`} role={cs.state === "Failed" ? "alert" : "status"}>
          {stateText}
        </div>
      )}
      {(actions.apply || actions.reject || actions.recheck) && (
        <div className="row">
          {actions.reject && (
            <Button size="sm" variant="ghost" disabled={busy} onClick={() => run("ai.change_set.reject", {})}>
              Reject
            </Button>
          )}
          {actions.recheck && (
            <Button size="sm" variant="primary" disabled={busy} onClick={() => run("ai.change_set.recheck", {})}>
              Re-check &amp; Review
            </Button>
          )}
          {actions.apply && (
            <Button size="sm" variant="primary" disabled={busy} onClick={() => (cs.requiresConfirmation ? setConfirmOpen(true) : apply(false))}>
              Apply Changes
            </Button>
          )}
        </div>
      )}
      {stale && <div className="hint">{STALE_HINT}</div>}
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title="Apply changes that delete items?"
        confirmLabel="Apply Changes"
        danger
        busy={busy}
        onConfirm={() => {
          setConfirmOpen(false);
          apply(true);
        }}
      >
        Some of these changes delete or permanently change items. Deleted items go to Recently Deleted, and you can use Undo to reverse the whole change.
      </ConfirmDialog>
    </>
  );
}
