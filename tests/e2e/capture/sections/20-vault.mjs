// WindowsHTML capture section: Idea Vault (project and global scope): views, add menu, every item drawer, dialogs, multiselect, context menus, folders, collections, recently deleted.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { picture } from "../seed.mjs";

const DIALOG = '//*[@role="dialog"]';
const DRAWER = '//aside[contains(@class,"drawer")]';
const SEARCH = '//input[@aria-label="Search ideas, tags, filenames"]';

/** A tiny silent WAV (8 kHz, 8 bit) so the audio player has something real to load. */
function wav(seconds = 1) {
  const n = 8000 * seconds;
  const b = Buffer.alloc(44 + n, 128);
  b.write("RIFF", 0);
  b.writeUInt32LE(36 + n, 4);
  b.write("WAVEfmt ", 8);
  b.writeUInt32LE(16, 16);
  b.writeUInt16LE(1, 20);
  b.writeUInt16LE(1, 22);
  b.writeUInt32LE(8000, 24);
  b.writeUInt32LE(8000, 28);
  b.writeUInt16LE(1, 32);
  b.writeUInt16LE(8, 34);
  b.write("data", 36);
  b.writeUInt32LE(n, 40);
  return b.toString("base64");
}
const text64 = (s) => Buffer.from(s).toString("base64");

