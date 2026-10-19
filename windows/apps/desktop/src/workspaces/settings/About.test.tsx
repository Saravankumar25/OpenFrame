import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { call } from "../../ipc/client";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import { AboutSection } from "./About";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);

const status = (channel: string) => ({
  channel,
  state: { state: channel === "directInstaller" ? "internalBuild" : "purchased" },
  summary: "",
  limitation: null,
  trialDays: 7,
  canEdit: true,
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
});
