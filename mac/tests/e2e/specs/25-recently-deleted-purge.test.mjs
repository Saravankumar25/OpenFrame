// @e2e-data-root: own
//
// Recently Deleted keeps items for 30 days, then removes them for good (owner decision 2026-10-07,
// ADR-0012 amendment; build plan area 9 box "Recently Deleted 30-day purge"; feature row CORE-007).
//
// Delete two project notes and an Idea Vault idea in the real app, see "30 days left" on both
// Recently Deleted screens, then quit. With the app closed, the spec moves the delete times back
// in the project file (one note 31 days, the idea 40 days, the other note 29 and a half days), as
// if that much time had passed. After a relaunch, opening the project removes what is due and
// nothing else: the live note stays, the young note shows "1 day left", and the Owner can still
// delete it permanently before then. No network is involved at any point.
//
// The delete times are moved with Node's built-in SQLite module while no app holds the file; the
// app's own Rust tests (`recently_deleted_purge.rs`) cover the boundary millisecond, clock changes
// and an interrupted purge.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join, sep } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { click, launchApp, openWorkspace, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, openProject, projectMenu, relaunch } from "../lib/flows.mjs";

const PROJECT = "Low Tide";
const KEEP = "Harbour master";
const OLD = "Old ferry route";
const YOUNG = "New ferry route";
const IDEA = "A ferry that only sails when nobody is waiting.";
const DAY = 24 * 60 * 60 * 1000;

let browser;
const step = stepper(() => browser, "purge");
const notes = () => browser.execute(() => [...document.querySelectorAll('section[aria-label="Project Notes"] li button')].map((b) => b.innerText.trim()));
const vaultItems = () => browser.execute(() => [...document.querySelectorAll('[aria-label="Idea Vault items"] [role="button"][data-item-id]')].map((e) => e.getAttribute("aria-label")));
/** The visible text of every table row on the current Recently Deleted screen. */
const trashRows = () => browser.execute(() => [...document.querySelectorAll("main table tbody tr")].map((r) => r.innerText.replace(/\s+/g, " ").trim()));
const rowWith = async (text) => (await trashRows()).find((r) => r.includes(text));
const closeDrawer = async () => {
  const b = await browser.$('//aside//button[@aria-label="Close"]');
  await b.click();
  await b.waitForExist({ reverse: true, timeout: 10_000 });
};

async function newNote(title) {
  await click(browser, "New Note");
  const t = await browser.$("#ne-title");
  await t.waitForDisplayed({ timeout: 10_000 });
  await t.setValue(title);
  await (await browser.$("#ne-body")).setValue(`${title}: written by the end-to-end suite.`);
  await closeDrawer();
  await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith(title)), { timeout: 10_000, timeoutMsg: `the note "${title}" is not listed` });
}

async function deleteNote(title) {
  await (await browser.$(`//section[@aria-label="Project Notes"]//li//button[starts-with(normalize-space(.),"${title}")]`)).click();
  await click(browser, "Delete", { within: "//aside" });
  await waitForText(browser, `Moved note “${title}” to Recently Deleted`);
  await browser.waitUntil(async () => !(await notes()).some((n) => n.startsWith(title)), { timeout: 10_000, timeoutMsg: `the note "${title}" was not deleted` });
}

