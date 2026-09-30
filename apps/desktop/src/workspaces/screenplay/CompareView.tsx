// Draft comparison (FSD §22; UX §3.15; mock 085). Read-only: no merge, no edit.

import { useEffect, useMemo, useState } from "react";
import { ArrowLeft, ChevronDown, ChevronUp } from "lucide-react";
import { Banner, Button, Chip, EmptyState, Segmented, Select, Skeleton } from "../../design-system";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplaySceneChange } from "../../ipc/generated/ScreenplaySceneChange";
import type { ScreenplayDiffLine } from "../../ipc/generated/ScreenplayDiffLine";
import { useCompare } from "./api";
import { ELEMENT_META } from "./editor/schema";

const KIND_LABEL: Record<string, string> = { added: "Added", removed: "Removed", changed: "Changed", moved: "Moved", unchanged: "Same" };
const KIND_TONE: Record<string, "g" | "r" | "b" | "y" | "default"> = { added: "g", removed: "r", changed: "b", moved: "y", unchanged: "default" };

function sceneName(c: ScreenplaySceneChange): string {
  const s = c.b ?? c.a;
  return s ? `Scene ${s.number}` : "Scene";
}

function describe(c: ScreenplaySceneChange): string {
  const n = sceneName(c);
  switch (c.kind) {
    case "added":
      return `${n} added`;
    case "removed":
      return `${c.a ? `Scene ${c.a.number}` : n} removed`;
    case "moved":
      return `${n} moved (was ${c.a?.number})`;
    case "changed":
      return `${c.linesChanged} line${c.linesChanged === 1 ? "" : "s"} changed in ${n}`;
    default:
      return `${n} unchanged`;
  }
}

function Line({ l, side }: { l: ScreenplayDiffLine | null; side: "left" | "right" }) {
  if (!l) return <div className="spx-dl empty" aria-hidden />;
  const cls = ELEMENT_META[l.elementType]?.cls ?? "sp-a";
  return (
    <div className={`spx-dl sp-el ${cls}`}>
      {l.segments.map((s, i) =>
        s.kind === "equal" ? (
          <span key={i}>{s.text}</span>
        ) : (
          <span key={i} className={s.kind === "removed" ? "diff-del" : "diff-add"}>
            <span className="sr-only">{side === "left" ? "removed: " : "added: "}</span>
            {s.text}
          </span>
        ),
      )}
      {l.text === "" && <span className="muted">(empty)</span>}
    </div>
  );
}

