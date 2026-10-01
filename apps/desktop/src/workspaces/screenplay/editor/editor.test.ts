import { describe, expect, it, vi } from "vitest";
import { EditorState } from "prosemirror-state";
import {
  applyOps,
  diffSnapshots,
  docFromSnapshot,
  mayAddBlocks,
  snapshotFromDoc,
  snapshotsEqual,
  type Snapshot,
} from "./model";
import { nextOnEnter, onEnterEmpty, tabNext, tabPrev } from "./rules";
import { findInText } from "./find";
import { SyncController } from "./sync";
import { schema } from "./schema";
import type { ScreenplayEditOp } from "../../../ipc/generated/ScreenplayEditOp";

const base: Snapshot = [
  {
    id: "s1",
    heading: "INT. POLICE STATION — NIGHT",
    elements: [
      { id: "e1", type: "action", text: "Arjun enters." },
      { id: "e2", type: "character", text: "MEERA" },
      { id: "e3", type: "dialogue", text: "You said you would\nnever come back." },
    ],
  },
  { id: "s2", heading: "EXT. STREET — DAY", elements: [{ id: "e4", type: "action", text: "Rain." }] },
  { id: "s3", heading: "INT. MORGUE — MORNING", elements: [{ id: "e5", type: "action", text: "Cold." }] },
];

function clone(s: Snapshot): Snapshot {
  return JSON.parse(JSON.stringify(s)) as Snapshot;
}

describe("document mapping", () => {
  it("round-trips through ProseMirror, including line breaks", () => {
    const doc = docFromSnapshot(base);
    expect(doc.firstChild?.type.name).toBe("scene_heading");
    expect(snapshotsEqual(snapshotFromDoc(doc), base)).toBe(true);
  });

  it("always produces a valid document", () => {
    const doc = docFromSnapshot([]);
    expect(() => doc.check()).not.toThrow();
    expect(() => docFromSnapshot(base).check()).not.toThrow();
  });
});

describe("diffSnapshots", () => {
  const check = (next: Snapshot) => {
    const ops = diffSnapshots(base, next);
    expect(snapshotsEqual(applyOps(base, ops), next)).toBe(true);
    return ops;
  };

  it("emits nothing when unchanged", () => {
    expect(diffSnapshots(base, clone(base))).toEqual([]);
  });

  it("typing updates one element", () => {
    const next = clone(base);
    next[0].elements[0].text = "Arjun enters carrying a pistol.";
    expect(check(next)).toEqual([{ op: "updateElement", id: "e1", text: "Arjun enters carrying a pistol." }]);
  });

  it("Enter splits an element and inserts a new one", () => {
    const next = clone(base);
    next[0].elements[0].text = "Arjun";
    next[0].elements.splice(1, 0, { id: "n1", type: "action", text: " enters." });
    const ops = check(next);
    expect(ops.map((o) => o.op)).toEqual(["updateElement", "insertElement"]);
  });

  it("changing a body element into a heading splits the scene", () => {
    const next = clone(base);
    const tail = next[0].elements.splice(1);
    next.splice(1, 0, { id: "n2", heading: "INT. CORRIDOR — NIGHT", elements: tail });
    next[0].elements = [next[0].elements[0]];
    const ops = check(next);
    expect(ops[0]).toMatchObject({ op: "insertScene", id: "n2", index: 1 });
    expect(ops.filter((o) => o.op === "moveElement")).toHaveLength(2);
  });

  it("removing a heading merges scenes and deletes the empty scene", () => {
    const next = clone(base);
    next[0].elements.push(...next[1].elements);
    next.splice(1, 1);
    const ops = check(next);
    expect(ops).toContainEqual({ op: "deleteScene", id: "s2" });
    expect(ops).toContainEqual({ op: "moveElement", id: "e4", sceneId: "s1", index: 3 });
  });

  it("deleting scenes keeps their text with them (recoverable)", () => {
    const next = clone(base).filter((s) => s.id !== "s3");
    const ops = check(next);
    expect(ops).toEqual([{ op: "deleteScene", id: "s3" }]);
  });

  it("reorders scenes and edits headings", () => {
    const next = clone(base);
    const [s3] = next.splice(2, 1);
    next.unshift(s3);
    next[1].heading = "INT. POLICE STATION — DAY";
    const ops = check(next);
    expect(ops).toContainEqual({ op: "moveScene", id: "s3", index: 0 });
    expect(ops).toContainEqual({ op: "updateScene", id: "s1", heading: "INT. POLICE STATION — DAY" });
  });

  it("handles a large mixed edit", () => {
    const next = clone(base);
    next[2].elements.push({ id: "n9", type: "transition", text: "CUT TO:" });
    next[0].elements[1].type = "action";
    next[1].elements.splice(0, 1);
    next.push({ id: "n8", heading: "", elements: [{ id: "n7", type: "action", text: "" }] });
    check(next);
  });
});

