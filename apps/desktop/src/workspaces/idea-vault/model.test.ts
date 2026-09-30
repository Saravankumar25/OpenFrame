import { describe, expect, it } from "vitest";
import type { VaultItemDto } from "../../ipc/generated/VaultItemDto";
import type { VaultFolderDto } from "../../ipc/generated/VaultFolderDto";
import {
  buildRows,
  bytesToBase64,
  folderPath,
  folderTree,
  formatDuration,
  hostOf,
  metaLine,
  nextSelection,
  parseTags,
  relativeDay,
  sameView,
} from "./model";

function item(p: Partial<VaultItemDto> & { id: string }): VaultItemDto {
  return {
    store: "project",
    itemType: "note",
    title: null,
    displayName: "Untitled note",
    body: null,
    caption: null,
    url: null,
    sourceText: null,
    asset: null,
    folderId: null,
    collectionIds: [],
    tags: [],
    pinned: false,
    sourceGlobalItemId: null,
    createdAt: Date.now(),
    updatedAt: Date.now(),
    rev: 1,
    matchReason: null,
    ...p,
  };
}

const folder = (id: string, parentId: string | null, name = id): VaultFolderDto => ({ id, name, parentId, itemCount: 0, rev: 1 });

describe("idea vault model", () => {
  it("formats durations and days", () => {
    expect(formatDuration(42_000)).toBe("0:42");
    expect(formatDuration(3_723_000)).toBe("1:02:03");
    expect(formatDuration(null)).toBeNull();
    const now = new Date(2026, 8, 30, 12).getTime();
    expect(relativeDay(new Date(2026, 8, 30, 1).getTime(), now)).toBe("today");
    expect(relativeDay(new Date(2026, 8, 29, 23).getTime(), now)).toBe("yesterday");
    expect(relativeDay(new Date(2026, 8, 12).getTime(), now)).toBe("12 Sep");
    expect(relativeDay(new Date(2024, 8, 12).getTime(), now)).toBe("12 Sep 2024");
  });

  it("builds meta lines like the mocks", () => {
    const now = Date.now();
    const voice = item({ id: "v", itemType: "voice", createdAt: now, asset: { id: "a", storageMode: "managed", originalName: "v.webm", mediaType: "audio/webm", byteSize: 1, width: null, height: null, durationMs: 42_000, path: null, available: true } });
    expect(metaLine(voice, [], now)).toBe("Voice note · 0:42 · today");
    const img = item({ id: "i", itemType: "image", collectionIds: ["c1"], tags: ["rain"] });
    expect(metaLine(img, [{ id: "c1", name: "Visual References" }])).toBe("Image · Visual References");
    expect(metaLine(item({ id: "n", tags: ["core concept"] }), [])).toBe("Note · core concept");
  });

  it("splits pinned items and chunks rows by columns", () => {
    const items = [item({ id: "1", pinned: true }), item({ id: "2" }), item({ id: "3" }), item({ id: "4" })];
    const rows = buildRows(items, 2, { splitPinned: true });
    expect(rows.map((r) => r.kind)).toEqual(["header", "items", "header", "items", "items"]);
    expect(rows[0]).toMatchObject({ label: "Pinned" });
    expect(rows[2]).toMatchObject({ label: "Everything else" });
    // Every item appears exactly once, whichever view builds the rows.
    const flat = rows.flatMap((r) => (r.kind === "items" ? r.items.map((i) => i.id) : []));
    expect(flat.sort()).toEqual(["1", "2", "3", "4"]);
    const noSplit = buildRows(items, 3);
    expect(noSplit.map((r) => r.kind)).toEqual(["items", "items"]);
    const withFolders = buildRows(items.slice(0, 1), 3, { folders: [folder("a", null), folder("b", null)] });
    expect(withFolders.map((r) => r.kind)).toEqual(["folders", "header", "items"]);
  });

  it("walks folder paths and trees", () => {
    const fs = [folder("a", null, "Scenes"), folder("b", "a", "Stations"), folder("c", "b", "Night"), folder("z", "missing")];
    expect(folderPath(fs, "c").map((f) => f.name)).toEqual(["Scenes", "Stations", "Night"]);
    expect(folderTree(fs).map((t) => `${t.depth}:${t.folder.id}`)).toEqual(["0:a", "1:b", "2:c", "0:z"]);
  });

  it("selects with ctrl and shift", () => {
    const order = ["a", "b", "c", "d"];
    expect([...nextSelection(new Set(), order, "b", null, {})]).toEqual(["b"]);
    expect([...nextSelection(new Set(["b"]), order, "d", "b", { shift: true })]).toEqual(["b", "c", "d"]);
    expect([...nextSelection(new Set(["b"]), order, "b", "b", { ctrl: true })]).toEqual([]);
  });

  it("parses tags, hosts, views and base64", () => {
    expect(parseTags("#rain, Rain,  night   bus \n")).toEqual(["rain", "night bus"]);
    expect(hostOf("https://archive-mag.example/brutalist")).toBe("archive-mag.example/brutalist");
    expect(sameView({ kind: "tag", tag: "Rain" }, { kind: "tag", tag: "rain" })).toBe(true);
    expect(sameView({ kind: "folder", folderId: null }, { kind: "all" })).toBe(false);
    expect(bytesToBase64(new Uint8Array([104, 105]))).toBe("aGk=");
  });
});
