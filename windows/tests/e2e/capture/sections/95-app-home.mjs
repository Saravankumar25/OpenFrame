// WindowsHTML capture section: Application Home with projects: card menu, rename, delete confirm, archived view, search, pinned, Global Idea Vault, global Settings dialog tabs, display name; ends by reopening Night Bus.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

const NAME_BTN = '//header//button[starts-with(@aria-label,"Your name:")]';
const SEARCH = '//input[@aria-label="Search projects"]';

export default async function (t) {
  const dialog = '//*[@role="dialog"]';
  const card = (title) => `//main//button[starts-with(@aria-label,${t.xq(`Open ${title}`)})]`;
  const cardMenu = (title) => t.press(`More actions for ${title}`);
  const projectsShown = () => t.browser.$('//main[@aria-label="Your projects"]').waitForExist({ timeout: 20_000 });
  const search = async (text) => {
    await t.type(SEARCH, text);
    await t.sleep(500);
  };
  // Pinning does not make the app reload the project list; showing the list again does.
  const refresh = async () => {
    await t.press("Global Idea Vault");
    await t.browser.$('//main[@aria-label="Global Idea Vault"]').waitForExist({ timeout: 10_000 });
    await t.press("Your projects", "//header");
    await projectsShown();
    await t.sleep(500);
  };
  const openSettingsTab = (label) => t.settingsTab(label);

  // Leave the projects as the tour found them, whatever state the states above ended in.
  const restoreProjects = async () => {
    await t.closeOverlays();
    const all = await t.op("project.list_recent", { includeArchived: true });
    for (const p of all) {
      if (p.archived) await t.op("project.set_archived", { projectId: p.projectId, archived: false });
      if (p.pinned) await t.op("project.set_pinned", { projectId: p.projectId, pinned: false });
      if (p.title === "Quiet Short") await t.op("project.rename", { projectId: p.projectId, title: "Empty Short" });
    }
    await t.sleep(1500);
    const back = await t.$('//button[normalize-space(.)="Back to projects"]');
    if (await back.isExisting()) await back.click();
    await t.sleep(500);
    await refresh();
  };

  await t.projectMenu("Close project");
  await projectsShown();
  await t.sleep(800);

  try {
    // Cards and their menus.
    await t.state("apphome-projects", () => t.sleep(500));
    await t.state("apphome-card-menu", () => cardMenu("Night Bus"));
    await t.closeOverlays();
    await t.state("apphome-card-context-menu", () => t.rightClick(card("Night Bus")));
    await t.closeOverlays();

    // Rename (Empty Short is renamed and then renamed back).
    await t.state("apphome-rename-dialog", async () => {
      await cardMenu("Empty Short");
      await t.menuItem("Rename");
      await t.el("#rp-title");
    });
    await t.state("apphome-rename-empty-error", () => t.type("#rp-title", ""), { blur: false });
    await t.state("apphome-rename-filled", () => t.type("#rp-title", "Quiet Short"), { blur: false });
    await t.state(
      "apphome-toast-renamed",
      async () => {
        await t.press("Rename", dialog);
        await t.sleep(300);
      },
      { keepToasts: true, wait: 0 },
    );
    await cardMenu("Quiet Short");
    await t.menuItem("Rename");
    await t.type("#rp-title", "Empty Short");
    await t.press("Rename", dialog);
    await t.sleep(1500);

    // Delete confirmation (never confirmed).
    await t.state("apphome-delete-dialog", async () => {
      await cardMenu("Empty Short");
      await t.menuItem("Delete Project");
      await t.el("#dp-confirm");
    });
    await t.state("apphome-delete-typed", () => t.type("#dp-confirm", "Empty Short"), { blur: false });
    await t.press("Cancel", dialog);
    await t.sleep(300);

    // Pinned section.
    await t.state("apphome-pinned-section", async () => {
      await cardMenu("Empty Short");
      await t.menuItem("Pin");
      await t.sleep(500);
      await refresh();
      await t.browser.$('//section[@aria-label="Pinned"]').waitForExist({ timeout: 10_000 });
      await t.sleep(500);
    });
    await t.state("apphome-card-menu-pinned", () => cardMenu("Empty Short"));
    await t.closeOverlays();
    await t.state("apphome-all-pinned", async () => {
      await cardMenu("Night Bus");
      await t.menuItem("Pin");
      await t.sleep(500);
      await refresh();
      await t.waitText("Every project here is pinned above.", { timeout: 10_000 });
    });
    await cardMenu("Night Bus");
    await t.menuItem("Unpin");
    await t.sleep(500);
    await refresh();
    await cardMenu("Empty Short");
    await t.menuItem("Unpin");
    await t.sleep(500);
    await refresh();

    // Search.
    await t.state("apphome-search-results", () => search("night"), { blur: false });
    await t.state("apphome-search-no-match", () => search("zzqxv"), { blur: false });
    await search("");

    // Archive Empty Short: a toast, the archived view and a search that only matches an archived project.
    await t.state(
      "apphome-toast-archived",
      async () => {
        await cardMenu("Empty Short");
        await t.menuItem("Archive");
        await t.sleep(1500);
      },
      { keepToasts: true, wait: 0 },
    );
    await t.state("apphome-search-archived-match", () => search("empty"), { blur: false });
    await search("");
    await t.state("apphome-archived-view", () => t.press("Open Archived (1)"));
    await t.state("apphome-archived-row-menu", () => t.rightClick('//tr[.//td[normalize-space(.)="Empty Short"]]'));
    await t.state("apphome-delete-dialog-archived", async () => {
      await t.menuItem("Delete Project");
      await t.el("#dp-confirm");
    });
    await t.press("Cancel", dialog);
    await t.sleep(300);
    await t.state("apphome-archived-search-no-match", () => search("zzqxv"), { blur: false });
    await search("");
    await t.press("Back to projects");
    await t.sleep(1500);
    await t.state("apphome-all-archived", async () => {
      await cardMenu("Night Bus");
      await t.menuItem("Archive");
      await t.sleep(1500);
    });
    await t.press("Open Archived (2)");
    await t.state("apphome-archived-view-two", () => t.sleep(300));
    const row = (title) => `//tr[.//td[normalize-space(.)="${title}"]]`;
    await t.press("Restore", row("Night Bus"));
    await t.sleep(1000);
    await refresh();
    await t.browser.$(row("Night Bus")).waitForExist({ timeout: 15_000, reverse: true });
    await t.sleep(500);
    await t.browser.execute(() => document.querySelectorAll('.toast button[aria-label="Dismiss"]').forEach((b) => b.click()));
    await t.state("apphome-toast-restored", () => t.press("Restore", row("Empty Short")), { keepToasts: true, wait: 0 });
    await t.sleep(1000);
    await refresh();
    await t.waitText("No archived projects.", { sel: "main", timeout: 15_000 });
    await t.state("apphome-archived-empty", () => t.sleep(300));
    await t.press("Back to projects");
    await t.sleep(1500);

    // Restore Backup or Package.
    await t.state("apphome-restore-package-dialog", () => t.press("Restore Backup or Package…"));
    await t.closeOverlays();

    // Global Idea Vault.
    await t.state("apphome-quick-capture-dialog", async () => {
      await t.keys(["Control", "Shift", "n"]);
      await t.waitText("Quick Capture");
    });
    await t.closeOverlays();
    await t.state("apphome-global-vault", () => t.sleep(1500));
    await t.state("apphome-global-vault-add-menu", () => t.press("Add", "//main"));
    await t.closeOverlays();
    await t.press("Your projects", "//header");
    await projectsShown();
    await t.sleep(500);

    // Settings dialog.
    await t.state("apphome-settings-appearance", () => t.go('//header//button[@aria-label="Settings"]'));
    await t.state("apphome-settings-shortcuts", () => openSettingsTab("Keyboard shortcuts"));
    await t.state("apphome-settings-shortcuts-filtered", () => t.type('//input[@aria-label="Filter commands"]', "save"), { blur: false });
    await t.type('//input[@aria-label="Filter commands"]', "");
    await t.state("apphome-settings-shortcuts-recording", () => t.go('(//button[starts-with(@aria-label,"Add a shortcut for")])[1]'), { blur: false });
    await t.keys("Escape");
    await t.sleep(300);
    await t.state("apphome-settings-storage", () => openSettingsTab("Storage"), { wait: 1500 });
    await t.state("apphome-settings-about", () => openSettingsTab("About and support"));
    await t.state("apphome-settings-about-support", async () => {
      await t.browser.execute(() => {
        const body = document.querySelector('[role="dialog"] .db');
        if (body) body.scrollTop = body.scrollHeight;
      });
      await t.sleep(300);
    });
    await t.press("Done", dialog);
    await t.sleep(300);

    // Display name.
    await t.state("apphome-display-name-dialog", () => t.go(NAME_BTN));
    await t.state("apphome-display-name-empty-error", () => t.type("#dn-name", ""), { blur: false });
    await t.closeOverlays();

    // New Project.
    await t.state("apphome-new-project-dialog", async () => {
      await t.keys(["Control", "n"]);
      await t.el("#np-title");
    });
    await t.state("apphome-new-project-title-required", async () => {
      await (await t.el("#np-title")).click();
      await t.keys("Tab");
      await t.waitText("A title is required.");
    });
    await t.state(
      "apphome-new-project-filled",
      async () => {
        await t.type("#np-title", "Harbor Light");
        await t.press("Short Film", dialog);
      },
      { blur: false },
    );
    await t.state("apphome-new-project-optional-details", () => t.go('//*[@role="dialog"]//summary'));
    await t.press("Cancel", dialog);
    await t.sleep(300);
  } finally {
    await restoreProjects();
    // Back into Night Bus so the tour continues.
    await t.go(card("Night Bus"));
    await t.browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
    await t.sleep(800);
  }

  t.unreachable("apphome-open-project-folder-picker", "Open Project opens the native Windows folder picker, which is outside the app window.");
  t.unreachable("apphome-card-unavailable", "Needs a project whose folder was moved or deleted outside the app.");
  t.unreachable("apphome-unavailable-dialog", "Needs a project whose folder was moved or deleted outside the app.");
  t.unreachable("apphome-load-error", "The project list loads from the local database, which cannot be made to fail on demand.");
  t.unreachable("apphome-new-project-cloud-warning", "Needs the projects folder to sit inside OneDrive, Dropbox or Google Drive.");
  t.unreachable("apphome-new-project-failed", "Needs a parent folder that refuses writes.");
  t.unreachable("apphome-new-project-location-picker", "Change... opens the native Windows folder picker.");

  await t.reset();
}
