// Settings > Appearance (features THEME-001, THEME-004, THEME-005, THEME-006).

import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, within, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithClient } from "../../app/home/testUtils";
import { AppearancePanel } from "./AppearancePanel";

// Test fixtures only: the IPC boundary and the file dialogs are replaced.
const calls: { op: string; args: Record<string, unknown> }[] = [];
const harbour = {
  id: "harbour-night",
  name: "Harbour Night",
  description: "Blue-grey with a lamp-orange accent.",
  author: "Test Author",
  builtIn: false,
  seeds: { neutralHue: 230, neutralTint: 0.02, accentHue: 55, accentChroma: 0.14, accentLightnessNudge: 0 },
  light: {},
  dark: { accent: "oklch(74% 0.14 55)" },
};
let state: Record<string, unknown>;
let checkResult: Record<string, unknown>;
const pickedFile: { value: string | null } = { value: null };
const savedPath: { value: string | null } = { value: null };

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => pickedFile.value),
  save: vi.fn(async () => savedPath.value),
}));

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code: string;
    retryable = false;
    op = "";
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
    is(p: string) {
      return this.code === p || this.code.startsWith(p + ".");
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      calls.push({ op, args });
      switch (op) {
        case "appearance.get":
          return state;
        case "appearance.set_mode":
          state = { ...state, mode: args.mode };
          return state;
        case "appearance.set_theme":
          state = { ...state, themeId: args.themeId };
          return state;
        case "appearance.check_theme":
          return checkResult;
        case "appearance.import_theme":
          state = { ...state, myThemes: [{ id: harbour.id, name: harbour.name, theme: harbour, problem: null }], themeId: args.apply ? harbour.id : state.themeId };
          return state;
        case "appearance.delete_theme":
          state = { ...state, myThemes: [], themeId: "openframe-default" };
          return state;
        case "appearance.rename_theme":
          return state;
        case "appearance.export_theme":
          return { path: String(args.path), fileName: "Harbour Night.json" };
        default:
          throw new Error(`unexpected operation ${op}`);
      }
    }),
  };
});

beforeEach(() => {
  calls.length = 0;
  pickedFile.value = null;
  savedPath.value = null;
  state = { mode: "system", themeId: "openframe-default", fallbackNotice: null, myThemes: [] };
  checkResult = { status: "ok", theme: harbour, problems: [], notes: [], contrast: [{ mode: "light", label: "Main text on panels", foreground: "ink", background: "panel", ratio: 12.5, passes: true }] };
});

const ops = () => calls.map((c) => c.op).filter((o) => o !== "appearance.get");

