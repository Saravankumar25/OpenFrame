// The Story page document: a ProseMirror schema with one paragraph per line of text and five
// marks (bold, italic, underline, highlight, size; underline and highlight carry a colour from a
// small palette), and the mapping to what the project stores:
// the plain text plus a formatting that lists the runs of each line (Rust:
// crates/openframe-import-export/src/prose.rs). The runs of a line joined together are exactly that
// line, so the plain text never depends on the formatting.

import { Schema, type Mark, type Node as PMNode } from "prosemirror-model";

/** Size of text with no size of its own, in points (the PDF uses the same). */
export const BODY_SIZE = 12;
export const SIZE_MIN = 6;
export const SIZE_MAX = 96;
/** The sizes offered in the toolbar. */
export const SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 60, 72];

/** Highlight colours (the first is the default). Rust: HIGHLIGHTS in prose.rs. */
export const HIGHLIGHT_COLORS = ["yellow", "green", "blue", "pink", "orange", "purple"] as const;
/** Underline colours; "ink" is the colour of the text. Rust: UNDERLINES in prose.rs. */
export const UNDERLINE_COLORS = ["ink", "red", "blue", "green", "orange", "purple"] as const;
export type HighlightColor = (typeof HIGHLIGHT_COLORS)[number];
export type UnderlineColor = (typeof UNDERLINE_COLORS)[number];

/** `key` when it is in the palette, else the palette's default. */
const inPalette = <T extends string>(list: readonly T[], key: unknown): T =>
  list.includes(key as T) ? (key as T) : list[0];

const clampSize = (pt: number) => Math.round(Math.min(SIZE_MAX, Math.max(SIZE_MIN, pt)) * 2) / 2;

/** A size from pasted CSS ("14pt", "18.67px"), or null. */
function cssSize(v: string): number | null {
  const m = /^(\d+(?:\.\d+)?)(pt|px)$/.exec(v.trim());
  if (!m) return null;
  return clampSize(m[2] === "px" ? Number(m[1]) * 0.75 : Number(m[1]));
}

export const storySchema = new Schema({
  nodes: {
    doc: { content: "paragraph+" },
    paragraph: { content: "text*", parseDOM: [{ tag: "p" }, { tag: "div" }], toDOM: () => ["p", 0] },
    text: {},
  },
  marks: {
    bold: {
      parseDOM: [{ tag: "strong" }, { tag: "b" }, { style: "font-weight", getAttrs: (v) => (/^(bold|[6-9]00)$/.test(v) ? null : false) }],
      toDOM: () => ["strong", 0],
    },
    italic: { parseDOM: [{ tag: "em" }, { tag: "i" }, { style: "font-style=italic" }], toDOM: () => ["em", 0] },
    underline: {
      attrs: { color: { default: UNDERLINE_COLORS[0] } },
      parseDOM: [
        { tag: "u", getAttrs: (el) => ({ color: inPalette(UNDERLINE_COLORS, el.getAttribute("data-color")) }) },
        { style: "text-decoration=underline" },
      ],
      toDOM: (m) => {
        const c = m.attrs.color as string;
        // An ink underline takes the colour of the text.
        return ["u", c === UNDERLINE_COLORS[0] ? { "data-color": c } : { "data-color": c, style: `text-decoration-color: var(--story-ul-${c})` }, 0];
      },
    },
    highlight: {
      attrs: { color: { default: HIGHLIGHT_COLORS[0] } },
      parseDOM: [{ tag: "mark", getAttrs: (el) => ({ color: inPalette(HIGHLIGHT_COLORS, el.getAttribute("data-color")) }) }],
      toDOM: (m) => ["mark", { "data-color": m.attrs.color as string, style: `background-color: var(--story-hl-${m.attrs.color as string})` }, 0],
    },
    size: {
      attrs: { pt: { default: BODY_SIZE } },
      parseDOM: [
        {
          style: "font-size",
          getAttrs: (v) => {
            const pt = cssSize(v);
            return pt === null || pt === BODY_SIZE ? false : { pt };
          },
        },
      ],
      toDOM: (m) => ["span", { style: `font-size:${m.attrs.pt as number}pt` }, 0],
    },
  },
});

