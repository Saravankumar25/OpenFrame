// Settings > Offline AI > Assistant memory (spec ai-conversational-harness, section 5.7; slice S6).
// What the assistant remembers about the person in this project: facts and preferences they stated
// outright, or answers they confirmed. Kept on this computer, never shared, each entry deletable.

import { useQueryClient } from "@tanstack/react-query";
import { OpError, call } from "../../ipc/client";
import { reportError, useOp } from "../../ipc/query";
import { Button, Skeleton } from "../../design-system";
import { toast } from "../../app/toast";
import { dayAndTime } from "../../app/home/format";
import type { AiMemoryListDto } from "../../ipc/generated/AiMemoryListDto";
import type { AiMemoryEntry } from "../../ipc/generated/AiMemoryEntry";

export function AssistantMemoryCard() {
  const qc = useQueryClient();
  const q = useOp<AiMemoryListDto>("ai.memory_list", {}, ["ai_memory"], { staleTime: 0 });
  const entries = q.data?.entries;

  const forget = (e: AiMemoryEntry) => {
    void call("ai.memory_delete", { id: e.id })
      .then(() => {
        void qc.invalidateQueries({ queryKey: ["ai.memory_list"] });
        toast.info("Forgotten.");
      })
      .catch((err) => (err instanceof OpError ? toast.error(err.message) : reportError(err)));
  };

  return (
    <section className="card pad" style={{ width: 420, flex: "none" }} aria-label="Assistant memory">
      <div className="h4" style={{ marginTop: 0 }}>
        Assistant memory
      </div>
      {!entries ? (
        <Skeleton h={60} />
      ) : entries.length === 0 ? (
        <p className="sm muted">
          Nothing is remembered yet. The assistant keeps a fact or preference only when you state it outright, or confirm one of its answers.
        </p>
      ) : (
        <ul className="sm" aria-label="Remembered entries" style={{ listStyle: "none", margin: 0, padding: 0, maxHeight: 320, overflow: "auto" }}>
          {entries.map((e) => (
            <li key={e.id} className="row" style={{ gap: 8, alignItems: "flex-start", padding: "6px 0" }}>
              <div className="grow">
                <div className="selectable-text">{e.text}</div>
                <div className="xs muted">
                  {e.kind === "preference" ? "Preference" : "Fact"}, saved {dayAndTime(e.createdAt)}
                </div>
              </div>
              <Button size="sm" variant="ghost" aria-label={`Delete: ${e.text}`} onClick={() => forget(e)}>
                Delete
              </Button>
            </li>
          ))}
        </ul>
      )}
      <p className="hint">
        Saved on this computer, in this project, for you only. Delete an entry and the assistant forgets it.
      </p>
    </section>
  );
}
