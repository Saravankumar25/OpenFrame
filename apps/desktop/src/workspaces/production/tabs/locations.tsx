// Locations (FSD §29, §99; UX §3.24, mocks 115–117):
// "Which places can we use? A practical scouting notebook."

import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, ArrowRight, ImagePlus, MapPin, Plus, Trash2 } from "lucide-react";
import { call } from "../../../ipc/client";
import { reportError, useCommand, useOp } from "../../../ipc/query";
import type { ProductionLocationDto } from "../../../ipc/generated/ProductionLocationDto";
import type { ProductionLocationNotes } from "../../../ipc/generated/ProductionLocationNotes";
import type { ProductionLocationStatus } from "../../../ipc/generated/ProductionLocationStatus";
import type { ProductionLocationReplaceResult } from "../../../ipc/generated/ProductionLocationReplaceResult";
import {
  Banner,
  Button,
  Checkbox,
  Chip,
  ConfirmDialog,
  ContextMenu,
  Dialog,
  Drawer,
  EmptyState,
  Field,
  IconButton,
  PageHeader,
  Segmented,
  Select,
  Skeleton,
  TextInput,
} from "../../../design-system";
import { LOCATION_STATUSES, PROD_TABLES, assetUrl, locationStatusTone, useCanEdit } from "../../../api/production";
import { useIntent, useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import { BufferedField, SceneChips, pickImages } from "../components/fields";
import type { ProductionLocationCreateArgs } from "../../../ipc/generated/ProductionLocationCreateArgs";
import type { ProductionLocationReplaceArgs } from "../../../ipc/generated/ProductionLocationReplaceArgs";
import type { ProductionLocationUpdateArgs } from "../../../ipc/generated/ProductionLocationUpdateArgs";

export const tab = { id: "locations", label: "Locations", order: 20 };

type Filter = "all" | ProductionLocationStatus;
const FILTERS: { value: Filter; label: string }[] = [{ value: "all", label: "All" }, ...LOCATION_STATUSES.map((s) => ({ value: s, label: s }))];

/** Practical notes as labelled lines (UX §3.24). */
const NOTE_FIELDS: { key: keyof ProductionLocationNotes; label: string; placeholder: string }[] = [
  { key: "parking", label: "Parking", placeholder: "Where can the unit park?" },
  { key: "noise", label: "Noise", placeholder: "Traffic, trains, neighbours…" },
  { key: "permission", label: "Permission", placeholder: "Who approves, what is needed" },
  { key: "access", label: "Access", placeholder: "Gates, keys, stairs, load-in" },
  { key: "power", label: "Power", placeholder: "Sockets, generator needed?" },
  { key: "toilets", label: "Toilets", placeholder: "Available for the crew?" },
  { key: "facilities", label: "Nearby facilities", placeholder: "Food, shelter, hospital…" },
  { key: "travel", label: "Travel", placeholder: "Distance and route from base" },
  { key: "general", label: "Other notes", placeholder: "Anything else" },
];

function Photo({ loc, height }: { loc: ProductionLocationDto; height: number }) {
  const url = assetUrl(loc.photos[0]?.asset);
  return url ? (
    <img src={url} alt="" style={{ width: "100%", height, objectFit: "cover", display: "block" }} />
  ) : (
    <div className="photo station" style={{ width: "100%", height, display: "flex", alignItems: "center", justifyContent: "center", color: "#fff9" }} aria-hidden>
      <MapPin size={20} />
    </div>
  );
}

export default function LocationsTab() {
  const route = useNav((s) => s.route);
  const canEdit = useCanEdit();
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [showArchived, setShowArchived] = useState(false);
  const paramId = route.params?.locationId ?? null;
  const [selected, setSelected] = useState<string | null>(paramId);
  const [lastParam, setLastParam] = useState(paramId);
  if (paramId !== lastParam) {
    setLastParam(paramId);
    if (paramId) setSelected(paramId);
  }
  const [adding, setAdding] = useState(false);
  const [replacing, setReplacing] = useState<ProductionLocationDto | null>(null);
  // "New Location" from the shell quick-actions menu.
  const intent = useIntent((s) => s.intent);
  useEffect(() => {
    if (intent === "production.new_location" && useIntent.getState().consume("production.new_location")) setAdding(true);
  }, [intent]);
  const [deleting, setDeleting] = useState<ProductionLocationDto | null>(null);
  const args = useMemo(() => ({ status: filter === "all" ? null : filter, search: search.trim() || null, includeArchived: showArchived }), [filter, search, showArchived]);
  const locs = useOp<ProductionLocationDto[]>("locations.list", args, PROD_TABLES);
  const list = locs.data ?? [];
  const current = list.find((l) => l.id === selected) ?? null;
  const detail = useOp<ProductionLocationDto>("locations.get", { id: selected }, PROD_TABLES, { enabled: !!selected && !current });
  const loc = current ?? (selected ? detail.data ?? null : null);

  const setArchived = (l: ProductionLocationDto, archived: boolean) =>
    void call("locations.set_archived", { id: l.id, archived })
      .then(() => toast.undoable(archived ? `Archived “${l.name}”` : `Restored “${l.name}” from the archive`))
      .catch(reportError);

  return (
    <div>
      <PageHeader
        title="Locations"
        sub="Which places can we use? A practical scouting notebook."
        actions={canEdit && <Button variant="primary" icon={<Plus size={15} />} onClick={() => setAdding(true)}>Add Location</Button>}
      />
      <div className="toolbar">
        <TextInput aria-label="Search locations" placeholder="Search locations" value={search} onChange={(e) => setSearch(e.target.value)} style={{ flex: "0 0 230px" }} />
        <Segmented ariaLabel="Status" value={filter} onChange={setFilter} options={FILTERS} />
        <span className="sp" />
        <Checkbox checked={showArchived} onChange={setShowArchived} label="Show archived" />
      </div>
      {locs.isLoading ? (
        <Skeleton h={140} />
      ) : locs.error ? (
        <Banner tone="err">{locs.error.message}</Banner>
      ) : list.length === 0 ? (
        search || filter !== "all" ? (
          <div className="sm muted" style={{ padding: 12 }}>No locations match.</div>
        ) : (
          <EmptyState title="No locations yet." actions={canEdit && <Button variant="primary" onClick={() => setAdding(true)}>Add Location</Button>}>
            Add a location you are considering for the film. Only the name is required.
          </EmptyState>
        )
      ) : (
        <div className="grid g4">
          {list.map((l) => (
            <ContextMenu
              key={l.id}
              items={[
                { label: "Open", onSelect: () => setSelected(l.id) },
                { label: l.archived ? "Restore from Archive" : "Archive", disabled: !canEdit, onSelect: () => setArchived(l, !l.archived) },
                { label: "Delete…", danger: true, separatorBefore: true, disabled: !canEdit, onSelect: () => setDeleting(l) },
              ]}
            >
              <button type="button" className={`vcard${l.id === selected ? " sel" : ""}`} style={{ textAlign: "left", padding: 0, opacity: l.archived ? 0.55 : 1 }} onClick={() => setSelected(l.id)}>
                <Photo loc={l} height={72} />
                <div className="vb">
                  <div className="vt">{l.name}</div>
                  <div className="row">
                    <Chip tone={locationStatusTone(l.status)}>{l.status}</Chip>
                    <span className="xs muted">{l.sceneCount} scene{l.sceneCount === 1 ? "" : "s"}</span>
                    {l.archived && <Chip>Archived</Chip>}
                  </div>
                </div>
              </button>
            </ContextMenu>
          ))}
        </div>
      )}
      {loc && (
        <LocationDrawer
          key={loc.id}
          loc={loc}
          canEdit={canEdit}
          onClose={() => setSelected(null)}
          onReplace={() => setReplacing(loc)}
          onArchive={() => setArchived(loc, !loc.archived)}
          onDelete={() => setDeleting(loc)}
        />
      )}
      {adding && <AddLocationDialog onClose={() => setAdding(false)} onCreated={setSelected} />}
      {replacing && <ReplaceLocationDialog loc={replacing} onClose={() => setReplacing(null)} />}
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(v) => !v && setDeleting(null)}
        title={`Delete “${deleting?.name ?? ""}”?`}
        confirmLabel="Delete"
        danger
        onConfirm={() => {
          const d = deleting;
          if (!d) return;
          void call("locations.delete", { id: d.id })
            .then(() => {
              toast.undoable(`Deleted “${d.name}”. Restore it from Recently Deleted if needed.`);
              setDeleting(null);
              setSelected(null);
            })
            .catch(reportError);
        }}
      >
        <p className="sm" style={{ marginTop: 0 }}>
          {deleting && deleting.sceneCount > 0 ? `It is used in ${deleting.sceneCount} scene${deleting.sceneCount === 1 ? "" : "s"}. ` : ""}
          Scene breakdowns and schedule history stay readable, and the screenplay is not changed. You can restore the location from Recently Deleted.
        </p>
      </ConfirmDialog>
    </div>
  );
}

