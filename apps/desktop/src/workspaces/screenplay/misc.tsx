// Title page (mock 100), Scene Hub (mock 097), reorder warning (mock 099) and
// the in-script find bar (mock 078) with its scope: this scene or the whole script
// (Screenplay and Production Depth Specification §18).

import { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp, X } from "lucide-react";
import { Button, Checkbox, Dialog, Drawer, Field, Segmented, Skeleton, TextArea, TextInput } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayDto } from "../../ipc/generated/ScreenplayDto";
import type { ScreenplayTitlePage } from "../../ipc/generated/ScreenplayTitlePage";
import type { ScreenplaySceneHub } from "../../ipc/generated/ScreenplaySceneHub";
import type { ScreenplayReplaceResult } from "../../ipc/generated/ScreenplayReplaceResult";
import { navigateTo } from "../../app/navigate";
import { toast } from "../../app/toast";
import { useSceneHub } from "./api";
import type { InterchangePrintPreview } from "../../ipc/generated/InterchangePrintPreview";
import { ScaledPage } from "../../features/interchange/PrintPreview";
import type { FindOptions } from "./editor/find";

const TITLE_FIELDS: { key: keyof ScreenplayTitlePage; label: string; multi?: boolean; hint?: string }[] = [
  { key: "writtenBy", label: "Written by", multi: true },
  { key: "credit", label: "Credit line", hint: "Empty prints “Written by”." },
  { key: "source", label: "Based on", multi: true, hint: "Optional source material." },
  { key: "draftLine", label: "Draft", hint: "Empty uses the draft name." },
  { key: "revisionLine", label: "Revision", hint: "Empty uses the open revision's label and colour." },
  { key: "date", label: "Date" },
  { key: "contactName", label: "Contact name" },
  { key: "address", label: "Address", multi: true },
  { key: "phone", label: "Phone" },
  { key: "email", label: "Email" },
  { key: "contact", label: "Other contact details", multi: true },
  { key: "representation", label: "Representation", multi: true, hint: "Agent or manager." },
  { key: "notes", label: "Notes on the title page", multi: true },
  { key: "copyright", label: "Copyright" },
];

/** Title page fields with a live preview (mock 100) and the export switch. */
export function TitlePageDialog({ screenplay, draftId, onClose }: { screenplay: ScreenplayDto; draftId: string; onClose: () => void }) {
  const [tp, setTp] = useState<ScreenplayTitlePage>(screenplay.titlePage);
  const [include, setInclude] = useState(screenplay.pageSetup.includeTitlePage);
  const [preview, setPreview] = useState<InterchangePrintPreview | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const set = (k: keyof ScreenplayTitlePage) => (v: string) => setTp((t) => ({ ...t, [k]: v }));
  // Preview from the same layout as the PDF, a moment after typing stops.
  useEffect(() => {
    const t = setTimeout(() => {
      call<InterchangePrintPreview>("screenplay.title_page_preview", { screenplayId: screenplay.id, titlePage: tp, draftId })
        .then(setPreview)
        .catch(() => setPreview(null));
    }, 300);
    return () => clearTimeout(t);
  }, [tp, screenplay.id, draftId]);
  const save = async () => {
    try {
      await call("screenplay.update_title_page", { screenplayId: screenplay.id, titlePage: tp });
      if (include !== screenplay.pageSetup.includeTitlePage) {
        await call("screenplay.update_page_setup", { screenplayId: screenplay.id, setup: { ...screenplay.pageSetup, includeTitlePage: include } });
      }
      toast.undoable("Saved the title page");
      onClose();
    } catch (e) {
      setErr((e as { message?: string }).message ?? "The title page could not be saved.");
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Title page"
      size="xl"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!tp.title.trim()} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      <div className="spx-tp">
        <div className="spx-tp-fields">
          <Field label="Title" required htmlFor="tp-title" error={err}>
            <TextInput id="tp-title" value={tp.title} autoFocus onChange={(e) => set("title")(e.target.value)} maxLength={200} />
          </Field>
          {TITLE_FIELDS.map((f) => (
            <Field key={f.key} label={f.label} htmlFor={`tp-${f.key}`} hint={f.hint}>
              {f.multi ? (
                <TextArea id={`tp-${f.key}`} value={tp[f.key]} rows={2} onChange={(e) => set(f.key)(e.target.value)} />
              ) : (
                <TextInput id={`tp-${f.key}`} value={tp[f.key]} onChange={(e) => set(f.key)(e.target.value)} />
              )}
            </Field>
          ))}
          <Checkbox checked={include} onChange={setInclude} label="Include the title page when exporting and printing" />
        </div>
        <div className="spx-tp-preview" aria-label="Title page preview">
          {preview && preview.pages.length > 0 ? <ScaledPage preview={preview} index={0} scale={0.45} /> : <Skeleton h={300} />}
        </div>
      </div>
    </Dialog>
  );
}

export function SceneHubDrawer({ sceneId, onClose, onOpenComments }: { sceneId: string; onClose: () => void; onOpenComments: () => void }) {
  const hub = useSceneHub(sceneId);
  const h = hub.data;
  return (
    <Drawer
      open
      onClose={onClose}
      typeLabel="Scene Hub"
      title={h ? `Scene ${h.number ?? ""}: ${h.heading || "Untitled scene"}` : "Scene"}
      width="n"
    >
      <p className="sm muted" style={{ marginTop: 0 }}>What exists around this scene?</p>
      {!h ? (
        <Skeleton h={160} />
      ) : (
        <div className="card">
          {h.rows.map((r) => (
            <button
              key={r.key}
              type="button"
              className="li spx-li-btn"
              onClick={() => {
                if (r.key === "comments") onOpenComments();
                else if (r.key !== "screenplay") navigateTo(r.nav);
                onClose();
              }}
            >
              <b className="grow" style={{ textAlign: "left" }}>{r.label}</b>
              <span className="sm muted">{r.detail}</span>
            </button>
          ))}
        </div>
      )}
      <p className="xs muted" style={{ marginTop: 10 }}>
        The hub is navigation and context, not a master edit screen. Each link opens the dedicated workspace focused on this scene.
      </p>
    </Drawer>
  );
}

