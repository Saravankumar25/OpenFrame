// @e2e-data-root: own
//
// Several screenplays per project (ADR-0012 amendment of 2026-10-08, masterplan area 11 box
// "Several screenplays per project"): write in the first screenplay, add a second one from the
// screenplay picker, write in it, switch back and forth, rename the second one, quit, relaunch and
// find both screenplays with their own text. Then delete the second one from the picker (it goes
// to Recently Deleted for 30 days, owner decision of 2026-10-08) and restore it with its text.
// The picker is driven from the keyboard.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, editorElements, firstLaunchProject, inDialog, menuItem, openProject, relaunch, retype } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "several-screenplays");
const typeKeys = (text) => browser.keys(text.split(""));
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");
async function waitSaved() {
  await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
}
/** Call an application operation exactly as the UI does (ipc/client.ts `call`). */
async function op(name, args) {
  return browser.execute((n, a) =>
    window.__TAURI_INTERNALS__
      .invoke("of_invoke", { op: n, args: a })
      .then((v) => ({ ok: v }))
      .catch((e) => ({ err: e })), name, args).then((r) => {
    if (r.err) throw new Error(`${name} failed: ${JSON.stringify(r.err)}`);
    return r.ok;
  });
}
/** Open the screenplay picker from the keyboard and choose an item. */
async function picker(open, item) {
  const trigger = await browser.$(`//button[@aria-label=${JSON.stringify(`Screenplay: ${open}. Switch screenplay`)}]`);
  await trigger.waitForClickable({ timeout: 10_000, timeoutMsg: `the picker does not show "${open}"` });
  await trigger.click();
  await menuItem(browser, item);
}
async function writeFirstLine(text) {
  await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
  await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
  await typeKeys(text);
  await waitSaved();
}
const hasLine = (line) => async () => (await editorElements(browser)).some((e) => e.endsWith(line));

describe("several screenplays per project", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("add a second screenplay, write in both and switch between them", async () => {
    await step("create", async () => {
      await firstLaunchProject(browser, "Two Rivers");
      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await writeFirstLine("The feature opens on the river.");

      // Second screenplay from the picker; the first stays as it is.
      await picker("Two Rivers", "New Screenplay");
      const title = await browser.$("#ns-title");
      await title.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, title, "Two Rivers");
      await waitForText(browser, "Another screenplay here already has this title");
      await retype(browser, title, "Two Rivers (short)");
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await browser.$('//button[@aria-label="Screenplay: Two Rivers (short). Switch screenplay"]').isExisting()), {
        timeout: 15_000,
        timeoutMsg: "the new screenplay did not open",
      });
      await writeFirstLine("The short opens on the bridge.");
      assert.ok(!(await hasLine("The feature opens on the river.")()), "the short starts empty");

      // Switch back: the feature's text is there and unchanged.
      await picker("Two Rivers (short)", "Two Rivers ·");
      await browser.waitUntil(hasLine("The feature opens on the river."), { timeout: 15_000, timeoutMsg: "switching back did not show the feature" });

      // Edit the feature, then the short again.
      const line = await browser.$("//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')][contains(.,'river')]");
      await line.click();
      await browser.keys("End");
      await typeKeys(" At dawn.");
      await waitSaved();
      await picker("Two Rivers", "Two Rivers (short)");
      await browser.waitUntil(hasLine("The short opens on the bridge."), { timeout: 15_000, timeoutMsg: "switching did not show the short" });

      // Rename the open screenplay.
      await picker("Two Rivers (short)", "Rename Screenplay");
      const rename = await browser.$("#rs-title");
      await rename.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, rename, "Two Rivers: The Short");
      await inDialog(browser, "Rename");
      await browser.$('//button[@aria-label="Screenplay: Two Rivers: The Short. Switch screenplay"]').waitForExist({ timeout: 10_000 });

      const ov = await op("screenplay.overview", {});
      assert.deepEqual(ov.screenplays.map((s) => s.title), ["Two Rivers", "Two Rivers: The Short"]);
      await assertQuiet(browser, "several screenplays");
    });
  });

  test("relaunch: both screenplays keep their own text", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Two Rivers");
      await openWorkspace(browser, "Screenplay");
      // The screenplay last open comes back.
      await browser.waitUntil(hasLine("The short opens on the bridge."), { timeout: 15_000, timeoutMsg: "the short did not reopen" });
      await picker("Two Rivers: The Short", "Two Rivers ·");
      await browser.waitUntil(hasLine("The feature opens on the river. At dawn."), { timeout: 15_000, timeoutMsg: "the feature edit did not persist" });
      const ov = await op("screenplay.overview", {});
      assert.equal(ov.screenplays.length, 2);
      for (const s of ov.screenplays) assert.equal(s.draftCount, 1, `${s.title} keeps its one draft`);
      await assertQuiet(browser, "after relaunch");
      await closeProject(browser);
    });
  });

  test("delete a screenplay to Recently Deleted and restore it with its text", async () => {
    await step("delete-restore", async () => {
      await openProject(browser, "Two Rivers");
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(hasLine("The feature opens on the river. At dawn."), { timeout: 15_000, timeoutMsg: "the feature did not open" });
      await picker("Two Rivers", "Two Rivers: The Short");
      await browser.waitUntil(hasLine("The short opens on the bridge."), { timeout: 15_000, timeoutMsg: "the short did not open" });

      // The confirmation names the screenplay and says where it goes.
      await picker("Two Rivers: The Short", "Delete Screenplay");
      await waitForText(browser, "Delete “Two Rivers: The Short”?");
      await waitForText(browser, "They stay there for 30 days");
      await inDialog(browser, "Delete Screenplay");
      await browser.$('//button[@aria-label="Screenplay: Two Rivers. Switch screenplay"]').waitForExist({
        timeout: 15_000,
        timeoutMsg: "the remaining screenplay did not open after the delete",
      });
      await browser.waitUntil(hasLine("The feature opens on the river. At dawn."), { timeout: 15_000, timeoutMsg: "the feature is not shown" });
      let ov = await op("screenplay.overview", {});
      assert.deepEqual(ov.screenplays.map((s) => s.title), ["Two Rivers"]);

      // Recently Deleted lists it and restores it.
      await click(browser, "Project: Two Rivers. Switch or manage projects");
      await menuItem(browser, "Recently Deleted");
      await waitForText(browser, "Two Rivers: The Short");
      await click(browser, "Restore Two Rivers: The Short");
      await openWorkspace(browser, "Screenplay");
      await picker("Two Rivers", "Two Rivers: The Short");
      await browser.waitUntil(hasLine("The short opens on the bridge."), { timeout: 15_000, timeoutMsg: "the restored short lost its text" });
      ov = await op("screenplay.overview", {});
      assert.deepEqual(ov.screenplays.map((s) => s.title), ["Two Rivers", "Two Rivers: The Short"]);
      await assertQuiet(browser, "delete and restore a screenplay");
      await closeProject(browser);
    });
  });
});
