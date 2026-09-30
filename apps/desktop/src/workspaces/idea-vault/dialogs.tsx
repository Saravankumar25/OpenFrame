// Idea Vault dialogs (mocks 032, 033, 035, 039, 041, 043, 046).

import { useCallback, useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { Eraser, FolderPlus, Mic, Square } from "lucide-react";
import { OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { VaultAddFilesResult } from "../../ipc/generated/VaultAddFilesResult";
import type { VaultStoryTarget } from "../../ipc/generated/VaultStoryTarget";
import type { VaultCollectionDto } from "../../ipc/generated/VaultCollectionDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import { Banner, Button, Dialog, Field, TextArea, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { useNav } from "../../app/stores";
import { useStoryTargets, useVaultOverview, vault } from "./api";
import { bytesToBase64, folderTree, formatDuration, itemsPhrase, parseTags, scopeLabel, type Scope } from "./model";

function errText(e: unknown): string {
  return e instanceof OpError ? e.message : "Something went wrong. Nothing was changed.";
}

// ------------------------------------------------------------------ Add URL

export function AddUrlDialog({ scope, onClose, onCreated }: { scope: Scope; onClose: () => void; onCreated: (i: VaultItemDto) => void }) {
  const [url, setUrl] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true);
    try {
      const item = await vault.create({ store: scope, itemType: "url", url, body: note || null });
      onCreated(item);
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Add URL"
      sub="Paste a link. The link itself is what matters; a preview is just a bonus."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !url.trim()} onClick={() => void submit()}>Add URL</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); void submit(); }}>
        <Field label="Link" htmlFor="vv-add-url" error={error}>
          <TextInput id="vv-add-url" autoFocus value={url} invalid={!!error} placeholder="https://…" onChange={(e) => { setUrl(e.target.value); setError(null); }} />
        </Field>
        <Field label="Your note" htmlFor="vv-add-url-note">
          <TextArea id="vv-add-url-note" rows={3} value={note} placeholder="Optional" onChange={(e) => setNote(e.target.value)} />
        </Field>
        <div className="xs muted">OpenFrame stores the link as you entered it. Nothing is downloaded.</div>
        <button type="submit" hidden aria-hidden tabIndex={-1} />
      </form>
    </Dialog>
  );
}

// -------------------------------------------------------------- Record Note

function pickMime(): string | undefined {
  if (typeof MediaRecorder === "undefined") return undefined;
  for (const m of ["audio/webm;codecs=opus", "audio/webm", "audio/ogg;codecs=opus", "audio/mp4"]) {
    if (MediaRecorder.isTypeSupported(m)) return m;
  }
  return undefined;
}

function extForMime(m: string): string {
  if (m.includes("ogg")) return "ogg";
  if (m.includes("mp4")) return "m4a";
  return "webm";
}

function stamp(): string {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}

