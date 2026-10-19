// The OpenFrame logo (owner request 2026-10-07; Trial_Licensing_Mocks section 5): a film-frame
// mark on a 24 by 24 grid whose gradient reads --lg1, --lg2 and --lg3 from the active theme, and
// the one-word wordmark "OpenFrame" ("Open" weight 300, "Frame" weight 700). There is no separate
// logo setting: choosing a theme or a mode recolours it.

import { useId } from "react";

export function LogoMark({ className = "mk" }: { className?: string }) {
  // Each mark needs its own gradient id: several logos can be on screen at once.
  const id = `ofg-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  return (
    <svg className={className} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id={id} gradientUnits="userSpaceOnUse" x1="2" y1="2" x2="22" y2="22">
          <stop offset="0" style={{ stopColor: "var(--lg1)" }} />
          <stop offset=".55" style={{ stopColor: "var(--lg2)" }} />
          <stop offset="1" style={{ stopColor: "var(--lg3)" }} />
        </linearGradient>
      </defs>
      <g fill="none" stroke={`url(#${id})`} strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
        <rect x="2" y="2" width="20" height="20" rx="2.2" />
        <line x1="7" y1="2" x2="7" y2="22" />
        <line x1="17" y1="2" x2="17" y2="22" />
        <line x1="2" y1="12" x2="22" y2="12" />
        <line x1="2" y1="7" x2="7" y2="7" />
        <line x1="2" y1="17" x2="7" y2="17" />
        <line x1="22" y1="7" x2="17" y2="7" />
        <line x1="22" y1="17" x2="17" y2="17" />
      </g>
    </svg>
  );
}

/** Mark and wordmark. `size="lg"` is the 40 px version used on the start-up screen. */
export function BrandLogo({ size }: { size?: "lg" }) {
  return (
    <span className={size === "lg" ? "brandlogo lg" : "brandlogo"} role="img" aria-label="OpenFrame">
      <LogoMark />
      <span className="wm" aria-hidden="true">
        <i>Open</i>Frame
      </span>
    </span>
  );
}
