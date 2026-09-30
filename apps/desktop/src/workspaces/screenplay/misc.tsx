// Title page (mock 100), Scene Hub (mock 097), reorder warning (mock 099) and
// the in-script find bar (mock 078).

import { useEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp, X } from "lucide-react";
import { Button, Checkbox, Dialog, Drawer, Field, Skeleton, TextArea, TextInput } from "../../design-system";
import { call } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import type { ScreenplayDto } from "../../ipc/generated/ScreenplayDto";
import type { ScreenplayTitlePage } from "../../ipc/generated/ScreenplayTitlePage";
import type { ScreenplaySceneHub } from "../../ipc/generated/ScreenplaySceneHub";
import type { ScreenplayReplaceResult } from "../../ipc/generated/ScreenplayReplaceResult";
import { navigateTo } from "../../app/navigate";
import { toast } from "../../app/toast";
import { useSceneHub } from "./api";
import type { FindOptions } from "./editor/find";

export function TitlePageDialog({ screenplay, onClose }: { screenplay: ScreenplayDto; onClose: () => void }) {
  const [tp, setTp] = useState<ScreenplayTitlePage>(screenplay.titlePage);
  const [err, setErr] = useState<string | null>(null);
  const set = (k: keyof ScreenplayTitlePage) => (v: string) => setTp({ ...tp, [k]: v });
  const save = async () => {
    try {
      await call("screenplay.update_title_page", { screenplayId: screenplay.id, titlePage: tp });
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
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!tp.title.trim()} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      <Field label="Title" required htmlFor="tp-title" error={err}>
        <TextInput id="tp-title" value={tp.title} autoFocus onChange={(e) => set("title")(e.target.value)} maxLength={200} />
      </Field>
      <Field label="Written by" htmlFor="tp-by">
        <TextInput id="tp-by" value={tp.writtenBy} onChange={(e) => set("writtenBy")(e.target.value)} />
      </Field>
      <Field label="Contact" htmlFor="tp-contact">
        <TextArea id="tp-contact" value={tp.contact} rows={2} onChange={(e) => set("contact")(e.target.value)} />
      </Field>
      <Field label="Draft / revision line" htmlFor="tp-line">
        <TextInput id="tp-line" value={tp.draftLine} onChange={(e) => set("draftLine")(e.target.value)} />
      </Field>
      <Field label="Notes on the title page" htmlFor="tp-notes" hint="Optional">
        <TextArea id="tp-notes" value={tp.notes} rows={2} onChange={(e) => set("notes")(e.target.value)} />
      </Field>
      <p className="xs muted">Used when you export or print. Revision and draft information can be included by the export options.</p>
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
      title={h ? `Scene ${h.number ?? ""} — ${h.heading || "Untitled scene"}` : "Scene"}
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
        <b>Scene {hub.number} — {hub.heading || "Untitled scene"}</b> is used in {hub.productionSummary}. It will become Scene {newNumber}.
      </p>
      <p className="sm muted">Production data follows the scene, not its number. Only the displayed number changes.</p>
    </Dialog>
  );
}

export interface FindState {
  index: number;
  count: number;
}

export function FindBar({ draftId, canReplace, showNotes, result, onSearch, onStep, onReplace, onBeforeReplaceAll, onClose }: {
  draftId: string;
  canReplace: boolean;
  showNotes: boolean;
  result: FindState;
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
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  useEffect(() => {
    onSearch(query ? { query, matchCase, wholeWord } : null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, matchCase, wholeWord]);
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
      });
      if (r.count === 0) toast.info("No matches found");
      else toast.undoable(`Replaced ${r.count} match${r.count === 1 ? "" : "es"}`);
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
          placeholder="Find in script"
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
        <Checkbox checked={matchCase} onChange={setMatchCase} label="Match case" />
        <Checkbox checked={wholeWord} onChange={setWholeWord} label="Whole word" />
      </div>
    </div>
  );
}