/** Mock 033: record, stop, save. The recording is kept exactly as recorded (FSD-IDEA-013). */
export function RecordNoteDialog({ scope, onClose, onSaved, onChooseFile }: {
  scope: Scope;
  onClose: () => void;
  onSaved: (i: VaultItemDto) => void;
  onChooseFile: () => void;
}) {
  const [phase, setPhase] = useState<"starting" | "recording" | "stopped" | "saving" | "error">("starting");
  const [error, setError] = useState<string | null>(null);
  const [elapsed, setElapsed] = useState(0);
  const [name, setName] = useState("");
  const rec = useRef<MediaRecorder | null>(null);
  const stream = useRef<MediaStream | null>(null);
  const chunks = useRef<Blob[]>([]);
  const started = useRef(0);
  const result = useRef<{ blob: Blob; durationMs: number } | null>(null);

  useEffect(() => {
    let cancelled = false;
    let tick: number | undefined;
    (async () => {
      try {
        if (!navigator.mediaDevices?.getUserMedia || typeof MediaRecorder === "undefined") {
          throw new Error("unsupported");
        }
        const s = await navigator.mediaDevices.getUserMedia({ audio: true });
        if (cancelled) {
          s.getTracks().forEach((t) => t.stop());
          return;
        }
        stream.current = s;
        const mime = pickMime();
        const r = new MediaRecorder(s, mime ? { mimeType: mime } : undefined);
        r.ondataavailable = (e) => {
          if (e.data.size > 0) chunks.current.push(e.data);
        };
        r.start(250);
        rec.current = r;
        started.current = performance.now();
        setPhase("recording");
        tick = window.setInterval(() => setElapsed(performance.now() - started.current), 250);
      } catch {
        if (!cancelled) {
          setPhase("error");
          setError("OpenFrame couldn't use the microphone. Check that one is connected and that Windows allows apps to use it (Settings › Privacy & security › Microphone).");
        }
      }
    })();
    return () => {
      cancelled = true;
      if (tick) window.clearInterval(tick);
      if (rec.current && rec.current.state !== "inactive") rec.current.stop();
      stream.current?.getTracks().forEach((t) => t.stop());
    };
  }, []);

  const stop = async (): Promise<{ blob: Blob; durationMs: number } | null> => {
    const r = rec.current;
    if (!r) return result.current;
    if (r.state !== "inactive") {
      await new Promise<void>((res) => {
        r.onstop = () => res();
        r.stop();
      });
    }
    stream.current?.getTracks().forEach((t) => t.stop());
    const durationMs = Math.round(performance.now() - started.current);
    rec.current = null;
    result.current = { blob: new Blob(chunks.current, { type: r.mimeType || "audio/webm" }), durationMs };
    setElapsed(durationMs);
    setPhase("stopped");
    return result.current;
  };

  const save = async () => {
    const res = await stop();
    if (!res || res.blob.size === 0) {
      setError("Nothing was recorded.");
      return;
    }
    setPhase("saving");
    try {
      const bytes = new Uint8Array(await res.blob.arrayBuffer());
      const mediaType = res.blob.type || "audio/webm";
      const item = await vault.ingest({
        store: scope,
        itemType: "voice",
        fileName: `Voice note ${stamp()}.${extForMime(mediaType)}`,
        mediaType,
        dataBase64: bytesToBase64(bytes),
        durationMs: res.durationMs,
        title: name.trim() || null,
      });
      onSaved(item);
    } catch (e) {
      // The recording stays in memory so saving can be retried.
      setPhase("stopped");
      setError(errText(e));
    }
  };

  const time = formatDuration(elapsed) ?? "0:00";
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Record Note"
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" icon={<Square size={12} />} disabled={phase === "starting" || phase === "saving" || phase === "error"} onClick={() => void save()}>
            {phase === "stopped" ? "Save" : "Stop & Save"}
          </Button>
        </>
      }
    >
      {phase === "error" ? (
        <Banner tone="err" actions={<Button size="sm" onClick={onChooseFile}>Choose an audio file…</Button>}>{error}</Banner>
      ) : (
        <>
          <div className="vv-rec" aria-live="polite">
            <span className={`vv-recdot${phase === "recording" ? " on" : ""}`} aria-hidden><Mic size={18} /></span>
            <span className="vv-rectime">{time}</span>
            <span className="sm muted">
              {phase === "starting" ? "Starting microphone…" : phase === "recording" ? "Recording…" : phase === "saving" ? "Saving…" : "Recording stopped"}
            </span>
          </div>
          <Field label="Name (optional)" htmlFor="vv-rec-name">
            <TextInput id="vv-rec-name" value={name} placeholder="Leave blank to keep it untitled" onChange={(e) => setName(e.target.value)} />
          </Field>
          {error && <div className="errtxt" role="alert">{error}</div>}
          <div className="xs muted">The recording is always kept exactly as recorded.</div>
        </>
      )}
    </Dialog>
  );
}

// -------------------------------------------------------------------- Sketch

