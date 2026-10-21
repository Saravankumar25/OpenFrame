// @e2e-data-root: own
//
// Beat Board, full flow (build plan: Beat Board and Idea Vault, "Beat Board end-to-end test"):
// create an act, a sequence and cards; drag a card with the mouse (undo and redo); send a card to
// the Parking Lot and restore it; convert a beat to a Scene Card; write a long scene description
// and switch Card Display presets (the board shows the preview, the data stays the same); build
// the screenplay with one card left out because it has no heading (owner decision 2026-10-07);
// delete two cards together and restore them in reverse order; scope cards to an episode in an
// episodic project; quit, relaunch and find everything again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, createProject, firstLaunchProject, inDialog, menuItem, openProject, projectMenu, redo, relaunch, undo } from "../lib/flows.mjs";

const A = "Rain finds the letter box";
const B = "Meera reads the first letter";
const C = "The postman never came back";
const BEAT = "A second letter arrives";
const LONG = "The street is flooded to the knees. Meera wades to the box with a torch in her teeth.";

let browser;
const step = stepper(() => browser, "story-flow");
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label^="Scene Card:"]')].map((e) => e.getAttribute("aria-label").replace("Scene Card: ", "")));
const cardsIn = (region) =>
  browser.execute(
    (r) => [...(document.querySelector(`section[aria-label="${r}"]`)?.querySelectorAll('[aria-label^="Scene Card:"]') ?? [])].map((e) => e.getAttribute("aria-label").replace("Scene Card: ", "")),
    region,
  );
const card = (text) => browser.$(`//*[@aria-label="Scene Card: ${text}"]`);

async function addCard(text) {
  await click(browser, "Add");
  await menuItem(browser, "Scene Card");
  const t = await browser.$('//textarea[@aria-label="Short description"]');
  await t.waitForDisplayed({ timeout: 10_000 });
  await t.setValue(text);
  await browser.keys("Enter");
  await browser.waitUntil(async () => (await cards()).includes(text), { timeout: 10_000, timeoutMsg: `card "${text}" not created` });
}

/** A real mouse drag: press on `src`, move past the 6 px activation distance, drop on the top part of `dst`. */
async function dragAbove(src, dst) {
  const { height } = await dst.getSize();
  await browser
    .action("pointer", { parameters: { pointerType: "mouse" } })
    .move({ origin: src })
    .down()
    .move({ origin: src, x: 0, y: -12, duration: 150 })
    .move({ origin: dst, x: 0, y: -Math.max(2, Math.floor(height / 4)), duration: 400 })
    .pause(250)
    .up()
    .perform();
  await browser.releaseActions().catch(() => undefined);
}

async function contextMenu(el, item) {
  await el.click({ button: "right" });
  await menuItem(browser, item);
}

