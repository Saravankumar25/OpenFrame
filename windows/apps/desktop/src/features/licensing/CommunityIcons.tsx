// Simple monochrome marks for the eight community accounts, drawn in the same 24 px outline style
// as the lucide icons the app already uses. They take the colour of the text around them.

import type { ReactNode } from "react";

function Mark({ children }: { children: ReactNode }) {
  return (
    <svg
      width={22}
      height={22}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {children}
    </svg>
  );
}

export const COMMUNITY_ICONS: Record<string, ReactNode> = {
  Instagram: (
    <Mark>
      <rect x="3" y="3" width="18" height="18" rx="5" />
      <circle cx="12" cy="12" r="4" />
      <path d="M17.5 6.5h.01" />
    </Mark>
  ),
  YouTube: (
    <Mark>
      <rect x="2" y="5" width="20" height="14" rx="4" />
      <path d="M10 9l5 3-5 3z" />
    </Mark>
  ),
  X: (
    <Mark>
      <path d="M4 4l16 16" />
      <path d="M20 4L4 20" />
    </Mark>
  ),
  Facebook: (
    <Mark>
      <path d="M17 3h-2.5A4.5 4.5 0 0 0 10 7.5V10H7v4h3v7h4v-7h3l1-4h-4V7.5c0-.8.7-1.5 1.5-1.5H17z" />
    </Mark>
  ),
  LinkedIn: (
    <Mark>
      <rect x="3" y="3" width="18" height="18" rx="3" />
      <path d="M8 11v5" />
      <path d="M8 8h.01" />
      <path d="M12 16v-5" />
      <path d="M12 13a2.5 2.5 0 0 1 5 0v3" />
    </Mark>
  ),
  Reddit: (
    <Mark>
      <ellipse cx="12" cy="14" rx="8" ry="5.5" />
      <path d="M9 14h.01" />
      <path d="M15 14h.01" />
      <path d="M9.5 16.5c1.5 1 3.5 1 5 0" />
      <path d="M12 8.5l1-4.5 4 1" />
      <circle cx="18" cy="5.5" r="1" />
    </Mark>
  ),
  Discord: (
    <Mark>
      <path d="M8 5C5.5 5.5 4 6.5 3 8c-1 3-1.3 6-1 9 1.5 1.5 3.5 2.5 5.5 3l1-2c1.5.5 4.5.5 6 0l1 2c2-.5 4-1.5 5.5-3 .3-3 0-6-1-9-1-1.5-2.5-2.5-5-3l-.7 1.3a11 11 0 0 0-6.6 0z" />
      <path d="M9 12.5h.01" />
      <path d="M15 12.5h.01" />
    </Mark>
  ),
  Threads: (
    <Mark>
      <path d="M17 8.5C16 6 14 5 12 5c-4 0-6 3-6 7s2 7 6 7c3 0 5-1.5 5-4 0-2-1.5-3-3.5-3-1.8 0-3 1-3 2.3 0 1.2 1 2 2.3 2 1.6 0 2.7-1.2 2.7-3.8V10" />
    </Mark>
  ),
};
