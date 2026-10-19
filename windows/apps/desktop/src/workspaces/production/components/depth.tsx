// Shared production depth pieces: availability windows (locations, cast, crew), contact
// fields that respect the contact privacy rule, and the explicit references shown before a
// delete. Rust validates and stores everything; these components only edit and display.

import { useState } from "react";
import { Plus, X } from "lucide-react";
import { useCommand, useOp } from "../../../ipc/query";
import type { AvailabilityWindowDto } from "../../../ipc/generated/AvailabilityWindowDto";
import type { AvailabilityWindowInput } from "../../../ipc/generated/AvailabilityWindowInput";
import type { AvailabilityOwner } from "../../../ipc/generated/AvailabilityOwner";
import type { AvailabilityKind } from "../../../ipc/generated/AvailabilityKind";
import type { ProductionRecordKind } from "../../../ipc/generated/ProductionRecordKind";
import type { ProductionReferences } from "../../../ipc/generated/ProductionReferences";
import { Button, Chip, Field, IconButton, Select, Skeleton, TextInput } from "../../../design-system";
import { PROD_TABLES } from "../../../api/production";
import { toast } from "../../../app/toast";
import { BufferedField, SceneChips } from "./fields";

const KIND_OPTIONS: { value: AvailabilityKind; label: string }[] = [
  { value: "Unavailable", label: "Not available" },
  { value: "Available", label: "Available" },
];

function range(w: AvailabilityWindowDto): string {
  const fmt = (d: string) => {
    const [y, m, day] = d.split("-").map(Number);
    return new Date(y, m - 1, day).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
  };
  return w.startDate === w.endDate ? fmt(w.startDate) : `${fmt(w.startDate)} to ${fmt(w.endDate)}`;
}

/** Dated availability (inclusive ranges). Saving replaces the owner's list as one undo step. */
export function AvailabilityEditor({ ownerType, ownerId, windows, canEdit, name }: {
  ownerType: AvailabilityOwner;
  ownerId: string;
  windows: AvailabilityWindowDto[];
  canEdit: boolean;
  name: string;
}) {
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [kind, setKind] = useState<AvailabilityKind>("Unavailable");
  const [note, setNote] = useState("");
  const save = useCommand<{ ownerType: AvailabilityOwner; ownerId: string; windows: AvailabilityWindowInput[] }, AvailabilityWindowDto[]>("availability.set", {
    onSuccess: () => toast.undoable(`Updated availability of ${name}`),
  });
  const asInput = (w: AvailabilityWindowDto): AvailabilityWindowInput => ({ startDate: w.startDate, endDate: w.endDate, kind: w.kind, note: w.note });
  const add = () => {
    if (!start) return;
    save.mutate(
      { ownerType, ownerId, windows: [...windows.map(asInput), { startDate: start, endDate: end || start, kind, note: note.trim() || null }] },
      {
        onSuccess: () => {
          setStart("");
          setEnd("");
          setNote("");
        },
      },
    );
  };
  const remove = (id: string) => save.mutate({ ownerType, ownerId, windows: windows.filter((w) => w.id !== id).map(asInput) });
  const prefix = `av-${ownerType}`;
  return (
    <Field label="Availability" hint="Dates are advisory: the schedule warns about them and never moves a scene by itself.">
      <div role="list" aria-label={`Availability of ${name}`}>
        {windows.length === 0 && <div className="xs muted">No dates entered.</div>}
        {windows.map((w) => (
          <div key={w.id} role="listitem" className="row" style={{ gap: 6, marginBottom: 4 }}>
            <Chip tone={w.kind === "Unavailable" ? "r" : "g"}>{w.kind === "Unavailable" ? "Not available" : "Available"}</Chip>
            <span className="sm grow">
              {range(w)}
              {w.note && <span className="muted">: {w.note}</span>}
            </span>
            {canEdit && (
              <IconButton label={`Remove ${range(w)}`} disabled={save.isPending} onClick={() => remove(w.id)}>
                <X size={13} />
              </IconButton>
            )}
          </div>
        ))}
      </div>
      {canEdit && (
        <form
          className="row wrap"
          style={{ gap: 6, marginTop: 6 }}
          aria-label={`Add availability for ${name}`}
          onSubmit={(e) => {
            e.preventDefault();
            add();
          }}
        >
          <Select<AvailabilityKind> id={`${prefix}-kind`} ariaLabel="Availability" value={kind} options={KIND_OPTIONS} onChange={setKind} />
          <TextInput id={`${prefix}-start`} aria-label="First day" type="date" value={start} onChange={(e) => setStart(e.target.value)} style={{ width: 150 }} />
          <TextInput id={`${prefix}-end`} aria-label="Last day" type="date" value={end} min={start || undefined} onChange={(e) => setEnd(e.target.value)} style={{ width: 150 }} />
          <TextInput id={`${prefix}-note`} aria-label="Availability note" placeholder="Note (optional)" value={note} maxLength={500} onChange={(e) => setNote(e.target.value)} style={{ flex: "1 1 120px" }} />
          <Button size="sm" type="submit" icon={<Plus size={13} />} disabled={!start || save.isPending}>
            Add Dates
          </Button>
        </form>
      )}
    </Field>
  );
}

/** A contact field. Roles that may not see contacts get a plain statement instead. */
export function ContactField({ id, label, value, hidden, canEdit, onCommit, maxLength = 500 }: {
  id: string;
  label: string;
  value: string | null;
  hidden: boolean;
  canEdit: boolean;
  onCommit: (v: string) => void;
  maxLength?: number;
}) {
  if (hidden) {
    return (
      <Field label={label}>
        <div className="sm muted">Contact details are hidden for your role.</div>
      </Field>
    );
  }
  return (
    <BufferedField
      id={id}
      label={label}
      value={value}
      disabled={!canEdit}
      maxLength={maxLength}
      onCommit={onCommit}
      hint="Private: visible to Owners and Editors only. Never shared with the assistant or put in exports."
    />
  );
}

/** What still refers to a record, and what deleting it does (stable identity rule). */
export function ReferencesNote({ kind, id }: { kind: ProductionRecordKind; id: string }) {
  const refs = useOp<ProductionReferences>("production.references", { kind, id }, PROD_TABLES);
  if (refs.isLoading) return <Skeleton h={40} />;
  if (!refs.data) return null;
  const r = refs.data;
  return (
    <div>
      <p className="sm" style={{ marginTop: 0 }}>{r.consequence}</p>
      {r.scenes.length > 0 && (
        <div style={{ marginBottom: 6 }}>
          <div className="xs muted" style={{ marginBottom: 4 }}>Used in</div>
          <SceneChips scenes={r.scenes} />
        </div>
      )}
      {r.links.length > 0 && (
        <ul className="sm" style={{ margin: "4px 0 0 18px", padding: 0 }} aria-label="Linked records">
          {r.links.map((l) => (
            <li key={l}>{l}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
