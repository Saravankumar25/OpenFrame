// WindowsHTML capture section: Files, Notes and Tasks, Activity, Recently Deleted, Templates, Project settings and every settings tab (Writing, Keyboard shortcuts, Offline AI, Advanced, Appearance incl. theme import), Export Diagnostics.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { idOf } from "../seed.mjs";

const DIALOG = '//*[@role="dialog"]';
const DRAWER = '//aside[contains(@class,"drawer")]';
const DRAWER_CLOSE = `${DRAWER}//button[@aria-label="Close"]`;
const FOLDERS = '//nav[@aria-label="Folders"]';
const rg = (label) => `//*[@role="radiogroup"][@aria-label="${label}"]`;
const log = (m) => console.log(`  pages: ${m}`);

const NAVY = JSON.stringify({
  formatVersion: 1,
  id: "tour-navy",
  name: "Tour Navy",
  description: "Deep blue surfaces with a bright blue accent.",
  author: "Capture tour",
  seeds: { neutralHue: 255, neutralTint: 0.03, accentHue: 238, accentChroma: 0.15, accentLightnessNudge: 0 },
});
const FOG = JSON.stringify({
  formatVersion: 1,
  id: "tour-fog",
  name: "Tour Fog",
  description: "Grey on grey.",
  author: "Capture tour",
  seeds: { neutralHue: 0, neutralTint: 0, accentHue: 250, accentChroma: 0.1, accentLightnessNudge: 0 },
  overrides: { light: { muted: "#bbbbbb" } },
});
const HARBOUR = JSON.stringify({
  formatVersion: 1,
  id: "tour-harbour",
  name: "Tour Harbour",
  description: "Blue-grey surfaces with a lamp-orange accent.",
  author: "Capture tour",
  seeds: { neutralHue: 230, neutralTint: 0.02, accentHue: 55, accentChroma: 0.14, accentLightnessNudge: 0 },
});

const DICTIONARY_WORDS = ["Lakshmi", "Venkat", "Farooq", "Padma", "Kurta", "Ananya", "Suresh", "Imran", "Divya", "Karthik", "Neha", "Ravi", "Jetty"];

