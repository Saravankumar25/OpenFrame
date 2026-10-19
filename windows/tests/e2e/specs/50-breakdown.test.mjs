// @e2e-data-root: own
//
// Breakdown (build plan item end-to-end specs per workspace): import a two-scene screenplay, choose it as the Production
// Source, accept a suggestion, add an element by hand (undo, redo), edit its notes, remove it from
// the scene and restore it from Recently Deleted, mark the scene complete; then quit, relaunch and
// find the breakdown again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, stepper, waitForText } from "../lib/app.mjs";
import { TWO_SCENES, assertQuiet, closeProject, firstLaunchProject, importFountain, inDialog, mainText, menuItem, openProject, projectMenu, redo, relaunch, undo } from "../lib/flows.mjs";

const ELEMENT = "Flickering monitor";

let browser;
const step = stepper(() => browser, "breakdown");
// The scene's own breakdown: the panel without its "Quick reuse from nearby scenes" offer, which
// names items from other scenes on purpose.
const panel = () =>
  browser.execute(() => {
    const aside = document.querySelector('aside[aria-label^="Scene "][aria-label$=" breakdown"]');
    if (!aside) return "";
    const reuse = aside.querySelector('section[aria-label="Quick reuse"]')?.innerText ?? "";
    return reuse ? aside.innerText.replace(reuse, "") : aside.innerText;
  });
const panelHas = async (text) => (await panel()).includes(text);

describe("breakdown", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("import a screenplay and choose it as the Production Source", async () => {
    await step("source", async () => {
      await firstLaunchProject(browser, "Copper Tunnel");
      await importFountain(browser, TWO_SCENES);
      await openWorkspace(browser, "Breakdown");
      await waitForText(browser, "Choose a screenplay source to begin the breakdown.");
      await click(browser, "Select Source");
      const draft = await browser.$('//*[@role="dialog"]//*[@role="radiogroup"][@aria-label="Screenplay drafts"]//*[@role="radio"]');
      await draft.waitForClickable({ timeout: 10_000 });
      await draft.click();
      await inDialog(browser, "Set as Production Source");
      await waitForText(browser, "Production Source:");
      await browser.$('//aside[@aria-label="Scene 1 breakdown"]').waitForDisplayed({ timeout: 10_000 });
      await assertQuiet(browser, "breakdown source");
    });
  });

  test("suggestions are only added when accepted", async () => {
    await step("suggest", async () => {
      await click(browser, "Suggest Elements");
      await waitForText(browser, "Nothing is added to production until you accept.");
      const accept = await browser.$('//aside[@aria-label="Scene 1 breakdown"]//button[starts-with(@aria-label,"Accept ")]');
      await accept.waitForClickable({ timeout: 10_000 });
      const name = (await accept.getAttribute("aria-label")).slice("Accept ".length);
      await accept.click();
      await waitForText(browser, `Accepted “${name}”`);
      assert.ok(await panelHas(name), `${name} is not in the scene after accepting`);
      await assertQuiet(browser, "suggestions");
    });
  });

  test("add an element by hand; undo and redo; edit its notes", async () => {
    await step("add", async () => {
      await click(browser, "Add Element");
      const cat = await browser.$("#add-cat");
      await cat.waitForDisplayed({ timeout: 10_000 });
      await cat.selectByVisibleText("Props");
      await (await browser.$("#add-name")).setValue(ELEMENT);
      await browser.waitUntil(async () => !(await (await browser.$('//*[@role="dialog"]')).getText()).includes("Looking for existing catalog items"), { timeout: 10_000 });
      await inDialog(browser, "Add to Scene 1");
      await waitForText(browser, `Added “${ELEMENT}” to Scene 1`);
      await browser.waitUntil(() => panelHas(ELEMENT), { timeout: 10_000, timeoutMsg: "element not in the scene" });
      await undo(browser);
      await browser.waitUntil(async () => !(await panelHas(ELEMENT)), { timeout: 10_000, timeoutMsg: "undo did not remove the element" });
      await redo(browser);
      await browser.waitUntil(() => panelHas(ELEMENT), { timeout: 10_000, timeoutMsg: "redo did not add the element again" });

      await click(browser, `Actions for ${ELEMENT}`);
      await menuItem(browser, "Edit notes");
      const notes = await browser.$("#el-notes");
      await notes.waitForDisplayed({ timeout: 10_000 });
      await notes.setValue("Must flicker on cue");
      await inDialog(browser, "Save");
      await browser.waitUntil(() => panelHas("Must flicker on cue"), { timeout: 10_000, timeoutMsg: "notes not shown" });
      await assertQuiet(browser, "add element");
    });
  });

  test("remove from the scene and restore from Recently Deleted", async () => {
    await step("remove", async () => {
      await click(browser, `Actions for ${ELEMENT}`);
      await menuItem(browser, "Remove from scene");
      await waitForText(browser, `Removed “${ELEMENT}” from Scene 1. The catalog item is kept.`);
      await browser.waitUntil(async () => !(await panelHas(ELEMENT)), { timeout: 10_000, timeoutMsg: "element still in the scene" });
      await projectMenu(browser, "Recently Deleted");
      await click(browser, `Restore ${ELEMENT} (Scene 1)`);
      await openWorkspace(browser, "Breakdown");
      await browser.waitUntil(() => panelHas(ELEMENT), { timeout: 10_000, timeoutMsg: "element not restored to the scene" });
      assert.ok(await panelHas("Must flicker on cue"), "the restored element lost its notes");
      await assertQuiet(browser, "restore element");
    });
  });

  test("mark the scene complete; scene 2 has its own breakdown", async () => {
    await step("complete", async () => {
      await click(browser, "Mark Complete");
      await browser.$('//aside[@aria-label="Scene 1 breakdown"]//button[normalize-space(.)="Complete"]').waitForDisplayed({ timeout: 10_000 });
      await (await browser.$('//*[@role="listbox"][@aria-label="Scenes in the Production Source"]//*[@role="option"][2]')).click();
      await browser.$('//aside[@aria-label="Scene 2 breakdown"]').waitForDisplayed({ timeout: 10_000 });
      assert.ok(!(await panelHas(ELEMENT)), "scene 2 shows scene 1's element");
      await assertQuiet(browser, "scene 2");
    });
  });

  test("relaunch: source, elements, notes and completion persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Copper Tunnel");
      await openWorkspace(browser, "Breakdown");
      await waitForText(browser, "Production Source:");
      await (await browser.$('//*[@role="listbox"][@aria-label="Scenes in the Production Source"]//*[@role="option"][1]')).click();
      await browser.$('//aside[@aria-label="Scene 1 breakdown"]').waitForDisplayed({ timeout: 10_000 });
      await browser.waitUntil(() => panelHas(ELEMENT), { timeout: 10_000, timeoutMsg: "element lost after relaunch" });
      assert.ok(await panelHas("Must flicker on cue"), "notes lost after relaunch");
      await browser.$('//aside[@aria-label="Scene 1 breakdown"]//button[normalize-space(.)="Complete"]').waitForDisplayed({ timeout: 10_000, timeoutMsg: "completion lost after relaunch" });
      assert.ok((await mainText(browser)).includes("Production Source:"));
      await assertQuiet(browser, "breakdown after relaunch");
      await closeProject(browser);
    });
  });
});
