import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import type { ScheduleStripDto } from "../../../api/schedule";
import { MarkerRow, StripRow, markerText } from "./Strip";

const strip: ScheduleStripDto = {
  id: "st1",
  sceneId: "s1",
  lineageId: "L1",
  number: 12,
  heading: "INT. POLICE STATION - NIGHT",
  intExt: "INT",
  dayNight: "N",
  ieLabel: "INT·N",
  stripClass: "in",
  locationName: null,
  locations: [],
  synopsis: "Meera hands the file",
  cast: [{ key: "char:a", character: "Arjun", actor: "Karthik Menon", initials: "AR" }],
  pageEighths: 14,
  pagesLabel: "1 6/8",
  pagesOverridden: false,
  estimatedMinutes: 90,
  dayId: null,
  sourceState: "Changed",
  changeKinds: ["Heading changed"],
  missingFromSource: false,
  notes: null,
  rev: 1,
};

describe("StripRow", () => {
  it("shows colour class, missing location text and script flag (never colour-only)", () => {
    const onOpen = vi.fn();
    render(<StripRow strip={strip} onOpen={onOpen} />);
    const el = screen.getByRole("button", { name: /Scene 12, INT·N, location missing/ });
    expect(el.className).toContain("strip in");
    expect(screen.getByText("No location")).toBeInTheDocument();
    expect(screen.getByText("Script changed")).toBeInTheDocument();
    expect(screen.getByText("1h 30m")).toBeInTheDocument();
    fireEvent.keyDown(el, { key: "Enter" });
    expect(onOpen).toHaveBeenCalledOnce();
  });

  it("renders break markers with time and duration", () => {
    const m = { id: "m", dayId: "d", markerType: "Meal", label: "Meal Break", atTime: "20:30", durationMinutes: 45, notes: null, rev: 1 };
    expect(markerText(m)).toBe("MEAL BREAK — 20:30 (45m)");
    render(<MarkerRow marker={m} />);
    expect(screen.getByRole("button", { name: "Break: MEAL BREAK — 20:30 (45m)" })).toBeInTheDocument();
  });
});