const INKS = [
  { color: "#1d2130", label: "Black ink" },
  { color: "#c8443a", label: "Red ink" },
  { color: "#2a57ad", label: "Blue ink" },
];

/** A simple drawing surface; saved as a PNG sketch item ("No transcription needed"). */
export function SketchDialog({ scope, onClose, onSaved }: { scope: Scope; onClose: () => void; onSaved: (i: VaultItemDto) => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [ink, setInk] = useState(INKS[0].color);
  const [size, setSize] = useState(3);
  const [erase, setErase] = useState(false);
  const [name, setName] = useState("");
  const [drawn, setDrawn] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const last = useRef<{ x: number; y: number } | null>(null);

  const clear = useCallback(() => {
    const c = canvas.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    ctx.fillStyle = "#ffffff";
    ctx.fillRect(0, 0, c.width, c.height);
    setDrawn(false);
  }, []);
  useEffect(clear, [clear]);

  const point = (e: ReactPointerEvent<HTMLCanvasElement>) => {
    const c = canvas.current!;
    const r = c.getBoundingClientRect();
    return { x: ((e.clientX - r.left) * c.width) / r.width, y: ((e.clientY - r.top) * c.height) / r.height };
  };
  const draw = (from: { x: number; y: number }, to: { x: number; y: number }) => {
    const ctx = canvas.current?.getContext("2d");
    if (!ctx) return;
    ctx.strokeStyle = erase ? "#ffffff" : ink;
    ctx.lineWidth = erase ? size * 4 : size;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(from.x, from.y);
    ctx.lineTo(to.x, to.y);
    ctx.stroke();
  };

  const save = async () => {
    const c = canvas.current;
    if (!c) return;
    setBusy(true);
    try {
      const blob = await new Promise<Blob | null>((res) => c.toBlob(res, "image/png"));
      if (!blob) throw new Error("empty");
      const item = await vault.ingest({
        store: scope,
        itemType: "sketch",
        fileName: `Sketch ${stamp()}.png`,
        mediaType: "image/png",
        dataBase64: bytesToBase64(new Uint8Array(await blob.arrayBuffer())),
        title: name.trim() || null,
      });
      onSaved(item);
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="New Sketch"
      sub="Draw freely. It is saved as an image — no transcription needed."
      size="lg"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !drawn} onClick={() => void save()}>Save Sketch</Button>
        </>
      }
    >
      <div className="row wrap" style={{ marginBottom: 8 }}>
        {INKS.map((i) => (
          <button
            key={i.color}
            type="button"
            className={`vv-swatch${!erase && ink === i.color ? " on" : ""}`}
            style={{ background: i.color }}
            aria-label={i.label}
            aria-pressed={!erase && ink === i.color}
            onClick={() => { setInk(i.color); setErase(false); }}
          />
        ))}
        <span className="seg" role="radiogroup" aria-label="Line width">
          {[2, 4, 8].map((s) => (
            <button key={s} type="button" role="radio" aria-checked={size === s} className={size === s ? "on" : ""} onClick={() => setSize(s)}>
              {s === 2 ? "Fine" : s === 4 ? "Medium" : "Bold"}
            </button>
          ))}
        </span>
        <Button size="sm" variant={erase ? "on" : "default"} icon={<Eraser size={13} />} aria-pressed={erase} onClick={() => setErase(!erase)}>Eraser</Button>
        <span className="grow" />
        <Button size="sm" onClick={clear}>Clear</Button>
      </div>
      <canvas
        ref={canvas}
        width={1200}
        height={700}
        className="vv-canvas"
        role="img"
        aria-label="Sketch drawing area. Draw with the mouse, pen or touch."
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          last.current = point(e);
          draw(last.current, last.current);
          setDrawn(true);
        }}
        onPointerMove={(e) => {
          if (!last.current) return;
          const p = point(e);
          draw(last.current, p);
          last.current = p;
        }}
        onPointerUp={() => (last.current = null)}
        onPointerCancel={() => (last.current = null)}
      />
      <Field label="Name (optional)" htmlFor="vv-sketch-name" error={error}>
        <TextInput id="vv-sketch-name" value={name} placeholder="Leave blank to keep it untitled" onChange={(e) => setName(e.target.value)} />
      </Field>
    </Dialog>
  );
}

