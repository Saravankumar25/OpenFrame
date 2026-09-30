// In-script find (FSD §15.8, UX §3.13). Same semantics as Rust's
// `screenplay.replace_all`: literal text, optional case sensitivity, optional
// whole-word boundaries where letters, digits and "_" are word characters.

export interface FindOptions {
  query: string;
  matchCase: boolean;
  wholeWord: boolean;
}

const WORD = /[\p{L}\p{N}_]/u;

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** [start, end) offsets (UTF-16) of matches in `text`, non-overlapping, left to right. */
export function findInText(text: string, o: FindOptions): [number, number][] {
  if (!o.query) return [];
  const re = new RegExp(escapeRegExp(o.query), o.matchCase ? "gu" : "giu");
  const out: [number, number][] = [];
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    const start = m.index;
    const end = start + m[0].length;
    if (m[0].length === 0) {
      re.lastIndex++;
      continue;
    }
    if (o.wholeWord) {
      const before = start > 0 ? Array.from(text.slice(0, start)).pop() : undefined;
      const after = end < text.length ? Array.from(text.slice(end))[0] : undefined;
      if ((before && WORD.test(before)) || (after && WORD.test(after))) continue;
    }
    out.push([start, end]);
  }
  return out;
}
