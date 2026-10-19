// Settings > Writing (SCRIPT-020): spell checking switch, main and second language from the
// installed dictionaries, and the personal dictionary. The IPC boundary is replaced.

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WritingSettings, languageName } from "./Writing";

let settings = { enabled: true, mainLanguage: "en-US", secondLanguage: null as string | null };
let installed = ["en-US", "en-GB", "hi-IN"];
let adapterAvailable = true;
let canEdit = true;
let serviceError = false;
let words: string[] = [];
const calls: { op: string; args: Record<string, unknown> }[] = [];

vi.mock("../../ipc/client", () => {
  class OpError extends Error {
    code: string;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string, args: Record<string, unknown>) => {
      calls.push({ op, args });
      switch (op) {
        case "writing.proofing":
          return { settings, installed, adapterAvailable, canEdit, serviceError };
        case "writing.set_proofing":
          if (args.enabled !== undefined) settings = { ...settings, enabled: args.enabled as boolean };
          if (args.mainLanguage) settings = { ...settings, mainLanguage: args.mainLanguage as string };
          if (args.secondLanguage !== undefined) settings = { ...settings, secondLanguage: (args.secondLanguage as string) || null };
          return settings;
        case "writing.dictionary":
          return { words };
        case "writing.dictionary_add": {
          const w = String(args.word).trim();
          if (w.includes(" ")) throw new OpError("validation.word", "Add one word at a time, without spaces.");
          words = [...words, w].sort();
          return { words };
        }
        case "writing.dictionary_remove":
          words = words.filter((x) => x !== args.word);
          return { words };
      }
      return null;
    }),
  };
});

beforeEach(() => {
  settings = { enabled: true, mainLanguage: "en-US", secondLanguage: null };
  installed = ["en-US", "en-GB", "hi-IN"];
  adapterAvailable = true;
  canEdit = true;
  serviceError = false;
  words = [];
  calls.length = 0;
});

const spelling = () => screen.getByRole("region", { name: "Spelling" });

describe("Settings > Writing", () => {
  it("shows the switch and the languages installed on this computer", async () => {
    render(<WritingSettings />);
    const box = await screen.findByRole("checkbox", { name: "Check spelling as I write" });
    expect(box).toBeChecked();
    const main = screen.getByRole("combobox", { name: "Main language" });
    expect(main).toHaveValue("en-US");
    expect(within(main).getAllByRole("option").map((o) => o.textContent)).toEqual([languageName("en-GB"), languageName("en-US"), languageName("hi-IN")].sort());
    expect(within(spelling()).getByText(/Telugu or Hindi on most computers, are never underlined/)).toBeInTheDocument();
  });

  it("turns spelling off and chooses a second language, each as one project change", async () => {
    const user = userEvent.setup();
    render(<WritingSettings />);
    await user.click(await screen.findByRole("checkbox", { name: "Check spelling as I write" }));
    expect(calls.at(-1)).toEqual({ op: "writing.set_proofing", args: { enabled: false } });
    await user.selectOptions(screen.getByRole("combobox", { name: "Second language" }), "hi-IN");
    expect(calls.at(-1)).toEqual({ op: "writing.set_proofing", args: { secondLanguage: "hi-IN" } });
    expect(screen.getByRole("combobox", { name: "Second language" })).toHaveValue("hi-IN");
    // The main language is not offered as the second one.
    expect(within(screen.getByRole("combobox", { name: "Second language" })).queryByRole("option", { name: languageName("en-US") })).toBeNull();
    await user.selectOptions(screen.getByRole("combobox", { name: "Second language" }), "");
    expect(calls.at(-1)).toEqual({ op: "writing.set_proofing", args: { secondLanguage: "" } });
  });

  it("says plainly when a chosen language is not installed here", async () => {
    settings = { enabled: true, mainLanguage: "te-IN", secondLanguage: null };
    render(<WritingSettings />);
    expect(await screen.findByText(/Telugu \(India\) is not installed on this computer/)).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Telugu (India), not installed on this computer" })).toBeInTheDocument();
  });

  it("explains a computer without dictionaries and one without the OpenFrame adapter", async () => {
    installed = [];
    const { unmount } = render(<WritingSettings />);
    expect(await screen.findByText(/No spelling dictionaries are installed/)).toBeInTheDocument();
    unmount();
    adapterAvailable = false;
    render(<WritingSettings />);
    expect(await screen.findByText(/uses the system's own spell checking/)).toBeInTheDocument();
  });

  it("a spelling service that does not answer is not reported as missing dictionaries", async () => {
    installed = [];
    serviceError = true;
    render(<WritingSettings />);
    expect(await screen.findByText(/Spell checking is not available right now/)).toBeInTheDocument();
    expect(screen.queryByText(/No spelling dictionaries are installed/)).toBeNull();
  });

  it("read-only roles see the settings but cannot change them", async () => {
    canEdit = false;
    render(<WritingSettings />);
    expect(await screen.findByRole("checkbox", { name: "Check spelling as I write" })).toBeDisabled();
    expect(screen.getByText(/these settings can't be changed here/)).toBeInTheDocument();
  });

  it("adds and removes personal dictionary words, with Unicode and a clear error", async () => {
    const user = userEvent.setup();
    render(<WritingSettings />);
    const dict = await screen.findByRole("region", { name: "Personal dictionary" });
    expect(within(dict).getByText("Your dictionary is empty.")).toBeInTheDocument();
    await user.type(within(dict).getByRole("textbox", { name: "New word" }), "Kothapeta{Enter}");
    await user.type(within(dict).getByRole("textbox", { name: "New word" }), "నమస్తే{Enter}");
    expect(within(dict).getByRole("list", { name: "Words in your dictionary" })).toHaveTextContent("Kothapeta");
    expect(within(dict).getByText("2 words")).toBeInTheDocument();
    await user.type(within(dict).getByRole("textbox", { name: "New word" }), "two words{Enter}");
    expect(within(dict).getByRole("alert")).toHaveTextContent("Add one word at a time");
    await user.click(within(dict).getByRole("button", { name: "Remove Kothapeta" }));
    expect(calls.at(-1)).toEqual({ op: "writing.dictionary_remove", args: { word: "Kothapeta" } });
    expect(within(dict).getByText("1 word")).toBeInTheDocument();
  });

  it("uses no em dashes in what it shows", async () => {
    render(<WritingSettings />);
    await screen.findByRole("checkbox", { name: "Check spelling as I write" });
    expect(document.body.textContent ?? "").not.toMatch(new RegExp(String.fromCharCode(0x2014)));
  });
});
