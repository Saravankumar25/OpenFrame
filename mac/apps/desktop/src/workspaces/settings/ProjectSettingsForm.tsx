// Project settings form (UX §3.4 "Project settings", mock 022). Used by the
// Project settings dialog on Project Home and by the Project settings page.
// Title and Type are required; everything else is optional. Only the Owner
// can change settings (the backend enforces ManageProject).

import { useState } from "react";
import { call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import type { ProjectSettings } from "../../ipc/generated/ProjectSettings";
import type { ProjectStatus } from "../../ipc/generated/ProjectStatus";
import type { ProjectType } from "../../ipc/generated/ProjectType";
import { Button, Dialog, Field, Select, Skeleton, TextArea, TextInput } from "../../design-system";
import { toast } from "../../app/toast";
import { PROJECT_STATUSES } from "../project-home/StatusMenu";

const TYPES: { value: ProjectType; label: string }[] = [
  { value: "Feature Film", label: "Feature Film" },
  { value: "Short Film", label: "Short Film" },
  { value: "Episodic", label: "Episodic" },
  { value: "Series", label: "Series" },
];

interface Draft {
  title: string;
  projectType: ProjectType;
  language: string;
  genre: string;
  creator: string;
  logline: string;
  status: ProjectStatus;
  projectNotes: string;
}

function draftFrom(s: ProjectSettings): Draft {
  return {
    title: s.project.title,
    projectType: s.project.projectType,
    language: s.project.language ?? "",
    genre: s.project.genre ?? "",
    creator: s.project.creator ?? "",
    logline: s.project.logline ?? "",
    status: (PROJECT_STATUSES.includes(s.project.status as ProjectStatus) ? s.project.status : "Idea") as ProjectStatus,
    projectNotes: s.projectNotes ?? "",
  };
}

const isEpisodic = (t: ProjectType) => t === "Episodic" || t === "Series";

/** Loads current settings and renders the form once they are available. */
export function ProjectSettingsForm({ formId, onSaved, onDirtyChange }: { formId: string; onSaved?: () => void; onDirtyChange?: (dirty: boolean) => void }) {
  const settings = useOp<ProjectSettings>("project.get_settings", {}, ["project"], { refetchOnMount: "always" });
  if (!settings.data) {
    return (
      <div className="col" aria-busy="true">
        <Skeleton h={32} />
        <Skeleton h={32} />
        <Skeleton h={70} />
      </div>
    );
  }
  return <Form key={`${settings.data.project.rev}`} formId={formId} initial={draftFrom(settings.data)} onSaved={onSaved} onDirtyChange={onDirtyChange} />;
}

function Form({ formId, initial, onSaved, onDirtyChange }: { formId: string; initial: Draft; onSaved?: () => void; onDirtyChange?: (dirty: boolean) => void }) {
  const [d, setD] = useState<Draft>(initial);
  const [saving, setSaving] = useState(false);
  const titleMissing = d.title.trim().length === 0;
  const set = <K extends keyof Draft>(k: K, v: Draft[K]) => {
    const next = { ...d, [k]: v };
    setD(next);
    onDirtyChange?.(JSON.stringify(next) !== JSON.stringify(initial));
  };
  const save = async () => {
    if (titleMissing || saving) return;
    setSaving(true);
    try {
      await call("project.update_settings", {
        title: d.title.trim(),
        projectType: d.projectType,
        language: d.language,
        genre: d.genre,
        creator: d.creator,
        logline: d.logline,
        settings: { projectNotes: d.projectNotes.trim() ? d.projectNotes : null },
      });
      if (d.status !== initial.status) await call("project.set_status", { status: d.status });
      toast.undoable("Updated project settings");
      onDirtyChange?.(false);
      onSaved?.();
    } catch (e) {
      reportError(e);
    } finally {
      setSaving(false);
    }
  };
  return (
    <form
      id={formId}
      aria-busy={saving}
      onSubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <div className="grid g2" style={{ gap: "0 12px" }}>
        <Field label="Title" required htmlFor={`${formId}-title`} error={titleMissing ? "A title is required." : null}>
          <TextInput id={`${formId}-title`} value={d.title} maxLength={200} invalid={titleMissing} onChange={(e) => set("title", e.target.value)} />
        </Field>
        <Field label="Type" required htmlFor={`${formId}-type`} hint={isEpisodic(d.projectType) !== isEpisodic(initial.projectType) ? "Changing the type never deletes anything you have written." : undefined}>
          <Select id={`${formId}-type`} value={d.projectType} onChange={(v) => set("projectType", v)} options={TYPES} />
        </Field>
        <Field label="Language" htmlFor={`${formId}-lang`}>
          <TextInput id={`${formId}-lang`} value={d.language} maxLength={80} placeholder="Add later" onChange={(e) => set("language", e.target.value)} />
        </Field>
        <Field label="Genre" htmlFor={`${formId}-genre`}>
          <TextInput id={`${formId}-genre`} value={d.genre} maxLength={80} placeholder="Add later" onChange={(e) => set("genre", e.target.value)} />
        </Field>
        <Field label="Creator" htmlFor={`${formId}-creator`}>
          <TextInput id={`${formId}-creator`} value={d.creator} maxLength={200} placeholder="Add later" onChange={(e) => set("creator", e.target.value)} />
        </Field>
        <Field label="Status" htmlFor={`${formId}-status`}>
          <Select id={`${formId}-status`} value={d.status} onChange={(v) => set("status", v)} options={PROJECT_STATUSES.map((s) => ({ value: s, label: s }))} />
        </Field>
      </div>
      <Field label="Logline" htmlFor={`${formId}-logline`}>
        <TextArea id={`${formId}-logline`} rows={2} maxLength={1000} value={d.logline} placeholder="One or two sentences about the film (optional)" onChange={(e) => set("logline", e.target.value)} />
      </Field>
      <Field label="Project notes" htmlFor={`${formId}-notes`}>
        <TextArea id={`${formId}-notes`} rows={3} maxLength={20000} value={d.projectNotes} placeholder="Shoot window, budget limits, anything to remember about this project" onChange={(e) => set("projectNotes", e.target.value)} />
      </Field>
      <div className="hint">Everything here is optional except title and type. Language, genre and creator are reused on exported documents.</div>
    </form>
  );
}

/** Mock 022. */
export function ProjectSettingsDialog({ onClose }: { onClose: () => void }) {
  const [dirty, setDirty] = useState(false);
  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="Project settings"
      sub="Project identity fields."
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" type="submit" form="project-settings-dialog" disabled={!dirty}>Save</Button>
        </>
      }
    >
      <ProjectSettingsForm formId="project-settings-dialog" onSaved={onClose} onDirtyChange={setDirty} />
    </Dialog>
  );
}