/** One stretch of a line in one style, as stored. `hc` and `uc` name the highlight and underline
 *  colour; they are left out for the defaults (yellow, ink), so older pages read the same. */
export type Run = { t: string; b?: boolean; i?: boolean; u?: boolean; h?: boolean; s?: number; hc?: string; uc?: string };

const styled = (r: Run) => !!(r.b || r.i || r.u || r.h || r.s !== undefined);

/** The stored formatting when it parses and matches the text (else null: show the plain text). */
export function parseFormat(text: string, format: string | null | undefined): Run[][] | null {
  if (!format) return null;
  let lines: unknown;
  try {
    lines = JSON.parse(format);
  } catch {
    return null;
  }
  const want = text.split("\n");
  if (!Array.isArray(lines) || lines.length !== want.length) return null;
  for (let n = 0; n < want.length; n++) {
    const line = lines[n] as unknown;
    if (!Array.isArray(line)) return null;
    if (line.some((r: Run) => typeof r?.t !== "string")) return null;
    if (line.map((r: Run) => r.t).join("") !== want[n]) return null;
  }
  return lines as Run[][];
}

function marksOf(r: Run): Mark[] {
  const m: Mark[] = [];
  const s = storySchema.marks;
  if (r.b) m.push(s.bold.create());
  if (r.i) m.push(s.italic.create());
  if (r.u) m.push(s.underline.create({ color: inPalette(UNDERLINE_COLORS, r.uc) }));
  if (r.h) m.push(s.highlight.create({ color: inPalette(HIGHLIGHT_COLORS, r.hc) }));
  if (r.s !== undefined && r.s !== BODY_SIZE) m.push(s.size.create({ pt: clampSize(r.s) }));
  return m;
}

/** The editor document of a stored page. */
export function docFrom(text: string, format: string | null | undefined): PMNode {
  const lines = parseFormat(text, format) ?? text.split("\n").map((t) => (t ? [{ t }] : []));
  const paras = lines.map((runs) =>
    storySchema.nodes.paragraph.create(
      null,
      runs.filter((r) => r.t).map((r) => storySchema.text(r.t, marksOf(r))),
    ),
  );
  return storySchema.nodes.doc.create(null, paras);
}

function runOf(node: PMNode): Run {
  const r: Run = { t: node.text ?? "" };
  for (const m of node.marks) {
    if (m.type.name === "bold") r.b = true;
    else if (m.type.name === "italic") r.i = true;
    else if (m.type.name === "underline") {
      r.u = true;
      if (m.attrs.color !== UNDERLINE_COLORS[0]) r.uc = m.attrs.color as string;
    } else if (m.type.name === "highlight") {
      r.h = true;
      if (m.attrs.color !== HIGHLIGHT_COLORS[0]) r.hc = m.attrs.color as string;
    }
    else if (m.type.name === "size" && m.attrs.pt !== BODY_SIZE) r.s = m.attrs.pt as number;
  }
  return r;
}

const sameStyle = (a: Run, b: Run) =>
  !!a.b === !!b.b && !!a.i === !!b.i && !!a.u === !!b.u && !!a.h === !!b.h && a.s === b.s && a.hc === b.hc && a.uc === b.uc;

/** What the project stores for an editor document: the plain text and the formatting (null when
 *  nothing is formatted). */
export function contentOf(doc: PMNode): { text: string; format: string | null } {
  const lines: Run[][] = [];
  doc.forEach((para) => {
    const runs: Run[] = [];
    para.forEach((n) => {
      const r = runOf(n);
      const last = runs[runs.length - 1];
      if (last && sameStyle(last, r)) last.t += r.t;
      else runs.push(r);
    });
    lines.push(runs);
  });
  const text = lines.map((l) => l.map((r) => r.t).join("")).join("\n");
  const format = lines.some((l) => l.some(styled)) ? JSON.stringify(lines) : null;
  return { text, format };
}

export function wordCount(text: string): number {
  const t = text.trim();
  return t ? t.split(/\s+/).length : 0;
}
