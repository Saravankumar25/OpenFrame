// Straft AI conversation (agentic spec §5, §30, §36, §42; FSD §42; UX §3.40).
//
// - One request runs a bounded, multi-step agent on this computer; the answer is
//   plain conversational text. Sources, steps, how sure the answer is, and any list that only
//   backs the text wait behind "See sources" (StraftAI.docx section 4.1). A list the person
//   asked for stays under the answer (the core says so with itemsAreAnswer); long lists show a
//   preview and "Show all".
// - Project changes arrive only as a "Proposed Changes" card the user reviews;
//   nothing is applied without [Apply Changes].
// - While the project context is rebuilt, a short notice explains that answers
//   may use fewer sources. No model names or technical details appear here.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Download } from "lucide-react";
import { OpError, call, onAppEvent } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button, Chip, IconButton, TextArea } from "../design-system";
import { useNav } from "../app/stores";
import { toast } from "../app/toast";
import type { AiCancelDto } from "../ipc/generated/AiCancelDto";
import type { AiReportResult } from "../ipc/generated/AiReportResult";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiHistoryDto } from "../ipc/generated/AiHistoryDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiResultDto } from "../ipc/generated/AiResultDto";
import type { AiSectionTreeDto } from "../ipc/generated/AiSectionTreeDto";
import {
  UNAVAILABLE_TEXT,
  confidenceLabel,
  indexNotice,
  indexStateOf,
  runtimeLabel,
  sourceChips,
  stepsSummary,
  toRoute,
} from "./model";
import { ProposedChanges } from "./ProposedChanges";
import { AnswerText } from "./answerText";
import SectionPicker from "./SectionPicker";
import { MAX_PICKS, askSections, draftOfPicks, isPicked, leafById, pruneToTree, sendHint, toggle, useAiPicks } from "./sections";

const HISTORY_TABLES = ["ai_request", "ai_result", "change_set"];
/** What the section pickers list: the screenplays, drafts and scenes, the shooting days and the call sheets. */
const TREE_TABLES = ["project", "screenplay", "screenplay_draft", "screenplay_scene", "shooting_day", "call_sheet"];

/** The line shown while a request runs: what Straft AI is doing now. */
export interface Progress {
  label: string;
  step: number;
  total: number;
}

/** A new conversation id made here, so a running request can be stopped before its answer exists.
 * The core accepts letters, digits and hyphens (up to 64). */
export function newConversationId(): string {
  const c = globalThis.crypto as Crypto | undefined;
  if (c && typeof c.randomUUID === "function") return c.randomUUID();
  return Array.from({ length: 4 }, () => Math.random().toString(16).slice(2, 10).padEnd(8, "0")).join("-");
}

/** The latest progress line of the request running in `conversation` (null when none arrived yet). */
export function useProgress(conversation: string | null): Progress | null {
  const [progress, setProgress] = useState<Progress | null>(null);
  useEffect(() => {
    setProgress(null);
    if (!conversation) return;
    const un = onAppEvent((ev) => {
      if (ev.type !== "module" || ev.module !== "ai" || ev.name !== "progress") return;
      const p = ev.payload as { conversationId?: unknown; label?: unknown; step?: unknown; total?: unknown } | null;
      if (!p || p.conversationId !== conversation || typeof p.label !== "string" || p.label === "") return;
      setProgress({ label: p.label, step: Number(p.step) || 0, total: Number(p.total) || 0 });
    });
    return () => {
      void un.then((f) => f());
    };
  }, [conversation]);
  return progress;
}

/** Project-context state; quiet (no notice) when the index isn't available in this build. */
function useIndexNotice() {
  const qc = useQueryClient();
  const q = useOp<unknown>("ai.index_status", {}, [], {
    retry: false,
    staleTime: 0,
    refetchInterval: (query) => (indexNotice(indexStateOf(query.state.data)) ? 3_000 : false),
  });
  useEffect(() => {
    const un = onAppEvent((ev) => {
      if (ev.type === "module" && ev.module === "ai") void qc.invalidateQueries({ queryKey: ["ai.index_status"] });
    });
    return () => {
      void un.then((f) => f());
    };
  }, [qc]);
  return q.isError ? null : indexNotice(indexStateOf(q.data));
}

