// Trial, purchase and first-run safety screens (owner mocks 182 to 198; ADR-0022 and ADR-0023).
// The IPC boundary is replaced by a small fake core. Flow tests let it answer `entitlement.*` the
// way the Rust rules do; the tests under "what the interface draws for each answer" fix the answer
// instead, so they check only the interface (the rule itself is tested in Rust). A real Mac, App
// Store or Microsoft Store is not used.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithClient } from "../../app/home/testUtils";
import { useNav } from "../../app/stores";
import { useToasts } from "../../app/toast";
import { registerPendingSave } from "../../app/pendingSaves";
import { LicenceSection } from "../../workspaces/settings/Licence";
import { LicenceScreens, WelcomeDialog } from "./LicenceScreens";
import { AboutSection } from "../../workspaces/settings/About";
import { TrialCorner } from "./TrialCorner";
import { NOTICE_OP, useCheckScreen } from "./licence";
import {
  CONTACT_URL,
  SOCIAL_LINKS,
  isAppShortcut,
  isFixedSocialUrl,
  localDay,
  newLaptopInstallStep,
  reminderHeadline,
  revealLabel,
  storeName,
  trialChip,
  trialEndLine,
  type SocialLink,
} from "./rules";

// ------------------------------------------------------------------ fake core

type State =
  | { state: "trialActive"; days_left: number }
  | { state: "trialEnded" | "purchased" | "revoked" | "unknown" | "needsOnlineCheck" | "trialNotStarted" | "internalBuild" | "clockChanged" };
type Notice =
  | { kind: "none" | "dataNote" | "tutorial" | "welcome" | "macFirstLaunch" | "locked" | "checkNeeded" | "clockChanged" }
  | { kind: "reminder"; daysLeft: number };

/** The approved community addresses, written out again here on purpose: a change in the config must fail this test. */
const EXPECTED_COMMUNITY = [
  ["Instagram", "https://www.instagram.com/straftstudio/"],
  ["YouTube", "https://www.youtube.com/@StraftStudio"],
  ["X", "https://x.com/straftstudio/"],
  ["Facebook", "https://www.facebook.com/straftstudio/"],
  ["LinkedIn", "https://www.linkedin.com/company/straftstudio/"],
  ["Reddit", "https://www.reddit.com/user/straftstudio/"],
  ["Discord", "https://straftstudio.com/discord"],
  ["Threads", "https://www.threads.com/@straftstudio/"],
];

const NOT_CONNECTED = "The Microsoft Store connection is not available in this build.";

const core = {
  channel: "microsoftStore" as string,
  state: { state: "trialActive", days_left: 7 } as State,
  reminderDay: null as string | null,
  flags: { dataNote: false, tutorial: false, welcome: false },
  buyRoute: { route: "unavailable" } as Record<string, unknown>,
  buyMessage: NOT_CONNECTED as string | null,
  /** When set, `entitlement.notice` answers this notice whatever the state. */
  fixedNotice: null as Notice | null,
  /** When set, `entitlement.record_notice` fails with this message. */
  recordFails: null as string | null,
  afterBuy: null as State | null,
  noticeFails: false,
  /** A debug build shows the placeholder social rows; a release build hides them. */
  debugBuild: true,
  endsAtMs: null as number | null,
  calls: [] as { op: string; args: Record<string, unknown> }[],
};

function due(today: string | undefined): Notice {
  if (core.fixedNotice) return core.fixedNotice;
  const s = core.state;
  if (core.channel === "directInstaller") return { kind: "none" };
  if (s.state === "trialNotStarted" && core.channel === "macAppStore") return { kind: "macFirstLaunch" };
  if (s.state === "trialEnded" || s.state === "revoked") return { kind: "locked" };
  if (s.state === "needsOnlineCheck") return { kind: "checkNeeded" };
  if (s.state === "purchased") {
    if (!core.flags.dataNote) return { kind: "dataNote" };
    if (!core.flags.tutorial) return { kind: "tutorial" };
    if (!core.flags.welcome) return { kind: "welcome" };
    return { kind: "none" };
  }
  if (s.state === "trialActive" && s.days_left <= 3 && today && !(core.reminderDay && core.reminderDay >= today)) {
    return { kind: "reminder", daysLeft: s.days_left };
  }
  return { kind: "none" };
}

const SUMMARY: Record<string, string> = {
  internalBuild: "Internal build, not for sale.",
  trialEnded: "Your free trial has ended. Buy Straft Studio or restore your purchase to open your projects again. Nothing is deleted.",
  purchased: "Straft Studio is unlocked. Thank you.",
};
const status = () => ({
  channel: core.channel,
  state: core.state,
  summary: SUMMARY[core.state.state] ?? "Free trial: 3 days left.",
  limitation: null,
  trialDays: 7,
  canEdit: true,
  trialEndsAtMs: core.endsAtMs,
});
const view = (today?: string) => ({
  status: status(),
  notice: due(today),
  dataRoot: "C:\\Users\\Nisha\\Documents\\Straft Studio",
  debugBuild: core.debugBuild,
});

const reveal = vi.fn(async () => undefined);
const open = vi.fn(async () => undefined);
const lockOrder: string[] = [];
const handlers = new Set<(e: unknown) => void>();
/** What the core pushes when the licence changed. */
const emit = (e: unknown) => handlers.forEach((h) => h(e));

vi.mock("../../ipc/client", async () => {
  class OpError extends Error {
    code: string;
    retryable = false;
    constructor(code: string, message: string) {
      super(message);
      this.code = code;
    }
    is(p: string) {
      return this.code === p || this.code.startsWith(p + ".");
    }
  }
  return {
    OpError,
    inTauri: () => false,
    onAppEvent: (h: (e: unknown) => void) => {
      handlers.add(h);
      return Promise.resolve(() => handlers.delete(h));
    },
    revealLocation: (...a: unknown[]) => reveal(...(a as [])),
    openUrl: (...a: unknown[]) => open(...(a as [])),
    call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
      core.calls.push({ op, args });
      switch (op) {
        case "app.info":
          return { platform: core.channel === "macAppStore" ? "macos" : "windows", projectsDir: "", profile: { displayName: "N" } };
        case "entitlement.status":
          return status();
        case "entitlement.notice":
          if (core.noticeFails) throw new OpError("internal.unexpected", "broken");
          return view(args.today as string | undefined);
        case "entitlement.record_notice": {
          if (core.recordFails) throw new OpError("io.write_failed", core.recordFails);
          const n = args.notice as string;
          if (n === "reminder") core.reminderDay = args.today as string;
          if (n === "dataNote") core.flags.dataNote = true;
          if (n === "tutorial") core.flags.tutorial = true;
          if (n === "welcome") core.flags.welcome = true;
          return view(args.today as string | undefined);
        }
        case "entitlement.buy":
          if (core.afterBuy) core.state = core.afterBuy;
          return { route: core.buyRoute, status: status(), message: core.buyMessage };
        case "entitlement.start_trial":
          core.state = { state: "trialActive", days_left: 7 };
          return { route: { route: "store" }, status: status(), message: null };
        case "entitlement.restore_purchase":
          if (core.afterBuy) core.state = core.afterBuy;
          return status();
        case "entitlement.lock_ready":
          lockOrder.push("lock_ready");
          return null;
        default:
          throw new Error(`unexpected operation in test: ${op}`);
      }
    }),
  };
});

