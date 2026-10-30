// WindowsHTML capture section: Story: Story page, Beat Board (board, menus, drawers, dialogs, zoom, filter, parking lot, build and apply-order dialogs, export), Outline, Characters, relationship map, Timeline.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

const DIALOG = '//*[@role="dialog"]';
const DRAWER = '//aside[contains(@class,"drawer")]';
const CTRL = "";

export default async function (t) {
  const b = t.browser;
  const X = t.xq;

  // ---------------------------------------------------------------- helpers

  /** Move the pointer off the content so no hover style is captured. */
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

  const card = (text) => `//*[@data-story-key][starts-with(@aria-label,"Scene Card: ")][contains(@aria-label,${X(text)})]`;
  const beat = (text) => `//*[@data-story-key][starts-with(@aria-label,"Beat: ")][contains(@aria-label,${X(text)})]`;
  const seq = (title) => `//*[@data-story-key][starts-with(@aria-label,"Sequence ")][contains(@aria-label,${X(title)})]`;
  const row = (text) => `//*[@role="treeitem"][@data-story-key][contains(.,${X(text)})]`;
  const press = (name) => t.press(name, DIALOG);
  const cancel = () => t.press("Cancel", DIALOG);
  const radio = (label) => t.go(`${DIALOG}//label[.//*[normalize-space(.)=${X(label)}]]`);
  const actOptions = (title) => `//button[@aria-label=${X(`Act options for ${title}`)}]`;
  const boardToLeft = () => b.execute(() => document.querySelectorAll(".story-canvas").forEach((e) => (e.scrollLeft = 0)));
  const closeDrawer = async () => {
    const x = await b.$(`${DRAWER}//button[@aria-label="Close"]`);
    if (await x.isExisting()) {
      await x.click();
      await t.sleep(300);
    }
  };
  const clearSelection = async () => {
    const x = await b.$('//button[@aria-label="Clear selection"]');
    if (await x.isExisting()) await x.click();
    else await b.execute(() => document.querySelector(".story-canvas")?.click());
    await t.sleep(250);
  };
  /** Open a card, beat or sequence in its drawer through the context menu (a click would select it). */
  const openDrawerOf = async (sel) => {
    await t.rightClick(sel);
    await t.menuItem("Open");
    await t.el(DRAWER);
    await t.sleep(500);
  };
  /** Ctrl+click: add `sel` to the selection (a click event carrying the Ctrl modifier). */
  const ctrlClick = async (sel) => {
    const el = await t.el(sel);
    await b.execute((e) => e.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, ctrlKey: true })), el);
    await t.sleep(250);
  };
  /** Run one group of states; a stop inside it is logged and the next group still runs. */
  const TAB = { page: "Story", outline: "Outline", characters: "Characters", timeline: "Timeline" };
  const block = async (name, fn) => {
    try {
      if (name !== "empty") {
        await t.closeOverlays().catch(() => undefined);
        await closeDrawer().catch(() => undefined);
        await t.storyTab(TAB[name] ?? "Board");
        if (!TAB[name]) await clearSelection();
      }
      await fn();
    } catch (e) {
      console.log(`  story group ${name} stopped: ${String(e.message).split("\n")[0].slice(0, 240)}`);
      await t.closeOverlays().catch(() => undefined);
      await closeDrawer().catch(() => undefined);
    }
  };
  const applyOrder = async () => {
    await t.go('//button[@aria-label="More Beat Board actions"]');
    await t.menuItem("Apply Board Order");
    await t.el(DIALOG);
    await t.waitText("Apply this order");
    await t.sleep(600);
  };
  const charRow = (name) => `//section[@aria-label="Characters"]//button[contains(normalize-space(.),${X(name)})]`;
  const zoom100 = async () => {
    for (let i = 0; i < 6; i++) {
      const n = parseInt(await t.text('[role="group"][aria-label="Zoom"]'), 10);
      if (n === 100 || Number.isNaN(n)) return;
      await t.press(n > 100 ? "Zoom out" : "Zoom in");
    }
  };
  const board = () => t.op("story.board", { episodeId: null });
  const allCards = (bd) => {
    const out = [];
    const walk = (items) => items.forEach((i) => (i.kind === "sequence" ? walk(i.items) : i.kind === "card" ? out.push(i) : 0));
    bd.acts.forEach((a) => walk(a.items));
    return out;
  };

  await t.workspace("Story");

  await block("page", async () => {
    // -------------------------------------------------------------- Story page

    await shot("story-page-filled", () => t.storyTab("Story"));
    await shot("story-page-highlight-colours", () => t.go('//button[@aria-label="Highlight colour"]'));
    await t.closeOverlays();
    await shot("story-page-underline-colours", () => t.go('//button[@aria-label="Underline colour"]'));
    await t.closeOverlays();
    // Dark page is a view preference: switch it back off for the other states.
    await shot("story-page-dark", () => t.go('//button[@aria-label="Dark page"]'), { wait: 600 });
    await t.go('//button[@aria-label="Dark page"]');
    await shot("story-page-export-dialog", () => t.press("Export"));
    await t.closeOverlays();

  });
  await block("board", async () => {
    // -------------------------------------------------------------- Beat Board

    await shot("story-board-filled", () => t.storyTab("Board"), { home: "Story" });
    // Horizontal layout is a view preference: back to Vertical for the other states.
    const layout = (name) => `//*[@aria-label="Beat Board layout"]//*[@role="radio"][normalize-space(.)="${name}"]`;
    await shot("story-board-horizontal", () => t.go(layout("Horizontal")), { wait: 800 });
    await t.go(layout("Vertical"));
    await t.sleep(500);
    await shot("story-board-add-menu", () => t.press("Add"));
    await t.closeOverlays();
    await shot("story-board-more-actions-menu", () => t.go('//button[@aria-label="More Beat Board actions"]'));
    await t.closeOverlays();

  });
  await block("zoom", async () => {
    for (let i = 0; i < 3; i++) await t.press("Zoom out");
    await shot("story-board-zoom-50", () => t.sleep(300));
    for (let i = 0; i < 5; i++) await t.press("Zoom in");
    await shot("story-board-zoom-150", async () => {
      await boardToLeft();
      await t.sleep(300);
    });
    for (let i = 0; i < 2; i++) await t.press("Zoom out");
    await boardToLeft();

  });
  await block("zoom-reset", zoom100);
  await block("filter", async () => {
    await shot("story-board-filter-active", async () => {
      await t.type('//input[@aria-label="Search / filter cards"]', "bus");
      await t.sleep(500);
    });
    await t.press("Clear filter");

  });
  await block("parking", async () => {
    await shot("story-board-parking-lot-closed", () => t.press("Parking Lot"));
    await t.press("Parking Lot");
    await boardToLeft();

  });
  await block("acts", async () => {
    await shot("story-board-act-menu", () => t.go(actOptions("Act 1: The Return")));
    await t.closeOverlays();
    await shot(
      "story-board-act-collapsed",
      async () => {
        await t.press("Collapse Act 3: The Signal");
        await t.sleep(300);
      },
    );
    await t.press("Expand Act 3: The Signal");
    await shot(
      "story-board-act-rename-inline",
      async () => {
        const title = await t.el('//section[@aria-label="Act Act 1: The Return"]//span[contains(@class,"truncate")]');
        await title.doubleClick();
        await t.el('//input[@aria-label="Act title"]');
      },
      { blur: false },
    );
    await b.keys("Escape");
    await t.sleep(250);

    await shot("story-board-new-act-input", () => t.press("Add Act"), { blur: false });
    await b.keys("Escape");
    await boardToLeft();
    await t.sleep(250);

  });
  await block("contextmenus", async () => {
    await shot("story-board-sequence-context-menu", () => t.rightClick(seq("Arrival in Rayapuram")));
    await t.closeOverlays();
    await shot("story-board-card-context-menu", () => t.rightClick(card("Lakshmi steps off")));
    await t.closeOverlays();
    await shot("story-board-beat-context-menu", () => t.rightClick(beat("She decides to stay")));
    await t.closeOverlays();
    await shot("story-board-parked-card-context-menu", () => t.rightClick(card("A dream about")));
    await t.closeOverlays();

  });
  await block("selection", async () => {
    // Selection.
    await shot("story-board-card-selected", () => t.go(card("Lakshmi steps off")));
    await shot("story-board-multiselect-bar", () => ctrlClick(card("The rusted ladder")));
    await shot("story-multiselect-move-dialog", () => t.press("Move…"));
    await cancel();
    await clearSelection();
    // With two or more cards selected the bar covers Export, so the selected-items scope is shown for one card.
    await t.go(card("Lakshmi steps off"));
    await shot("story-export-dialog-selected", () => t.press("Export"));
    await t.closeOverlays();
    await clearSelection();

  });
  await block("drawers", async () => {
    // Drawers.
    await shot("story-card-drawer", () => openDrawerOf(card("Lakshmi steps off")));
    await closeDrawer();
    await shot("story-card-drawer-parked", () => openDrawerOf(card("A dream about")));
    await closeDrawer();
    await shot("story-beat-drawer", () => openDrawerOf(beat("She decides to stay")));
    await closeDrawer();
    await shot("story-sequence-drawer", () => openDrawerOf(seq("Climbing the tower")));
    await closeDrawer();
    t.unreachable("story-card-drawer-linked", "the banner Used to create screenplay Scene n appears only after a card is built into a screenplay, which would add a screenplay the other sections do not expect");

  });
  await block("decisions", async () => {
    // Decision dialogs (every one is cancelled).
    await shot("story-delete-act-dialog", async () => {
      await t.go(actOptions("Act 2: The Tower"));
      await t.menuItem("Delete");
      await t.el(DIALOG);
    });
    await shot("story-delete-act-dialog-delete-all", () => radio("Delete the Act and everything inside it"));
    await cancel();
    await shot("story-delete-sequence-dialog", async () => {
      await t.rightClick(seq("Climbing the tower"));
      await t.menuItem("Delete");
      await t.el(DIALOG);
    });
    await shot("story-delete-sequence-dialog-delete-all", () => radio("Delete the Sequence and everything inside it"));
    await cancel();
    await shot("story-move-dialog", async () => {
      await t.rightClick(card("The district office"));
      await t.menuItem("Move…");
      await t.el(DIALOG);
    });
    await cancel();
    await shot("story-act-note-dialog", async () => {
      await t.go(actOptions("Act 1: The Return"));
      await t.menuItem("Edit note");
      await t.el("#act-note");
    });
    await cancel();
    t.unreachable("story-delete-linked-card-dialog", "needs a card that was built into a screenplay scene, which would add a screenplay the other sections do not expect");

  });
  await block("carddisplay", async () => {
    // Card Display Settings, and the board after a preset change.
    await shot("story-card-display-dialog", async () => {
      await t.go('//button[@aria-label="More Beat Board actions"]');
      await t.menuItem("Card Display Settings");
      await t.el(DIALOG);
    });
    const presets = await b.execute(() =>
      [...document.querySelectorAll('input[name="card-preset"]')].map((i) => ({ checked: i.checked, label: i.closest("label").querySelector("b").textContent })),
    );
    const original = presets.find((p) => p.checked)?.label;
    const other = [...presets].reverse().find((p) => !p.checked && p.label !== "Custom");
    if (other && original) {
      await shot("story-card-display-dialog-changed", () => radio(other.label));
      await press("Done");
      await shot("story-board-rich-cards", async () => {
        await boardToLeft();
        await t.sleep(500);
      });
      await t.go('//button[@aria-label="More Beat Board actions"]');
      await t.menuItem("Card Display Settings");
      await t.el(DIALOG);
      await radio(original);
      await press("Done");
      await t.sleep(400);
    } else {
      await press("Done");
    }

  });
  await block("build", async () => {
    // Build Screenplay (nothing is built; every dialog is cancelled).
    await shot("story-build-dialog", () => t.press("Build Screenplay"));
    await shot("story-build-dialog-new-screenplay", () => radio("Build a new screenplay document"));
    const dialogText = await t.text('[role="dialog"]');
    const existing = /“(.+?)” already has a screenplay/.exec(dialogText)?.[1];
    if (existing) {
      await shot("story-build-dialog-title-clash", async () => {
        await t.type("#build-title", existing);
        await t.waitText("Another screenplay already has this title");
      });
    }
    await cancel();
    await shot("story-build-dialog-preselected", async () => {
      await t.rightClick(card("The district office"));
      await t.menuItem("Convert to Screenplay Scene");
      await t.el(DIALOG);
    });
    await cancel();
    // A card without a scene heading: its heading is cleared for this one screen and put back.
    {
      const target = allCards(await board()).find((c) => c.shortDescription.startsWith("Grandfather at the workbench"));
      if (target?.sceneHeading) {
        try {
          await t.op("story.update_card", { id: target.id, sceneHeading: "" });
          await t.sleep(600);
          await shot("story-build-dialog-missing-heading", async () => {
            await t.press("Build Screenplay");
            await t.waitText("no valid scene heading");
          });
          await cancel();
        } finally {
          await t.op("story.update_card", { id: target.id, sceneHeading: target.sceneHeading });
          await t.sleep(400);
        }
      } else {
        t.unreachable("story-build-dialog-missing-heading", "no seeded card with a heading to clear");
      }
    }

  });
  await block("applyorder", async () => {
    // Apply Board Order to Screenplay.
    await shot("story-apply-order-dialog", applyOrder);
    await cancel();
    {
      const ov = await t.op("screenplay.overview", {});
      const draft = ov.drafts.find((d) => d.isCurrent) ?? ov.drafts[0];
      let locked = false;
      try {
        await t.op("screenplay.lock_draft", { draftId: draft.id });
        locked = true;
        await t.sleep(600);
        await shot("story-apply-order-dialog-locked", async () => {
          await applyOrder();
          await t.waitText("is locked as the shooting draft");
        });
        await cancel();
      } catch (e) {
        t.unreachable("story-apply-order-dialog-locked", `the draft could not be locked for this screen: ${String(e.message).split("\n")[0]}`);
      } finally {
        if (locked) await t.op("screenplay.unlock_draft", { draftId: draft.id }).catch(() => undefined);
      }
    }

    await shot("story-export-dialog", () => t.press("Export"));
    await t.closeOverlays();

  });
  await block("outline", async () => {
    // ----------------------------------------------------------------- Outline

    await shot("story-outline-view", () => t.storyTab("Outline"));
    await shot("story-outline-act-menu", () => t.go(actOptions("Act 2: The Tower")));
    await t.closeOverlays();
    await shot("story-outline-card-context-menu", () => t.rightClick(row("Lakshmi steps off")));
    await t.closeOverlays();
    await shot("story-outline-new-act-input", () => t.press("Add Act"), { blur: false });
    await b.keys("Escape");
    await t.sleep(250);
    await shot("story-outline-card-drawer", () => openDrawerOf(row("The first broadcast")));
    await closeDrawer();
    await t.storyTab("Board");

  });
  await block("characters", async () => {
    // -------------------------------------------------------------- Characters

    await shot("story-characters-list", () => t.storyTab("Characters"));
    await shot("story-character-detail-venkat", () => t.go(charRow("VENKAT")));
    await t.go(charRow("LAKSHMI"));
    await shot("story-character-more-menu", () => t.go('//button[@aria-label="More actions for LAKSHMI"]'));
    await t.closeOverlays();
    await shot("story-new-character-dialog", () => t.press("Add Character"));
    await shot("story-new-character-duplicate-warning", async () => {
      await t.type("#nc-name", "Lakshmi");
      await t.waitText("Possible match");
    });
    await cancel();
    await shot("story-edit-character-dialog", () => t.press("Edit", '//section[@aria-label="Character details"]'));
    await cancel();
    await shot("story-add-relationship-dialog", () => t.press("Add Relationship", '//section[@aria-label="Character details"]'));
    await shot("story-add-relationship-dialog-filled", async () => {
      await t.press("friend of", DIALOG);
      await t.type("#rel-note", "Met on the coast road");
    });
    await cancel();
    await shot("story-relationship-map", () => t.press("Relationship Map", '//section[@aria-label="Character details"]'), { wait: 900 });
    await shot("story-relationship-map-whole-film", () => t.press("Whole film map"), { wait: 900 });
    await t.press("Back to Characters");

    for (const name of ["LAKSHMI", "PADMA"]) {
      await t.go(charRow(name));
      await t.go(`//button[@aria-label="More actions for ${name}"]`);
      await t.menuItem("Delete");
      await t.el(DIALOG);
      const inUse = (await t.text('[role="dialog"]')).includes("is in use");
      const state = `story-delete-character-dialog-${name.toLowerCase()}${inUse ? "-in-use" : ""}`;
      if (!(await shot(state))) await t.closeOverlays();
      await t.closeOverlays();
    }

    // Archive PADMA, show archived characters, bring her back.
    await t.go(charRow("PADMA"));
    await shot(
      "story-character-archived-toast",
      async () => {
        await t.go('//button[@aria-label="More actions for PADMA"]');
        await t.menuItem("Archive");
        await t.waitText("Archived PADMA");
      },
      { keepToasts: true },
    );
    await shot("story-characters-show-archived", () => t.go('//label[contains(normalize-space(.),"Show archived characters")]'));
    await t.go(charRow("PADMA"));
    await t.go('//button[@aria-label="More actions for PADMA"]');
    await t.menuItem("Unarchive");
    await t.sleep(500);
    await t.go('//label[contains(normalize-space(.),"Show archived characters")]');
    await t.sleep(300);

  });
  await block("timeline", async () => {
    // ---------------------------------------------------------------- Timeline

    await shot("story-timeline-filled", () => t.storyTab("Timeline"));
    const tl = await t.op("story.timeline", { episodeId: null, screenplayId: null });
    const scenes = tl.groups.flatMap((g) => g.scenes);
    const pick = (n) => `//label[normalize-space(.)=${X(`Select scene ${n}`)}]`;
    try {
      for (const n of [1, 2, 3]) await t.go(pick(n));
      await shot("story-timeline-scenes-selected", () => t.sleep(200));
      await shot("story-assign-story-day-dialog", () => t.press("Assign Story Day (3)"));
      await shot("story-assign-story-day-dialog-filled", () => t.type("#story-day", "1"));
      await shot(
        "story-timeline-day-assigned",
        async () => {
          await press("Assign");
          await t.waitText("Assigned Story Day 1");
          await t.sleep(400);
        },
        { keepToasts: true },
      );
      await shot("story-timeline-time-note", async () => {
        await t.type('//input[@aria-label="Time of day note for scene 1"]', "Night, after the last bus");
        await b.keys("Tab");
        await t.sleep(500);
      });
      if (scenes.length >= 6) {
        await t.op("story.assign_story_day", { sceneIds: scenes.slice(3, 5).map((s) => s.sceneId), storyDay: "2" });
        await t.op("story.assign_story_day", { sceneIds: [scenes[5].sceneId], storyDay: "Flashback" });
        await t.sleep(700);
        await shot("story-timeline-grouped-days", () => t.sleep(200));
      }
    } finally {
      await t.op("story.assign_story_day", { sceneIds: scenes.map((s) => s.sceneId), storyDay: null }).catch(() => undefined);
      for (const s of scenes.slice(0, 3)) await t.op("story.set_time_note", { sceneId: s.sceneId, timeNote: null }).catch(() => undefined);
      await t.sleep(500);
    }

  });
  await block("empty", async () => {
    // ------------------------------------------------------ empty project views

    await t.reset();
    await t.inEmptyProject(async () => {
      await t.storyTab("Story");
      await shot("story-page-empty", () => t.waitText("0 words"));
      await shot("story-board-empty", () => t.storyTab("Board"));
      await shot("story-board-empty-add-menu", () => t.press("Add"));
      await t.closeOverlays();
      await shot("story-board-empty-new-act-input", () => t.press("Add Act"), { blur: false });
      await b.keys("Escape");
      await t.sleep(250);
      await shot("story-build-dialog-no-cards", () => t.press("Build Screenplay"));
      await t.press("Close", DIALOG);
      await shot("story-apply-order-dialog-no-screenplay", applyOrder);
      await cancel();
      await shot("story-characters-empty", () => t.storyTab("Characters"));
      await shot("story-timeline-empty", () => t.storyTab("Timeline"));
      await t.workspace("Home");
    });

  });
  await t.reset();
}
