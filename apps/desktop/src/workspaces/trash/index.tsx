// Recently Deleted (FSD §52; UX §3.38; mock 157).
// "Everything you delete stays recoverable until you delete it permanently."
// Restore returns an item to its previous place; when that place no longer
// exists it goes to Unassigned / the top level and the user is told (FSD §52.5).
// Delete Permanently always asks for a second, deliberate confirmation (§52.2).

import { useEffect, useState } from "react";
import { RotateCcw, Search, Trash2 } from "lucide-react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { DeletedItemRow } from "../../ipc/generated/DeletedItemRow";
import type { DeletedItemView } from "../../ipc/generated/DeletedItemView";
import { Banner, Button, ConfirmDialog, EmptyState, PageHeader, Segmented, Skeleton } from "../../design-system";
import { useUi } from "../../app/stores";
import { toast } from "../../app/toast";
import { dayWord } from "../../app/home/format";

type Scope = "project" | "global";

/**
 * Human confirmation after a restore (FSD §52.5 — say where it went). The
 * backend's `restoredNote` (set when the item could not go back to its original
 * place) is authoritative; otherwise the list's view of the item is used.
 */
export function restoredMessage(item: DeletedItemView, restored?: Pick<DeletedItemRow, "restoredNote"> | null): string {
  const name = item.title ? `“${item.title}”` : `the ${item.typeLabel.toLowerCase()}`;
  if (restored?.restoredNote) return `Restored ${name}. ${restored.restoredNote}`;
  if (item.parentMissing) return `Restored ${name}. Its original place no longer exists, so it was put in Unassigned.`;
  return `Restored ${name} to ${item.wasIn}`;
}

export default function RecentlyDeleted() {
  const [scope, setScope] = useState<Scope>("project");
  const [text, setText] = useState("");
  const [purging, setPurging] = useState<DeletedItemView | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const setUndoScope = useUi((s) => s.setUndoScope);
  const items = useOp<DeletedItemView[]>("project.deleted_items", { store: scope }, ["deleted_item", "*"], { store: scope });

  // The Global Idea Vault has its own undo history; leave global undo pointing at the project.
  useEffect(() => () => useUi.getState().setUndoScope("project"), []);

  const changeScope = (s: Scope) => {
    setScope(s);
    setUndoScope(s);
  };

  const restore = async (item: DeletedItemView) => {
    setBusy(item.id);
    try {
      const row = await call<DeletedItemRow>("trash.restore", { store: scope, id: item.id });
      toast.undoable(restoredMessage(item, row), scope);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(null);
    }
  };

  const purge = async (item: DeletedItemView) => {
    setPurging(null);
    setBusy(item.id);
    try {
      await call("trash.purge", { store: scope, id: item.id });
      toast.info(`${item.title ? `“${item.title}”` : "The item"} was deleted permanently.`);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(null);
    }
  };

  const q = text.trim().toLowerCase();
  const all = items.data ?? [];
  const shown = all.filter((i) => !q || [i.title ?? "", i.typeLabel, i.wasIn].some((s) => s.toLowerCase().includes(q)));

  return (
    <div className="content" style={{ overflow: "auto" }}>
      <PageHeader
        title="Recently Deleted"
        sub="Everything you delete stays recoverable until you delete it permanently."
        actions={
          <Segmented
            ariaLabel="Which deleted items"
            value={scope}
            onChange={changeScope}
            options={[{ value: "project", label: "This project" }, { value: "global", label: "Global Idea Vault" }]}
          />
        }
      />
      {all.length > 0 && (
        <div className="toolbar">
          <div className="input" style={{ width: 260, padding: "0 10px" }}>
            <Search size={15} aria-hidden style={{ color: "var(--muted)" }} />
            <input type="search" aria-label="Search deleted items" placeholder="Search deleted items" value={text} onChange={(e) => setText(e.target.value)} style={{ border: 0, outline: "none", flex: 1, background: "transparent", minHeight: 30 }} />
          </div>
          <span className="sp" />
          <span className="xs muted">{all.length} item{all.length === 1 ? "" : "s"}</span>
        </div>
      )}
      {items.isLoading ? (
        <div className="col"><Skeleton h={32} /><Skeleton h={32} /></div>
      ) : items.isError ? (
        <EmptyState title="Recently Deleted couldn't be loaded." actions={<Button onClick={() => void items.refetch()}>Try Again</Button>}>
          {items.error?.message}
        </EmptyState>
      ) : all.length === 0 ? (
        <div className="card" style={{ padding: 8 }}>
          <EmptyState icon={<Trash2 size={30} />} title="Nothing has been deleted.">
            When you delete something it appears here and stays recoverable until you delete it permanently.
          </EmptyState>
        </div>
      ) : (
        <div className="card" style={{ overflow: "hidden" }}>
          {shown.length === 0 ? (
            <div className="muted" style={{ padding: 20, textAlign: "center" }}>No deleted items match “{text}”.</div>
          ) : (
            <table className="tbl" aria-label="Deleted items">
              <thead>
                <tr><th>Item</th><th>Type</th><th>Deleted</th><th>Was in</th><th><span className="sr-only">Actions</span></th></tr>
              </thead>
              <tbody>
                {shown.map((i) => (
                  <tr key={i.id}>
                    <td><b>{i.title || `Untitled ${i.typeLabel.toLowerCase()}`}</b></td>
                    <td>{i.typeLabel}</td>
                    <td title={new Date(i.deletedAt).toLocaleString()}>
                      {dayWord(i.deletedAt)}
                      {i.deletedByName && <div className="xs muted">by {i.deletedByName}</div>}
                    </td>
                    <td>
                      {i.wasIn}
                      {i.parentMissing && <div className="xs muted">That place no longer exists — restores to Unassigned</div>}
                    </td>
                    <td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                      <Button size="xs" icon={<RotateCcw size={12} />} disabled={busy === i.id} onClick={() => void restore(i)} aria-label={`Restore ${i.title ?? i.typeLabel}`}>Restore</Button>{" "}
                      <Button size="xs" variant="danger" disabled={busy === i.id} onClick={() => setPurging(i)} aria-label={`Delete ${i.title ?? i.typeLabel} permanently`}>Delete Permanently</Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
      <div style={{ marginTop: 12 }}>
        <Banner tone="warn">
          <b>Delete Permanently asks for a second, deliberate confirmation. If the original place no longer exists, Restore puts the item in an Unassigned area and tells you.</b>
        </Banner>
      </div>
      {purging && (
        <ConfirmDialog
          open
          onOpenChange={(v) => !v && setPurging(null)}
          title={`Delete ${purging.title ? `“${purging.title}”` : `this ${purging.typeLabel.toLowerCase()}`} permanently?`}
          confirmLabel="Delete Permanently"
          cancelLabel="Keep in Recently Deleted"
          danger
          onConfirm={() => void purge(purging)}
        >
          <p style={{ marginTop: 0 }}>
            This {purging.typeLabel.toLowerCase()} will be removed from the project for good. This can't be undone, and it can't be restored afterwards.
          </p>
          <p className="muted sm" style={{ marginBottom: 0 }}>Anything that was created from it elsewhere in the project is not affected.</p>
        </ConfirmDialog>
      )}
    </div>
  );
}
