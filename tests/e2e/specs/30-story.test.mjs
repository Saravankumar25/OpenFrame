// @e2e-data-root: own
//
// Story (build plan item end-to-end specs per workspace): build an act, a sequence and two Scene Cards; edit a card in its
// drawer (autosave) and rename the act, each with undo and redo; delete a card (undo, redo,
// restore from Recently Deleted); create, edit, delete and restore a character; check the Outline;
// then quit, relaunch and find everything again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openBeatBoard, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, menuItem, openProject, projectMenu, redo, relaunch, retype, undo } from "../lib/flows.mjs";

const CARD_A = "Boats fold themselves at dawn";
const CARD_A2 = "Boats fold themselves at first light";
const CARD_B = "The river carries the boats away";

let browser;
const step = stepper(() => browser, "story");
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label^="Scene Card:"]')].map((e) => e.getAttribute("aria-label").replace("Scene Card: ", "")));
const acts = () => browser.execute(() => [...document.querySelectorAll('section[aria-label^="Act "]')].map((e) => e.getAttribute("aria-label").slice(4)));
const storyTab = async (name) => {
  const t = await browser.$(`//*[@role="tablist"]//*[@role="tab"][normalize-space(.)="${name}"]`);
  await t.waitForClickable({ timeout: 10_000 });
  await t.click();
  await sleep(300);
};
const characters = () => browser.execute(() => document.querySelector("main")?.innerText ?? "");

