// Advanced screenplay settings (Screenplay and Production Depth Specification
// §8, §14, §15, §17; ADR-0015): page setup with presets, header and footer,
// custom elements, a scene's production number, and the page view that shows
// the pages exactly as they print (the same layout as the PDF).

import { useEffect, useMemo, useState } from "react";
import { Button, Checkbox, Dialog, Drawer, Field, Select, Skeleton, TextInput, Banner } from "../../design-system";
import { call, OpError } from "../../ipc/client";
import { reportError } from "../../ipc/query";
import { toast } from "../../app/toast";
import type { ScreenplayPageSetup } from "../../ipc/generated/ScreenplayPageSetup";
import type { ScreenplayPaper } from "../../ipc/generated/ScreenplayPaper";
import type { ScreenplayLineSpacing } from "../../ipc/generated/ScreenplayLineSpacing";
import type { ScreenplayCustomElementDto } from "../../ipc/generated/ScreenplayCustomElementDto";
import type { ScreenplaySceneNumberResult } from "../../ipc/generated/ScreenplaySceneNumberResult";
import { ScaledPage, pageTitle } from "../../features/interchange/PrintPreview";
import { usePageSetup, usePageView } from "./api";

const PLACEHOLDERS = ["{title}", "{draft}", "{revision}", "{date}", "{page}"] as const;

