// Answers read like a conversation (OpenFrameAI.docx sections 4.1 and 4.3): plain text first,
// sources behind a closed "See sources", and a long list is complete behind "Show all".

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
    nav: null,
    steps: [],
    changeSet: null,
    ...over,
  } as unknown as AiResultDto;
}

describe("assistant answers", () => {
  it("show the answer as plain text with its sources behind a closed See sources", () => {
    renderWithClient(<ResultView r={result({ details: ["From the screenplay rows."] })} onChanged={() => undefined} />);
    const text = screen.getByText("There are 130 scenes in the current screenplay.");
    expect(text.tagName).not.toBe("B");
    const sources = screen.getByText("See sources").closest("details")!;
    expect(sources.open).toBe(false);
    expect(sources.textContent).toContain("Draft: ENE");
    // How the answer was worked out stays out of the conversation.
    expect(document.body.textContent).not.toContain("Exact · from project data");
    expect(document.body.textContent).not.toContain("From the screenplay rows.");
  });

  it("show no See sources when the answer has only its scope", () => {
    renderWithClient(
      <ResultView r={result({ provenance: [{ kind: "Scope", label: "Whole Project", nav: null }] } as Partial<AiResultDto>)} onChanged={() => undefined} />,
    );
    expect(screen.queryByText("See sources")).toBeNull();
  });

  it("list every item behind Show all and never end with and N more", () => {
    const items = Array.from({ length: 52 }, (_, i) => ({ label: "CHARACTER " + i, detail: null, nav: null }));
    renderWithClient(<ResultView r={result({ content: "52 characters speak in ENE.", items })} onChanged={() => undefined} />);
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
