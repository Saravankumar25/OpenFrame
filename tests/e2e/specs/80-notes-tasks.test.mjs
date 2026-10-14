// @e2e-data-root: own
//
// Notes & Tasks (build plan item end-to-end specs per workspace): write a project note (autosave), undo and redo the edit,
// delete it and restore it from Recently Deleted; add a task, complete it, undo and redo the
// completion; then quit, relaunch and find both again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, openProject, projectMenu, redo, relaunch, undo } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "notes");
const notes = () => browser.execute(() => [...document.querySelectorAll('section[aria-label="Project Notes"] li button')].map((b) => b.innerText.trim()));
const taskBox = (title, done) => browser.$(`//section[@aria-label="Tasks"]//input[@aria-label="${done ? `Reopen “${title}”` : `Mark “${title}” done`}"]`);
const closeDrawer = async () => {
  const b = await browser.$('//aside//button[@aria-label="Close"]');
  await b.click();
  await b.waitForExist({ reverse: true, timeout: 10_000 });
};

describe("notes and tasks", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("a new note autosaves its title and text; undo and redo the edit", async () => {
    await step("note", async () => {
      await firstLaunchProject(browser, "Tide Tables");
      await projectMenu(browser, "Notes & Tasks");
      await waitForText(browser, "No notes yet.");
      await click(browser, "New Note");
      const title = await browser.$("#ne-title");
      await title.waitForDisplayed({ timeout: 10_000 });
      await title.setValue("Location scout");
      await (await browser.$("#ne-body")).setValue("Check the old pier at low tide.");
      await closeDrawer();
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith("Location scout")), { timeout: 10_000, timeoutMsg: "the note is not listed" });
      await undo(browser);
      await browser.waitUntil(async () => !(await notes()).some((n) => n.startsWith("Location scout")), { timeout: 10_000, timeoutMsg: "undo did not revert the note edit" });
      await redo(browser);
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith("Location scout")), { timeout: 10_000, timeoutMsg: "redo did not reapply the note edit" });
      await assertQuiet(browser, "note");
    });
  });

  test("delete the note and restore it from Recently Deleted", async () => {
    await step("note-trash", async () => {
      await (await browser.$('//section[@aria-label="Project Notes"]//li//button[starts-with(normalize-space(.),"Location scout")]')).click();
      await click(browser, "Delete", { within: "//aside" });
      await waitForText(browser, "Moved note “Location scout” to Recently Deleted");
      await browser.waitUntil(async () => (await notes()).length === 0, { timeout: 10_000, timeoutMsg: "the note was not deleted" });
      await projectMenu(browser, "Recently Deleted");
      await click(browser, "Restore Location scout");
      await waitForText(browser, "Restored “Location scout”");
      await projectMenu(browser, "Notes & Tasks");
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith("Location scout")), { timeout: 10_000, timeoutMsg: "the note was not restored" });
      await assertQuiet(browser, "note restore");
    });
  });

  test("add a task, complete it, undo and redo the completion", async () => {
    await step("task", async () => {
      await click(browser, "New Task");
      const t = await browser.$("#te-title");
      await t.waitForDisplayed({ timeout: 10_000 });
      await t.setValue("Confirm pier permit");
      await click(browser, "Add Task");
      await waitForText(browser, "Added task “Confirm pier permit”");
      const box = await taskBox("Confirm pier permit", false);
      await box.waitForDisplayed({ timeout: 10_000 });
      await box.click();
      await waitForText(browser, "Completed task “Confirm pier permit”");
      await undo(browser);
      await (await taskBox("Confirm pier permit", false)).waitForDisplayed({ timeout: 10_000, timeoutMsg: "undo did not reopen the task" });
      await redo(browser);
      await click(browser, "All", { within: `//*[@role="radiogroup"][@aria-label="Show tasks"]` });
      await (await taskBox("Confirm pier permit", true)).waitForDisplayed({ timeout: 10_000, timeoutMsg: "redo did not complete the task again" });
      await assertQuiet(browser, "task");
    });
  });

  test("relaunch: the note and the completed task persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Tide Tables");
      await projectMenu(browser, "Notes & Tasks");
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith("Location scout")), { timeout: 15_000, timeoutMsg: "the note is gone after relaunch" });
      await click(browser, "All", { within: `//*[@role="radiogroup"][@aria-label="Show tasks"]` });
      await (await taskBox("Confirm pier permit", true)).waitForDisplayed({ timeout: 10_000, timeoutMsg: "the completed task is gone after relaunch" });
      await (await browser.$('//section[@aria-label="Project Notes"]//li//button[starts-with(normalize-space(.),"Location scout")]')).click();
      assert.equal(await (await browser.$("#ne-body")).getValue(), "Check the old pier at low tide.");
      await closeDrawer();
      await assertQuiet(browser, "notes after relaunch");
      await closeProject(browser);
    });
  });
});