/** Page setup, header and footer, title page switch and presets. */
export function PageSetupDialog({ screenplayId, canEdit, onClose }: { screenplayId: string; canEdit: boolean; onClose: () => void }) {
  const info = usePageSetup(screenplayId);
  const [setup, setSetup] = useState<ScreenplayPageSetup | null>(null);
  const [presetName, setPresetName] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (info.data && !setup) setSetup(info.data.setup);
  }, [info.data, setup]);
  const set = <K extends keyof ScreenplayPageSetup>(k: K, v: ScreenplayPageSetup[K]) => setSetup((s) => (s ? { ...s, [k]: v } : s));
  const save = async () => {
    if (!setup) return;
    setBusy(true);
    try {
      await call("screenplay.update_page_setup", { screenplayId, setup });
      toast.undoable("Changed the page setup");
      onClose();
    } catch (e) {
      setErr(e instanceof OpError ? e.message : "The page setup could not be saved.");
    } finally {
      setBusy(false);
    }
  };
  const savePreset = async () => {
    if (!setup || !presetName.trim()) return;
    try {
      await call("screenplay.save_page_preset", { name: presetName.trim(), setup });
      toast.undoable(`Saved the preset “${presetName.trim()}”`);
      setPresetName("");
    } catch (e) {
      setErr(e instanceof OpError ? e.message : "The preset could not be saved.");
    }
  };
  const deletePreset = async (id: string, name: string) => {
    try {
      await call("screenplay.delete_page_preset", { id });
      toast.undoable(`Moved the preset “${name}” to Recently Deleted`);
    } catch (e) {
      reportError(e);
    }
  };
  const insert = (k: "header" | "footer", p: string) => setSetup((s) => (s ? { ...s, [k]: `${s[k]}${s[k] && !s[k].endsWith(" ") ? " " : ""}${p}` } : s));
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Page setup"
      sub="Saved with this screenplay. The page view, print preview and PDF all use it."
      size="lg"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!canEdit || !setup || busy} onClick={() => void save()}>Save</Button>
        </>
      }
    >
      {!setup || !info.data ? (
        <Skeleton h={200} />
      ) : (
        <div className="spx-setup">
          {err && <Banner tone="err">{err}</Banner>}
          <Field label="Preset" htmlFor="ps-preset" hint="Applies the preset's settings here; Save keeps them.">
            <div className="row wrap gap4" id="ps-preset">
              {info.data.presets.map((p) => (
                <span key={p.id} className="row gap4">
                  <Button size="sm" onClick={() => setSetup(p.setup)}>{p.name}</Button>
                  {!p.builtIn && canEdit && (
                    <button type="button" className="iconbtn" aria-label={`Delete the preset ${p.name}`} onClick={() => void deletePreset(p.id, p.name)}>×</button>
                  )}
                </span>
              ))}
            </div>
          </Field>
          <div className="row wrap gap8">
            <Field label="Paper" htmlFor="ps-paper">
              <Select<ScreenplayPaper> id="ps-paper" value={setup.paper} onChange={(v) => set("paper", v)} options={[{ value: "Letter", label: "US Letter" }, { value: "A4", label: "A4" }]} />
            </Field>
            <Field label="Font" htmlFor="ps-font">
              <Select id="ps-font" value={setup.font} onChange={(v) => set("font", v)} options={info.data.fonts.map((f) => ({ value: f, label: f }))} />
            </Field>
            <Field label="Size" htmlFor="ps-size">
              <Select
                id="ps-size"
                value={String(setup.fontSize)}
                onChange={(v) => set("fontSize", Number(v))}
                options={Array.from({ length: info.data.maxFontSize - info.data.minFontSize + 1 }, (_, i) => {
                  const n = info.data!.minFontSize + i;
                  return { value: String(n), label: n === 12 ? "12 pt (standard)" : `${n} pt` };
                })}
              />
            </Field>
            <Field label="Line spacing" htmlFor="ps-spacing">
              <Select<ScreenplayLineSpacing>
                id="ps-spacing"
                value={setup.lineSpacing}
                onChange={(v) => set("lineSpacing", v)}
                options={[{ value: "single", label: "Single (standard)" }, { value: "oneAndHalf", label: "1.5 lines" }, { value: "double", label: "Double" }]}
              />
            </Field>
            <Field label="First page number" htmlFor="ps-first">
              <TextInput id="ps-first" type="number" min={1} max={9999} value={setup.firstPageNumber} onChange={(e) => set("firstPageNumber", Math.max(1, Number(e.target.value) || 1))} style={{ width: 90 }} />
            </Field>
          </div>
          {(["header", "footer"] as const).map((k) => (
            <Field key={k} label={k === "header" ? "Header" : "Footer"} htmlFor={`ps-${k}`} hint="Optional. Placeholders fill in on every page.">
              <TextInput id={`ps-${k}`} value={setup[k]} maxLength={120} onChange={(e) => set(k, e.target.value)} placeholder={k === "header" ? "{title} {draft}" : "Page {page}"} />
              <div className="row wrap gap4" style={{ marginTop: 4 }}>
                {PLACEHOLDERS.map((p) => (
                  <button key={p} type="button" className="chip" onClick={() => insert(k, p)} aria-label={`Insert ${p} into the ${k}`}>{p}</button>
                ))}
              </div>
            </Field>
          ))}
          <Checkbox checked={setup.numberFirstPage} onChange={(v) => set("numberFirstPage", v)} label="Print the page number on the first page" />
          <Checkbox checked={setup.headerOnFirstPage} onChange={(v) => set("headerOnFirstPage", v)} label="Show the header and footer on the first page" />
          <Checkbox checked={setup.includeTitlePage} onChange={(v) => set("includeTitlePage", v)} label="Include the title page when exporting and printing" />
          <Checkbox checked={setup.sceneNumbers} onChange={(v) => set("sceneNumbers", v)} label="Print scene numbers" />
          <Checkbox checked={setup.revisionMarks} onChange={(v) => set("revisionMarks", v)} label="Mark revised lines with an asterisk" />
          <p className="xs muted">{info.data.linesPerPage} lines fit on a page with the saved settings.</p>
          {canEdit && (
            <div className="row gap4">
              <TextInput aria-label="New preset name" placeholder="Save these settings as a preset" value={presetName} maxLength={80} onChange={(e) => setPresetName(e.target.value)} />
              <Button size="sm" disabled={!presetName.trim()} onClick={() => void savePreset()}>Save preset</Button>
            </div>
          )}
        </div>
      )}
    </Dialog>
  );
}

const LOOKS = [
  { value: "action", label: "Like Action" },
  { value: "centered", label: "Centred" },
  { value: "character", label: "Like Character" },
  { value: "parenthetical", label: "Like Parenthetical" },
  { value: "dialogue", label: "Like Dialogue" },
  { value: "transition", label: "Like Transition (right)" },
  { value: "shot", label: "Like Shot" },
] as const;

const NEXT_BUILT_INS = [
  { value: "action", label: "Action" },
  { value: "scene_heading", label: "Scene Heading" },
  { value: "character", label: "Character" },
  { value: "parenthetical", label: "Parenthetical" },
  { value: "dialogue", label: "Dialogue" },
  { value: "transition", label: "Transition" },
  { value: "shot", label: "Shot" },
  { value: "note", label: "Note" },
];

