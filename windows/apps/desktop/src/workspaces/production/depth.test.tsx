// Production depth pieces (Production depth contract section 3): availability windows,
// contact fields under the contact privacy rule, and the references shown before a delete.
// Rust validates and stores; these tests prove what the components send and show.

import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { AvailabilityWindowDto } from "../../ipc/generated/AvailabilityWindowDto";
import { AvailabilityEditor, ContactField, ReferencesNote } from "./components/depth";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a), convertFileSrc: (p: string) => p }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

function wrap(ui: ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>);
}

const exams: AvailabilityWindowDto = { id: "w1", startDate: "2027-06-09", endDate: "2027-06-20", kind: "Unavailable", note: "పరీక్షలు (exams)" };

describe("availability editor", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue([]);
  });

  it("lists dated windows in words, not by colour alone", () => {
    wrap(<AvailabilityEditor ownerType="cast_member" ownerId="c1" windows={[exams]} canEdit name="Karthik Menon" />);
    const list = screen.getByRole("list", { name: "Availability of Karthik Menon" });
    expect(list).toHaveTextContent("Not available");
    expect(list).toHaveTextContent("పరీక్షలు (exams)");
  });

  it("adding a window sends the whole list so the change is one undo step", async () => {
    wrap(<AvailabilityEditor ownerType="location" ownerId="l1" windows={[exams]} canEdit name="Old Railway Station" />);
    fireEvent.change(screen.getByLabelText("First day"), { target: { value: "2027-07-01" } });
    fireEvent.change(screen.getByLabelText("Availability note"), { target: { value: "  स्टेशन बंद  " } });
    fireEvent.click(screen.getByRole("button", { name: "Add Dates" }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("of_invoke", {
        op: "availability.set",
        args: {
          ownerType: "location",
          ownerId: "l1",
          windows: [
            { startDate: "2027-06-09", endDate: "2027-06-20", kind: "Unavailable", note: "పరీక్షలు (exams)" },
            { startDate: "2027-07-01", endDate: "2027-07-01", kind: "Unavailable", note: "स्टेशन बंद" },
          ],
        },
      }),
    );
  });

  it("Add Dates waits for a first day", () => {
    wrap(<AvailabilityEditor ownerType="crew_member" ownerId="k1" windows={[]} canEdit name="Priya Nambiar" />);
    expect(screen.getByText("No dates entered.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Add Dates" })).toBeDisabled();
  });

  it("removing a window sends the rest of the list", async () => {
    wrap(<AvailabilityEditor ownerType="cast_member" ownerId="c1" windows={[exams]} canEdit name="Karthik Menon" />);
    fireEvent.click(screen.getByRole("button", { name: /^Remove / }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "availability.set", args: { ownerType: "cast_member", ownerId: "c1", windows: [] } }),
    );
  });

  it("read-only roles see the dates and no editing controls", () => {
    wrap(<AvailabilityEditor ownerType="cast_member" ownerId="c1" windows={[exams]} canEdit={false} name="Karthik Menon" />);
    expect(screen.queryByRole("button", { name: "Add Dates" })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Remove / })).toBeNull();
    expect(screen.getByRole("list", { name: "Availability of Karthik Menon" })).toHaveTextContent("Not available");
  });
});

describe("contact field", () => {
  it("roles that may not see contacts get a statement and no value", () => {
    wrap(<ContactField id="c" label="Contact" value={null} hidden canEdit={false} onCommit={() => {}} />);
    expect(screen.getByText("Contact details are hidden for your role.")).toBeInTheDocument();
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("Owners and Editors see the value with the privacy hint", () => {
    wrap(<ContactField id="c" label="Contact" value="98xxx 10001" hidden={false} canEdit onCommit={() => {}} />);
    expect(screen.getByDisplayValue("98xxx 10001")).toBeInTheDocument();
    expect(screen.getByText(/Never shared with the assistant or put in exports/)).toBeInTheDocument();
  });
});

describe("references before a delete", () => {
  beforeEach(() => invoke.mockReset());

  it("names the scenes, the linked records and what the delete does", async () => {
    invoke.mockImplementation(async (_cmd: string, a?: { op: string }) =>
      a?.op === "production.references"
        ? {
            name: "Old Railway Station",
            scenes: [{ sceneId: "s1", number: "13", heading: "EXT. OLD RAILWAY STATION - NIGHT", inSource: true }],
            links: ["Catalog item: Old Railway Station"],
            consequence: "Deleting moves it to Recently Deleted. The scenes keep their text.",
          }
        : null,
    );
    wrap(<ReferencesNote kind="location" id="l1" />);
    expect(await screen.findByText(/Deleting moves it to Recently Deleted/)).toBeInTheDocument();
    expect(screen.getByText("Used in")).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "Linked records" })).toHaveTextContent("Catalog item: Old Railway Station");
    expect(invoke).toHaveBeenCalledWith("of_invoke", { op: "production.references", args: { kind: "location", id: "l1" } });
  });
});