export function CompareView({ drafts, initialA, initialB, onBack }: {
  drafts: ScreenplayDraftDto[];
  initialA: string | null;
  initialB: string | null;
  onBack: () => void;
}) {
  const sorted = useMemo(() => drafts.slice().sort((a, b) => a.createdAt - b.createdAt), [drafts]);
  const current = drafts.find((d) => d.isCurrent);
  const [a, setA] = useState<string>(() => initialA ?? sorted.find((d) => d.id !== (initialB ?? current?.id))?.id ?? sorted[0]?.id ?? "");
  const [b, setB] = useState<string>(() => initialB ?? current?.id ?? sorted[sorted.length - 1]?.id ?? "");
  const [mode, setMode] = useState<"scene" | "text">("text");
  const cmp = useCompare(a, b);
  const changes = cmp.data?.scenes ?? [];
  const interesting = changes.map((c, i) => ({ c, i })).filter(({ c }) => c.kind !== "unchanged");
  const [sel, setSel] = useState<number>(-1);
  useEffect(() => {
    setSel(interesting[0]?.i ?? -1);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cmp.data]);
  const step = (dir: 1 | -1) => {
    const pos = interesting.findIndex(({ i }) => i === sel);
    const next = interesting[(pos + dir + interesting.length) % interesting.length];
    if (next) setSel(next.i);
  };
  const s = cmp.data?.summary;
  const selected = sel >= 0 ? changes[sel] : null;
  const opts = sorted.map((d) => ({ value: d.id, label: d.name }));
  const groups: [string, ScreenplaySceneChange[]][] = ["removed", "added", "changed", "moved"].map((k) => [k, changes.filter((c) => c.kind === k)]);

  return (
    <div className="spx-compare">
      <div className="sp-top">
        <Button size="sm" variant="ghost" icon={<ArrowLeft size={14} />} onClick={onBack}>Back to script</Button>
        <Select ariaLabel="Before" value={a} onChange={setA} options={opts} />
        <span aria-hidden>→</span>
        <Select ariaLabel="After" value={b} onChange={setB} options={opts} />
        <span className="grow" />
        <Segmented ariaLabel="Comparison view" value={mode} onChange={setMode} options={[{ value: "scene", label: "Scene-only" }, { value: "text", label: "Text-only" }]} />
        <Button size="sm" icon={<ChevronUp size={14} />} disabled={interesting.length === 0} onClick={() => step(-1)}>Previous</Button>
        <Button size="sm" icon={<ChevronDown size={14} />} disabled={interesting.length === 0} onClick={() => step(1)}>Next change</Button>
      </div>
      {a === b ? (
        <div className="content"><Banner tone="info">Choose two different drafts to compare.</Banner></div>
      ) : cmp.isLoading ? (
        <div className="content"><Skeleton h={200} /></div>
      ) : cmp.error ? (
        <div className="content"><Banner tone="err">{cmp.error.message}</Banner></div>
      ) : (
        <div className="sp-body">
          <nav className="sp-nav spx-scroll" aria-label="Changes">
            <div className="h4">Changes</div>
            {groups.map(([k, list]) =>
              list.map((c) => {
                const i = changes.indexOf(c);
                return (
                  <button key={i} type="button" className={`sni${i === sel ? " on" : ""}`} onClick={() => setSel(i)}>
                    <Chip tone={KIND_TONE[k]}>{KIND_LABEL[k]}</Chip>
                    <span>{describe(c)}{c.ambiguous ? " · check match" : ""}</span>
                  </button>
                );
              }),
            )}
            {s && s.unchanged > 0 && (
              <div className="sni muted">
                <Chip>Same</Chip>
                <span>{s.unchanged} scene{s.unchanged === 1 ? "" : "s"} unchanged</span>
              </div>
            )}
          </nav>
          <div className="grow spx-scroll" style={{ padding: 16 }}>
            {s && (
              <div className="sm" style={{ marginBottom: 10 }}>
                <b>Summary:</b> {s.added} scene{s.added === 1 ? "" : "s"} added · {s.removed} removed · {s.changed} changed · {s.moved} moved · {s.unchanged} unchanged
                {s.ambiguous > 0 && (
                  <div style={{ marginTop: 6 }}>
                    <Banner tone="warn">{s.ambiguous} scene match{s.ambiguous === 1 ? " is" : "es are"} uncertain (same heading, no shared history). Check them before relying on this comparison.</Banner>
                  </div>
                )}
              </div>
            )}
            {interesting.length === 0 ? (
              <EmptyState title="No differences found." />
            ) : mode === "scene" ? (
              <table className="tbl card">
                <thead>
                  <tr><th>Change</th><th>{cmp.data?.draftA.name}</th><th>{cmp.data?.draftB.name}</th><th>Matched by</th></tr>
                </thead>
                <tbody>
                  {changes.map((c, i) => (
                    <tr key={i} className={i === sel ? "sel" : ""} onClick={() => { setSel(i); setMode("text"); }} style={{ cursor: "pointer" }}>
                      <td><Chip tone={KIND_TONE[c.kind]}>{KIND_LABEL[c.kind]}</Chip>{c.moved && c.kind === "changed" ? <Chip tone="y">Moved</Chip> : null}</td>
                      <td>{c.a ? `${c.a.number}. ${c.a.heading || "Untitled scene"}` : "—"}</td>
                      <td>{c.b ? `${c.b.number}. ${c.b.heading || "Untitled scene"}` : "—"}</td>
                      <td className="xs muted">{c.matchedBy === "identity" ? "Same scene" : c.matchedBy === "heading" ? `Heading${c.ambiguous ? " (uncertain)" : ""}` : "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : selected ? (
              <>
                <div className="row" style={{ marginBottom: 8 }}>
                  <b>{describe(selected)}</b>
                  {selected.ambiguous && <Chip tone="y">Uncertain match</Chip>}
                </div>
                <div className="spx-diff" role="table" aria-label="Side-by-side comparison">
                  <div className="spx-diff-h" role="row">
                    <div role="columnheader">{cmp.data?.draftA.name} — before</div>
                    <div role="columnheader">{cmp.data?.draftB.name} — after</div>
                  </div>
                  {selected.rows.length === 0 ? (
                    <p className="muted sm">The text of this scene is identical; only its position changed.</p>
                  ) : (
                    selected.rows.map((r, i) => (
                      <div key={i} className={`spx-diff-r ${r.kind}`} role="row">
                        <Line l={r.left} side="left" />
                        <Line l={r.right} side="right" />
                      </div>
                    ))
                  )}
                </div>
              </>
            ) : null}
          </div>
        </div>
      )}
    </div>
  );
}
