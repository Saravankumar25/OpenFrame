// The Story page (owner request 2026-10-20): a free page left of the Beat Board to write the story
// in prose, like a notepad. It is not linked to the Beat Board, the screenplay or any other page.
// The text is saved a moment after typing stops (story.save_page) and before the page is left;
// Export writes it as plain text, PDF or a Word document (story.export_page).

import { useCallback, useEffect, useRef, useState } from "react";
import { Banner, PageHeader } from "../../design-system";
import { call, OpError } from "../../ipc/client";
import { useOp } from "../../ipc/query";
import { registerPendingSave } from "../../app/pendingSaves";
import { StoryPageExportButton } from "../../features/export/buttons";
import type { StoryPageDto } from "../../ipc/generated/StoryPageDto";

const SAVE_DELAY_MS = 1000;

export function wordCount(text: string): number {
  return text.split(/[^\p{L}\p{N}'’]+/u).filter((w) => /[\p{L}\p{N}]/u.test(w)).length;
}

export function StoryPage() {
  const page = useOp<StoryPageDto>("story.page", {}, ["story_page"]);
  const [text, setText] = useState<string | null>(null);
  const [state, setState] = useState<"saved" | "saving" | "unsaved">("saved");
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const rev = useRef(0);
  const saved = useRef("");
  const latest = useRef("");
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (page.data && text === null) {
      rev.current = page.data.rev;
      saved.current = page.data.body;
      latest.current = page.data.body;
      setText(page.data.body);
    }
  }, [page.data, text]);

  const save = useCallback(async (): Promise<boolean> => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const body = latest.current;
    if (body === saved.current) return true;
    setState("saving");
    try {
      const p = await call<StoryPageDto>("story.save_page", { body, baseRev: rev.current });
      rev.current = p.rev;
      saved.current = p.body;
      setError(null);
      setState(latest.current === body ? "saved" : "unsaved");
      return true;
    } catch (e) {
      if (e instanceof OpError && e.code === "conflict.stale") setConflict(true);
      setError(e instanceof Error ? e.message : String(e));
      setState("unsaved");
      return false;
    }
  }, []);

  useEffect(() => registerPendingSave(save), [save]);
  useEffect(() => () => void save(), [save]);

  const onChange = (v: string) => {
    setText(v);
    latest.current = v;
    if (conflict) return;
    setState(v === saved.current ? "saved" : "unsaved");
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void save(), SAVE_DELAY_MS);
  };

  const reload = async () => {
    try {
      await navigator.clipboard.writeText(latest.current);
    } catch {
      // The clipboard may be unavailable; the reload still goes ahead.
    }
    const p = await call<StoryPageDto>("story.page", {});
    rev.current = p.rev;
    saved.current = p.body;
    latest.current = p.body;
    setText(p.body);
    setConflict(false);
    setError(null);
    setState("saved");
  };

  const words = wordCount(text ?? "");
  const status = `${words === 1 ? "1 word" : `${words} words`} · ${state === "saved" ? "Saved" : state === "saving" ? "Saving" : "Not saved yet"}`;
  return (
    <div className="content story-page">
      <PageHeader
        title="Story"
        sub="A free page to write your story. It is not linked to the Beat Board or the screenplay."
        actions={
          <>
            <span className="xs muted" aria-live="polite">{status}</span>
            <StoryPageExportButton disabled={!(text ?? "").trim()} onBeforeOpen={save} />
          </>
        }
      />
      {conflict && (
        <Banner tone="warn">
          The Story page was changed in another window. <button type="button" className="link" onClick={() => void reload()}>Reload it</button> to see that version; what you typed here is copied to the clipboard first.
        </Banner>
      )}
      {!conflict && error && <Banner tone="err">{error}</Banner>}
      {page.error && <Banner tone="err">{page.error.message}</Banner>}
      <textarea
        className="story-page-text"
        aria-label="Story"
        placeholder="Write your story here."
        value={text ?? ""}
        disabled={text === null || page.data?.canEdit === false}
        onChange={(e) => onChange(e.target.value)}
        onBlur={() => void save()}
        spellCheck
      />
    </div>
  );
}