describe("story", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("act, sequence and two Scene Cards", async () => {
    await step("build", async () => {
      await firstLaunchProject(browser, "Paper Boats");
      await openBeatBoard(browser);
      await waitForText(browser, "Your Beat Board is empty.");
      await click(browser, "Add Act");
      await (await browser.$('//input[@aria-label="Act title"]')).setValue("Act One");
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await acts()).includes("Act One"), { timeout: 10_000, timeoutMsg: "act not created" });
      await click(browser, "Add");
      await menuItem(browser, "Sequence");
      await (await browser.$('//input[@aria-label="Sequence name"]')).setValue("Departure");
      await browser.keys("Enter");
      await waitForText(browser, "Departure");
      await click(browser, "Add");
      await menuItem(browser, "Scene Card");
      await (await browser.$('//textarea[@aria-label="Short description"]')).setValue(CARD_A);
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await cards()).includes(CARD_A), { timeout: 10_000, timeoutMsg: "first card not created" });
      await click(browser, "Add");
      await menuItem(browser, "Scene Card");
      await (await browser.$('//textarea[@aria-label="Short description"]')).setValue(CARD_B);
      await browser.keys("Enter");
      // The new card renders before its description arrives (seen on the installed build), so wait for the text too.
      await browser.waitUntil(async () => (await cards()).join("|") === [CARD_A, CARD_B].join("|"), { timeout: 10_000, timeoutMsg: "second card not created" });
      assert.deepEqual(await cards(), [CARD_A, CARD_B]);
      await assertQuiet(browser, "story build");
    });
  });

  test("edit a card in its drawer, undo and redo", async () => {
    await step("edit-card", async () => {
      await (await browser.$(`//*[@aria-label="Scene Card: ${CARD_A}"]`)).doubleClick();
      const desc = await browser.$("#f-short-description");
      await desc.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, desc, CARD_A2);
      await browser.waitUntil(async () => (await browser.$('//aside[@aria-label="Scene Card"]//*[@role="status"]').getText()).trim() === "Saved", {
        timeout: 10_000,
        timeoutMsg: "the drawer did not save",
      });
      await click(browser, "Close", { within: '//aside[@aria-label="Scene Card"]' });
      await browser.waitUntil(async () => (await cards()).includes(CARD_A2), { timeout: 10_000, timeoutMsg: "card text not updated" });
      await undo(browser);
      await browser.waitUntil(async () => (await cards()).includes(CARD_A), { timeout: 10_000, timeoutMsg: "undo did not restore the card text" });
      await redo(browser);
      await browser.waitUntil(async () => (await cards()).includes(CARD_A2), { timeout: 10_000, timeoutMsg: "redo did not reapply the card text" });
      await assertQuiet(browser, "card edit");
    });
  });

  test("rename the act, undo and redo", async () => {
    await step("rename-act", async () => {
      await click(browser, "Act options for Act One");
      await menuItem(browser, "Rename");
      const input = await browser.$('//input[@aria-label="Act title"]');
      await retype(browser, input, "Act One — Launch");
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await acts()).includes("Act One — Launch"), { timeout: 10_000, timeoutMsg: "act not renamed" });
      await undo(browser);
      await browser.waitUntil(async () => (await acts()).includes("Act One"), { timeout: 10_000, timeoutMsg: "undo did not restore the act title" });
      await redo(browser);
      await browser.waitUntil(async () => (await acts()).includes("Act One — Launch"), { timeout: 10_000, timeoutMsg: "redo did not reapply the act title" });
      await assertQuiet(browser, "act rename");
    });
  });

  test("delete a card: undo, redo, restore from Recently Deleted", async () => {
    await step("delete-card", async () => {
      await (await browser.$(`//*[@aria-label="Scene Card: ${CARD_B}"]`)).click();
      await browser.keys("Delete");
      await browser.waitUntil(async () => !(await cards()).includes(CARD_B), { timeout: 10_000, timeoutMsg: "card not deleted" });
      await undo(browser);
      await browser.waitUntil(async () => (await cards()).includes(CARD_B), { timeout: 10_000, timeoutMsg: "undo did not bring the card back" });
      await redo(browser);
      await browser.waitUntil(async () => !(await cards()).includes(CARD_B), { timeout: 10_000, timeoutMsg: "redo did not delete the card again" });
      await projectMenu(browser, "Recently Deleted");
      await click(browser, `Restore ${CARD_B}`);
      await waitForText(browser, `Restored “${CARD_B}”`);
      await openBeatBoard(browser);
      await browser.waitUntil(async () => (await cards()).includes(CARD_B), { timeout: 10_000, timeoutMsg: "card not restored" });
      assert.deepEqual(await cards(), [CARD_A2, CARD_B], "the restored card returns to its place");
      await assertQuiet(browser, "card restore");
    });
  });

  test("Characters: create, edit, delete and restore", async () => {
    await step("characters", async () => {
      await storyTab("Characters");
      await click(browser, "Add Character");
      await (await browser.$("#nc-name")).setValue("Mira");
      await (await browser.$("#nc-role")).setValue("Boat maker");
      await inDialog(browser, "Create Character");
      await waitForText(browser, "Added character Mira");
      await click(browser, "Edit", { within: "//main" });
      const role = await browser.$("#ec-role");
      await role.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, role, "Paper boat maker");
      await inDialog(browser, "Save");
      await browser.waitUntil(async () => (await characters()).includes("Paper boat maker"), { timeout: 10_000, timeoutMsg: "character role not updated" });
      await click(browser, "More actions for Mira");
      await menuItem(browser, "Delete");
      await inDialog(browser, "Delete Character");
      await waitForText(browser, "Deleted character Mira");
      await projectMenu(browser, "Recently Deleted");
      await click(browser, "Restore Mira");
      await openBeatBoard(browser);
      await storyTab("Characters");
      await browser.waitUntil(async () => (await characters()).includes("Paper boat maker"), { timeout: 10_000, timeoutMsg: "character not restored" });
      await assertQuiet(browser, "characters");
    });
  });

  test("Outline shows the same structure", async () => {
    await step("outline", async () => {
      await storyTab("Outline");
      const rows = await browser.execute(() => [...document.querySelectorAll('[aria-label="Story outline"] [role="treeitem"]')].map((e) => e.innerText.replace(/\s+/g, " ").trim()));
      assert.ok(rows.some((r) => r.includes(CARD_A2)) && rows.some((r) => r.includes(CARD_B)), `outline rows: ${rows.join(" | ")}`);
      await assertQuiet(browser, "outline");
      await storyTab("Board");
    });
  });

  test("relaunch: act, cards and character persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Paper Boats");
      await openBeatBoard(browser);
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 15_000, timeoutMsg: "cards missing after relaunch" });
      assert.deepEqual(await cards(), [CARD_A2, CARD_B]);
      assert.ok((await acts()).includes("Act One — Launch"), "act title lost after relaunch");
      await storyTab("Characters");
      await browser.waitUntil(async () => (await characters()).includes("Paper boat maker"), { timeout: 10_000, timeoutMsg: "character lost after relaunch" });
      await assertQuiet(browser, "story after relaunch");
      await storyTab("Board");
      await closeProject(browser);
    });
  });
});
