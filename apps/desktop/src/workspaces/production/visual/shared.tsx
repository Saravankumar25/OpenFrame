// Shared pieces of the visual planning tabs (Moodboards, Storyboards, Shot Lists):
// remembered view selection, scene review indicators and the no-script state.

import { FileText } from "lucide-react";
import { create } from "zustand";
import { useEffect, useState } from "react";
import { Banner, Button, EmptyState, Field, TextInput } from "../../../design-system";
import { useCommand } from "../../../ipc/query";
import { useNav } from "../../../app/stores";
import { toast } from "../../../app/toast";
import { formatWhen } from "../../../api/visual";
import type { VisualScene } from "../../../ipc/generated/VisualScene";
import type { MarkSceneReviewedArgs } from "../../../ipc/generated/MarkSceneReviewedArgs";

/** View state only (which scene/board is open). Project truth stays in Rust. */
interface VisualView {
  /** Shot list: scene lineage id, or "all" for the full project list. */
  shotScene: string | null;
  /** Storyboards: "scene:<lineage>" or "board:<id>". */
  boardKey: string | null;
  /** Storyboards: chosen board when a scene has several. */
  sceneBoard: Record<string, string>;
  moodboardId: string | null;
  showRemoved: boolean;
  set: (patch: Partial<Omit<VisualView, "set">>) => void;
}

export const useVisualView = create<VisualView>((set) => ({
  shotScene: null,
  boardKey: null,
  sceneBoard: {},
  moodboardId: null,
  showRemoved: false,
  set: (patch) => set(patch),
}));

/** "Scene 12 changed since planning" (FSD §55, §125: what, when, action). */
export function SceneReviewBanner({ scene, what }: { scene: VisualScene; what: string }) {
  const mark = useCommand<MarkSceneReviewedArgs>("visual.mark_scene_reviewed", {
    onSuccess: () => toast.undoable(`Marked Scene ${scene.number} as reviewed`),
  });
  if (!scene.needsReview) return null;
  const when = formatWhen(scene.changedAt);
  return (
    <Banner
      tone="warn"
      actions={
        <Button size="sm" disabled={mark.isPending} onClick={() => mark.mutate({ sceneLineageId: scene.lineageId })}>
          Mark reviewed
        </Button>
      }
    >
      <b>Scene {scene.number} changed since planning{when ? ` (${when})` : ""}.</b> Your {what} are kept — check them against the
      script, then mark the scene reviewed.
    </Banner>
  );
}

export function RemovedSceneBanner({ heading, what }: { heading: string; what: string }) {
  return (
    <Banner tone="info">
      <b>Scene removed from the current script{heading ? ` — ${heading}` : ""}.</b> Its {what} are kept for reference and hidden from
      active planning.
    </Banner>
  );
}

/** Shown when the project has no screenplay scenes to plan against. */
export function NoScriptState({ what }: { what: string }) {
  const go = useNav((s) => s.go);
  return (
    <EmptyState
      icon={<FileText size={34} />}
      title="No screenplay scenes yet"
      actions={
        <Button variant="primary" onClick={() => go({ workspace: "screenplay" })}>
          Open Screenplay
        </Button>
      }
    >
      {what} are planned scene by scene. Write or import a screenplay first — scene headings come from the script, so you never
      type them twice.
    </EmptyState>
  );
}

/** Free text with common suggestions (no forced taxonomy — FSD §103). Commits on blur/Enter. */
export function SuggestField({
  label,
  value,
  options,
  placeholder,
  onCommit,
}: {
  label: string;
  value: string | null;
  options?: string[];
  placeholder?: string;
  onCommit: (v: string) => void;
}) {
  const [text, setText] = useState(value ?? "");
  useEffect(() => setText(value ?? ""), [value]);
  const id = `f-${label.replace(/\W+/g, "-").toLowerCase()}`;
  const commit = () => {
    if (text.trim() !== (value ?? "")) onCommit(text.trim());
  };
  return (
    <Field label={label} htmlFor={id}>
      <TextInput
        id={id}
        list={options ? `${id}-list` : undefined}
        value={text}
        placeholder={placeholder}
        onChange={(e) => setText(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            commit();
          }
        }}
      />
      {options && (
        <datalist id={`${id}-list`}>
          {options.map((o) => (
            <option key={o} value={o} />
          ))}
        </datalist>
      )}
    </Field>
  );
}
