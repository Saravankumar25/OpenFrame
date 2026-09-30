import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { BreakdownSceneRow } from "../../ipc/generated/BreakdownSceneRow";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { BreakdownElementDto } from "../../ipc/generated/BreakdownElementDto";
import { SceneList, filterScenes } from "./SceneList";
import { ElementsPanel } from "./ElementsPanel";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

function row(n: number, over: Partial<BreakdownSceneRow> = {}): BreakdownSceneRow {
  return {
    sceneId: `s${n}`,
    lineageId: `l${n}`,
    number: String(n),
    heading: `INT. ROOM ${n} - DAY`,
    omitted: false,
    confirmedCount: 0,
    suggestedCount: 0,
    complete: false,
    needsReview: false,
    headingChanged: false,
    textChanged: false,
    needsBreakdown: false,
    changeMessage: null,
    changedAt: null,
    ...over,
  };
}

function el(over: Partial<BreakdownElementDto>): BreakdownElementDto {
  return {
    id: "e1",
    sceneId: "s1",
    category: "Props",
    name: "Pistol",
    displayName: "Pistol",
    catalogItemId: "c1",
    catalogStatus: "Required",
    catalogArchived: false,
    catalogRemoved: false,
    state: "Confirmed",
    origin: "manual",
    evidence: null,
    confidence: null,
    notes: null,
    spanElementId: null,
    spanStart: null,
    spanEnd: null,
    archived: false,
    matchKind: null,
    matches: [],
    rev: 1,
    ...over,
  };
}

function wrap(ui: ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>);
}

describe("scene list", () => {
  const rows = [row(1, { complete: true, confirmedCount: 2 }), row(2, { suggestedCount: 3 }), row(3, { needsReview: true, confirmedCount: 1 }), row(4, { needsBreakdown: true })];

  it("filters by derived status (FSD-BREAKDOWN-015)", () => {
    expect(filterScenes(rows, "complete").map((r) => r.number)).toEqual(["1"]);
    expect(filterScenes(rows, "incomplete").map((r) => r.number)).toEqual(["2", "3", "4"]);
    expect(filterScenes(rows, "suggested").map((r) => r.number)).toEqual(["2"]);
    expect(filterScenes(rows, "needsReview").map((r) => r.number)).toEqual(["3"]);
    expect(filterScenes(rows, "needsBreakdown").map((r) => r.number)).toEqual(["4"]);
    expect(filterScenes(rows, "all")).toHaveLength(4);
  });

  it("shows derived numbers, headings and status chips", () => {
    const onSelect = vi.fn();
    render(
      <SceneList rows={rows} total={4} selected="s1" filter="all" onFilter={() => {}} onSelect={onSelect} reviewCount={1} historicalCount={0} onOpenReview={() => {}} onOpenHistory={() => {}} />,
    );
    expect(screen.getByText("INT. ROOM 2 - DAY")).toBeInTheDocument();
    expect(screen.getByText("3 suggested")).toBeInTheDocument();
    expect(screen.getByText("Needs review")).toBeInTheDocument();
    expect(screen.getByText("Needs breakdown")).toBeInTheDocument();
    expect(screen.getByText("Complete")).toBeInTheDocument();
    expect(screen.getByRole("option", { selected: true })).toHaveTextContent("INT. ROOM 1 - DAY");
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });
    expect(onSelect).toHaveBeenCalledWith("s2");
    expect(screen.getByText("Needs Review (1)")).toBeInTheDocument();
  });
});

describe("elements panel", () => {
  const detail: BreakdownSceneDetail = {
    sceneId: "s1",
    lineageId: "l1",
    number: "12",
    heading: "INT. POLICE STATION — NIGHT",
    inSource: true,
    draftName: "Draft 6",
    sceneCount: 42,
    facts: { intExt: "INT", setName: "Police Station", timeOfDay: "NIGHT" },
    script: [],
    groups: [
      { category: "Props", confirmed: [el({})], suggestions: [] },
      {
        category: "Wardrobe",
        confirmed: [],
        suggestions: [el({ id: "g1", category: "Wardrobe", name: "Coat", displayName: "Coat", state: "Suggested", catalogItemId: null, catalogStatus: null, evidence: "from his coat.", confidence: "Likely", matchKind: "none" })],
      },
    ],
    confirmedCount: 1,
    suggestionCount: 1,
    complete: false,
    needsReview: false,
    headingChanged: false,
    textChanged: false,
    needsBreakdown: false,
    changeMessage: null,
  };

  it("shows only categories with content and marks suggestions distinctly", () => {
    wrap(<ElementsPanel detail={detail} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} />);
    expect(screen.getByText("Scene 12 breakdown")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Props" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Cast" })).toBeNull();
    const sug = screen.getByText("Coat — suggested").closest(".bd-el");
    expect(sug).toHaveClass("sug");
    expect(screen.getByText("matched: “from his coat.”")).toBeInTheDocument();
    expect(screen.getByText("Suggested elements — 1")).toBeInTheDocument();
  });

  it("rejecting a suggestion calls the backend (no local truth)", async () => {
    invoke.mockResolvedValue({ count: 1 });
    wrap(<ElementsPanel detail={detail} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Reject Coat" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "breakdown.reject", args: { ids: ["g1"] } }));
  });

  it("read-only users see no editing actions", () => {
    wrap(<ElementsPanel detail={detail} canEdit={false} hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} />);
    expect(screen.queryByRole("button", { name: "Accept" })).toBeNull();
    expect(screen.queryByRole("button", { name: /Add Element/ })).toBeNull();
  });
});
