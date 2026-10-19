import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { CallSheetDetail } from "../../api/schedule";
import { CallSheetEditor } from "./CallSheetEditor";

// Test fixtures only: the IPC boundary is replaced by a recorder and a fixed call sheet.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let detail: CallSheetDetail;

vi.mock("../../ipc/client", () => ({
  call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    return null;
  }),
  inTauri: () => false,
  OpError: class OpError extends Error {
    code = "";
  },
}));
vi.mock("../../ipc/query", () => ({
  reportError: vi.fn(),
  useOp: vi.fn(() => ({ data: detail, isLoading: false, error: null })),
}));
vi.mock("../../app/toast", () => ({ toast: { undoable: vi.fn(), info: vi.fn(), error: vi.fn(), success: vi.fn() } }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("../../features/export/buttons", () => ({ CallSheetExportButton: () => null }));

function fixture(over: Partial<CallSheetDetail> = {}): CallSheetDetail {
  return {
    id: "cs1",
    title: "Call Sheet: Day 4",
    revision: 1,
    status: "Draft",
    editable: true,
    sourceChanged: false,
    stale: null,
    dayId: "d4",
    dayTitle: "Shoot Day 4 · Mon 14 Jun 2027",
    dateLong: "MONDAY 14 JUNE 2027",
    document: {
      version: 2,
      title: "Black Rain",
      header: { company: "", address: "", producer: "", firstAd: "", director: "", contact: "" },
      dayLabel: "SHOOT DAY 4",
      dayOf: "Day 4 of 12",
      date: "2027-06-14",
      crewCall: "",
      times: { generalCall: "", shootingCall: "19:00", breakfast: "", lunch: "20:30", estimatedWrap: "" },
      dayNotes: "",
      locations: [{ key: "l1", name: "Police Station", address: "", meetingPoint: "", parking: "", hospital: "", emergencyContact: "", access: "", notes: "" }],
      cast: [{ key: "arjun", actor: "Karthik Menon", character: "ARJUN", callTime: "", notes: "", castId: "1", status: "", pickup: "", makeup: "", contact: "" }],
      scenes: [
        { key: "L12", number: "12", heading: "INT. POLICE STATION - NIGHT", description: "Meera hands the file", pages: "1 2/8", kind: "", start: "", intExt: "INT", dayNight: "NIGHT", castIds: "1", location: "Police Station", minutes: 90 },
        { key: "m1", number: "", heading: "Meal Break", description: "", pages: "", kind: "Meal", start: "", intExt: "", dayNight: "", castIds: "", location: "", minutes: 45 },
      ],
      practical: { parking: "", meetingPoint: "", travelNotes: "", mealBreak: "", emergencyContact: "", productionNotes: "", toilets: "", access: "", safety: "", specialInstructions: "" },
      optional: { weather: null, sunrise: "", sunset: "", specialNotes: null, attachments: null, referenceImages: null, extras: null, departments: null, advance: null, crew: null, communication: null },
      extraFields: [],
    },
    source: {
      version: 1,
      productionTitle: "Black Rain",
      sourceLabel: "Draft 6",
      dayId: "d4",
      dayNumber: 4,
      dayLabel: "Shoot Day 4",
      date: "2027-06-14",
      dayNotes: null,
      scenes: [],
      cast: [],
      locations: [],
      breaks: [],
      totalDays: 12,
      times: { crewCall: null, shootingStart: null, shootingCall: null, targetWrap: null, estimatedWrap: null, lunch: null },
      rows: [],
      extras: [],
      departments: [],
      advance: null,
    },
    missing: ["crewCall", "locations.0.address"],
    estimatedStarts: ["19:00", "20:30"],
    defaultSections: ["Production header", "Day and times", "Locations", "Scene schedule", "Talent", "Practical notes"],
    optionalSections: [
      { key: "weather", label: "Weather" },
      { key: "extras", label: "Extras" },
    ],
    snapshotId: null,
    finalizedAt: null,
    supersededBy: null,
    previousId: null,
    revisions: [{ id: "cs1", revision: 1, status: "Draft" }],
    updatedAt: 1,
    rev: 1,
    ...over,
  };
}

describe("call sheet editor", () => {
  beforeEach(() => {
    calls.length = 0;
    detail = fixture();
  });

  it("shows the six default sections and hides optional ones that were not added", () => {
    render(<CallSheetEditor id="cs1" />);
    for (const name of ["Production header", "Day and times", "Locations", "Scene schedule", "Talent", "Practical notes"]) {
      expect(screen.getByRole("region", { name })).toBeInTheDocument();
    }
    expect(screen.queryByRole("region", { name: "Weather" })).toBeNull();
    expect(screen.getByRole("button", { name: "Add Section" })).toBeInTheDocument();
    expect(screen.getByText(/2 values read Needs input/)).toBeInTheDocument();
  });

  it("marks values nobody entered as Needs input and never fills them in", () => {
    render(<CallSheetEditor id="cs1" />);
    const crew = screen.getByLabelText("Crew call") as HTMLInputElement;
    expect(crew.value).toBe("");
    expect(crew.placeholder).toBe("Needs input");
    expect(crew).toHaveAttribute("aria-invalid", "true");
    expect((screen.getByLabelText("Address") as HTMLInputElement).placeholder).toBe("Needs input");
  });

  it("shows estimated starts for rows without their own time, including the meal row", () => {
    render(<CallSheetEditor id="cs1" />);
    expect((screen.getByLabelText("Start of scene 12") as HTMLInputElement).placeholder).toBe("about 19:00");
    expect((screen.getByLabelText("Start of Meal Break") as HTMLInputElement).placeholder).toBe("about 20:30");
  });

  it("saves edits to the call sheet only", async () => {
    render(<CallSheetEditor id="cs1" />);
    fireEvent.change(screen.getByLabelText("Crew call"), { target: { value: "18:00" } });
    await waitFor(() => expect(calls.map((c) => c.op)).toEqual(["callsheets.update"]));
    const doc = calls[0].args.document as CallSheetDetail["document"];
    expect(doc.crewCall).toBe("18:00");
    expect(calls[0].args.id).toBe("cs1");
  });

  it("shows an added optional section and lets the user remove it", async () => {
    detail = fixture();
    detail.document.optional.weather = "";
    render(<CallSheetEditor id="cs1" />);
    expect(screen.getByRole("region", { name: "Weather" })).toBeInTheDocument();
    expect((screen.getByLabelText("Sunrise") as HTMLInputElement).placeholder).toBe("Needs input");
    fireEvent.click(screen.getByRole("button", { name: "Remove Weather" }));
    await waitFor(() => expect(calls.map((c) => c.op)).toEqual(["callsheets.update"]));
    expect((calls[0].args.document as CallSheetDetail["document"]).optional.weather).toBeNull();
  });

  it("a Final sheet is read-only and offers a new revision instead of edits", () => {
    detail = fixture({ status: "Final", editable: false, finalizedAt: 1, stale: { areas: ["Scenes"], preparedAt: 1, dayDeleted: false }, sourceChanged: true });
    render(<CallSheetEditor id="cs1" />);
    expect(screen.getByLabelText("Crew call")).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Finalize…" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Add Section" })).toBeNull();
    expect(screen.getAllByRole("button", { name: "Start New Revision" }).length).toBeGreaterThan(0);
    expect(screen.getByText(/The final version never changes/)).toBeInTheDocument();
  });

  it("a draft whose day changed reads Needs refresh and nothing changes until the review", () => {
    detail = fixture({ status: "Needs refresh", sourceChanged: true, stale: { areas: ["Scenes", "Times"], preparedAt: 1, dayDeleted: false } });
    render(<CallSheetEditor id="cs1" />);
    expect(screen.getAllByText("Needs refresh").length).toBeGreaterThan(0);
    expect(screen.getByText(/Scenes, Times/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review Update" })).toBeInTheDocument();
    expect(calls).toEqual([]);
  });
});
