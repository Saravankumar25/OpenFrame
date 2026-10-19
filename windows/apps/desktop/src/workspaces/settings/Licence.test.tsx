import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { LicenceSection } from "./Licence";
import { renderWithClient } from "../../app/home/testUtils";

// Test fixtures only: the IPC boundary is replaced.
const calls: string[] = [];
let statusFail: { code: string; message: string } | null = null;
let restoreFail: { code: string; message: string } | "unexpected" | null = null;
let current: Record<string, unknown> | null = null;

const trial = {
  channel: "microsoftStore",
  state: { state: "trialActive", days_left: 4 },
  summary: "Free trial: 4 days left.",
  limitation: "The Microsoft Store connection is not available in this build.",
  trialDays: 7,
  canEdit: true,
};
const purchased = {
  channel: "microsoftStore",
  state: { state: "purchased" },
  summary: "OpenFrame Studio is unlocked. Thank you.",
  limitation: null,
  trialDays: 7,
  canEdit: true,
};

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
    onAppEvent: () => Promise.resolve(() => undefined),
    call: vi.fn(async (op: string) => {
      calls.push(op);
      if (op === "entitlement.status") {
        if (statusFail) throw new OpError(statusFail.code, statusFail.message);
        return current ?? trial;
      }
      if (op === "entitlement.restore_purchase") {
        if (restoreFail === "unexpected") throw new TypeError("boom");
        if (restoreFail) throw new OpError(restoreFail.code, restoreFail.message);
        return purchased;
      }
      return null;
    }),
  };
});

beforeEach(() => {
  calls.length = 0;
  statusFail = null;
  restoreFail = null;
  current = null;
});

function section() {
  return screen.getByRole("region", { name: "Licence" });
}

describe("Settings > Licence", () => {
  it("shows the summary, the channel in plain words and the limitation", async () => {
    renderWithClient(<LicenceSection />);
    expect(await screen.findByText("Free trial: 4 days left.")).toBeInTheDocument();
    expect(within(section()).getByText("Installed from: Microsoft Store")).toBeInTheDocument();
    expect(within(section()).getByText("The Microsoft Store connection is not available in this build.")).toBeInTheDocument();
    expect(calls).toEqual(["entitlement.status"]);
  });

  it("restores a purchase from the keyboard and shows the new state", async () => {
    const user = userEvent.setup();
    renderWithClient(<LicenceSection />);
    await screen.findByText("Free trial: 4 days left.");
    await user.tab();
    const button = screen.getByRole("button", { name: "Restore purchase" });
    expect(button).toHaveFocus();
    await user.keyboard("{Enter}");
    expect(await screen.findByText("OpenFrame Studio is unlocked. Thank you.")).toBeInTheDocument();
    expect(calls).toEqual(["entitlement.status", "entitlement.restore_purchase"]);
    expect(within(section()).getByText("Installed from: Microsoft Store")).toBeInTheDocument();
    expect(within(section()).queryByText(/not available in this build/)).toBeNull();
    expect(within(section()).getByText("Checked again.")).toBeInTheDocument();
  });

  it("shows a refused restore in plain words and keeps the button usable", async () => {
    restoreFail = { code: "permission.denied", message: "Only the person using this computer can restore a purchase." };
    const user = userEvent.setup();
    renderWithClient(<LicenceSection />);
    await screen.findByText("Free trial: 4 days left.");
    await user.click(screen.getByRole("button", { name: "Restore purchase" }));
    expect(await within(section()).findByRole("alert")).toHaveTextContent("Only the person using this computer");
    expect(screen.getByRole("button", { name: "Restore purchase" })).toBeEnabled();
    expect(screen.getByText("Free trial: 4 days left.")).toBeInTheDocument();
  });

  it("never throws on an unexpected failure and says so plainly", async () => {
    restoreFail = "unexpected";
    const user = userEvent.setup();
    renderWithClient(<LicenceSection />);
    await screen.findByText("Free trial: 4 days left.");
    await user.click(screen.getByRole("button", { name: "Restore purchase" }));
    expect(await within(section()).findByRole("alert")).toHaveTextContent("could not restore your purchase right now");
  });

  it("explains when the status cannot be loaded", async () => {
    statusFail = { code: "internal.unexpected", message: "Something went wrong inside OpenFrame. Your project was not changed." };
    renderWithClient(<LicenceSection />);
    expect(await within(section()).findByRole("alert")).toHaveTextContent("Something went wrong inside OpenFrame");
    expect(screen.getByRole("button", { name: "Restore purchase" })).toBeEnabled();
  });

  it("says the internal installer build is not for sale and offers nothing to restore", async () => {
    current = {
      channel: "directInstaller",
      state: { state: "internalBuild" },
      summary: "Internal build, not for sale.",
      limitation: null,
      trialDays: 7,
      canEdit: true,
    };
    renderWithClient(<LicenceSection />);
    expect(await screen.findByText("Internal build, not for sale.")).toBeInTheDocument();
    expect(within(section()).getByText("Installed from: Internal build")).toBeInTheDocument();
    expect(within(section()).queryByRole("button", { name: "Restore purchase" })).toBeNull();
  });

  it("uses no em dashes in what it shows", async () => {
    renderWithClient(<LicenceSection />);
    await screen.findByText("Free trial: 4 days left.");
    expect(document.body.textContent ?? "").not.toMatch(new RegExp(String.fromCharCode(0x2014)));
  });
});
