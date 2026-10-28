// @e2e-data-root: own
//
// Application Home and Project Home (build plan item end-to-end specs per workspace): create a project, change its status on
// Project Home with undo/redo, rename, duplicate, archive and restore it on Application Home, and
// find every change again after quitting and relaunching the app.
// Delete Project (Windows phase W2): a debug build on an isolated data root moves the deleted folder
// into <root>/recycle-bin instead of the Windows Recycle Bin, so the step never touches the
// computer's own bin; the operation, the dialog and the Home list are the real ones.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, mainText, menuItem, openProject, redo, relaunch, retype, undo } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "home");
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('main button[aria-label^="Open "]')].map((b) => b.getAttribute("aria-label")));
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
      await browser.waitUntil(async () => (await cards()).includes("Open Harbour Lights Redux, Feature Film, Writing"), { timeout: 10_000, timeoutMsg: "card not renamed" });

      await click(browser, "More actions for Harbour Lights Redux");
      await menuItem(browser, "Duplicate Project");
      await waitForText(browser, "It is a separate project with its own identity.");
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
      await browser.waitUntil(async () => (await cards()).length === 1, { timeout: 10_000, timeoutMsg: "archived project still listed" });
      await click(browser, "Open Archived (1)");
      await waitForText(browser, "Archived projects");
      assert.ok((await mainText(browser)).includes(copyTitle), "the archived project is not on the Archived list");
      await click(browser, "Restore");
      await waitForText(browser, `“${copyTitle}” is back in your projects.`);
      await browser.waitUntil(async () => !(await mainText(browser)).includes(copyTitle), { timeout: 10_000, timeoutMsg: "the restored project stayed on the Archived list" });
      await click(browser, "Back to projects");
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 10_000, timeoutMsg: "restored project not listed" });

      // Pin and Unpin move the card at once, without leaving the page.
      const pinnedSection = () => browser.execute(() => !!document.querySelector('main section[aria-label="Pinned"]'));
      await click(browser, `More actions for ${copyTitle}`);
      await menuItem(browser, "Pin");
      await browser.waitUntil(pinnedSection, { timeout: 10_000, timeoutMsg: "Pin did not show the Pinned section" });
      await click(browser, `More actions for ${copyTitle}`);
      await menuItem(browser, "Unpin");
      await browser.waitUntil(async () => !(await pinnedSection()), { timeout: 10_000, timeoutMsg: "Unpin left the Pinned section" });
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

  test("Delete Project: the typed name confirms, the folder leaves, Home forgets it after a relaunch", async () => {
    await step("delete", async () => {
      const bin = join(process.env.E2E_DATA_ROOT ?? "", "recycle-bin");
      const before = existsSync(bin) ? readdirSync(bin).length : 0;
      const doomed = (await cards()).find((c) => !c.startsWith("Open Harbour Lights Redux,"));
      const title = /^Open (.+), Feature Film, /.exec(doomed ?? "")?.[1];
      assert.ok(title, `no second project to delete: ${await cards()}`);
      await click(browser, `More actions for ${title}`);
      await menuItem(browser, "Delete Project");
      const confirm = await browser.$("#dp-confirm");
      await confirm.waitForDisplayed({ timeout: 10_000 });
      const remove = await browser.$('//*[@role="dialog"]//button[normalize-space(.)="Delete Project"]');
      assert.equal(await remove.isEnabled(), false, "Delete Project is enabled before the name is typed");
      await confirm.setValue(title);
      await remove.waitForEnabled({ timeout: 5_000 });
      await remove.click();
      await waitForText(browser, `“${title}” was moved to the Windows Recycle Bin.`);
      await browser.waitUntil(async () => (await cards()).length === 1, { timeout: 10_000, timeoutMsg: "the deleted project is still listed" });
      assert.equal(readdirSync(bin).length, before + 1, "the project folder did not reach the bin");

      browser = await relaunch(browser);
      const list = await cards();
      assert.deepEqual(list, ["Open Harbour Lights Redux, Feature Film, Writing"], `projects after relaunch: ${list}`);
      await assertQuiet(browser, "application home after delete");
    });
  });
});

