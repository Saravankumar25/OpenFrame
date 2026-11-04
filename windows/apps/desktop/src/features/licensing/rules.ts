// Trial, purchase and first-run safety screens (owner mocks 182 to 198,
// docs/engineering/36-entitlement-and-trial.md section 10). Pure rules and fixed text. The Rust
// core decides which screen is due (`entitlement.notice`); these helpers only shape what is shown.

import type { Channel } from "../../ipc/generated/Channel";
import type { EntitlementState } from "../../ipc/generated/EntitlementState";

/** The one contact address (ADR-0039 item 11). Opened in the browser; the app sends nothing. */
export const CONTACT_URL = "https://straftstudio.com/contact/";

/** Trial reminders and the amber chip start when this many days or fewer are left (days 5 to 7). */
export const LAST_DAYS = 3;

/** The user's local calendar day as `YYYY-MM-DD` (the core stores the reminder day in this form). */
export function localDay(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

export interface TrialChip {
  daysLeft: number;
  /** The last 3 days: amber, and the screen reader text says the trial ends soon. */
  ending: boolean;
  label: string;
  spoken: string;
}

/** The corner chip, shown from trial day 1 until purchase. `null` in every other state. */
export function trialChip(state: EntitlementState | undefined): TrialChip | null {
  if (!state || state.state !== "trialActive") return null;
  const daysLeft = Math.max(1, Math.min(7, state.days_left));
  const ending = daysLeft <= LAST_DAYS;
  const label = daysLeft === 1 ? "Trial ends in 1 day" : `Trial ends in ${daysLeft} days`;
  const spoken =
    daysLeft === 1
      ? "Free trial, last day. Your trial ends within 24 hours."
      : `Free trial. Ends in ${daysLeft} days.${ending ? " Your trial ends soon." : ""}`;
  return { daysLeft, ending, label, spoken };
}

/** The reminder headline: 3 or 2 days left "ends in N days"; on the last day "ends today" when the
 * end moment is on today's local date, otherwise "ends tomorrow". */
export function reminderHeadline(daysLeft: number, endsAtMs?: number | null, now: Date = new Date()): string {
  if (daysLeft > 1) return `Your free trial ends in ${daysLeft} days`;
  const later = endsAtMs != null && localDay(new Date(endsAtMs)) !== localDay(now);
  return later ? "Your free trial ends tomorrow" : "Your free trial ends today";
}

/** "It ends on Thu 5 Nov at 9:42 PM." in local time, built from parts so the shape does not depend
 * on the system language. `null` when the core gave no end moment. */
export function trialEndLine(endsAtMs: number | null | undefined): string | null {
  if (endsAtMs == null) return null;
  const parts = new Intl.DateTimeFormat("en-US", { weekday: "short", day: "numeric", month: "short", hour: "numeric", minute: "2-digit", hour12: true }).formatToParts(new Date(endsAtMs));
  const p = (t: string) => parts.find((x) => x.type === t)?.value ?? "";
  return `It ends on ${p("weekday")} ${p("day")} ${p("month")} at ${p("hour")}:${p("minute")} ${p("dayPeriod")}.`;
}

/** The store's name as the screens say it: "the App Store" on the Mac, "the Microsoft Store" on
 * Windows. The screens never name a price; the store's own purchase window does (ADR-0023). */
export function storeName(channel: Channel | undefined): string {
  return channel === "macAppStore" ? "the App Store" : "the Microsoft Store";
}

/** Tutorial step 3, second point: how to install on the new laptop, in the words of each channel.
 * Only the stores have an account to sign in to; the internal installer build names none. */
export function newLaptopInstallStep(channel: Channel | undefined): string {
  switch (channel) {
    case "macAppStore":
      return "Install Straft Studio on the new laptop from the App Store, signed in with the same Apple Account.";
    case "microsoftStore":
      return "Install Straft Studio on the new laptop from the Microsoft Store, signed in with the same Microsoft account.";
    default:
      return "Install Straft Studio on the new laptop.";
  }
}

/** Keys that keep working while a mandatory screen holds the app's shortcuts back: copy, cut,
 * paste and select all, so the user can copy the root folder path. */
const CLIPBOARD_KEYS = new Set(["a", "c", "v", "x"]);

/** True for a key press that is one of the app's own shortcuts (a Ctrl, Alt or Cmd combination),
 * false for plain typing and for the clipboard keys. */
export function isAppShortcut(e: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">): boolean {
  if (!e.ctrlKey && !e.metaKey && !e.altKey) return false;
  const clipboard = (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && CLIPBOARD_KEYS.has(e.key.toLowerCase());
  return !clipboard;
}

/** Social accounts on the welcome popup. The owner has not sent the real handles or addresses yet
 * (placeholders, 2026-10-07), so no account has an address and none can be opened. When the real
 * ones arrive, each `url` is a fixed https address on that network's own site. */
export interface SocialLink {
  name: string;
  short: string;
  handle: string;
  url: string | null;
}

export const SOCIAL_LINKS: readonly SocialLink[] = [
  { name: "Instagram", short: "IG", handle: "@straftstudio", url: null },
  { name: "YouTube", short: "YT", handle: "@straftstudio", url: null },
  { name: "X", short: "X", handle: "@straftstudio", url: null },
  { name: "Facebook", short: "f", handle: "@straftstudio", url: null },
  { name: "LinkedIn", short: "in", handle: "@straftstudio", url: null },
  { name: "Reddit", short: "Rd", handle: "@straftstudio", url: null },
  { name: "Discord", short: "Dc", handle: "@straftstudio", url: null },
  { name: "Threads", short: "Th", handle: "@straftstudio", url: null },
];

/** A release build hides the rows that have no address yet (ADR-0039 item 12); a debug build shows
 * them all, so the mocks and tests still work. */
export function visibleSocialLinks(links: readonly SocialLink[], debugBuild: boolean): readonly SocialLink[] {
  return debugBuild ? links : links.filter((l) => isFixedSocialUrl(l.url));
}

/** The only sites a welcome link may open. */
const SOCIAL_HOSTS = new Set([
  "www.instagram.com",
  "instagram.com",
  "www.youtube.com",
  "youtube.com",
  "x.com",
  "www.facebook.com",
  "facebook.com",
  "www.linkedin.com",
  "linkedin.com",
  "www.reddit.com",
  "reddit.com",
  "discord.gg",
  "discord.com",
  "www.threads.net",
  "threads.net",
]);

/** A fixed social address: https, a known social site, no user name or password, no port. */
export function isFixedSocialUrl(url: string | null): url is string {
  if (!url) return false;
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  return u.protocol === "https:" && !u.username && !u.password && !u.port && SOCIAL_HOSTS.has(u.hostname);
}

/** Text the shell shows for "Show in File Explorer" or "Show in Finder". */
export function revealLabel(platform: string | undefined): string {
  return platform === "macos" ? "Show in Finder" : "Show in File Explorer";
}
