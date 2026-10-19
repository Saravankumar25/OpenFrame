import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

import { call } from "../../ipc/client";
import type { PackageReviewWorkspace } from "../../ipc/generated/PackageReviewWorkspace";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import { ReviewViewer } from "./ReviewViewer";

const el = (id: string, elementType: string, text: string, extra: object = {}) => ({
  id,
  elementType,
  text,
  customName: null,
  uppercase: false,
  italic: false,
  ...extra,
});

function workspace(comments: PackageReviewWorkspace["comments"] = []): PackageReviewWorkspace {
  return {
    key: "pkg-1",
    source: "review",
    packageType: "scriptReview",
    typeLabel: "Script Review Package",
    projectTitle: "Night Bus",
    screenplayTitle: "Night Bus",
    draftName: "Draft 3",
    exportedBy: "Nisha",
    exportedAt: 1_700_000_000_000,
    scenes: [
      {
        id: "s1",
        number: 1,
        label: "1",
        heading: "EXT. DEPOT - NIGHT",
        synopsis: null,
        elements: [el("e1", "action", "The last bus idles."), el("e2", "dialogue", "hello? who is this?", { customName: "Telephone", uppercase: true, italic: true })],
      },
      { id: "s2", number: 2, label: "12A", heading: "INT. BUS - NIGHT", synopsis: null, elements: [el("e3", "action", "Rain on the glass.")] },
    ],
    comments,
    canComment: true,
    responseExportedAt: null,
    summary: [],
  };
}

describe("review package viewer", () => {
  beforeEach(() => {
    vi.mocked(call).mockReset();
  });

  it("shows production scene numbers and lays out a custom element with its author's look", async () => {
    vi.mocked(call).mockImplementation(answerFrom({ "packages.review_get": workspace() }) as typeof call);
    renderWithClient(<ReviewViewer source="review" reviewKey="pkg-1" onClose={() => undefined} />);
    expect(await screen.findByText("12A")).toBeTruthy();
    const phone = screen.getByText("hello? who is this?");
    expect(phone.getAttribute("title")).toBe("Telephone");
    expect(phone.className).toBe("sp-d");
    expect(phone.style.textTransform).toBe("uppercase");
    expect(phone.style.fontStyle).toBe("italic");

    await userEvent.click(screen.getByRole("button", { name: /INT\. BUS - NIGHT/ }));
    expect(await screen.findByText("Comments on Scene 12A")).toBeTruthy();
  });

  it("adds a comment with Ctrl+Enter and keeps Export Response off until there is one", async () => {
    const added = workspace([
      { id: "c1", sceneId: "s1", quotedText: null, body: "Tighten this.", authorName: "Priya", createdAt: 1_700_000_100_000, mine: true },
    ]);
    const comment = vi.fn(() => added);
    vi.mocked(call).mockImplementation(answerFrom({ "packages.review_get": workspace(), "packages.review_comment": comment }) as typeof call);
    renderWithClient(<ReviewViewer source="review" reviewKey="pkg-1" onClose={() => undefined} />);
    const exportButton = await screen.findByRole("button", { name: "Export Response Package" });
    expect((exportButton as HTMLButtonElement).disabled).toBe(true);
    const box = screen.getByLabelText("Comment");
    await userEvent.type(box, "Tighten this.");
    await userEvent.keyboard("{Control>}{Enter}{/Control}");
    await waitFor(() => expect(comment).toHaveBeenCalledWith({ packageId: "pkg-1", sceneId: "s1", body: "Tighten this.", quotedText: null }));
    expect(await screen.findByText("Yours")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Export Response Package" }) as HTMLButtonElement).disabled).toBe(false);
  });
});
