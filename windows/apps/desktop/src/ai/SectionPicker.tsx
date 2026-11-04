// Section picker for the Straft AI (spec ai-section-scoped-assistant section 3.8, ADR-0040).
//
// The person picks one to three sections (leaves of the project tree) before asking; nothing is
// picked for them. The tree is a list of checkboxes grouped by branch, opened and closed with one
// button; a chosen section shows as a chip with a remove button, and the sections that ask for one
// more thing (which screenplay, scene, shooting day or call sheet) show that choice under the chip.
// Everything is a native control, so Tab, Space and Enter work and a screen reader names each one.

import { useId, useRef, useState } from "react";
import { X } from "lucide-react";
import { Button, Checkbox, Select } from "../design-system";
import type { AiLeafDto } from "../ipc/generated/AiLeafDto";
import type { AiPickScreenplayDto } from "../ipc/generated/AiPickScreenplayDto";
import type { AiSectionTreeDto } from "../ipc/generated/AiSectionTreeDto";
import { CAP_HINT, MAX_PICKS, type Pick, isPicked, leafById, setInstance, toggle } from "./sections";

interface Props {
  tree: AiSectionTreeDto | undefined;
  picks: Pick[];
  onChange: (picks: Pick[]) => void;
  /** The draft whose scenes are wanted when a Screenplay pick chose a draft other than the current one. */
  onDraft: (draftId: string | null) => void;
  disabled?: boolean;
  /** The sections could not be loaded (and none are shown); `onRetry` asks again. */
  failed?: boolean;
  onRetry?: () => void;
}