function LocationDrawer({ loc, canEdit, onClose, onReplace, onArchive, onDelete }: {
  loc: ProductionLocationDto;
  canEdit: boolean;
  onClose: () => void;
  onReplace: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  const go = useNav((s) => s.go);
  const update = useCommand<ProductionLocationUpdateArgs, ProductionLocationDto>("locations.update");
  const status = useCommand<{ id: string; status: ProductionLocationStatus }, ProductionLocationDto>("locations.set_status");
  const setNote = (key: keyof ProductionLocationNotes, v: string) => update.mutate({ id: loc.id, notes: { ...loc.notes, [key]: v || null } });
  const addPhotos = async () => {
    try {
      const paths = await pickImages(true);
      if (paths.length) await call("locations.add_photos", { id: loc.id, paths });
    } catch (e) {
      reportError(e);
    }
  };
  const move = (photoId: string, index: number) => void call("locations.move_photo", { photoId, index }).catch(reportError);
  const removePhoto = (photoId: string) =>
    void call("locations.remove_photo", { id: photoId }).then(() => toast.undoable("Photo removed")).catch(reportError);
  const inSourceScenes = loc.scenes.filter((s) => s.inSource).length;
  return (
    <Drawer
      open
      onClose={onClose}
      width="w"
      typeLabel="Location"
      title={loc.name}
      footer={
        <>
          {canEdit && <Button variant="ghost" onClick={onDelete}>Delete</Button>}
          <span className="grow" />
          {canEdit && <Button onClick={onArchive}>{loc.archived ? "Restore" : "Archive"}</Button>}
          <Button disabled={loc.scenes.length === 0} onClick={() => go({ workspace: "breakdown", params: { sceneId: loc.scenes[0].sceneId } })}>Open Scenes</Button>
        </>
      }
    >
      <div style={{ marginBottom: 10 }}>
        <Segmented<ProductionLocationStatus>
          ariaLabel="Location status"
          value={loc.status}
          onChange={(v) => canEdit && v !== loc.status && status.mutate({ id: loc.id, status: v })}
          options={LOCATION_STATUSES.map((s) => ({ value: s, label: s }))}
        />
        <div className="xs muted" style={{ marginTop: 4 }}>A status change never alters the screenplay.</div>
      </div>
      {loc.status === "Rejected" && inSourceScenes > 0 && (
        <div style={{ marginBottom: 10 }}>
          <Banner tone="warn" actions={canEdit && <Button size="xs" onClick={onReplace}>Replace…</Button>}>
            Rejected, but still used in {inSourceScenes} scene{inSourceScenes === 1 ? "" : "s"}.
          </Banner>
        </div>
      )}
      {loc.replacement && <div className="sm muted" style={{ marginBottom: 8 }}>Practical replacement: <b>{loc.replacement.name}</b></div>}
      <Field label="Photos" hint="The first photo is the thumbnail. Photos are stored in the project and work offline.">
        <div className="grid g3" style={{ gap: 6 }}>
          {loc.photos.map((p, i) => {
            const url = assetUrl(p.asset);
            return (
              <div key={p.id} style={{ position: "relative" }}>
                {url ? (
                  <img src={url} alt={`${loc.name} photo ${i + 1}`} style={{ width: "100%", height: 70, objectFit: "cover", borderRadius: 6, display: "block" }} />
                ) : (
                  <div className="photo" style={{ height: 70, borderRadius: 6 }}><span className="lb">Unavailable</span></div>
                )}
                {canEdit && (
                  <div className="row" style={{ position: "absolute", top: 2, right: 2, gap: 0, background: "#fffd", borderRadius: 6 }}>
                    <IconButton label="Move earlier" disabled={i === 0} onClick={() => move(p.id, i - 1)}><ArrowLeft size={12} /></IconButton>
                    <IconButton label="Move later" disabled={i === loc.photos.length - 1} onClick={() => move(p.id, i + 1)}><ArrowRight size={12} /></IconButton>
                    <IconButton label="Remove photo" onClick={() => removePhoto(p.id)}><Trash2 size={12} /></IconButton>
                  </div>
                )}
              </div>
            );
          })}
        </div>
        {canEdit && <div style={{ marginTop: 6 }}><Button size="sm" icon={<ImagePlus size={14} />} onClick={() => void addPhotos()}>Add Photos</Button></div>}
      </Field>
      <BufferedField id="loc-name" label="Name" required value={loc.name} disabled={!canEdit} maxLength={200} onCommit={(v) => update.mutate({ id: loc.id, name: v })} />
      <BufferedField id="loc-addr" label="Address / area" value={loc.address} disabled={!canEdit} maxLength={1000} onCommit={(v) => update.mutate({ id: loc.id, address: v })} />
      <BufferedField id="loc-contact" label="Contact" value={loc.contact} disabled={!canEdit} maxLength={500} onCommit={(v) => update.mutate({ id: loc.id, contact: v })} />
      <div className="h4" style={{ marginTop: 8 }}>Practical notes</div>
      {NOTE_FIELDS.map((f) => (
        <BufferedField
          key={f.key}
          id={`loc-note-${f.key}`}
          label={`${f.label}:`}
          value={loc.notes[f.key] ?? null}
          placeholder={f.placeholder}
          disabled={!canEdit}
          maxLength={4000}
          onCommit={(v) => setNote(f.key, v)}
        />
      ))}
      <Field label="Scenes">
        <SceneChips scenes={loc.scenes} empty="No scenes use this location yet. Tag it in the Breakdown (Location / Set)." />
      </Field>
    </Drawer>
  );
}

function AddLocationDialog({ onClose, onCreated }: { onClose: () => void; onCreated: (id: string) => void }) {
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [contact, setContact] = useState("");
  const [status, setStatus] = useState<ProductionLocationStatus>("Idea");
  const create = useCommand<ProductionLocationCreateArgs, ProductionLocationDto>("locations.create", {
    onSuccess: (l) => {
      toast.undoable(`Added location “${l.name}”`);
      onCreated(l.id);
      onClose();
    },
  });
  const submit = () => name.trim() && create.mutate({ name: name.trim(), address: address.trim() || null, contact: contact.trim() || null, status });
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="Add Location"
      sub="Only the name is required. Confirming a location does not assign it to scenes."
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!name.trim() || create.isPending} onClick={submit}>Add Location</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <Field label="Name" required htmlFor="nl-name">
          <TextInput id="nl-name" autoFocus value={name} maxLength={200} onChange={(e) => setName(e.target.value)} placeholder="e.g. Old Railway Station" />
        </Field>
        <Field label="Address / area" htmlFor="nl-addr">
          <TextInput id="nl-addr" value={address} maxLength={1000} onChange={(e) => setAddress(e.target.value)} />
        </Field>
        <Field label="Contact" htmlFor="nl-contact">
          <TextInput id="nl-contact" value={contact} maxLength={500} onChange={(e) => setContact(e.target.value)} />
        </Field>
        <Field label="Status" htmlFor="nl-status">
          <Select id="nl-status" value={status} options={LOCATION_STATUSES.map((s) => ({ value: s, label: s }))} onChange={setStatus} />
        </Field>
      </form>
    </Dialog>
  );
}

