// Advanced screenplay settings: page setup with header, footer and presets, and
// custom elements (Screenplay and Production Depth Specification §8, §14, §15).
// Keyboard and label access, what is sent to the core, and errors shown in place.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { call, OpError } from "../../ipc/client";
import type { ScreenplayPageSetup } from "../../ipc/generated/ScreenplayPageSetup";
import type { ScreenplayPageSetupInfo } from "../../ipc/generated/ScreenplayPageSetupInfo";
import type { ScreenplayCustomElementDto } from "../../ipc/generated/ScreenplayCustomElementDto";
import { answerFrom } from "../../app/home/testUtils";
import { CustomElementsDialog, PageSetupDialog, SceneNumberDialog } from "./pagesetup";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));
const callMock = vi.mocked(call);
vi.setConfig({ testTimeout: 20_000 });

const STANDARD: ScreenplayPageSetup = {
  paper: "Letter",
  font: "Courier",
  fontSize: 12,
  lineSpacing: "single",
  firstPageNumber: 1,
  header: "",
  footer: "",
  headerOnFirstPage: false,
  includeTitlePage: true,
  sceneNumbers: false,
  revisionMarks: true,
};

const INFO: ScreenplayPageSetupInfo = {
  screenplayId: "sp1",
  setup: STANDARD,
  presets: [
    { id: "standard-letter", name: "Standard (US Letter)", builtIn: true, setup: STANDARD },
    { id: "standard-a4", name: "Standard (A4)", builtIn: true, setup: { ...STANDARD, paper: "A4" } },
  ],
  linesPerPage: 55,
  fonts: ["Courier"],
  minFontSize: 10,
  maxFontSize: 14,
};

function wrap(node: ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  return render(<QueryClientProvider client={client}>{node}</QueryClientProvider>);
}

beforeEach(() => {
  callMock.mockReset();
});

