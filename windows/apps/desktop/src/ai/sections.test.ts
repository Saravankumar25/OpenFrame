import { describe, expect, it } from "vitest";
import type { AiSectionTreeDto } from "../ipc/generated/AiSectionTreeDto";
import { pruneToTree, sendHint, setInstance, useAiPicks, type Pick } from "./sections";

const leaf = (id: string, picker = "none") => ({ id, label: id, pathLabel: id, picker, note: null });
const tree: AiSectionTreeDto = {
  branches: [
    { id: "screenplay", label: "Screenplay", leaves: [leaf("screenplay/scenes", "screenplay")] },
    { id: "production", label: "Production", leaves: [leaf("production/shot-lists", "scene"), leaf("production/schedule", "shootingDay"), leaf("call-sheets/call-sheets", "callSheet")] },
  ],
  screenplays: [
    {
      id: "sp1",
      title: "ONE",
      drafts: [
        { id: "d1", name: "Draft 1", current: true },
        { id: "d2", name: "Draft 2", current: false },
      ],
      scenesDraftId: "d1",
      scenes: [{ id: "sc1", number: 1, heading: "INT. A" }],
    },
    { id: "sp2", title: "TWO", drafts: [{ id: "d3", name: "Draft 1", current: true }], scenesDraftId: "d3", scenes: [{ id: "sc3", number: 1, heading: "EXT. B" }] },
  ],
  days: [{ id: "day1", label: "Shoot Day 1", date: null }],
  callSheets: [{ id: "cs1", title: "Day 1", dayId: "day1" }],
};

describe("pruneToTree", () => {
  it("keeps every choice the tree still offers", () => {
    const picks: Pick[] = [
      { leaf: "screenplay/scenes", screenplayId: "sp1", draftId: "d2", sceneId: "sc1" },
      { leaf: "production/schedule", shootingDayId: "day1" },
      { leaf: "call-sheets/call-sheets", callSheetId: "cs1" },
    ];
    expect(pruneToTree(tree, picks)).toBe(picks);
  });

  it("drops the ids of a deleted screenplay, draft, scene, day or call sheet but keeps the section", () => {
    const picks: Pick[] = [
      { leaf: "screenplay/scenes", screenplayId: "gone", draftId: "d1", sceneId: "sc1" },
      { leaf: "production/shot-lists", sceneId: "gone" },
      { leaf: "production/schedule", shootingDayId: "gone" },
      { leaf: "call-sheets/call-sheets", callSheetId: "gone" },
    ];
    expect(pruneToTree(tree, picks)).toEqual([
      { leaf: "screenplay/scenes" },
      { leaf: "production/shot-lists" },
      { leaf: "production/schedule" },
      { leaf: "call-sheets/call-sheets" },
    ]);
    expect(pruneToTree(tree, [{ leaf: "screenplay/scenes", screenplayId: "sp1", draftId: "gone", sceneId: "sc1" }])).toEqual([
      { leaf: "screenplay/scenes", screenplayId: "sp1" },
    ]);
  });

  it("drops a section the tree no longer offers, and does nothing without a tree", () => {
    const picks: Pick[] = [{ leaf: "gone/leaf" }, { leaf: "screenplay/scenes" }];
    expect(pruneToTree(tree, picks)).toEqual([{ leaf: "screenplay/scenes" }]);
    expect(pruneToTree(undefined, picks)).toBe(picks);
  });
});

describe("setInstance", () => {
  const picks: Pick[] = [
    { leaf: "screenplay/scenes", screenplayId: "sp1", sceneId: "sc1" },
    { leaf: "production/shot-lists", sceneId: "sc1" },
  ];

  it("clears the scene of every pick when the screenplay changes", () => {
    expect(setInstance(picks, "screenplay/scenes", "screenplayId", "sp2")).toEqual([
      { leaf: "screenplay/scenes", screenplayId: "sp2" },
      { leaf: "production/shot-lists" },
    ]);
  });

  it("clears the scene of every pick when the draft changes", () => {
    expect(setInstance(picks, "screenplay/scenes", "draftId", "d2")).toEqual([
      { leaf: "screenplay/scenes", screenplayId: "sp1", draftId: "d2" },
      { leaf: "production/shot-lists" },
    ]);
  });
});

describe("sendHint", () => {
  it("does not let a question go while sections are picked but the tree is not there", () => {
    expect(sendHint(undefined, [{ leaf: "screenplay/scenes" }])).toBeTruthy();
    expect(sendHint(tree, [{ leaf: "production/schedule" }])).toBeNull();
  });
});

describe("picks follow the project", () => {
  it("empties when another project opens and is kept for the same one", () => {
    useAiPicks.setState({ picks: [], projectId: null });
    useAiPicks.getState().bind("a");
    useAiPicks.getState().set([{ leaf: "screenplay/scenes" }]);
    useAiPicks.getState().bind("a");
    expect(useAiPicks.getState().picks).toHaveLength(1);
    useAiPicks.getState().bind(null);
    expect(useAiPicks.getState().picks).toHaveLength(1);
    useAiPicks.getState().bind("b");
    expect(useAiPicks.getState().picks).toEqual([]);
  });
});