/** "Replace a rejected location" (mock 117). */
function ReplaceLocationDialog({ loc, onClose }: { loc: ProductionLocationDto; onClose: () => void }) {
  const all = useOp<ProductionLocationDto[]>("locations.list", { status: null, search: null, includeArchived: false }, PROD_TABLES);
  const options = (all.data ?? []).filter((l) => l.id !== loc.id && l.status !== "Rejected");
  const inSource = loc.scenes.filter((s) => s.inSource);
  const [target, setTarget] = useState("");
  const [scenes, setScenes] = useState<Set<string>>(new Set(inSource.map((s) => s.sceneId)));
  const replace = useCommand<ProductionLocationReplaceArgs, ProductionLocationReplaceResult>("locations.replace", {
    onSuccess: (r) => {
      toast.undoable(`Replacement applied to ${r.scenesUpdated} scene${r.scenesUpdated === 1 ? "" : "s"}. The screenplay was not changed.`);
      onClose();
    },
  });
  const chosen = target || options.find((o) => o.status === "Confirmed")?.id || options[0]?.id || "";
  const nums = inSource.map((s) => s.number).join(", ");
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      size="md"
      title="Replace a rejected location"
      sub={`${loc.name} is ${loc.status}. It is used in ${inSource.length} scene${inSource.length === 1 ? "" : "s"}${nums ? ` (${nums})` : ""}.`}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!chosen || scenes.size === 0 || replace.isPending} onClick={() => replace.mutate({ locationId: loc.id, replacementId: chosen, sceneIds: Array.from(scenes) })}>
            Apply Replacement
          </Button>
        </>
      }
    >
      {options.length === 0 ? (
        <Banner tone="info">Add the practical replacement as a location first (Idea, Shortlisted or Confirmed).</Banner>
      ) : (
        <Field label="Practical replacement" htmlFor="rl-target">
          <Select id="rl-target" value={chosen} options={options.map((o) => ({ value: o.id, label: `${o.name} (${o.status})` }))} onChange={setTarget} />
        </Field>
      )}
      <Field label="Apply to scenes">
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {inSource.map((s) => (
            <Checkbox
              key={s.sceneId}
              checked={scenes.has(s.sceneId)}
              onChange={(v) => {
                const next = new Set(scenes);
                if (v) next.add(s.sceneId);
                else next.delete(s.sceneId);
                setScenes(next);
              }}
              label={`Scene ${s.number} — ${s.heading}`}
            />
          ))}
        </div>
      </Field>
      <div className="xs muted">
        This updates production associations only. The screenplay text (which still says {loc.name.toUpperCase()}) is not changed; edit the script yourself if you want that.
      </div>
    </Dialog>
  );
}
