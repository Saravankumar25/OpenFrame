// Global search overlay (FSD §41, UX §2.8 / mock 156).

import { useEffect, useMemo, useRef, useState } from "react";
import * as RDialog from "@radix-ui/react-dialog";
import { Search } from "lucide-react";
import { useOp } from "../../ipc/query";
import type { SearchHit } from "../../ipc/generated/SearchHit";
import { Chip, Segmented } from "../../design-system";
import { navigateTo } from "../navigate";

function Snippet({ text }: { text: string }) {
  // Matches are wrapped in \u0001 … \u0002 by the backend.
  // eslint-disable-next-line no-control-regex
  const parts = text.split(/(\u0001[^\u0002]*\u0002)/g);
  return (
    <span>
      {parts.map((p, i) =>
        p.startsWith("\u0001") ? <mark key={i} style={{ background: "#ffe27a" }}>{p.slice(1, -1)}</mark> : <span key={i}>{p}</span>,
      )}
    </span>
  );
}

const TYPE_LABELS: Record<string, string> = {
  vault_item: "IDEA VAULT",
  story_act: "ACT",
  story_sequence: "SEQUENCE",
  story_beat: "BEAT",
  story_scene_card: "SCENE CARD",
  story_character: "CHARACTER",
  screenplay_scene: "SCENE",
  screenplay_element: "SCREENPLAY",
  catalog_item: "CATALOG",
  location: "LOCATION",
  cast_member: "CAST",
  crew_member: "CREW",
  shot: "SHOT",
  storyboard: "STORYBOARD",
  call_sheet: "CALL SHEET",
  project_file: "FILE",
  comment: "COMMENT",
  project_note: "NOTE",
  private_note: "PRIVATE NOTE",
};

export function SearchOverlay({ onClose }: { onClose: () => void }) {
  const [text, setText] = useState("");
  const [debounced, setDebounced] = useState("");
  const [scope, setScope] = useState<"project" | "global">("project");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const t = window.setTimeout(() => setDebounced(text), 150);
    return () => window.clearTimeout(t);
  }, [text]);
  const args = useMemo(() => ({ text: debounced, includeGlobal: scope === "global", limit: 60 }), [debounced, scope]);
  const res = useOp<SearchHit[]>("search.query", args, ["search_doc"], { enabled: debounced.trim().length > 0 });
  const hits = debounced.trim() ? res.data ?? [] : [];

  const open = (h: SearchHit) => {
    navigateTo(h.nav, h.store);
    onClose();
  };

  return (
    <RDialog.Root open onOpenChange={(v) => !v && onClose()}>
      <RDialog.Portal>
        <RDialog.Overlay className="scrim" />
        <RDialog.Content
          className="dialog xl"
          aria-describedby={undefined}
          style={{ position: "fixed", top: 70, left: "50%", transform: "translateX(-50%)", width: 720, zIndex: 51 }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") { e.preventDefault(); setActive((a) => Math.min(a + 1, hits.length - 1)); }
            if (e.key === "ArrowUp") { e.preventDefault(); setActive((a) => Math.max(a - 1, 0)); }
            if (e.key === "Enter" && hits[active]) { e.preventDefault(); open(hits[active]); }
          }}
        >
          <RDialog.Title className="sr-only">Search</RDialog.Title>
          <div className="dh" style={{ paddingBottom: 12 }}>
            <div className="input grow" style={{ fontSize: 16, padding: "10px 12px" }}>
              <Search size={18} />
              <input
                autoFocus
                aria-label="Search"
                value={text}
                onChange={(e) => { setText(e.target.value); setActive(0); }}
                placeholder="Search scenes, ideas, characters, locations, files…"
                style={{ border: 0, outline: "none", flex: 1, fontSize: 16, background: "transparent" }}
              />
              <kbd>Esc</kbd>
            </div>
          </div>
          <div className="row" style={{ padding: "0 18px 8px" }}>
            <Segmented ariaLabel="Search scope" value={scope} onChange={setScope} options={[{ value: "project", label: "This project" }, { value: "global", label: "+ Global Idea Vault" }]} />
            <span className="grow" />
            {debounced && <span className="muted sm">{res.isFetching ? "Searching…" : `${hits.length} result${hits.length === 1 ? "" : "s"}`}</span>}
          </div>
          <div className="db" ref={listRef} role="listbox" aria-label="Results" style={{ maxHeight: 460, padding: "4px 10px 12px" }}>
            {debounced && !res.isFetching && hits.length === 0 && (
              <div className="muted" style={{ padding: 16, textAlign: "center" }}>No matches. Search looks at titles, text, notes, file names and tags.</div>
            )}
            {hits.map((h, i) => (
              <button
                key={`${h.store}:${h.entityType}:${h.entityId}`}
                role="option"
                aria-selected={i === active}
                onMouseEnter={() => setActive(i)}
                onClick={() => open(h)}
                className="row"
                style={{ width: "100%", textAlign: "left", border: 0, borderRadius: 8, padding: "8px 10px", background: i === active ? "var(--accent-soft)" : "transparent" }}
              >
                <span style={{ width: 120, flex: "none" }}><Chip>{TYPE_LABELS[h.entityType] ?? h.entityType.toUpperCase()}</Chip></span>
                <span className="grow">
                  <div className="b" style={{ fontSize: 13 }}>{h.title || "Untitled"}</div>
                  <div className="sm muted" style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}><Snippet text={h.snippet} /></div>
                </span>
                <span className="xs muted" style={{ flex: "none" }}>{h.store === "global" ? "Global Idea Vault" : h.context}</span>
              </button>
            ))}
          </div>
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}
