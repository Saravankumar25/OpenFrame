// The Story page (Story > Story tab): one plain writing page for the writer's free text.
// It is not linked to cards, scenes, the screenplay or the assistant. The text lives in the
// project (story.page.get / story.page.save); the only extra is Export (TXT, PDF, DOCX).
//
// Nothing typed is lost: the text is saved 400 ms after the last key (like the screenplay), on
// leaving the box, when the tab or workspace changes (unmount flush), when the project closes and
// when the window closes (all through the shared pending-save registry).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { EmptyState, PageHeader, Skeleton } from "../../design-system";
import { story } from "../../api/story";
import { OpError } from "../../ipc/client";
import { StoryPageExportButton } from "../../features/export/buttons";
import { useDebouncedSave } from "../notes/useDebouncedSave";

const SAVE_DELAY_MS = 400;

type Status = "saved" | "saving" | "failed";
const STATUS_LABEL: Record<Status, string> = { saved: "Saved", saving: "Saving…", failed: "Not saved yet" };

// Saves run one after another, also across a quick leave and return: the page that opens again
// waits for the save of the page that was just closed, so it never shows older text.
let lastSave: Promise<unknown> = Promise.resolve();
const queueSave = <T,>(job: () => Promise<T>): Promise<T> => {
  const run = lastSave.then(job, job);
  lastSave = run.catch(() => undefined);
  return run;
};
async function settledSaves(): Promise<void> {
  let seen: Promise<unknown>;
  do {
    seen = lastSave;
    await seen;
  } while (seen !== lastSave);
}

function wordCount(text: string): number {
  const t = text.trim();
  return t ? t.split(/\s+/).length : 0;
}

export function StoryPage() {
  const [text, setText] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>("saved");
  const rev = useRef(0);
  const latest = useRef("");

  useEffect(() => {
    let live = true;
    void settledSaves()
      .then(() => story.pageGet())
      .then(
        (p) => {
          if (!live) return;
          rev.current = p.rev;
          latest.current = p.text;
          setText(p.text);
        },
        (e: unknown) => live && setLoadError(e instanceof Error ? e.message : "The page could not be read."),
      );
    return () => {
      live = false;
    };
  }, []);

  const persist = useCallback(
    (value: string) =>
      queueSave(async () => {
        try {
          let saved;
          try {
            saved = await story.pageSave(value, rev.current);
          } catch (e) {
            // Another window saved first. Take its revision and keep what the writer typed.
            if (!(e instanceof OpError) || !e.code.startsWith("conflict")) throw e;
            rev.current = (await story.pageGet()).rev;
            saved = await story.pageSave(value, rev.current);
          }
          rev.current = saved.rev;
          if (latest.current === value) setStatus("saved");
        } catch (e) {
          setStatus("failed");
          throw e;
        }
      }),
    [],
  );
  const { schedule, flush } = useDebouncedSave(persist, SAVE_DELAY_MS);

  const words = useMemo(() => wordCount(text ?? ""), [text]);

  if (loadError) {
    return (
      <div className="content fill">
        <PageHeader title="Story" />
        <EmptyState title="The Story page could not be opened.">{loadError}</EmptyState>
      </div>
    );
  }

  return (
    <div className="content fill story-write">
      <PageHeader
        title="Story"
        sub="Write your story freely. This page is not linked to anything else."
        actions={<StoryPageExportButton beforeOpen={flush} />}
      />
      {text === null ? (
        <Skeleton h={28} w={240} />
      ) : (
        <div className="story-write-col">
          <textarea
            className="story-write-area"
            aria-label="Story text"
            placeholder="Start writing your story here."
            spellCheck
            dir="auto"
            autoFocus
            value={text}
            onChange={(e) => {
              const v = e.target.value;
              latest.current = v;
              setText(v);
              setStatus("saving");
              schedule(v);
            }}
            onBlur={() => void flush()}
          />
          <div className="story-write-foot xs muted">
            <span>{words === 1 ? "1 word" : `${words.toLocaleString()} words`}</span>
            <span role="status" aria-live="polite">{STATUS_LABEL[status]}</span>
          </div>
        </div>
      )}
    </div>
  );
}