const today = localDay();
const ops = (name: string) => core.calls.filter((c) => c.op === name);

beforeEach(() => {
  core.channel = "microsoftStore";
  core.state = { state: "trialActive", days_left: 7 };
  core.reminderDay = null;
  core.flags = { dataNote: false, tutorial: false, welcome: false };
  core.buyRoute = { route: "unavailable" };
  core.buyMessage = NOT_CONNECTED;
  core.fixedNotice = null;
  core.recordFails = null;
  core.afterBuy = null;
  core.noticeFails = false;
  core.debugBuild = true;
  core.endsAtMs = null;
  core.calls = [];
  handlers.clear();
  lockOrder.length = 0;
  reveal.mockClear();
  open.mockClear();
  useNav.getState().reset();
  useCheckScreen.setState({ closed: false });
  useToasts.setState({ items: [] });
});

afterEach(() => {
  vi.useRealTimers();
});

/** Wait until the notice query has its answer, so "no dialog" means the answer drew none. */
async function settled(client: { getQueryState: (key: unknown[]) => { status: string } | undefined }) {
  await waitFor(() => expect(client.getQueryState([NOTICE_OP, today])?.status).toBe("success"));
}

/** A price or a currency must never appear on a licensing screen (ADR-0023). The reminder says "Pay
 * once, own forever. No subscription." (owner mock 184), so the pricing model words are allowed. */
const PRICE_TALK = [
  /[$€£₹¥]/,
  /\d[\d.,]*\s*(USD|EUR|GBP|INR|JPY|AUD|CAD|CHF)\b/i,
  /\b(USD|EUR|GBP|INR|JPY|AUD|CAD|CHF)\s*\d/i,
  /\bprices?\b/i,
  /\b(rupees?|dollars?|euros?)\b/i,
];
function expectNoPriceTalk(text: string, where: string) {
  for (const re of PRICE_TALK) expect(text, `${where}: ${re}`).not.toMatch(re);
}

// ------------------------------------------------------------------ rules

describe("rules", () => {
  it("formats the local calendar day", () => {
    expect(localDay(new Date(2026, 9, 7, 23, 59))).toBe("2026-10-07");
    expect(localDay(new Date(2027, 0, 1, 0, 0))).toBe("2027-01-01");
  });

  it("shows the chip only during the trial and turns it amber in the last 3 days", () => {
    for (const n of [7, 6, 5, 4, 3, 2, 1]) {
      const chip = trialChip({ state: "trialActive", days_left: n });
      expect(chip?.label).toBe(n === 1 ? "Trial ends in 1 day" : `Trial ends in ${n} days`);
      expect(chip?.ending).toBe(n <= 3);
      expect(chip?.spoken).toBe(
        n === 1
          ? "Free trial, last day. Your trial ends within 24 hours."
          : `Free trial. Ends in ${n} days.${n <= 3 ? " Your trial ends soon." : ""}`,
      );
    }
    for (const state of ["purchased", "trialEnded", "revoked", "unknown", "needsOnlineCheck", "trialNotStarted", "internalBuild", "clockChanged"] as const) {
      expect(trialChip({ state })).toBeNull();
    }
    expect(trialChip(undefined)).toBeNull();
  });

  it("words the reminder for days 5, 6 and 7", () => {
    const noon = new Date(2026, 10, 4, 12, 0);
    expect(reminderHeadline(3)).toBe("Your free trial ends in 3 days");
    expect(reminderHeadline(2)).toBe("Your free trial ends in 2 days");
    // The last day: today when the end moment is on today's date, tomorrow when it is on the next.
    expect(reminderHeadline(1, new Date(2026, 10, 4, 21, 42).getTime(), noon)).toBe("Your free trial ends today");
    expect(reminderHeadline(1, new Date(2026, 10, 5, 9, 42).getTime(), noon)).toBe("Your free trial ends tomorrow");
    expect(reminderHeadline(1, null, noon)).toBe("Your free trial ends today");
    expect(reminderHeadline(1, undefined, noon)).toBe("Your free trial ends today");
  });

  it("words the end moment in local time, in one fixed shape", () => {
    // Built from local parts, so the check does not depend on the machine's time zone.
    expect(trialEndLine(new Date(2026, 10, 5, 21, 42).getTime())).toBe("It ends on Thu 5 Nov at 9:42 PM.");
    expect(trialEndLine(new Date(2026, 10, 9, 0, 5).getTime())).toBe("It ends on Mon 9 Nov at 12:05 AM.");
    expect(trialEndLine(new Date(2026, 10, 12, 9, 7).getTime())).toBe("It ends on Thu 12 Nov at 9:07 AM.");
    expect(trialEndLine(null)).toBeNull();
    expect(trialEndLine(undefined)).toBeNull();
  });

  it("keeps the contact address in one place", () => {
    expect(CONTACT_URL).toBe("https://straftstudio.com/contact/");
  });

  it("names each store the way the screens say it", () => {
    expect(storeName("macAppStore")).toBe("the App Store");
    expect(storeName("microsoftStore")).toBe("the Microsoft Store");
  });

  it("words the new-laptop install step for each channel and names no account for the internal build", () => {
    expect(newLaptopInstallStep("macAppStore")).toContain("App Store, signed in with the same Apple Account");
    expect(newLaptopInstallStep("microsoftStore")).toContain("Microsoft Store, signed in with the same Microsoft account");
    expect(newLaptopInstallStep("directInstaller")).toBe("Install Straft Studio on the new laptop.");
    expect(newLaptopInstallStep("directInstaller")).not.toMatch(/account|sign in|store/i);
  });

  it("treats Ctrl, Alt and Cmd combinations as app shortcuts except copy, cut, paste and select all", () => {
    const key = (k: string, mods: Partial<Record<"ctrlKey" | "metaKey" | "altKey" | "shiftKey", boolean>> = {}) => ({
      key: k,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      shiftKey: false,
      ...mods,
    });
    for (const k of ["c", "a", "x", "v", "C", "V"]) {
      expect(isAppShortcut(key(k, { ctrlKey: true }))).toBe(false);
      expect(isAppShortcut(key(k, { metaKey: true }))).toBe(false);
    }
    expect(isAppShortcut(key("k", { ctrlKey: true }))).toBe(true);
    expect(isAppShortcut(key("j", { metaKey: true }))).toBe(true);
    expect(isAppShortcut(key("c", { ctrlKey: true, shiftKey: true }))).toBe(true);
    expect(isAppShortcut(key("c", { altKey: true }))).toBe(true);
    expect(isAppShortcut(key("F4", { altKey: true }))).toBe(true);
    expect(isAppShortcut(key("a"))).toBe(false);
    expect(isAppShortcut(key("Tab"))).toBe(false);
  });

  it("accepts only fixed https addresses on known social sites", () => {
    expect(isFixedSocialUrl("https://www.instagram.com/straftstudio")).toBe(true);
    expect(isFixedSocialUrl("https://straftstudio.com/discord")).toBe(true);
    for (const bad of [
      null,
      "",
      "http://www.instagram.com/x",
      "javascript:alert(1)",
      "https://evil.example/instagram.com",
      "https://user:pw@x.com/a",
      "https://x.com:8443/a",
      "file:///C:/Windows",
      "https://www.instagram.com.evil.example/",
      "https://discord.gg/wPKGPf5mm",
      "https://discord.com/invite/abc",
      "https://www.threads.net/@straftstudio",
      "https://instagram.com/straftstudio/",
    ]) {
      expect(isFixedSocialUrl(bad)).toBe(false);
    }
    expect(SOCIAL_LINKS).toHaveLength(8);
    expect(SOCIAL_LINKS.every((l) => isFixedSocialUrl(l.url))).toBe(true);
  });

  it("names the file manager of each system", () => {
    expect(revealLabel("windows")).toBe("Show in File Explorer");
    expect(revealLabel("macos")).toBe("Show in Finder");
    expect(revealLabel(undefined)).toBe("Show in File Explorer");
  });
});

