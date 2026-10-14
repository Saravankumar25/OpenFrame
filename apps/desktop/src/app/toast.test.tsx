// Toasts: the optional action (Undo, Change shortcuts, ...) runs a command, so it is a button
// that a keyboard and a screen reader treat as one, and it closes the toast.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toaster, toast, useToasts } from "./toast";

afterEach(() => {
  act(() => useToasts.setState({ items: [] }));
});

describe("toast action", () => {
  it("is a button that runs its command and closes the toast", async () => {
    const run = vi.fn();
    render(<Toaster />);
    act(() => toast.info("Shortcuts: Ctrl+N new project", { label: "Change shortcuts", run }));
    const action = screen.getByRole("button", { name: "Change shortcuts" });
    expect(screen.queryByRole("link")).toBeNull();
    await userEvent.click(action);
    expect(run).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("Shortcuts: Ctrl+N new project")).toBeNull();
  });

  it("works from the keyboard with Space as well as Enter", async () => {
    const run = vi.fn();
    render(<Toaster />);
    act(() => toast.info("Moved to Recently Deleted", { label: "Undo", run }));
    screen.getByRole("button", { name: "Undo" }).focus();
    await userEvent.keyboard(" ");
    expect(run).toHaveBeenCalledTimes(1);
  });
});
