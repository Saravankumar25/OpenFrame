// WindowsHTML capture section: Production planning tabs: Overview, Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, their dialogs, drawers, menus and empty states.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

const DIALOG = '//*[@role="dialog"]';
const DRAWER = '//aside[contains(@class,"drawer")]';
const radio = (group, text) => `//*[@role="radiogroup"][@aria-label="${group}"]//*[@role="radio"][contains(normalize-space(.),"${text}")]`;
const GROUP = '//*[@role="group"][@aria-label="Storyboard panels"]';
const panelCard = (n) => `${GROUP}//*[contains(@class,"pnl")][${n}]`;
const tile = (prefix) => `//*[@role="button"][starts-with(@aria-label,"${prefix}")]`;

export default async function (t) {
  const b = () => t.browser;
  const row = (name) => `//main//tr[.//b[normalize-space(.)=${t.xq(name)}]]`;
  const card = (name) => `//main//button[contains(@class,"vcard")][.//*[contains(@class,"vt")][normalize-space(.)=${t.xq(name)}]]`;

  /** A click that is cleanup, not a way to the next screen (not noted for page links). */
  const quiet = async (sel) => {
    try {
      const el = await t.el(sel);
      await el.scrollIntoView({ block: "center" }).catch(() => undefined);
      await el.click();
      await t.sleep(300);
    } catch (e) {
      console.log(`  quiet click failed: ${String(sel).slice(0, 90)}: ${e instanceof Error ? e.message.split("\n")[0].slice(0, 120) : e}`);
      await t.closeOverlays().catch(() => undefined);
    }
  };
  /** An interaction outside a captured state: a failure is logged and the section goes on. */
  const soft = (fn) => fn().catch((e) => console.log(`  step failed: ${e instanceof Error ? e.message.split("\n")[0].slice(0, 140) : e}`));
  const cancel = () => quiet(t.btn("Cancel", DIALOG));
  const closeDrawer = async () => {
    const x = await b().$(`${DRAWER}//button[@aria-label="Close"]`);
    if (await x.isExisting()) {
      await x.click();
      await t.sleep(300);
    }
  };
  const reload = async () => {
    await b().refresh();
    await b().$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
    await t.sleep(1000);
  };
  const tab = (label) => quiet(`//nav[@aria-label="Production"]//button[normalize-space(.)=${t.xq(label)}]`);
  const undoLast = async (n = 1) => {
    for (let i = 0; i < n; i++) {
      const r = await t.op("history.undo", { store: "project" });
      console.log(`    undid: ${r?.label ?? "nothing"}`);
    }
    await t.sleep(600);
  };
  const scrollDrawer = () =>
    b().execute(() => {
      for (const el of document.querySelectorAll("aside.drawer .db, aside.drawer .db > div")) el.scrollTop = el.scrollHeight;
    });
  /** Choose an option of a native select by its text. */
  const choose = async (sel, text) => {
    await (await t.el(sel)).selectByVisibleText(text);
    await t.sleep(500);
  };
  const stroke = async (area, from, to) => {
    await b()
      .action("pointer", { parameters: { pointerType: "mouse" } })
      .move({ origin: area, x: from[0], y: from[1] })
      .down()
      .move({ origin: area, x: to[0], y: to[1], duration: 250 })
      .up()
      .perform();
    await b().releaseActions().catch(() => undefined);
  };

  // ---- Overview ----------------------------------------------------------------------------
  await t.workspace("Production");
  await t.state("prod-overview", () => t.el('//main//div[contains(@class,"grid")]//button[contains(@class,"card")]'), { home: "Production" });
  await t.state("prod-overview-source-dialog", () => t.press("Change Production Source"));
  await t.closeOverlays();

  // ---- Empty project: every tab without data -----------------------------------------------
  await t.inEmptyProject(async () => {
    await t.workspace("Production");
    await tab("Overview");
    await t.state("prod-overview-no-source", () => t.waitText("Start Production Setup"));
    await t.state("prod-overview-start-setup-dialog", () => t.press("Start Production Setup"));
    await t.closeOverlays();
    await t.state("prod-catalog-empty", async () => {
      await t.productionTab("Catalog");
      await t.waitText("The catalog is empty.");
    });
    await t.state("prod-locations-empty", async () => {
      await t.productionTab("Locations");
      await t.waitText("No locations yet.");
    });
    await t.state("prod-cast-empty", async () => {
      await t.productionTab("Cast & Crew");
      await t.waitText("No cast yet.");
    });
    await t.state("prod-crew-empty", async () => {
      await t.go(radio("Directory", "Crew"));
      await t.waitText("No crew yet.");
    });
    await t.state("prod-moodboards-empty", async () => {
      await t.productionTab("Moodboards");
      await t.waitText("No moodboards yet");
    });
    await t.state("prod-storyboards-empty", async () => {
      await t.productionTab("Storyboards");
      await t.waitText("No storyboards yet");
    });
    await t.state("prod-shots-no-script", async () => {
      await t.productionTab("Shot Lists");
      await t.waitText("No screenplay scenes yet");
    });
  });

  // ---- Catalog -----------------------------------------------------------------------------
  await t.workspace("Production");
  await t.state("prod-catalog", async () => {
    await t.productionTab("Catalog");
    await t.waitText("Soldering iron");
  });
  await t.state("prod-catalog-category-props", () => t.go(radio("Category", "Props")));
  await t.state("prod-catalog-more-menu", () => t.go('//button[@aria-label="More categories"]'));
  await t.state("prod-catalog-no-match", async () => {
    await t.menuItem("Sound");
    await t.waitText("No catalog items match.");
  });
  await quiet(radio("Category", "All"));
  await t.state("prod-catalog-row-menu", () => t.rightClick(row("Soldering iron")));
  await t.state("prod-catalog-drawer", () => t.menuItem("Open"));
  await t.state("bd-catalog-scenes-banner", async () => {
    await t.press("Open Scenes", DRAWER);
    await t.waitText("scene that use");
  });
  await quiet(t.btn("Show all scenes"));
  await soft(() => t.productionTab("Catalog"));
  await quiet(row("Soldering iron"));
  await t.state("prod-catalog-archive-confirm", () => t.press("Archive", DRAWER));
  await t.state("prod-catalog-replace-dialog", () => t.press("Replace in Scenes…", DIALOG));
  await cancel();
  await t.state("prod-catalog-delete-confirm", () => t.press("Delete", DRAWER));
  await cancel();
  await closeDrawer();
  await t.state("prod-catalog-add-dialog", () => t.press("Add Item"));
  await t.state("prod-catalog-add-duplicate-warning", async () => {
    await t.type("#ai-name", "Marine radio receiver");
    await t.waitText("The catalog already has");
  });
  await cancel();
  await t.state("prod-catalog-export-dialog", () => t.press("Export"));
  await t.closeOverlays();

  // ---- Locations ---------------------------------------------------------------------------
  // Two locations are tagged in scenes for the length of this part, and untagged again with undo.
  const scenes = (await t.op("breakdown.scenes", { filter: null, category: null, catalogItemId: null })).scenes;
  let tagged = 0;
  for (const [name, scene] of [["Rayapuram bus stand", scenes[0]], ["Old cinema hall", scenes[1]]]) {
    try {
      await t.op("breakdown.add_element", { sceneId: scene.sceneId, category: "Location / Set", name, notes: null, span: null, catalog: { mode: "auto", catalogItemId: null } });
      tagged++;
    } catch (e) {
      console.log(`  prod could not tag ${name}: ${e instanceof Error ? e.message.split("\n")[0] : e}`);
    }
  }
  await reload();
  await t.state("prod-locations", async () => {
    await t.productionTab("Locations");
    await t.waitText("Rayapuram bus stand");
  });
  await t.state("prod-locations-filter-confirmed", () => t.go(radio("Status", "Confirmed")));
  await quiet(radio("Status", "All"));
  await t.state("prod-locations-no-match", async () => {
    await t.type('//input[@aria-label="Search locations"]', "zzzz");
    await t.waitText("No locations match.");
  });
  await soft(() => t.type('//input[@aria-label="Search locations"]', ""));
  await t.sleep(300);
  await t.state("prod-location-row-menu", () => t.rightClick(card("Rayapuram bus stand")));
  await t.state("prod-location-drawer", () => t.menuItem("Open"));
  await t.state("prod-location-drawer-notes", () => scrollDrawer());
  await t.state("prod-location-delete-confirm", () => t.press("Delete", DRAWER));
  await cancel();
  await closeDrawer();
  await t.state("prod-location-drawer-hospital", async () => {
    await t.go(card("Pedda Revu tower"));
    await t.el("#loc-hospital");
  });
  await closeDrawer();
  if (tagged === 2) {
    await t.state("prod-location-rejected-banner", async () => {
      await t.go(card("Old cinema hall"));
      await t.waitText("Rejected, but still used in");
    });
    await t.state("prod-location-replace-dialog", () => t.press("Replace…"));
    await cancel();
    await closeDrawer();
  } else {
    t.unreachable("prod-location-rejected-banner", "the rejected location could not be tagged in a scene when the section ran");
  }
  await t.state("prod-location-add-dialog", () => t.press("Add Location"));
  await cancel();
  await undoLast(tagged);

  // ---- Cast and crew -----------------------------------------------------------------------
  await t.state("prod-cast", async () => {
    await t.productionTab("Cast & Crew");
    await t.waitText("Ananya Rao");
  });
  await t.state("prod-cast-assign-dialog", () => t.go('(//main//button[normalize-space(.)="Assign"])[1]'));
  await t.state("prod-cast-assign-alternate-banner", async () => {
    await t.type("#as-person", "Lata Nair");
    await t.go(`${DIALOG}//*[@role="radio"][.//b[normalize-space(.)="LAKSHMI"]]`);
    await t.go(`${DIALOG}//*[@role="radio"][normalize-space(.)="Primary actor"]`);
    await t.waitText("becomes an alternate");
  });
  await cancel();
  await t.state("prod-cast-row-menu", () => t.rightClick(row("Ananya Rao")));
  await t.state("prod-cast-assign-existing-dialog", () => t.menuItem("Assign Character"));
  await cancel();
  await soft(() => t.rightClick(row("Ananya Rao")));
  await t.state("prod-cast-drawer", () => t.menuItem("Open"));
  await t.state("prod-cast-remove-confirm", () => t.press("Delete", DRAWER));
  await cancel();
  await closeDrawer();
  await t.state("prod-crew", () => t.go(radio("Directory", "Crew")));
  await t.state("prod-crew-drawer", () => t.go(row("Divya Menon")));
  await t.state("prod-crew-remove-confirm", () => t.press("Delete", DRAWER));
  await cancel();
  await closeDrawer();
  await t.state("prod-crew-add-dialog", () => t.press("Add Person"));
  await cancel();

  // ---- Moodboards --------------------------------------------------------------------------
  await t.state("prod-moodboard", async () => {
    await t.productionTab("Moodboards");
    await t.el(tile("Note: "));
  });
  await t.state("prod-moodboard-note-selected", () => t.go(tile("Note: ")));
  await t.state("prod-moodboard-multi-selection", async () => {
    const el = await t.el(tile("Link: "));
    await b().actions([
      b().action("key").down("").pause(0).pause(0).pause(0).up(""),
      b().action("pointer", { parameters: { pointerType: "mouse" } }).move({ origin: el }).down().up().pause(0).pause(0),
    ]);
    await b().releaseActions().catch(() => undefined);
    await t.waitText("2 items selected");
  });
  await t.state("prod-moodboard-item-menu", () => t.rightClick(tile("Note: ")));
  await t.state("prod-moodboard-note-drawer", () => t.menuItem("Edit"));
  await t.state("prod-moodboard-note-error", async () => {
    await t.type("#mb-note", "");
    await t.keys("Tab");
    await t.waitText("A note needs some text.");
  });
  await closeDrawer();
  await soft(() => t.rightClick(tile("Link: ")));
  await t.state("prod-moodboard-link-drawer", () => t.menuItem("Edit"));
  await closeDrawer();
  await soft(() => t.rightClick(tile("Image")));
  await t.state("prod-moodboard-image-drawer", () => t.menuItem("Edit"));
  await closeDrawer();
  await t.state("prod-moodboard-add-image-menu", () => t.go('//button[normalize-space(.)="Add image"]'));
  await t.state("prod-moodboard-vault-dialog", () => t.menuItem("From Idea Vault"));
  await cancel();
  await t.state("prod-moodboard-add-note-dialog", () => t.press("Add note"));
  await t.state("prod-moodboard-add-note-error", async () => {
    await t.press("Add Note", DIALOG);
    await t.waitText("Write the note first.");
  });
  await cancel();
  await t.state("prod-moodboard-add-link-dialog", () => t.press("Add link"));
  await t.state("prod-moodboard-add-link-filled", async () => {
    await t.type("#new-link", "https://example.com/blue-hour");
    await t.type("#new-link-title", "Blue hour reference");
  });
  await cancel();
  const boardRow = '//nav[@aria-label="Moodboards"]//button[contains(@class,"li")][1]';
  await t.state("prod-moodboard-list-menu", () => t.rightClick(boardRow));
  await t.state("prod-moodboard-rename-dialog", () => t.menuItem("Rename"));
  await cancel();
  await soft(() => t.rightClick(boardRow));
  await t.state("prod-moodboard-board-notes-dialog", () => t.menuItem("Board notes"));
  await cancel();
  await soft(() => t.rightClick(boardRow));
  await t.state("prod-moodboard-delete-confirm", () => t.menuItem("Delete"));
  await cancel();
  await t.state("prod-moodboard-export-dialog", () => t.press("Export"));
  await t.closeOverlays();
  await t.state("prod-moodboard-new-dialog", () => t.press("New Moodboard"));
  await t.state("prod-moodboard-new-error", async () => {
    await t.press("Create Moodboard", DIALOG);
    await t.waitText("Give the board a name.");
  });
  await t.state("prod-moodboard-new-filled", async () => {
    await t.go(`${DIALOG}//button[contains(@class,"chip")][1]`);
  });
  let made = false;
  await t.state("prod-moodboard-empty-canvas", async () => {
    await t.press("Create Moodboard", DIALOG);
    made = true;
    await t.waitText("Drop images, notes and references here.");
  });
  if (made) {
    await undoLast(1);
    await reload();
  }

  // ---- Storyboards -------------------------------------------------------------------------
  const sceneSelect = '//select[@aria-label="Scene or storyboard"]';
  const pickBoard = async (match) => {
    const opts = await b().execute(() => [...document.querySelectorAll('select[aria-label="Scene or storyboard"] option')].map((o) => [o.value, o.textContent]));
    const hit = opts.find(([, text]) => match(text));
    if (!hit) throw new Error(`no storyboard option: ${JSON.stringify(opts.slice(0, 5))}`);
    await (await t.el(sceneSelect)).selectByAttribute("value", hit[0]);
    await t.sleep(600);
  };
  await t.state("prod-storyboard-panels", async () => {
    await t.productionTab("Storyboards");
    await t.el(GROUP);
  });
  await t.state("prod-storyboard-new-dialog", () => t.press("New Storyboard"));
  await t.state("prod-storyboard-new-error", async () => {
    await choose("#new-sb-scene", "No scene (standalone storyboard)");
    await t.press("Create Storyboard", DIALOG);
    await t.waitText("Give the storyboard a name, or choose a scene.");
  });
  let board = false;
  await t.state("prod-storyboard-no-panels", async () => {
    await t.type("#new-sb-name", "Title sequence");
    await t.press("Create Storyboard", DIALOG);
    board = true;
    await t.waitText("Create panels for this scene.");
  });
  await pickBoard((x) => x.startsWith("Scene 1:"));
  await t.el(GROUP);
  await t.state("prod-storyboard-panel-menu", () => t.rightClick(panelCard(1)));
  await t.state("prod-storyboard-move-panel-dialog", () => t.menuItem("Move to storyboard"));
  await cancel();
  await t.state("prod-storyboard-panel-drawer", () => t.go(panelCard(2)));
  await t.state("prod-storyboard-panel-duration-error", async () => {
    await t.type("#panel-dur", "abc");
    await t.keys("Tab");
    await t.waitText("Enter seconds between 0 and 3600.");
  });
  await closeDrawer();
  await t.state("prod-storyboard-add-panel-menu", () => t.go('//div[contains(@class,"pha")]//button[normalize-space(.)="Add Panel"]'));
  await t.state("prod-storyboard-sketch-dialog", () => t.menuItem("Draw / Sketch"));
  await t.state("prod-storyboard-sketch-drawn", async () => {
    const area = await t.el('//*[@aria-label="Sketch area"]');
    await stroke(area, [-90, 30], [-20, -40]);
    await stroke(area, [-20, -40], [80, 40]);
  });
  await cancel();
  await t.state("prod-storyboard-actions-menu", () => t.go('//button[@aria-label="Storyboard actions"]'));
  await t.state("prod-storyboard-rename-dialog", () => t.menuItem("Rename"));
  await cancel();
  await quiet('//button[@aria-label="Storyboard actions"]');
  await t.state("prod-storyboard-link-scene-dialog", () => t.menuItem("Change scene"));
  await cancel();
  await quiet('//button[@aria-label="Storyboard actions"]');
  await t.state("prod-storyboard-new-for-scene-dialog", () => t.menuItem("Another storyboard"));
  await cancel();
  await quiet('//button[@aria-label="Storyboard actions"]');
  await t.state("prod-storyboard-delete-confirm", () => t.menuItem("Delete storyboard"));
  await cancel();
  await t.state("prod-storyboard-export-dialog", () => t.press("Export"));
  await t.closeOverlays();
  await t.state("prod-storyboard-scene-no-board", async () => {
    await pickBoard((x) => x.startsWith("Scene 3:"));
    await t.waitText("Create panels for this scene.");
  });
  if (board) {
    await undoLast(1);
    await reload();
  }

  // ---- Shot lists --------------------------------------------------------------------------
  const sceneChoice = '//select[@aria-label="Scene"]';
  await t.state("prod-shots-list", async () => {
    await t.productionTab("Shot Lists");
    await t.el('//*[@role="list"][@aria-label="Shots"]');
  });
  await t.state("prod-shots-grid", () => t.go(radio("Shot list view", "Grid")));
  await quiet(radio("Shot list view", "List"));
  await t.state("prod-shots-all-scenes", () => choose(sceneChoice, "All scenes"));
  await t.state("prod-shots-scene-empty", async () => {
    await (await t.el(sceneChoice)).selectByIndex(3);
    await t.waitText("No shots in this scene yet");
  });
  await t.state("prod-shots-composer-error", async () => {
    await t.go('//main//form//button[normalize-space(.)="Add Shot"]');
    await t.waitText("Describe the shot to add it.");
  });
  await (await t.el(sceneChoice)).selectByIndex(1);
  await t.sleep(500);
  await t.el('//*[@role="list"][@aria-label="Shots"]');
  const shot = '//*[@role="list"][@aria-label="Shots"]//*[@role="listitem"][1]';
  await t.state("prod-shot-menu", () => t.rightClick(shot));
  await t.state("prod-shot-drawer", () => t.menuItem("Open"));
  await closeDrawer();
  await soft(() => t.rightClick(shot));
  await t.state("prod-shot-move-dialog", () => t.menuItem("Move to scene"));
  await cancel();
  await soft(() => t.rightClick(shot));
  await t.state("prod-shot-attach-dialog", () => t.menuItem("Attach Storyboard"));
  await cancel();
  await t.state("prod-shots-scene-actions-menu", () => t.go('//button[@aria-label="More scene actions"]'));
  await t.state("prod-shots-copy-planning-dialog", () => t.menuItem("Copy planning"));
  await cancel();
  await t.state("prod-shots-export-dialog", () => t.press("Export"));
  await t.closeOverlays();

  t.unreachable("prod-catalog-add-image", "Add Image opens a native image picker");
  t.unreachable("prod-location-add-photos", "Add Photos opens a native image picker");
  t.unreachable("prod-cast-add-photo", "Add Photo opens a native image picker");
  t.unreachable("prod-moodboard-add-image-from-file", "From file opens a native image picker");
  t.unreachable("prod-storyboard-import-image", "Import Image opens a native image picker");
  t.unreachable("prod-shot-reference-image", "Add reference image opens a native image picker");

  await t.reset();
}
