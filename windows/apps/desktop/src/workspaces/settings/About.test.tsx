import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { call, openUrl } from "../../ipc/client";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import { AboutSection } from "./About";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
  openUrl: vi.fn(async () => undefined),
}));

const callMock = vi.mocked(call);

const status = (channel: string) => ({
  channel,
  state: { state: channel === "directInstaller" ? "internalBuild" : "purchased" },
  summary: "",
  limitation: null,
  trialDays: 7,
  canEdit: true,
  trialEndsAtMs: null,
});

describe("About", () => {
  // Braces: a function returned from beforeEach is run as a clean-up hook after each test.
  beforeEach(() => {
    callMock.mockReset();
  });

  it("shows the version and says an internal build is not for sale", async () => {
    callMock.mockImplementation(
      answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("directInstaller") }) as typeof call,
    );
    renderWithClient(<AboutSection />);
    expect(await screen.findByText("Version 0.4.2")).toBeInTheDocument();
    expect(await screen.findByText("Internal build, not for sale.")).toBeInTheDocument();
  });

  it("shows the version only in a store build", async () => {
    callMock.mockImplementation(
      answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("microsoftStore") }) as typeof call,
    );
    renderWithClient(<AboutSection />);
    expect(await screen.findByText("Version 0.4.2")).toBeInTheDocument();
    expect(screen.queryByText("Internal build, not for sale.")).not.toBeInTheDocument();
  });

  it("shows the publisher, the address and the copyright", async () => {
    callMock.mockImplementation(answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("microsoftStore") }) as typeof call);
    renderWithClient(<AboutSection />);
    expect(await screen.findByText("Publisher: Vighnajith Advisories Private Limited")).toBeInTheDocument();
    expect(screen.getByText("12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India")).toBeInTheDocument();
    expect(screen.getByText("Copyright 2026 Vighnajith Advisories Private Limited")).toBeInTheDocument();
    expect(screen.getByText(/Gemma Terms of Use and its Prohibited Use Policy, and llama.cpp/)).toBeInTheDocument();
  });

  it("shows the same rows in the internal build", async () => {
    callMock.mockImplementation(answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("directInstaller") }) as typeof call);
    renderWithClient(<AboutSection />);
    expect(await screen.findByText("Internal build, not for sale.")).toBeInTheDocument();
    expect(screen.getByText("Publisher: Vighnajith Advisories Private Limited")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Contact us" })).toBeInTheDocument();
  });

  it("Contact us opens exactly the contact page in the browser and nothing else", async () => {
    callMock.mockImplementation(answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("microsoftStore") }) as typeof call);
    renderWithClient(<AboutSection />);
    await userEvent.click(await screen.findByRole("button", { name: "Contact us" }));
    expect(vi.mocked(openUrl)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(openUrl)).toHaveBeenCalledWith("https://straftstudio.com/contact/");
  });

  it("opens the third-party licences in a dialog", async () => {
    callMock.mockImplementation(answerFrom({ "app.info": { appVersion: "0.4.2" }, "entitlement.status": status("microsoftStore") }) as typeof call);
    renderWithClient(<AboutSection />);
    expect(screen.queryByRole("dialog")).toBeNull();
    await userEvent.click(await screen.findByRole("button", { name: "Third-party licences" }));
    const dialog = await screen.findByRole("dialog", { name: "Third-party licences" });
    expect(within(dialog).getByText(/Third-Party Licenses: Straft Studio/)).toBeInTheDocument();
  });
});
