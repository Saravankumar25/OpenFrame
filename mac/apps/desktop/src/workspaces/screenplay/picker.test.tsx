import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { call } from "../../ipc/client";
import type { ScreenplayDto } from "../../ipc/generated/ScreenplayDto";
import type { ScreenplaySummary } from "../../ipc/generated/ScreenplaySummary";
import { answerFrom } from "../../app/home/testUtils";
import { useOverview } from "./api";
import { DeleteScreenplayDialog, NewScreenplayDialog, RenameScreenplayDialog, ScreenplayPicker, titleTaken } from "./picker";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
vi.setConfig({ testTimeout: 20_000 });

const summary = (p: Partial<ScreenplaySummary>): ScreenplaySummary => ({
  id: "s1",
  episodeId: null,
  title: "Black Rain",
  format: "Feature",
  currentDraftId: "d1",
  currentDraftName: "Draft 1",
  draftCount: 1,
  createdAt: 1,
  ...p,
});

const SCREENPLAYS = [summary({ id: "s1", title: "Black Rain", draftCount: 3 }), summary({ id: "s2", title: "Black Rain (short)", currentDraftId: "d9" })];

const dto = (p: Partial<ScreenplayDto>): ScreenplayDto =>
  ({ id: "s1", episodeId: null, title: "Black Rain", format: "Feature", currentDraftId: "d1", titlePage: {}, pageSetup: {}, rev: 1, ...p }) as ScreenplayDto;

