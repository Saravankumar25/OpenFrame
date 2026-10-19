// Breakdown category identity (depth spec section 23): every category shows its colour
// together with an icon, its text label and an accessible tooltip, so colour is never the
// only signal. Styles come from Rust (`breakdown.categories`); Advanced settings may change
// the colour and the icon, never the label.

import type { CSSProperties, ReactNode } from "react";
import { Box, Camera, Car, Flame, MapPin, Palette, PawPrint, Scissors, Shirt, Sparkles, Star, Tag, User, Users, Volume2, Wrench, type LucideIcon } from "lucide-react";
import { useOp } from "../../ipc/query";
import "./categoryStyle.css";
import type { BreakdownCategory } from "../../ipc/generated/BreakdownCategory";
import type { BreakdownCategoryStyles } from "../../ipc/generated/BreakdownCategoryStyles";
import type { BreakdownCategoryStyleDto } from "../../ipc/generated/BreakdownCategoryStyleDto";

export const CATEGORY_STYLE_TABLES = ["breakdown_category_style"];

/** Icon keys stored by Rust (`categories.rs` ICONS) mapped to the icon set. */
export const CATEGORY_ICONS: Record<string, LucideIcon> = {
  user: User,
  users: Users,
  "map-pin": MapPin,
  box: Box,
  shirt: Shirt,
  car: Car,
  scissors: Scissors,
  flame: Flame,
  sparkles: Sparkles,
  volume: Volume2,
  paw: PawPrint,
  star: Star,
  camera: Camera,
  palette: Palette,
  wrench: Wrench,
  tag: Tag,
};

/** Neutral look until the styles have loaded; the defaults live only in Rust. */
const NEUTRAL = { icon: "tag", hex: "var(--muted)" };

export type CategoryLook = { category: BreakdownCategory; label: string; tooltip: string; hex: string; icon: LucideIcon };

export function lookOf(category: BreakdownCategory, styles?: BreakdownCategoryStyleDto[] | null): CategoryLook {
  const s = styles?.find((x) => x.category === category);
  return {
    category,
    label: s?.label ?? category,
    tooltip: s?.tooltip ?? category,
    hex: s?.colorHex ?? NEUTRAL.hex,
    icon: CATEGORY_ICONS[s?.icon ?? NEUTRAL.icon] ?? Tag,
  };
}

export function useCategoryStyles() {
  return useOp<BreakdownCategoryStyles>("breakdown.categories", {}, CATEGORY_STYLE_TABLES);
}

/** Lookup function bound to the current styles. */
export function useCategoryLook(): (c: BreakdownCategory) => CategoryLook {
  const styles = useCategoryStyles().data?.categories;
  return (c) => lookOf(c, styles);
}

/** Colour swatch + icon (+ label). The tooltip and the accessible name carry the description. */
export function CategoryMark({ look, label, size = 13, children }: { look: CategoryLook; label?: string | false; size?: number; children?: ReactNode }) {
  const Icon = look.icon;
  const text = label === false ? null : (label ?? look.label);
  return (
    <span className="catmark" title={look.tooltip} style={{ "--cat": look.hex } as CSSProperties} data-category={look.category}>
      <span className="cat-dot" aria-hidden />
      <Icon size={size} aria-hidden className="cat-icon" />
      {text ? <span className="cat-label">{text}</span> : <span className="sr-only">{look.label}</span>}
      {children}
    </span>
  );
}
