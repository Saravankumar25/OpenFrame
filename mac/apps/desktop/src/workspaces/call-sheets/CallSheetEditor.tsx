// Paper-style call sheet editor (UX §3.31; production depth spec §29 to §32; mocks 143 to 147).
// The six default sections are always shown; the rest come in through "Add Section" and stay
// hidden while they are not added. Derived values (scenes, times, cast, locations) come from the
// schedule; editable fields use the dashed `.edt` style and values nobody has entered read
// "Needs input" in the red `.need` style. Edits are saved to the call sheet only, never back into
// the screenplay, the breakdown or the schedule. Status words: Draft, Needs refresh, Final.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { ArrowLeft, Paperclip, Plus, X } from "lucide-react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Banner, Button, Chip, Dialog, IconButton, Menu, PageHeader, Skeleton, type MenuItemSpec } from "../../design-system";
import { call, inTauri, OpError } from "../../ipc/client";
import { assetSrc } from "../../ipc/assetSrc";
import { reportError, useOp } from "../../ipc/query";
import { toast } from "../../app/toast";
import { usePendingSave } from "../../app/pendingSaves";
import { useNav } from "../../app/stores";
import type { AssetInfo } from "../../ipc/generated/AssetInfo";
import type { CallSheetAttachment } from "../../ipc/generated/CallSheetAttachment";
import type { CallSheetDetail, CallSheetDocument, CallSheetRefreshPreview } from "../../api/schedule";
import { CALL_SHEET_TABLES, callSheetStatusWord, callSheetTone, formatDateTime } from "../../api/schedule";
import { CallSheetExportButton } from "../../features/export/buttons";

type Doc = CallSheetDocument;
const SAVE_DELAY_MS = 500;
/** What the sheet shows where nobody has entered a value yet (never invented text). */
const NEEDS_INPUT = "Needs input";

// ------------------------------------------------------------------ small editable inputs

