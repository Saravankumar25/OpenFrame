import { describe, expect, it } from "vitest";
import type { AiChangeSetDto } from "../ipc/generated/AiChangeSetDto";
import type { AiStatusDto } from "../ipc/generated/AiStatusDto";
import {
  DEGRADED_TEXT,
  PREPARING_TEXT,
  changeCount,
  changeSetActions,
  changeSetStateText,
  confidenceLabel,
  defaultScope,
  formatBytes,
  indexNotice,
  indexStateOf,
  panelMode,
  progressPercent,
  scopeArgs,
  scopeOptions,
  sourceChips,
  stepsSummary,
  toRoute,
} from "./model";

function cs(state: string, extra: Partial<AiChangeSetDto> = {}): AiChangeSetDto {
  return {
    id: "c1", title: "Proposed Scene Card", summary: "", state, validationState: "Valid", rows: [], exclusions: [], affectedModules: ["Story"],
    targets: [], operationCount: 1, staleReason: null, errorMessage: null, createdAt: 1, approvedAt: null, appliedAt: null, appliedOperations: 0,
    partCount: 1, requiresConfirmation: false,
    ...extra,
  };
}

function status(p: Partial<AiStatusDto>): AiStatusDto {
  return { installed: false, mode: "Off", runtimeState: "NotInstalled", runtimeMessage: null, install: null, localOnly: true, updateAvailable: false, installedBytes: 0, ...p };
}

describe("scope resolution", () => {
  it("inherits the current scene in the Screenplay", () => {
    const route = { workspace: "screenplay" as const, params: { sceneId: "s1", draftId: "d1" } };
    expect(defaultScope(route)).toBe("CurrentScene");
    expect(scopeArgs("CurrentScene", route)).toEqual({ kind: "CurrentScene", draftId: null, sceneId: "s1", selection: [], shootingDayId: null, callSheetId: null });
    expect(scopeArgs("SpecificDraft", route).draftId).toBe("d1");
  });

  it("offers only scopes that have a target", () => {
    const opts = scopeOptions({ workspace: "home" });
    expect(opts).not.toContain("CurrentScene");
    expect(opts).not.toContain("CurrentSelection");
    expect(opts[opts.length - 1]).toBe("WholeProject");
    expect(defaultScope({ workspace: "home" })).toBe("WholeProject");
    expect(defaultScope({ workspace: "story" })).toBe("StoryBoard");
  });

  it("uses the Idea Vault selection in the Idea Vault", () => {
    const route = { workspace: "vault" as const, params: { itemId: "v1" } };
    expect(defaultScope(route)).toBe("IdeaVaultSelection");
    expect(scopeArgs("IdeaVaultSelection", route).selection).toEqual([{ id: "v1" }]);
    // Selection ids are never sent for unrelated scopes (minimal context).
    expect(scopeArgs("WholeProject", route).selection).toEqual([]);
  });
});

describe("navigation targets", () => {
  it("maps known workspaces and ignores unknown ones", () => {
    expect(toRoute({ workspace: "screenplay", params: { sceneId: "s2" } })).toEqual({ workspace: "screenplay", params: { sceneId: "s2" } });
    expect(toRoute({ workspace: "somewhere-else", params: {} })).toBeNull();
    expect(toRoute(null)).toBeNull();
  });
});

describe("download display", () => {
  it("formats sizes in simple units", () => {
    expect(formatBytes(2_497_280_256)).toBe("2.3 GB");
    expect(formatBytes(19_164_803)).toBe("19 MB");
    expect(formatBytes(12 * 1024 ** 3)).toBe("12 GB");
  });

  it("computes progress and panel mode", () => {
    const install = { taskId: "t", phase: "downloading", bytesDone: 50, bytesTotal: 200, message: "", error: null, active: true };
    expect(progressPercent(install)).toBe(25);
    expect(progressPercent({ ...install, bytesTotal: 0 })).toBeNull();
    expect(panelMode(undefined, false)).toBe("unavailable");
    // Not installed → the one-click setup view straight away.
    expect(panelMode(status({}), false)).toBe("setup");
    expect(panelMode(status({ install }), false)).toBe("installing");
    expect(panelMode(status({ install: { ...install, active: false, phase: "paused" } }), false)).toBe("setup");
    expect(panelMode(status({ installed: true, mode: "AI Ready" }), false)).toBe("ready");
    expect(panelMode(status({ installed: true, mode: "AI Ready", updateAvailable: true }), true)).toBe("setup");
  });
});

