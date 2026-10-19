import { beforeEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { call } from "../../ipc/client";
import type { TeamGateStatus } from "../../ipc/generated/TeamGateStatus";
import { answerFrom, renderWithClient } from "../../app/home/testUtils";
import { TeamGate, waitClock } from "./TeamGate";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
vi.setConfig({ testTimeout: 20_000 });

const locked: TeamGateStatus = { required: true, unlocked: false, waitSeconds: 0, triesBeforeWait: 5 };

describe("Team password gate", () => {
  // Braces: a function returned from beforeEach is run as a clean-up hook after each test.
  beforeEach(() => {
    callMock.mockReset();
  });

  it("opens the app straight away in a store build", async () => {
    callMock.mockImplementation(
      answerFrom({ "team_gate.status": { required: false, unlocked: true, waitSeconds: 0, triesBeforeWait: 0 } }) as typeof call,
    );
    renderWithClient(<TeamGate><p>The app</p></TeamGate>);
    expect(await screen.findByText("The app")).toBeInTheDocument();
  });

  it("shows nothing of the app until the password is right", async () => {
    const user = userEvent.setup();
    const unlock = vi
      .fn()
      .mockReturnValueOnce({ status: { ...locked, triesBeforeWait: 4 }, message: "That password is not right. 4 tries left before a wait." })
      .mockReturnValueOnce({ status: { ...locked, unlocked: true }, message: null });
    callMock.mockImplementation(answerFrom({ "team_gate.status": locked, "team_gate.unlock": unlock }) as typeof call);
    renderWithClient(<TeamGate><p>The app</p></TeamGate>);
    const input = await screen.findByLabelText("Team password");
    expect(screen.queryByText("The app")).not.toBeInTheDocument();
    expect(screen.getByText(/internal build of OpenFrame Studio for the team/)).toBeInTheDocument();

    await user.type(input, "wrong one");
    await user.click(screen.getByRole("button", { name: "Open OpenFrame Studio" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("4 tries left before a wait");
    expect(unlock).toHaveBeenCalledWith({ password: "wrong one" });
    expect(screen.getByLabelText("Team password")).toHaveValue("");
    expect(screen.queryByText("The app")).not.toBeInTheDocument();

    await user.type(screen.getByLabelText("Team password"), "right one{Enter}");
    expect(await screen.findByText("The app")).toBeInTheDocument();
  });

  it("holds every try while a wait runs and says how long is left", async () => {
    callMock.mockImplementation(answerFrom({ "team_gate.status": { ...locked, waitSeconds: 65 } }) as typeof call);
    renderWithClient(<TeamGate><p>The app</p></TeamGate>);
    expect(await screen.findByText("Next try in 1:05.")).toBeInTheDocument();
    expect(screen.getByLabelText("Team password")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Open OpenFrame Studio" })).toBeDisabled();
    expect(waitClock(9)).toBe("0:09");
    expect(waitClock(1800)).toBe("30:00");
  });

  it("offers to try again when the check cannot start, and never opens the app", async () => {
    callMock.mockImplementation(() => Promise.reject(new Error("ipc down")));
    renderWithClient(<TeamGate><p>The app</p></TeamGate>);
    expect(await screen.findByRole("alert")).toHaveTextContent("could not start the password check");
    await waitFor(() => expect(screen.queryByText("The app")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Try again" })).toBeInTheDocument();
  });
});