function Edt({ value, onChange, label, need, placeholder, disabled, wide, inputRef, invalid }: {
  value: string;
  onChange: (v: string) => void;
  label: string;
  need?: boolean;
  placeholder?: string;
  disabled?: boolean;
  wide?: boolean;
  inputRef?: React.Ref<HTMLInputElement>;
  invalid?: boolean;
}) {
  const missing = need && !value.trim();
  const shown = missing ? NEEDS_INPUT : placeholder;
  return (
    <input
      ref={inputRef}
      className={`edt${missing ? " need" : ""}${invalid ? " of-crew-invalid" : ""}`}
      aria-label={label}
      aria-invalid={missing || invalid || undefined}
      value={value}
      placeholder={shown}
      disabled={disabled}
      size={wide ? undefined : Math.max(8, Math.min(40, (value || shown || "").length + 1))}
      style={wide ? { width: "100%" } : undefined}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

function EdtArea({ value, onChange, label, disabled, need }: { value: string; onChange: (v: string) => void; label: string; disabled?: boolean; need?: boolean }) {
  const missing = need && !value.trim();
  return (
    <textarea
      className={`edt${missing ? " need" : ""}`}
      aria-label={label}
      aria-invalid={missing || undefined}
      value={value}
      placeholder={missing ? NEEDS_INPUT : undefined}
      disabled={disabled}
      rows={2}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

/** A label and a one-line editable value. */
function Fld({ label, value, onChange, disabled, need }: { label: string; value: string; onChange: (v: string) => void; disabled?: boolean; need?: boolean }) {
  return (
    <div className="of-field">
      <span>{label}</span>
      <Edt wide value={value} onChange={onChange} label={label} need={need} disabled={disabled} />
    </div>
  );
}

function Section({ title, children, onRemove, disabled }: { title: string; children: ReactNode; onRemove?: () => void; disabled?: boolean }) {
  return (
    <section aria-label={title}>
      <div className="st" style={{ display: "flex", alignItems: "center" }}>
        <span style={{ flex: 1 }}>{title}</span>
        {onRemove && !disabled && (
          <button type="button" aria-label={`Remove ${title}`} onClick={onRemove} style={{ background: "none", border: 0, color: "#fff" /* document colour: white icon on the call-sheet section header */, cursor: "pointer" }}>
            <X size={12} />
          </button>
        )}
      </div>
      {children}
    </section>
  );
}

function RemoveRow({ label, onClick, disabled }: { label: string; onClick: () => void; disabled?: boolean }) {
  if (disabled) return null;
  return (
    <IconButton label={label} onClick={onClick}>
      <X size={12} />
    </IconButton>
  );
}

function Thumb({ att }: { att: CallSheetAttachment }) {
  const q = useOp<AssetInfo>("files.asset", { assetId: att.assetId }, ["asset"]);
  const src = assetSrc(q.data);
  if (!src) return <span className="chip">{att.name}</span>;
  return <img className="of-thumb" src={src} alt={att.name} title={att.name} />;
}

// ------------------------------------------------------------------ dialogs

function ChangeList({ title, added, removed, changed }: { title: string; added: string[]; removed: string[]; changed: string[] }) {
  if (added.length + removed.length + changed.length === 0) return null;
  return (
    <div className="sm" style={{ marginTop: 4 }}>
      <b>{title}:</b>
      {added.length > 0 && <> added {added.join(", ")}.</>}
      {removed.length > 0 && <> removed {removed.join(", ")}.</>}
      {changed.length > 0 && <> changed {changed.join(", ")}.</>}
    </div>
  );
}

function RefreshDialog({ id, onClose, onApplied }: { id: string; onClose: () => void; onApplied: (newId?: string) => void }) {
  const q = useOp<CallSheetRefreshPreview>("callsheets.refresh_preview", { id }, CALL_SHEET_TABLES);
  const [busy, setBusy] = useState(false);
  const p = q.data;
  const apply = async () => {
    if (!p) return;
    setBusy(true);
    try {
      if (p.createsRevision) {
        const newId = await call<string>("callsheets.new_revision", { id });
        toast.undoable("Started a new call sheet revision. The final one is unchanged.");
        onApplied(newId);
      } else {
        await call("callsheets.refresh", { id });
        toast.undoable("Updated the call sheet from the schedule");
        onApplied();
      }
    } catch (e) {
      reportError(e);
    }
    setBusy(false);
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Update call sheet: review changes"
      sub="Nothing is applied until you click Apply Update."
      size="lg"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !p || p.dayDeleted} onClick={apply}>
            {p?.createsRevision ? "Create New Revision" : "Apply Update"}
          </Button>
        </>
      }
    >
      {!p ? (
        <Skeleton h={120} />
      ) : p.dayDeleted ? (
        <Banner tone="warn">The shooting day of this call sheet was deleted, so there is nothing to update from. The call sheet stays readable.</Banner>
      ) : (
        <>
          {!p.hasChanges && <Banner tone="ok">This call sheet already matches the schedule.</Banner>}
          {p.hasChanges && (
            <ul className="sm" aria-label="Summary of changes" style={{ margin: "0 0 6px", paddingLeft: 18 }}>
              {p.summary.map((line) => (
                <li key={line}>{line}</li>
              ))}
              {p.other.length > 0 && <li>Also changes: {p.other.join(", ")}</li>}
            </ul>
          )}
          <ChangeList title="Scenes" {...p.scenes} />
          <ChangeList title="Cast calls" {...p.cast} />
          <ChangeList title="Locations" {...p.locations} />
          <table className="tbl" style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th scope="col">Area</th>
                <th scope="col">Now in call sheet</th>
                <th scope="col">After update</th>
              </tr>
            </thead>
            <tbody>
              {p.rows.map((r) => (
                <tr key={r.area} className={r.changed ? "of-changed" : undefined}>
                  <td className="b">
                    {r.area}
                    {r.changed && <span className="xs"> (changes)</span>}
                  </td>
                  <td style={{ whiteSpace: "pre-wrap" }}>{r.now}</td>
                  <td style={{ whiteSpace: "pre-wrap" }}>{r.after}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="hint" style={{ marginTop: 8 }}>
            Your call times, notes, contacts and added sections are kept where they still apply.
            {p.createsRevision && " This call sheet is final, so the update becomes a new revision and the final version stays unchanged."}
          </div>
        </>
      )}
    </Dialog>
  );
}

function FinalizeDialog({ detail, onClose, onMissingCrewCall }: { detail: CallSheetDetail; onClose: () => void; onMissingCrewCall: () => void }) {
  const [stale, setStale] = useState(false);
  const [busy, setBusy] = useState(false);
  const doc = detail.document;
  const scenes = doc.scenes.filter((s) => !s.kind).length;
  const finalize = async (acknowledgeStale: boolean) => {
    setBusy(true);
    try {
      await call("callsheets.finalize", { id: detail.id, acknowledgeStale });
      toast.undoable("Finalized the call sheet");
      onClose();
    } catch (e) {
      if (e instanceof OpError && e.code === "validation.crew_call") {
        toast.error(e.message);
        onClose();
        onMissingCrewCall();
      } else if (e instanceof OpError && e.code === "validation.call_sheet_stale") {
        setStale(true);
      } else {
        reportError(e);
      }
    }
    setBusy(false);
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Finalize call sheet"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {stale ? (
            <Button variant="primary" disabled={busy} onClick={() => finalize(true)}>
              Finalize this version anyway
            </Button>
          ) : (
            <Button variant="primary" disabled={busy} onClick={() => finalize(false)}>
              Finalize
            </Button>
          )}
        </>
      }
    >
      <div className="row" style={{ marginBottom: 6 }}>
        <Chip tone={callSheetTone(detail.status)}>{callSheetStatusWord(detail.status)}</Chip>
        <b>Is this the version you are sending?</b>
      </div>
      <div className="paper" style={{ padding: "10px 14px", marginBottom: 8 }}>
        <b>{doc.title}</b> · {doc.dayLabel} · {detail.dateLong ?? "No date"}
        <br />
        Crew call {doc.crewCall || <span className="need">{NEEDS_INPUT}</span>} · {scenes} {scenes === 1 ? "scene" : "scenes"} · {doc.cast.length} cast
      </div>
      <div className="sm">
        Final freezes this document as a snapshot. Later schedule changes mark it for a new revision but never change the final version.
      </div>
      {detail.missing.length > 0 && (
        <div className="hint" style={{ marginTop: 6 }}>
          {detail.missing.length} {detail.missing.length === 1 ? "value still reads" : "values still read"} {NEEDS_INPUT}.
        </div>
      )}
      {stale && <Banner tone="warn">The schedule changed after this call sheet was prepared. Refresh it first, or finalize this version anyway.</Banner>}
    </Dialog>
  );
}

// ------------------------------------------------------------------ sections

type Edit = (fn: (d: Doc) => void) => void;

function HeaderSection({ doc, d, edit, ro }: { doc: Doc; d: CallSheetDetail; edit: Edit; ro: boolean }) {
  const h = doc.header;
  const fields: [keyof Doc["header"], string][] = [
    ["company", "Production company"],
    ["address", "Production office address"],
    ["producer", "Producer"],
    ["director", "Director"],
    ["firstAd", "1st AD"],
    ["contact", "Production contact"],
  ];
  return (
    <Section title="Production header">
      <h1>
        <Edt value={doc.title} onChange={(v) => edit((x) => void (x.title = v))} label="Production title" need disabled={ro} />
      </h1>
      <div style={{ textAlign: "center", fontWeight: 700, margin: "4px 0" }}>
        {doc.dayLabel}
        {doc.dayOf && <> ({doc.dayOf})</>} · {d.dateLong ?? <span className="need">{NEEDS_INPUT}: set the date in the schedule</span>}
      </div>
      <div className="grid g2" style={{ gap: "0 16px" }}>
        {fields.map(([k, label]) => (
          <Fld key={k} label={label} value={h[k]} onChange={(v) => edit((x) => void (x.header[k] = v))} disabled={ro} />
        ))}
      </div>
    </Section>
  );
}

function TimesSection({ doc, edit, ro, crewRef, crewInvalid, onCrew }: { doc: Doc; edit: Edit; ro: boolean; crewRef: React.Ref<HTMLInputElement>; crewInvalid: boolean; onCrew: () => void }) {
  const t = doc.times;
  const times: [keyof Doc["times"], string][] = [
    ["generalCall", "General call"],
    ["shootingCall", "Shooting call"],
    ["breakfast", "Breakfast"],
    ["lunch", "Lunch"],
    ["estimatedWrap", "Estimated wrap"],
  ];
  return (
    <Section title="Day and times">
      <div style={{ textAlign: "center", fontSize: 13, marginBottom: 4 }}>
        CREW CALL:{" "}
        <Edt
          inputRef={crewRef}
          value={doc.crewCall}
          onChange={(v) => {
            onCrew();
            edit((x) => void (x.crewCall = v));
          }}
          label="Crew call"
          need
          disabled={ro}
          invalid={crewInvalid}
        />
      </div>
      <div className="grid g3" style={{ gap: "0 16px" }}>
        {times.map(([k, label]) => (
          <Fld key={k} label={label} value={t[k]} onChange={(v) => edit((x) => void (x.times[k] = v))} disabled={ro} />
        ))}
      </div>
      <div className="of-field" style={{ alignItems: "flex-start" }}>
        <span>Notes from the day</span>
        <EdtArea value={doc.dayNotes} onChange={(v) => edit((x) => void (x.dayNotes = v))} label="Notes from the day" disabled={ro} />
      </div>
    </Section>
  );
}

function LocationsSection({ doc, edit, ro }: { doc: Doc; edit: Edit; ro: boolean }) {
  const fields: [keyof Doc["locations"][number], string, boolean][] = [
    ["address", "Address", true],
    ["meetingPoint", "Meeting point", false],
    ["parking", "Parking", false],
    ["access", "Access", false],
    ["hospital", "Nearest hospital", true],
    ["emergencyContact", "Location contact", false],
    ["notes", "Notes", false],
  ];
  return (
    <Section title="Locations">
      {doc.locations.length === 0 && <span className="need">{NEEDS_INPUT}: no location for these scenes</span>}
      {doc.locations.map((l, i) => (
        <div key={l.key || i} style={{ marginBottom: 6 }}>
          <b>{l.name}</b>
          <div className="grid g2" style={{ gap: "0 16px" }}>
            {fields.map(([k, label, need]) => (
              <Fld key={k} label={label} value={l[k]} onChange={(v) => edit((x) => void (x.locations[i][k] = v))} need={need} disabled={ro} />
            ))}
          </div>
        </div>
      ))}
    </Section>
  );
}

function SceneSection({ doc, d, edit, ro }: { doc: Doc; d: CallSheetDetail; edit: Edit; ro: boolean }) {
  return (
    <Section title="Scene schedule">
      {doc.scenes.length === 0 ? (
        <div className="muted">No scenes on this day.</div>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">Start</th>
              <th scope="col">Sc</th>
              <th scope="col">Set and description</th>
              <th scope="col">I/E</th>
              <th scope="col">D/N</th>
              <th scope="col">Pages</th>
              <th scope="col">Cast</th>
              <th scope="col">Location</th>
            </tr>
          </thead>
          <tbody>
            {doc.scenes.map((s, i) => {
              const estimate = d.estimatedStarts[i] ?? "";
              if (s.kind) {
                return (
                  <tr key={s.key || i} className="of-sub">
                    <td>
                      <Edt value={s.start} placeholder={estimate ? `about ${estimate}` : undefined} onChange={(v) => edit((x) => void (x.scenes[i].start = v))} label={`Start of ${s.heading || s.kind}`} disabled={ro} />
                    </td>
                    <td colSpan={7}>
                      <b>{s.kind}</b>
                      {s.heading && s.heading !== s.kind && <> · {s.heading}</>}
                      {s.minutes !== null && <> · {s.minutes} min</>}
                    </td>
                  </tr>
                );
              }
              return (
                <tr key={s.key || i}>
                  <td>
                    <Edt value={s.start} placeholder={estimate ? `about ${estimate}` : undefined} onChange={(v) => edit((x) => void (x.scenes[i].start = v))} label={`Start of scene ${s.number}`} disabled={ro} />
                  </td>
                  <td>{s.number}</td>
                  <td>
                    {s.heading}
                    {s.description && <div className="xs">{s.description}</div>}
                  </td>
                  <td>{s.intExt}</td>
                  <td>{s.dayNight}</td>
                  <td>{s.pages}</td>
                  <td>{s.castIds}</td>
                  <td>{s.location}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </Section>
  );
}

function TalentSection({ doc, edit, ro }: { doc: Doc; edit: Edit; ro: boolean }) {
  return (
    <Section title="Talent">
      {doc.cast.length === 0 ? (
        <div className="muted">No cast in these scenes.</div>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">ID</th>
              <th scope="col">Actor</th>
              <th scope="col">Character</th>
              <th scope="col">Status</th>
              <th scope="col">Pickup</th>
              <th scope="col">Makeup</th>
              <th scope="col">Call</th>
              <th scope="col">Contact and notes</th>
            </tr>
          </thead>
          <tbody>
            {doc.cast.map((c, i) => (
              <tr key={c.key || i}>
                <td>{c.castId}</td>
                <td>
                  <Edt value={c.actor} onChange={(v) => edit((x) => void (x.cast[i].actor = v))} label={`Actor for ${c.character}`} need disabled={ro} />
                </td>
                <td>{c.character}</td>
                <td>
                  <Edt value={c.status} placeholder="SW, W, WF" onChange={(v) => edit((x) => void (x.cast[i].status = v))} label={`Work status of ${c.character}`} disabled={ro} />
                </td>
                <td>
                  <Edt value={c.pickup} onChange={(v) => edit((x) => void (x.cast[i].pickup = v))} label={`Pickup for ${c.character}`} disabled={ro} />
                </td>
                <td>
                  <Edt value={c.makeup} onChange={(v) => edit((x) => void (x.cast[i].makeup = v))} label={`Makeup time for ${c.character}`} disabled={ro} />
                </td>
                <td>
                  <Edt value={c.callTime} onChange={(v) => edit((x) => void (x.cast[i].callTime = v))} label={`Call time for ${c.character}`} need disabled={ro} />
                </td>
                <td>
                  <Edt value={c.contact} placeholder="Contact" onChange={(v) => edit((x) => void (x.cast[i].contact = v))} label={`Contact for ${c.character}`} disabled={ro} />{" "}
                  <Edt value={c.notes} placeholder="Notes" onChange={(v) => edit((x) => void (x.cast[i].notes = v))} label={`Notes for ${c.character}`} disabled={ro} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </Section>
  );
}

function PracticalSection({ doc, edit, ro }: { doc: Doc; edit: Edit; ro: boolean }) {
  const p = doc.practical;
  const fields: [keyof Doc["practical"], string, boolean][] = [
    ["parking", "Parking", false],
    ["meetingPoint", "Meeting point", false],
    ["travelNotes", "Transport", false],
    ["mealBreak", "Meals", false],
    ["toilets", "Toilets", false],
    ["access", "Access", false],
    ["safety", "Safety", false],
    ["emergencyContact", "Emergency contact", true],
  ];
  return (
    <Section title="Practical notes">
      {fields.map(([k, label, need]) => (
        <Fld key={k} label={label} value={p[k]} onChange={(v) => edit((x) => void (x.practical[k] = v))} need={need} disabled={ro} />
      ))}
      <div className="of-field" style={{ alignItems: "flex-start" }}>
        <span>Special instructions</span>
        <EdtArea value={p.specialInstructions} onChange={(v) => edit((x) => void (x.practical.specialInstructions = v))} label="Special instructions" disabled={ro} />
      </div>
      <div className="of-field" style={{ alignItems: "flex-start" }}>
        <span>Production notes</span>
        <EdtArea value={p.productionNotes} onChange={(v) => edit((x) => void (x.practical.productionNotes = v))} label="Production notes" disabled={ro} />
      </div>
    </Section>
  );
}

function OptionalSections({ doc, edit, ro, attach }: { doc: Doc; edit: Edit; ro: boolean; attach: (s: "attachments" | "referenceImages") => void }) {
  const o = doc.optional;
  return (
    <>
      {o.weather !== null && (
        <Section title="Weather" disabled={ro} onRemove={() => edit((x) => void (x.optional.weather = null))}>
          <div className="hint xs">Type the forecast, sunrise and sunset. OpenFrame never looks them up online.</div>
          <Fld label="Forecast" value={o.weather} onChange={(v) => edit((x) => void (x.optional.weather = v))} need disabled={ro} />
          <div className="grid g2" style={{ gap: "0 16px" }}>
            <Fld label="Sunrise" value={o.sunrise} onChange={(v) => edit((x) => void (x.optional.sunrise = v))} need disabled={ro} />
            <Fld label="Sunset" value={o.sunset} onChange={(v) => edit((x) => void (x.optional.sunset = v))} need disabled={ro} />
          </div>
        </Section>
      )}
      {o.extras !== null && (
        <Section title="Extras" disabled={ro} onRemove={() => edit((x) => void (x.optional.extras = null))}>
          <table>
            <thead>
              <tr>
                <th scope="col">Description</th>
                <th scope="col">Quantity</th>
                <th scope="col">Call</th>
                <th scope="col">On set</th>
                <th scope="col">Notes</th>
                {!ro && <th scope="col" aria-label="Actions" />}
              </tr>
            </thead>
            <tbody>
              {o.extras.map((e, i) => (
                <tr key={e.key || `typed-${i}`}>
                  {(["description", "quantity", "callTime", "onSet", "notes"] as const).map((k) => (
                    <td key={k}>
                      <Edt value={e[k]} onChange={(v) => edit((x) => void (x.optional.extras![i][k] = v))} label={`Extras ${i + 1} ${k}`} need={k === "callTime"} disabled={ro} />
                    </td>
                  ))}
                  {!ro && (
                    <td>
                      <RemoveRow label={`Remove extras row ${e.description || i + 1}`} onClick={() => edit((x) => void x.optional.extras!.splice(i, 1))} />
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
          {!ro && (
            <Button size="xs" icon={<Plus size={12} />} onClick={() => edit((x) => void x.optional.extras!.push({ key: "", description: "", quantity: "", callTime: "", onSet: "", notes: "" }))}>
              Add extras row
            </Button>
          )}
        </Section>
      )}
      {o.departments !== null && (
        <Section title="Department notes" disabled={ro} onRemove={() => edit((x) => void (x.optional.departments = null))}>
          {o.departments.map((dep, i) => (
            <div key={i} className="of-field" style={{ alignItems: "flex-start" }}>
              <Edt value={dep.department} placeholder="Department" onChange={(v) => edit((x) => void (x.optional.departments![i].department = v))} label={`Department ${i + 1}`} disabled={ro} />
              <EdtArea value={dep.notes} onChange={(v) => edit((x) => void (x.optional.departments![i].notes = v))} label={`Notes for ${dep.department || `department ${i + 1}`}`} disabled={ro} />
              <RemoveRow label={`Remove ${dep.department || `department ${i + 1}`}`} onClick={() => edit((x) => void x.optional.departments!.splice(i, 1))} disabled={ro} />
            </div>
          ))}
          {!ro && (
            <Button size="xs" icon={<Plus size={12} />} onClick={() => edit((x) => void x.optional.departments!.push({ department: "", notes: "" }))}>
              Add department
            </Button>
          )}
        </Section>
      )}
      {o.advance !== null && (
        <Section title="Advance schedule" disabled={ro} onRemove={() => edit((x) => void (x.optional.advance = null))}>
          {(
            [
              ["day", "Next day"],
              ["location", "Location"],
              ["cast", "Cast"],
              ["props", "Props"],
              ["wardrobe", "Wardrobe"],
              ["preparation", "Preparation"],
            ] as const
          ).map(([k, label]) => (
            <Fld key={k} label={label} value={o.advance![k]} onChange={(v) => edit((x) => void (x.optional.advance![k] = v))} need={k === "day"} disabled={ro} />
          ))}
        </Section>
      )}
      {o.crew !== null && (
        <Section title="Crew" disabled={ro} onRemove={() => edit((x) => void (x.optional.crew = null))}>
          <table>
            <thead>
              <tr>
                <th scope="col">Department</th>
                <th scope="col">Role</th>
                <th scope="col">Name</th>
                <th scope="col">Call</th>
                <th scope="col">Contact</th>
                {!ro && <th scope="col" aria-label="Actions" />}
              </tr>
            </thead>
            <tbody>
              {o.crew.map((m, i) => (
                <tr key={m.key || `typed-${i}`}>
                  {(["department", "role", "name", "callTime", "contact"] as const).map((k) => (
                    <td key={k}>
                      <Edt value={m[k]} onChange={(v) => edit((x) => void (x.optional.crew![i][k] = v))} label={`Crew ${m.name || i + 1} ${k}`} disabled={ro} />
                    </td>
                  ))}
                  {!ro && (
                    <td>
                      <RemoveRow label={`Remove ${m.name || `crew row ${i + 1}`}`} onClick={() => edit((x) => void x.optional.crew!.splice(i, 1))} />
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
          {!ro && (
            <Button size="xs" icon={<Plus size={12} />} onClick={() => edit((x) => void x.optional.crew!.push({ key: "", name: "", role: "", department: "", callTime: "", contact: "" }))}>
              Add crew row
            </Button>
          )}
        </Section>
      )}
      {o.communication !== null && (
        <Section title="Walkie channels and contacts" disabled={ro} onRemove={() => edit((x) => void (x.optional.communication = null))}>
          {(
            [
              ["walkieChannels", "Walkie channels"],
              ["productionContact", "Production contact"],
              ["emergencyContact", "Emergency contact"],
              ["note", "Note"],
            ] as const
          ).map(([k, label]) => (
            <Fld key={k} label={label} value={o.communication![k]} onChange={(v) => edit((x) => void (x.optional.communication![k] = v))} disabled={ro} />
          ))}
        </Section>
      )}
      {o.specialNotes !== null && (
        <Section title="Special notes" disabled={ro} onRemove={() => edit((x) => void (x.optional.specialNotes = null))}>
          <EdtArea value={o.specialNotes} onChange={(v) => edit((x) => void (x.optional.specialNotes = v))} label="Special notes" disabled={ro} />
        </Section>
      )}
      {o.attachments !== null && (
        <Section title="Attachments" disabled={ro} onRemove={() => edit((x) => void (x.optional.attachments = null))}>
          <div className="of-attach">
            {o.attachments.map((a, i) => (
              <span key={a.assetId + i} className="chip">
                <Paperclip size={11} aria-hidden /> {a.name}
                {!ro && (
                  <button type="button" aria-label={`Remove ${a.name}`} style={{ background: "none", border: 0, cursor: "pointer" }} onClick={() => edit((x) => void x.optional.attachments!.splice(i, 1))}>
                    <X size={11} />
                  </button>
                )}
              </span>
            ))}
            {o.attachments.length === 0 && <span className="muted">No attachments yet.</span>}
            {!ro && inTauri() && (
              <Button size="xs" icon={<Plus size={12} />} onClick={() => attach("attachments")}>
                Add file…
              </Button>
            )}
          </div>
        </Section>
      )}
      {o.referenceImages !== null && (
        <Section title="Reference images" disabled={ro} onRemove={() => edit((x) => void (x.optional.referenceImages = null))}>
          <div className="of-attach">
            {o.referenceImages.map((a, i) => (
              <span key={a.assetId + i} style={{ position: "relative" }}>
                <Thumb att={a} />
                <RemoveRow label={`Remove ${a.name}`} onClick={() => edit((x) => void x.optional.referenceImages!.splice(i, 1))} disabled={ro} />
              </span>
            ))}
            {o.referenceImages.length === 0 && <span className="muted">No images yet.</span>}
            {!ro && inTauri() && (
              <Button size="xs" icon={<Plus size={12} />} onClick={() => attach("referenceImages")}>
                Add image…
              </Button>
            )}
          </div>
        </Section>
      )}
      {doc.extraFields.length > 0 && (
        <Section title="Additional information">
          {doc.extraFields.map((f, i) => (
            <div key={i} className="of-field">
              <Edt value={f.label} onChange={(v) => edit((x) => void (x.extraFields[i].label = v))} label={`Field ${i + 1} name`} placeholder="Field name" disabled={ro} />
              <Edt wide value={f.value} onChange={(v) => edit((x) => void (x.extraFields[i].value = v))} label={`Field ${i + 1} value`} disabled={ro} />
              <RemoveRow label={`Remove field ${f.label || i + 1}`} onClick={() => edit((x) => void x.extraFields.splice(i, 1))} disabled={ro} />
            </div>
          ))}
        </Section>
      )}
    </>
  );
}

/** Is an optional section (by its key) on the sheet? */
function sectionShown(doc: Doc, key: string): boolean {
  const o = doc.optional;
  switch (key) {
    case "weather":
      return o.weather !== null;
    case "extras":
      return o.extras !== null;
    case "departments":
      return o.departments !== null;
    case "advance":
      return o.advance !== null;
    case "crew":
      return o.crew !== null;
    case "communication":
      return o.communication !== null;
    case "referenceImages":
      return o.referenceImages !== null;
    case "attachments":
      return o.attachments !== null;
    case "specialNotes":
      return o.specialNotes !== null;
    default:
      return false;
  }
}

// ------------------------------------------------------------------ editor

export function CallSheetEditor({ id }: { id: string }) {
  const q = useOp<CallSheetDetail>("callsheets.get", { id }, CALL_SHEET_TABLES);
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const [doc, setDoc] = useState<Doc | null>(null);
  const docRef = useRef<Doc | null>(null);
  const dirty = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  const crewRef = useRef<HTMLInputElement>(null);
  const [crewInvalid, setCrewInvalid] = useState(false);
  const [dialog, setDialog] = useState<"refresh" | "finalize" | null>(null);
  const [saving, setSaving] = useState(false);

  const serverJson = q.data ? JSON.stringify(q.data.document) : null;
  useEffect(() => {
    if (q.data && !dirty.current) {
      setDoc(q.data.document);
      docRef.current = q.data.document;
    }
    // Re-sync only when the stored document itself changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serverJson]);

  const inflight = useRef<Promise<void> | null>(null);
  // Waits for a save already running. Only the document that was sent is marked saved: an edit
  // made while it was on its way stays dirty and goes in the next save.
  const flush = useCallback(async (): Promise<boolean> => {
    window.clearTimeout(timer.current);
    if (inflight.current) await inflight.current;
    const sent = docRef.current;
    if (!dirty.current || !sent) return true;
    setSaving(true);
    const run = call("callsheets.update", { id, document: sent }).then(
      () => {
        if (docRef.current === sent) dirty.current = false;
      },
      (e: unknown) => reportError(e),
    );
    inflight.current = run;
    try {
      await run;
    } finally {
      if (inflight.current === run) inflight.current = null;
      setSaving(false);
    }
    return !dirty.current;
  }, [id]);

  usePendingSave(flush);

  // Actions requested from the list's context menu.
  const action = route.params?.action;
  useEffect(() => {
    if (action === "refresh" || action === "finalize") setDialog(action);
  }, [action]);

  const edit: Edit = (fn) => {
    if (!docRef.current) return;
    const next = structuredClone(docRef.current);
    fn(next);
    docRef.current = next;
    setDoc(next);
    dirty.current = true;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => void flush(), SAVE_DELAY_MS);
  };

  if (q.isLoading) return <div className="content"><Skeleton h={28} w={240} /></div>;
  if (q.error || !q.data || !doc) {
    return (
      <div className="content">
        <Banner tone="err" actions={<Button size="xs" onClick={() => go({ workspace: "callsheets" })}>Back to Call Sheets</Button>}>
          {q.error?.message ?? "This call sheet could not be opened."}
        </Banner>
      </div>
    );
  }
  const d = q.data;
  const ro = !d.editable;
  const word = callSheetStatusWord(d.status);
  const back = async () => {
    await flush();
    go({ workspace: "callsheets" });
  };
  const withFlush = (fn: () => void) => async () => {
    await flush();
    fn();
  };
  const run = async (op: string, args: object, text: string) => {
    await flush();
    try {
      await call(op, args);
      toast.undoable(text);
    } catch (e) {
      reportError(e);
    }
  };
  const attach = async (section: "attachments" | "referenceImages") => {
    if (!inTauri()) return;
    const picked = await openDialog({
      multiple: false,
      directory: false,
      filters: section === "referenceImages" ? [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }] : undefined,
    });
    if (typeof picked !== "string") return;
    await run("callsheets.attach", { id, section, path: picked }, "Attached a file to the call sheet");
  };

  // "Add Section": the server fills the section from the project where it has the data.
  const addItems: MenuItemSpec[] = [
    ...d.optionalSections.map((s) => ({
      label: s.label,
      disabled: sectionShown(doc, s.key),
      onSelect: () => void run("callsheets.add_section", { id, section: s.key }, `Added ${s.label} to the call sheet`),
    })),
    { label: "Custom field", separatorBefore: true, onSelect: () => edit((x) => void x.extraFields.push({ label: "", value: "" })) },
  ];

  const stale = d.stale;
  const replaced = !!d.supersededBy;
  return (
    <div className="content fill">
      <PageHeader
        title={d.title}
        badge={<Chip tone={callSheetTone(d.status)}>{word}</Chip>}
        sub="What does everyone need to know for this day?"
        actions={
          <Button size="sm" icon={<ArrowLeft size={14} />} onClick={back}>
            Call Sheets
          </Button>
        }
      />
      {replaced && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="info" actions={<Button size="xs" onClick={() => go({ workspace: "callsheets", params: { callSheetId: d.supersededBy! } })}>Open newer revision</Button>}>
            A newer revision replaced this one. This final version stays unchanged as a record.
          </Banner>
        </div>
      )}
      {stale && !replaced && (
        <div style={{ marginBottom: 8 }}>
          {stale.dayDeleted ? (
            <Banner tone="warn">The shooting day of this call sheet was deleted. The call sheet stays readable and unchanged.</Banner>
          ) : d.editable ? (
            <Banner tone="warn" actions={<Button size="xs" variant="primary" onClick={() => setDialog("refresh")}>Review Update</Button>}>
              <b>Needs refresh.</b> The schedule changed after this call sheet was prepared ({formatDateTime(stale.preparedAt)}). Changed:{" "}
              {stale.areas.join(", ") || "day details"}. Nothing changes until you review the update.
            </Banner>
          ) : (
            <Banner tone="info" actions={<Button size="xs" onClick={() => setDialog("refresh")}>Start New Revision</Button>}>
              The schedule changed after this call sheet was finalized ({stale.areas.join(", ") || "day details"}). The final version never changes; start a new
              revision to update it.
            </Banner>
          )}
        </div>
      )}
      <div className="row" style={{ alignItems: "stretch", gap: 14, flex: 1, minHeight: 0 }}>
        <div className="of-paper-wrap">
          <div className="paper" aria-label="Call sheet document">
            <HeaderSection doc={doc} d={d} edit={edit} ro={ro} />
            <TimesSection doc={doc} edit={edit} ro={ro} crewRef={crewRef} crewInvalid={crewInvalid} onCrew={() => setCrewInvalid(false)} />
            <LocationsSection doc={doc} edit={edit} ro={ro} />
            <SceneSection doc={doc} d={d} edit={edit} ro={ro} />
            <TalentSection doc={doc} edit={edit} ro={ro} />
            <PracticalSection doc={doc} edit={edit} ro={ro} />
            <OptionalSections doc={doc} edit={edit} ro={ro} attach={attach} />
          </div>
        </div>

        <div className="card pad of-scroll" style={{ width: 290, flex: "none" }}>
          <div className="h4">Source</div>
          <div className="sm">
            <button type="button" className="btn ghost sm" style={{ padding: 0, color: "var(--blue)" }} onClick={withFlush(() => go({ workspace: "production", sub: "schedule", params: { dayId: d.dayId } }))}>
              {d.dayTitle}
            </button>
          </div>
          <div className="row" style={{ margin: "6px 0" }}>
            <Chip tone={callSheetTone(d.status)}>{word}</Chip>
            <span className="xs muted">v{d.revision}</span>
            {saving && <span className="xs muted">Saving…</span>}
          </div>
          {d.missing.length > 0 && d.editable && (
            <div className="xs" style={{ color: "var(--red-t)", marginBottom: 6 }}>
              {d.missing.length} {d.missing.length === 1 ? "value reads" : "values read"} {NEEDS_INPUT}.
            </div>
          )}
          <div className="hint">
            Scenes, times, cast and locations come from the schedule. Editable fields use dashed highlights. Editing this sheet never changes the screenplay,
            breakdown or schedule.
          </div>
          <div className="hr" />
          <div className="col" style={{ gap: 6 }}>
            {d.editable && <Menu items={addItems} trigger={<Button size="sm">Add Section</Button>} />}
            {!replaced && (
              <Button size="sm" onClick={withFlush(() => setDialog("refresh"))} disabled={!!stale?.dayDeleted}>
                {d.editable ? "Refresh from schedule" : "Start New Revision"}
              </Button>
            )}
            {d.editable && (
              <Button size="sm" variant="primary" onClick={withFlush(() => setDialog("finalize"))}>
                Finalize…
              </Button>
            )}
            <CallSheetExportButton size="sm" id={id} title={d.title} status={word} />
          </div>
          {d.finalizedAt && <div className="xs muted" style={{ marginTop: 8 }}>Finalized {formatDateTime(d.finalizedAt)}.</div>}
          {d.revisions.length > 1 && (
            <>
              <div className="hr" />
              <div className="h4">Revisions</div>
              {d.revisions.map((r) => (
                <button
                  key={r.id}
                  type="button"
                  className={`li btn ghost${r.id === d.id ? " sel" : ""}`}
                  style={{ width: "100%", justifyContent: "space-between" }}
                  onClick={withFlush(() => go({ workspace: "callsheets", params: { callSheetId: r.id } }))}
                >
                  <span>v{r.revision}</span>
                  <Chip tone={callSheetTone(r.status)}>{callSheetStatusWord(r.status)}</Chip>
                </button>
              ))}
            </>
          )}
          <div className="hr" />
          <Button
            size="sm"
            variant="ghost"
            onClick={async () => {
              window.clearTimeout(timer.current);
              dirty.current = false;
              try {
                await call("callsheets.delete", { id });
                toast.undoable(`Deleted ${d.title}`);
                go({ workspace: "callsheets" });
              } catch (e) {
                reportError(e);
              }
            }}
          >
            Delete call sheet
          </Button>
        </div>
      </div>
      {dialog === "refresh" && (
        <RefreshDialog
          id={id}
          onClose={() => setDialog(null)}
          onApplied={(newId) => {
            setDialog(null);
            if (newId) go({ workspace: "callsheets", params: { callSheetId: newId } });
          }}
        />
      )}
      {dialog === "finalize" && (
        <FinalizeDialog
          detail={d}
          onClose={() => setDialog(null)}
          onMissingCrewCall={() => {
            setCrewInvalid(true);
            window.setTimeout(() => crewRef.current?.focus(), 0);
          }}
        />
      )}
    </div>
  );
}