interface CustomForm {
  id: string | null;
  name: string;
  shortcut: string;
  look: string;
  uppercase: boolean;
  italic: boolean;
  spaceBefore: number;
  nextElement: string;
  exportName: string;
}

const EMPTY: CustomForm = { id: null, name: "", shortcut: "", look: "action", uppercase: false, italic: false, spaceBefore: 1, nextElement: "action", exportName: "" };

/** Custom elements under Advanced (name, shortcut, look, next element, import and export identity). */
export function CustomElementsDialog({ screenplayId, elements, canEdit, onClose }: {
  screenplayId: string;
  elements: ScreenplayCustomElementDto[];
  canEdit: boolean;
  onClose: () => void;
}) {
  const [form, setForm] = useState<CustomForm | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const set = <K extends keyof CustomForm>(k: K, v: CustomForm[K]) => setForm((f) => (f ? { ...f, [k]: v } : f));
  const save = async () => {
    if (!form) return;
    const args = {
      name: form.name,
      shortcut: form.shortcut,
      look: form.look,
      uppercase: form.uppercase,
      italic: form.italic,
      spaceBefore: form.spaceBefore,
      nextElement: form.nextElement,
      exportName: form.exportName,
    };
    try {
      if (form.id) await call("screenplay.update_custom_element", { id: form.id, ...args });
      else await call("screenplay.create_custom_element", { screenplayId, ...args });
      toast.undoable(form.id ? `Changed “${form.name}”` : `Added the custom element “${form.name}”`);
      setForm(null);
      setErr(null);
    } catch (e) {
      setErr(e instanceof OpError ? e.message : "The custom element could not be saved.");
    }
  };
  const remove = async (e: ScreenplayCustomElementDto) => {
    try {
      await call("screenplay.delete_custom_element", { id: e.id });
      toast.undoable(`Moved “${e.name}” to Recently Deleted`);
    } catch (x) {
      reportError(x);
    }
  };
  const nextOptions = useMemo(
    () => [...NEXT_BUILT_INS, ...elements.map((e) => ({ value: `custom:${e.id}`, label: e.name }))],
    [elements],
  );
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Custom elements"
      sub="Extra paragraph kinds for this screenplay, such as a telephone voice or a lower third. Final Draft custom elements arrive here when you import."
      size="lg"
      footer={<Button onClick={onClose}>Done</Button>}
    >
      {err && <Banner tone="err">{err}</Banner>}
      {elements.length === 0 && !form && <p className="muted sm">No custom elements yet.</p>}
      <div className="card">
        {elements.map((e) => (
          <div key={e.id} className="li">
            <div className="grow">
              <b>{e.name}</b>
              <div className="xs muted">
                {LOOKS.find((l) => l.value === e.look)?.label ?? e.look}
                {e.uppercase ? " · capitals" : ""}
                {e.italic ? " · italic" : ""}
                {e.shortcut ? ` · ${e.shortcut}` : ""} · Final Draft name {e.exportName} · used {e.usageCount} time{e.usageCount === 1 ? "" : "s"}
              </div>
            </div>
            {canEdit && (
              <>
                <Button size="sm" onClick={() => setForm({ id: e.id, name: e.name, shortcut: e.shortcut ?? "", look: e.look, uppercase: e.uppercase, italic: e.italic, spaceBefore: e.spaceBefore, nextElement: e.nextElement, exportName: e.exportName })}>Edit</Button>
                <Button size="sm" disabled={e.usageCount > 0} title={e.usageCount > 0 ? "In use: change those paragraphs to another element first." : undefined} onClick={() => void remove(e)}>Delete</Button>
              </>
            )}
          </div>
        ))}
      </div>
      {canEdit && !form && (
        <Button style={{ marginTop: 10 }} onClick={() => setForm(EMPTY)}>Add custom element</Button>
      )}
      {form && (
        <div className="card" style={{ marginTop: 10, padding: 10 }}>
          <Field label="Name" required htmlFor="ce-name">
            <TextInput id="ce-name" autoFocus value={form.name} maxLength={40} onChange={(e) => set("name", e.target.value)} />
          </Field>
          <div className="row wrap gap8">
            <Field label="Looks" htmlFor="ce-look">
              <Select id="ce-look" value={form.look} onChange={(v) => set("look", v)} options={LOOKS} />
            </Field>
            <Field label="Shortcut" htmlFor="ce-short">
              <Select
                id="ce-short"
                value={form.shortcut}
                onChange={(v) => set("shortcut", v)}
                options={[{ value: "", label: "None" }, ...Array.from({ length: 9 }, (_, i) => ({ value: `Ctrl+Alt+${i + 1}`, label: `Ctrl+Alt+${i + 1}` }))]}
              />
            </Field>
            <Field label="Blank lines before" htmlFor="ce-space">
              <Select id="ce-space" value={String(form.spaceBefore)} onChange={(v) => set("spaceBefore", Number(v))} options={["0", "1", "2", "3"].map((n) => ({ value: n, label: n }))} />
            </Field>
            <Field label="Enter creates" htmlFor="ce-next">
              <Select id="ce-next" value={form.nextElement} onChange={(v) => set("nextElement", v)} options={nextOptions} />
            </Field>
          </div>
          <Checkbox checked={form.uppercase} onChange={(v) => set("uppercase", v)} label="Capital letters" />
          <Checkbox checked={form.italic} onChange={(v) => set("italic", v)} label="Italic" />
          <Field label="Name in Final Draft files" htmlFor="ce-export" hint="Used when exporting and importing .fdx. Empty uses the name in capitals.">
            <TextInput id="ce-export" value={form.exportName} maxLength={60} onChange={(e) => set("exportName", e.target.value)} />
          </Field>
          <div className="row gap4">
            <Button onClick={() => setForm(null)}>Cancel</Button>
            <Button variant="primary" disabled={!form.name.trim()} onClick={() => void save()}>{form.id ? "Save" : "Add"}</Button>
          </div>
        </div>
      )}
    </Dialog>
  );
}

