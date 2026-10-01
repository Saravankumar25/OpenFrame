// The assistant conversation (agentic spec §5, §30, §36, §42; FSD §42; UX §3.40).
//
// - One request runs a bounded, multi-step agent on this computer; the answer
//   shows its sources as chips that open the referenced place in OpenFrame.
// - Exact facts are marked "Exact · from project data"; text the AI wrote is
//   marked "Inferred".
// - Project changes arrive only as a "Proposed Changes" card the user reviews;
//   nothing is applied without [Apply Changes].
// - While the project context is rebuilt, a short notice explains that answers
//   may use fewer sources. No model names or technical details appear here.

import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { call, onAppEvent } from "../ipc/client";
import { reportError, useOp } from "../ipc/query";
import { Banner, Button, Chip, Select, TextArea } from "../design-system";
import { useNav } from "../app/stores";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import type { AiHistoryDto } from "../ipc/generated/AiHistoryDto";
import type { AiExchange } from "../ipc/generated/AiExchange";
import type { AiResultDto } from "../ipc/generated/AiResultDto";
import type { AiScopeKind } from "../ipc/generated/AiScopeKind";
import {
  SCOPE_LABELS,
  UNAVAILABLE_TEXT,
  confidenceLabel,
  defaultScope,
  indexNotice,
  indexStateOf,
  runtimeLabel,
  scopeArgs,
  scopeOptions,
  sourceChips,
  stepsSummary,
  toRoute,
} from "./model";
import { ProposedChanges } from "./ProposedChanges";

const HISTORY_TABLES = ["ai_request", "ai_result", "change_set"];

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

export default function Assistant({ status, hasProject }: { status: AiStatusDto; hasProject: boolean }) {
  const qc = useQueryClient();
  const route = useNav((s) => s.route);
  const go = useNav((s) => s.go);
  const options = useMemo(() => scopeOptions(route), [route]);
  const [scope, setScope] = useState<AiScopeKind>(() => defaultScope(route));
  const [conversation, setConversation] = useState<string | null | undefined>(undefined);
  const [text, setText] = useState("");
  const [pending, setPending] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement>(null);
  const effectiveScope = options.includes(scope) ? scope : defaultScope(route);
  const notice = useIndexNotice();

  const history = useOp<AiHistoryDto>("ai.history", { conversationId: conversation ?? null, limit: 50 }, HISTORY_TABLES, {
    enabled: hasProject && conversation !== null,
  });
  const exchanges = conversation === null ? [] : history.data?.exchanges ?? [];
  const last = exchanges[exchanges.length - 1];

  useEffect(() => {
    endRef.current?.scrollIntoView?.({ block: "end" });
  }, [exchanges.length, pending]);

  const refresh = () => qc.invalidateQueries({ queryKey: ["ai.history"] });

  const send = () => {
    const q = text.trim();
    if (!q || pending) return;
    setPending(q);
    setText("");
    call<AiExchange>("ai.ask", {
      text: q,
      scope: scopeArgs(effectiveScope, route),
      conversationId: conversation === null ? null : (conversation ?? history.data?.conversationId ?? null),
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
      .finally(() => setPending(null));
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  const rl = runtimeLabel(status.runtimeState);
  const usingLabel = last && last.scopeLabel && scopeOfLast(last) === effectiveScope ? last.scopeLabel : `Using: ${SCOPE_LABELS[effectiveScope]}`;

  if (!hasProject) {
    return (
      <div className="body">
        <div className="empty" style={{ height: "auto", padding: 20 }}>
          <h2 style={{ fontSize: 15 }}>Open a project to ask about it</h2>
          <p className="sm">The assistant answers from the project that is open, on this computer.</p>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="body" style={{ paddingBottom: 0 }}>
        <div className="row">
          <label className="xs muted" htmlFor="ai-scope">
            Scope
          </label>
          <div style={{ flex: 1 }}>
            <Select<AiScopeKind>
              id="ai-scope"
              value={effectiveScope}
              onChange={setScope}
              options={options.map((k) => ({ value: k, label: SCOPE_LABELS[k] }))}
            />
          </div>
          <Button size="xs" variant="ghost" onClick={() => setConversation(null)} title="Start a new conversation">
            New
          </Button>
        </div>
        <div className="xs muted">{usingLabel}</div>
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
              Ask about your project — “How many scenes are in the script?”, “Open Scene 12.”, “Create a task to scout the railway station.”. The
              assistant can look things up in several steps; changes are only ever prepared for you to review.
            </div>
          )}
          {exchanges.map((x) => (
            <ExchangeView key={x.requestId} x={x} onChanged={refresh} />
          ))}
          {pending && (
            <>
              <div className="msg u">{pending}</div>
              <div className="msg a muted" role="status">
                Working on it on this computer…
              </div>
            </>
          )}
          <div ref={endRef} />
        </div>
      </div>
      <div className="ai-input">
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
        <Button size="sm" variant="primary" onClick={send} disabled={!text.trim() || !!pending}>
          Send
        </Button>
      </div>
    </>
  );
}

function scopeOfLast(x: AiExchange): AiScopeKind | null {
  for (const [k, label] of Object.entries(SCOPE_LABELS) as [AiScopeKind, string][]) {
    if (x.scopeLabel === `Using: ${label}`) return k;
  }
  return null;
}

function ExchangeView({ x, onChanged }: { x: AiExchange; onChanged: () => void }) {
  return (
    <>
      <div className="msg u">{x.requestText}</div>
      <ResultView r={x.result} onChanged={onChanged} />
    </>
  );
}

export function ResultView({ r, onChanged }: { r: AiResultDto; onChanged: () => void }) {
  const go = useNav((s) => s.go);
  const conf = confidenceLabel(r.confidence);
  const navRoute = toRoute(r.nav);
  const denied = r.kind === "Denied";
  const chips = sourceChips(r.provenance);
  const steps = stepsSummary(r.steps);
  return (
    <>
      <div className={`msg a${r.kind === "Failed" || r.kind === "Unavailable" ? " ai-err" : ""}`} data-kind={r.kind}>
        {denied || r.kind === "Answer" || r.kind === "Proposal" || r.kind === "Suggestion" ? <b>{r.content}</b> : r.content}
        {r.details.length > 0 && (
          <div className="xs muted" style={{ marginTop: 4 }}>
            {r.details.map((d, i) => (
              <div key={i}>{denied && !d.startsWith("Nothing") ? `• ${d}` : d}</div>
            ))}
          </div>
        )}
        {r.items.length > 0 && (
          <ul className="ai-items">
            {r.items.slice(0, 25).map((it, i) => {
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
            {r.items.length > 25 && <li className="muted">…and {r.items.length - 25} more</li>}
          </ul>
        )}
        {(conf || navRoute) && (
          <div className="row" style={{ marginTop: 6, flexWrap: "wrap", gap: 6 }}>
            {conf && <Chip tone={conf.tone}>{conf.text}</Chip>}
            {navRoute && (
              <Button size="xs" onClick={() => go(navRoute)}>
                {r.kind === "Navigate" ? "Open again" : "Open"}
              </Button>
            )}
          </div>
        )}
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
      </div>
      {r.changeSet && <ProposedChanges cs={r.changeSet} onChanged={onChanged} />}
    </>
  );
}