describe("Settings > Appearance", () => {
  it("offers Light, Dark and System and the sixteen themes, OpenFrame Default selected", async () => {
    renderWithClient(<AppearancePanel />);
    const modes = await screen.findByRole("radiogroup", { name: "Mode" });
    expect(within(modes).getAllByRole("radio").map((r) => r.textContent)).toEqual(["Light", "Dark", "System"]);
    expect(within(modes).getByRole("radio", { name: "System" })).toHaveAttribute("aria-checked", "true");
    const themes = screen.getByRole("radiogroup", { name: "Theme" });
    const cards = within(themes).getAllByRole("radio");
    expect(cards).toHaveLength(16);
    expect(cards[0]).toHaveAccessibleName(/^OpenFrame Default/);
    expect(cards[0]).toHaveAttribute("aria-checked", "true");
    // Each card is drawn in its own theme.
    expect(cards[3]).toHaveAttribute("data-theme", "midnight-reel");
  });

  it("switches the mode and the theme with a click, without asking twice", async () => {
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("radio", { name: "Dark" }));
    await user.click(screen.getByRole("radio", { name: /^Slate Noir/ }));
    await waitFor(() => expect(ops()).toEqual(["appearance.set_mode", "appearance.set_theme"]));
    expect(calls.find((c) => c.op === "appearance.set_theme")?.args).toEqual({ themeId: "slate-noir" });
    expect(await screen.findByText("Slate Noir theme applied.")).toBeInTheDocument();
  });

  it("moves through the themes with the arrow keys (one tab stop)", async () => {
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    const first = await screen.findByRole("radio", { name: /^OpenFrame Default/ });
    expect(first).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: /^Studio Amber/ })).toHaveAttribute("tabindex", "-1");
    first.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: /^Studio Amber/ })).toHaveFocus();
    await user.keyboard("{End}");
    expect(screen.getByRole("radio", { name: /^High Contrast/ })).toHaveFocus();
    await waitFor(() => expect(calls.filter((c) => c.op === "appearance.set_theme").map((c) => c.args.themeId)).toEqual(["studio-amber", "high-contrast"]));
  });

  it("imports pasted theme text: check, preview, then save and apply", async () => {
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("button", { name: "Paste Theme…" }));
    const dialog = await screen.findByRole("dialog", { name: "Import theme" });
    await user.click(within(dialog).getByLabelText("Theme text"));
    await user.paste('{"formatVersion":1}');
    await user.click(within(dialog).getByRole("button", { name: "Check Theme" }));
    expect(await within(dialog).findByText("This theme is readable in Light and Dark.")).toBeInTheDocument();
    expect(within(dialog).getByText("Harbour Night")).toBeInTheDocument();
    expect(within(dialog).getByLabelText("Dark preview")).toHaveAttribute("data-theme", "__preview");
    await user.click(within(dialog).getByRole("button", { name: "Save and Apply" }));
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.import_theme")?.args).toEqual({ text: '{"formatVersion":1}', apply: true }));
    expect(await screen.findByRole("listitem", { name: "Harbour Night" })).toBeInTheDocument();
  });

  it("a refused theme says why, cannot be saved and changes nothing", async () => {
    checkResult = { status: "rejected", theme: null, problems: ['The file has a setting OpenFrame does not know: "script".'], notes: [], contrast: [] };
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("button", { name: "Paste Theme…" }));
    const dialog = await screen.findByRole("dialog", { name: "Import theme" });
    await user.click(within(dialog).getByLabelText("Theme text"));
    await user.paste('{"script":"alert(1)"}');
    await user.click(within(dialog).getByRole("button", { name: "Check Theme" }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent('This theme cannot be used. Nothing was changed.The file has a setting OpenFrame does not know: "script".');
    expect(within(dialog).getByRole("button", { name: "Save and Apply" })).toBeDisabled();
    expect(within(dialog).getByRole("button", { name: "Save to My Themes" })).toBeDisabled();
    expect(ops()).toEqual(["appearance.check_theme"]);
  });

  it("refuses pasted text over 10 KB before sending it", async () => {
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("button", { name: "Paste Theme…" }));
    const dialog = await screen.findByRole("dialog", { name: "Import theme" });
    await user.click(within(dialog).getByLabelText("Theme text"));
    await user.paste("x".repeat(10 * 1024 + 1));
    expect(within(dialog).getByText(/larger than 10 KB/)).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Check Theme" })).toBeDisabled();
  });

  it("checks a picked file straight away", async () => {
    pickedFile.value = "C:\\Themes\\harbour.json";
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("button", { name: "Import Theme File…" }));
    const dialog = await screen.findByRole("dialog", { name: "Import theme" });
    expect(await within(dialog).findByText("This theme is readable in Light and Dark.")).toBeInTheDocument();
    expect(calls.find((c) => c.op === "appearance.check_theme")?.args).toEqual({ path: "C:\\Themes\\harbour.json" });
    await user.click(within(dialog).getByRole("button", { name: "Save to My Themes" }));
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.import_theme")?.args).toEqual({ path: "C:\\Themes\\harbour.json", apply: false }));
  });

  it("My Themes: apply, export and delete (with a confirmation)", async () => {
    state = { ...state, myThemes: [{ id: harbour.id, name: harbour.name, theme: harbour, problem: null }] };
    savedPath.value = "C:\\Exports\\Harbour Night.json";
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    const row = await screen.findByRole("listitem", { name: "Harbour Night" });
    await user.click(within(row).getByRole("button", { name: "Apply" }));
    await user.click(within(row).getByRole("button", { name: "Export Harbour Night" }));
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.export_theme")?.args).toEqual({ id: "harbour-night", path: "C:\\Exports\\Harbour Night.json" }));
    await user.click(within(row).getByRole("button", { name: "Delete Harbour Night" }));
    const confirm = await screen.findByRole("dialog", { name: 'Delete the theme "Harbour Night"?' });
    expect(confirm).toHaveTextContent("Your projects are not affected.");
    await user.click(within(confirm).getByRole("button", { name: "Delete Theme" }));
    await waitFor(() => expect(ops()).toEqual(["appearance.set_theme", "appearance.export_theme", "appearance.delete_theme"]));
  });

  it("a damaged saved theme is listed with the reason and can only be deleted; the fallback is explained", async () => {
    state = {
      ...state,
      fallbackNotice: "Your saved theme could not be used, so OpenFrame Default is shown. Your projects are not affected.",
      myThemes: [{ id: "broken-one", name: "Broken One", theme: null, problem: "Its saved data is damaged." }],
    };
    renderWithClient(<AppearancePanel />);
    expect(await screen.findByText(/OpenFrame Default is shown/)).toBeInTheDocument();
    const row = screen.getByRole("listitem", { name: "Broken One" });
    expect(row).toHaveTextContent("This theme cannot be used. Its saved data is damaged.");
    expect(within(row).queryByRole("button", { name: "Apply" })).toBeNull();
    expect(within(row).queryByRole("button", { name: /Export/ })).toBeNull();
    expect(within(row).getByRole("button", { name: "Delete Broken One" })).toBeEnabled();
  });

  it("the mode is one tab stop that moves with the arrow keys", async () => {
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    const system = await screen.findByRole("radio", { name: "System" });
    expect(system).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveAttribute("tabindex", "-1");
    system.focus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("radio", { name: "Light" })).toHaveFocus();
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.set_mode")?.args).toEqual({ mode: "light" }));
  });

  it("renames a theme and saves with the preview's digest", async () => {
    state = { ...state, myThemes: [{ id: harbour.id, name: harbour.name, theme: harbour, problem: null }] };
    const user = userEvent.setup();
    renderWithClient(<AppearancePanel />);
    await user.click(await screen.findByRole("button", { name: "Rename Harbour Night" }));
    const dialog = await screen.findByRole("dialog", { name: "Rename theme" });
    const input = within(dialog).getByLabelText(/Theme name/);
    await user.clear(input);
    await user.type(input, "Harbour Lamps");
    await user.click(within(dialog).getByRole("button", { name: "Rename" }));
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.rename_theme")?.args).toEqual({ id: "harbour-night", name: "Harbour Lamps" }));

    checkResult = { ...checkResult, digest: "abc123" };
    pickedFile.value = "C:\\Themes\\harbour.json";
    await user.click(screen.getByRole("button", { name: "Import Theme File…" }));
    const imp = await screen.findByRole("dialog", { name: "Import theme" });
    await within(imp).findByText("This theme is readable in Light and Dark.");
    await user.click(within(imp).getByRole("button", { name: "Save and Apply" }));
    await waitFor(() => expect(calls.find((c) => c.op === "appearance.import_theme")?.args).toEqual({ path: "C:\\Themes\\harbour.json", apply: true, expectedDigest: "abc123" }));
  });

  it("uses no em dashes in what it shows", async () => {
    renderWithClient(<AppearancePanel />);
    await screen.findByRole("radiogroup", { name: "Theme" });
    expect(document.body.textContent ?? "").not.toMatch(new RegExp(String.fromCharCode(0x2014)));
  });
});