export default async function (t) {
  const dir = join(process.env.E2E_DATA_ROOT ?? ".", "capture-files-pages");
  mkdirSync(dir, { recursive: true });
  const ownFile = join(dir, "Tour permit letter.txt");
  const linkedFile = join(dir, "Tour linked schedule.txt");
  writeFileSync(ownFile, "Letter from the panchayat office allowing the tower climb.\n");
  writeFileSync(linkedFile, "Linked copy of the draft schedule.\n");

  const closeDrawer = async () => {
    const b = await t.$(DRAWER_CLOSE);
    if (!(await b.isExisting())) return;
    await b.click();
    await b.waitForExist({ reverse: true, timeout: 5000 }).catch(() => undefined);
    await t.sleep(250);
  };
  const tidy = async () => {
    await closeDrawer().catch(() => undefined);
    await t.closeOverlays().catch(() => undefined);
  };
  const escape = () => t.keys("Escape");
  const scrollEnd = (top = 1e6) =>
    t.browser.execute((y) => {
      for (const el of document.querySelectorAll(".content")) if (el.scrollHeight > el.clientHeight) el.scrollTop = y;
    }, top);
  const setArea = async (sel, value) => {
    const el = await t.el(sel);
    await t.browser.execute(
      (n, v) => {
        Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value").set.call(n, v);
        n.dispatchEvent(new Event("input", { bubbles: true }));
      },
      el,
      value,
    );
    await t.sleep(200);
  };
  const clearToasts = async () => {
    await t.browser.execute(() => document.querySelectorAll('.toast button[aria-label="Dismiss"]').forEach((b) => b.click()));
    await t.browser.waitUntil(async () => !(await t.browser.execute(() => !!document.querySelector(".toast"))), { timeout: 5000 }).catch(() => undefined);
  };
  const state = (name, fn, opts) =>
    t.state(
      name,
      async () => {
        if (opts?.keepToasts) await clearToasts();
        await fn();
      },
      opts?.keepToasts ? { wait: 1200, ...opts } : opts,
    );
  const exists = async (xpath) => (await t.$(xpath)).isExisting();
  const valueOf = async (sel) => (await t.$(sel)).getValue();
  const quiet = async (label, fn) => {
    try {
      await fn();
    } catch (e) {
      log(`${label}: ${String(e?.message ?? e).split("\n")[0].slice(0, 200)}`);
    }
  };
  const purgeMine = async () => {
    for (const store of ["project", "global"]) {
      const items = (await t.op("project.deleted_items", { store })) ?? [];
      for (const i of items) if (/^Tour /.test(i.title ?? "")) await t.op("trash.purge", { store, id: i.id });
    }
  };

  await t.reset();

  // ------------------------------------------------------------------ Files
  await state("files-list", async () => {
    await t.workspace("Files");
    await t.el('//table[@aria-label="Files"]');
    await t.waitText("Crew contacts", { sel: "main" });
  }, { home: "Files" });

  await state("files-grid", async () => {
    await t.press("Grid", rg("View"));
    await t.el('//*[contains(@class,"filecard")]');
  });

  await state("files-folder-contracts", async () => {
    await t.press("List", rg("View"));
    await t.go(`${FOLDERS}//button[starts-with(normalize-space(.),"Contracts")]`);
    await t.waitText("Location release", { sel: "main" });
  });

  await state("files-folder-top-level", async () => {
    await t.go(`${FOLDERS}//button[starts-with(normalize-space(.),"Top level")]`);
    await t.waitText("Crew contacts", { sel: "main" });
  });

  await state("files-search-results", async () => {
    await t.go(`${FOLDERS}//button[starts-with(normalize-space(.),"All files")]`);
    await t.type('input[aria-label="Search files"]', "crew");
    await t.waitText("Crew contacts", { sel: "main" });
  });

  await state("files-search-no-match", async () => {
    await t.type('input[aria-label="Search files"]', "zzzz");
    await t.waitText("No files match", { sel: "main" });
  });

  await state("files-add-menu", async () => {
    await t.type('input[aria-label="Search files"]', "");
    await t.press("Add File");
    await t.el('//*[@role="menu"]');
  });

  await state("files-row-menu", async () => {
    await escape();
    await t.go('//button[starts-with(@aria-label,"More actions for Crew")]');
    await t.el('//*[@role="menu"]');
  });

  await state("files-row-context-menu", async () => {
    await escape();
    await t.rightClick('//tr[starts-with(@aria-label,"Tower survey")]');
    await t.el('//*[@role="menu"]');
  });

  await state("files-details-drawer", async () => {
    await escape();
    await t.go('//tr[starts-with(@aria-label,"Crew contacts")]');
    await t.el(`${DRAWER}//input[@id="fd-name"]`);
  });

  await state("files-details-notes", async () => {
    await closeDrawer();
    await t.op("files.add", { paths: [ownFile], folderId: null, mode: "copy" });
    await t.go('//tr[starts-with(@aria-label,"Tour permit letter")]');
    await t.type("#fd-notes", "Signed copy needed before the tower climb. Ask the panchayat clerk for two.");
    await t.sleep(900);
  });

  await state("files-details-renamed", async () => {
    await t.type("#fd-name", "Tour permit letter v2");
    await t.keys("Enter");
    await t.waitText("Renamed file to", { timeout: 8000 });
  }, { keepToasts: true });

  await state("files-file-deleted-toast", async () => {
    await t.press("Delete", DRAWER);
    await t.waitText("to Recently Deleted", { timeout: 8000 });
  }, { keepToasts: true });

  await state("files-linked-file", async () => {
    await tidy();
    await t.op("files.add", { paths: [linkedFile], folderId: null, mode: "link" });
    await t.waitText("Linked (external)", { sel: "main" });
  });

  await state("files-unavailable-linked-file", async () => {
    rmSync(linkedFile, { force: true });
    await t.browser.refresh();
    await t.$('//main[@aria-label="Files"]').waitForExist({ timeout: 30_000 });
    await t.waitText("Unavailable", { sel: "main" });
  });

  await state("files-unavailable-details", async () => {
    await t.go('//tr[starts-with(@aria-label,"Tour linked schedule")]');
    await t.waitText("can't be found", { sel: "aside.drawer" });
  });

  await state("files-linked-file-deleted-toast", async () => {
    await t.press("Delete", DRAWER);
    await t.waitText("to Recently Deleted", { timeout: 8000 });
  }, { keepToasts: true });

  await state("files-new-folder-dialog", async () => {
    await tidy();
    await t.press("New Folder");
    await t.el("#fo-name");
  });

  await state("files-new-folder-filled", async () => {
    await t.type("#fo-name", "Tour Folder");
  }, { blur: false });

  await state("files-folder-created", async () => {
    await t.press("Create", DIALOG);
    await t.waitText("This folder is empty", { sel: "main" });
  }, { keepToasts: true });

  await state("files-rename-folder-dialog", async () => {
    await t.press("Rename", FOLDERS);
    await t.el("#fo-name");
  });

  await state("files-delete-folder-confirm", async () => {
    await t.press("Cancel", DIALOG);
    await t.press("Delete", FOLDERS);
    await t.el(t.dialog("Delete folder “Tour Folder”?"));
  });

  await state("files-folder-deleted-toast", async () => {
    await t.press("Cancel", DIALOG);
    await t.press("Delete", FOLDERS);
    await t.press("Delete Folder", DIALOG);
    await t.waitText("Deleted folder", { timeout: 8000 });
  }, { keepToasts: true });

  await t.inEmptyProject(async () => {
    await state("files-empty", async () => {
      await t.workspace("Files");
      await t.waitText("Keep project documents and attachments here.", { sel: "main" });
    });
    await state("notes-empty", async () => {
      await t.projectMenu("Notes & Tasks");
      await t.waitText("No notes yet.", { sel: "main" });
      await t.waitText("No tasks yet.", { sel: "main" });
    });
    await state("activity-new-project", async () => {
      await t.projectMenu("Activity");
      await t.el('//main[@aria-label="Activity"]');
    });
    await state("trash-empty", async () => {
      await t.projectMenu("Recently Deleted");
      await t.waitText("Nothing has been deleted.", { sel: "main" });
    });
  });
  t.unreachable("files-add-copy-dialog", "Add File opens the Windows file picker (native dialog), which the WebDriver session cannot see or drive.");
  t.unreachable("files-link-dialog", "Link to file opens the Windows file picker (native dialog).");
  t.unreachable("files-export-copy-dialog", "Export Copy opens the Windows Save dialog (native dialog).");
  t.unreachable("files-drop-overlay", "The drop overlay needs a real drag of files from Explorer into the window (OS drag and drop).");
  t.unreachable("files-reveal-in-file-manager", "Reveal in File Manager and Open start another Windows application outside the window.");
  t.unreachable("activity-empty-all", "The unfiltered \"No major activity yet.\" never shows: creating any project already logs an entry. The empty look is captured by activity-filter-exchange.");
  t.unreachable("activity-show-older", "Show older activity appears only after 200 or more entries, more than the demo project holds.");

  // ---------------------------------------------------------- Notes & Tasks
  const notesRow = (title) => `//section[@aria-label="Project Notes"]//li//button[starts-with(normalize-space(.),"${title}")]`;
  const taskLink = (title) => `//section[@aria-label="Tasks"]//li//button[contains(normalize-space(.),"${title}")]`;

  await state("notes-filled", async () => {
    await t.projectMenu("Notes & Tasks");
    await t.el('//section[@aria-label="Tasks"]');
    await t.waitText("Permits", { sel: "main" });
    await t.waitText("Scout the east jetty", { sel: "main" });
  });

  await state("notes-search-filtered", async () => {
    await t.type('input[aria-label="Search notes"]', "permit");
    await t.waitText("Music", { sel: "main", present: false });
  });

  await state("notes-search-no-match", async () => {
    await t.type('input[aria-label="Search notes"]', "zzzz");
    await t.waitText("No notes match", { sel: "main" });
  });

  await state("notes-note-editor", async () => {
    await t.type('input[aria-label="Search notes"]', "");
    await t.go(notesRow("Permits"));
    await t.el("#ne-title");
  });

  await state("notes-note-editor-pinned", async () => {
    await closeDrawer();
    await t.go(notesRow("Monsoon schedule risk"));
    await t.el(`${DRAWER}//button[normalize-space(.)="Unpin"]`);
  });

  await state("notes-new-note-editor", async () => {
    await closeDrawer();
    await t.press("New Note");
    await t.el("#ne-title");
  });

  await state("notes-new-note-filled", async () => {
    await t.type("#ne-title", "Tour note on the radio tower");
    await t.type("#ne-body", "Sound check at the harbour first, tower climb after the safety briefing.");
    await t.sleep(900);
  });

  await state("notes-note-deleted-toast", async () => {
    await t.press("Delete", DRAWER);
    await t.waitText("Moved note", { timeout: 8000 });
  }, { keepToasts: true });

  await state("notes-task-editor-new", async () => {
    await t.press("New Task");
    await t.el("#te-title");
  }, { blur: false });

  await state("notes-task-editor-title-error", async () => {
    await t.press("Add Task", DRAWER);
    await t.waitText("Give the task a short title.");
  });

  await state("notes-task-editor-filled", async () => {
    await t.type("#te-title", "Tour task call the depot");
    await t.setNative("#te-due", "2026-11-20");
    await t.type("#te-notes", "Ask about the spare bus for the night shoot.");
  });

  await state("notes-task-editor-related-results", async () => {
    await t.type('input[aria-label="Find a scene, location, character, shooting day or call sheet"]', "bus");
    await t.el('//*[@role="listbox"][@aria-label="Matching items"]');
    await t.browser.waitUntil(async () => !(await t.text('[aria-label="Matching items"]')).includes("Searching"), { timeout: 10_000 });
  });

  await state("notes-task-editor-related-chosen", async () => {
    await t.go('//*[@role="listbox"][@aria-label="Matching items"]//*[@role="option"][1]');
    await t.waitText("Remove", { sel: "aside.drawer" });
  });

  await state("notes-task-added-toast", async () => {
    await t.press("Add Task", DRAWER);
    await t.waitText("Added task", { timeout: 8000 });
  }, { keepToasts: true });

  await state("notes-task-editor-existing", async () => {
    await t.go(taskLink("Scout the east jetty"));
    await t.el(`${DRAWER}//button[normalize-space(.)="Mark Done"]`);
  });

  await state("notes-tasks-done", async () => {
    await closeDrawer();
    await t.press("Done", rg("Show tasks"));
    await t.waitText("Lock the shooting script", { sel: "main" });
  });

  await state("notes-tasks-all", async () => {
    await t.press("All", rg("Show tasks"));
    await t.waitText("Tour task call the depot", { sel: "main" });
  });

  await state("notes-task-completed-toast", async () => {
    await t.press("Open", rg("Show tasks"));
    await t.go('//section[@aria-label="Tasks"]//input[starts-with(@aria-label,"Mark “Tour task")]');
    await t.waitText("Completed task", { timeout: 8000 });
  }, { keepToasts: true });

  await state("notes-task-deleted-toast", async () => {
    await t.press("All", rg("Show tasks"));
    await t.go(taskLink("Tour task call the depot"));
    await t.press("Delete", DRAWER);
    await t.waitText("Moved task", { timeout: 8000 });
  }, { keepToasts: true });
  await t.press("Open", rg("Show tasks")).catch(() => undefined);

  // --------------------------------------------------------------- Activity
  const activityFilter = (label) => async () => {
    await t.press(label, rg("Show activity"));
    await t.sleep(300);
  };
  await state("activity-all", async () => {
    await t.projectMenu("Activity");
    await t.el('//ol[@aria-label="Activity"]');
  });
  await state("activity-filter-drafts", activityFilter("Drafts"));
  await state("activity-filter-story", activityFilter("Story"));
  await state("activity-filter-production", activityFilter("Production"));
  await state("activity-filter-exchange", activityFilter("Exchange"));
  await t.press("All", rg("Show activity")).catch(() => undefined);

  // -------------------------------------------------------- Recently Deleted
  await state("trash-filled", async () => {
    await t.projectMenu("Recently Deleted");
    await t.el('//table[@aria-label="Deleted items"]');
    await t.waitText("Old title ideas", { sel: "main" });
  });

  await state("trash-search-filtered", async () => {
    await t.type('input[aria-label="Search deleted items"]', "Tour task");
    await t.waitText("Old title ideas", { sel: "main", present: false });
  });

  await state("trash-search-no-match", async () => {
    await t.type('input[aria-label="Search deleted items"]', "zzzz");
    await t.waitText("No deleted items match", { sel: "main" });
  });

  await state("trash-purge-confirm", async () => {
    await t.type('input[aria-label="Search deleted items"]', "");
    await t.press("Delete Old title ideas permanently");
    await t.el(t.dialog("Delete “Old title ideas” permanently?"));
  });

  let globalEmpty = false;
  await state("trash-global-empty", async () => {
    await t.press("Keep in Recently Deleted", DIALOG);
    await t.press("Global Idea Vault", rg("Which deleted items"));
    await t.sleep(600);
    globalEmpty = (await t.text("main")).includes("Nothing has been deleted.");
  });
  if (!globalEmpty) log("the Global Idea Vault already has deleted items, so the global empty state is not named empty");

  let globalId = null;
  await quiet("global item", async () => {
    const made = await t.op("vault.create", { store: "global", itemType: "note", title: "Tour global idea", body: "A ferry story for later.", caption: null, url: null, sourceText: null, folderId: null, collectionId: null, tags: [] });
    globalId = idOf(made);
    if (globalId) await t.op("vault.delete", { store: "global", ids: [globalId] });
  });

  await state("trash-global-filled", async () => {
    await t.waitText("Tour global idea", { sel: "main" });
  });

  await state("trash-global-purge-confirm", async () => {
    await t.press("Delete Tour global idea permanently");
    await t.el(t.dialog("Delete “Tour global idea” permanently?"));
  });

  await state("trash-purged-toast", async () => {
    await t.press("Delete Permanently", DIALOG);
    await t.waitText("was deleted permanently", { timeout: 8000 });
  }, { keepToasts: true });

  await t.press("This project", rg("Which deleted items")).catch(() => undefined);

  // -------------------------------------------------------------- Templates
  const actionsFor = (name) => `//button[@aria-label=${t.xq(`Actions for ${name}`)}]`;
  const fillTemplate = async (name, description, lines) => {
    await t.type("#tp-name", name);
    await t.type("#tp-desc", description);
    await setArea("#tp-lines", lines);
  };

  await state("templates-page", async () => {
    await t.projectMenu("Templates");
    await t.waitText("Create Template", { sel: "main" });
    await t.el('(//button[starts-with(@aria-label,"Actions for ")])[1]');
  });

  await state("templates-actions-menu-builtin", async () => {
    await t.go('(//button[starts-with(@aria-label,"Actions for ")])[1]');
    await t.el('//*[@role="menu"]');
  });

  await state("templates-preview-dialog", async () => {
    await t.menuItem("Preview");
    await t.el(`${DIALOG}//div[@class="db"]`);
  });

  await state("templates-create-dialog", async () => {
    await t.closeOverlays();
    await t.press("Create Template");
    await t.el("#tp-name");
  });

  await state("templates-create-name-error", async () => {
    await t.press("Create", DIALOG);
    await t.waitText("Give the template a name.");
  });

  await state("templates-create-filled", async () => {
    await fillTemplate("Tour global call sheet", "One page for night shoots", "Weather\nCrew\nExtras");
    await t.press("Every project", rg("Keep template in"));
  });

  await state("templates-created-toast", async () => {
    await t.press("Create", DIALOG);
    await t.waitText("in your templates", { timeout: 8000 });
    await scrollEnd();
    await t.sleep(300);
  }, { keepToasts: true });

  await quiet("project template", async () => {
    await scrollEnd(0);
    await t.press("Create Template");
    await t.el("#tp-name");
    await fillTemplate("Tour project call sheet", "Day shoot with a short crew list", "Weather\nCrew");
    await t.press("Create", DIALOG);
    await t.waitText("in this project", { timeout: 8000 });
    await scrollEnd();
  });

  await state("templates-actions-menu-own", async () => {
    await t.go(actionsFor("Tour project call sheet"));
    await t.el('//*[@role="menu"]');
  });

  await state("templates-edit-dialog", async () => {
    await t.menuItem("Edit");
    await t.waitText("Edit Template");
  });

  await state("templates-apply-dialog", async () => {
    await t.press("Cancel", DIALOG);
    await t.go(actionsFor("Tour project call sheet"));
    await t.menuItem("Create a call sheet");
    await t.el(t.dialog("Create a call sheet"));
  });

  await state("templates-delete-confirm", async () => {
    await t.press("Cancel", DIALOG);
    await t.go(actionsFor("Tour global call sheet"));
    await t.menuItem("Delete");
    await t.el(t.dialog("Delete template “Tour global call sheet”?"));
  });

  await state("templates-global-deleted-toast", async () => {
    await t.press("Cancel", DIALOG);
    await t.go(actionsFor("Tour global call sheet"));
    await t.menuItem("Delete");
    await t.press("Delete Template", DIALOG);
    await t.waitText("Deleted template", { timeout: 8000 });
  }, { keepToasts: true });

  await state("templates-project-deleted-toast", async () => {
    await t.go(actionsFor("Tour project call sheet"));
    await t.menuItem("Delete");
    await t.waitText("Moved template", { timeout: 8000 });
  }, { keepToasts: true });

  // ------------------------------------------------------- Project settings
  const ID = "#project-settings-page";
  await state("settings-general", async () => {
    await t.projectMenu("Project settings");
    await t.el(`${ID}-title`);
    await t.sleep(400);
  });

  const original = {};
  for (const f of ["title", "genre", "logline"]) original[f] = await valueOf(`${ID}-${f}`).catch(() => "");
  await state("settings-general-dirty", async () => {
    await t.type(`${ID}-genre`, "Rural drama");
    await t.type(`${ID}-logline`, "A night bus, a broken tower and a village that stops listening.");
    await t.el('//button[normalize-space(.)="Save"][not(@disabled)]');
  });

  await state("settings-general-title-error", async () => {
    await t.type(`${ID}-title`, "");
    await t.waitText("A title is required.");
  });

  await state("settings-general-lower", async () => {
    await t.type(`${ID}-title`, original.title);
    await t.type(`${ID}-genre`, original.genre);
    await t.type(`${ID}-logline`, original.logline);
    await scrollEnd();
    await t.sleep(300);
  });

  await state("settings-export-diagnostics-dialog", async () => {
    await t.press("Export Diagnostics…");
    await t.el('//ul[@aria-label="Files in the bundle"]');
  });

  await state("settings-export-diagnostics-contents", async () => {
    await t.go('(//button[starts-with(@aria-label,"Show contents of ")])[1]');
    await t.el("//pre");
  });
  t.unreachable("settings-export-diagnostics-complete", "Export saves through the Windows Save dialog (native dialog) before the Export complete screen appears.");

  // ---------------------------------------------------------------- Writing
  await state("settings-writing", async () => {
    await t.closeOverlays();
    await t.settingsTab("Writing");
    await t.el('//section[@aria-label="Personal dictionary"]');
    await t.sleep(500);
  });

  const dictionaryBefore = ((await t.op("writing.dictionary", {}).catch(() => null))?.words ?? []).map((w) => w.toLowerCase());
  await state("settings-writing-word-added", async () => {
    await t.type('input[aria-label="New word"]', "Rayapuram");
    await t.press("Add", '//section[@aria-label="Personal dictionary"]');
    await t.waitText("Added “Rayapuram”", { timeout: 8000 });
  }, { keepToasts: true });

  await state("settings-writing-dictionary-filled", async () => {
    await t.type('input[aria-label="New word"]', "");
    for (const w of DICTIONARY_WORDS) await t.op("writing.dictionary_add", { word: w });
    await t.waitText(`${DICTIONARY_WORDS.length + 1} words`, { timeout: 8000 });
    await t.el('//input[@aria-label="Filter words"]');
  });

  await state("settings-writing-dictionary-filtered", async () => {
    await t.type('input[aria-label="Filter words"]', "ra");
    await t.sleep(300);
  });

  await state("settings-writing-word-removed-toast", async () => {
    await t.type('input[aria-label="Filter words"]', "");
    await t.go('//button[@aria-label="Remove Rayapuram"]');
    await t.waitText("Removed “Rayapuram”", { timeout: 8000 });
  }, { keepToasts: true });

  const spellingBox = '//label[contains(.,"Check spelling as I write")]//input[@type="checkbox"]';
  const spellingWasOn = await (await t.$(spellingBox)).isSelected().catch(() => null);
  await state("settings-writing-spelling-toggled", async () => {
    await t.go(spellingBox);
    await t.waitText("Turned spell checking", { timeout: 8000 });
  }, { keepToasts: true });
  await state("settings-writing-word-error", async () => {
    await t.type('input[aria-label="New word"]', "12345");
    await t.press("Add", '//section[@aria-label="Personal dictionary"]');
    await t.waitText("A word needs at least one letter.");
  });
  if (spellingWasOn !== null && (await (await t.$(spellingBox)).isSelected()) !== spellingWasOn) await quiet("spelling restore", () => t.go(spellingBox));
  await quiet("dictionary cleanup", async () => {
    for (const w of [...DICTIONARY_WORDS, "Rayapuram"]) if (!dictionaryBefore.includes(w.toLowerCase())) await t.op("writing.dictionary_remove", { word: w });
  });

  // ------------------------------------------------------ Keyboard shortcuts
  const addFor = (label) => `Add a shortcut for ${label}`;
  await state("settings-shortcuts", async () => {
    await t.settingsTab("Keyboard shortcuts");
    await t.waitText("Restore all defaults", { sel: "main" });
  });

  await state("settings-shortcuts-filtered", async () => {
    await t.type('input[aria-label="Filter commands"]', "undo");
    await t.sleep(300);
  });

  await state("settings-shortcuts-no-match", async () => {
    await t.type('input[aria-label="Filter commands"]', "zzzz");
    await t.waitText("No command matches");
  });

  await state("settings-shortcut-recording", async () => {
    await t.type('input[aria-label="Filter commands"]', "");
    await t.press(addFor("Open or close the assistant"));
    await t.waitText("Press the new keys");
  }, { blur: false });

  await state("settings-shortcut-invalid-plain-key", async () => {
    await t.keys("a");
    await t.waitText("Add Ctrl or Alt to that key");
  }, { blur: false });

  await state("settings-shortcut-invalid-reserved", async () => {
    await t.keys(["Control", "c"]);
    await t.waitText("kept for copy, cut, paste and select all");
  }, { blur: false });

  await state("settings-shortcut-duplicate-error", async () => {
    await t.keys("Escape");
    await t.press(addFor("Search"));
    await t.keys(["Control", "k"]);
    await t.waitText("is already a shortcut for Search");
  });

  await state("settings-shortcut-changed", async () => {
    await t.press(addFor("Open or close the assistant"));
    await t.keys("F9");
    await t.waitText("(changed)");
  }, { keepToasts: true });

  await state("settings-shortcut-conflict-dialog", async () => {
    await t.press(addFor("Save now"));
    await t.keys(["Control", "k"]);
    await t.el(t.dialog("Ctrl+K is already used"));
  });

  await state("settings-shortcut-moved", async () => {
    await t.press("Move it here", DIALOG);
    await t.waitText("Shortcuts updated", { timeout: 8000 });
  }, { keepToasts: true });

  await state("settings-shortcut-restore-conflict-dialog", async () => {
    await t.press("Restore the default for Search");
    await t.el(t.dialog("Restore the default?"));
  });

  await state("settings-shortcut-restore-all-confirm", async () => {
    await t.press("Cancel", DIALOG);
    await t.press("Restore all defaults");
    await t.el(t.dialog("Restore all keyboard shortcuts?"));
  });

  await state("settings-shortcuts-restored-toast", async () => {
    await t.press("Restore all defaults", DIALOG);
    await t.waitText("back to their defaults", { timeout: 8000 });
  }, { keepToasts: true });

  await quiet("engineered shortcut conflict", async () => {
    await t.op("app.set_shortcuts", { changes: [{ commandId: "app.save", keys: ["Mod+K"] }] });
    await t.waitText("used by two commands", { timeout: 8000 });
    await state("settings-shortcuts-conflict-banner", async () => {
      await t.sleep(200);
    });
  });
  await quiet("shortcuts restore", async () => {
    await t.op("app.reset_shortcuts", {});
    await t.waitText("used by two commands", { present: false, timeout: 8000 });
  });

  // ------------------------------------------------------------- Offline AI
  await state("settings-offline-ai-ready", async () => {
    await t.settingsTab("Offline AI");
    await t.el('//section[@aria-label="AI health"]');
    await t.sleep(800);
  });
  const aiReady = await exists('//button[normalize-space(.)="Remove OpenFrame AI…"]');
  if (aiReady) {
    await state("settings-offline-ai-remove-confirm", async () => {
      await t.press("Remove OpenFrame AI…");
      await t.el(t.dialog("Remove OpenFrame AI?"));
    });
    await state("settings-offline-ai-technical-details", async () => {
      await t.press("Cancel", DIALOG);
      await t.go('//summary[normalize-space(.)="Technical details"]');
      await t.el('//table[@aria-label="Offline AI components"]');
      await scrollEnd();
    });
    await state("settings-offline-ai-stopped-toast", async () => {
      await scrollEnd(0);
      await t.press("Stop Offline AI (free memory)");
      await t.waitText("Offline AI stopped", { timeout: 8000 });
    }, { keepToasts: true });
    await state("settings-offline-ai-graphics-off", async () => {
      await t.press("Off", rg("Graphics card"));
      await t.waitText("will run on the processor", { timeout: 8000 });
    }, { keepToasts: true });
    await quiet("graphics restore", () => t.press("Automatic", rg("Graphics card")));
  } else {
    log("Offline AI is not ready here, so its ready-state actions are not captured");
  }
  t.unreachable("settings-offline-ai-not-downloaded", "Needs the AI models removed from this computer (spec header e2e-ai-models: none); the tour runs with the models present.");
  t.unreachable("settings-offline-ai-download-states", "Downloading, paused, failed, not enough space, damaged and cannot-download need the test model host steered to fail or stall, which the tour does not run.");
  t.unreachable("settings-offline-ai-health-filled", "The AI health table fills only after questions were asked in the AI Assistant, which happens after this section in the tour.");
  t.unreachable("settings-offline-ai-export-report", "Export AI report opens the Windows Save dialog (native dialog).");

  // --------------------------------------------------------------- Advanced
  await state("settings-advanced", async () => {
    await t.settingsTab("Advanced");
    await t.waitText("Breakdown category colours and icons", { sel: "main" });
    await t.sleep(400);
  });

  await state("settings-advanced-changed-toast", async () => {
    await t.go('(//*[@role="group"][contains(@aria-label," colour")])[1]//button[not(@aria-pressed="true")][2]');
    await t.waitText("Changed the look of", { timeout: 8000 });
  }, { keepToasts: true });

  await state("settings-advanced-restored-toast", async () => {
    await t.press("Restore Defaults");
    await t.waitText("Restored the default category colours", { timeout: 8000 });
  }, { keepToasts: true });

  // ------------------------------------------------------------- Appearance
  await state("settings-appearance", async () => {
    await t.settingsTab("Appearance");
    await t.el(rg("Theme"));
    await t.waitText("No imported themes yet", { sel: "main" });
  });

  await state("settings-appearance-my-themes-empty", async () => {
    await scrollEnd();
    await t.sleep(300);
  });

  await state("settings-theme-paste-dialog", async () => {
    await t.press("Paste Theme…");
    await t.el("#ap-paste");
  });

  await state("settings-theme-too-large", async () => {
    await setArea("#ap-paste", "a".repeat(10 * 1024 + 300));
    await t.waitText("This is larger than 10 KB");
  });

  await state("settings-theme-paste-filled", async () => {
    await setArea("#ap-paste", NAVY);
    await t.el('//button[normalize-space(.)="Check Theme"][not(@disabled)]');
  });

  await state("settings-theme-check-corrected", async () => {
    await t.press("Check Theme", DIALOG);
    await t.waitText("Small changes were made", { timeout: 10_000 });
  });

  await state("settings-theme-check-readability", async () => {
    await t.go('//details/summary[starts-with(normalize-space(.),"Readability check")]');
    await t.el('//table[contains(@class,"ap-contrast")]');
  });

  await state("settings-theme-check-rejected", async () => {
    await t.press("Edit the text", DIALOG);
    await setArea("#ap-paste", FOG);
    await t.press("Check Theme", DIALOG);
    await t.waitText("This theme cannot be used. Nothing was changed.", { timeout: 10_000 });
  });

  await state("settings-theme-check-ok", async () => {
    await t.press("Edit the text", DIALOG);
    await setArea("#ap-paste", HARBOUR);
    await t.press("Check Theme", DIALOG);
    await t.waitText("This theme is readable in Light and Dark.", { timeout: 10_000 });
  });

  await state("settings-my-themes-filled", async () => {
    await t.press("Save to My Themes", DIALOG);
    await t.el('//*[@role="listitem"][@aria-label="Tour Harbour"]');
    await scrollEnd();
  }, { keepToasts: true });

  await state("settings-my-themes-rename-dialog", async () => {
    await t.press("Rename Tour Harbour");
    await t.el("#ap-rename-name");
  });

  await state("settings-my-themes-delete-confirm", async () => {
    await t.press("Cancel", DIALOG);
    await t.press("Delete Tour Harbour");
    await t.el(t.dialog('Delete the theme "Tour Harbour"?'));
  });

  await state("settings-my-themes-deleted-toast", async () => {
    await t.press("Delete Theme", DIALOG);
    await t.waitText("Theme deleted.", { timeout: 8000 });
  }, { keepToasts: true });
  t.unreachable("settings-theme-import-file", "Import Theme File opens the Windows file picker (native dialog).");
  t.unreachable("settings-my-themes-export", "Export of a theme opens the Windows Save dialog (native dialog).");
  t.unreachable("settings-theme-fallback-banner", "Shown only when the saved theme has disappeared from the app database, which cannot be arranged from the window.");

  // ---------------------------------------------------------------- Cleanup
  await tidy();
  await quiet("purge own deleted items", purgeMine);
  rmSync(dir, { recursive: true, force: true });
  await t.reset();
}
