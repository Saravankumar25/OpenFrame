// The legal pages on the public website. Each address is written in full so scripts/check-links.mjs
// can match it against scripts/app-addresses.json; nothing is fetched, a page opens in the browser
// only when the person clicks.

export const LEGAL_PAGES = [
  { id: "privacy", label: "Privacy Policy", url: "https://straftstudio.com/legal/privacy/" },
  { id: "terms", label: "Terms", url: "https://straftstudio.com/legal/terms/" },
  { id: "refunds", label: "Refunds", url: "https://straftstudio.com/legal/refunds/" },
] as const;

export type LegalPageId = (typeof LEGAL_PAGES)[number]["id"];

export function legalUrl(id: LegalPageId): string {
  return LEGAL_PAGES.find((p) => p.id === id)!.url;
}
