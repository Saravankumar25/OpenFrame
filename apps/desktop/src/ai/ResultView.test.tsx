// Answers read like a conversation (OpenFrameAI.docx sections 4.1 and 4.3): plain text first,
// sources, steps and supporting lists behind "See sources"; a list the person asked for stays under
// the answer, and a long one is complete behind "Show all".

import { describe, expect, it } from "vitest";
import { fireEvent, screen } from "@testing-library/react";
import { renderWithClient } from "../app/home/testUtils";
import { ResultView } from "./Conversation";
import type { AiResultDto } from "../ipc/generated/AiResultDto";

function result(over: Partial<AiResultDto>): AiResultDto {
  return {
    kind: "Answer",
    status: "Informational",
    content: "There are 130 scenes in the current screenplay.",
    details: [],
    provenance: [{ kind: "Draft", label: "ENE", target: null }],
    confidence: "Exact",
    items: [],
    itemsAreAnswer: false,
    nav: null,
    steps: [],
    changeSet: null,
    ...over,
  } as unknown as AiResultDto;
}

describe("assistant answers", () => {
  it("show the answer as plain text with the metadata behind See sources", () => {
    renderWithClient(<ResultView r={result({ details: ["From the screenplay rows."] })} onChanged={() => undefined} />);
    const text = screen.getByText("There are 130 scenes in the current screenplay.");
    expect(text.tagName).not.toBe("B");
    expect(screen.queryByText("Details")).toBeNull();
    const summary = screen.getByText("See sources");
    expect(summary.tagName).toBe("SUMMARY");
    expect(summary.className).toContain("ghost");
    const details = summary.closest("details")!;
    expect(details.open).toBe(false);
    expect(details.textContent).toContain("Based on:");
    expect(details.textContent).toContain("From the screenplay rows.");
    expect(details.textContent).toContain("Exact");
  });

  it("keep the rows of a count under See sources, not under the answer", () => {
    const items = [{ label: "Scene 1: INT. KITCHEN - DAY", detail: null, nav: null }];
    renderWithClient(<ResultView r={result({ items, itemsAreAnswer: false })} onChanged={() => undefined} />);
    const row = screen.getByText("Scene 1: INT. KITCHEN - DAY");
    expect(row.closest("details")).toBeTruthy();
    expect(row.closest("details")!.open).toBe(false);
  });

  it("keep a requested list under the answer and leave no empty See sources", () => {
    const items = [{ label: "Scene 1: INT. KITCHEN - DAY", detail: null, nav: null }];
    renderWithClient(
      <ResultView r={result({ items, itemsAreAnswer: true, provenance: [], confidence: null })} onChanged={() => undefined} />,
    );
    expect(screen.getByText("Scene 1: INT. KITCHEN - DAY").closest("details")).toBeNull();
    expect(screen.queryByText("See sources")).toBeNull();
  });

  it("list every item behind Show all and never end with and N more", () => {
    const items = Array.from({ length: 52 }, (_, i) => ({ label: "CHARACTER " + i, detail: null, nav: null }));
    renderWithClient(
      <ResultView r={result({ content: "52 characters speak in ENE.", items, itemsAreAnswer: true })} onChanged={() => undefined} />,
    );
    expect(screen.queryByText(/and \d+ more/)).toBeNull();
    expect(screen.queryByText("CHARACTER 51")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Show all 52" }));
    expect(screen.getByText("CHARACTER 51")).toBeTruthy();
  });

  it("keep the reasons of a refusal in view", () => {
    renderWithClient(
      <ResultView r={result({ kind: "Denied", content: "I can't do that.", details: ["You are a Viewer on this project."] })} onChanged={() => undefined} />,
    );
    expect(screen.getByText(/You are a Viewer on this project/)).toBeTruthy();
  });
});
