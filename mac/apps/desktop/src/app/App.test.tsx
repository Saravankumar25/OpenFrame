// The application root: the splash covers only the first answer of `project.current`. Under the
// store lock the core refuses that read, and Home's packages overlay reads it too; showing the
// splash again on every refetch unmounted Home, the overlay asked again when it mounted, and the
// window stayed on "Opening your local projects…" in a loop of refused calls.

import { afterEach, describe, expect, it, vi } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import { useOp } from "../ipc/query";
import { renderWithClient } from "./home/testUtils";

const calls: string[] = [];
vi.mock("../ipc/client", async (orig) => {
  const real = await orig<typeof import("../ipc/client")>();
  return {
    ...real,
    call: vi.fn(async (op: string) => {
      calls.push(op);
      if (op === "project.current") {
        throw new real.OpError(op, { code: "permission.licence_required", message: "Buy OpenFrame Studio or restore your purchase.", retryable: false });
      }
      throw new Error(`unexpected operation in test: ${op}`);
    }),
    onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
  };
});
vi.mock("../features/library-setup/LibrarySetup", () => ({ LibrarySetupGate: ({ children }: { children: React.ReactNode }) => <>{children}</> }));
vi.mock("../theme/useAppearance", () => ({ useAppearance: () => undefined }));
vi.mock("./shortcuts", () => ({ useShortcutSync: () => undefined }));
vi.mock("../features/licensing/LicenceScreens", () => ({ LicenceScreens: () => <div role="dialog" aria-label="Your free trial has ended" /> }));
vi.mock("./shell/Shell", () => ({ Shell: () => <div>Project shell</div> }));
// Home with a child that reads the current project as well, like the packages overlay.
vi.mock("./home/AppHome", () => ({
  AppHome: () => {
    useOp("project.current", {}, ["project"]);
    return <main aria-label="Your projects" />;
  },
}));

const { App } = await import("./App");

afterEach(() => {
  calls.length = 0;
});

describe("App root under the store lock", () => {
  it("leaves the splash after the first refused answer and does not ask again in a loop", async () => {
    renderWithClient(<App />);
    await screen.findByRole("dialog", { name: "Your free trial has ended" });
    expect(screen.queryByText("Opening your local projects…")).toBeNull();
    // Give a loop time to show itself: a handful of asks at most, not hundreds.
    await new Promise((r) => setTimeout(r, 300));
    await waitFor(() => expect(screen.getByRole("main", { name: "Your projects" })).toBeInTheDocument());
    expect(calls.filter((c) => c === "project.current").length).toBeLessThanOrEqual(3);
  });
});