/** The one project file in this spec's own data root (never a backup copy). */
function projectDatabase() {
  const root = process.env.E2E_DATA_ROOT;
  assert.ok(root, "E2E_DATA_ROOT is not set; run the suite with `npm run test:e2e`");
  const found = readdirSync(join(root, "projects"), { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile() && e.name === "project.sqlite")
    .map((e) => join(e.parentPath ?? e.path, e.name))
    .filter((p) => !p.split(sep).includes("backups"));
  assert.equal(found.length, 1, `expected one project file, found: ${found.join(", ")}`);
  return found[0];
}

/**
 * Move a delete back in time by `days`, both the Recently Deleted record and the deleted object,
 * as if it had happened that long ago. `where` picks the record. Runs only while the app is closed.
 */
function age(db, where, params, days) {
  const shift = Math.round(days * DAY);
  const rows = db.prepare(`SELECT id, object_id AS objectId, table_name AS tableName FROM deleted_item WHERE ${where}`).all(...params);
  assert.equal(rows.length, 1, `expected one Recently Deleted record for ${where} ${params.join(", ")}`);
  const [{ id, objectId, tableName }] = rows;
  assert.match(tableName, /^[a-z_]+$/, "a plain table name");
  db.prepare("UPDATE deleted_item SET deleted_at = deleted_at - ? WHERE id = ?").run(shift, id);
  const moved = db.prepare(`UPDATE "${tableName}" SET deleted_at = deleted_at - ? WHERE id = ? AND deleted_at IS NOT NULL`).run(shift, objectId);
  assert.equal(Number(moved.changes), 1, `the deleted object behind ${id} was not found in ${tableName}`);
}

describe("Recently Deleted 30-day purge", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("deleted items show 30 days left on both Recently Deleted screens", async () => {
    await step("delete", async () => {
      await firstLaunchProject(browser, PROJECT);
      await projectMenu(browser, "Notes & Tasks");
      await waitForText(browser, "No notes yet.");
      for (const title of [KEEP, OLD, YOUNG]) await newNote(title);
      await deleteNote(OLD);
      await deleteNote(YOUNG);

      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Quick Capture");
      await (await browser.$('//textarea[@aria-label="Idea"]')).setValue(IDEA);
      await inDialog(browser, "Save");
      await waitForText(browser, "Saved to the Project Idea Vault");
      await browser.waitUntil(async () => (await vaultItems()).length === 1, { timeout: 10_000, timeoutMsg: "the idea is not in the Vault" });
      await (await browser.$('//*[@aria-label="Idea Vault items"]//*[@role="button"][@data-item-id]')).click();
      await browser.keys("Delete");
      await browser.waitUntil(async () => (await vaultItems()).length === 0, { timeout: 10_000, timeoutMsg: "the idea was not deleted" });

      // The Vault's own Recently Deleted.
      await click(browser, "Recently Deleted");
      await waitForText(browser, "Deleted items stay here for 30 days, then they are removed for good.");
      await browser.$('//main//table//th[normalize-space(.)="Removed in"]').waitForDisplayed({ timeout: 10_000 });
      await browser.waitUntil(async () => (await trashRows()).length === 1, { timeout: 10_000, timeoutMsg: "the idea is not in the Vault's Recently Deleted" });
      assert.match((await trashRows())[0], /30 days left/);
      await assertQuiet(browser, "vault recently deleted");
      await click(browser, "Back to Vault");

      // The project's Recently Deleted.
      await projectMenu(browser, "Recently Deleted");
      await waitForText(browser, "Deleted items stay here for 30 days, then they are removed for good.");
      await browser.$('//main//table//th[normalize-space(.)="Removed in"]').waitForDisplayed({ timeout: 10_000 });
      for (const title of [OLD, YOUNG]) {
        await browser.waitUntil(async () => !!(await rowWith(title)), { timeout: 10_000, timeoutMsg: `"${title}" is not in Recently Deleted` });
        assert.match(await rowWith(title), /30 days left/, `"${title}" should have 30 days left`);
      }
      assert.equal(await rowWith(KEEP), undefined, "a live note is never in Recently Deleted");
      await assertQuiet(browser, "project recently deleted");
    });
  });

  test("after 30 days the next open removes what is due, and nothing else", async () => {
    await step("age-and-reopen", async () => {
      await closeProject(browser);
      await browser.deleteSession().catch(() => undefined);
      browser = undefined;
      await new Promise((r) => setTimeout(r, 1500));

      // As if time had passed: the app is closed and holds nothing open.
      const db = new DatabaseSync(projectDatabase());
      try {
        age(db, "title = ? AND table_name <> 'vault_item'", [OLD], 31);
        age(db, "title = ? AND table_name <> 'vault_item'", [YOUNG], 29.5);
        age(db, "table_name = 'vault_item'", [], 40);
      } finally {
        db.close();
      }

      browser = await launchApp();
      await waitForText(browser, "Your projects", 30_000);
      await openProject(browser, PROJECT);

      await projectMenu(browser, "Recently Deleted");
      await browser.waitUntil(async () => !!(await rowWith(YOUNG)), { timeout: 15_000, timeoutMsg: `"${YOUNG}" should still be in Recently Deleted` });
      assert.equal(await rowWith(OLD), undefined, `"${OLD}" was deleted 31 days ago and should be gone`);
      assert.match(await rowWith(YOUNG), /\b1 day left/, `"${YOUNG}" was deleted 29.5 days ago and has 1 day left`);
      // Only the young note is left: the old note and the idea deleted 40 days ago are gone.
      const left = await trashRows();
      assert.equal(left.length, 1, `Recently Deleted after the purge: ${left.join(" | ")}`);

      // The live note is untouched; the removed one cannot come back.
      await projectMenu(browser, "Notes & Tasks");
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith(KEEP)), { timeout: 10_000, timeoutMsg: "the live note is gone" });
      assert.ok(!(await notes()).some((n) => n.startsWith(OLD)), "the removed note is back in the list");

      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Recently Deleted");
      await waitForText(browser, "Nothing deleted");
      await click(browser, "Back to Vault");
      await assertQuiet(browser, "after the purge");
    });
  });

  test("the Owner can still delete an item permanently before its 30 days are up", async () => {
    await step("delete-early", async () => {
      await projectMenu(browser, "Recently Deleted");
      await click(browser, `Delete ${YOUNG} permanently`);
      await inDialog(browser, "Delete Permanently");
      await browser.waitUntil(async () => !(await rowWith(YOUNG)), { timeout: 10_000, timeoutMsg: `"${YOUNG}" is still in Recently Deleted` });
      await assertQuiet(browser, "delete early");
    });
  });

  test("relaunch: nothing removed comes back and the live note stays", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, PROJECT);
      await projectMenu(browser, "Notes & Tasks");
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith(KEEP)), { timeout: 15_000, timeoutMsg: "the live note is gone after relaunch" });
      const listed = await notes();
      assert.ok(!listed.some((n) => n.startsWith(OLD) || n.startsWith(YOUNG)), `removed notes came back: ${listed.join(" | ")}`);
      await assertQuiet(browser, "relaunch after purge");
      await closeProject(browser);
    });
  });
});
