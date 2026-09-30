// Vault Recently Deleted (mock 044, FSD §5.11): restore, or permanently delete
// as a deliberate, confirmed second step.

import { useState } from "react";
import { ArrowLeft, Trash2 } from "lucide-react";
import { reportError } from "../../ipc/query";
import type { VaultTrashRow } from "../../ipc/generated/VaultTrashRow";
import { Banner, Button, ConfirmDialog, EmptyState, PageHeader, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import { useVaultTrash, vault } from "./api";
import { relativeDay, scopeLabel, type Scope } from "./model";

export function DeletedView({ scope, onBack }: { scope: Scope; onBack: () => void }) {
  const trash = useVaultTrash(scope);
  const [purging, setPurging] = useState<VaultTrashRow | null>(null);
  const [busy, setBusy] = useState(false);
  const rows = trash.data ?? [];

  const restore = async (r: VaultTrashRow) => {
    try {
      const res = await vault.restore(scope, r.deleted.id);
      toast.undoable(res.message, scope);
    } catch (e) {
      reportError(e);
    }
  };
  const purge = async () => {
    if (!purging) return;
    setBusy(true);
    try {
      await vault.purge(scope, purging.deleted.id);
      toast.info(`Permanently deleted “${purging.deleted.title ?? "item"}”.`);
      setPurging(null);
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="content" style={{ display: "flex", flexDirection: "column" }}>
      <PageHeader
        title="Recently Deleted"
        sub={`Items you deleted can be restored. · ${scopeLabel(scope)}`}
        actions={<Button icon={<ArrowLeft size={14} />} onClick={onBack}>Back to Vault</Button>}
      />
      <Banner tone="info">Deleted items stay here until you permanently delete them. Nothing is gone after one click.</Banner>
      <div style={{ marginTop: 12, overflow: "auto", flex: 1 }}>
        {trash.isLoading ? (
          <Skeleton h={120} />
        ) : rows.length === 0 ? (
          <EmptyState icon={<Trash2 size={28} />} title="Nothing deleted">Items you delete from the Idea Vault will appear here.</EmptyState>
        ) : (
          <table className="tbl card">
            <thead>
              <tr>
                <th>Item</th>
                <th>Type</th>
                <th>Deleted</th>
                <th>Original place</th>
                <th aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.deleted.id}>
                  <td className="b">{r.deleted.title ?? "Untitled"}</td>
                  <td>{r.typeLabel}</td>
                  <td className="muted">
                    {relativeDay(r.deleted.deletedAt)} {new Date(r.deleted.deletedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                  </td>
                  <td>{r.originalPlace}</td>
                  <td style={{ textAlign: "right", whiteSpace: "nowrap" }}>
                    <Button size="sm" onClick={() => void restore(r)}>Restore</Button>{" "}
                    <Button size="sm" variant="danger" onClick={() => setPurging(r)}>Delete Permanently</Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
      <ConfirmDialog
        open={!!purging}
        onOpenChange={(v) => !v && setPurging(null)}
        title="Delete permanently?"
        confirmLabel="Delete Permanently"
        cancelLabel="Keep in Recently Deleted"
        danger
        busy={busy}
        onConfirm={() => void purge()}
      >
        <p style={{ marginTop: 0 }}>
          “{purging?.deleted.title ?? "This item"}” and any file stored with it will be removed from this computer. This can't be undone.
        </p>
      </ConfirmDialog>
    </div>
  );
}
