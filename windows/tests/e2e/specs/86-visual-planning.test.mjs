// @e2e-data-root: own
//
// Visual planning (Windows phase W4): the moodboard canvas, the sketch dialog and drag reorder,
// driven with a real mouse. Make a moodboard, add a note and drag it to a new place on the canvas;
// draw in the storyboard sketch dialog, take one stroke back with Undo, and save the panel; add two
// empty panels and drag the last one to the front; restart and find the note where it was dropped
// and the panels in their new order.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, pageText, stepper } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, importFountain, inDialog, menuItem, openProject, productionTab, relaunch } from "../lib/flows.mjs";
import { NIGHT_BUS } from "../lib/ai.mjs";

const TITLE = "Pictures First";
const NOTE = "Cold blue light, warm windows";
let browser;
const step = stepper(() => browser, "visual-planning");
const tile = () => browser.$(`//*[@role="button"][@aria-label="Note: ${NOTE}"]`);
const tileAt = () => browser.execute((label) => {
  const el = document.querySelector(`[role="button"][aria-label="${label}"]`);
  return el ? { x: parseFloat(el.style.left), y: parseFloat(el.style.top) } : null;
}, `Note: ${NOTE}`);
/** For each storyboard panel in order: does it show a picture (the sketch) or not (an empty panel)? */
const panels = () =>
  browser.execute(() => [...document.querySelectorAll('[role="group"][aria-label="Storyboard panels"] .pnl')].map((p) => (p.querySelector("img") ? "sketch" : "empty")));

async function drag(el, dx, dy) {
  await browser
    .action("pointer", { parameters: { pointerType: "mouse" } })
    .move({ origin: el })
    .down()
    .move({ origin: el, x: 8, y: 8, duration: 120 })
    .move({ origin: el, x: dx, y: dy, duration: 400 })
    .pause(200)
    .up()
    .perform();
  await browser.releaseActions().catch(() => undefined);
}

async function stroke(area, from, to) {
  await browser
    .action("pointer", { parameters: { pointerType: "mouse" } })
    .move({ origin: area, x: from[0], y: from[1] })
    .down()
    .move({ origin: area, x: to[0], y: to[1], duration: 250 })
    .up()
    .perform();
  await browser.releaseActions().catch(() => undefined);
}

describe("visual planning", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("a note on the moodboard canvas is dragged to a new place", async () => {
    await step("canvas", async () => {
      await firstLaunchProject(browser, TITLE);
      await importFountain(browser, NIGHT_BUS);
      await productionTab(browser, "Moodboards");
      await click(browser, "New Moodboard");
      await (await browser.$("#mb-name")).setValue("Overall Look");
      await inDialog(browser, "Create Moodboard");
      await click(browser, "Add note");
      await (await browser.$("#new-note")).setValue(NOTE);
      await inDialog(browser, "Add Note");
      await (await tile()).waitForDisplayed({ timeout: 10_000 });
      const before = await tileAt();
      await drag(await tile(), 160, 90);
      await browser.waitUntil(async () => {
        const now = await tileAt();
        return now && now.x - before.x > 100 && now.y - before.y > 50;
      }, { timeout: 10_000, timeoutMsg: `the note did not move: ${JSON.stringify(before)}` });
    });
  });

  test("the sketch dialog draws, takes a stroke back, and saves a storyboard panel", async () => {
    await step("sketch", async () => {
      await productionTab(browser, "Storyboards");
      await click(browser, "Draw / Sketch");
      const area = await browser.$('//*[@aria-label="Sketch area"]');
      await area.waitForDisplayed({ timeout: 10_000 });
      await stroke(area, [-80, -40], [80, 40]);
      await stroke(area, [-80, 40], [80, -40]);
      await click(browser, "Undo", { within: '//*[@role="dialog"]' });
      await inDialog(browser, "Save Panel");
      await browser.waitUntil(async () => (await panels()).join() === "sketch", {
        timeout: 15_000,
        timeoutMsg: `the sketch panel was not saved; the page says: ${(await pageText(browser)).slice(0, 400)}`,
      });
    });
  });

  test("two empty panels are added and the last one is dragged to the front", async () => {
    await step("reorder", async () => {
      for (let i = 0; i < 2; i++) {
        await click(browser, "Add Panel", { within: '//*[@role="group"][@aria-label="Storyboard panels"]' });
        await menuItem(browser, "Empty Panel");
        await browser.waitUntil(async () => (await panels()).length === i + 2, { timeout: 10_000, timeoutMsg: "the empty panel was not added" });
      }
      assert.deepEqual(await panels(), ["sketch", "empty", "empty"]);
      const first = await browser.$('//*[@role="group"][@aria-label="Storyboard panels"]//*[contains(@class,"pnl")][1]');
      const last = await browser.$('//*[@role="group"][@aria-label="Storyboard panels"]//*[contains(@class,"pnl")][3]');
      const a = await first.getLocation();
      const b = await last.getLocation();
      await drag(last, Math.round(a.x - b.x) - 20, Math.round(a.y - b.y));
      await browser.waitUntil(async () => (await panels())[0] === "empty", { timeout: 10_000, timeoutMsg: `drag did not reorder the panels: ${(await panels()).join()}` });
    });
  });

  test("after a restart the note is where it was dropped and the panels keep their order", async () => {
    await step("restart", async () => {
      await productionTab(browser, "Moodboards");
      const where = await tileAt();
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, TITLE);
      await productionTab(browser, "Moodboards");
      await (await tile()).waitForDisplayed({ timeout: 10_000 });
      assert.deepEqual(await tileAt(), where);
      await productionTab(browser, "Storyboards");
      await browser.waitUntil(async () => (await panels()).length === 3, { timeout: 10_000, timeoutMsg: "the panels are not shown after the restart" });
      assert.deepEqual(await panels(), ["empty", "sketch", "empty"]);
    });
  });
});
