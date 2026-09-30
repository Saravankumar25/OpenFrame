// Pure helpers for the Breakdown script pane: highlighting tagged / suggested
// text spans and turning a DOM text selection into offsets within one
// screenplay element. Offsets are UTF-16 code units (JavaScript string
// indices), the same unit the Rust side stores.

export interface Mark {
  start: number;
  end: number;
  /** "tag" = production element, "suggestion" = pending suggestion. */
  kind: "tag" | "suggestion";
  /** Label for the tooltip, e.g. "Props — Pistol". */
  label: string;
}

export interface Segment {
  text: string;
  mark?: Mark;
}

/** Split `text` into plain and marked segments. Overlaps keep the earlier / tag mark. */
export function segmentText(text: string, marks: Mark[]): Segment[] {
  const valid = marks
    .filter((m) => m.start >= 0 && m.end <= text.length && m.start < m.end)
    .sort((a, b) => a.start - b.start || (a.kind === "tag" ? -1 : 1));
  const out: Segment[] = [];
  let pos = 0;
  for (const m of valid) {
    if (m.start < pos) continue; // overlapping — the earlier mark wins
    if (m.start > pos) out.push({ text: text.slice(pos, m.start) });
    out.push({ text: text.slice(m.start, m.end), mark: m });
    pos = m.end;
  }
  if (pos < text.length || out.length === 0) out.push({ text: text.slice(pos) });
  return out;
}

/** Offset of (node, offset) measured in text characters from the start of `root`. */
export function offsetWithin(root: Node, node: Node, offset: number): number | null {
  if (!root.contains(node)) return null;
  if (node.nodeType === Node.TEXT_NODE) {
    const before = textBefore(root, node);
    return before + offset;
  }
  // Element boundary: before child `offset`, or after the element's last child.
  const child = node.childNodes[offset];
  if (child) return textBefore(root, child);
  return textBefore(root, node) + (node.textContent?.length ?? 0);
}

/** Length of all text in `root` that comes before `target` in document order. */
function textBefore(root: Node, target: Node): number {
  let total = 0;
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let cur = walker.nextNode();
  while (cur) {
    if (cur === target || target.contains(cur) || target.compareDocumentPosition(cur) & Node.DOCUMENT_POSITION_FOLLOWING) {
      return total;
    }
    total += cur.textContent?.length ?? 0;
    cur = walker.nextNode();
  }
  return total;
}

export interface TextSelection {
  elementId: string;
  start: number;
  end: number;
  text: string;
}

/**
 * The selection inside one script line (`[data-element-id]`), trimmed of
 * surrounding whitespace. Selections spanning several lines are not taggable.
 */
export function selectionInLine(sel: Selection | null): TextSelection | null {
  if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return null;
  const range = sel.getRangeAt(0);
  const lineOf = (n: Node | null): HTMLElement | null => {
    let cur: Node | null = n;
    while (cur && !(cur instanceof HTMLElement && cur.dataset.elementId)) cur = cur.parentNode;
    return cur as HTMLElement | null;
  };
  const line = lineOf(range.startContainer);
  if (!line || line !== lineOf(range.endContainer)) return null;
  const a = offsetWithin(line, range.startContainer, range.startOffset);
  const b = offsetWithin(line, range.endContainer, range.endOffset);
  if (a === null || b === null) return null;
  let start = Math.min(a, b);
  let end = Math.max(a, b);
  const full = line.textContent ?? "";
  while (start < end && /\s/.test(full[start])) start++;
  while (end > start && /\s/.test(full[end - 1])) end--;
  if (start >= end) return null;
  return { elementId: line.dataset.elementId!, start, end, text: full.slice(start, end) };
}

/** Title-case a tagged phrase for the catalog name ("RED FOLDER" → "Red Folder"). */
export function suggestName(text: string): string {
  const t = text.replace(/\s+/g, " ").trim().replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
  if (!t) return "";
  return t
    .split(" ")
    .map((w) => (w.length ? w[0].toUpperCase() + w.slice(1).toLowerCase() : w))
    .join(" ");
}
