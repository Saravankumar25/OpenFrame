import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

import type { PackageChange } from "../../ipc/generated/PackageChange";
import type { PackageImportSession } from "../../ipc/generated/PackageImportSession";
import {
  PACKAGE_INFO,
  backupFileName,
  defaultSelection,
  groupSelection,
  joinPath,
  packageFilter,
  resultLabel,
  staleDefault,
  stateChip,
  toggleGroup,
} from "./api";
import { packageActions, packageMenuItems, usePackagesUi } from "./index";

function change(id: string, group: string, extra: Partial<PackageChange> = {}): PackageChange {
  return {
    id,
    group,
    groupLabel: group,
    label: id,
    detail: null,
    kind: "comment",
    state: "safe",
    applicable: true,
    selectedByDefault: true,
    copyOnly: false,
    candidates: [],
    ...extra,
  };
}

describe("package naming and filters", () => {
  it("names backups with project, date/time and an optional label (FSD §44.8)", () => {
    expect(backupFileName("BLACK RAIN 2026-09-29 1042", "")).toBe("BLACK RAIN 2026-09-29 1042.ofbackup");
    expect(backupFileName("BLACK RAIN 2026-09-29 1042", " before/rewrite ")).toBe("BLACK RAIN 2026-09-29 1042 before_rewrite.ofbackup");
  });
  it("joins default save paths on Windows and POSIX", () => {
    expect(joinPath("E:\\Backups", "a.ofbackup")).toBe("E:\\Backups\\a.ofbackup");
    expect(joinPath("/home/x/", "a.ofbackup")).toBe("/home/x/a.ofbackup");
  });
  it("keeps each package class distinct", () => {
    const exts = new Set(Object.values(PACKAGE_INFO).map((p) => p.extension));
    expect(exts.size).toBe(Object.keys(PACKAGE_INFO).length);
    expect(packageFilter(["story"], "Story")[0].extensions).toEqual(["ofstory"]);
  });
});

describe("import preview selection", () => {
  const changes = [
    change("a", "comments"),
    change("b", "comments", { selectedByDefault: false }),
    change("c", "text", { kind: "info", applicable: false, state: "review" }),
    change("d", "copy", { copyOnly: true, selectedByDefault: false }),
    change("e", "story.cards", { kind: "update", state: "conflict", selectedByDefault: false }),
  ];
  it("starts from the safe defaults, never copies or conflicts", () => {
    expect([...defaultSelection(changes)]).toEqual(["a"]);
  });
  it("toggles a whole group and reports partial selection", () => {
    const sel = defaultSelection(changes);
    expect(groupSelection(sel, changes, "comments")).toBe("some");
    const all = toggleGroup(sel, changes, "comments", true);
    expect(groupSelection(all, changes, "comments")).toBe("all");
    expect(groupSelection(toggleGroup(all, changes, "comments", false), changes, "comments")).toBe("none");
    // Informational items can't be selected.
    expect(toggleGroup(sel, changes, "text", true).has("c")).toBe(false);
  });
  it("labels states and results with the spec's words", () => {
    expect(stateChip("safe").label).toBe("Safe mapping");
    expect(stateChip("unmapped").label).toBe("Unmapped");
    expect(resultLabel("PartiallyApplied")).toBe("Partially Applied");
    expect(resultLabel("PendingReview")).toBe("Pending Review");
  });
  it("defaults stale packages to comments only (FSD §47.9)", () => {
    const s = { changes } as unknown as PackageImportSession;
    expect(staleDefault(s)).toBe("commentsOnly");
    const none = { changes: [changes[2]] } as unknown as PackageImportSession;
    expect(staleDefault(none)).toBe("record");
  });
});

describe("entry points", () => {
  it("lists the Share / Exchange actions in the UX order and opens dialogs", () => {
    const labels = packageActions.map((a) => a.label);
    expect(labels.slice(0, 6)).toEqual([
      "Export Review Package (script)…",
      "Story Board Package…",
      "Breakdown Package…",
      "Shot List Package…",
      "Schedule Package…",
      "Call Sheet Review Package…",
    ]);
    expect(labels).toContain("Import Exchange Package…");
    expect(labels).toContain("Create Backup…");
    packageActions.find((a) => a.id === "packages.backup")!.run();
    expect(usePackagesUi.getState().dialog).toEqual({ kind: "backup" });
    usePackagesUi.getState().close();
  });
  it("adds menu items only while a project is open", () => {
    expect(packageMenuItems({ projectOpen: false })).toEqual([]);
    const items = packageMenuItems({ projectOpen: true });
    expect(items[0]).toMatchObject({ label: "Share / Exchange", header: true });
    expect(items.some((i) => i.label === "Backup & Portability" && i.header)).toBe(true);
  });
});
