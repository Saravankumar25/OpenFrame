import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { BreakdownSceneRow } from "../../ipc/generated/BreakdownSceneRow";
import type { BreakdownSceneDetail } from "../../ipc/generated/BreakdownSceneDetail";
import type { BreakdownElementDto } from "../../ipc/generated/BreakdownElementDto";
import type { BreakdownCategoryStyles } from "../../ipc/generated/BreakdownCategoryStyles";
import type { BreakdownFilterCounts } from "../../ipc/generated/BreakdownFilterCounts";
import { SceneList } from "./SceneList";
import { ElementsPanel } from "./ElementsPanel";
import { ScriptPane } from "./ScriptPane";
import { segmentText } from "./highlight";

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
    categories: [],
    ...over,
  };
}

const CATS = ["Cast", "Extras / Background", "Location / Set", "Props", "Wardrobe", "Vehicles", "Hair / Makeup", "Special Effects", "VFX", "Sound", "Animals"] as const;
const STYLES: BreakdownCategoryStyles = {
  categories: CATS.map((c, i) => ({
    category: c,
    label: c,
    tooltip: `${c}: what this category covers.`,
    color: `t${i}`,
    colorHex: `#10${String(i).padStart(2, "0")}aa`,
    icon: ["user", "users", "map-pin", "box", "shirt", "car", "scissors", "flame", "sparkles", "volume", "paw"][i],
    customized: false,
  })),
  palette: [],
  icons: [],
};

/** Answer IPC calls per operation; anything else resolves to `fallback`. */
function routeInvoke(fallback: unknown = undefined) {
  invoke.mockImplementation(async (_cmd: string, payload: { op: string }) => (payload?.op === "breakdown.categories" ? STYLES : fallback));
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
  const rows = [row(1, { complete: true, confirmedCount: 2, categories: ["Cast", "Props"] }), row(2, { suggestedCount: 3 }), row(3, { needsReview: true, confirmedCount: 1 }), row(4, { needsBreakdown: true })];
  const counts: BreakdownFilterCounts = { all: 4, unreviewed: 2, suggested: 1, missing: 2, needsReview: 1, complete: 1, incomplete: 3 };

  it("shows derived numbers, headings, status chips and category marks", () => {
    routeInvoke();
    const onSelect = vi.fn();
    wrap(
      <SceneList rows={rows} total={4} counts={counts} selected="s1" filter="all" category={null} onFilter={() => {}} onCategory={() => {}} onSelect={onSelect} reviewCount={1} historicalCount={0} onOpenReview={() => {}} onOpenHistory={() => {}} />,
    );
    expect(screen.getByText("INT. ROOM 2 - DAY")).toBeInTheDocument();
    expect(screen.getByText("3 suggested")).toBeInTheDocument();
    expect(screen.getByText("Needs review")).toBeInTheDocument();
    expect(screen.getByText("Needs breakdown")).toBeInTheDocument();
    expect(screen.getByText("Complete")).toBeInTheDocument();
    expect(screen.getByText("1 of 4 complete")).toBeInTheDocument();
    expect(screen.getByRole("option", { selected: true })).toHaveTextContent("INT. ROOM 1 - DAY");
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });
    expect(onSelect).toHaveBeenCalledWith("s2");
    expect(screen.getByText("Needs Review (1)")).toBeInTheDocument();
  });

  it("shows how many scenes a filter keeps out of the whole source", () => {
    routeInvoke();
    wrap(
      <SceneList rows={rows.slice(1, 3)} total={4} counts={counts} selected={null} filter="unreviewed" category="Props" onFilter={() => {}} onCategory={() => {}} onSelect={() => {}} reviewCount={1} historicalCount={0} onOpenReview={() => {}} onOpenHistory={() => {}} />,
    );
    expect(screen.getByText("2 of 4")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Filter scenes: Only unreviewed, Props" })).toBeInTheDocument();
  });
});

