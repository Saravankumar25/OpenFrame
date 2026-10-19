import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { DoodCell, DoodRow } from "../../../api/schedule";
import { DoodNoteDialog, doodArgs } from "./dood";

// Test fixtures only: the IPC boundary is replaced by a recorder.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let fail = false;

vi.mock("../../../ipc/client", () => ({
  call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    if (fail) throw new Error("The schedule is finalized.");
    return null;
  }),
}));
vi.mock("../../../ipc/query", () => ({ reportError: vi.fn(), useOp: vi.fn() }));
vi.mock("../../../app/toast", () => ({ toast: { undoable: vi.fn(), info: vi.fn(), error: vi.fn() } }));

const cell: DoodCell = { dayId: "d2", code: "T", derivedCode: "", overridden: true, note: "Flies in" };
const row: DoodRow = {
  key: "char:meera",
  number: 2,
  character: "Meera",
  actor: null,
  label: "Meera",
  cells: [cell],
  workDays: 1,
  holdDays: 0,
  travelDays: 1,
  otherDays: 0,
  startLabel: "Shoot Day 1",
  finishLabel: "Shoot Day 1",
};

describe("Day Out of Days", () => {
  beforeEach(() => {
    calls.length = 0;
    fail = false;
  });

  it("range and cast selection are view arguments only", () => {
    expect(doodArgs({ castKeys: null, fromDayId: null, toDayId: null })).toEqual({});
    expect(doodArgs({ castKeys: ["char:a"], fromDayId: "d1", toDayId: "d3" })).toEqual({ castKeys: ["char:a"], fromDayId: "d1", toDayId: "d3" });
  });

  it("edits a correction note and keeps the corrected code", async () => {
    const onClose = vi.fn();
    render(<DoodNoteDialog scheduleId="s1" target={{ row, cell, dayTitle: "Shoot Day 2" }} onClose={onClose} />);
    const box = screen.getByLabelText("Note") as HTMLTextAreaElement;
    expect(box.value).toBe("Flies in");
    fireEvent.change(box, { target: { value: "  Flies in from Mumbai  " } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(calls).toEqual([
      {
        op: "schedule.set_dood_override",
        args: { scheduleId: "s1", castKey: "char:meera", dayId: "d2", code: "T", note: "Flies in from Mumbai" },
      },
    ]);
  });

  it("stays open when saving fails, so nothing typed is lost", async () => {
    fail = true;
    const onClose = vi.fn();
    render(<DoodNoteDialog scheduleId="s1" target={{ row, cell, dayTitle: "Shoot Day 2" }} onClose={onClose} />);
    fireEvent.change(screen.getByLabelText("Note"), { target: { value: "Rehearsal moved" } });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(calls).toHaveLength(1));
    expect(onClose).not.toHaveBeenCalled();
    expect((screen.getByLabelText("Note") as HTMLTextAreaElement).value).toBe("Rehearsal moved");
  });

  it("renders nothing without a cell", () => {
    const { container } = render(<DoodNoteDialog scheduleId="s1" target={null} onClose={() => undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
});
