// Small editing helpers shared by the production tabs: buffered fields that
// commit on blur (Rust holds the truth; the buffer only exists while typing),
// scene chips that open the Breakdown, and file pickers for images.

import { useState, type ReactNode } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { ProductionSceneRef } from "../../../ipc/generated/ProductionSceneRef";
import { Field, TextArea, TextInput } from "../../../design-system";
import { IMAGE_EXTENSIONS } from "../../../api/production";
import { useNav } from "../../../app/stores";

/** Text field that keeps a local buffer while focused and commits on blur / Enter. */
export function BufferedField({ id, label, value, onCommit, multiline, required, placeholder, maxLength, disabled, hint }: {
  id: string;
  label: string;
  value: string | null;
  onCommit: (v: string) => void;
  multiline?: boolean;
  required?: boolean;
  placeholder?: string;
  maxLength?: number;
  disabled?: boolean;
  hint?: ReactNode;
}) {
  const [draft, setDraft] = useState(value ?? "");
  const [prev, setPrev] = useState(value);
  const [focused, setFocused] = useState(false);
  if (value !== prev && !focused) {
    setPrev(value);
    setDraft(value ?? "");
  }
  const commit = () => {
    setFocused(false);
    const v = draft.trim();
    if (v === (value ?? "").trim()) return;
    if (required && !v) {
      setDraft(value ?? "");
      return;
    }
    onCommit(v);
  };
  const common = {
    id,
    value: draft,
    placeholder,
    maxLength,
    disabled,
    onFocus: () => setFocused(true),
    onBlur: commit,
  };
  return (
    <Field label={label} required={required} htmlFor={id} hint={hint}>
      {multiline ? (
        <TextArea {...common} rows={3} onChange={(e) => setDraft(e.target.value)} />
      ) : (
        <TextInput
          {...common}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
            if (e.key === "Escape") {
              setDraft(value ?? "");
              (e.target as HTMLInputElement).blur();
            }
          }}
        />
      )}
    </Field>
  );
}

/** Scene chips; clicking one opens its breakdown (FSD §28.4). */
export function SceneChips({ scenes, empty = "Not used in any scene yet." }: { scenes: ProductionSceneRef[]; empty?: string }) {
  const go = useNav((s) => s.go);
  if (scenes.length === 0) return <div className="sm muted">{empty}</div>;
  return (
    <div className="row wrap">
      {scenes.map((s) => (
        <button
          key={s.sceneId}
          type="button"
          className="chip out"
          title={s.inSource ? s.heading : `${s.heading} — scene removed from current source`}
          style={{ cursor: "pointer" }}
          onClick={() => go({ workspace: "breakdown", params: { sceneId: s.sceneId } })}
        >
          {s.number ? `Scene ${s.number}` : "Removed scene"}
        </button>
      ))}
    </div>
  );
}

/** Ask the OS for image files (validated again in Rust). */
export async function pickImages(multiple: boolean): Promise<string[]> {
  const res = await openDialog({ multiple, directory: false, filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }] });
  if (!res) return [];
  return Array.isArray(res) ? res : [res];
}
