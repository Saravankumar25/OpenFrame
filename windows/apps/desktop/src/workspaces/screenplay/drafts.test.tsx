import { beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { call, onAppEvent } from "../../ipc/client";
import { installInvalidation } from "../../ipc/query";
import type { AppEvent } from "../../ipc/generated/AppEvent";
import type { ScreenplayDraftDto } from "../../ipc/generated/ScreenplayDraftDto";
import type { ScreenplayPermissions } from "../../ipc/generated/ScreenplayPermissions";
import { answerFrom } from "../../app/home/testUtils";
import { useOverview } from "./api";
import { DraftsDrawer, type DraftActions } from "./drafts";

vi.mock("../../ipc/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../ipc/client")>()),
  call: vi.fn(),
  onAppEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

const callMock = vi.mocked(call);
const onAppEventMock = vi.mocked(onAppEvent);
// Interaction tests (menus, dialogs, typing) can be slow on a busy machine.
vi.setConfig({ testTimeout: 20_000 });

const PERMISSIONS: ScreenplayPermissions = { edit: true, comment: true, resolveComments: true, lock: true, unlock: true, delete: true };

const draft = (p: Partial<ScreenplayDraftDto>): ScreenplayDraftDto => ({
  id: "d1",
  screenplayId: "s1",
  name: "Draft 1",
  note: null,
  status: "Draft",
  isCurrent: false,
  createdFromDraftId: null,
  createdFromName: null,
  lineage: [],
  lockedAt: null,
  lockedByName: null,
  revisionLabel: null,
  revisionColor: null,
  revisionReason: null,
  sceneCount: 1,
  openComments: 0,
  createdAt: Date.now() - 60_000,
  lastModified: Date.now() - 60_000,
  rev: 1,
  ...p,
});

const actions: DraftActions = {
  open: vi.fn(),
  makeCurrent: vi.fn(),
  restore: vi.fn(),
  lock: vi.fn(),
  review: vi.fn(),
  compare: vi.fn(),
  newDraft: vi.fn(),
};

/** The drawer fed by the real overview query, as the Screenplay workspace does. */
function Workspace() {
  const overview = useOverview(null);
  if (!overview.data) return null;
  return <DraftsDrawer drafts={overview.data.drafts} openDraftId="d1" permissions={PERMISSIONS} actions={actions} onClose={() => undefined} />;
}

function renderWorkspace() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity }, mutations: { retry: false } } });
  installInvalidation(client);
  render(
    <QueryClientProvider client={client}>
      <Workspace />
    </QueryClientProvider>,
  );
}

/** Deliver an event the way the Rust core does after a committed change. */
function emit(ev: AppEvent) {
  for (const [handler] of onAppEventMock.mock.calls) handler(ev);
}

/** Draft names in the drawer. `hidden` also reads it while a dialog covers it. */
const rowNames = (hidden = false) =>
  within(screen.getByRole("complementary", { name: "Drafts", hidden }))
    .getAllByRole("button", { name: /^Actions for /, hidden })
    .map((b) => b.getAttribute("aria-label")?.slice("Actions for ".length));

describe("Drafts drawer", () => {
  beforeEach(() => {
    callMock.mockReset();
  });

  it("keeps the drawer open under Rename and lists the new name once Rust reports the change", async () => {
    const user = userEvent.setup();
    let drafts = [draft({ id: "d1", name: "Draft 1", isCurrent: true }), draft({ id: "d2", name: "Rewrite A", createdAt: Date.now() })];
    const rename = vi.fn((a: Record<string, unknown>) => {
      drafts = drafts.map((d) => (d.id === a.draftId ? { ...d, name: String(a.name) } : d));
      return drafts.find((d) => d.id === a.draftId);
    });
    callMock.mockImplementation(
      answerFrom({
        "screenplay.overview": () => ({ drafts }),
        "screenplay.history_points": [],
        "screenplay.rename_draft": rename,
      }) as typeof call,
    );
    renderWorkspace();
    await waitFor(() => expect(rowNames()).toEqual(["Rewrite A", "Draft 1"]));

    await user.click(screen.getByRole("button", { name: "Actions for Rewrite A" }));
    await user.click(await screen.findByRole("menuitem", { name: /Rename/ }));
    const field = await screen.findByRole("textbox", { name: /Draft name/ });
    await user.clear(field);
    await user.type(field, "Rewrite B");
    await user.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(rename).toHaveBeenCalledWith({ draftId: "d2", name: "Rewrite B", note: "" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());

    // The drawer is still there after the dialog closes. What the core emits
    // after `screenplay.rename_draft` refreshes it with the new name.
    act(() => emit({ type: "dataChanged", store: "project", tables: ["screenplay_draft"], ids: ["d2"], origin: "local" }));
    await waitFor(() => expect(rowNames()).toEqual(["Rewrite B", "Draft 1"]));
  });

  it("keeps the dialog open and shows why when the name is taken", async () => {
    const user = userEvent.setup();
    const drafts = [draft({ id: "d1", name: "Draft 1", isCurrent: true }), draft({ id: "d2", name: "Rewrite A", createdAt: Date.now() })];
    callMock.mockImplementation(
      answerFrom({
        "screenplay.overview": { drafts },
        "screenplay.history_points": [],
        "screenplay.rename_draft": () => {
          throw Object.assign(new Error("Another draft is already called â€œDraft 1â€."), { code: "validation.duplicate", message: "Another draft is already called â€œDraft 1â€." });
        },
      }) as typeof call,
    );
    renderWorkspace();
    await waitFor(() => expect(rowNames()).toHaveLength(2));
    await user.click(screen.getByRole("button", { name: "Actions for Rewrite A" }));
    await user.click(await screen.findByRole("menuitem", { name: /Rename/ }));
    const field = await screen.findByRole("textbox", { name: /Draft name/ });
    await user.clear(field);
    await user.type(field, "Draft 1");
    await user.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Another draft is already called â€œDraft 1â€.")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(rowNames(true)).toEqual(["Rewrite A", "Draft 1"]);
  });
});