/** Set or clear a scene's production number; warns when production planning uses the scene. */
export function SceneNumberDialog({ sceneId, label, onClose }: { sceneId: string; label: string; onClose: () => void }) {
  const [value, setValue] = useState(label);
  const [warning, setWarning] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const save = async (acknowledge: boolean) => {
    try {
      const r = await call<ScreenplaySceneNumberResult>("screenplay.set_scene_number", {
        sceneId,
        number: value.trim() || undefined,
        acknowledgeDependencies: acknowledge,
      });
      toast.undoable(`Scene number is now ${r.label}`);
      onClose();
    } catch (e) {
      if (e instanceof OpError && e.code === "validation.dependencies" && !acknowledge) setWarning(e.message);
      else setErr(e instanceof OpError ? e.message : "The scene number could not be changed.");
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Scene number"
      sub="A number is only a label: breakdown, shots and the schedule stay linked to the scene."
      size="sm"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          {warning ? (
            <Button variant="primary" onClick={() => void save(true)}>Change the number</Button>
          ) : (
            <Button variant="primary" onClick={() => void save(false)}>Save</Button>
          )}
        </>
      }
    >
      <Field label="Number" htmlFor="sn-value" hint="Letters and digits, for example 12 or 12A. Empty returns to the automatic number." error={err}>
        <TextInput id="sn-value" autoFocus value={value} maxLength={12} onChange={(e) => { setValue(e.target.value); setWarning(null); setErr(null); }} />
      </Field>
      {warning && <Banner tone="warn">{warning}</Banner>}
    </Dialog>
  );
}

/** The pages exactly as they print (editor page view: the PDF layout). */
export function PageViewDrawer({ draftId, onClose }: { draftId: string; onClose: () => void }) {
  const [marks, setMarks] = useState(false);
  const view = usePageView(draftId, marks);
  return (
    <Drawer open onClose={onClose} typeLabel="Screenplay" title="Page view" width="w">
      <div className="row gap8" style={{ marginBottom: 8 }}>
        <Checkbox checked={marks} onChange={setMarks} label="Show revision marks" />
        {view.data && <span className="xs muted">{view.data.paper} · {view.data.scriptPageCount} page{view.data.scriptPageCount === 1 ? "" : "s"}</span>}
      </div>
      {!view.data ? (
        <Skeleton h={300} />
      ) : (
        <div className="col gap8" aria-label="Pages">
          {view.data.warnings.map((w) => <Banner key={w.code} tone="warn">{w.message}</Banner>)}
          {view.data.pages.map((_, i) => (
            <div key={i}>
              <div className="xs muted">{pageTitle(view.data!, i)}</div>
              <ScaledPage preview={view.data!} index={i} scale={0.55} />
            </div>
          ))}
        </div>
      )}
    </Drawer>
  );
}
