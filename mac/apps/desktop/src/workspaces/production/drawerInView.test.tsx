// A side panel must stay in view when a row far down a long list is selected (owner issue list,
// 2026-10-24). The panel is absolutely positioned, so it follows its nearest positioned ancestor. If
// that ancestor is the box that scrolls, the panel scrolls away with the list and opens off screen.
// jsdom has no layout, so this checks the structure that decides it: the panel's containing block
// must be an ancestor of the scroll box, never the scroll box itself or something inside it.

import { render, screen, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import ProductionWorkspace from "./index";
import { useNav } from "../../app/stores";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

const n = (i: number) => String(i).padStart(2, "0");
const notes = { access: null, parking: null, noise: null, power: null, permission: null, toilets: null, facilities: null, travel: null, general: null };
const catalog = Array.from({ length: 60 }, (_, i) => ({
  id: "c" + i, category: "Props", name: "Item " + n(i), description: null, notes: null, contact: null, contactHidden: false, status: "Searching", image: null,
  archived: false, characterId: null, locationId: null, aliases: [], usedIn: [], usedInLabel: "Not used yet", rev: 1, updatedAt: 0,
}));
const locations = Array.from({ length: 40 }, (_, i) => ({
  id: "l" + i, name: "Place " + n(i), address: null, contact: null, nearestHospital: null, emergencyContact: null, contactHidden: false, status: "Idea", notes,
  availability: [], photos: [], scenes: [], sceneCount: 0, replacement: null, archived: false, rev: 1, updatedAt: 0,
}));
const cast = Array.from({ length: 40 }, (_, i) => ({
  id: "m" + i, personName: "Actor " + n(i), characterId: null, characterName: null, isPrimary: true, contact: null, contactHidden: false, photo: null,
  notes: null, availabilityNotes: null, availability: [], archived: false, scenes: [], rev: 1,
}));
const crew = Array.from({ length: 40 }, (_, i) => ({
  id: "w" + i, personName: "Crew " + n(i), role: "Grip", department: "Grip", contact: null, contactHidden: false, notes: null, availabilityNotes: null, availability: [], archived: false, rev: 1,
}));

const data: Record<string, unknown> = {
  "catalog.list": catalog,
  "catalog.find_matches": [],
  "locations.list": locations,
  "cast.list": { members: cast, characters: [] },
  "crew.list": crew,
  "project.members": [],
};

const pos = (e: HTMLElement) => e.style.position;
const scrolls = (e: HTMLElement) => e.style.overflow === "auto" || e.style.overflow === "scroll";

function expectInView(drawer: HTMLElement) {
  const scroller = screen.getByTestId("production-scroll");
  // Nearest positioned ancestor of the panel (the CSS class .content is relative; the page marks it inline).
  let box: HTMLElement | null = drawer.parentElement;
  while (box && !["relative", "absolute", "fixed", "sticky"].includes(pos(box))) box = box.parentElement;
  expect(box).not.toBeNull();
  expect(scrolls(scroller)).toBe(true);
  expect(pos(scroller)).toBe("");
  expect(scroller.contains(box)).toBe(false);
  expect(box!.contains(scroller)).toBe(true);
  expect(scrolls(box!)).toBe(false);
}

function open(tab: string) {
  invoke.mockImplementation(async (_cmd: string, a?: { op?: string }) => data[a?.op ?? ""] ?? null);
  useNav.getState().go({ workspace: "production", sub: tab });
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(
    <QueryClientProvider client={qc}>
      <ProductionWorkspace />
    </QueryClientProvider>,
  );
}

describe("side panels stay in view in a long list", () => {
  beforeEach(() => invoke.mockReset());

  it("Catalog: the panel for a low row is outside the scrolling list", async () => {
    open("catalog");
    fireEvent.click(await screen.findByText("Item 55"));
    expectInView(await screen.findByRole("complementary", { name: "Item 55" }));
  });

  it("Locations: the panel for a low card is outside the scrolling list", async () => {
    open("locations");
    fireEvent.click(await screen.findByText("Place 38"));
    expectInView(await screen.findByRole("complementary", { name: "Place 38" }));
  });

  it("Cast & Crew: the panels for a low cast row and a low crew row are outside the scrolling list", async () => {
    open("cast-crew");
    fireEvent.click(await screen.findByText("Actor 37"));
    expectInView(await screen.findByRole("complementary", { name: "Actor 37" }));
    fireEvent.click(screen.getByRole("radio", { name: "Crew" }));
    fireEvent.click(await screen.findByText("Crew 36"));
    expectInView(await screen.findByRole("complementary", { name: "Crew 36" }));
  });
});