describe("elements panel", () => {
  const detail: BreakdownSceneDetail = {
    sceneId: "s1",
    lineageId: "l1",
    number: "12",
    heading: "INT. POLICE STATION - NIGHT",
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
    wrap(<ElementsPanel detail={detail} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} onShowSource={() => {}} />);
    expect(screen.getByText("Scene 12 breakdown")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Props" })).toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Cast" })).toBeNull();
    const sug = screen.getByText("Coat (suggested)").closest(".bd-el");
    expect(sug).toHaveClass("sug");
    expect(screen.getByText("matched: “from his coat.”")).toBeInTheDocument();
    expect(screen.getByText("Suggested elements: 1")).toBeInTheDocument();
  });

  it("rejecting a suggestion calls the backend (no local truth)", async () => {
    routeInvoke({ count: 1 });
    wrap(<ElementsPanel detail={detail} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} onShowSource={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Reject Coat" }));
    await waitFor(() => expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "breakdown.reject", args: { ids: ["g1"] } }));
  });

  it("read-only users see no editing actions", () => {
    wrap(<ElementsPanel detail={detail} canEdit={false} hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} onShowSource={() => {}} />);
    expect(screen.queryByRole("button", { name: "Accept" })).toBeNull();
    expect(screen.queryByRole("button", { name: /Add Element/ })).toBeNull();
    expect(screen.queryByRole("region", { name: "Quick reuse" })).toBeNull();
  });

  it("accessibility: every category shows colour together with an icon, its text and a tooltip", async () => {
    routeInvoke();
    const all: BreakdownSceneDetail = {
      ...detail,
      groups: CATS.map((c, i) => ({ category: c, confirmed: [el({ id: `e${i}`, category: c, name: `Item ${i}`, displayName: `Item ${i}` })], suggestions: [] })),
    };
    wrap(<ElementsPanel detail={all} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} onShowSource={() => {}} />);
    await waitFor(() => expect(document.querySelector('.catmark[title="Cast: what this category covers."]')).not.toBeNull());
    for (const [i, c] of CATS.entries()) {
      const region = screen.getByRole("region", { name: c });
      const mark = region.querySelector(".catmark") as HTMLElement;
      expect(mark).toHaveAttribute("title", `${c}: what this category covers.`);
      expect(mark.querySelector(".cat-label")).toHaveTextContent(c);
      const icon = mark.querySelector("svg");
      expect(icon).not.toBeNull();
      expect(icon).toHaveAttribute("aria-hidden", "true");
      expect(mark.querySelector(".cat-dot")).toHaveAttribute("aria-hidden");
      expect(mark.style.getPropertyValue("--cat")).toBe(`#10${String(i).padStart(2, "0")}aa`);
    }
  });

  it("offers Show in script for elements with a source phrase", async () => {
    routeInvoke();
    const onShow = vi.fn();
    const d: BreakdownSceneDetail = { ...detail, groups: [{ category: "Props", confirmed: [el({ evidence: "pistol" })], suggestions: [] }] };
    wrap(<ElementsPanel detail={d} canEdit hasSelection={false} onAdd={() => {}} onTagSelection={() => {}} onReview={() => {}} onEdit={() => {}} onChanges={() => {}} onNotes={() => {}} onShowSource={onShow} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Actions for Pistol" }));
    await user.click(await screen.findByRole("menuitem", { name: "Show in script" }));
    expect(onShow).toHaveBeenCalled();
  });
});

describe("script pane", () => {
  it("marks the jumped-to source phrase ahead of the tag it overlaps", () => {
    const segs = segmentText("Arjun enters carrying a pistol.", [
      { start: 24, end: 30, kind: "tag", label: "Props: Pistol", category: "Props" },
      { start: 24, end: 30, kind: "jump", label: "Source phrase: pistol" },
    ]);
    expect(segs.map((x) => x.mark?.kind ?? null)).toEqual([null, "jump", null]);
  });

  it("highlights the source phrase line in the reading pane", () => {
    routeInvoke();
    const d = {
      sceneId: "s1",
      lineageId: "l1",
      number: "12",
      heading: "INT. POLICE STATION - NIGHT",
      inSource: true,
      draftName: "Draft 6",
      sceneCount: 1,
      facts: { intExt: "INT", setName: "Police Station", timeOfDay: "NIGHT" },
      script: [{ elementId: "x1", elementType: "action", text: "Arjun enters carrying a pistol." }],
      groups: [],
      confirmedCount: 0,
      suggestionCount: 0,
      complete: false,
      needsReview: false,
      headingChanged: false,
      textChanged: false,
      needsBreakdown: false,
      changeMessage: null,
    } as BreakdownSceneDetail;
    wrap(<ScriptPane detail={d} canEdit onTag={() => {}} onSelection={() => {}} jump={{ sceneId: "s1", status: "moved", phrase: "pistol", elementId: "x1", start: 24, end: 30 }} />);
    const mark = document.querySelector("mark.jump");
    expect(mark).toHaveTextContent("pistol");
    expect(mark).toHaveAttribute("title", "Source phrase: pistol");
  });
});

describe("saved scene filter", () => {
  afterEach(() => window.localStorage.clear());

  it("comes back after a restart and is per user view state", async () => {
    const { readFilter, writeFilter } = await import("./filterPref");
    writeFilter({ filter: "missing", category: "Props" });
    expect(readFilter()).toEqual({ filter: "missing", category: "Props" });
  });

  it("an old or damaged entry falls back to all scenes instead of breaking the list", async () => {
    const { readFilter } = await import("./filterPref");
    for (const raw of ["{not json", JSON.stringify({ filter: "everything" }), JSON.stringify(null), JSON.stringify({ filter: "suggested", category: "Weapons" })]) {
      window.localStorage.setItem("openframe.breakdown.sceneFilter", raw);
      const v = readFilter();
      expect(["all", "suggested"]).toContain(v.filter);
      expect(v.category).toBeNull();
    }
  });
});
