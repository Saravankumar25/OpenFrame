// How an assistant answer is drawn (harness plan 5.15 and 7): paragraphs on blank lines, simple lists,
// and the "Idea:" label in bold. The text is plain text; every piece is built as a React element, so
// nothing in an answer can become live HTML (noLiveHtml.test.tsx keeps that true).

import type { ReactNode } from "react";

export type AnswerBlock =
  | { kind: "p"; lines: string[] }
  | { kind: "ul"; items: string[] }
  | { kind: "ol"; items: string[] };

const BULLET = /^\s{0,3}[-*•]\s+(.*\S)\s*$/;
const NUMBERED = /^\s{0,3}\d{1,3}[.)]\s+(.*\S)\s*$/;

/** Split an answer into blocks. A line break inside a paragraph stays a line break. */
export function parseAnswer(text: string): AnswerBlock[] {
  const blocks: AnswerBlock[] = [];
  const push = (kind: "p" | "ul" | "ol", line: string) => {
    const last = blocks[blocks.length - 1];
    if (kind === "p") {
      if (last?.kind === "p") last.lines.push(line);
      else blocks.push({ kind: "p", lines: [line] });
    } else if (last?.kind === kind) {
      last.items.push(line);
    } else {
      blocks.push({ kind, items: [line] });
    }
  };
  for (const raw of text.replace(/\r\n?/g, "\n").split("\n")) {
    if (raw.trim() === "") {
      // A blank line ends the paragraph or list, so the next line starts a new block.
      blocks.push({ kind: "p", lines: [] });
      continue;
    }
    const bullet = BULLET.exec(raw);
    const numbered = bullet ? null : NUMBERED.exec(raw);
    if (bullet) push("ul", bullet[1]);
    else if (numbered) push("ol", numbered[1]);
    else push("p", raw.trimEnd());
  }
  return blocks.filter((b) => b.kind !== "p" || b.lines.length > 0);
}

const IDEA = /^(\s*)(Idea(?:\s+\d+)?:)(\s*)(.*)$/s;

/** One line of an answer: an "Idea:" label in bold, the rest as written. */
function line(text: string): ReactNode {
  const m = IDEA.exec(text);
  if (!m) return text;
  return (
    <>
      {m[1]}
      <b>{m[2]}</b>
      {m[3]}
      {m[4]}
    </>
  );
}

export function AnswerText({ text }: { text: string }) {
  const blocks = parseAnswer(text);
  return (
    <div className="ai-text">
      {blocks.map((b, i) => {
        if (b.kind === "ul" || b.kind === "ol") {
          const Tag = b.kind;
          return (
            <Tag key={i} className="ai-list">
              {b.items.map((it, j) => (
                <li key={j}>{line(it)}</li>
              ))}
            </Tag>
          );
        }
        return (
          <p key={i} className="ai-p">
            {b.lines.map((l, j) => (
              <span key={j}>
                {j > 0 && <br />}
                {line(l)}
              </span>
            ))}
          </p>
        );
      })}
    </div>
  );
}
