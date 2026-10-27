// Answers show line breaks, simple lists and the "Idea:" label, built as plain React elements.

import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { AnswerText, parseAnswer } from "./answerText";

describe("parseAnswer", () => {
  it("makes a paragraph of each run of lines and starts a new one on a blank line", () => {
    expect(parseAnswer("One.\nStill one.\n\nTwo.")).toEqual([
      { kind: "p", lines: ["One.", "Still one."] },
      { kind: "p", lines: ["Two."] },
    ]);
  });

  it("reads lines starting with a dash as a bullet list and with a number as a numbered list", () => {
    expect(parseAnswer("The scenes:\n- Kitchen\n- Garden\n\n1. First\n2) Second")).toEqual([
      { kind: "p", lines: ["The scenes:"] },
      { kind: "ul", items: ["Kitchen", "Garden"] },
      { kind: "ol", items: ["First", "Second"] },
    ]);
  });

  it("does not treat a dash or number inside a line as a list", () => {
    expect(parseAnswer("Scene 1. Devi - alone, 3. maybe")).toEqual([{ kind: "p", lines: ["Scene 1. Devi - alone, 3. maybe"] }]);
  });

  it("copes with empty text, Windows line ends and trailing blank lines", () => {
    expect(parseAnswer("")).toEqual([]);
    expect(parseAnswer("\n\n")).toEqual([]);
    expect(parseAnswer("A\r\n- b\r\n\r\n")).toEqual([
      { kind: "p", lines: ["A"] },
      { kind: "ul", items: ["b"] },
    ]);
  });
});

describe("AnswerText", () => {
  it("draws paragraphs, a bullet list and a numbered list", () => {
    const { container } = render(<AnswerText text={"Two places.\n\n- Kitchen\n- Garden\n\n1. Open\n2. Close"} />);
    expect(container.querySelectorAll("p")).toHaveLength(1);
    expect([...container.querySelectorAll("ul li")].map((l) => l.textContent)).toEqual(["Kitchen", "Garden"]);
    expect([...container.querySelectorAll("ol li")].map((l) => l.textContent)).toEqual(["Open", "Close"]);
  });

  it("keeps a line break inside a paragraph", () => {
    const { container } = render(<AnswerText text={"First line\nSecond line"} />);
    expect(container.querySelectorAll("p br")).toHaveLength(1);
    expect(container.textContent).toBe("First lineSecond line");
  });

  it("writes the Idea: label in bold and nothing else", () => {
    const { container } = render(<AnswerText text={"Idea: let the kite fall.\nIdea: cut to the station.\nNot an Idea: here"} />);
    const bold = [...container.querySelectorAll("b")].map((b) => b.textContent);
    expect(bold).toEqual(["Idea:", "Idea:"]);
    expect(container.textContent).toContain("let the kite fall.");
    const inList = render(<AnswerText text={"- Idea: a flood"} />).container;
    expect(inList.querySelector("li b")?.textContent).toBe("Idea:");
  });

  it("shows markup as the characters it is, never as elements", () => {
    const hostile = ["<img src=x onerror=alert(1)>", "- <script>alert(1)</script>", "1. [link](https://evil.example)", "Idea: <b>bold</b>"].join("\n");
    const { container } = render(<AnswerText text={hostile} />);
    for (const tag of ["img", "script", "a", "iframe"]) expect(container.querySelectorAll(tag).length, tag).toBe(0);
    // The only bold element is the label.
    expect([...container.querySelectorAll("b")].map((b) => b.textContent)).toEqual(["Idea:"]);
    expect(container.textContent).toContain("<script>alert(1)</script>");
    expect(container.textContent).toContain("<img src=x onerror=alert(1)>");
    expect(container.textContent).toContain("[link](https://evil.example)");
  });
});
