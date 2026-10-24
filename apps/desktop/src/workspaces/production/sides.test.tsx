// Sides, "Selected cast": scenes without the selected cast are shown as left out, the preview and the
// export use only the scenes that stay, an empty result explains itself, and All cast restores.

import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import SidesTab from "./tabs/sides";
import { useNav } from "../../app/stores";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

const arjun = { key: "char:arjun", character: "ARJUN", actor: "Ravi", initials: "A" };
const meera = { key: "char:meera", character: "MEERA", actor: "Anu", initials: "M" };
const strip = (id: string, number: number, heading: string, cast: unknown[]) => ({ id, sceneId: "sc" + id, number, heading, cast, locations: [] });
const strips = [strip("s1", 1, "INT. KITCHEN - DAY", [arjun, meera]), strip("s2", 2, "EXT. ROAD - NIGHT", [meera]), strip("s3", 3, "INT. HALL - DAY", [arjun])];
const day = {
  id: "d1", title: "Shoot Day 1", isOffDay: false,
  items: strips.map((s) => ({ kind: "strip", strip: s, marker: null })),
  summary: { cast: [arjun, meera], locations: [] },
};

const sidesContent = { header: "H", productionTitle: "P", dayLabel: "Shoot Day 1", dateLabel: null, sourceLabel: "S", showDraftName: false, cover: null, scenes: [{ sceneId: "sc1", number: "1", heading: "INT. KITCHEN - DAY", elements: [] }] };

function backend() {
  invoke.mockImplementation(async (_cmd: string, a?: { op?: string }) => {
    switch (a?.op) {
      case "schedule.get":
        return { schedule: { id: "sch" }, days: [day], unscheduled: [] };
      case "sides.list":
        return [];
      case "sides.preview":
        return sidesContent;
      default:
        return null;
    }
  });
}
const previewCalls = () => invoke.mock.calls.filter((c) => (c[1] as { op?: string })?.op === "sides.preview").map((c) => (c[1] as { args: { stripIds: string[]; castKeys: string[] | null } }).args);
const box = (name: RegExp) => screen.getByRole("checkbox", { name }) as HTMLInputElement;

function show() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={qc}>
      <SidesTab />
    </QueryClientProvider>,
  );
}

describe("sides for selected cast", () => {
  beforeEach(() => {
    invoke.mockReset();
    backend();
    useNav.getState().go({ workspace: "production", sub: "sides" });
  });

  it("marks scenes without the selected cast as left out and previews only the others", async () => {
    show();
    await screen.findByText("1: INT. KITCHEN - DAY", { exact: false });
    expect(box(/KITCHEN/).checked && box(/ROAD/).checked && box(/HALL/).checked).toBe(true);
    fireEvent.click(screen.getByRole("radio", { name: "Selected cast" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Ravi \(ARJUN\) · 2 scenes/ }));
    // Arjun is in scenes 1 and 3; scene 2 (road) is left out, unticked and disabled, with the reason.
    expect(box(/ROAD/).checked).toBe(false);
    expect(box(/ROAD/).disabled).toBe(true);
    expect(screen.getByText(/Left out: no selected cast in this scene/)).toBeInTheDocument();
    expect(box(/KITCHEN/).checked && box(/HALL/).checked).toBe(true);
    expect(screen.getByRole("status")).toHaveTextContent("2 of 3 scenes have the selected cast");
    await waitFor(() => expect(previewCalls().at(-1)).toMatchObject({ stripIds: ["s1", "s3"], castKeys: ["char:arjun"] }));
  });

  it("explains an empty result and restores everything with All cast", async () => {
    show();
    await screen.findByText("1: INT. KITCHEN - DAY", { exact: false });
    // Untick the two scenes Arjun is in, then pick Arjun: nothing is left.
    fireEvent.click(box(/KITCHEN/));
    fireEvent.click(box(/HALL/));
    fireEvent.click(screen.getByRole("radio", { name: "Selected cast" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /Ravi \(ARJUN\)/ }));
    expect(screen.getByText(/All the scenes with the selected cast are unticked/)).toBeInTheDocument();
    // Back to All cast: the scene the user left ticked (road) is still the only one.
    fireEvent.click(screen.getByRole("radio", { name: "All cast" }));
    expect(box(/ROAD/).checked).toBe(true);
    expect(box(/KITCHEN/).checked).toBe(false);
    await waitFor(() => expect(previewCalls().at(-1)).toMatchObject({ stripIds: ["s2"], castKeys: null }));
  });

  it("says so when no scene has the selected cast", async () => {
    const lonely = { key: "char:zed", character: "ZED", actor: null, initials: "Z" };
    day.summary.cast = [arjun, meera, lonely];
    show();
    await screen.findByText("1: INT. KITCHEN - DAY", { exact: false });
    fireEvent.click(screen.getByRole("radio", { name: "Selected cast" }));
    fireEvent.click(screen.getByRole("checkbox", { name: /ZED · 0 scenes/ }));
    expect(box(/KITCHEN/).disabled && box(/ROAD/).disabled && box(/HALL/).disabled).toBe(true);
    expect(within(screen.getByRole("status")).getByText(/No scene on this day has the selected cast/)).toBeInTheDocument();
    expect(screen.getAllByText(/no sides to show/i).length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Save Sides" })).toBeDisabled();
  });
});
