import { describe, expect, it } from "vitest";
import {
  canKeepAsProjectFile,
  defaultExportFileName,
  failureStage,
  fileName,
  importModes,
  importReady,
  previewBanner,
  sceneStatusChip,
  targetLabel,
} from "./model";

const targets = (over: Partial<Parameters<typeof importModes>[0]> = {}) => ({
  canCreateScreenplay: false,
  existingScreenplays: [{ id: "s1", title: "Black Rain", episodeTitle: null, draftCount: 2, currentDraftName: "Draft 2" }],
  episodic: false,
  episodes: [],
  ...over,
});

describe("interchange model", () => {
  it("suggests Windows-safe export file names", () => {
    expect(defaultExportFileName("Black Rain", "Draft 6 — Shooting Draft", "pdf")).toBe("Black Rain - Draft 6 - Shooting Draft.pdf");
    expect(defaultExportFileName("A/B: C?", "", "fdx")).toBe("A B C.fdx");
    expect(defaultExportFileName("  ", "  ", "fountain")).toBe("Screenplay.fountain");
    expect(defaultExportFileName("x".repeat(300), "", "docx").length).toBeLessThanOrEqual(105);
  });

  it("reads file names from Windows and POSIX paths", () => {
    expect(fileName("C:\\Scripts\\blackrain_v3.pdf")).toBe("blackrain_v3.pdf");
    expect(fileName("/home/a/b.fdx")).toBe("b.fdx");
  });

  it("maps scene statuses to the mock's chips", () => {
    expect(sceneStatusChip("ok")).toEqual({ tone: "g", label: "OK" });
    expect(sceneStatusChip("uncertain_heading").label).toBe("Uncertain heading");
    expect(sceneStatusChip("potentially_empty").tone).toBe("y");
  });

  it("never presents uncertain parsing as certain", () => {
    const base = { warnings: [], confidence: "high", needsReview: false, sourceFormat: "fdx", attentionCount: 0 };
    expect(previewBanner(base).tone).toBe("ok");
    const low = previewBanner({
      ...base,
      confidence: "low",
      sourceFormat: "pdf",
      warnings: [
        { code: "a", message: "m", level: "attention", sceneIndex: 2 },
        { code: "b", message: "n", level: "info", sceneIndex: null },
      ],
    });
    expect(low.tone).toBe("warn");
    expect(low.title).toBe("2 warnings.");
    expect(low.text).toContain("This PDF was interpreted with limited certainty");
  });

  it("names the failing stage and whether the file can still be kept", () => {
    expect(failureStage("import.pdf_not_screenplay", "file")).toBe("Reading text from the PDF");
    expect(failureStage("import.not_a_screenplay", "pasted")).toBe("Interpreting the screenplay");
    expect(canKeepAsProjectFile("import.pdf_not_screenplay")).toBe(true);
    expect(canKeepAsProjectFile("import.empty_source")).toBe(false);
    expect(canKeepAsProjectFile("validation.invalid_input")).toBe(false);
  });

  it("offers New Screenplay beside the existing ones and a named New Draft (FSD-SCRIPT-044, ADR-0012)", () => {
    expect(importModes(targets()).map((m) => m.value)).toEqual(["new_draft"]);
    expect(importModes(targets({ canCreateScreenplay: true, existingScreenplays: [] })).map((m) => m.value)).toEqual(["new_screenplay"]);
    const both = importModes(targets({ canCreateScreenplay: true }));
    expect(both.map((m) => m.value)).toEqual(["new_screenplay", "new_draft"]);
    expect(both[1].label).toBe("New Draft in “Black Rain”");
    expect(importReady(targets(), "new_draft", "", "")).toBe(true);
    expect(importReady(targets(), "new_screenplay", "", "")).toBe(false);
    // Several screenplays: the one that receives the draft must be chosen.
    const several = targets({
      canCreateScreenplay: true,
      existingScreenplays: [
        { id: "s1", title: "Black Rain", episodeTitle: null, draftCount: 2, currentDraftName: "Draft 2" },
        { id: "s2", title: "Black Rain (short)", episodeTitle: null, draftCount: 1, currentDraftName: "Draft 1" },
      ],
    });
    expect(importModes(several)[1].label).toBe("New Draft in…");
    expect(importReady(several, "new_draft", "", "")).toBe(false);
    expect(importReady(several, "new_draft", "s2", "")).toBe(true);
    expect(importReady(several, "new_screenplay", "", "")).toBe(true);
    const episodic = targets({ episodic: true, canCreateScreenplay: true, episodes: [{ id: "e2", title: "The Flood" }] });
    expect(importReady(episodic, "new_screenplay", "", "")).toBe(false);
    expect(importReady(episodic, "new_screenplay", "", "e2")).toBe(true);
    expect(targetLabel({ title: "Monsoon", episodeTitle: "Pilot" })).toBe("Pilot · Monsoon");
  });
});
