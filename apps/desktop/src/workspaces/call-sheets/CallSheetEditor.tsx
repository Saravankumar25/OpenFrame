// Paper-style call sheet editor (UX §3.31; mocks 143–147).
// Derived fields (scenes, cast, location) come from the schedule; editable
// fields use the dashed `.edt` style and missing values the red `.need` style.
// Edits are saved to the call sheet only — never back into the schedule.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { ArrowLeft, Paperclip, Plus, X } from "lucide-react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Banner, Button, Chip, Dialog, IconButton, Menu, PageHeader, Skeleton, type MenuItemSpec } from "../../design-system";
import { call, inTauri, OpError } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import type { AssetInfo } from "../../ipc/generated/AssetInfo";
import type { CallSheetAttachment } from "../../ipc/generated/CallSheetAttachment";
import type { CallSheetDetail, CallSheetDocument, CallSheetRefreshPreview } from "../../api/schedule";
import { CALL_SHEET_TABLES, callSheetStatusLabel, callSheetTone, formatDateTime } from "../../api/schedule";
import { CallSheetExportButton } from "../../features/export/buttons";

type Doc = CallSheetDocument;
const SAVE_DELAY_MS = 500;

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
  return (
    <input
      ref={inputRef}
      className={`edt${missing ? " need" : ""}${invalid ? " of-crew-invalid" : ""}`}
      aria-label={label}
      aria-invalid={missing || invalid || undefined}
      value={value}
      placeholder={missing ? "needs input" : placeholder}
      disabled={disabled}
      size={wide ? undefined : Math.max(8, Math.min(40, (value || placeholder || "needs input").length + 1))}
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
      value={value}
      placeholder={missing ? "needs input" : undefined}
      disabled={disabled}
      rows={2}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

function Section({ title, children, onRemove, disabled }: { title: string; children: ReactNode; onRemove?: () => void; disabled?: boolean }) {
  return (
    <section aria-label={title}>
      <div className="st" style={{ display: "flex", alignItems: "center" }}>
        <span style={{ flex: 1 }}>{title}</span>
        {onRemove && !disabled && (
          <button type="button" aria-label={`Remove ${title}`} onClick={onRemove} style={{ background: "none", border: 0, color: "#fff", cursor: "pointer" }}>
            <X size={12} />
          </button>
        )}
      </div>
      {children}
    </section>
  );
}

function Thumb({ att }: { att: CallSheetAttachment }) {
  const q = useOp<AssetInfo>("files.asset", { assetId: att.assetId }, ["asset"]);
  const path = q.data?.available ? q.data.path : null;
  if (!path || !inTauri()) return <span className="chip">{att.name}</span>;
  return <img className="of-thumb" src={convertFileSrc(path)} alt={att.name} title={att.name} />;
}

// ------------------------------------------------------------------ dialogs

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
        toast.undoable("Started a new call sheet revision; the issued one is unchanged");
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
      title="Update Call Sheet — review changes"
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
          <table className="tbl" style={{ marginTop: 6 }}>
            <thead>
              <tr>
                <th>Area</th>
                <th>Now in call sheet</th>
                <th>After update</th>
              </tr>
            </thead>
            <tbody>
              {p.rows.map((r) => (
                <tr key={r.area} className={r.changed ? "of-changed" : undefined}>
                  <td className="b">
                    {r.area}
                    {r.changed && <span className="sr-only"> (changes)</span>}
                  </td>
                  <td style={{ whiteSpace: "pre-wrap" }}>{r.now}</td>
                  <td style={{ whiteSpace: "pre-wrap" }}>{r.after}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="hint" style={{ marginTop: 8 }}>
            Your edited call times and notes are kept where they still apply.
            {p.createsRevision && " This call sheet is finalized, so the update becomes a new revision and the issued version stays unchanged."}
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
        <Chip tone={callSheetTone(detail.status)}>{callSheetStatusLabel(detail.status)}</Chip>
        <b>Is this the version I am actually sending?</b>
      </div>
      <div className="paper" style={{ padding: "10px 14px", marginBottom: 8 }}>
        <b>{doc.title}</b> · {doc.dayLabel} · {detail.dateLong ?? "No date"}
        <br />
        Crew call {doc.crewCall || <span className="need">needs input</span>} · {doc.scenes.length} scenes · {doc.cast.length} cast
      </div>
      <div className="sm">Freezes this document as a stable, issued version. Later schedule changes will mark it for a new revision but never change the issued version.</div>
      {detail.missing.length > 0 && <div className="hint" style={{ marginTop: 6 }}>{detail.missing.length} value(s) are still marked “needs input”.</div>}
      {stale && (
        <Banner tone="warn">The schedule changed after this call sheet was prepared. Refresh it first, or finalize this version anyway.</Banner>
      )}
    </Dialog>
  );
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

  const flush = useCallback(async () => {
    window.clearTimeout(timer.current);
    if (!dirty.current || !docRef.current) return;
    setSaving(true);
    try {
      await call("callsheets.update", { id, document: docRef.current });
      dirty.current = false;
    } catch (e) {
      reportError(e);
    }
    setSaving(false);
  }, [id]);

  useEffect(() => () => void flush(), [flush]);

  // Actions requested from the list's context menu.
  const action = route.params?.action;
  useEffect(() => {
    if (action === "refresh" || action === "finalize") setDialog(action);
  }, [action]);

  const edit = (fn: (d: Doc) => void) => {
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

  const o = doc.optional;
  const optionalItems: MenuItemSpec[] = [
    { label: "Weather", disabled: o.weather !== null, onSelect: () => edit((x) => void (x.optional.weather = "")) },
    { label: "Attachments", disabled: o.attachments !== null, onSelect: () => edit((x) => void (x.optional.attachments = [])) },
    { label: "Reference images", disabled: o.referenceImages !== null, onSelect: () => edit((x) => void (x.optional.referenceImages = [])) },
    { label: "Special notes", disabled: o.specialNotes !== null, onSelect: () => edit((x) => void (x.optional.specialNotes = "")) },
    { label: "Custom field", separatorBefore: true, onSelect: () => edit((x) => void x.extraFields.push({ label: "", value: "" })) },
  ];

  const stale = d.stale;
  return (
    <div className="content fill">
      <PageHeader
        title={d.title}
        badge={<Chip tone={callSheetTone(d.status)}>{callSheetStatusLabel(d.status)}</Chip>}
        sub="What does everyone need to know for this day?"
        actions={
          <Button size="sm" icon={<ArrowLeft size={14} />} onClick={back}>
            Call Sheets
          </Button>
        }
      />
      {d.status === "Superseded" && d.supersededBy && (
        <div style={{ marginBottom: 8 }}>
          <Banner tone="info" actions={<Button size="xs" onClick={() => go({ workspace: "callsheets", params: { callSheetId: d.supersededBy! } })}>Open newer revision</Button>}>
            This revision was replaced by a newer one. It stays unchanged as a historical document.
          </Banner>
        </div>
      )}
      {stale && d.status !== "Superseded" && (
        <div style={{ marginBottom: 8 }}>
          {stale.dayDeleted ? (
            <Banner tone="warn">The shooting day of this call sheet was deleted. The call sheet remains readable and unchanged.</Banner>
          ) : d.editable ? (
            <Banner tone="warn" actions={<Button size="xs" variant="primary" onClick={() => setDialog("refresh")}>Update Call Sheet</Button>}>
              <b>Source Schedule Changed — Review Update.</b> This call sheet is based on an older schedule version (prepared {formatDateTime(stale.preparedAt)}). Changed:{" "}
              {stale.areas.join(", ") || "day details"}. It remains readable and unchanged.
            </Banner>
          ) : (
            <Banner tone="info" actions={<Button size="xs" onClick={() => setDialog("refresh")}>Start New Revision</Button>}>
              The schedule changed after this call sheet was issued ({stale.areas.join(", ") || "day details"}). The issued document never changes; start a new revision to update it.
            </Banner>
          )}
        </div>
      )}
      <div className="row" style={{ alignItems: "stretch", gap: 14, flex: 1, minHeight: 0 }}>
        <div className="of-paper-wrap">
          <div className="paper" aria-label="Call sheet document">
            <h1>
              <Edt value={doc.title} onChange={(v) => edit((x) => void (x.title = v))} label="Production title" disabled={ro} />
            </h1>
            <div style={{ textAlign: "center", fontWeight: 700, margin: "4px 0" }}>
              {doc.dayLabel} · {d.dateLong ?? <span className="need">date not set in schedule</span>}
            </div>
            <div style={{ textAlign: "center", fontSize: 13 }}>
              CREW CALL:{" "}
              <Edt inputRef={crewRef} value={doc.crewCall} onChange={(v) => { setCrewInvalid(false); edit((x) => void (x.crewCall = v)); }} label="Crew call" need disabled={ro} invalid={crewInvalid} />
            </div>

            <Section title="Location">
              {doc.locations.length === 0 && <span className="need">No location for these scenes</span>}
              {doc.locations.map((l, i) => (
                <div key={l.key} style={{ marginBottom: 4 }}>
                  <b>{l.name}</b> — <Edt value={l.address} onChange={(v) => edit((x) => void (x.locations[i].address = v))} label={`Address of ${l.name}`} need disabled={ro} />
                  {" · "}Meeting point: <Edt value={l.meetingPoint} onChange={(v) => edit((x) => void (x.locations[i].meetingPoint = v))} label={`Meeting point at ${l.name}`} disabled={ro} />
                  {" · "}Parking: <Edt value={l.parking} onChange={(v) => edit((x) => void (x.locations[i].parking = v))} label={`Parking at ${l.name}`} disabled={ro} />
                </div>
              ))}
            </Section>

            <Section title="Cast">
              {doc.cast.length === 0 ? (
                <div className="muted">No cast confirmed for these scenes.</div>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Actor</th>
                      <th>Character</th>
                      <th>Call</th>
                      <th>Notes</th>
                    </tr>
                  </thead>
                  <tbody>
                    {doc.cast.map((c, i) => (
                      <tr key={c.key}>
                        <td>
                          <Edt value={c.actor} onChange={(v) => edit((x) => void (x.cast[i].actor = v))} label={`Actor for ${c.character}`} need disabled={ro} />
                        </td>
                        <td>{c.character}</td>
                        <td>
                          <Edt value={c.callTime} onChange={(v) => edit((x) => void (x.cast[i].callTime = v))} label={`Call time for ${c.character}`} need disabled={ro} />
                        </td>
                        <td>
                          <Edt value={c.notes} onChange={(v) => edit((x) => void (x.cast[i].notes = v))} label={`Notes for ${c.character}`} disabled={ro} />
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </Section>

            <Section title="Scenes">
              {doc.scenes.length === 0 ? (
                <div className="muted">No scenes on this day.</div>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Sc</th>
                      <th>Heading</th>
                      <th>Description</th>
                      <th>Pages</th>
                    </tr>
                  </thead>
                  <tbody>
                    {doc.scenes.map((s) => (
                      <tr key={s.key}>
                        <td>{s.number}</td>
                        <td>{s.heading}</td>
                        <td>{s.description}</td>
                        <td>{s.pages}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </Section>

            <Section title="Practical notes">
              {(
                [
                  ["parking", "Parking"],
                  ["meetingPoint", "Meeting point"],
                  ["travelNotes", "Travel notes"],
                  ["mealBreak", "Meal / break"],
                  ["emergencyContact", "Emergency contact"],
                ] as const
              ).map(([k, label]) => (
                <div key={k} className="of-field">
                  <span>{label}</span>
                  <Edt wide value={doc.practical[k]} onChange={(v) => edit((x) => void (x.practical[k] = v))} label={label} need={k === "emergencyContact"} disabled={ro} />
                </div>
              ))}
              <div className="of-field" style={{ alignItems: "flex-start" }}>
                <span>Production notes</span>
                <EdtArea value={doc.practical.productionNotes} onChange={(v) => edit((x) => void (x.practical.productionNotes = v))} label="Production notes" disabled={ro} />
              </div>
              <div className="of-field" style={{ alignItems: "flex-start" }}>
                <span>Notes from the day</span>
                <EdtArea value={doc.dayNotes} onChange={(v) => edit((x) => void (x.dayNotes = v))} label="Notes from the day" disabled={ro} />
              </div>
            </Section>

            {o.weather !== null && (
              <Section title="Weather" disabled={ro} onRemove={() => edit((x) => void (x.optional.weather = null))}>
                <EdtArea value={o.weather} onChange={(v) => edit((x) => void (x.optional.weather = v))} label="Weather" disabled={ro} />
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
                      {!ro && (
                        <IconButton label={`Remove ${a.name}`} onClick={() => edit((x) => void x.optional.referenceImages!.splice(i, 1))}>
                          <X size={12} />
                        </IconButton>
                      )}
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
                    {!ro && (
                      <IconButton label={`Remove field ${f.label || i + 1}`} onClick={() => edit((x) => void x.extraFields.splice(i, 1))}>
                        <X size={12} />
                      </IconButton>
                    )}
                  </div>
                ))}
              </Section>
            )}
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
            <Chip tone={callSheetTone(d.status)}>{callSheetStatusLabel(d.status)}</Chip>
            <span className="xs muted">v{d.revision}</span>
            {saving && <span className="xs muted">Saving…</span>}
          </div>
          {d.missing.length > 0 && d.editable && <div className="xs" style={{ color: "#8f2b22", marginBottom: 6 }}>{d.missing.length} value(s) need input.</div>}
          <div className="hint">
            Derived fields (scenes, cast, location) come from the schedule. Editable fields (call times, notes, meeting point, attachments) use dashed highlights.
          </div>
          <div className="hr" />
          <div className="col" style={{ gap: 6 }}>
            {d.editable && <Menu items={optionalItems} trigger={<Button size="sm">Add optional section</Button>} />}
            {d.status !== "Superseded" && (
              <Button size="sm" onClick={withFlush(() => setDialog("refresh"))} disabled={!!stale?.dayDeleted}>
                {d.editable ? "Refresh from schedule" : "Start New Revision"}
              </Button>
            )}
            {d.editable && (
              <Button size="sm" onClick={() => run("callsheets.set_ready", { id, ready: d.status !== "Ready" }, d.status === "Ready" ? "Returned the call sheet to Draft" : "Marked the call sheet Ready")}>
                {d.status === "Ready" ? "Back to Draft" : "Mark Ready"}
              </Button>
            )}
            {d.editable && (
              <Button size="sm" variant="primary" onClick={withFlush(() => setDialog("finalize"))}>
                Finalize…
              </Button>
            )}
            {d.status === "Final" && (
              <Button size="sm" variant="primary" onClick={() => run("callsheets.issue", { id }, "Marked the call sheet as issued")}>
                Mark as Issued
              </Button>
            )}
            <CallSheetExportButton size="sm" id={id} title={d.title} status={d.status} />
          </div>
          {(d.finalizedAt || d.issuedAt) && (
            <div className="xs muted" style={{ marginTop: 8 }}>
              {d.finalizedAt && <>Finalized {formatDateTime(d.finalizedAt)}. </>}
              {d.issuedAt && <>Issued {formatDateTime(d.issuedAt)}.</>}
            </div>
          )}
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
                  <Chip tone={callSheetTone(r.status)}>{callSheetStatusLabel(r.status)}</Chip>
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
