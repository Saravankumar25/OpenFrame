// @e2e-data-root: own
//
// Project pages (build plan item end-to-end specs per workspace): Project settings (edit, save, undo, redo), Templates (create,
// delete, restore), Recently Deleted (restore and permanent delete with its second confirmation)
// and Activity (entries and filters); then quit, relaunch and check what persisted.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, menuItem, openProject, projectMenu, redo, relaunch, retype, undo } from "../lib/flows.mjs";

const LOGLINE = "A ferry pilot races the tide to reach her brother.";

let browser;
const step = stepper(() => browser, "project-pages");
const windowTitle = () => browser.execute(() => document.title);
const value = async (id) => (await browser.$(`#${id}`)).getValue();
const templates = () =>
  browser.execute(() => [...document.querySelectorAll('section[aria-label="This project"] button[aria-label^="Preview "]')].map((b) => b.getAttribute("aria-label")));

async function createTemplate(name) {
  await click(browser, "Create Template");
  const n = await browser.$("#tp-name");
  await n.waitForDisplayed({ timeout: 10_000 });
  await n.setValue(name);
  await inDialog(browser, "Create");
  await browser.waitUntil(async () => (await templates()).some((t) => t.startsWith(`Preview ${name},`)), { timeout: 10_000, timeoutMsg: `template ${name} not listed` });
}

async function deleteTemplate(name) {
  await click(browser, `Actions for ${name}`);
  await menuItem(browser, "Delete");
  await waitForText(browser, `Moved template “${name}” to Recently Deleted`);
  await browser.waitUntil(async () => !(await templates()).some((t) => t.startsWith(`Preview ${name},`)), { timeout: 10_000, timeoutMsg: `template ${name} still listed` });
}

async function templatesTab() {
  await projectMenu(browser, "Project settings");
  const tab = await browser.$('//*[@role="tablist"][@aria-label="Settings"]//*[@role="tab"][normalize-space(.)="Templates"]');
  await tab.waitForClickable({ timeout: 10_000 });
  await tab.click();
  await waitForText(browser, "Create Template");
}

describe("project pages", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("Project settings: change title and logline, save, undo, redo", async () => {
    await step("settings", async () => {
      await firstLaunchProject(browser, "Tide Line");
      await projectMenu(browser, "Project settings");
      const title = await browser.$("#project-settings-page-title");
      await title.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, title, "Tide Line Revised");
      await (await browser.$("#project-settings-page-logline")).setValue(LOGLINE);
      await click(browser, "Save");
      await waitForText(browser, "Updated project settings");
      await browser.waitUntil(async () => (await windowTitle()) === "Tide Line Revised - OpenFrame Studio", { timeout: 10_000, timeoutMsg: "window title not updated" });
      await undo(browser);
      await browser.waitUntil(async () => (await windowTitle()) === "Tide Line - OpenFrame Studio", { timeout: 10_000, timeoutMsg: "undo did not restore the title" });
      await browser.waitUntil(async () => (await value("project-settings-page-logline")) === "", { timeout: 10_000, timeoutMsg: "undo did not clear the logline" });
      await redo(browser);
      await browser.waitUntil(async () => (await windowTitle()) === "Tide Line Revised - OpenFrame Studio", { timeout: 10_000, timeoutMsg: "redo did not reapply the title" });
      await browser.waitUntil(async () => (await value("project-settings-page-logline")) === LOGLINE, { timeout: 10_000, timeoutMsg: "redo did not reapply the logline" });
      await assertQuiet(browser, "project settings");
    });
  });

  test("Templates: create two, delete both, restore one, delete the other permanently", async () => {
    await step("templates", async () => {
      await templatesTab();
      await createTemplate("Night exterior call");
      await createTemplate("Old call template");
      await deleteTemplate("Night exterior call");
      await deleteTemplate("Old call template");
      await assertQuiet(browser, "templates");

      await projectMenu(browser, "Recently Deleted");
      await click(browser, "Restore Night exterior call");
      await waitForText(browser, "Restored “Night exterior call”");
      await click(browser, "Delete Old call template permanently");
      await waitForText(browser, "Delete “Old call template” permanently?");
      await inDialog(browser, "Delete Permanently");
      await waitForText(browser, "“Old call template” was deleted permanently.");
      await browser.waitUntil(async () => !(await browser.execute(() => document.querySelector("main")?.innerText ?? "")).includes("Old call template"), {
        timeout: 10_000,
        timeoutMsg: "the purged template is still listed",
      });
      await assertQuiet(browser, "recently deleted");

      await templatesTab();
      await browser.waitUntil(async () => (await templates()).some((t) => t.startsWith("Preview Night exterior call,")), { timeout: 10_000, timeoutMsg: "restored template missing" });
    });
  });

  test("Activity lists what happened and filters by kind", async () => {
    await step("activity", async () => {
      await projectMenu(browser, "Activity");
      await waitForText(browser, "Activity History");
      const entries = await browser.execute(() => [...document.querySelectorAll('ol[aria-label="Activity"] li')].map((li) => li.innerText));
      assert.ok(entries.length >= 1, "no activity entries");
      assert.ok(entries.some((e) => /E2E Tester/.test(e)), `entries do not name the user: ${entries.join(" | ")}`);
      await click(browser, "Story", { within: `//*[@role="radiogroup"][@aria-label="Show activity"]` });
      await waitForText(browser, "Nothing of this kind has happened yet.");
      await click(browser, "All", { within: `//*[@role="radiogroup"][@aria-label="Show activity"]` });
      await browser.$('//ol[@aria-label="Activity"]').waitForDisplayed({ timeout: 10_000 });
      await assertQuiet(browser, "activity");
    });
  });

  test("relaunch: settings, the restored template and the purge persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Tide Line Revised");
      await projectMenu(browser, "Project settings");
      await browser.waitUntil(async () => (await value("project-settings-page-logline")) === LOGLINE, { timeout: 10_000, timeoutMsg: "logline lost after relaunch" });
      await templatesTab();
      // The page header (with Create Template) renders before the list loads,
      // so wait for the "This project" section instead of reading it at once.
      let list = [];
      await browser.waitUntil(
        async () => {
          list = await templates();
          return list.some((t) => t.startsWith("Preview Night exterior call,"));
        },
        { timeout: 10_000, timeoutMsg: "restored template missing after relaunch" },
      );
      assert.ok(!list.some((t) => t.startsWith("Preview Old call template,")), "purged template came back");
      await projectMenu(browser, "Recently Deleted");
      await waitForText(browser, "Nothing has been deleted.");
      await assertQuiet(browser, "project pages after relaunch");
      await closeProject(browser);
    });
  });
});
