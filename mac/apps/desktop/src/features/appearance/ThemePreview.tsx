import { LogoMark } from "../../theme/BrandLogo";
import type { ResolvedMode } from "../../theme/engine";

/** A small picture of a theme, drawn in that theme: sidebar with the logo, a panel, text and accent. */
export function ThemePreview({ themeId, mode }: { themeId: string; mode: ResolvedMode }) {
  return (
    <span className="ap-pv" data-theme={themeId} data-mode={mode} aria-hidden="true">
      <b>
        <LogoMark />
      </b>
      <span />
    </span>
  );
}