// -------------------------------------------------------------- Quick Capture

/** Mock 043 / FSD §10: one field, saves and closes; save errors keep the text. */
export function QuickCaptureDialog({ scope, context, onClose }: { scope: Scope; context: string; onClose: () => void }) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const save = async () => {
    if (!text.trim() || busy) return;
    setBusy(true);
    try {
      await vault.create({ store: scope, itemType: "note", body: text });
      toast.undoable(`Saved to the ${scopeLabel(scope)}`, scope);
      onClose();
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Quick Capture"
      sub={context}
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !text.trim()} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      <TextArea
        autoFocus
        rows={5}
        value={text}
        aria-label="Idea"
        placeholder="Type the idea before it's gone…"
        onChange={(e) => { setText(e.target.value); setError(null); }}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey || !e.shiftKey)) {
            e.preventDefault();
            void save();
          }
        }}
      />
      {error && <div className="errtxt" role="alert">{error}</div>}
      <div className="xs muted" style={{ marginTop: 6 }}>Enter or Ctrl+Enter to save · Shift+Enter for a new line · Esc to cancel</div>
    </Dialog>
  );
}

// ------------------------------------------------------------ Send to Story

const STORY_OPTIONS: { value: VaultStoryTarget; label: string; hint?: string }[] = [
  { value: "beat", label: "Beat", hint: "a small story event" },
  { value: "sceneCard", label: "Scene Card", hint: "a compact scene reminder" },
  { value: "sequence", label: "Sequence idea", hint: "a group of scenes" },
  { value: "character", label: "Character note" },
];

/** Mock 039: creates a copy in Story; the Vault original stays (FSD §5.10). */
export function SendToStoryDialog({ scope, item, onClose }: { scope: Scope; item: VaultItemDto; onClose: () => void }) {
  const targets = useStoryTargets(true);
  const go = useNav((s) => s.go);
  const [target, setTarget] = useState<VaultStoryTarget>("sceneCard");
  const [place, setPlace] = useState("parking");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const acts = useMemo(() => targets.data?.acts ?? [], [targets.data]);

  const placeOptions = useMemo(() => {
    if (target === "character") return [];
    const opts: { value: string; label: string }[] = [];
    if (target !== "sequence") opts.push({ value: "parking", label: "Parking Lot" });
    for (const a of acts) {
      opts.push({ value: `act:${a.id}`, label: a.title });
      if (target !== "sequence") for (const s of a.sequences) opts.push({ value: `seq:${s.id}`, label: `${a.title} ▸ ${s.title}` });
    }
    return opts;
  }, [acts, target]);

  useEffect(() => {
    if (placeOptions.length > 0 && !placeOptions.some((o) => o.value === place)) setPlace(placeOptions[0].value);
  }, [placeOptions, place]);

  const submit = async () => {
    setBusy(true);
    setError(null);
    const [kind, id] = place.split(":");
    const parentType = target === "character" ? null : kind === "act" ? "act" : kind === "seq" ? "sequence" : "parking";
    try {
      const r = await vault.sendToStory({ store: scope, id: item.id, target, parentType, parentId: id ?? null });
      toast.info(r.message, { label: "Open Story Board", run: () => go({ workspace: "story" }) });
      onClose();
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const needsAct = target === "sequence" && acts.length === 0;
  const preview = item.body ?? item.caption ?? item.url ?? item.displayName;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Send to Story Board"
      sub={<>What should this become? This creates a <b>copy</b>. The original stays in your Idea Vault.</>}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || needsAct || targets.isLoading} onClick={() => void submit()}>Create Copy in Story Board</Button>
        </>
      }
    >
      <div className="card pad" style={{ marginBottom: 12, background: item.itemType === "note" ? "#fff9e8" : undefined }}>
        <div className="xs muted" style={{ marginBottom: 2 }}>Idea</div>
        <div className="vv-clamp-5">{item.title ? <b>{item.title}. </b> : null}{preview}</div>
      </div>
      <fieldset className="vv-fieldset">
        <legend className="h4">What should this become?</legend>
        {STORY_OPTIONS.map((o) => (
          <label key={o.value} className="check vv-radio">
            <input type="radio" name="vv-story-target" value={o.value} checked={target === o.value} onChange={() => setTarget(o.value)} style={{ accentColor: "var(--accent)" }} />
            <span><b>{o.label}</b>{o.hint && <span className="muted"> — {o.hint}</span>}</span>
          </label>
        ))}
      </fieldset>
      {target !== "character" && (
        <Field label="Place it in" htmlFor="vv-story-place" hint={target === "sequence" ? "A sequence idea goes into an act." : "Or leave it in the Parking Lot."}>
          {needsAct ? (
            <Banner tone="info">There are no acts yet. Add an act on the Story Board first, or choose Beat or Scene Card.</Banner>
          ) : (
            <select id="vv-story-place" className="select" value={place} onChange={(e) => setPlace(e.target.value)}>
              {placeOptions.map((o) => (
                <option key={o.value} value={o.value}>{o.label}</option>
              ))}
            </select>
          )}
        </Field>
      )}
      {error && <div className="errtxt" role="alert">{error}</div>}
    </Dialog>
  );
}

