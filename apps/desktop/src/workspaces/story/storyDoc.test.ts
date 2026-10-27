// The Story page document mapping: plain text plus formatting in, the same out; a formatting that
// does not match its text is ignored, never applied.

import { describe, expect, it } from "vitest";
import { BODY_SIZE, contentOf, docFrom, parseFormat, storySchema, wordCount } from "./storyDoc";

const FORMAT = JSON.stringify([
  [{ t: "Hero", b: true, s: 20 }, { t: " walks ", i: true }, { t: "away", u: true, h: true }],
  [],
  [{ t: "The end" }],
]);
const TEXT = "Hero walks away\n\nThe end";

describe("Story page document", () => {
  it("round-trips the text and its formatting", () => {
    const doc = docFrom(TEXT, FORMAT);
    expect(doc.childCount).toBe(3);
    expect(() => doc.check()).not.toThrow();
    const back = contentOf(doc);
    expect(back.text).toBe(TEXT);
    expect(JSON.parse(back.format!)).toEqual(JSON.parse(FORMAT));
  });

  it("stores plain text with no formatting at all", () => {
    expect(contentOf(docFrom("One\n\nTwo", null))).toEqual({ text: "One\n\nTwo", format: null });
    expect(contentOf(docFrom("", null))).toEqual({ text: "", format: null });
  });

  it("ignores a formatting that does not match the text", () => {
    expect(parseFormat("Other words", FORMAT)).toBeNull();
    expect(parseFormat(TEXT, "not json")).toBeNull();
    expect(parseFormat(TEXT, JSON.stringify([[{ t: "Hero walks away" }]]))).toBeNull();
    const doc = docFrom("Other words", FORMAT);
    expect(contentOf(doc)).toEqual({ text: "Other words", format: null });
  });

  it("joins neighbouring text of the same style into one run and drops the body size", () => {
    const s = storySchema;
    const para = s.nodes.paragraph.create(null, [
      s.text("Ab", [s.marks.bold.create()]),
      s.text("cd", [s.marks.bold.create(), s.marks.size.create({ pt: BODY_SIZE })]),
      s.text("ef", [s.marks.size.create({ pt: 18 })]),
    ]);
    const { format } = contentOf(s.nodes.doc.create(null, [para]));
    expect(JSON.parse(format!)).toEqual([[{ t: "Abcd", b: true }, { t: "ef", s: 18 }]]);
  });

  it("round-trips highlight and underline colours and reads old runs as yellow and ink", () => {
    const coloured = JSON.stringify([
      [
        { t: "Green", h: true, hc: "green" },
        { t: " red", u: true, uc: "red" },
        { t: " old", u: true, h: true },
        { t: " both", u: true, uc: "purple", h: true, hc: "pink" },
      ],
    ]);
    const text = "Green red old both";
    const doc = docFrom(text, coloured);
    expect(JSON.parse(contentOf(doc).format!)).toEqual(JSON.parse(coloured));
    const marks = doc.firstChild!.child(2).marks;
    expect(marks.map((m) => [m.type.name, m.attrs.color])).toEqual([
      ["underline", "ink"],
      ["highlight", "yellow"],
    ]);
    // Neighbouring text with different colours stays in separate runs.
    const s = storySchema;
    const para = s.nodes.paragraph.create(null, [
      s.text("ab", [s.marks.highlight.create({ color: "blue" })]),
      s.text("cd", [s.marks.highlight.create({ color: "orange" })]),
    ]);
    expect(JSON.parse(contentOf(s.nodes.doc.create(null, [para])).format!)).toEqual([
      [{ t: "ab", h: true, hc: "blue" }, { t: "cd", h: true, hc: "orange" }],
    ]);
    // A name outside the palette (never saved by the app) falls back to the default.
    expect(docFrom("x", JSON.stringify([[{ t: "x", h: true, hc: "teal" }]])).firstChild!.firstChild!.marks[0].attrs.color).toBe("yellow");
  });

  it("counts words", () => {
    expect(wordCount("  one two\n\nthree ")).toBe(3);
    expect(wordCount("")).toBe(0);
  });
});