export default async function (t) {
  const b = t.browser;
  const X = t.xq;

  // ---------------------------------------------------------------- helpers

  /** Move the pointer off the content so no hover shadow or checkbox is captured. */
  const away = async () => {
    await b.action("pointer", { parameters: { pointerType: "mouse" } }).move({ x: 800, y: 2, duration: 0 }).perform().catch(() => undefined);
    await b.releaseActions().catch(() => undefined);
  };
  const shot = (name, fn, opts) =>
    t.state(
      name,
      async () => {
        if (fn) await fn();
        await away();
      },
      opts,
    );

  const card = (name) => `//*[@aria-label="Idea Vault items"]//*[@data-item-id][contains(@aria-label,${X(name)})]`;
  const rowOf = (name) => `//*[@role="row"][@data-item-id][contains(.,${X(name)})]`;
  const panel = '//nav[@aria-label="Idea Vault sections"]';
  const panelItem = (label) => `${panel}//button[starts-with(normalize-space(.),${X(label)})]`;
  const view = (label) => t.go(`//*[@role="radiogroup"][@aria-label="View"]//*[@role="radio"][normalize-space(.)=${X(label)}]`);
  const scope = (label) => t.go(`//*[@role="radiogroup"][@aria-label="Which Idea Vault"]//*[@role="radio"][normalize-space(.)=${X(label)}]`);
  const cancel = () => t.press("Cancel", DIALOG);
  const closeDrawer = async () => {
    const x = await b.$(`${DRAWER}//button[@aria-label="Close"]`);
    if (await x.isExisting()) {
      await x.click();
      await t.sleep(300);
    }
  };
  /** Open an item's drawer through its context menu (a plain click would select it and show the bar). */
  const openItem = async (name) => {
    await t.rightClick(card(name));
    await t.menuItem("Open");
    await t.el(DRAWER);
    await t.sleep(400);
  };
  const clearSelection = async () => {
    const x = await b.$('//button[@aria-label="Clear selection"]');
    if (await x.isExisting()) await x.click();
    await t.sleep(250);
  };
  const clearSearch = async () => {
    const x = await b.$('//button[@aria-label="Clear search"]');
    if (await x.isExisting()) await x.click();
    await t.sleep(500);
  };
  const trash = () => t.op("vault.trash_list", { store: "project" });
  /** Permanently remove what was deleted since `before` (so the seeded Recently Deleted row stays alone). */
  const purgeNew = async (before) => {
    for (const r of await trash()) if (!before.has(r.deleted.id)) await t.op("vault.purge", { store: "project", id: r.deleted.id });
  };
  const trashIds = async () => new Set((await trash()).map((r) => r.deleted.id));
  const vaultCreate = (args, store = "project") =>
    t.op("vault.create", { store, title: null, body: null, caption: null, url: null, sourceText: null, folderId: null, collectionId: null, tags: [], ...args });
  const idOf = (x) => (typeof x === "string" ? x : x?.id);
  const press = (name) => t.press(name, DIALOG);
  const radio = (label) => t.go(`${DIALOG}//label[.//*[normalize-space(.)=${X(label)}]]`);

  /** Back to a known place: overlays shut, nothing selected or searched, This Project, Grid, All items. */
  const baseline = async () => {
    await t.closeOverlays().catch(() => undefined);
    await closeDrawer().catch(() => undefined);
    if (!(await (await b.$('//main[@aria-label="Idea Vault"]')).isExisting())) await t.workspace("Idea Vault");
    await clearSelection();
    await clearSearch();
    if ((await (await b.$('//*[@role="radiogroup"][@aria-label="Which Idea Vault"]//*[@role="radio"][normalize-space(.)="This Project"]')).getAttribute("aria-checked")) !== "true") await scope("This Project");
    await view("Grid");
    await t.go(panelItem("All items"));
  };
  /** Run one group of states; a stop inside it is logged, then the next group starts from the baseline. */
  const block = async (name, fn) => {
    try {
      await baseline();
      await fn();
    } catch (e) {
      console.log(`  vault group ${name} stopped: ${String(e.message).split("\n")[0].slice(0, 240)}`);
    }
  };

  await t.workspace("Idea Vault");
  await view("Grid");
  await t.sleep(600);

  // ------------------------------------------------------- views and scope

  await shot("vault-grid-all-items", () => t.sleep(300), { home: "Idea Vault" });
  await shot("vault-view-card", () => view("Card"));
  await shot("vault-view-list", () => view("List"));
  await shot("vault-view-folder", () => view("Folder"));
  await shot("vault-view-folder-open", () => t.go('//button[contains(@class,"vv-folder")][contains(.,"Locations")]'));
  await view("Grid");
  await t.go(panelItem("All items"));

  await block("panelviews", async () => {
  await shot("vault-view-pinned", () => t.go(panelItem("Pinned")));
  await shot("vault-view-recently-added", () => t.go(panelItem("Recently added")));
  await shot("vault-view-recently-modified", () => t.go(panelItem("Recently modified")));
  await shot("vault-view-collection", () => t.go(panelItem("Look and feel")));
  await shot("vault-view-tag", () => t.go(panelItem("opening")));
  await shot("vault-view-folder-from-panel", () => t.go(panelItem("Locations")));
  await t.go(panelItem("All items"));

  });
  await block("addmenu", async () => {
  await shot("vault-add-menu", () => t.press("Add"));
  await t.closeOverlays();

  });
  await block("search", async () => {
  await shot("vault-search-results", async () => {
    await t.type(SEARCH, "radio");
    await t.waitText("result");
    await t.sleep(400);
  });
  await shot("vault-search-no-results", async () => {
    await t.type(SEARCH, "zzzzqq");
    await t.waitText("No results for");
  });
  await clearSearch();

  });
  await block("global", async () => {
  await shot("vault-scope-global", () => scope("Global"));
  await shot("vault-global-pinned-empty", () => t.go(panelItem("Pinned")));
  await t.go(panelItem("All items"));
  await shot("vault-global-item-context-menu", () => t.rightClick(card("Ideas for the next film")));
  await t.closeOverlays();

  // Global to project copy: the dialog, the result, and the copy's drawer.
  const before = await trashIds();
  await shot("vault-copy-to-project-dialog", async () => {
    await t.rightClick(card("Ideas for the next film"));
    await t.menuItem("Copy to Project");
    await t.el(DIALOG);
  });
  await shot(
    "vault-copy-to-project-done",
    async () => {
      await press("Copy to Project");
      await t.waitText("Copied");
    },
    { keepToasts: true },
  );
  await scope("This Project");
  await t.sleep(500);
  await shot("vault-drawer-copied-from-global", () => openItem("Ideas for the next film"));
  await closeDrawer();
  const copies = await t.op("vault.list", { store: "project", view: { kind: "all" }, search: "Ideas for the next film" });
  if (copies.length) await t.op("vault.delete", { store: "project", ids: copies.map((i) => i.id) });
  await purgeNew(before);
  await t.sleep(500);

  });
  await block("drawers", async () => {
  // -------------------------------------------------------------- drawers

  await shot("vault-drawer-note", () => openItem("Opening image"));
  await closeDrawer();
  await shot("vault-drawer-quote", () => openItem("A town is only"));
  await closeDrawer();
  await shot("vault-drawer-url", () => openItem("Coastal weather"));
  await shot("vault-drawer-url-invalid", async () => {
    await t.type("#vv-url", "not a link");
    await t.go("#vv-title");
    await t.waitText("web link");
  });
  // Put the real link back so the seeded item is unchanged.
  await t.type("#vv-url", "https://example.com/coastal-weather");
  await t.go("#vv-title");
  await t.waitText("Saved automatically");
  await closeDrawer();

  });
  await block("newnote", async () => {
  // A new untitled note from the Add menu, then deleted again.
  {
    const keep = await trashIds();
    await shot("vault-drawer-note-new", async () => {
      await t.press("Add");
      await t.menuItem("Note");
      await t.el("#vv-body");
      await t.sleep(500);
    });
    await shot(
      "vault-item-deleted-toast",
      async () => {
        await t.press("Delete", DRAWER);
        await t.waitText("Deleted");
        await t.sleep(300);
      },
      { keepToasts: true },
    );
    await purgeNew(keep);
  }
  await closeDrawer();

  });
  await block("menus", async () => {
  // Pinned and the research flag from the context menu.
  await shot("vault-item-context-menu", () => t.rightClick(card("Radio sounds")));
  await t.closeOverlays();
  await shot("vault-item-context-menu-pinned", () => t.rightClick(card("Opening image")));
  await t.closeOverlays();

  });
  await block("multi", async () => {
  await t.go(card("Radio sounds"));
  await t.go('//button[starts-with(@aria-label,"Select ")][contains(@aria-label,"A town is only")]');
  await shot("vault-multiselect-bar", () => t.sleep(200));
  await shot("vault-item-context-menu-multi", () => t.rightClick(card("Radio sounds")));
  await t.closeOverlays();
  // A dialog's backdrop dims the pictures behind it, which a static page cannot repeat; the List view has none.
  await view("List");
  // The selection bar dialogs (the same two items stay selected).
  await shot("vault-copy-to-global-dialog", () => t.press("Copy to Global Vault"));
  await t.closeOverlays();
  await shot("vault-move-to-folder-dialog", () => t.press("Move to Folder"));
  await shot("vault-move-to-folder-new-folder", () => t.press("New Folder…", DIALOG));
  await t.closeOverlays();
  await shot("vault-add-to-collection-dialog", () => t.press("Add to Collection"));
  await t.closeOverlays();
  await shot("vault-tag-dialog", () => t.press("Tag"));
  await shot("vault-tag-dialog-required", () => press("Add Tags"));
  await t.closeOverlays();
  await clearSelection();

  });
  await block("dialogs", async () => {
  await view("List");
  // ---------------------------------------------------------------- dialogs

  await shot("vault-add-url-dialog", async () => {
    await t.press("Add");
    await t.menuItem("URL");
    await t.el("#vv-add-url");
  });
  await shot("vault-add-url-invalid", async () => {
    await t.type("#vv-add-url", "not a link");
    await press("Add URL");
    await t.waitText("web link");
  });
  await t.closeOverlays();

  await shot("vault-quick-capture-dialog", () => t.press("Quick Capture"));
  {
    const keep = await trashIds();
    await shot("vault-quick-capture-filled", () => t.type('//textarea[@aria-label="Idea"]', "A bus that only runs when someone is waiting for it."));
    await shot(
      "vault-quick-capture-saved-toast",
      async () => {
        await press("Save");
        await t.waitText("Saved to the");
      },
      { keepToasts: true },
    );
    const made = await t.op("vault.list", { store: "project", view: { kind: "all" }, search: "A bus that only runs" });
    if (made.length) await t.op("vault.delete", { store: "project", ids: made.map((i) => i.id) });
    await purgeNew(keep);
    await t.sleep(500);
  }

  await shot("vault-sketch-dialog", async () => {
    await t.press("Add");
    await t.menuItem("Sketch");
    await t.el('//canvas[contains(@aria-label,"Sketch drawing area")]');
  });
  await shot("vault-sketch-dialog-drawn", async () => {
    const canvas = await t.el('//canvas[contains(@aria-label,"Sketch drawing area")]');
    await t.go('//button[@aria-label="Red ink"]');
    const stroke = async (from, to) => {
      await b
        .action("pointer", { parameters: { pointerType: "mouse" } })
        .move({ origin: canvas, x: from[0], y: from[1] })
        .down()
        .move({ origin: canvas, x: to[0], y: to[1], duration: 250 })
        .up()
        .perform();
      await b.releaseActions().catch(() => undefined);
    };
    await stroke([-300, 80], [-100, -60]);
    await stroke([-100, -60], [100, 90]);
    await stroke([100, 90], [320, -80]);
    await t.go('//button[@aria-label="Blue ink"]');
    await stroke([-320, 120], [320, 120]);
    await t.sleep(300);
  });
  await t.closeOverlays();

  });
  await block("record", async () => {
  await view("List");
  // Recording needs a microphone: the dialog shows whichever state this computer is in.
  await t.go('//button[normalize-space(.)="Record Note"]');
  await t.sleep(2500);
  const recText = await t.text('[role="dialog"]');
  if (recText.includes("couldn't use the microphone")) {
    await shot("vault-record-note-mic-error");
  } else if (recText.includes("Recording")) {
    await shot("vault-record-note-recording");
  } else if (recText.includes("Starting microphone")) {
    await shot("vault-record-note-dialog");
  } else {
    t.unreachable("vault-record-note-dialog", "the microphone did not answer in the test window");
  }
  await t.closeOverlays();
  t.unreachable("vault-record-note-saved", "needs a real recording; the test window has no microphone input");
  t.unreachable("vault-add-files-native-picker", "Add > Image, File, Audio file and Video open the Windows file picker, which a screenshot tour cannot drive");
  t.unreachable("vault-add-result-dialog", "shown only after the native picker adds a file that cannot be read");
  t.unreachable("vault-drop-overlay", "appears while files are dragged in from Explorer, an OS drag that WebDriver cannot start");

  });
  await block("send", async () => {
  await view("List");
  // Send to Beat Board (cancelled each time: the board stays as it is).
  await shot("vault-send-to-beat-board-dialog", async () => {
    await t.rightClick(rowOf("Opening image"));
    await t.menuItem("Send to Story");
    await t.el(DIALOG);
  });
  await shot("vault-send-to-beat-board-sequence", () => radio("Sequence idea"));
  await shot("vault-send-to-beat-board-character", () => radio("Character note"));
  await t.closeOverlays();

  });
  await block("folders", async () => {
  await view("List");
  // ---------------------------------------------------- folders, collections

  await shot("vault-new-collection-dialog", () => t.go(`${panel}//button[contains(normalize-space(.),"New Collection")]`));
  await shot("vault-new-collection-name-required", () => press("Create"));
  await t.type("#vv-name", "Scratch collection");
  await press("Create");
  await t.sleep(700);
  await shot("vault-collection-empty", () => t.sleep(200));

  await shot("vault-new-folder-dialog", () => t.go(`${panel}//button[contains(normalize-space(.),"New Folder")]`));
  await shot("vault-new-folder-name-required", () => press("Create"));
  await t.type("#vv-name", "Scratch folder");
  await press("Create");
  await t.sleep(700);
  await t.go(panelItem("Scratch folder"));
  await shot("vault-folder-empty", () => t.sleep(200));

  await shot("vault-folder-context-menu", () => t.rightClick(panelItem("Locations")));
  await t.closeOverlays();
  await shot("vault-collection-context-menu", () => t.rightClick(panelItem("Look and feel")));
  await t.closeOverlays();

  await shot("vault-rename-folder-dialog", async () => {
    await t.rightClick(panelItem("Scratch folder"));
    await t.menuItem("Rename");
    await t.el("#vv-name");
  });
  await t.closeOverlays();
  await shot("vault-rename-collection-dialog", async () => {
    await t.rightClick(panelItem("Scratch collection"));
    await t.menuItem("Rename");
    await t.el("#vv-name");
  });
  await t.closeOverlays();

  await shot(
    "vault-folder-deleted-toast",
    async () => {
      await t.rightClick(panelItem("Scratch folder"));
      await t.menuItem("Delete Folder");
      await t.waitText("Deleted folder");
    },
    { keepToasts: true },
  );
  await t.rightClick(panelItem("Scratch collection"));
  await t.menuItem("Delete Collection");
  await t.waitText("Deleted collection");
  await t.go(panelItem("All items"));

  await view("Grid");

  });
  await block("foldercards", async () => {
  // Folder view: the folder context menu on a folder card.
  await view("Folder");
  await shot("vault-folder-card-context-menu", () => t.rightClick('//button[contains(@class,"vv-folder")][contains(.,"Locations")]'));
  await t.closeOverlays();
  await view("Grid");
  await t.go(panelItem("All items"));

  });
  await block("pinned", async () => {
  // The seeded Pinned item, unpinned for the empty Pinned view and pinned again.
  try {
    await t.rightClick(card("Opening image"));
    await t.menuItem("Unpin");
    await t.sleep(500);
    await t.go(panelItem("Pinned"));
    await shot("vault-pinned-empty", () => t.sleep(200));
  } finally {
    const pinned = await t.op("vault.list", { store: "project", view: { kind: "all" }, search: "Opening image" });
    await t.op("vault.set_pinned", { store: "project", ids: pinned.map((i) => i.id), pinned: true });
    await t.sleep(500);
  }

  });
  await block("types", async () => {
  // ------------------------------------------------------ every item type

  const dir = mkdtempSync(join(process.env.E2E_DATA_ROOT ?? tmpdir(), "capture-vault-"));
  const typed = [];
  try {
    const folder = idOf(await t.op("vault.create_folder", { store: "project", name: "Reference files", parentId: null }));
    const ingest = async (itemType, fileName, mediaType, dataBase64, title, extra = {}) => {
      try {
        const it = await t.op("vault.ingest_bytes", {
          store: "project", itemType, fileName, mediaType, dataBase64, durationMs: null, title, caption: null, folderId: folder, collectionId: null, ...extra,
        });
        typed.push(idOf(it));
        return true;
      } catch {
        return false;
      }
    };
    const made = {
      image: await ingest("image", "Jetty at noon.png", "image/png", picture(480, 480, 70), "Jetty at noon", { caption: "Square crop, east jetty" }),
      sketch: await ingest("sketch", "Rooftop sketch.png", "image/png", picture(480, 480, 90), "Rooftop sketch"),
      voice: await ingest("voice", "Voice note 2026.wav", "audio/wav", wav(2), "Radio hiss at the harbour", { durationMs: 2000 }),
      audio: await ingest("audio", "Theme idea.wav", "audio/wav", wav(1), "Theme idea", { durationMs: 1000 }),
      video: await ingest("video", "Jetty walk.mp4", "video/mp4", text64("not a real video, kept as is"), "Jetty walk-through", { durationMs: 42000 }),
      pdf: await ingest("pdf", "Tower permit.pdf", "application/pdf", text64("%PDF-1.4\n% permit\n"), "Tower permit"),
      document: await ingest("document", "Treatment draft.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", text64("PK treatment"), "Treatment draft"),
      file: await ingest("file", "Crew contacts.csv", "text/csv", text64("Name,Role\nDivya,DOP\n"), "Crew contacts"),
    };
    // A linked file that disappears: the card keeps its place and says Unavailable.
    const gone = join(dir, "Harbour notes.txt");
    writeFileSync(gone, "Tide tables for the east jetty.\n");
    const linked = await t.op("vault.add_files", { store: "project", paths: [gone], mode: "link", folderId: folder, collectionId: null, itemType: null });
    typed.push(...(linked.added ?? []).map((i) => i.id));
    rmSync(gone);
    await b.refresh();
    await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
    await t.workspace("Idea Vault");
    await view("Grid");
    await t.sleep(900);

    await shot("vault-grid-all-types", () => t.sleep(300));
    await shot("vault-view-list-all-types", () => view("List"));
    await view("Grid");
    const drawers = [
      ["image", "vault-drawer-image", "Jetty at noon"],
      ["sketch", "vault-drawer-sketch", "Rooftop sketch"],
      ["voice", "vault-drawer-voice", "Radio hiss"],
      ["audio", "vault-drawer-audio", "Theme idea"],
      ["video", "vault-drawer-video", "Jetty walk"],
      ["pdf", "vault-drawer-pdf", "Tower permit"],
      ["document", "vault-drawer-document", "Treatment draft"],
      ["file", "vault-drawer-file", "Crew contacts"],
    ];
    for (const [key, name, label] of drawers) {
      if (!made[key]) {
        t.unreachable(name, `the app refused to store a ${key} item from bytes in this run`);
        continue;
      }
      await shot(name, () => openItem(label), { wait: 1000 });
      await closeDrawer();
    }
    if ((linked.added ?? []).length) {
      await shot("vault-drawer-file-unavailable", () => openItem("Harbour notes"), { wait: 900 });
      await closeDrawer();
    }
  } finally {
    const keep = await trashIds();
    if (typed.length) await t.op("vault.delete", { store: "project", ids: typed }).catch(() => undefined);
    await purgeNew(keep).catch(() => undefined);
    const folders = (await t.op("vault.overview", { store: "project" }).catch(() => ({ folders: [] }))).folders ?? [];
    for (const f of folders.filter((x) => x.name === "Reference files")) await t.op("vault.delete_folder", { store: "project", id: f.id }).catch(() => undefined);
    rmSync(dir, { recursive: true, force: true });
  }
  await t.sleep(600);

  });
  await block("notfound", async () => {
  // ----------------------------------------------- the item that is not there

  await b.execute(() =>
    sessionStorage.setItem("openframe.route", JSON.stringify({ workspace: "vault", params: { itemId: "00000000-0000-4000-8000-000000000000" } })),
  );
  await b.refresh();
  await b.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
  await shot("vault-drawer-not-found", () => t.waitText("Item not found", { timeout: 15_000 }));
  await closeDrawer();
  await t.workspace("Idea Vault");

  });
  await block("trash", async () => {
  // -------------------------------------------------------- Recently Deleted

  await shot("vault-recently-deleted", () => t.press("Recently Deleted"));
  await shot("vault-purge-confirm", () => t.press("Delete Permanently"));
  await press("Keep in Recently Deleted");
  await t.sleep(300);
  await t.press("Back to Vault");
  await scope("Global");
  await t.press("Recently Deleted");
  await shot("vault-recently-deleted-empty", () => t.waitText("Nothing deleted"));
  await t.press("Back to Vault");
  await scope("This Project");

  });
  await block("empty", async () => {
  // -------------------------------------------------------------- empty

  await t.reset();
  await t.inEmptyProject(async () => {
    await t.workspace("Idea Vault");
    await shot("vault-empty-project", () => t.waitText("Start collecting anything about this film."));
    await shot("vault-empty-add-menu", () => t.press("Add"));
    await t.closeOverlays();
    // An idea with no acts on the board yet: sending it as a sequence has nowhere to go.
    const note = await vaultCreate({ itemType: "note", title: "Idea for later", body: "The tower hums when the bus passes." });
    await t.sleep(700);
    try {
      await shot("vault-send-to-beat-board-no-acts", async () => {
        await t.rightClick(card("Idea for later"));
        await t.menuItem("Send to Story");
        await t.el(DIALOG);
        await radio("Sequence idea");
        await t.waitText("There are no acts yet");
      });
      await t.closeOverlays();
    } finally {
      const keep = await t.op("vault.trash_list", { store: "project" }).then((r) => new Set(r.map((x) => x.deleted.id)));
      await t.op("vault.delete", { store: "project", ids: [idOf(note)] }).catch(() => undefined);
      for (const r of await t.op("vault.trash_list", { store: "project" })) if (!keep.has(r.deleted.id)) await t.op("vault.purge", { store: "project", id: r.deleted.id });
    }
    await t.workspace("Home");
  });

  });
  await t.reset();
}