// ------------------------------------------------------------------ corner chip and Buy (183, 188)

describe("trial chip and Buy full version", () => {
  it("shows the days left with screen reader text, plain on day 3 of the trial", async () => {
    core.state = { state: "trialActive", days_left: 5 };
    renderWithClient(<TrialCorner />);
    const group = await screen.findByRole("group", { name: "Free trial" });
    expect(within(group).getByText("Trial ends in 5 days")).toBeInTheDocument();
    expect(within(group).getByText("Free trial. Ends in 5 days.")).toHaveClass("sr-only");
    expect(group.querySelector(".chip")).not.toHaveClass("y");
    expect(within(group).getByRole("button", { name: "Buy full version" })).toBeInTheDocument();
  });

  it("turns amber with words in the last 3 days", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    renderWithClient(<TrialCorner />);
    const group = await screen.findByRole("group", { name: "Free trial" });
    expect(group.querySelector(".chip")).toHaveClass("y");
    expect(within(group).getByText("Free trial. Ends in 2 days. Your trial ends soon.")).toBeInTheDocument();
  });

  it("shows on Application Home through the same component, with the Buy button", async () => {
    // AppHome mounts <TrialCorner /> in its top bar (src/app/home/AppHome.test.tsx asserts the mount);
    // here the component alone answers for every day.
    for (const n of [7, 4, 3, 2, 1]) {
      core.state = { state: "trialActive", days_left: n };
      const r = renderWithClient(<TrialCorner />);
      const group = await screen.findByRole("group", { name: "Free trial" });
      expect(within(group).getByText(n === 1 ? "Trial ends in 1 day" : `Trial ends in ${n} days`)).toBeInTheDocument();
      expect(within(group).getByRole("button", { name: "Buy full version" })).toBeInTheDocument();
      r.unmount();
    }
  });

  it("is gone after the purchase and in every other state", async () => {
    for (const state of ["purchased", "trialEnded", "unknown"] as const) {
      core.state = { state };
      const r = renderWithClient(<TrialCorner />);
      await settled(r.client);
      expect(screen.queryByRole("group", { name: "Free trial" })).toBeNull();
      expect(screen.queryByRole("button", { name: "Buy full version" })).toBeNull();
      r.unmount();
    }
  });

  it("hands Buy to the Microsoft Store's purchase dialog and says plainly when it cannot", async () => {
    core.buyRoute = { route: "store" };
    core.buyMessage = null;
    core.afterBuy = { state: "purchased" };
    const r = renderWithClient(<TrialCorner />);
    await userEvent.click(await screen.findByRole("button", { name: "Buy full version" }));
    await waitFor(() => expect(ops("entitlement.buy")).toHaveLength(1));
    await settled(r.client);
    await waitFor(() => expect(screen.queryByRole("group", { name: "Free trial" })).toBeNull());
    expect(useToasts.getState().items).toEqual([]);
    r.unmount();

    // A build without the Store connection says so.
    core.state = { state: "trialActive", days_left: 4 };
    core.buyRoute = { route: "unavailable" };
    core.buyMessage = NOT_CONNECTED;
    core.afterBuy = null;
    renderWithClient(<TrialCorner />);
    await userEvent.click(await screen.findByRole("button", { name: "Buy full version" }));
    await waitFor(() => expect(useToasts.getState().items.map((t) => t.text)).toEqual([NOT_CONNECTED]));
    expect(open).not.toHaveBeenCalled();
  });

  it("the internal installer build shows no chip and no Buy, even in a trial state set by hand", async () => {
    core.channel = "directInstaller";
    core.state = { state: "trialActive", days_left: 2 };
    const r = renderWithClient(
      <>
        <TrialCorner />
        <LicenceScreens />
      </>,
    );
    await settled(r.client);
    expect(screen.queryByRole("group")).toBeNull();
    expect(screen.queryByRole("button", { name: "Buy full version" })).toBeNull();
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

// ------------------------------------------------------------------ reminders (184, 185, 189)

describe("trial reminders", () => {
  it("day 5 says ends in 3 days, is remembered for today and closes with Continue trial", async () => {
    core.state = { state: "trialActive", days_left: 3 };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Trial reminder" });
    expect(within(dialog).getByText("Your free trial ends in 3 days")).toBeInTheDocument();
    expect(within(dialog).getByText(/Nothing is deleted when the trial ends/)).toBeInTheDocument();
    expect(
      within(dialog).getByText("Buy the full version to keep using Straft Studio. Pay once, own forever. No subscription."),
    ).toBeInTheDocument();
    expect(within(dialog).queryByText(/^It ends on/)).toBeNull();
    await waitFor(() => expect(ops("entitlement.record_notice")[0]?.args).toEqual({ notice: "reminder", today }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Continue trial" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    // Asked again later the same day: nothing.
    await waitFor(() => expect(ops("entitlement.notice").length).toBeGreaterThan(1));
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("day 6 says ends in 2 days and day 7 says ends today", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    const first = renderWithClient(<LicenceScreens />);
    expect(await screen.findByText("Your free trial ends in 2 days")).toBeInTheDocument();
    first.unmount();
    core.state = { state: "trialActive", days_left: 1 };
    core.reminderDay = null;
    renderWithClient(<LicenceScreens />);
    expect(await screen.findByText("Your free trial ends today")).toBeInTheDocument();
  });

  it("never shows on days 1 to 4 or after a purchase", async () => {
    core.state = { state: "trialActive", days_left: 4 };
    const a = renderWithClient(<LicenceScreens />);
    await settled(a.client);
    expect(screen.queryByRole("dialog")).toBeNull();
    a.unmount();
    core.state = { state: "purchased" };
    core.flags = { dataNote: true, tutorial: true, welcome: true };
    const b = renderWithClient(<LicenceScreens />);
    await settled(b.client);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(ops("entitlement.record_notice")).toHaveLength(0);
    b.unmount();
  });

  it("shows on Application Home too: the component needs no open project", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    renderWithClient(<LicenceScreens />);
    expect(await screen.findByRole("dialog", { name: "Trial reminder" })).toBeInTheDocument();
  });

  it("says the real end moment under the headline, today or tomorrow on the last day", async () => {
    core.state = { state: "trialActive", days_left: 1 };
    // Ends later today when it is before midnight now; use today 23:58 so the test is stable.
    const endsToday = new Date();
    endsToday.setHours(23, 58, 0, 0);
    core.endsAtMs = endsToday.getTime();
    const first = renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Trial reminder" });
    expect(within(dialog).getByText("Your free trial ends today")).toBeInTheDocument();
    expect(within(dialog).getByText(trialEndLine(endsToday.getTime())!)).toBeInTheDocument();
    first.unmount();

    core.reminderDay = null;
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    tomorrow.setHours(9, 42, 0, 0);
    core.endsAtMs = tomorrow.getTime();
    renderWithClient(<LicenceScreens />);
    const next = await screen.findByRole("dialog", { name: "Trial reminder" });
    expect(within(next).getByText("Your free trial ends tomorrow")).toBeInTheDocument();
    expect(within(next).getByText(/^It ends on .+ at 9:42 AM\.$/)).toBeInTheDocument();
  });

  it("closes at once when a purchase is seen, and a later \"none\" answer alone does not close it", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    const r = renderWithClient(<LicenceScreens />);
    await screen.findByRole("dialog", { name: "Trial reminder" });
    // The reminder was recorded as shown, so the next answer says none: it stays open.
    await act(async () => {
      await r.client.invalidateQueries({ queryKey: [NOTICE_OP] });
    });
    expect(screen.getByRole("dialog", { name: "Trial reminder" })).toBeInTheDocument();
    // A purchase made elsewhere: the core says so, and the reminder is gone.
    core.state = { state: "purchased" };
    core.flags = { dataNote: true, tutorial: true, welcome: true };
    await act(async () => {
      emit({ type: "module", module: "entitlement", name: "changed", payload: null });
    });
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("Buy during the reminder, then the purchase mid-trial starts the data note", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    core.channel = "macAppStore";
    core.buyRoute = { route: "store" };
    core.buyMessage = null;
    core.afterBuy = { state: "purchased" };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Trial reminder" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Buy full version" }));
    expect(await screen.findByRole("dialog", { name: "Your data, your responsibility" })).toBeInTheDocument();
  });

  it("Buy without the store connection says so inside the reminder and keeps it open", async () => {
    core.state = { state: "trialActive", days_left: 3 };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Trial reminder" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Buy full version" }));
    expect(await within(dialog).findByText(NOT_CONNECTED)).toBeInTheDocument();
  });

  it("a reminder day that cannot be saved goes to the diagnostics, not on screen", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    core.recordFails = "The disk is full.";
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    try {
      renderWithClient(<LicenceScreens />);
      expect(await screen.findByRole("dialog", { name: "Trial reminder" })).toBeInTheDocument();
      await waitFor(() => expect(warn).toHaveBeenCalledWith(expect.stringContaining("io.write_failed"), expect.anything()));
      expect(useToasts.getState().items).toEqual([]);
    } finally {
      warn.mockRestore();
    }
  });

  it("a failed check never crashes and shows nothing", async () => {
    core.noticeFails = true;
    renderWithClient(<LicenceScreens />);
    await waitFor(() => expect(ops("entitlement.notice").length).toBeGreaterThan(0));
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

// ------------------------------------------------------------------ data note (193)

describe("data note", () => {
  beforeEach(() => {
    core.state = { state: "purchased" };
  });

  it("enables OK only after scrolling to the end and ticking, and cannot be closed", async () => {
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Your data, your responsibility" });
    expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(6);
    expect(within(dialog).getByText("C:\\Users\\Nisha\\Documents\\Straft Studio")).toBeInTheDocument();

    // jsdom has no layout, so the list "fits" at first; give it a real scroll size and scroll it.
    const list = within(dialog).getByRole("region", { name: "Six points about your data" });
    expect(list).toHaveAttribute("tabindex", "0");
    const ok = within(dialog).getByRole("button", { name: "OK, I understand" });
    const box = within(dialog).getByRole("checkbox");
    await userEvent.click(box);
    expect(ok).toBeEnabled();

    // Escape and a click outside do nothing.
    await userEvent.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "Your data, your responsibility" })).toBeInTheDocument();
    await userEvent.click(ok);
    expect(await screen.findByRole("dialog", { name: "File safety tutorial" })).toBeInTheDocument();
    expect(ops("entitlement.record_notice")[0].args).toMatchObject({ notice: "dataNote" });
  });

  it("keeps OK disabled until the list is scrolled to the end", async () => {
    const sizes = { scrollHeight: 900, clientHeight: 260, scrollTop: 0 };
    const region = (el: Element) => el.getAttribute("role") === "region";
    const spyH = vi.spyOn(Element.prototype, "scrollHeight", "get").mockImplementation(function (this: Element) {
      return region(this) ? sizes.scrollHeight : 0;
    });
    const spyC = vi.spyOn(Element.prototype, "clientHeight", "get").mockImplementation(function (this: Element) {
      return region(this) ? sizes.clientHeight : 0;
    });
    const spyT = vi.spyOn(Element.prototype, "scrollTop", "get").mockImplementation(function (this: Element) {
      return region(this) ? sizes.scrollTop : 0;
    });
    try {
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your data, your responsibility" });
      const ok = within(dialog).getByRole("button", { name: "OK, I understand" });
      const box = within(dialog).getByRole("checkbox");
      expect(box).toBeDisabled();
      expect(ok).toBeDisabled();
      expect(within(dialog).getByText("Scroll to the end of the list to continue.")).toBeInTheDocument();
      const list = within(dialog).getByRole("region", { name: "Six points about your data" });
      sizes.scrollTop = 300;
      fireEvent.scroll(list);
      expect(box).toBeDisabled();
      sizes.scrollTop = 640;
      fireEvent.scroll(list);
      expect(box).toBeEnabled();
      expect(ok).toBeDisabled();
      expect(within(dialog).getByText("Tick the box to continue.")).toBeInTheDocument();
      // Keyboard only: tick with Space.
      box.focus();
      await userEvent.keyboard(" ");
      expect(ok).toBeEnabled();
    } finally {
      spyH.mockRestore();
      spyC.mockRestore();
      spyT.mockRestore();
    }
  });

  it("says what uninstalling removes on Windows, and only to back up on the Mac", async () => {
    const r = renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Your data, your responsibility" });
    expect(
      within(dialog).getByText(
        "Uninstalling Straft Studio removes its private data on this computer (settings and downloaded AI files), but not your projects in the root folder.",
      ),
    ).toBeInTheDocument();
    r.unmount();
    core.channel = "macAppStore";
    renderWithClient(<LicenceScreens />);
    const mac = await screen.findByRole("dialog", { name: "Your data, your responsibility" });
    expect(within(mac).getByText("Back up your root folder.")).toBeInTheDocument();
    expect(within(mac).queryByText(/Uninstalling/)).toBeNull();
  });

  it("shows the real root folder through the platform boundary", async () => {
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Your data, your responsibility" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Show in File Explorer" }));
    expect(reveal).toHaveBeenCalledWith("dataRoot");
  });

  it("holds back the app's shortcuts while it is open, but lets copy, cut, paste and select all through", async () => {
    const seen = vi.fn();
    window.addEventListener("keydown", seen);
    try {
      renderWithClient(<LicenceScreens />);
      await screen.findByRole("dialog", { name: "Your data, your responsibility" });
      fireEvent.keyDown(window, { key: "k", ctrlKey: true });
      fireEvent.keyDown(window, { key: "j", metaKey: true });
      fireEvent.keyDown(window, { key: "c", ctrlKey: true, shiftKey: true });
      expect(seen).not.toHaveBeenCalled();
      fireEvent.keyDown(window, { key: "a" });
      expect(seen).toHaveBeenCalledTimes(1);
      // Copy, select all, cut and paste reach the page and are not cancelled, so the root folder
      // path can be copied.
      expect(fireEvent.keyDown(window, { key: "c", ctrlKey: true })).toBe(true);
      expect(fireEvent.keyDown(window, { key: "a", ctrlKey: true })).toBe(true);
      expect(fireEvent.keyDown(window, { key: "x", ctrlKey: true })).toBe(true);
      expect(fireEvent.keyDown(window, { key: "v", metaKey: true })).toBe(true);
      expect(seen).toHaveBeenCalledTimes(5);
    } finally {
      window.removeEventListener("keydown", seen);
    }
  });
});

