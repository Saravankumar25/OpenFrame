// New Project dialog (FSD §4.1–4.2; UX §3.3; mocks 008–010).
// Only Title and Type are required. On success the project opens at Project
// Home — there is no setup wizard. On failure nothing is created and the
// entries stay in the form.

import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { useCommand, useOp } from "../../ipc/query";
import type { AppInfo } from "../../ipc/generated/AppInfo";
import type { OpenProjectResult } from "../../ipc/generated/OpenProjectResult";
import type { ProjectType } from "../../ipc/generated/ProjectType";
import { Banner, Button, Dialog, Field, Segmented, TextInput } from "../../design-system";
import { useNav } from "../stores";
import type { CreateProjectArgs } from "../../ipc/generated/CreateProjectArgs";

/** The segmented control groups the two episodic types (mock 008: "Episodic / Series"). */
export type TypeChoice = "Feature Film" | "Short Film" | "Episodic / Series";
export type EpisodicKind = "Episodic" | "Series";

/** Map the dialog's choices to the stored project type (FSD §4.1 has four types). */
export function toProjectType(choice: TypeChoice, episodic: EpisodicKind): ProjectType {
  return choice === "Episodic / Series" ? episodic : choice;
}

/** Returns the message to show for the title, or null when valid. */
export function titleError(title: string): string | null {
  const t = title.trim();
  if (!t) return "A title is required.";
  if (t.length > 200) return "The title is too long (maximum 200 characters).";
  return null;
}

export function NewProjectDialog({ onClose }: { onClose: () => void }) {
  const [title, setTitle] = useState("");
  const [choice, setChoice] = useState<TypeChoice>("Feature Film");
  const [episodic, setEpisodic] = useState<EpisodicKind>("Episodic");
  const [language, setLanguage] = useState("");
  const [genre, setGenre] = useState("");
  const [creator, setCreator] = useState("");
  const [parentDir, setParentDir] = useState<string | null>(null);
  const [touched, setTouched] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const info = useOp<AppInfo>("app.info", {}, []);
  const reset = useNav((s) => s.reset);
  const create = useCommand<CreateProjectArgs, OpenProjectResult>("project.create", {
    silent: true,
    onSuccess: () => {
      reset();
      onClose();
    },
    onError: (e) => setFailed(e.message),
  });
  const error = titleError(title);
  const submit = () => {
    setTouched(true);
    if (error || create.isPending) return;
    setFailed(null);
    create.mutate({
      title: title.trim(),
      projectType: toProjectType(choice, episodic),
      language: language.trim() || null,
      genre: genre.trim() || null,
      creator: creator.trim() || null,
      parentDir,
    });
  };
  const chooseFolder = async () => {
    const dir = await openDialog({ directory: true, title: "Choose where to keep the new project" });
    if (typeof dir === "string") setParentDir(dir);
  };

  return (
    <Dialog
      open
      onOpenChange={(v) => !v && onClose()}
      title="New Project"
      sub="What is this project called and what type is it?"
      size="md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!!error || create.isPending} onClick={submit}>
            {create.isPending ? "Creating…" : failed ? "Retry" : "Create"}
          </Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}
      >
        <Field label="Title" required htmlFor="np-title" error={touched && error ? error : null}>
          <TextInput
            id="np-title"
            autoFocus
            placeholder="e.g. BLACK RAIN"
            value={title}
            maxLength={200}
            invalid={touched && !!error}
            aria-required="true"
            onBlur={() => setTouched(true)}
            onChange={(e) => {
              setTitle(e.target.value);
              setFailed(null);
            }}
          />
        </Field>
        <Field label="Type" required>
          <Segmented
            ariaLabel="Project type"
            value={choice}
            onChange={setChoice}
            options={[
              { value: "Feature Film", label: "Feature Film" },
              { value: "Short Film", label: "Short Film" },
              { value: "Episodic / Series", label: "Episodic / Series" },
            ]}
          />
        </Field>
        {choice === "Episodic / Series" && (
          <fieldset className="field" style={{ border: 0, padding: 0, margin: "0 0 10px" }}>
            <legend className="sm b" style={{ color: "var(--ink2)", marginBottom: 4 }}>Structure</legend>
            <label className="check">
              <input type="radio" name="np-episodic" checked={episodic === "Episodic"} onChange={() => setEpisodic("Episodic")} style={{ accentColor: "var(--accent)" }} />
              <span>Episodic <span className="muted sm">— episodes of one story</span></span>
            </label>
            <label className="check">
              <input type="radio" name="np-episodic" checked={episodic === "Series"} onChange={() => setEpisodic("Series")} style={{ accentColor: "var(--accent)" }} />
              <span>Series <span className="muted sm">— seasons of episodes</span></span>
            </label>
          </fieldset>
        )}
        <details>
          <summary className="h4" style={{ cursor: "pointer", margin: "6px 0 10px" }}>Optional details</summary>
          <div className="grid g3" style={{ gap: 10 }}>
            <Field label="Language" htmlFor="np-lang">
              <TextInput id="np-lang" placeholder="Add later" value={language} maxLength={80} onChange={(e) => setLanguage(e.target.value)} />
            </Field>
            <Field label="Genre" htmlFor="np-genre">
              <TextInput id="np-genre" placeholder="Add later" value={genre} maxLength={80} onChange={(e) => setGenre(e.target.value)} />
            </Field>
            <Field label="Creator" htmlFor="np-creator">
              <TextInput id="np-creator" placeholder="Add later" value={creator} maxLength={200} onChange={(e) => setCreator(e.target.value)} />
            </Field>
          </div>
          <Field label="Saved in">
            <div className="row">
              <span className="sm muted grow selectable-text" style={{ wordBreak: "break-all" }}>{parentDir ?? info.data?.projectsDir ?? "Documents\\OpenFrame\\Projects"}</span>
              <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => void chooseFolder()}>Change…</Button>
            </div>
          </Field>
        </details>
        <div className="hint">You can add or change any of this later. You do not need it to start writing.</div>
        {failed && (
          <div style={{ marginTop: 10 }}>
            <Banner tone="err">
              <b>The project could not be created. Nothing was saved and no partial project was made. Your entries are still here.</b>
              <div className="sm" style={{ marginTop: 2 }}>{failed}</div>
            </Banner>
          </div>
        )}
        <button type="submit" hidden aria-hidden tabIndex={-1} />
      </form>
    </Dialog>
  );
}