// ---------------------------------------------------------- Copy between vaults

/** Mock 041: copies are independent; nothing is synced (FSD §5.8, §88.9). */
export function CopyDialog({ from, items, projectLabel, onClose, onDone }: {
  from: Scope;
  items: VaultItemDto[];
  projectLabel: string;
  onClose: () => void;
  onDone?: () => void;
}) {
  const to: Scope = from === "global" ? "project" : "global";
  const dest = useVaultOverview(to);
  const [collectionId, setCollectionId] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const title = to === "project" ? "Copy to Project" : "Copy to Global Vault";
  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await vault.copy({ from, to, ids: items.map((i) => i.id), collectionId: collectionId || null });
      if (r.failed.length > 0) {
        toast.error(`${r.copied.length} copied. ${r.failed.length} could not be copied: ${r.failed.map((f) => `${f.name} (${f.reason})`).join("; ")}`);
      } else {
        toast.undoable(`Copied ${itemsPhrase(items)} to the ${scopeLabel(to)}`, to);
      }
      onDone?.();
      onClose();
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={title}
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={() => void submit()}>{title}</Button>
        </>
      }
    >
      <p style={{ marginTop: 0 }}>
        {to === "project"
          ? "The copy is independent. Editing it will not change the Global original, and the Global original stays where it is."
          : "The copy is independent. Editing it will not change this project's item, and the project item stays where it is."}
      </p>
      <div className="card pad" style={{ marginBottom: 12 }}>
        {items.length === 1 ? (
          <>
            <div className="b vv-ellipsis">{items[0].displayName}</div>
            <div className="xs muted">{from === "global" ? "Global" : "Project"} {items[0].itemType === "note" ? "note" : "item"}</div>
          </>
        ) : (
          <div className="b">{items.length} items</div>
        )}
      </div>
      <Field label="Copy into">
        <div className="input" aria-readonly>{to === "project" ? projectLabel : "Global Idea Vault"}</div>
      </Field>
      <Field label="Collection (optional)" htmlFor="vv-copy-coll">
        <select id="vv-copy-coll" className="select" value={collectionId} onChange={(e) => setCollectionId(e.target.value)}>
          <option value="">None</option>
          {(dest.data?.collections ?? []).map((c) => (
            <option key={c.id} value={c.id}>{c.name}</option>
          ))}
        </select>
      </Field>
      {error && <div className="errtxt" role="alert">{error}</div>}
    </Dialog>
  );
}

