// @e2e-data-root: own
//
// Idea Vault (build plan item end-to-end specs per workspace): capture an idea, give it a title in its drawer (autosave), undo
// and redo the edit, delete it with the keyboard, undo and redo the delete, restore it from the
// Vault's Recently Deleted list, and find it again after quitting and relaunching the app.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, openProject, redo, relaunch, undo } from "../lib/flows.mjs";

const IDEA = "A lighthouse keeper collects letters that never arrived.";
const TITLE = "Lighthouse keeper";

let browser;
const step = stepper(() => browser, "vault");
const items = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label="Idea Vault items"] [role="button"][data-item-id]')].map((e) => e.getAttribute("aria-label")));
const card = (label) => browser.$(`//*[@aria-label="Idea Vault items"]//*[@role="button"][@data-item-id][@aria-label="${label}"]`);
const closeDrawer = async () => {
  const b = await browser.$('//aside//button[@aria-label="Close"]');
  await b.click();
  await b.waitForExist({ reverse: true, timeout: 10_000 });
};

describe("idea vault", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("Quick Capture adds an idea to the project's Vault", async () => {
    await step("capture", async () => {
      await firstLaunchProject(browser, "Lantern Coast");
      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Quick Capture");
      await (await browser.$('//textarea[@aria-label="Idea"]')).setValue(IDEA);
      await inDialog(browser, "Save");
      await waitForText(browser, "Saved to the Project Idea Vault");
      await browser.waitUntil(async () => (await items()).length === 1, { timeout: 10_000, timeoutMsg: "the idea is not in the Vault" });
      assert.match((await items())[0], /^A lighthouse keeper/);
      await assertQuiet(browser, "quick capture");
    });
  });

  test("edit the title in the drawer (autosave), undo and redo", async () => {
    await step("edit", async () => {
      const before = (await items())[0];
      await (await card(before)).doubleClick();
      const title = await browser.$("#vv-title");
      await title.waitForDisplayed({ timeout: 10_000 });
      await title.setValue(TITLE);
      await waitForText(browser, "Saved automatically");
      await closeDrawer();
      await browser.waitUntil(async () => (await items()).includes(TITLE), { timeout: 10_000, timeoutMsg: "the card does not show the new title" });
      await undo(browser);
      await browser.waitUntil(async () => (await items()).includes(before), { timeout: 10_000, timeoutMsg: "undo did not restore the untitled idea" });
      await redo(browser);
      await browser.waitUntil(async () => (await items()).includes(TITLE), { timeout: 10_000, timeoutMsg: "redo did not reapply the title" });
      await assertQuiet(browser, "vault edit");
    });
  });

  test("delete with the keyboard, undo, redo, then restore from Recently Deleted", async () => {
    await step("delete", async () => {
      await (await card(TITLE)).click();
      await browser.keys("Delete");
      await waitForText(browser, `Deleted “${TITLE}”. You can restore from Recently Deleted.`);
      await browser.waitUntil(async () => (await items()).length === 0, { timeout: 10_000, timeoutMsg: "the idea was not deleted" });
      await undo(browser);
      await browser.waitUntil(async () => (await items()).includes(TITLE), { timeout: 10_000, timeoutMsg: "undo did not bring the idea back" });
      await redo(browser);
      await browser.waitUntil(async () => (await items()).length === 0, { timeout: 10_000, timeoutMsg: "redo did not delete the idea again" });

      await click(browser, "Recently Deleted");
      await waitForText(browser, "Items you deleted can be restored.");
      const row = await browser.$(`//table//tr[.//*[contains(normalize-space(.),"${TITLE}")]]//button[normalize-space(.)="Restore"]`);
      await row.waitForClickable({ timeout: 10_000 });
      await row.click();
      await click(browser, "Back to Vault");
      await browser.waitUntil(async () => (await items()).includes(TITLE), { timeout: 10_000, timeoutMsg: "the idea was not restored" });
      await assertQuiet(browser, "vault restore");
    });
  });

  test("relaunch: the titled idea and its text persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Lantern Coast");
      await openWorkspace(browser, "Idea Vault");
      await browser.waitUntil(async () => (await items()).includes(TITLE), { timeout: 15_000, timeoutMsg: "the idea is gone after relaunch" });
      await (await card(TITLE)).doubleClick();
      const body = await browser.$("#vv-body");
      await body.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await body.getValue(), IDEA);
      await closeDrawer();
      await assertQuiet(browser, "vault after relaunch");
      await closeProject(browser);
    });
  });
});
