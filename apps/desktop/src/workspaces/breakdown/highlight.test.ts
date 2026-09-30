import { offsetWithin, segmentText, selectionInLine, suggestName, type Mark } from "./highlight";

const tag = (start: number, end: number): Mark => ({ start, end, kind: "tag", label: "Props — Pistol" });

describe("segmentText", () => {
  it("splits text around marks", () => {
    const segs = segmentText("carrying a pistol.", [tag(11, 17)]);
    expect(segs.map((s) => s.text)).toEqual(["carrying a ", "pistol", "."]);
    expect(segs[1].mark?.kind).toBe("tag");
  });

  it("ignores invalid and overlapping marks", () => {
    const segs = segmentText("abcdef", [tag(1, 4), { start: 2, end: 5, kind: "suggestion", label: "x" }, tag(4, 99)]);
    expect(segs.map((s) => s.text)).toEqual(["a", "bcd", "ef"]);
  });

  it("keeps empty text renderable", () => {
    expect(segmentText("", [])).toEqual([{ text: "" }]);
  });
});

describe("selection offsets", () => {
  it("measures offsets across highlighted spans in one line", () => {
    document.body.innerHTML = `<div data-element-id="e1">He places the <mark>RED</mark> FOLDER on the desk.</div>`;
    const line = document.querySelector("[data-element-id]")!;
    const markText = line.querySelector("mark")!.firstChild!;
    const tail = markText.parentNode!.nextSibling!;
    expect(offsetWithin(line, markText, 0)).toBe(14);
    expect(offsetWithin(line, tail, 7)).toBe(24);
    expect(offsetWithin(line, line, 1)).toBe(14);

    const range = document.createRange();
    range.setStart(markText, 0);
    range.setEnd(tail, 8); // " FOLDER " → trailing space trimmed
    const sel = window.getSelection()!;
    sel.removeAllRanges();
    sel.addRange(range);
    const got = selectionInLine(sel);
    expect(got).toEqual({ elementId: "e1", start: 14, end: 24, text: "RED FOLDER" });
    expect(suggestName(got!.text)).toBe("Red Folder");
  });

  it("refuses selections across lines", () => {
    document.body.innerHTML = `<div data-element-id="a">one</div><div data-element-id="b">two</div>`;
    const [a, b] = Array.from(document.querySelectorAll("[data-element-id]"));
    const range = document.createRange();
    range.setStart(a.firstChild!, 0);
    range.setEnd(b.firstChild!, 2);
    const sel = window.getSelection()!;
    sel.removeAllRanges();
    sel.addRange(range);
    expect(selectionInLine(sel)).toBeNull();
  });

  it("title-cases tagged phrases", () => {
    expect(suggestName("  “pistol.” ")).toBe("Pistol");
    expect(suggestName("wet  grey coat")).toBe("Wet Grey Coat");
  });
});
