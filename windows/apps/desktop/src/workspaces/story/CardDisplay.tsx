// Beat Board, Card Display Settings (Screenplay and Production Depth Specification sections
// 21.2, 21.3 and 52). What a minimized Scene Card shows is the user's own view setting, kept by
// Rust in view state: choosing a preset or ticking a box never changes a card, a scene or the
// screenplay, and never enters undo.

import type { ReactNode } from "react";
import { MessageSquare, Paperclip, Users } from "lucide-react";
import { Banner, Button, Checkbox, Dialog, Skeleton } from "../../design-system";
import { reportError } from "../../ipc/query";
import { story, useCardDisplay } from "../../api/story";
import type { StoryCardDisplayState } from "../../ipc/generated/StoryCardDisplayState";
import type { StoryCardPreset } from "../../ipc/generated/StoryCardPreset";
import type { StoryCardToggles } from "../../ipc/generated/StoryCardToggles";
import type { StorySceneCardDto } from "../../ipc/generated/StorySceneCardDto";
import { useStoryUi } from "./ui";

/** The twelve checkboxes, in the order the specification lists them. */
export const TOGGLES: { key: keyof StoryCardToggles; label: string }[] = [
  { key: "sceneHeading", label: "Scene Heading" },
  { key: "shortDescription", label: "Short Description" },
  { key: "longDescriptionPreview", label: "Long Description Preview" },
  { key: "notes", label: "Notes" },
  { key: "attachments", label: "Attachments" },
  { key: "characterCount", label: "Character Count" },
  { key: "location", label: "Location" },
  { key: "dayNight", label: "Day/Night" },
  { key: "comments", label: "Comments" },
  { key: "revisionChange", label: "Revision/Change" },
  { key: "breakdownStatus", label: "Breakdown Status" },
  { key: "shotStatus", label: "Shot/Storyboard Status" },
];

/** What the compact card showed before Card Display existed; used until the setting loads. */
export const MINIMAL: StoryCardToggles = {
  sceneHeading: false,
  shortDescription: true,
  longDescriptionPreview: false,
  notes: false,
  attachments: false,
  characterCount: false,
  location: false,
  dayNight: false,
  comments: true,
  revisionChange: false,
  breakdownStatus: false,
  shotStatus: false,
};

/** True when the toggles add nothing beyond the compact card, so it keeps its compact layout. */
export function isCompact(t: StoryCardToggles): boolean {
  return TOGGLES.every(({ key }) => t[key] === MINIMAL[key]);
}

function firstLine(text: string | null | undefined, max: number): string {
  const line = (text ?? "").split("\n").map((l) => l.trim()).find(Boolean) ?? "";
  return line.length > max ? `${line.slice(0, max).trimEnd()}…` : line;
}

/** Facts a non-compact card shows under its text, as words (never colour alone). */
export function cardFacts(c: StorySceneCardDto, t: StoryCardToggles): string[] {
  const out: string[] = [];
  const place = [t.location ? c.setName : null, t.dayNight ? c.timeOfDay : null].filter(Boolean).join(" · ");
  if (place) out.push(place);
  const s = c.sceneStatus;
  if (t.revisionChange && s?.revised) out.push("Revised");
  if (t.breakdownStatus && s) {
    out.push(s.breakdownComplete ? "Breakdown done" : s.breakdownItems > 0 ? `Breakdown: ${s.breakdownItems} item${s.breakdownItems === 1 ? "" : "s"}` : "No breakdown yet");
  }
  if (t.shotStatus && s) {
    out.push(`${s.shots} shot${s.shots === 1 ? "" : "s"} · ${s.storyboards} storyboard${s.storyboards === 1 ? "" : "s"}`);
  }
  return out;
}