export function ReorderWarningDialog({ hub, newNumber, onCancel, onContinue }: {
  hub: ScreenplaySceneHub;
  newNumber: number;
  onCancel: () => void;
  onContinue: () => void;
}) {
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onCancel()}
      title="This scene is already used in production planning"
      sub="Changing its order will update its script number but will not automatically reorder the shooting schedule."
      size="md"
      footer={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant="primary" onClick={onContinue}>Continue</Button>
        </>
      }
    >
      <p>
        <b>Scene {hub.number}: {hub.heading || "Untitled scene"}</b> is used in {hub.productionSummary}. It will become Scene {newNumber}.
      </p>
      <p className="sm muted">Production data follows the scene, not its number. Only the displayed number changes.</p>
    </Dialog>
  );
}

export interface FindState {
  index: number;
  count: number;
}

export type FindScope = "script" | "scene";

export function FindBar({ draftId, canReplace, showNotes, result, currentScene, onSearch, onStep, onReplace, onBeforeReplaceAll, onClose }: {
  draftId: string;
  canReplace: boolean;
  showNotes: boolean;
  result: FindState;
  /** The scene the writer is in (used when the scope is "This scene"). */
  currentScene: { id: string; number: number } | null;
  onSearch: (o: FindOptions | null) => void;
  onStep: (dir: 1 | -1) => void;
  onReplace: (replacement: string) => void;
  onBeforeReplaceAll: () => Promise<void>;
  onClose: () => void;
}) {
  const [query, setQuery] = useState("");
  const [replacement, setReplacement] = useState("");
  const [matchCase, setMatchCase] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [scope, setScope] = useState<FindScope>("script");
  // "This scene" stays on the scene chosen when the scope was set, even while stepping
  // through matches or typing in the find box.
  const [scene, setScene] = useState<{ id: string; number: number } | null>(null);
  const sceneId = scope === "scene" ? (scene?.id ?? null) : null;
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  useEffect(() => {
    onSearch(query ? { query, matchCase, wholeWord, sceneId } : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, matchCase, wholeWord, sceneId]);
  const chooseScope = (v: FindScope) => {
    if (v === "scene") {
      if (!currentScene) {
        toast.info("Click in a scene first, then choose This scene.");
        return;
      }
      setScene(currentScene);
    }
    setScope(v);
  };
  const replaceAll = async () => {
    try {
      await onBeforeReplaceAll();
      const r = await call<ScreenplayReplaceResult>("screenplay.replace_all", {
        draftId,
        query,
        replacement,
        matchCase,
        wholeWord,
        includeNotes: showNotes,
        ...(sceneId ? { sceneId } : {}),
      });
      const where = sceneId && scene ? ` in Scene ${scene.number}` : "";
      if (r.count === 0) toast.info(`No matches found${where}`);
      else toast.undoable(`Replaced ${r.count} match${r.count === 1 ? "" : "es"}${where}`);
    } catch (e) {
      reportError(e);
    }
  };
  return (
    <div className="spx-find card" role="search" aria-label="Find in script">
      <div className="row">
        <TextInput
          ref={input}
          aria-label="Find"
          placeholder={sceneId && scene ? `Find in Scene ${scene.number}` : "Find in script"}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              onStep(e.shiftKey ? -1 : 1);
            } else if (e.key === "Escape") {
              e.preventDefault();
              onClose();
            }
          }}
        />
        <span className="xs muted spx-count" aria-live="polite">
          {query ? (result.count === 0 ? "No matches found" : `${result.index + 1} of ${result.count}`) : ""}
        </span>
        <button type="button" className="iconbtn" aria-label="Previous match (Shift+Enter)" title="Previous (Shift+Enter)" disabled={result.count === 0} onClick={() => onStep(-1)}>
          <ChevronUp size={15} />
        </button>
        <button type="button" className="iconbtn" aria-label="Next match (Enter)" title="Next (Enter)" disabled={result.count === 0} onClick={() => onStep(1)}>
          <ChevronDown size={15} />
        </button>
        <button type="button" className="iconbtn" aria-label="Close find (Esc)" title="Close (Esc)" onClick={onClose}>
          <X size={15} />
        </button>
      </div>
      {canReplace && (
        <div className="row" style={{ marginTop: 6 }}>
          <TextInput
            aria-label="Replace with"
            placeholder="Replace with…"
            value={replacement}
            onChange={(e) => setReplacement(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && onClose()}
          />
          <Button size="sm" disabled={result.count === 0} onClick={() => onReplace(replacement)}>Replace</Button>
          <Button size="sm" disabled={!query} onClick={() => void replaceAll()}>Replace All</Button>
        </div>
      )}
      <div className="row" style={{ marginTop: 6 }}>
        <Segmented<FindScope>
          ariaLabel="Search in"
          value={scope}
          onChange={chooseScope}
          options={[
            { value: "scene", label: scene && scope === "scene" ? `This scene (${scene.number})` : "This scene" },
            { value: "script", label: "Whole script" },
          ]}
        />
      </div>
      <div className="row" style={{ marginTop: 6 }}>
        <Checkbox checked={matchCase} onChange={setMatchCase} label="Match case" />
        <Checkbox checked={wholeWord} onChange={setWholeWord} label="Whole word" />
      </div>
    </div>
  );
}

