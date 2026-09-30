// Typed helpers for the production module (Production Source, Breakdown,
// Catalog, Locations, Cast & Crew). All truth lives in Rust; these are
// operation names, dependency tables for query invalidation and small
// display helpers shared by the Breakdown and Production workspaces.

import { convertFileSrc } from "@tauri-apps/api/core";
import { inTauri } from "../ipc/client";
import { useOp } from "../ipc/query";
import type { AssetInfo } from "../ipc/generated/AssetInfo";
import type { BreakdownCategory } from "../ipc/generated/BreakdownCategory";
import type { CatalogStatus } from "../ipc/generated/CatalogStatus";
import type { ProductionLocationStatus } from "../ipc/generated/ProductionLocationStatus";
import type { Member } from "../ipc/generated/Member";
import type { ProductionSourceInfo } from "../ipc/generated/ProductionSourceInfo";
import type { ChipTone } from "../design-system";

/** Breakdown categories, exactly in FSD §26.3 order. */
export const CATEGORIES: readonly BreakdownCategory[] = [
  "Cast",
  "Extras / Background",
  "Location / Set",
  "Props",
  "Wardrobe",
  "Vehicles",
  "Hair / Makeup",
  "Special Effects",
  "VFX",
  "Sound",
  "Animals",
];

/** Short singular labels used in catalog tables ("Prop", "Vehicle"). */
export const CATEGORY_SHORT: Record<BreakdownCategory, string> = {
  Cast: "Cast",
  "Extras / Background": "Extras",
  "Location / Set": "Location",
  Props: "Prop",
  Wardrobe: "Wardrobe",
  Vehicles: "Vehicle",
  "Hair / Makeup": "Hair / Makeup",
  "Special Effects": "Special Effect",
  VFX: "VFX",
  Sound: "Sound",
  Animals: "Animal",
};

export const CATALOG_STATUSES: readonly CatalogStatus[] = ["Required", "Searching", "Shortlisted", "Confirmed", "Not Required"];
export const LOCATION_STATUSES: readonly ProductionLocationStatus[] = ["Idea", "Shortlisted", "Confirmed", "Rejected"];

export function catalogStatusTone(s: CatalogStatus | null | undefined): ChipTone {
  switch (s) {
    case "Confirmed":
      return "g";
    case "Searching":
      return "y";
    case "Shortlisted":
      return "a";
    case "Not Required":
      return "default";
    default:
      return "b";
  }
}

export function locationStatusTone(s: ProductionLocationStatus): ChipTone {
  switch (s) {
    case "Confirmed":
      return "g";
    case "Shortlisted":
      return "a";
    case "Rejected":
      return "r";
    default:
      return "default";
  }
}

/** Tables read by production queries (drives invalidation from Rust `dataChanged`). */
export const PROD_TABLES = [
  "production_source",
  "production_scene_state",
  "breakdown_element",
  "breakdown_dismissed",
  "catalog_item",
  "catalog_alias",
  "location",
  "location_photo",
  "cast_member",
  "crew_member",
  "screenplay",
  "screenplay_draft",
  "screenplay_scene",
  "screenplay_element",
  "story_character",
  "asset",
];

export function useProductionSource() {
  return useOp<ProductionSourceInfo | null>("production.source", {}, PROD_TABLES);
}

/** Whether the local user may edit (UI hint only — Rust enforces permissions). */
export function useCanEdit(): boolean {
  const members = useOp<Member[]>("project.members", {}, ["project_member"]);
  const me = members.data?.find((m) => m.isYou);
  return !me || me.role === "Owner" || me.role === "Editor";
}

/** Displayable URL for a stored image, or null when it is unavailable. */
export function assetUrl(a: AssetInfo | null | undefined): string | null {
  if (!a || !a.available || !a.path) return null;
  return inTauri() ? convertFileSrc(a.path) : a.path;
}

/** "Scene 12" / "Scene removed from current source". */
export function sceneLabel(number: string | null | undefined): string {
  return number ? `Scene ${number}` : "Removed scene";
}

export const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];