/** Body of a non-compact Scene Card on the board. */
export function RichCardFace({ card: c, toggles: t, indicators }: { card: StorySceneCardDto; toggles: StoryCardToggles; indicators: ReactNode }) {
  const facts = cardFacts(c, t);
  const long = t.longDescriptionPreview ? firstLine(c.longDescription, 160) : "";
  const note = t.notes ? firstLine(c.notes, 90) : "";
  const counts: ReactNode[] = [];
  if (t.attachments && c.attachments.length > 0)
    counts.push(
      <span key="a" title={`${c.attachments.length} attachment${c.attachments.length === 1 ? "" : "s"}`}>
        <Paperclip size={10} aria-hidden /> {c.attachments.length}
      </span>,
    );
  if (t.characterCount && c.characterIds.length > 0)
    counts.push(
      <span key="c" title={`${c.characterIds.length} character${c.characterIds.length === 1 ? "" : "s"}`}>
        <Users size={10} aria-hidden /> {c.characterIds.length}
      </span>,
    );
  if (t.comments && c.commentCount > 0)
    counts.push(
      <span key="m" title={`${c.commentCount} open comment${c.commentCount === 1 ? "" : "s"}`}>
        <MessageSquare size={10} aria-hidden /> {c.commentCount}
      </span>,
    );
  return (
    <>
      {t.sceneHeading && c.sceneHeading && <div className="sc-head" title={c.sceneHeading}>{c.sceneHeading}</div>}
      {t.shortDescription && (
        <div className="sc-text">{c.shortDescription.trim() ? c.shortDescription : <span className="muted"><i>Blank scene card</i></span>}</div>
      )}
      {long && <div className="sc-long">{long}</div>}
      {note && <div className="sc-note" title={note}>Note: {note}</div>}
      {facts.length > 0 && <div className="sc-facts">{facts.join(" · ")}</div>}
      <span className="ind">
        {indicators}
        {counts}
      </span>
    </>
  );
}

export function CardDisplayDialog() {
  const q = useCardDisplay();
  const close = () => useStoryUi.getState().openDialog(null);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && close()}
      title="Card Display Settings"
      sub="Choose what minimized Scene Cards show. This changes only your view; the story and the screenplay stay exactly as they are."
      footer={<Button onClick={close}>Done</Button>}
    >
      {q.data ? <CardDisplayForm state={q.data} /> : q.isError ? <Banner tone="err">The settings could not be loaded. {q.error?.message}</Banner> : <Skeleton h={160} />}
    </Dialog>
  );
}

function CardDisplayForm({ state }: { state: StoryCardDisplayState }) {
  const { current, presets } = state;
  const save = (preset: StoryCardPreset, toggles?: StoryCardToggles) => void story.setCardDisplay(preset, toggles ?? null).catch(reportError);
  const choices: { preset: StoryCardPreset; label: string; description: string }[] = [
    ...presets.map((p) => ({ preset: p.preset, label: p.label, description: p.description })),
    { preset: "custom", label: "Custom", description: "Any combination of the boxes below." },
  ];
  return (
    <>
      <div className="h4" id="card-preset-label">Preset</div>
      <div role="radiogroup" aria-labelledby="card-preset-label" style={{ marginBottom: 12 }}>
        {choices.map((c) => (
          <label key={c.preset} className="story-radio">
            <input
              type="radio"
              name="card-preset"
              checked={current.preset === c.preset}
              onChange={() => save(c.preset, c.preset === "custom" ? current.toggles : undefined)}
            />
            <span>
              <b>{c.label}</b>
              <span className="muted" style={{ display: "block", fontSize: 12.5 }}>{c.description}</span>
            </span>
          </label>
        ))}
      </div>
      <div className="h4" id="card-toggles-label">Show on minimized cards</div>
      <div role="group" aria-labelledby="card-toggles-label" className="grid g2" style={{ gap: "4px 16px" }}>
        {TOGGLES.map(({ key, label }) => (
          <Checkbox
            key={key}
            checked={current.toggles[key]}
            label={label}
            onChange={(v) => save("custom", { ...current.toggles, [key]: v })}
          />
        ))}
      </div>
      <p className="muted" style={{ fontSize: 12.5, marginBottom: 0 }}>
        Ticking a box switches to Custom. Open a card to read and edit everything it holds.
      </p>
    </>
  );
}