export default function Assistant({ status, hasProject, projectId }: { status: AiStatusDto; hasProject: boolean; projectId?: string | null }) {
  const qc = useQueryClient();
  const go = useNav((s) => s.go);
  const picks = useAiPicks((s) => s.picks);
  const setPicks = useAiPicks((s) => s.set);
  const resetPicks = useAiPicks((s) => s.reset);
  const bindProject = useAiPicks((s) => s.bind);
  // The draft whose scenes the pickers list, when a Screenplay pick chose one other than the current.
  const [sceneDraft, setSceneDraft] = useState<string | null>(null);
  const treeQuery = useOp<AiSectionTreeDto>("ai.sections", { draftId: sceneDraft }, TREE_TABLES, {
    enabled: hasProject,
    placeholderData: (prev) => prev,
  });
  const tree = treeQuery.data;
  const [conversation, setConversation] = useState<string | null | undefined>(undefined);
  const [text, setText] = useState("");
  const [pending, setPending] = useState<string | null>(null);
  // The conversation the running request belongs to, and whether Cancel was pressed for it.
  const [running, setRunning] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const progress = useProgress(running);
  const endRef = useRef<HTMLDivElement>(null);
  const notice = useIndexNotice();
  const hint = sendHint(tree, picks);

  // Picks made in another project mean nothing here: another project empties them.
  useEffect(() => bindProject(projectId ?? null), [projectId, bindProject]);
  // A section or choice the project no longer offers (the person's role changed, a series became a film, a
  // screenplay was deleted) leaves the selection. The stale tree shown while a new one loads is not judged.
  const arrivedTree = treeQuery.isPlaceholderData ? undefined : tree;
  useEffect(() => {
    const kept = pruneToTree(arrivedTree, picks);
    if (kept !== picks) setPicks(kept);
  }, [arrivedTree, picks, setPicks]);
  useEffect(() => {
    setSceneDraft(draftOfPicks(tree, picks));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [picks.map((p) => p.draftId ?? "").join(",")]);

  const history = useOp<AiHistoryDto>("ai.history", { conversationId: conversation ?? null, limit: 50 }, HISTORY_TABLES, {
    enabled: hasProject && conversation !== null,
  });
  const exchanges = conversation === null ? [] : history.data?.exchanges ?? [];

  useEffect(() => {
    endRef.current?.scrollIntoView?.({ block: "end" });
  }, [exchanges.length, pending]);

  const refresh = () => qc.invalidateQueries({ queryKey: ["ai.history"] });

  // "Add <section>" on an answer that pointed elsewhere: the section joins the selection and the
  // person types the question again.
  const inputRef = useRef<HTMLDivElement>(null);
  const canAdd = (leaf: string) => picks.length < MAX_PICKS && !isPicked(picks, leaf) && !!leafById(tree, leaf);
  const addSection = (leaf: string) => {
    setPicks(toggle(picks, leaf, true));
    inputRef.current?.querySelector("textarea")?.focus();
  };

  // Only the person's own click writes anything: the file goes where the Save dialog says and nowhere else.
  const conversationId = exchanges[0]?.conversationId ?? null;
  const saveForReview = async () => {
    if (!conversationId) return;
    let path: string | null;
    try {
      path = await saveDialog({
        title: "Save conversation for review",
        defaultPath: "straft-ai-conversation.json",
        filters: [{ name: "JSON file", extensions: ["json"] }],
      });
    } catch (e) {
      reportError(e);
      return;
    }
    if (!path) return;
    try {
      const r = await call<AiReportResult>("ai.export_conversation", { conversationId, path });
      toast.success(`Saved ${r.fileName} (${r.rows} ${r.rows === 1 ? "question" : "questions"}).`);
    } catch (e) {
      if (e instanceof OpError) toast.error(e.message);
      else reportError(e);
    }
  };

  // Stop the running request. Nothing was changed by it; the answer says so.
  const cancel = () => {
    if (!running || stopping) return;
    setStopping(true);
    call<AiCancelDto>("ai.cancel", { conversationId: running })
      // Nothing was running to stop (the request was not registered yet, or just ended): try again.
      .then((r) => {
        if (!r.stopping) setStopping(false);
      })
      .catch((e) => {
        setStopping(false);
        reportError(e);
      });
  };

  const send = () => {
    const q = text.trim();
    if (!q || pending || hint) return;
    // A new conversation gets its id here, so Cancel can name it while the first answer is awaited.
    const conversationForAsk =
      (conversation === null ? null : (conversation ?? history.data?.conversationId ?? null)) ?? newConversationId();
    setPending(q);
    setRunning(conversationForAsk);
    setStopping(false);
    setText("");
    call<AiExchange>("ai.ask", {
      text: q,
      sections: askSections(picks),
      conversationId: conversationForAsk,
    })
      .then((x) => {
        setConversation(x.conversationId);
        void refresh();
        // Navigation is application control, not a project change: it runs directly (AI spec §5.3).
        if (x.result.kind === "Navigate") {
          const r = toRoute(x.result.nav);
          if (r) go(r);
        }
      })
      .catch((e) => {
        setText(q);
        reportError(e);
      })
      .finally(() => {
        setPending(null);
        setRunning(null);
        setStopping(false);
      });
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  const rl = runtimeLabel(status.runtimeState);

  if (!hasProject) {
    return (
      <div className="body">
        <div className="empty" style={{ height: "auto", padding: 20 }}>
          <h2 style={{ fontSize: 15 }}>Open a project to ask about it</h2>
          <p className="sm">Straft AI answers from the project that is open, on this computer.</p>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="body" style={{ paddingBottom: 0 }}>
        <div className="row">
          <span className="xs muted grow">Sections to read</span>
          <IconButton label="Save conversation for review" disabled={!conversationId} onClick={() => void saveForReview()}>
            <Download size={14} />
          </IconButton>
          <Button
            size="xs"
            variant="ghost"
            disabled={!!pending}
            onClick={() => {
              setConversation(null);
              resetPicks();
            }}
            title="Start a new conversation"
          >
            New
          </Button>
        </div>
        <SectionPicker
          tree={tree}
          picks={picks}
          onChange={setPicks}
          onDraft={setSceneDraft}
          disabled={!!pending}
          failed={treeQuery.isError}
          onRetry={() => void treeQuery.refetch()}
        />
        {status.runtimeState === "Failed" && (
          <Banner tone="warn">{status.runtimeMessage ?? UNAVAILABLE_TEXT} Ask again to restart it.</Banner>
        )}
        {rl && <div className="xs muted" role="status">{rl}</div>}
        {notice && (
          <div className="ai-notice" role="status">
            <b>{notice.preparing}</b> {notice.degraded}
          </div>
        )}
        <div className="ai-conv" aria-live="polite">
          {exchanges.length === 0 && !pending && (
            <div className="xs muted" style={{ padding: "8px 0" }}>
              Ask about your project, for example “How many scenes are in the script?”, “Open Scene 12.” or “Create a task to scout the
              railway station.” Answers come from your project and say what they are based on; changes are only ever prepared for you
              to review.
            </div>
          )}
          {exchanges.map((x) => (
            <ExchangeView key={x.requestId} x={x} onChanged={refresh} canAdd={canAdd} onAdd={addSection} />
          ))}
          {pending && (
            <>
              <div className="msg u">{pending}</div>
              <div className="msg a muted" role="status" aria-live="polite">
                <div className="ai-progress">
                  <span className="grow">{stopping ? "Stopping…" : (progress?.label ?? "Working on it on this computer…")}</span>
                  <Button size="xs" variant="ghost" onClick={cancel} disabled={stopping}>
                    Cancel
                  </Button>
                </div>
              </div>
            </>
          )}
          <div ref={endRef} />
        </div>
      </div>
      <div className="ai-input" ref={inputRef}>
        <TextArea
          aria-label="Ask or command"
          placeholder="Ask or command… e.g. “Open Scene 42.”"
          rows={2}
          maxLength={2000}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={onKey}
          disabled={!!pending}
        />
        <div className="ai-send">
          <Button size="sm" variant="primary" onClick={send} disabled={!text.trim() || !!pending || !!hint} aria-describedby={hint ? "ai-send-hint" : undefined}>
            Send
          </Button>
          {hint && (
            <div id="ai-send-hint" className="xs muted" role="status">
              {hint}
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function ExchangeView({
  x,
  onChanged,
  canAdd,
  onAdd,
}: {
  x: AiExchange;
  onChanged: () => void;
  canAdd: (leaf: string) => boolean;
  onAdd: (leaf: string) => void;
}) {
  return (
    <>
      <div className="msg u">{x.requestText}</div>
      <ResultView r={x.result} onChanged={onChanged} using={x.scopeLabel} canAdd={canAdd} onAdd={onAdd} />
    </>
  );
}

/** Items shown before "Show all" (a long answer is a preview plus the full list, never "and N more"). */
const PREVIEW_ITEMS = 8;

/** The result's rows: eight at first, the rest behind "Show all". */
function ItemList({ r }: { r: AiResultDto }) {
  const go = useNav((s) => s.go);
  const [allItems, setAllItems] = useState(false);
  const items = allItems ? r.items : r.items.slice(0, PREVIEW_ITEMS);
  return (
    <>
      <ul className="ai-items">
        {items.map((it, i) => {
          const to = toRoute(it.nav);
          return (
            <li key={i}>
              {r.kind === "Suggestion" && <Chip tone="y">Suggested</Chip>}
              <span className="grow">
                {it.label}
                {it.detail && <span className="muted"> · {it.detail}</span>}
              </span>
              {to && (
                <Button size="xs" onClick={() => go(to)}>
                  Show
                </Button>
              )}
            </li>
          );
        })}
      </ul>
      {r.items.length > PREVIEW_ITEMS && (
        <Button size="xs" variant="ghost" onClick={() => setAllItems(!allItems)} aria-expanded={allItems}>
          {allItems ? "Show fewer" : `Show all ${r.items.length}`}
        </Button>
      )}
    </>
  );
}

/** `using` is the line that says which sections the answer was read from ("Using: Screenplay + Cast & Crew").
 * When the answer pointed to a section that was not picked, `canAdd` says whether there is room and `onAdd`
 * adds it; with three sections picked the sentence already says to remove one, so there is no button. */
export function ResultView({
  r,
  onChanged,
  using,
  canAdd,
  onAdd,
}: {
  r: AiResultDto;
  onChanged: () => void;
  using?: string | null;
  canAdd?: (leaf: string) => boolean;
  onAdd?: (leaf: string) => void;
}) {
  const go = useNav((s) => s.go);
  const conf = confidenceLabel(r.confidence);
  const navRoute = toRoute(r.nav);
  const denied = r.kind === "Denied";
  const chips = sourceChips(r.provenance);
  const steps = stepsSummary(r.steps);
  // A refusal, failure or question needs its reasons in view; for an ordinary answer they are
  // technical detail and wait behind "See sources" (StraftAI.docx section 4.1).
  const reasonsVisible = denied || r.kind === "Failed" || r.kind === "Unavailable" || r.kind === "Clarify" || r.kind === "Private";
  // Rows are the answer only when the person asked for a list; otherwise they back the text.
  const hasItems = r.items.length > 0;
  const itemsInline = hasItems && r.itemsAreAnswer;
  const itemsAsSources = hasItems && !r.itemsAreAnswer;
  const hasSources = (!reasonsVisible && r.details.length > 0) || !!conf || chips.length > 0 || !!steps || itemsAsSources;
  return (
    <>
      <div className={`msg a${r.kind === "Failed" || r.kind === "Unavailable" ? " ai-err" : ""}`} data-kind={r.kind}>
        <AnswerText text={r.content} />

        {reasonsVisible && r.details.length > 0 && (
          <div className="xs muted" style={{ marginTop: 4 }}>
            {r.details.map((d, i) => (
              <div key={i}>{denied && !d.startsWith("Nothing") ? `• ${d}` : d}</div>
            ))}
          </div>
        )}
        {itemsInline && <ItemList r={r} />}
        {r.pointer && onAdd && canAdd?.(r.pointer.leaf) && (
          <div className="row" style={{ marginTop: 6 }}>
            <Button size="xs" onClick={() => onAdd(r.pointer!.leaf)}>
              Add {r.pointer.label}
            </Button>
          </div>
        )}
        {navRoute && (
          <div className="row" style={{ marginTop: 6 }}>
            <Button size="xs" onClick={() => go(navRoute)}>
              {r.kind === "Navigate" ? "Open again" : "Open"}
            </Button>
          </div>
        )}
        {hasSources && (
          <details className="ai-details">
            <summary className="btn ghost xs">See sources</summary>
            {!reasonsVisible &&
              r.details.map((d, i) => (
                <div key={i} className="xs muted">
                  {d}
                </div>
              ))}
            {conf && <div className="xs muted">{conf.text}</div>}
            {itemsAsSources && <ItemList r={r} />}
            {chips.length > 0 && (
              <div className="ai-sources" aria-label="Sources">
                <span className="xs muted">Based on:</span>
                {chips.map((c) =>
                  c.route ? (
                    <button key={c.key} type="button" className="chip b ai-src" title="Open" onClick={() => go(c.route!)}>
                      {c.text}
                    </button>
                  ) : (
                    <Chip key={c.key}>{c.text}</Chip>
                  ),
                )}
              </div>
            )}
            {steps && <div className="xs muted ai-steps">{steps}</div>}
          </details>
        )}
        {using && <div className="xs muted ai-using">{using}</div>}
      </div>
      {r.changeSet && <ProposedChanges cs={r.changeSet} onChanged={onChanged} />}
    </>
  );
}
