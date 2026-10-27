// WindowsHTML capture section: Project shell and Project Home: nav rail (collapsed too), project menu, Create menu, search overlay, status panel and recovery dialogs, toasts, display name, status menu, project settings dialog, back and forward.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

const PROJECT_BTN = '//button[starts-with(@aria-label,"Project: ")]';
const STATUS_CHIP = '//main//button[starts-with(@aria-label,"Status:")]';
const SAVE_BTN = '//footer//button[starts-with(@aria-label,"Save status")]';
const NAME_BTN = '//header//button[starts-with(@aria-label,"Your name:")]';

export default async function (t) {
  const dialog = '//*[@role="dialog"]';

  /** The project's current status label, read from the status chip. */
  const currentStatus = async () => {
    const label = await (await t.el(STATUS_CHIP)).getAttribute("aria-label");
    return label.slice("Status: ".length).split(/[,.]/)[0];
  };
  /** Choose a status that differs from the current one; returns the status that was there. */
  const changeStatus = async () => {
    const before = await currentStatus();
    await t.go(STATUS_CHIP);
    await t.menuItem(before === "Development" ? "Writing" : "Development");
    return before;
  };
  const undo = async () => {
    await t.keys(["Control", "z"]);
    await t.sleep(500);
  };
  const openSearch = () => t.go('//button[@aria-label="Search this project"]');
  const searchFor = async (text) => {
    await t.type('//input[@aria-label="Search"]', text);
    await t.sleep(900);
  };
  const openRecovery = async () => {
    await t.go(SAVE_BTN);
    await t.press("Open recovery");
    await t.waitText("Keep Current Work");
  };

  // Project Home, the shell around it, and the project menu.
  await t.state("shell-project-home", () => t.sleep(800), { home: "Home" });
  await t.state("shell-project-menu", () => t.go(PROJECT_BTN));
  await t.closeOverlays();
  await t.state("shell-create-menu", () => t.go('//button[@aria-label="Create"]'));
  await t.closeOverlays();

  // Navigation rail.
  await t.state("shell-nav-collapsed", () => t.press("Collapse the navigation"));
  try {
    await t.state("shell-nav-collapsed-files", () => t.workspace("Files"));
    await t.workspace("Home");
  } finally {
    await t.press("Expand the navigation");
    await t.sleep(300);
  }

  // Search overlay.
  await t.state("shell-search-empty", () => openSearch());
  await t.state("shell-search-results", () => searchFor("Lakshmi"), { blur: false });
  await t.state("shell-search-results-second-active", () => t.keys("ArrowDown"), { blur: false });
  await t.state("shell-search-results-location", () => searchFor("Rayapuram"), { blur: false });
  await t.state("shell-search-no-matches", () => searchFor("zzqxv"), { blur: false });
  await t.state("shell-search-global-scope-empty", async () => {
    await t.type('//input[@aria-label="Search"]', "");
    await t.press("+ Global Idea Vault", dialog);
  });
  await t.state("shell-search-global-scope", () => searchFor("ferry"), { blur: false });
  await t.closeOverlays();

  // Status menu, toasts, undo and redo.
  await t.state("shell-status-menu", () => t.go(STATUS_CHIP));
  await t.closeOverlays();
  let original = null;
  await t.state(
    "shell-toast-undoable",
    async () => {
      original = await changeStatus();
    },
    { keepToasts: true, wait: 0 },
  );
  await t.state("shell-project-menu-undo-enabled", () => t.go(PROJECT_BTN));
  await t.closeOverlays();
  await t.state(
    "shell-toast-undid",
    async () => {
      await undo();
    },
    { keepToasts: true, wait: 0 },
  );
  await t.state("shell-project-menu-redo-enabled", () => t.go(PROJECT_BTN));
  await t.closeOverlays();
  if (original && (await currentStatus()) !== original) await undo();

  // Status panel and the recovery dialogs.
  await t.state("shell-status-panel", () => t.go(SAVE_BTN));
  await t.closeOverlays();
  const rs = await t.op("project.recovery_state", {});
  if (!rs.hasCheckpoint) await t.state("shell-open-recovery-no-checkpoint", openRecovery);
  await t.closeOverlays();
  await t.state("shell-toast-saved", () => t.keys(["Control", "s"]), { keepToasts: true, wait: 0 });
  await t.sleep(800);
  const beforeEdit = await changeStatus();
  await t.sleep(600);
  await t.state("shell-open-recovery-with-checkpoint", openRecovery);
  await t.closeOverlays();
  if ((await currentStatus()) !== beforeEdit) await undo();
  await t.sleep(600);

  // Display name.
  await t.state("shell-display-name-dialog", () => t.go(NAME_BTN));
  await t.state("shell-display-name-empty-error", () => t.type("#dn-name", ""), { blur: false });
  await t.closeOverlays();

  // Archived banner (restored again).
  try {
    await t.state("shell-status-menu-before-archive", () => t.go(STATUS_CHIP));
    await t.state(
      "shell-toast-archived",
      async () => {
        await t.menuItem("Archived");
        await t.sleep(300);
      },
      { keepToasts: true, wait: 0 },
    );
    await t.state("shell-archived-banner", () => t.sleep(500));
    await t.state("shell-status-menu-archived", () => t.go(STATUS_CHIP));
    await t.closeOverlays();
  } finally {
    const banner = await t.$('//button[normalize-space(.)="Restore to Active"]');
    if (await banner.isExisting()) {
      await t.state("shell-toast-restored", () => t.go(banner), { keepToasts: true, wait: 0 });
    }
  }
  await t.sleep(600);

  // Project settings dialog.
  await t.state("shell-project-settings-dialog", () => t.press("Project settings", "//main"));
  await t.state("shell-project-settings-edited", () => t.type("#project-settings-dialog-logline", "A radio shop, a last bus and one long night."), { blur: false });
  await t.state("shell-project-settings-title-required", () => t.type("#project-settings-dialog-title", ""), { blur: false });
  await t.press("Cancel", dialog);
  await t.sleep(400);
  await t.closeOverlays();

  // Back and forward.
  await t.state("shell-back-enabled", async () => {
    await t.workspace("Files");
    await t.workspace("Call Sheets");
  });
  await t.state("shell-back-and-forward-enabled", () => t.go('//button[@aria-label="Back"]'));
  await t.state("shell-forward-enabled", () => t.go('//button[@aria-label="Back"]'));
  await t.workspace("Home");

  // Project Home of a project with nothing in it. Night Bus is pinned first so its menu shows a Pinned group.
  const recents = await t.op("project.list_recent", { includeArchived: false });
  const bus = recents.find((r) => r.title === "Night Bus");
  if (bus) await t.op("project.set_pinned", { projectId: bus.projectId, pinned: true });
  await t.inEmptyProject(async () => {
    await t.state("shell-empty-project-home", () => t.sleep(800));
    await t.state("shell-empty-project-menu-pinned", () => t.go(PROJECT_BTN));
    await t.closeOverlays();
    await t.state("shell-empty-project-status-panel", () => t.go(SAVE_BTN));
    await t.closeOverlays();
    if (rs.hasCheckpoint) await t.state("shell-open-recovery-no-checkpoint", openRecovery);
    await t.closeOverlays();
    await t.state("shell-empty-project-settings-dialog", () => t.press("Project settings", "//main"));
    await t.press("Cancel", dialog);
    await t.sleep(300);
  });
  if (bus) await t.op("project.set_pinned", { projectId: bus.projectId, pinned: false });

  // Not reachable through the app on demand.
  t.unreachable("shell-save-status-saving", "The amber Saving state lasts a fraction of a second after a change and cannot be held for a screenshot.");
  t.unreachable("shell-recovery-offer", "The offer to open a recovery state only appears after the app was killed in the middle of a session.");
  t.unreachable("shell-save-error-banner", "Needs the disk write to fail (a read-only or removed drive).");
  t.unreachable("shell-save-error-dialog", "Needs the disk write to fail (a read-only or removed drive).");
  t.unreachable("shell-leave-guard", "Needs unsaved changes after a failed save, which needs the disk write to fail.");
  t.unreachable("shell-workspace-error-boundary", "Only appears after a view crashes; the app has no way to force that.");
  t.unreachable("shell-open-project-location", "Opens File Explorer outside the app window.");

  await t.reset();
}