// -------------------------------------------------------------- name dialog

/** New Collection (mock 046), New Folder and Rename. */
export function NameDialog({ title, label, hint, confirmLabel, initial = "", onSubmit, onClose }: {
  title: string;
  label: string;
  hint?: string;
  confirmLabel: string;
  initial?: string;
  onSubmit: (name: string) => Promise<unknown>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (!name.trim()) {
      setError(`${label} is required.`);
      return;
    }
    setBusy(true);
    try {
      await onSubmit(name.trim());
      onClose();
    } catch (e) {
      setError(errText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={title}
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={() => void submit()}>{confirmLabel}</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); void submit(); }}>
        <Field label={label} required htmlFor="vv-name" error={error} hint={hint}>
          <TextInput id="vv-name" autoFocus value={name} maxLength={120} invalid={!!error} onChange={(e) => { setName(e.target.value); setError(null); }} />
        </Field>
        <button type="submit" hidden aria-hidden tabIndex={-1} />
      </form>
    </Dialog>
  );
}

export const COLLECTION_HINT =
  "Examples: Ending ideas · Visual References · Crazy scenes · Locations · Research · Music · Things to remember. OpenFrame never prescribes names.";

// ---------------------------------------------------------- move / collection

export function MoveToFolderDialog({ scope, items, folders, onClose }: { scope: Scope; items: VaultItemDto[]; folders: VaultFolderDto[]; onClose: () => void }) {
  const current = items.length === 1 ? items[0].folderId ?? "" : "__keep";
  const [target, setTarget] = useState<string>(current === "__keep" ? "" : current);
  const [newFolder, setNewFolder] = useState(false);
  const [busy, setBusy] = useState(false);
  const tree = folderTree(folders);
  const submit = async () => {
    setBusy(true);
    try {
      await vault.moveToFolder(scope, items.map((i) => i.id), target || null);
      const name = folders.find((f) => f.id === target)?.name;
      toast.undoable(`Moved ${itemsPhrase(items)} to ${name ?? "All items (no folder)"}`, scope);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  if (newFolder) {
    return (
      <NameDialog
        title="New Folder"
        label="Folder name"
        confirmLabel="Create and Move"
        onClose={() => setNewFolder(false)}
        onSubmit={async (name) => {
          const f = await vault.createFolder(scope, name, null);
          await vault.moveToFolder(scope, items.map((i) => i.id), f.id);
          toast.undoable(`Moved ${itemsPhrase(items)} to ${name}`, scope);
          onClose();
        }}
      />
    );
  }
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Move to Folder"
      sub="Folders only organise. Nothing is copied or changed."
      size="sm"
      footer={
        <>
          <Button icon={<FolderPlus size={13} />} onClick={() => setNewFolder(true)}>New Folder…</Button>
          <span className="grow" />
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy} onClick={() => void submit()}>Move</Button>
        </>
      }
    >
      <div role="radiogroup" aria-label="Folder" className="vv-picklist">
        <label className="check vv-radio">
          <input type="radio" name="vv-folder" checked={target === ""} onChange={() => setTarget("")} />
          <span>No folder (All items)</span>
        </label>
        {tree.map(({ folder, depth }) => (
          <label key={folder.id} className="check vv-radio" style={{ paddingLeft: 8 + depth * 16 }}>
            <input type="radio" name="vv-folder" checked={target === folder.id} onChange={() => setTarget(folder.id)} />
            <span>{folder.name}</span>
          </label>
        ))}
      </div>
    </Dialog>
  );
}

