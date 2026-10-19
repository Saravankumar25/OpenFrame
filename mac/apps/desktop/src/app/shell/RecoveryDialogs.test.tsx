import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { RecoveryOffer, describeRecovery } from "./RecoveryDialogs";
import type { ProjectSummary } from "../../ipc/generated/ProjectSummary";
import type { RecoveryState } from "../../ipc/generated/RecoveryState";

// Test fixtures only: the IPC boundary is replaced; the dialog under test is real.
vi.mock("../../ipc/client", () => ({
  call: vi.fn(async () => null),
  inTauri: () => false,
  onAppEvent: () => Promise.resolve(() => undefined),
}));

const project = { id: "p1", title: "Railway" } as unknown as ProjectSummary;
// 16:29 and 16:30 today, the times observed in evidence row M0-28.
const today = new Date();
today.setHours(16, 29, 0, 0);
const autosave = today.getTime();
const saved = autosave + 60_000;

function state(over: Partial<RecoveryState>): RecoveryState {
  return {
    offer: { previousSessionStartedAt: autosave - 3_600_000, lastAutosaveAt: autosave, hasCheckpoint: true, checkpointAt: saved },
    hasCheckpoint: true,
    checkpointAt: saved,
    lastChangeAt: saved,
    changesSinceCheckpoint: 0,
    ...over,
  };
}

describe("crash recovery offer (CORE-004)", () => {
  it("never calls the recovery state more recent when it is not newer than the last save", () => {
    render(<RecoveryOffer project={project} state={state({})} />);
    expect(screen.queryByText(/more recent/i)).toBeNull();
    expect(screen.getByText(/matches your last confirmed save/i)).toBeTruthy();
    // The recovery card shows the latest recorded time, never an older autosave time.
    const cards = screen.getAllByText(/Today, /);
    expect(cards).toHaveLength(2);
    expect(cards[0].textContent).toBe(cards[1].textContent);
    expect(screen.getByRole("dialog").textContent).not.toMatch(/newer/);
  });

  it("calls the recovery state more recent only when it holds newer changes", () => {
    const later = saved + 120_000;
    render(<RecoveryOffer project={project} state={state({ lastChangeAt: later, changesSinceCheckpoint: 3 })} />);
    expect(screen.getByText(/more recent automatic recovery state/i)).toBeTruthy();
    expect(screen.getByText("3 changes newer")).toBeTruthy();
  });

  it("works without a confirmed save", () => {
    render(
      <RecoveryOffer
        project={project}
        state={state({ hasCheckpoint: false, checkpointAt: null, changesSinceCheckpoint: null, offer: { previousSessionStartedAt: 1, lastAutosaveAt: autosave, hasCheckpoint: false, checkpointAt: null } })}
      />,
    );
    expect(screen.getByText("No confirmed save yet")).toBeTruthy();
    expect(screen.queryByText(/more recent/i)).toBeNull();
  });
});

describe("describeRecovery", () => {
  it("uses the later of the autosave time and the last change time", () => {
    expect(describeRecovery(state({})).recoveryAt).toBe(saved);
    expect(describeRecovery(state({ lastChangeAt: null })).recoveryAt).toBe(autosave);
  });

  it("is newer when changes were recorded after the save, or the time is later", () => {
    expect(describeRecovery(state({})).newer).toBe(false);
    expect(describeRecovery(state({ changesSinceCheckpoint: 2 })).newer).toBe(true);
    expect(describeRecovery(state({ changesSinceCheckpoint: null, lastChangeAt: saved + 1 })).newer).toBe(true);
    expect(describeRecovery(state({ hasCheckpoint: false, checkpointAt: null, changesSinceCheckpoint: null })).newer).toBe(true);
  });
});
