// @e2e-data-root: own
//
// Application Home and Project Home (build plan item end-to-end specs per workspace): create a project, change its status on
// Project Home with undo/redo, rename, duplicate, archive and restore it on Application Home, and
// find every change again after quitting and relaunching the app.
// Delete Project is not exercised: it moves the project folder to the Windows Recycle Bin, which is
// outside the isolated data root.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, mainText, menuItem, openProject, redo, relaunch, retype, undo } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "home");
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('main button[aria-label^="Open "]')].map((b) => b.getAttribute("aria-label")));
/** Leave Application Home and come back (the page reloads its list when it mounts). */
async function refreshHome() {
  await click(browser, "Global Idea Vault");
  await click(browser, "Your projects");
  await waitForText(browser, "Pick up where you left off");
}
const statusChip = async () => (await browser.$('//main//button[starts-with(@aria-label,"Status: ")]')).getAttribute("aria-label");

describe("home", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("first launch creates a project that opens on an empty Project Home", async () => {
    await step("create", async () => {
      await firstLaunchProject(browser, "Harbour Lights");
      await waitForText(browser, "Harbour Lights");
      const ways = await browser.$('//*[@role="list" and @aria-label="Ways to begin"]');
      await ways.waitForDisplayed({ timeout: 10_000, timeoutMsg: "an empty project should offer ways to begin" });
      await assertQuiet(browser, "empty project home");
    });
  });

  test("status change on Project Home, undo and redo", async () => {
    await step("status", async () => {
      assert.equal(await statusChip(), "Status: Idea. Change status");
      await (await browser.$('//main//button[starts-with(@aria-label,"Status: ")]')).click();
      await menuItem(browser, "Writing");
      await browser.waitUntil(async () => (await statusChip()) === "Status: Writing. Change status", { timeout: 10_000, timeoutMsg: "status did not change" });
      await waitForText(browser, "Changed project status to Writing");
      await undo(browser);
      await browser.waitUntil(async () => (await statusChip()) === "Status: Idea. Change status", { timeout: 10_000, timeoutMsg: "undo did not restore the status" });
      await redo(browser);
      await browser.waitUntil(async () => (await statusChip()) === "Status: Writing. Change status", { timeout: 10_000, timeoutMsg: "redo did not reapply the status" });
      await assertQuiet(browser, "status change");
    });
  });

  test("Application Home lists a renamed project without leaving the page", async () => {
    await step("rename-refresh", async () => {
      await closeProject(browser);
      assert.deepEqual(await cards(), ["Open Harbour Lights, Feature Film, Writing"]);
      await click(browser, "More actions for Harbour Lights");
      await menuItem(browser, "Rename");
      await retype(browser, await browser.$("#rp-title"), "Harbour Lights Redux");
      await inDialog(browser, "Rename");
      await waitForText(browser, "Renamed to “Harbour Lights Redux”.");
      await browser.waitUntil(async () => (await cards()).includes("Open Harbour Lights Redux, Feature Film, Writing"), { timeout: 10_000, timeoutMsg: "the card kept the old title after Rename" });
    });
  });

  test("Application Home: rename, duplicate, archive and restore", async () => {
    await step("app-home", async () => {
      await refreshHome();
      await browser.waitUntil(async () => (await cards()).includes("Open Harbour Lights Redux, Feature Film, Writing"), { timeout: 10_000, timeoutMsg: "card not renamed" });

      await click(browser, "More actions for Harbour Lights Redux");
      await menuItem(browser, "Duplicate Project");
      await waitForText(browser, "It is a separate project with its own identity.");
      await refreshHome();
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 10_000, timeoutMsg: "the duplicate is not listed" });
      const copy = (await cards()).find((c) => c !== "Open Harbour Lights Redux, Feature Film, Writing");
      const copyTitle = /^Open (.+), Feature Film, /.exec(copy)?.[1];
      assert.ok(copyTitle && copyTitle !== "Harbour Lights Redux", `unexpected duplicate card ${copy}`);

      // Search filters the list.
      const search = await browser.$('//input[@aria-label="Search projects"]');
      await search.setValue("Redux");
      await browser.waitUntil(async () => (await cards()).every((c) => c.includes("Redux")), { timeout: 10_000, timeoutMsg: "search did not filter" });
      await retype(browser, search, "");
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 10_000 });

      await click(browser, `More actions for ${copyTitle}`);
      await menuItem(browser, "Archive");
      await waitForText(browser, `“${copyTitle}” was archived.`);
      await refreshHome();
      await browser.waitUntil(async () => (await cards()).length === 1, { timeout: 10_000, timeoutMsg: "archived project still listed" });
      await click(browser, "Open Archived (1)");
      await waitForText(browser, "Archived projects");
      assert.ok((await mainText(browser)).includes(copyTitle), "the archived project is not on the Archived list");
      await click(browser, "Restore");
      await waitForText(browser, `“${copyTitle}” is back in your projects.`);
      await click(browser, "Back to projects");
      await refreshHome();
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 10_000, timeoutMsg: "restored project not listed" });
      await assertQuiet(browser, "application home");
    });
  });

  test("relaunch: both projects, the new title and the status persist", async () => {
    await step("relaunch", async () => {
      browser = await relaunch(browser);
      const list = await cards();
      assert.equal(list.length, 2, `projects after relaunch: ${list}`);
      assert.ok(list.includes("Open Harbour Lights Redux, Feature Film, Writing"), `renamed project missing: ${list}`);
      await openProject(browser, "Harbour Lights Redux");
      await browser.waitUntil(async () => (await browser.execute(() => document.title)) === "Harbour Lights Redux - OpenFrame Studio", { timeout: 10_000 });
      assert.equal(await statusChip(), "Status: Writing. Change status");
      await assertQuiet(browser, "reopened project home");
      await closeProject(browser);
    });
  });
});

