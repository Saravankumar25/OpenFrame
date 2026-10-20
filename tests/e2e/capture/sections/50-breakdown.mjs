// WindowsHTML capture section: Breakdown: source dialogs, main view, filters, suggestions, tagging, element dialogs and menus, review views, export.
// See tests/e2e/capture/kit.mjs for the toolkit and the rules every section follows.

import { randomUUID } from "node:crypto";

const LIST = '//*[@role="listbox"][@aria-label="Scenes in the Production Source"]';
const FILTER = '//button[starts-with(@aria-label,"Filter scenes:")]';
const DIALOG = '//*[@role="dialog"]';
const sceneOpt = (n) => `${LIST}//*[@role="option"][${n}]`;
const aside = (n) => `//aside[@aria-label="Scene ${n} breakdown"]`;
const radioLabel = (tag, text) => `//label[contains(@class,"exp-radio")][.//${tag}[normalize-space(.)="${text}"]]`;

export default async function (t) {
  const b = () => t.browser;

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
  const reload = async () => {
    await b().refresh();
    await b().$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
    await t.sleep(1000);
  };
  /** Wait for an element; after an operation made outside the UI, reload once if it does not show. */
  const ensure = async (xp) => {
    for (let i = 0; i < 2; i++) {
      try {
        await (await b().$(xp)).waitForExist({ timeout: 8_000 });
        return;
      } catch {
        await reload();
      }
    }
    await (await b().$(xp)).waitForExist({ timeout: 8_000 });
  };
  const panelText = () => b().execute(() => document.querySelector('aside[aria-label$=" breakdown"]')?.innerText ?? "");
  const matchesDone = async () => {
    await t.waitText("Looking for existing catalog items", { present: false, timeout: 10_000 }).catch(() => undefined);
    await t.sleep(300);
  };
  const undoLast = async (n = 1) => {
    for (let i = 0; i < n; i++) {
      const r = await t.op("history.undo", { store: "project" });
      console.log(`    undid: ${r?.label ?? "nothing"}`);
    }
    await t.sleep(600);
  };
  const rows = async () => (await t.op("breakdown.scenes", { filter: null, category: null, catalogItemId: null })).scenes;
  const chooseFilter = async (text) => {
    await quiet(FILTER);
    await quiet(`//*[@role="menuitem"][starts-with(normalize-space(.),${t.xq(text)})]`);
  };
  /** Select two words inside one script line and let the pane see the mouse release. */
  const selectInScript = () =>
    b().execute(() => {
      const re = /[A-Za-z]{4,}\s+[A-Za-z]{3,}/;
      for (const line of document.querySelectorAll(".sp-page [data-element-id].sp-a")) {
        const node = document.createTreeWalker(line, NodeFilter.SHOW_TEXT).nextNode();
        const m = node && re.exec(node.textContent);
        if (!m) continue;
        const range = document.createRange();
        range.setStart(node, m.index);
        range.setEnd(node, m.index + m[0].length);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(range);
        line.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
        return m[0];
      }
      return null;
    });

  // ---- No source (the empty project) -------------------------------------------------------
  await t.inEmptyProject(async () => {
    await t.workspace("Breakdown");
    await t.state("bd-no-source", () => t.waitText("Choose a screenplay source to begin the breakdown."));
    await t.state("bd-select-source-empty-project", () => t.press("Select Source"));
    await t.closeOverlays();
  });

  // ---- The main view -----------------------------------------------------------------------
  await t.workspace("Breakdown");
  await t.waitText("Production Source:");
  await (await b().$(aside(1))).waitForDisplayed({ timeout: 15_000 });
  await t.state("bd-main", undefined, { home: "Breakdown" });
  await t.state("bd-change-source-dialog", () => t.press("Change source"));
  await t.closeOverlays();

  // ---- Filters (nothing picked yet, so an empty filter shows the empty state) --------------
  await t.state("bd-filter-menu", () => t.go(FILTER));
  await t.state("bd-filter-only-suggested", () => t.menuItem("Only suggested"));
  await chooseFilter("All scenes");
  await chooseFilter("Props");
  await t.state("bd-filter-by-category", () => t.sleep(500));
  await t.state("bd-filter-no-match", async () => {
    await chooseFilter("Script changed");
    await t.waitText("No scenes match this filter.");
  });
  await chooseFilter("All scenes");
  await chooseFilter("Any category");
  await t.sleep(400);

  // ---- Hide and show the scene list and the panel ------------------------------------------
  await t.state("bd-scene-list-hidden", () => t.press("Hide scenes"));
  await quiet(t.btn("Show scenes"));
  await t.state("bd-panel-hidden", () => t.press("Hide panel"));
  await t.state("bd-both-hidden", () => t.press("Hide scenes"));
  await quiet(t.btn("Show scenes"));
  await quiet(t.btn("Show panel"));
  await t.sleep(300);

  // ---- Scenes: suggestions, review dialog, edit ---------------------------------------------
  const list = await rows();
  const idx = (pred) => list.findIndex(pred) + 1;
  const suggested = idx((r) => r.suggestedCount > 0 && r.confirmedCount === 0 && !r.complete);
  const empty = idx((r) => r.suggestedCount === 0 && r.confirmedCount === 0 && !r.complete);
  console.log(`  bd scenes: ${list.length}; with suggestions ${suggested}; empty ${empty}`);

  if (suggested > 0) {
    await t.state("bd-scene-suggestions", () => t.go(sceneOpt(suggested)));
    await t.state("bd-suggestion-review-dialog", () => t.press("Review…"));
    await t.state("bd-edit-suggestion-dialog", () => t.go(`${DIALOG}//button[starts-with(@aria-label,"Edit ")]`));
    await cancel();
    const similar = await b().$(`${DIALOG}//*[normalize-space(.)="Similar items, choose one"]/ancestor::div[contains(@class,"bd-el")]//button[starts-with(@aria-label,"Accept ")]`);
    if (await similar.isExisting()) {
      await t.state("bd-possible-existing-item-dialog", () => t.go(similar));
      await cancel();
    }
    await t.closeOverlays();

    await t.state("bd-scene-marked-complete", () => t.press("Mark Complete"));
    await quiet(`${aside(list[suggested - 1].number)}//button[normalize-space(.)="Complete"]`);
    await t.sleep(400);
  } else {
    t.unreachable("bd-scene-suggestions", "no scene had a suggestion left to review when the section ran");
  }

  // Quick reuse: a scene next to the one with elements offers them again.
  for (let n = 2; n <= 5 && n <= list.length; n++) {
    await quiet(sceneOpt(n));
    await t.sleep(500);
    if (await (await b().$('//section[@aria-label="Quick reuse"]')).isExisting()) {
      await t.state("bd-scene-quick-reuse", undefined);
      break;
    }
    if (n === 5) t.unreachable("bd-scene-quick-reuse", "no scene near scene 1 offered items to reuse");
  }

  // ---- A scene with nothing yet: tag text, add, actions, notes, show in script -------------
  if (empty > 0) {
    const n = list[empty - 1].number;
    await t.state("bd-scene-nothing-yet", () => t.go(sceneOpt(empty)));
    await t.state("bd-tag-menu", async () => {
      const text = await selectInScript();
      if (!text) throw new Error("no script line to select");
      await t.el(".tagmenu");
    });
    await t.state("bd-tag-element-dialog", async () => {
      await t.press("Props", '//div[contains(@class,"tagmenu")]');
      await t.el("#add-name");
      await matchesDone();
    });
    await t.state("bd-add-element-existing-match", async () => {
      await t.type("#add-name", "Soldering iron");
      await matchesDone();
      await t.waitText("Use existing: Soldering iron", { sel: "body" });
    });
    await t.state("bd-element-tagged", async () => {
      await t.press(`Add to Scene ${n}`, DIALOG);
      await (await b().$("mark.hl")).waitForDisplayed({ timeout: 10_000 });
    });
    await t.state("bd-element-actions-menu", () => t.go(`//button[@aria-label="Actions for Soldering iron"]`));
    await t.state("bd-element-notes-dialog", async () => {
      await t.menuItem("Edit notes");
      await t.type("#el-notes", "Keep it plugged in on the bench, cable visible.");
    });
    await cancel();
    await quiet(`//button[@aria-label="Actions for Soldering iron"]`);
    await t.state("bd-show-in-script", async () => {
      await t.menuItem("Show in script");
      await (await b().$("mark.jump")).waitForDisplayed({ timeout: 10_000 });
    });
    await t.state("bd-add-element-dialog", async () => {
      await t.press("Add Element");
      await t.el("#add-name");
    });
    await cancel();
    await undoLast(1);
    await t.waitText("Nothing broken down yet", { sel: "aside" }).catch(() => undefined);
  } else {
    t.unreachable("bd-scene-nothing-yet", "every scene already had elements or suggestions when the section ran");
  }

  // ---- Export ------------------------------------------------------------------------------
  await t.state("bd-export-dialog", () => t.press("Export"));
  await t.state("bd-export-selected-scenes", () => t.go(radioLabel("span", "Selected scenes")));
  await t.state("bd-export-csv", async () => {
    await t.go(radioLabel("b", "CSV"));
  });
  await t.closeOverlays();

  // ---- A scene changed after planning, then removed from the script ------------------------
  // The scene is added to the source draft and taken away again with undo, so nothing is left behind.
  const sourceDraft = (await t.op("production.drafts", {})).activeDraftId;
  const doc0 = await t.op("screenplay.document", { draftId: sourceDraft });
  const tmp = randomUUID();
  const line = randomUUID();
  let done = 0;
  const change = async (fn) => {
    await fn();
    done++;
  };
  const edit = (ops) => change(() => t.op("screenplay.apply_edits", { draftId: sourceDraft, ops }));
  const queue = '//*[contains(@class,"queue")]';
  const footer = (text) => `//nav[@aria-label="Scenes"]//button[starts-with(normalize-space(.),"${text}")]`;
  try {
    await edit([{ op: "insertScene", id: tmp, heading: "INT. SIGNAL CABIN - NIGHT" }]);
    await t.sleep(1200);
    await reload();
    const last = doc0.scenes.length;
    await ensure(sceneOpt(last + 1));
    await t.state("bd-scene-no-text", () => t.go(sceneOpt(last + 1)));
    await edit([{ op: "insertElement", id: line, sceneId: tmp, elementType: "action", text: "Venkat checks the antenna cable by torchlight." }]);
    await change(() => t.op("breakdown.add_element", { sceneId: tmp, category: "Props", name: "Soldering iron", notes: null, span: null, catalog: { mode: "auto", catalogItemId: null } }));
    await edit([{ op: "updateElement", id: line, text: "Venkat re-cuts the antenna cable by torchlight, swearing softly." }]);
    const num = (await rows()).at(-1).number;
    await t.sleep(1200);
    await reload();
    await t.go(sceneOpt(last + 1));
    await ensure(`${aside(num)}//button[normalize-space(.)="Review"]`);
    await t.state("bd-scene-changed-banner", () => t.sleep(300));
    await t.state("bd-scene-changes-dialog", () => t.press("Review", aside(num)));
    await t.press("Review Manually", DIALOG);
    await t.state("bd-needs-review-view", () => t.go(footer("Needs Review")));
    await quiet(t.btn("Back to Breakdown"));
    await edit([{ op: "deleteScene", id: tmp }]);
    await t.sleep(1200);
    await reload();
    await ensure(footer("Removed scenes"));
    await t.state("bd-removed-scenes-view", () => t.go(footer("Removed scenes")));
    await t.state("bd-removed-scene-panel", () => t.press("Open", queue));
  } catch (e) {
    await t.closeOverlays().catch(() => undefined);
    console.log(`  bd temporary scene steps stopped: ${e instanceof Error ? e.message.split("\n")[0] : e}`);
  } finally {
    await undoLast(done);
    const back = await t.op("screenplay.document", { draftId: sourceDraft });
    const clean = back.scenes.length === doc0.scenes.length && !back.scenes.some((s) => s.id === tmp);
    console.log(`  bd temporary scene undone: ${clean ? "yes" : "NO"} (${done} steps)`);
    await reload();
  }
  await t.reset();
}