export function AddToCollectionDialog({ scope, items, collections, onClose }: { scope: Scope; items: VaultItemDto[]; collections: VaultCollectionDto[]; onClose: () => void }) {
  const [target, setTarget] = useState(collections[0]?.id ?? "");
  const [creating, setCreating] = useState(collections.length === 0);
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    if (!target) return;
    setBusy(true);
    try {
      await vault.addToCollection(scope, items.map((i) => i.id), target);
      toast.undoable(`Added ${itemsPhrase(items)} to ${collections.find((c) => c.id === target)?.name ?? "the collection"}`, scope);
      onClose();
    } catch (e) {
      reportError(e);
    } finally {
      setBusy(false);
    }
  };
  if (creating) {
    return (
      <NameDialog
        title="New Collection"
        label="Name"
        hint={COLLECTION_HINT}
        confirmLabel="Create"
        onClose={() => (collections.length === 0 ? onClose() : setCreating(false))}
        onSubmit={async (name) => {
          const c = await vault.createCollection(scope, name);
          await vault.addToCollection(scope, items.map((i) => i.id), c.id);
          toast.undoable(`Added ${itemsPhrase(items)} to ${name}`, scope);
          onClose();
        }}
      />
    );
  }
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Add to Collection"
      sub="Collections are references. The items stay where they are."
      size="sm"
      footer={
        <>
          <Button onClick={() => setCreating(true)}>New Collection…</Button>
          <span className="grow" />
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={busy || !target} onClick={() => void submit()}>Add</Button>
        </>
      }
    >
      <div role="radiogroup" aria-label="Collection" className="vv-picklist">
        {collections.map((c) => (
          <label key={c.id} className="check vv-radio">
            <input type="radio" name="vv-coll" checked={target === c.id} onChange={() => setTarget(c.id)} />
            <span>{c.name} <span className="xs muted">{c.itemCount}</span></span>
          </label>
        ))}
      </div>
    </Dialog>
  );
}

export function TagDialog({ scope, items, onClose }: { scope: Scope; items: VaultItemDto[]; onClose: () => void }) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const submit = async () => {
    const tags = parseTags(text);
    if (tags.length === 0) {
      setError("Type at least one tag.");
      return;
    }
    try {
      await vault.addTags(scope, items.map((i) => i.id), tags);
      toast.undoable(`Tagged ${itemsPhrase(items)}`, scope);
      onClose();
    } catch (e) {
      setError(errText(e));
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Tag"
      sub={`Add tags to ${itemsPhrase(items)}. Tags help you find things; they never classify.`}
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" onClick={() => void submit()}>Add Tags</Button>
        </>
      }
    >
      <form onSubmit={(e) => { e.preventDefault(); void submit(); }}>
        <Field label="Tags" htmlFor="vv-tags" hint="Separate tags with commas." error={error}>
          <TextInput id="vv-tags" autoFocus value={text} placeholder="rain, night, locations" onChange={(e) => { setText(e.target.value); setError(null); }} />
        </Field>
        <button type="submit" hidden aria-hidden tabIndex={-1} />
      </form>
    </Dialog>
  );
}

// ------------------------------------------------------------- add results

/** Mock 035: successful items were kept; only failed ones need attention. */
export function AddResultDialog({ result, onRetry, onClose }: { result: VaultAddFilesResult; onRetry: (paths: string[]) => void; onClose: () => void }) {
  const added = result.added.length;
  const failed = result.failed.length;
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title={`${added} ${added === 1 ? "item" : "items"} added, ${failed} could not be added`}
      sub="Successful items were kept. Only the failed item needs attention."
      size="md"
      footer={
        <>
          <Button onClick={() => onRetry(result.failed.map((f) => f.path))}>{failed === 1 ? "Try Failed Item Again" : "Try Failed Items Again"}</Button>
          <Button variant="primary" onClick={onClose}>Done</Button>
        </>
      }
    >
      <table className="tbl">
        <tbody>
          {result.added.map((i) => (
            <tr key={i.id}>
              <td className="vv-ellipsis">{i.displayName}</td>
              <td><span className="chip g">Added</span></td>
              <td />
            </tr>
          ))}
          {result.failed.map((f) => (
            <tr key={f.path}>
              <td className="vv-ellipsis">{f.name}</td>
              <td><span className="chip r">Not added</span></td>
              <td className="sm">{f.reason}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
