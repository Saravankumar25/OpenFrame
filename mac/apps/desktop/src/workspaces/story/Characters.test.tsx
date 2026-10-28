// Character relationship maps: the character's own map shows only their relationships, the whole
// film map is reachable on its own, and each map has a written list for screen readers.

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { StoryCharacterDto } from "../../ipc/generated/StoryCharacterDto";
import type { StoryRelationshipDto } from "../../ipc/generated/StoryRelationshipDto";

const ch = (id: string, name: string): StoryCharacterDto => ({ id, name, roleLabel: null, description: null, notes: null, archived: false, episodeId: null, image: null, sceneCount: 1, rev: 1 });
const chars = [ch("a", "Arjun"), ch("b", "Bala"), ch("c", "Chitra"), ch("d", "Dev")];
const rel = (id: string, f: StoryCharacterDto, t: StoryCharacterDto, type: string): StoryRelationshipDto => ({ id, fromCharacterId: f.id, fromName: f.name, toCharacterId: t.id, toName: t.name, relationshipType: type, note: null, rev: 1 });
const rels = [rel("1", chars[0], chars[1], "friend of"), rel("2", chars[2], chars[0], "mother of"), rel("3", chars[1], chars[3], "rival of")];

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null) }));
vi.mock("../../app/toast", () => ({ toast: { success: vi.fn(), error: vi.fn(), info: vi.fn(), undoable: vi.fn() } }));
vi.mock("../../api/story", () => ({
  story: {},
  useCharacters: () => ({ data: chars, isLoading: false }),
  useCharacter: (id: string) => {
    const character = chars.find((c) => c.id === id)!;
    return { data: { character, scenes: [], relationships: rels.filter((r) => r.fromCharacterId === id || r.toCharacterId === id), linkedCards: [] } };
  },
  useRelationships: () => ({ data: rels }),
  useStoryBoard: () => ({ data: null }),
}));

import { CharactersView } from "./Characters";

const show = () => render(<CharactersView episodeId={null} readOnly={false} wantNew={false} onNewHandled={() => undefined} />);
const map = () => screen.getByRole("img", { name: /relationship/i });
const names = () => within(map().closest("section")!).getAllByRole("button", { name: /^Show relationships of/ }).map((b) => b.getAttribute("aria-label"));

describe("character relationship maps", () => {
  it("opens one character's own map with only their direct relationships", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Relationship Map" }));
    expect(screen.getByRole("heading", { name: "ARJUN: relationships" })).toBeTruthy();
    // Arjun's own relationships: friend of Bala, mother of Arjun (Chitra). Dev is Bala's, not Arjun's.
    expect(names().sort()).toEqual(["Show relationships of Bala", "Show relationships of Chitra"]);
    expect(map().getAttribute("aria-label")).toContain("2 relationships");
    const list = within(map().closest("section")!).getAllByRole("listitem");
    expect(list).toHaveLength(2);
    expect(screen.queryByText(/Dev/i)).toBeNull();
  });

  it("opens the whole-film map on its own from the character list", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Whole film map" }));
    expect(screen.getByRole("heading", { name: "Character relationships" })).toBeTruthy();
    expect(map().getAttribute("aria-label")).toContain("3 relationships");
    expect(names()).toHaveLength(4);
  });

  it("moves between a character's map and the whole film, and back", () => {
    show();
    fireEvent.click(screen.getByRole("button", { name: "Relationship Map" }));
    fireEvent.click(screen.getByRole("button", { name: "Whole film map" }));
    expect(screen.getByRole("heading", { name: "Character relationships" })).toBeTruthy();
    fireEvent.keyDown(screen.getByRole("button", { name: "Show relationships of Dev" }), { key: "Enter" });
    expect(screen.getByRole("heading", { name: "DEV: relationships" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Back to Characters" }));
    expect(screen.getByRole("region", { name: "Characters" })).toBeTruthy();
  });
});