describe("element rules", () => {
  it("follows the mock's Enter progression", () => {
    expect(nextOnEnter("scene_heading")).toBe("action");
    expect(nextOnEnter("action")).toBe("character");
    expect(nextOnEnter("character")).toBe("dialogue");
    expect(nextOnEnter("dialogue")).toBe("action");
    expect(nextOnEnter("transition")).toBe("scene_heading");
    expect(onEnterEmpty("action")).toBe("scene_heading");
    expect(onEnterEmpty("character")).toBe("action");
    expect(onEnterEmpty("scene_heading")).toBeNull();
    expect(tabNext("character")).toBe("parenthetical");
    expect(tabPrev("dialogue")).toBe("character");
  });
});

describe("find", () => {
  it("matches like Rust replace_all", () => {
    const t = "The folder. FOLDERS of folder_x and Folder!";
    expect(findInText(t, { query: "folder", matchCase: true, wholeWord: false })).toHaveLength(2);
    expect(findInText(t, { query: "folder", matchCase: false, wholeWord: false })).toHaveLength(4);
    expect(findInText(t, { query: "folder", matchCase: false, wholeWord: true })).toEqual([
      [4, 10],
      [36, 42],
    ]);
    expect(findInText("a (b) c", { query: "(", matchCase: false, wholeWord: false })).toEqual([[2, 3]]);
    expect(findInText("x", { query: "", matchCase: false, wholeWord: false })).toEqual([]);
  });
});

describe("SyncController", () => {
  type Send = (ops: ScreenplayEditOp[]) => Promise<{ seq: number }>;
  function setup(sendImpl?: Send) {
    let current = clone(base);
    const applied: Snapshot[] = [];
    const send = vi.fn<Send>(sendImpl ?? (async () => ({ seq: 2 })));
    const sync = new SyncController(clone(base), 1, {
      send,
      read: () => current,
      apply: (s) => {
        applied.push(s);
        current = clone(s);
      },
      onError: () => "revert",
    });
    return {
      sync,
      send,
      applied,
      edit: (f: (s: Snapshot) => void) => {
        f(current);
        sync.markDirty();
      },
    };
  }

  it("batches edits and ignores its own echo and stale reads", async () => {
    const t = setup();
    t.edit((s) => (s[0].elements[0].text = "A"));
    t.edit((s) => (s[0].elements[0].text = "AB"));
    await t.sync.flush();
    expect(t.send).toHaveBeenCalledTimes(1);
    expect(t.send.mock.calls[0][0]).toEqual([{ op: "updateElement", id: "e1", text: "AB" }]);
    // A stale read (seq 1) must not overwrite the editor.
    t.sync.receiveServer(1, clone(base));
    expect(t.applied).toHaveLength(0);
    // The echo of our own save changes nothing.
    const echo = clone(base);
    echo[0].elements[0].text = "AB";
    t.sync.receiveServer(2, echo);
    expect(t.applied).toHaveLength(0);
    // A newer server state (e.g. after undo) replaces the editor.
    t.sync.receiveServer(2, clone(base));
    expect(t.applied).toHaveLength(1);
  });

  it("never replaces content while local edits are pending", async () => {
    const t = setup();
    t.edit((s) => (s[1].heading = "EXT. ROAD — DAY"));
    t.sync.receiveServer(5, clone(base));
    expect(t.applied).toHaveLength(0);
    await t.sync.flush();
    expect(t.send).toHaveBeenCalledTimes(1);
  });

  it("reverts to Rust's state when a save is rejected", async () => {
    const t = setup(async () => {
      throw new Error("locked");
    });
    t.edit((s) => (s[0].elements[0].text = "Changed"));
    await t.sync.flush();
    expect(t.applied).toHaveLength(1);
    expect(t.applied[0][0].elements[0].text).toBe("Arjun enters.");
  });

  it("schema rejects a document that does not start with a heading", () => {
    const doc = schema.nodes.doc.createChecked(null, [schema.nodes.scene_heading.create({ id: "x" })]);
    const state = EditorState.create({ doc });
    expect(state.doc.childCount).toBe(1);
    expect(() => schema.nodes.doc.createChecked(null, [schema.nodes.action.create({ id: "y" })])).toThrow();
  });
});

describe("block id bookkeeping", () => {
  const state = () => EditorState.create({ schema, doc: docFromSnapshot(base) });
  it("typing never needs the id pass", () => {
    const s = state();
    expect(mayAddBlocks(s.tr.insertText("x", 3))).toBe(false);
    expect(mayAddBlocks(s.tr.delete(3, 4))).toBe(false);
  });
  it("splitting or inserting blocks does", () => {
    const s = state();
    expect(mayAddBlocks(s.tr.split(5))).toBe(true);
    const para = s.doc.child(1);
    expect(mayAddBlocks(s.tr.insert(0, para.copy(para.content)))).toBe(true);
    expect(mayAddBlocks(s.tr.setNodeMarkup(0, undefined, { ...s.doc.child(0).attrs }))).toBe(true);
  });
});