// ------------------------------------------------------------------ tutorial (194 to 197) and welcome (198)

describe("file safety tutorial and welcome", () => {
  beforeEach(() => {
    core.state = { state: "purchased" };
    core.flags.dataNote = true;
  });

  it("walks 4 steps by keyboard; Finish needs all three boxes; then the welcome once", async () => {
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "File safety tutorial" });
    expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
    expect(within(dialog).getByText("Step 1 of 4 · mandatory")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Show in File Explorer" })).toBeInTheDocument();
    expect(within(dialog).queryByRole("button", { name: "Back" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "File safety tutorial" })).toBeInTheDocument();

    const next = () => within(dialog).getByRole("button", { name: "Next" });
    next().focus();
    await userEvent.keyboard("{Enter}");
    expect(within(dialog).getByRole("heading", { name: "Back up the root folder to Google Drive" })).toHaveFocus();
    expect(within(dialog).getByText("Step 2 of 4 · mandatory")).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Back" }));
    expect(within(dialog).getByRole("heading", { name: "Where your data lives" })).toBeInTheDocument();
    await userEvent.click(next());
    await userEvent.click(next());
    expect(within(dialog).getByText(/Linked external files/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Microsoft Store, signed in with the same Microsoft account/)).toBeInTheDocument();
    await userEvent.click(next());
    const finish = within(dialog).getByRole("button", { name: "Finish" });
    expect(finish).toBeDisabled();
    const boxes = within(dialog).getAllByRole("checkbox");
    expect(boxes).toHaveLength(3);
    await userEvent.click(boxes[0]);
    await userEvent.click(boxes[1]);
    expect(finish).toBeDisabled();
    boxes[2].focus();
    await userEvent.keyboard(" ");
    expect(finish).toBeEnabled();
    await userEvent.click(finish);

    const welcome = await screen.findByRole("dialog", { name: "Welcome to the Straft Studio community" });
    expect(within(welcome).getByText("Thank you for backing independent filmmaking. Be a part of our community.")).toBeInTheDocument();
    expect(within(welcome).queryByText(/family/)).toBeNull();
    // All eight accounts are buttons that open the approved address and nothing else.
    for (const [name, url] of EXPECTED_COMMUNITY) {
      await userEvent.click(within(welcome).getByRole("button", { name: `${name}, opens in your browser` }));
      expect(open).toHaveBeenLastCalledWith(url);
    }
    expect(open).toHaveBeenCalledTimes(8);
    expect(within(welcome).getAllByRole("button").map((b) => b.textContent).filter(Boolean)).toEqual([
      ...EXPECTED_COMMUNITY.map(([name]) => name),
      "OK",
    ]);
    await userEvent.click(within(welcome).getByRole("button", { name: "OK" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(ops("entitlement.record_notice").map((c) => c.args.notice)).toEqual(["tutorial", "welcome"]);
  });

  it("opens a welcome link only through the safe path and only for a fixed https address", async () => {
    const links: SocialLink[] = [
      { name: "Instagram", url: "https://www.instagram.com/a" },
      { name: "Bad", url: "http://www.instagram.com/b" },
      { name: "Evil", url: "https://evil.example/c" },
    ];
    renderWithClient(<WelcomeDialog links={links} />);
    await userEvent.click(screen.getByRole("button", { name: "Instagram, opens in your browser" }));
    expect(open).toHaveBeenCalledWith("https://www.instagram.com/a");
    expect(screen.queryByRole("button", { name: /Bad/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /Evil/ })).toBeNull();
  });

  it("shows the eight community accounts with the exact addresses and no old value, in any build", async () => {
    core.state = { state: "purchased" };
    core.flags = { dataNote: true, tutorial: true, welcome: false };
    for (const debugBuild of [true, false]) {
      core.debugBuild = debugBuild;
      const r = renderWithClient(<LicenceScreens />);
      const welcome = await screen.findByRole("dialog", { name: "Welcome to the Straft Studio community" });
      expect(within(welcome).getAllByRole("listitem")).toHaveLength(8);
      expect(within(welcome).queryByText(/@straftstudio/)).toBeNull();
      r.unmount();
    }
    expect(JSON.stringify(SOCIAL_LINKS)).not.toMatch(/wPKGPf5mm|discord.gg|threads.net|openframe|placeholder/i);
    expect(SOCIAL_LINKS.map((l) => [l.name, l.url])).toEqual(EXPECTED_COMMUNITY);
  });

  it("trial users see none of the three screens", async () => {
    core.state = { state: "trialActive", days_left: 6 };
    core.flags.dataNote = false;
    const r = renderWithClient(<LicenceScreens />);
    await settled(r.client);
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

// ------------------------------------------------------------------ store screens (187, 190, 192)

describe("store screens", () => {
  it("Mac first launch offers the free trial, the purchase and restore, and cannot be closed", async () => {
    core.channel = "macAppStore";
    core.state = { state: "trialNotStarted" };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Welcome to Straft Studio" });
    expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "Welcome to Straft Studio" })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Need help? Contact us" }));
    expect(open).toHaveBeenCalledWith(CONTACT_URL);
    await userEvent.click(within(dialog).getByRole("button", { name: "Start free trial" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(ops("entitlement.start_trial")).toHaveLength(1);
  });

  it("the clock lock has its own screen so a lock never has none: Check again and the contact link only", async () => {
    core.channel = "macAppStore";
    core.state = { state: "clockChanged" };
    core.fixedNotice = { kind: "clockChanged" };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Your Mac's clock looks wrong" });
    expect(
      within(dialog).getByText(
        "Straft Studio checks the date to count your free trial. Turn on Set time and date automatically in System Settings, then choose Check again. Your projects are safe.",
      ),
    ).toBeInTheDocument();
    expect(within(dialog).getAllByRole("button").map((b) => b.textContent)).toEqual(["Check again", "Need help? Contact us"]);
    await userEvent.keyboard("{Escape}");
    expect(screen.getByRole("dialog", { name: "Your Mac's clock looks wrong" })).toBeInTheDocument();
    await userEvent.click(within(dialog).getByRole("button", { name: "Check again" }));
    expect(ops("entitlement.restore_purchase")).toHaveLength(1);
  });

  it("the lock stores typed text first, then tells the core it may close its flush window, once", async () => {
    const order = lockOrder;
    const unregister = registerPendingSave(async () => {
      await new Promise((r) => setTimeout(r, 30));
      order.push("flushed");
    });
    core.state = { state: "trialEnded" };
    try {
      renderWithClient(<LicenceScreens />);
      await screen.findByRole("dialog", { name: "Your free trial has ended" });
      await waitFor(() => expect(order).toEqual(["flushed", "lock_ready"]));
      // Later refreshes of the same lock do not repeat it.
      await act(async () => {
        await new Promise((r) => setTimeout(r, 20));
      });
      expect(ops("entitlement.lock_ready")).toHaveLength(1);
    } finally {
      unregister();
    }
  });

  it("the lock still tells the core when a flush fails, and Escape and an outside click do nothing", async () => {
    const unregister = registerPendingSave(async () => false);
    core.state = { state: "revoked" };
    try {
      renderWithClient(<LicenceScreens />);
      await screen.findByRole("dialog", { name: "Your purchase is no longer active" });
      await waitFor(() => expect(ops("entitlement.lock_ready")).toHaveLength(1));
      await userEvent.keyboard("{Escape}");
      fireEvent.pointerDown(document.querySelector(".scrim") ?? document.body);
      expect(screen.getByRole("dialog", { name: "Your purchase is no longer active" })).toBeInTheDocument();
    } finally {
      unregister();
    }
  });

  it("the contact path is openUrl only: no form, no request from the app", async () => {
    core.state = { state: "trialEnded" };
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    try {
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your free trial has ended" });
      expect(dialog.querySelector("form")).toBeNull();
      expect(dialog.querySelector("input, textarea")).toBeNull();
      await userEvent.click(within(dialog).getByRole("button", { name: "Need help? Contact us" }));
      expect(open).toHaveBeenCalledWith(CONTACT_URL);
      expect(fetchSpy).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllGlobals();
    }
  });

  for (const channel of ["macAppStore", "microsoftStore"] as const) {
    const store = channel === "macAppStore" ? "the App Store" : "the Microsoft Store";

    it(`${channel}: an ended trial locks with only Buy full version and Restore purchase`, async () => {
      core.channel = channel;
      core.state = { state: "trialEnded" };
      core.buyRoute = { route: "store" };
      core.buyMessage = `Straft could not reach ${store}. Connect to the internet, then try again.`;
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your free trial has ended" });
      const again = channel === "macAppStore" ? "Restore purchase" : "Check again";
      expect(within(dialog).getAllByRole("button").map((b) => b.textContent)).toEqual(["Buy full version", again, "Need help? Contact us"]);
      expect(within(dialog).getByText("Buy the full version to keep using Straft Studio.")).toBeInTheDocument();
      const where = channel === "macAppStore" ? "this Mac" : "this computer";
      expect(within(dialog).getByText(`Your projects are safe on ${where}. Buying unlocks them again exactly as you left them.`)).toBeInTheDocument();
      await userEvent.keyboard("{Escape}");
      fireEvent.keyDown(window, { key: "k", ctrlKey: true });
      expect(screen.getByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
      await userEvent.click(within(dialog).getByRole("button", { name: "Buy full version" }));
      expect(await within(dialog).findByText(/could not reach/)).toBeInTheDocument();
      // Restore finds the purchase: the lock goes, every query is asked again, the data note follows.
      const infoAsks = ops("app.info").length;
      core.afterBuy = { state: "purchased" };
      await userEvent.click(within(dialog).getByRole("button", { name: again }));
      expect(await screen.findByRole("dialog", { name: "Your data, your responsibility" })).toBeInTheDocument();
      await waitFor(() => expect(ops("app.info").length).toBeGreaterThan(infoAsks));
    });

    it(`${channel}: a refund locks too, with the refunded wording`, async () => {
      core.channel = channel;
      core.state = { state: "revoked" };
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your purchase is no longer active" });
      const again = channel === "macAppStore" ? "Restore purchase" : "Check again";
      const where = channel === "macAppStore" ? "this Mac" : "this computer";
      expect(within(dialog).getByText("The store no longer shows a purchase for this account, so Straft Studio is locked.")).toBeInTheDocument();
      expect(
        within(dialog).getByText(`Your projects are still on ${where}. If this is a mistake, choose ${again} or contact us.`),
      ).toBeInTheDocument();
      expect(within(dialog).getByRole("button", { name: again })).toBeInTheDocument();
      expect(within(dialog).queryByRole("button", { name: "Close" })).toBeNull();
    });

    it(`${channel}: the contact link opens exactly the contact address, nothing else is called`, async () => {
      core.channel = channel;
      core.state = { state: "trialEnded" };
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your free trial has ended" });
      await userEvent.click(within(dialog).getByRole("button", { name: "Need help? Contact us" }));
      expect(open).toHaveBeenCalledTimes(1);
      expect(open).toHaveBeenCalledWith("https://straftstudio.com/contact/");
      expect(ops("entitlement.buy")).toHaveLength(0);
      expect(useToasts.getState().items).toEqual([]);
    });

    it(`${channel}: a licence that cannot be confirmed asks to try again and can be closed`, async () => {
      core.channel = channel;
      core.state = { state: "needsOnlineCheck" };
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Checking your purchase" });
      expect(within(dialog).getByText(`We could not reach ${store}.`)).toBeInTheDocument();
      expect(within(dialog).getByText("Your projects stay open and can be exported. Nothing is deleted.")).toBeInTheDocument();
      await userEvent.click(within(dialog).getByRole("button", { name: "Need help? Contact us" }));
      expect(open).toHaveBeenCalledWith(CONTACT_URL);
      core.afterBuy = { state: "trialActive", days_left: 4 };
      await userEvent.click(within(dialog).getByRole("button", { name: "Try again" }));
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
      expect(ops("entitlement.restore_purchase")).toHaveLength(1);
    });
  }

  it("the check screen closes with Not now, Escape or Close, and does not hold the app's shortcuts", async () => {
    core.state = { state: "needsOnlineCheck" };
    const seen = vi.fn();
    window.addEventListener("keydown", seen);
    try {
      for (const how of ["Not now", "Escape", "Close"] as const) {
        useCheckScreen.setState({ closed: false });
        const r = renderWithClient(<LicenceScreens />);
        const dialog = await screen.findByRole("dialog", { name: "Checking your purchase" });
        seen.mockClear();
        fireEvent.keyDown(window, { key: "k", ctrlKey: true });
        expect(seen).toHaveBeenCalledTimes(1);
        if (how === "Escape") await userEvent.keyboard("{Escape}");
        else await userEvent.click(within(dialog).getByRole("button", { name: how }));
        await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
        r.unmount();
      }
    } finally {
      window.removeEventListener("keydown", seen);
    }
    expect(ops("entitlement.restore_purchase")).toHaveLength(0);
  });

  it("a closed check screen stays closed while due, the top bar opens it again, and the lock shows when the trial ends", async () => {
    core.state = { state: "needsOnlineCheck" };
    const r = renderWithClient(
      <>
        <TrialCorner />
        <LicenceScreens />
      </>,
    );
    const dialog = await screen.findByRole("dialog", { name: "Checking your purchase" });
    await userEvent.click(within(dialog).getByRole("button", { name: "Not now" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await act(async () => {
      await r.client.invalidateQueries({ queryKey: [NOTICE_OP] });
    });
    await settled(r.client);
    expect(screen.queryByRole("dialog")).toBeNull();
    const group = screen.getByRole("group", { name: "Licence" });
    await userEvent.click(within(group).getByRole("button", { name: "Check purchase" }));
    expect(await screen.findByRole("dialog", { name: "Checking your purchase" })).toBeInTheDocument();
    expect(screen.queryByRole("group", { name: "Licence" })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Not now" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    core.state = { state: "trialEnded" };
    await act(async () => {
      await r.client.invalidateQueries({ queryKey: [NOTICE_OP] });
    });
    expect(await screen.findByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
    expect(screen.queryByRole("group", { name: "Licence" })).toBeNull();
  });
});

// ------------------------------------------------------------------ fixed answers

describe("what the interface draws for each answer", () => {
  it("draws nothing for the answer none, in every state and channel, once the answer has arrived", async () => {
    core.fixedNotice = { kind: "none" };
    for (const channel of ["microsoftStore", "directInstaller", "macAppStore"] as const) {
      for (const state of ["trialEnded", "needsOnlineCheck", "trialNotStarted", "revoked", "unknown", "internalBuild"] as const) {
        core.channel = channel;
        core.state = { state };
        const r = renderWithClient(<LicenceScreens />);
        await settled(r.client);
        expect(screen.queryByRole("dialog")).toBeNull();
        r.unmount();
      }
    }
  });

  it("draws exactly the screen the core names", async () => {
    const cases = [
      [{ kind: "macFirstLaunch" }, "Welcome to Straft Studio"],
      [{ kind: "locked" }, "Your free trial has ended"],
      [{ kind: "checkNeeded" }, "Checking your purchase"],
      [{ kind: "dataNote" }, "Your data, your responsibility"],
      [{ kind: "tutorial" }, "File safety tutorial"],
      [{ kind: "welcome" }, "Welcome to the Straft Studio community"],
      [{ kind: "reminder", daysLeft: 2 }, "Trial reminder"],
    ] as const;
    for (const [notice, title] of cases) {
      core.fixedNotice = notice;
      // The core only names a reminder inside the trial; the screen draws it only then.
      core.state = notice.kind === "reminder" ? { state: "trialActive", days_left: 2 } : { state: "trialEnded" };
      const r = renderWithClient(<LicenceScreens />);
      await settled(r.client);
      expect(await screen.findByRole("dialog", { name: title })).toBeInTheDocument();
      expect(screen.getAllByRole("dialog")).toHaveLength(1);
      r.unmount();
    }
  });
});

// ------------------------------------------------------------------ no price anywhere (ADR-0023)

describe("no price, currency or pricing model on any licensing screen", () => {
  it("finds none in the reminder, the lock, the check, the first launch, the tutorial, the data note, the welcome, the chip and Settings > Licence", async () => {
    const texts: [string, string][] = [];
    const grab = async (where: string, title: string) => {
      const dialog = await screen.findByRole("dialog", { name: title });
      texts.push([where, dialog.textContent ?? ""]);
      return dialog;
    };
    for (const channel of ["macAppStore", "microsoftStore"] as const) {
      core.channel = channel;
      for (const [notice, title, state] of [
        [{ kind: "reminder", daysLeft: 3 }, "Trial reminder", { state: "trialActive", days_left: 3 }],
        [{ kind: "locked" }, "Your free trial has ended", { state: "trialEnded" }],
        [{ kind: "locked" }, "Your purchase is no longer active", { state: "revoked" }],
        [{ kind: "checkNeeded" }, "Checking your purchase", { state: "needsOnlineCheck" }],
        [{ kind: "macFirstLaunch" }, "Welcome to Straft Studio", { state: "trialNotStarted" }],
        [{ kind: "dataNote" }, "Your data, your responsibility", { state: "purchased" }],
        [{ kind: "welcome" }, "Welcome to the Straft Studio community", { state: "purchased" }],
        [{ kind: "clockChanged" }, "Your Mac's clock looks wrong", { state: "clockChanged" }],
      ] as const) {
        core.fixedNotice = notice;
        core.state = state;
        useCheckScreen.setState({ closed: false });
        const r = renderWithClient(<LicenceScreens />);
        await grab(`${channel} ${notice.kind}`, title);
        r.unmount();
      }
      // Every step of the tutorial.
      core.fixedNotice = { kind: "tutorial" };
      core.state = { state: "purchased" };
      const t = renderWithClient(<LicenceScreens />);
      const dialog = await grab(`${channel} tutorial step 1`, "File safety tutorial");
      for (const step of [2, 3, 4]) {
        await userEvent.click(within(dialog).getByRole("button", { name: "Next" }));
        texts.push([`${channel} tutorial step ${step}`, dialog.textContent ?? ""]);
      }
      t.unmount();
      // The corner chip and Settings > Licence.
      core.fixedNotice = { kind: "none" };
      core.state = { state: "trialActive", days_left: 2 };
      const c = renderWithClient(
        <>
          <TrialCorner />
          <LicenceSection />
        </>,
      );
      await screen.findByRole("group", { name: "Free trial" });
      await screen.findByText("Free trial: 3 days left.");
      texts.push([`${channel} chip and Settings > Licence`, document.body.textContent ?? ""]);
      c.unmount();
      const about = renderWithClient(<AboutSection />);
      await screen.findByText("Contact us");
      texts.push([`${channel} About`, document.body.textContent ?? ""]);
      about.unmount();
    }
    expect(texts.length).toBeGreaterThan(20);
    for (const [where, text] of texts) expectNoPriceTalk(text, where);
  });

  it("the scan itself catches price talk", () => {
    for (const bad of ["Only $9.99", "₹499 once", "Costs 12 EUR", "EUR 12", "See the price", "89 dollars"]) {
      expect(() => expectNoPriceTalk(bad, "sample")).toThrow();
    }
    expect(() => expectNoPriceTalk("Your free trial ends in 3 days. 7 days. No credit card. Pay once, own forever.", "sample")).not.toThrow();
  });
});

// The core pushes "entitlement changed" when the store refresh moved the state or a purchase was made
// elsewhere. Everything is asked again, so a lock that lifts or falls reaches every query.
describe("entitlement changed event", () => {
  it("asks every query again, and ignores other events", async () => {
    const r = renderWithClient(<LicenceScreens />);
    await settled(r.client);
    const spy = vi.spyOn(r.client, "invalidateQueries");
    await act(async () => {
      emit({ type: "module", module: "other", name: "changed", payload: null });
      emit({ type: "module", module: "entitlement", name: "other", payload: null });
      emit({ type: "projectClosed", projectId: "p" });
    });
    expect(spy).not.toHaveBeenCalled();
    await act(async () => {
      emit({ type: "module", module: "entitlement", name: "changed", payload: null });
    });
    expect(spy).toHaveBeenCalledWith();
    const before = ops("entitlement.notice").length;
    core.state = { state: "trialEnded" };
    await act(async () => {
      emit({ type: "module", module: "entitlement", name: "changed", payload: null });
    });
    expect(await screen.findByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
    expect(ops("entitlement.notice").length).toBeGreaterThan(before);
  });

  it("stops listening when the screens go away", async () => {
    const r = renderWithClient(<LicenceScreens />);
    await settled(r.client);
    expect(handlers.size).toBe(1);
    r.unmount();
    await waitFor(() => expect(handlers.size).toBe(0));
  });
});

// The day rolls over at local midnight even when nothing else re-renders.
describe("midnight", () => {
  it("asks for the new day's notice when local midnight passes", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date(2026, 10, 4, 23, 59, 30));
    renderWithClient(<TrialCorner />);
    await waitFor(() => expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-04" }));
    await act(async () => {
      vi.advanceTimersByTime(90_000);
    });
    await waitFor(() => expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-05" }));
  });
});

// The interval refresh is what notices a lock that starts while the app is open.
describe("refresh", () => {
  it("asks the core again on its own", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderWithClient(<LicenceScreens />);
    await waitFor(() => expect(ops("entitlement.notice")).toHaveLength(1));
    core.channel = "macAppStore";
    core.state = { state: "trialEnded" };
    await act(async () => {
      vi.advanceTimersByTime(5 * 60_000 + 10);
    });
    expect(await screen.findByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
  });
});

// Review fixes before the store release.
describe("reminder, lock and clock edge cases", () => {
  it("a reminder left open when the trial ends gives way to the lock, which then tells the core", async () => {
    core.state = { state: "trialActive", days_left: 2 };
    renderWithClient(<LicenceScreens />);
    await screen.findByRole("dialog", { name: "Trial reminder" });
    core.state = { state: "trialEnded" };
    await act(async () => {
      emit({ type: "module", module: "entitlement", name: "changed", payload: null });
    });
    expect(await screen.findByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
    expect(screen.queryByRole("dialog", { name: "Trial reminder" })).toBeNull();
    await waitFor(() => expect(ops("entitlement.lock_ready")).toHaveLength(1));
  });

  it("keeps the screen and does not store twice when local midnight passes", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date(2026, 10, 4, 23, 59, 30));
    core.state = { state: "trialEnded" };
    renderWithClient(<LicenceScreens />);
    await screen.findByRole("dialog", { name: "Your free trial has ended" });
    await waitFor(() => expect(ops("entitlement.lock_ready")).toHaveLength(1));
    await act(async () => {
      vi.advanceTimersByTime(90_000);
    });
    await waitFor(() => expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-05" }));
    expect(screen.getByRole("dialog", { name: "Your free trial has ended" })).toBeInTheDocument();
    expect(ops("entitlement.lock_ready")).toHaveLength(1);
  });

  it("still notices the new day after the clock was moved back across midnight", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(new Date(2026, 10, 4, 23, 59, 30));
    renderWithClient(<TrialCorner />);
    await waitFor(() => expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-04" }));
    vi.setSystemTime(new Date(2026, 10, 4, 12, 0, 0));
    await act(async () => {
      vi.advanceTimersByTime(90_000);
    });
    expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-04" });
    vi.setSystemTime(new Date(2026, 10, 5, 10, 0, 0));
    await act(async () => {
      vi.advanceTimersByTime(61_000);
    });
    await waitFor(() => expect(ops("entitlement.notice").at(-1)?.args).toEqual({ today: "2026-11-05" }));
  });

  it("warns on the lock when the latest typing could not be saved, and still tells the core", async () => {
    const unregister = registerPendingSave(async () => false);
    core.state = { state: "trialEnded" };
    try {
      renderWithClient(<LicenceScreens />);
      const dialog = await screen.findByRole("dialog", { name: "Your free trial has ended" });
      expect(
        await within(dialog).findByText("Some of your latest typing could not be saved. Your projects are safe up to your last saved change."),
      ).toBeInTheDocument();
      await waitFor(() => expect(ops("entitlement.lock_ready")).toHaveLength(1));
    } finally {
      unregister();
    }
  });

  it("shows no typing warning when everything was saved", async () => {
    core.state = { state: "trialEnded" };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Your free trial has ended" });
    await waitFor(() => expect(ops("entitlement.lock_ready")).toHaveLength(1));
    expect(within(dialog).queryByText(/could not be saved/)).toBeNull();
  });

  it("puts the contact link under the buttons on the check screen", async () => {
    core.state = { state: "needsOnlineCheck" };
    renderWithClient(<LicenceScreens />);
    const dialog = await screen.findByRole("dialog", { name: "Checking your purchase" });
    const contact = within(dialog).getByRole("button", { name: "Need help? Contact us" });
    const again = within(dialog).getByRole("button", { name: "Try again" });
    expect(contact.parentElement).not.toBe(again.parentElement);
    expect(again.compareDocumentPosition(contact) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