describe("change set review", () => {
  it("only pending proposals can be applied; stale ones must be re-checked", () => {
    expect(changeSetActions(cs("Pending"))).toEqual({ apply: true, reject: true, recheck: false });
    expect(changeSetActions(cs("Stale"))).toEqual({ apply: false, reject: true, recheck: true });
    expect(changeSetActions(cs("Conflict")).apply).toBe(false);
    expect(changeSetActions(cs("Applied"))).toEqual({ apply: false, reject: false, recheck: false });
    expect(changeSetActions(cs("Rejected")).apply).toBe(false);
  });

  it("describes outcomes truthfully", () => {
    expect(changeSetStateText(cs("Applied", { appliedOperations: 3 }))).toBe("Applied 3 changes. Use Undo to reverse it.");
    expect(changeSetStateText(cs("Rejected"))).toBe("Rejected. Nothing was changed.");
    expect(changeSetStateText(cs("Failed", { errorMessage: "X. Nothing was changed." }))).toBe("X. Nothing was changed.");
    expect(changeSetStateText(cs("Pending"))).toBeNull();
  });

  it("labels exact facts distinctly from inferred text", () => {
    expect(confidenceLabel("Exact")?.text).toBe("Exact · from project data");
    expect(confidenceLabel("Inferred")?.tone).toBe("b");
    expect(confidenceLabel(null)).toBeNull();
  });

  it("counts changes plainly", () => {
    expect(changeCount(1)).toBe("1 change");
    expect(changeCount(4)).toBe("4 changes");
  });
});

describe("agent answers", () => {
  it("turns provenance into chips that open their source when they can", () => {
    const chips = sourceChips([
      { kind: "Scope", label: "Whole Project" },
      { kind: "Character", label: "Ravi", nav: { workspace: "story", params: { characterId: "c1" } } },
      { kind: "Scene", label: "Scene 3", nav: { workspace: "not-a-workspace", params: {} } },
    ]);
    expect(chips.map((c) => c.text)).toEqual(["Whole Project", "Character: Ravi", "Scene: Scene 3"]);
    expect(chips[0].route).toBeNull();
    expect(chips[1].route).toEqual({ workspace: "story", params: { characterId: "c1" } });
    expect(chips[2].route).toBeNull();
  });

  it("summarises multi-step answers without exposing reasoning", () => {
    expect(stepsSummary([{ index: 1, tool: "count_scenes", label: "Count scenes", status: "Succeeded" }])).toBeNull();
    expect(
      stepsSummary([
        { index: 1, tool: "search_project", label: "Search project", status: "Succeeded" },
        { index: 2, tool: "open_scene", label: "Open scene", status: "Succeeded" },
      ]),
    ).toBe("2 steps: Search project · Open scene");
  });

  it("shows the project-context notice only while it is being rebuilt", () => {
    expect(indexStateOf("Rebuilding")).toBe("Rebuilding");
    expect(indexStateOf({ state: "Stale", documents: 3 })).toBe("Stale");
    expect(indexStateOf(42)).toBeNull();
    expect(indexNotice("Rebuilding")).toEqual({ preparing: PREPARING_TEXT, degraded: DEGRADED_TEXT });
    expect(indexNotice("Stale")).not.toBeNull();
    for (const quiet of ["Current", "Updating", "Failed", "Unavailable", null]) expect(indexNotice(quiet)).toBeNull();
    expect(PREPARING_TEXT).toBe("Preparing project context…");
    expect(DEGRADED_TEXT).toBe("AI can still answer some questions while project context is being prepared.");
  });
});
