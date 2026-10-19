import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { BudgetDto, BudgetView } from "../../../api/schedule";
import BudgetTab from "./budget";

// Test fixtures only: the IPC boundary is replaced by a recorder and a fixed budget.
const calls: { op: string; args: Record<string, unknown> }[] = [];
let view: BudgetView;

vi.mock("../../../ipc/client", () => ({
  call: vi.fn(async (op: string, args: Record<string, unknown> = {}) => {
    calls.push({ op, args });
    return null;
  }),
}));
vi.mock("../../../ipc/query", () => ({
  reportError: vi.fn(),
  useOp: vi.fn(() => ({ data: view, isLoading: false, error: null })),
}));
vi.mock("../../../app/toast", () => ({ toast: { undoable: vi.fn(), info: vi.fn(), error: vi.fn() } }));
vi.mock("../../../features/export/buttons", () => ({ BudgetExportButton: () => null }));

const CATS = ["Cast", "Crew", "Locations", "Equipment", "Art/Props", "Travel/Transport", "Food", "Post/Other", "Contingency"];

function budget(): BudgetDto {
  return {
    id: "b1",
    label: null,
    currency: "USD",
    plannedTotal: null,
    contingencyMode: "percent",
    contingencyValue: 1000,
    contingencyAmount: 17000,
    plannedLinesTotal: 170000,
    actualTotal: 55000,
    difference: -5000,
    actualCount: 1,
    categories: CATS.map((c) => ({
      category: c,
      planned: c === "Cast" ? 50000 : c === "Food" ? 120000 : 0,
      actual: c === "Cast" ? 55000 : 0,
      difference: c === "Cast" ? -5000 : 0,
      count: c === "Cast" || c === "Food" ? 1 : 0,
      actualCount: c === "Cast" ? 1 : 0,
    })),
    lines: [
      { id: "l1", category: "Cast", description: "Lead actor", planned: 50000, actual: 55000, difference: -5000, notes: null, rev: 1 },
      { id: "l2", category: "Food", description: "Catering", planned: 120000, actual: null, difference: null, notes: null, rev: 1 },
    ],
    notes: null,
    isCurrent: true,
    frozenAt: null,
    rev: 1,
  } as BudgetDto;
}

describe("budget with Planned and Actual", () => {
  beforeEach(() => {
    calls.length = 0;
    view = { categories: CATS, current: budget(), snapshots: [], reviewReminder: null };
  });

  it("shows Planned, Actual and the difference in words, not colour alone", () => {
    render(<BudgetTab />);
    const table = screen.getByRole("table", { name: "Budget lines" });
    const lead = within(table).getByText("Lead actor").closest("tr")!;
    expect(lead).toHaveTextContent("$500");
    expect(lead).toHaveTextContent("$550");
    expect(lead).toHaveTextContent("over plan");
    const catering = within(table).getByText("Catering").closest("tr")!;
    expect(catering).toHaveTextContent("Not entered");
    expect(catering).toHaveTextContent("Not spent yet");
    expect(screen.getByText("1 of 2 lines have an Actual amount")).toBeInTheDocument();
  });

  it("adds a line with a Planned amount and an optional Actual amount", async () => {
    render(<BudgetTab />);
    fireEvent.click(screen.getByRole("button", { name: "Add Line" }));
    fireEvent.change(screen.getByLabelText(/Line item/), { target: { value: "Permit" } });
    fireEvent.change(screen.getByLabelText(/Planned \(USD\)/), { target: { value: "250" } });
    const dialog = screen.getByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: "Add Line" }));
    await waitFor(() => expect(calls.map((c) => c.op)).toEqual(["budget.add_line"]));
    expect(calls[0].args).toMatchObject({ budgetId: "b1", description: "Permit", planned: 25000, actual: null });
  });

  it("refuses an Actual amount that is not a number and sends nothing", async () => {
    render(<BudgetTab />);
    fireEvent.click(screen.getByRole("button", { name: "Add Line" }));
    fireEvent.change(screen.getByLabelText(/Line item/), { target: { value: "Permit" } });
    fireEvent.change(screen.getByLabelText(/Planned \(USD\)/), { target: { value: "250" } });
    fireEvent.change(screen.getByLabelText(/Actual \(USD\)/), { target: { value: "abc" } });
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Add Line" }));
    await screen.findByText(/Enter the actual amount as a non-negative number/);
    expect(calls).toEqual([]);
  });
});