describe("Page setup dialog", () => {
  it("applies the A4 preset, builds a header from placeholders and saves the page setup", async () => {
    const sent: unknown[] = [];
    callMock.mockImplementation(
      answerFrom({
        "screenplay.page_setup": INFO,
        "screenplay.update_page_setup": (a: Record<string, unknown>) => {
          sent.push(a);
          return null;
        },
      }) as typeof call,
    );
    const onClose = vi.fn();
    wrap(<PageSetupDialog screenplayId="sp1" canEdit onClose={onClose} />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Standard (A4)" }));
    await user.click(screen.getByRole("button", { name: "Insert {title} into the header" }));
    await user.click(screen.getByRole("button", { name: "Insert {page} into the header" }));
    await user.type(screen.getByLabelText("Footer"), "Blue revision");
    await user.click(screen.getByRole("checkbox", { name: /first page/ }));
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(sent).toEqual([
      {
        screenplayId: "sp1",
        setup: { ...STANDARD, paper: "A4", header: "{title} {page}", footer: "Blue revision", headerOnFirstPage: true },
      },
    ]);
  });

  it("shows the core's refusal in the dialog and keeps it open", async () => {
    callMock.mockImplementation(
      answerFrom({
        "screenplay.page_setup": INFO,
        "screenplay.update_page_setup": () => {
          throw new OpError("screenplay.update_page_setup", { code: "validation.header", message: "The header can be at most 120 characters.", retryable: false });
        },
      }) as typeof call,
    );
    const onClose = vi.fn();
    wrap(<PageSetupDialog screenplayId="sp1" canEdit onClose={onClose} />);
    await userEvent.setup().click(await screen.findByRole("button", { name: "Save" }));
    expect(await screen.findByText("The header can be at most 120 characters.")).toBeTruthy();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("a viewer can read the setup but not save it or keep presets", async () => {
    callMock.mockImplementation(answerFrom({ "screenplay.page_setup": INFO }) as typeof call);
    wrap(<PageSetupDialog screenplayId="sp1" canEdit={false} onClose={() => undefined} />);
    expect(await screen.findByText("55 lines fit on a page with the saved settings.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Save" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByLabelText("New preset name")).toBeNull();
  });
});

const TEL: ScreenplayCustomElementDto = {
  id: "tel",
  screenplayId: "sp1",
  name: "Telephone",
  shortcut: "Ctrl+Alt+1",
  look: "dialogue",
  uppercase: true,
  italic: false,
  spaceBefore: 1,
  nextElement: "action",
  exportName: "TELEPHONE",
  usageCount: 3,
  rev: 1,
};

describe("Custom elements dialog", () => {
  it("adds a custom element with a Telugu name from the keyboard", async () => {
    const sent: unknown[] = [];
    callMock.mockImplementation(
      answerFrom({
        "screenplay.create_custom_element": (a: Record<string, unknown>) => {
          sent.push(a);
          return { ...TEL, id: "new" };
        },
      }) as typeof call,
    );
    wrap(<CustomElementsDialog screenplayId="sp1" elements={[]} canEdit onClose={() => undefined} />);
    const user = userEvent.setup();
    expect(screen.getByText("No custom elements yet.")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Add custom element" }));
    expect(document.activeElement?.id).toBe("ce-name");
    await user.keyboard("ఫోన్ కాల్");
    await user.click(screen.getByRole("checkbox", { name: "Italic" }));
    await user.type(screen.getByLabelText("Name in Final Draft files"), "PHONE CALL");
    await user.click(screen.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(sent).toHaveLength(1));
    expect(sent[0]).toMatchObject({ screenplayId: "sp1", name: "ఫోన్ కాల్", italic: true, exportName: "PHONE CALL", look: "action", nextElement: "action" });
  });

  it("an element in use cannot be deleted, and a refused name is shown in place", async () => {
    callMock.mockImplementation(
      answerFrom({
        "screenplay.update_custom_element": () => {
          throw new OpError("screenplay.update_custom_element", { code: "validation.name", message: "Another custom element already has this name.", retryable: false });
        },
      }) as typeof call,
    );
    wrap(<CustomElementsDialog screenplayId="sp1" elements={[TEL]} canEdit onClose={() => undefined} />);
    const user = userEvent.setup();
    expect(screen.getByText(/used 3 times/)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Delete" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Another custom element already has this name.")).toBeTruthy();
  });

  it("a viewer sees the list without edit controls", () => {
    wrap(<CustomElementsDialog screenplayId="sp1" elements={[TEL]} canEdit={false} onClose={() => undefined} />);
    expect(screen.getByText("Telephone")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Add custom element" })).toBeNull();
  });
});

describe("Scene number dialog", () => {
  it("asks before changing a number production planning uses, then sends the acknowledgement", async () => {
    const sent: Record<string, unknown>[] = [];
    callMock.mockImplementation(
      answerFrom({
        "screenplay.set_scene_number": (a: Record<string, unknown>) => {
          sent.push(a);
          if (!a.acknowledgeDependencies) {
            throw new OpError("screenplay.set_scene_number", {
              code: "validation.dependencies",
              message: "Scene 12 is used by production planning (2 shots). Change the number anyway?",
              retryable: false,
            });
          }
          return { sceneId: "sc12", label: "12A" };
        },
      }) as typeof call,
    );
    const onClose = vi.fn();
    wrap(<SceneNumberDialog sceneId="sc12" label="12" onClose={onClose} />);
    const user = userEvent.setup();
    const input = screen.getByLabelText("Number");
    expect(document.activeElement).toBe(input);
    await user.clear(input);
    await user.type(input, "12A");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText(/used by production planning/)).toBeTruthy();
    expect(onClose).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Change the number" }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
    expect(sent.map((a) => [a.number, a.acknowledgeDependencies])).toEqual([
      ["12A", false],
      ["12A", true],
    ]);
  });
});