export default function SectionPicker({ tree, picks, onChange, onDraft, disabled, failed, onRetry }: Props) {
  const [open, setOpen] = useState(false);
  const panelId = useId();
  const toggleRef = useRef<HTMLButtonElement>(null);
  const full = picks.length >= MAX_PICKS;

  if (!tree) {
    if (failed) {
      return (
        <div className="ai-sections xs" role="alert">
          <span>The sections could not be loaded.</span>
          <div className="row">
            <Button size="xs" onClick={onRetry}>
              Retry
            </Button>
          </div>
        </div>
      );
    }
    return (
      <div className="ai-sections xs muted" role="status">
        Loading the sections…
      </div>
    );
  }

  return (
    <div className="ai-sections">
      <div className="row">
        <Button
          ref={toggleRef}
          size="xs"
          aria-expanded={open}
          aria-controls={open ? panelId : undefined}
          onClick={() => setOpen(!open)}
          disabled={disabled}
        >
          {open ? "Done" : picks.length > 0 ? "Change sections" : "Choose sections"}
        </Button>
        <span className="xs muted">
          {picks.length} of {MAX_PICKS} chosen
        </span>
      </div>

      {open && (
        <div id={panelId} role="group" aria-label="Sections" className="ai-tree">
          {tree.branches.map((b) => (
            <div key={b.id} role="group" aria-label={b.label} className="ai-branch">
              {b.leaves.length > 1 || b.leaves[0]?.label !== b.label ? <div className="xs muted ai-branch-name">{b.label}</div> : null}
              {b.leaves.map((l) => {
                const on = isPicked(picks, l.id);
                return (
                  <div key={l.id} className="ai-leaf">
                    <Checkbox
                      checked={on}
                      disabled={disabled || (!on && full)}
                      onChange={(v) => onChange(toggle(picks, l.id, v))}
                      label={l.label}
                    />
                    {l.note && <div className="xs muted ai-leaf-note">{l.note}</div>}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      )}

      {full && (
        <div className="xs muted" role="status">
          {CAP_HINT}
        </div>
      )}

      {picks.length > 0 && (
        <ul className="ai-picks" aria-label="Chosen sections">
          {picks.map((p) => {
            const leaf = leafById(tree, p.leaf);
            if (!leaf) return null;
            return (
              <li key={p.leaf} className="ai-pick">
                <span className="chip b ai-pick-chip">
                  <span>{leaf.pathLabel}</span>
                  <button
                    type="button"
                    className="ai-pick-x"
                    aria-label={`Remove ${leaf.pathLabel}`}
                    disabled={disabled}
                    onClick={() => {
                      onChange(toggle(picks, p.leaf, false));
                      // The chip with focus is about to go; keep the keyboard where the person is.
                      toggleRef.current?.focus();
                    }}
                  >
                    <X size={12} aria-hidden />
                  </button>
                </span>
                <InstancePickers tree={tree} leaf={leaf} pick={p} picks={picks} onChange={onChange} onDraft={onDraft} disabled={disabled} />
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

// ----------------------------------------------------------------- instance pickers

interface InstanceProps {
  tree: AiSectionTreeDto;
  leaf: AiLeafDto;
  pick: Pick;
  picks: Pick[];
  onChange: (picks: Pick[]) => void;
  onDraft: (draftId: string | null) => void;
  disabled?: boolean;
}

type Opt = { value: string; label: string };

/** A labelled choice with an "All" (or "choose") entry first. */
function Choice({ label, value, first, options, onChange, disabled }: { label: string; value: string; first: Opt; options: Opt[]; onChange: (v: string) => void; disabled?: boolean }) {
  const id = useId();
  return (
    <div className="ai-instance">
      <label className="xs muted" htmlFor={id}>
        {label}
      </label>
      <Select id={id} value={value} onChange={onChange} options={[first, ...options]} disabled={disabled} />
    </div>
  );
}

function scenesOf(tree: AiSectionTreeDto, screenplayId: string | undefined): { label: string; value: string }[] {
  const several = tree.screenplays.length > 1;
  const list: AiPickScreenplayDto[] = screenplayId ? tree.screenplays.filter((s) => s.id === screenplayId) : tree.screenplays;
  return list.flatMap((s) =>
    s.scenes.map((sc) => ({
      value: sc.id,
      label: `${several && !screenplayId ? `${s.title} · ` : ""}Scene ${sc.number}${sc.heading ? `: ${sc.heading}` : ""}`,
    })),
  );
}

function InstancePickers({ tree, leaf, pick, picks, onChange, onDraft, disabled }: InstanceProps) {
  const set = (key: Parameters<typeof setInstance>[2], value: string) => {
    onChange(setInstance(picks, pick.leaf, key, value));
    if (key === "draftId" || key === "screenplayId") onDraft(key === "draftId" && value ? value : null);
  };
  const several = tree.screenplays.length > 1;
  const sp = tree.screenplays.find((s) => s.id === pick.screenplayId) ?? (several ? undefined : tree.screenplays[0]);
  // Scene lists come from the screenplay a Screenplay pick named, else from the project's only one.
  const named = picks.find((p) => leafById(tree, p.leaf)?.picker === "screenplay")?.screenplayId;
  const sceneOptions = scenesOf(tree, leaf.picker === "screenplay" ? sp?.id : named);
  const scene = (
    <Choice label="Scene" value={pick.sceneId ?? ""} first={{ value: "", label: "All scenes" }} options={sceneOptions} onChange={(v) => set("sceneId", v)} disabled={disabled} />
  );
  const day = (
    <Choice
      label="Shooting day"
      value={pick.shootingDayId ?? ""}
      first={{ value: "", label: "All days" }}
      options={tree.days.map((d) => ({ value: d.id, label: d.label }))}
      onChange={(v) => set("shootingDayId", v)}
      disabled={disabled}
    />
  );
  switch (leaf.picker) {
    case "screenplay":
      return (
        <>
          {several && (
            <Choice
              label="Screenplay"
              value={pick.screenplayId ?? ""}
              first={{ value: "", label: "Choose a screenplay" }}
              options={tree.screenplays.map((s) => ({ value: s.id, label: s.title }))}
              onChange={(v) => set("screenplayId", v)}
              disabled={disabled}
            />
          )}
          {sp && sp.drafts.length > 1 && (
            <Choice
              label="Draft"
              value={pick.draftId ?? ""}
              first={{ value: "", label: "Current draft" }}
              options={sp.drafts.map((d) => ({ value: d.id, label: d.name }))}
              onChange={(v) => set("draftId", v)}
              disabled={disabled}
            />
          )}
          {sp && scene}
        </>
      );
    case "scene":
      return scene;
    case "shootingDay":
      return day;
    case "dayAndScene":
      return (
        <>
          {day}
          {scene}
        </>
      );
    case "callSheet":
      return (
        <Choice
          label="Call sheet"
          value={pick.callSheetId ?? ""}
          first={{ value: "", label: "All call sheets" }}
          options={tree.callSheets.map((c) => ({ value: c.id, label: c.title }))}
          onChange={(v) => set("callSheetId", v)}
          disabled={disabled}
        />
      );
    default:
      return null;
  }
}

