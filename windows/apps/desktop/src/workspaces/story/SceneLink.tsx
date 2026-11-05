// Link a Scene Card to a screenplay scene that already exists: written by hand or imported
// (Final Draft, Fountain, PDF and so on). The card keeps a reference only; nothing is copied.

import { useState } from "react";
import { Button, Select } from "../../design-system";
import { reportError } from "../../ipc/query";
import { useNav } from "../../app/stores";
import { story, useLinkableScenes } from "../../api/story";
import type { StoryCardDetail } from "../../ipc/generated/StoryCardDetail";

export function SceneLink({ cardId, linked, readOnly }: { cardId: string; linked: StoryCardDetail["linkedScene"]; readOnly: boolean }) {
  const go = useNav((s) => s.go);
  const [choosing, setChoosing] = useState(false);
  const [picked, setPicked] = useState("");
  const scenes = useLinkableScenes(choosing || !linked);
  const options = [
    { value: "", label: scenes.isLoading ? "Loading scenes…" : "Choose a scene…" },
    ...(scenes.data ?? []).map((s) => ({ value: s.sceneId, label: `${s.screenplayTitle} · Scene ${s.label}: ${s.heading || "Untitled"}` })),
  ];
  const link = (sceneId: string | null) =>
    story.linkScene(cardId, sceneId).then(
      () => {
        setChoosing(false);
        setPicked("");
      },
      (e) => reportError(e),
    );
  const picker = (
    <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
      <Select value={picked} onChange={setPicked} options={options} ariaLabel="Screenplay scene" disabled={readOnly} />
      <Button size="xs" variant="primary" disabled={readOnly || !picked} onClick={() => void link(picked)}>Link</Button>
      {linked && <Button size="xs" onClick={() => setChoosing(false)}>Cancel</Button>}
    </div>
  );
  if (!linked) {
    if (scenes.data && scenes.data.length === 0) return <div className="muted" style={{ fontSize: 12.5 }}>No screenplay scenes to link yet. Write or import a screenplay first.</div>;
    return (
      <div className="banner info" role="status" style={{ flexDirection: "column", alignItems: "stretch", gap: 8 }}>
        <div>Not linked to a screenplay scene. Link this card to a scene you wrote or imported.</div>
        {picker}
      </div>
    );
  }
  return (
    <div className="banner info" role="status" style={{ flexDirection: "column", alignItems: "stretch", gap: 8 }}>
      <div>
        Linked to screenplay <b>Scene {linked.number}: {linked.heading}</b> in “{linked.screenplayTitle}”, {linked.draftName}. Changes here don't change the screenplay.
      </div>
      {choosing ? (
        picker
      ) : (
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <Button size="xs" onClick={() => go({ workspace: "screenplay", params: { sceneId: linked.sceneId, draftId: linked.draftId, screenplayId: linked.screenplayId } })}>
            Open Screenplay
          </Button>
          <Button size="xs" disabled={readOnly} onClick={() => setChoosing(true)}>Change</Button>
          <Button size="xs" disabled={readOnly} onClick={() => void link(null)}>Unlink</Button>
        </div>
      )}
    </div>
  );
}