describe("Screenplay picker", () => {
  beforeEach(() => {
    callMock.mockReset();
  });

  it("names the open screenplay and switches to another one from the keyboard", async () => {
    const user = userEvent.setup();
    const onSwitch = vi.fn();
    render(<ScreenplayPicker screenplays={SCREENPLAYS} openId="s1" canEdit onSwitch={onSwitch} onNew={vi.fn()} onRename={vi.fn()} />);
    const trigger = screen.getByRole("button", { name: "Screenplay: Black Rain. Switch screenplay" });
    trigger.focus();
    await user.keyboard("{Enter}");
    const open = await screen.findByRole("menuitem", { name: "Black Rain · 3 drafts · Open" });
    expect(open).toHaveAttribute("data-disabled");
    await user.click(screen.getByRole("menuitem", { name: "Black Rain (short) · 1 draft" }));
    expect(onSwitch).toHaveBeenCalledWith("s2");
  });

  it("offers New and Rename only to roles that may edit", async () => {
    const user = userEvent.setup();
    const onNew = vi.fn();
    const { unmount } = render(<ScreenplayPicker screenplays={SCREENPLAYS} openId="s1" canEdit onSwitch={vi.fn()} onNew={onNew} onRename={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /Switch screenplay/ }));
    await user.click(await screen.findByRole("menuitem", { name: "New Screenplay…" }));
    expect(onNew).toHaveBeenCalled();
    unmount();

    render(<ScreenplayPicker screenplays={SCREENPLAYS} openId="s2" canEdit={false} onSwitch={vi.fn()} onNew={vi.fn()} onRename={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Screenplay: Black Rain (short). Switch screenplay" }));
    await screen.findByRole("menuitem", { name: /Black Rain · 3 drafts/ });
    expect(screen.queryByRole("menuitem", { name: "New Screenplay…" })).not.toBeInTheDocument();
    expect(screen.queryByRole("menuitem", { name: "Rename Screenplay…" })).not.toBeInTheDocument();
  });

  it("creates a second screenplay with the typed title and refuses a title already in use", async () => {
    const user = userEvent.setup();
    const create = vi.fn((a: Record<string, unknown>) => ({ screenplay: dto({ id: "s3", title: String(a.title) }), draft: { id: "d3" } }));
    callMock.mockImplementation(answerFrom({ "screenplay.create": create }) as typeof call);
    const onCreated = vi.fn();
    render(<NewScreenplayDialog episodeId={null} screenplays={SCREENPLAYS} onClose={vi.fn()} onCreated={onCreated} />);
    const field = screen.getByRole("textbox", { name: /Title/ });
    await user.type(field, "  black rain ");
    expect(screen.getByRole("alert")).toHaveTextContent("Another screenplay here already has this title");
    expect(screen.getByRole("button", { name: "Create Screenplay" })).toBeDisabled();

    await user.clear(field);
    await user.type(field, "Monsoon{Enter}");
    await waitFor(() => expect(create).toHaveBeenCalledWith({ title: "Monsoon" }));
    await waitFor(() => expect(onCreated).toHaveBeenCalledWith(expect.objectContaining({ screenplay: expect.objectContaining({ id: "s3" }) })));
  });

  it("sends the episode for a screenplay in a series", async () => {
    const user = userEvent.setup();
    const create = vi.fn(() => ({ screenplay: dto({ id: "s4", title: "Pilot B", episodeId: "e1" }), draft: { id: "d4" } }));
    callMock.mockImplementation(answerFrom({ "screenplay.create": create }) as typeof call);
    render(<NewScreenplayDialog episodeId="e1" screenplays={[]} onClose={vi.fn()} onCreated={vi.fn()} />);
    await user.type(screen.getByRole("textbox", { name: /Title/ }), "Pilot B");
    await user.click(screen.getByRole("button", { name: "Create Screenplay" }));
    await waitFor(() => expect(create).toHaveBeenCalledWith({ title: "Pilot B", episodeId: "e1" }));
  });

  it("renames the open screenplay and shows the core's reason when it refuses", async () => {
    const user = userEvent.setup();
    let refuse = true;
    const rename = vi.fn((a: Record<string, unknown>) => {
      if (refuse) throw Object.assign(new Error("Title is too long."), { code: "validation.invalid_input", message: "Title is too long." });
      return dto({ title: String(a.title) });
    });
    callMock.mockImplementation(answerFrom({ "screenplay.rename": rename }) as typeof call);
    const onClose = vi.fn();
    render(<RenameScreenplayDialog screenplay={dto({})} screenplays={SCREENPLAYS} onClose={onClose} />);
    const field = screen.getByRole("textbox", { name: /Title/ });
    expect(screen.getByRole("button", { name: "Rename" })).toBeDisabled();
    await user.clear(field);
    await user.type(field, "Black Rain (short)");
    expect(screen.getByRole("button", { name: "Rename" })).toBeDisabled();
    await user.clear(field);
    await user.type(field, "Black Rain (feature)");
    await user.click(screen.getByRole("button", { name: "Rename" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Title is too long.");
    expect(onClose).not.toHaveBeenCalled();
    refuse = false;
    await user.click(screen.getByRole("button", { name: "Rename" }));
    await waitFor(() => expect(rename).toHaveBeenLastCalledWith({ screenplayId: "s1", title: "Black Rain (feature)" }));
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("offers Delete only to roles that may delete, from the keyboard", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn();
    const { unmount } = render(
      <ScreenplayPicker screenplays={SCREENPLAYS} openId="s1" canEdit canDelete onSwitch={vi.fn()} onNew={vi.fn()} onRename={vi.fn()} onDelete={onDelete} />,
    );
    screen.getByRole("button", { name: /Switch screenplay/ }).focus();
    await user.keyboard("{Enter}");
    const item = await screen.findByRole("menuitem", { name: "Delete Screenplay…" });
    item.focus();
    await user.keyboard("{Enter}");
    expect(onDelete).toHaveBeenCalled();
    unmount();

    render(<ScreenplayPicker screenplays={SCREENPLAYS} openId="s1" canEdit={false} canDelete={false} onSwitch={vi.fn()} onNew={vi.fn()} onRename={vi.fn()} onDelete={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /Switch screenplay/ }));
    await screen.findByRole("menuitem", { name: /Black Rain · 3 drafts/ });
    expect(screen.queryByRole("menuitem", { name: "Delete Screenplay…" })).not.toBeInTheDocument();
  });

  it("confirms by name, moves the screenplay to Recently Deleted and opens another one", async () => {
    const user = userEvent.setup();
    const remove = vi.fn(() => null);
    callMock.mockImplementation(answerFrom({ "screenplay.delete": remove }) as typeof call);
    const onDeleted = vi.fn();
    const onClose = vi.fn();
    render(<DeleteScreenplayDialog screenplay={dto({ id: "s2", title: "Black Rain (short)" })} screenplays={SCREENPLAYS} onClose={onClose} onDeleted={onDeleted} />);
    expect(screen.getByRole("heading", { name: "Delete “Black Rain (short)”?" })).toBeInTheDocument();
    expect(screen.getByText(/move to Recently Deleted\. They stay there for 30 days/)).toBeInTheDocument();
    expect(screen.getByText("Your other screenplays are not changed.")).toBeInTheDocument();
    // Cancel changes nothing.
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalled();
    expect(remove).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Delete Screenplay" }));
    await waitFor(() => expect(remove).toHaveBeenCalledWith({ screenplayId: "s2" }));
    await waitFor(() => expect(onDeleted).toHaveBeenCalledWith("s1"));
  });

  it("shows the core's reason when the screenplay is the Production Source, and allows the last one", async () => {
    const user = userEvent.setup();
    const message =
      "“Black Rain” is the Production Source for Breakdown, Schedule and Call Sheets, so it can't be deleted. Choose a draft of another screenplay as the Production Source first.";
    const remove = vi.fn(() => {
      throw Object.assign(new Error(message), { code: "conflict.state", message });
    });
    callMock.mockImplementation(answerFrom({ "screenplay.delete": remove }) as typeof call);
    const onDeleted = vi.fn();
    const { unmount } = render(<DeleteScreenplayDialog screenplay={dto({})} screenplays={SCREENPLAYS} onClose={vi.fn()} onDeleted={onDeleted} />);
    await user.click(screen.getByRole("button", { name: "Delete Screenplay" }));
    expect(await screen.findByText(message)).toBeInTheDocument();
    expect(onDeleted).not.toHaveBeenCalled();
    unmount();

    callMock.mockImplementation(answerFrom({ "screenplay.delete": () => null }) as typeof call);
    render(<DeleteScreenplayDialog screenplay={dto({})} screenplays={[SCREENPLAYS[0]]} onClose={vi.fn()} onDeleted={onDeleted} />);
    expect(screen.getByText(/This is the only screenplay in this project/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Delete Screenplay" }));
    await waitFor(() => expect(onDeleted).toHaveBeenCalledWith(null));
  });

  it("compares titles without case or outer spaces", () => {
    expect(titleTaken(SCREENPLAYS, " BLACK RAIN ")).toBe(true);
    expect(titleTaken(SCREENPLAYS, "Black Rain", "s1")).toBe(false);
    expect(titleTaken(SCREENPLAYS, "Monsoon")).toBe(false);
  });
});

function Overview({ screenplayId }: { screenplayId: string | null }) {
  const o = useOverview(null, screenplayId);
  return <div>{o.data ? `shown ${(o.data as { screenplay: { id: string } }).screenplay.id}` : "loading"}</div>;
}

describe("useOverview", () => {
  beforeEach(() => {
    callMock.mockReset();
  });

  it("asks the core for the chosen screenplay", async () => {
    const overview = vi.fn((a: Record<string, unknown>) => ({ screenplay: { id: a.screenplayId ?? "s1" }, screenplays: SCREENPLAYS, drafts: [] }));
    callMock.mockImplementation(answerFrom({ "screenplay.overview": overview }) as typeof call);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <Overview screenplayId="s2" />
      </QueryClientProvider>,
    );
    expect(await screen.findByText("shown s2")).toBeInTheDocument();
    expect(overview).toHaveBeenCalledWith({ screenplayId: "s2" });
  });
});