describe("beat board flow", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("create an act, a sequence and three cards", async () => {
    await step("create", async () => {
      await firstLaunchProject(browser, "Monsoon Letters");
      await openWorkspace(browser, "Story");
      await waitForText(browser, "Your Beat Board is empty.");
      await click(browser, "Add Act");
      await (await browser.$('//input[@aria-label="Act title"]')).setValue("Act One");
      await browser.keys("Enter");
      await click(browser, "Add");
      await menuItem(browser, "Sequence");
      await (await browser.$('//input[@aria-label="Sequence name"]')).setValue("The Letters");
      await browser.keys("Enter");
      await waitForText(browser, "The Letters");
      for (const t of [A, B, C]) await addCard(t);
      assert.deepEqual(await cards(), [A, B, C]);
      await assertQuiet(browser, "story create");
    });
  });

  test("drag a card with the mouse, undo and redo", async () => {
    await step("drag", async () => {
      await dragAbove(await card(C), await card(A));
      await browser.waitUntil(async () => (await cards())[0] === C, { timeout: 10_000, timeoutMsg: `drag did not move the card: ${(await cards()).join(" | ")}` });
      assert.deepEqual(await cards(), [C, A, B]);
      await undo(browser);
      await browser.waitUntil(async () => (await cards()).join() === [A, B, C].join(), { timeout: 10_000, timeoutMsg: "undo did not restore the order" });
      await redo(browser);
      await browser.waitUntil(async () => (await cards()).join() === [C, A, B].join(), { timeout: 10_000, timeoutMsg: "redo did not reapply the drag" });
      await assertQuiet(browser, "drag");
    });
  });

  test("Parking Lot: park a card and restore it to its place", async () => {
    await step("parking", async () => {
      const toggle = await browser.$('//button[@aria-pressed][normalize-space(.)="Parking Lot"]');
      await toggle.waitForDisplayed({ timeout: 10_000 });
      if ((await toggle.getAttribute("aria-pressed")) !== "true") await toggle.click();
      await contextMenu(await card(A), "Send to Parking Lot");
      await browser.waitUntil(async () => (await cardsIn("Parking Lot")).includes(A), { timeout: 10_000, timeoutMsg: "card not parked" });
      assert.deepEqual(await cardsIn("Act Act One"), [C, B]);
      await contextMenu(await card(A), "Restore to Story");
      await browser.waitUntil(async () => (await cardsIn("Act Act One")).join() === [C, A, B].join(), {
        timeout: 10_000,
        timeoutMsg: `card not restored to its place: ${(await cardsIn("Act Act One")).join(" | ")}`,
      });
      await assertQuiet(browser, "parking");
    });
  });

  test("convert a beat to a Scene Card", async () => {
    await step("convert", async () => {
      await (await card(B)).click();
      await click(browser, "Add");
      await menuItem(browser, "Beat");
      const t = await browser.$('//textarea[@aria-label="Beat text"]');
      await t.waitForDisplayed({ timeout: 10_000 });
      await t.setValue(BEAT);
      await browser.keys("Enter");
      const beat = await browser.$(`//*[@aria-label="Beat: ${BEAT}"]`);
      await beat.waitForDisplayed({ timeout: 10_000 });
      await contextMenu(beat, "Convert to Scene");
      await browser.waitUntil(async () => (await cards()).includes(BEAT), { timeout: 10_000, timeoutMsg: "no Scene Card from the beat" });
      await browser.$(`//*[@aria-label="Beat: ${BEAT} (converted)"]`).waitForDisplayed({ timeout: 10_000 });
      await undo(browser);
      await browser.waitUntil(async () => !(await cards()).includes(BEAT), { timeout: 10_000, timeoutMsg: "undo did not remove the converted card" });
      await redo(browser);
      await browser.waitUntil(async () => (await cards()).includes(BEAT), { timeout: 10_000, timeoutMsg: "redo did not convert again" });
      await assertQuiet(browser, "convert");
    });
  });

  test("long scene description and Card Display presets", async () => {
    await step("card-display", async () => {
      await (await card(A)).doubleClick();
      const long = await browser.$("#f-long-description");
      await long.waitForDisplayed({ timeout: 10_000 });
      await long.setValue(LONG);
      await browser.waitUntil(async () => (await browser.$('//aside[@aria-label="Scene Card"]//*[@role="status"]').getText()).trim() === "Saved", {
        timeout: 10_000,
        timeoutMsg: "the long description did not save",
      });
      await click(browser, "Close", { within: '//aside[@aria-label="Scene Card"]' });
      // Minimal (default): no preview on the card.
      assert.equal(await browser.$(".sc-long").isExisting(), false);
      await click(browser, "More Beat Board actions");
      await menuItem(browser, "Card Display Settings");
      await (await browser.$('//*[@role="dialog"]//label[.//b[normalize-space(.)="Story"]]//input')).click();
      await browser.waitUntil(async () => (await browser.$(".sc-long").isExisting()) && (await browser.$(".sc-long").getText()).startsWith("The street is flooded"), {
        timeout: 10_000,
        timeoutMsg: "the Story preset does not show the long description preview",
      });
      // A box ticked by hand makes it Custom.
      const shots = await browser.$('//*[@role="dialog"]//label[normalize-space(.)="Shot/Storyboard Status"]//input');
      await shots.click();
      await browser.waitUntil(async () => (await browser.$('//*[@role="dialog"]//label[.//b[normalize-space(.)="Custom"]]//input').isSelected()), {
        timeout: 10_000,
        timeoutMsg: "ticking a box did not switch to Custom",
      });
      await inDialog(browser, "Done");
      // Changing the layout changed no story data: same cards in the same order.
      assert.deepEqual(await cardsIn("Act Act One"), [C, A, B, BEAT]);
      await assertQuiet(browser, "card display");
    });
  });

  test("Build Screenplay: a card without a heading is left out, never invented", async () => {
    await step("build", async () => {
      await click(browser, "Build Screenplay");
      await waitForText(browser, "Build Screenplay from Beat Board");
      await waitForText(browser, "no valid scene heading");
      await waitForText(browser, "OpenFrame never makes up a heading.");
      // Rows are in board order: C, A, B, BEAT (the beat was added after B and converted in place).
      await (await browser.$('//input[@aria-label="Heading for scene 1"]')).setValue("EXT. POST OFFICE - NIGHT");
      await (await browser.$('//input[@aria-label="Heading for scene 2"]')).setValue("EXT. FLOODED STREET - NIGHT");
      await (await browser.$('//input[@aria-label="Heading for scene 3"]')).setValue("INT. MEERA'S ROOM - NIGHT");
      const build = await browser.$('//*[@role="dialog"]//button[normalize-space(.)="Build Screenplay"]');
      assert.equal(await build.isEnabled(), false, "Build must wait until every included card has a heading");
      await click(browser, "Leave out scene 4");
      await browser.waitUntil(async () => build.isEnabled(), { timeout: 10_000, timeoutMsg: "Build stayed disabled after leaving the card out" });
      await waitForText(browser, "3 of 4 cards will be built");
      await build.click();
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
      await waitForText(browser, "Draft 1 from Beat Board");
      // The toast names the draft at once; the editor shows it when it has loaded.
      const headingsNow = () =>
        browser.execute(() => [...document.querySelectorAll('.spx-doc .sp-el[data-type="scene_heading"]')].map((e) => e.textContent));
      await browser.waitUntil(async () => (await headingsNow()).length > 0, { timeout: 15_000, timeoutMsg: "the built draft did not open in the editor" });
      const headings = await headingsNow();
      assert.deepEqual(headings, ["EXT. POST OFFICE - NIGHT", "EXT. FLOODED STREET - NIGHT", "INT. MEERA'S ROOM - NIGHT"], "exactly the three headed cards, in board order");
      await assertQuiet(browser, "build");
      await openWorkspace(browser, "Story");
    });
  });

  test("delete two cards together and restore them in reverse order", async () => {
    await step("batch-restore", async () => {
      // Click A, Ctrl+click B: both selected.
      await (await card(A)).click();
      const CTRL = ""; // WebDriver key code for Control
      await browser.action("key").down(CTRL).perform(true);
      await (await card(B)).click();
      await browser.action("key").up(CTRL).perform();
      await browser.waitUntil(async () => (await browser.$$('.sc.sel').length) === 2, { timeout: 10_000, timeoutMsg: "two cards are not selected" });
      await browser.keys("Delete");
      // The cards were built into the screenplay in the step before, so the delete asks first and
      // says that their screenplay scenes stay.
      await waitForText(browser, "Delete 2 items?");
      await waitForText(browser, "Screenplay scenes are never deleted from the Beat Board.");
      await inDialog(browser, "Delete");
      await browser.waitUntil(async () => !(await cards()).includes(A) && !(await cards()).includes(B), { timeout: 10_000, timeoutMsg: "cards not deleted" });
      await projectMenu(browser, "Recently Deleted");
      await click(browser, `Restore ${B}`);
      await waitForText(browser, `Restored “${B}”`);
      await click(browser, `Restore ${A}`);
      await waitForText(browser, `Restored “${A}”`);
      await openWorkspace(browser, "Story");
      await browser.waitUntil(async () => (await cardsIn("Act Act One")).join() === [C, A, B, BEAT].join(), {
        timeout: 10_000,
        timeoutMsg: `restored cards are out of order: ${(await cardsIn("Act Act One")).join(" | ")}`,
      });
      await assertQuiet(browser, "batch restore");
    });
  });

  test("relaunch: order, long description and Card Display survive", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Monsoon Letters");
      await openWorkspace(browser, "Story");
      await browser.waitUntil(async () => (await cardsIn("Act Act One")).join() === [C, A, B, BEAT].join(), { timeout: 15_000, timeoutMsg: "order lost after relaunch" });
      assert.ok((await browser.$(".sc-long").getText()).startsWith("The street is flooded"), "Card Display setting or long description lost after relaunch");
      await (await card(A)).doubleClick();
      const long = await browser.$("#f-long-description");
      await long.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await long.getValue(), LONG);
      await click(browser, "Close", { within: '//aside[@aria-label="Scene Card"]' });
      await assertQuiet(browser, "story after relaunch");
      await closeProject(browser);
    });
  });

  // Episodic and Series projects can no longer be created (owner change list item 9, 2026-10-19);
  // the episode views stay for projects made earlier and are covered by the Rust story tests.
});
