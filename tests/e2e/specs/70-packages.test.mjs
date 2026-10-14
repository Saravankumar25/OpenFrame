// @e2e-data-root: own
//
// Packages and backup (build plan area "Packages and backup", features XFER-011 and XFER-012):
// create a backup, export a Full Project Package and a screenplay review package through the real
// dialogs (the Windows Save and Open dialogs are filled through UI Automation), see them in the
// package history. The review package is opened as a reviewer, a comment is added with the
// keyboard, a Response package goes back, and the author previews it and imports the comment.
// Then the project is closed and the backup restored from Application Home. Before the restore
// runs, the screen says the backup's private note will become the restorer's (ADR-0012
// amendment of 2026-10-08). The project already exists, so the identity collision choice appears
// and the backup opens as a copy. A damaged backup is refused with a message and creates nothing. After a relaunch both
// projects are listed.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { click, field, launchApp, pageText, stepper, waitForText } from "../lib/app.mjs";
import { TWO_SCENES, assertQuiet, closeProject, firstLaunchProject, importFountain, inDialog, projectMenu, relaunch } from "../lib/flows.mjs";
import { chooseInFileDialog } from "../lib/native-dialog.mjs";

let browser;
const step = stepper(() => browser, "packages");
const OUT = join(process.env.E2E_DATA_ROOT ?? ".", "package-out");
const BACKUP = join(OUT, "Harbour Lights backup.ofbackup");
const PROJECT = join(OUT, "Harbour Lights.ofproject");
const REVIEW = join(OUT, "Harbour Lights review.ofscriptreview");
const RESPONSE = join(OUT, "Harbour Lights reply.ofresponse");
const DAMAGED = join(OUT, "damaged.ofbackup");
const PRIVATE_NOTE = "Harbour private note for the restore check";

/** Click a dialog button that opens a native file dialog, then choose `path` in it. */
async function pickFile(name, dialogTitle, path) {
  const chooser = chooseInFileDialog(dialogTitle, path);
  await inDialog(browser, name);
  await chooser;
}

const homeTitles = () =>
  browser.execute(() => [...document.querySelectorAll("main button[aria-label^='Open ']")].map((b) => b.getAttribute("aria-label")));

describe("packages and backup", () => {
  before(async () => {
    mkdirSync(OUT, { recursive: true });
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("create a backup through Create Backup and the Save dialog", async () => {
    await step("backup", async () => {
      await firstLaunchProject(browser, "Harbour Lights");
      await importFountain(browser, TWO_SCENES);
      // A private note in the first scene, so the restore screen has private notes to announce.
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      await click(browser, "Notes", { within: '//*[@role="toolbar"][@aria-label="Screenplay"]' });
      await click(browser, "Add private note");
      await (await field(browser, "Private note")).setValue(PRIVATE_NOTE);
      await click(browser, "Save", { within: "//*[contains(@class,'cmt') and contains(@class,'priv')]" });
      await waitForText(browser, PRIVATE_NOTE);
      await projectMenu(browser, "Create Backup…");
      await waitForText(browser, "A recoverable copy of supported project state.");
      await pickFile("Browse…", "Save backup to", BACKUP);
      await browser.waitUntil(async () => (await (await browser.$('//input[@aria-label="Backup location"]')).getValue()) === BACKUP, {
        timeout: 10_000,
        timeoutMsg: "the chosen backup location is not shown",
      });
      await inDialog(browser, "Create Backup");
      await waitForText(browser, "Backup created.", 60_000);
      assert.ok(existsSync(BACKUP), "the backup file exists");
      await inDialog(browser, "Close");
      await assertQuiet(browser, "backup");
    });
  });

  test("export a Full Project Package and a screenplay review package", async () => {
    await step("exports", async () => {
      await projectMenu(browser, "Export Full Project…");
      await waitForText(browser, "Private project transfer.");
      await pickFile("Export Project Package…", "Export Project Package", PROJECT);
      await waitForText(browser, "Project package exported.", 60_000);
      await inDialog(browser, "Close");
      assert.ok(existsSync(PROJECT), "the project package exists");

      await projectMenu(browser, "Export Review Package (script)…");
      await waitForText(browser, "Script Review Package");
      await browser.waitUntil(async () => (await pageText(browser)).includes("Private notes"), { timeout: 15_000, timeoutMsg: "the review package preview did not load" });
      await pickFile("Export Package…", "Export Screenplay review package", REVIEW);
      await waitForText(browser, "Package exported.", 30_000);
      await inDialog(browser, "Close");
      assert.ok(existsSync(REVIEW), "the review package exists");

      await projectMenu(browser, "Import & Export History");
      // The history opens on Imports; what this project created and exported is the Package log.
      await click(browser, "Package log", { within: '//*[@role="dialog"]' });
      await waitForText(browser, "Created backup");
      await waitForText(browser, "Exported project package");
      await browser.keys("Escape");
      await assertQuiet(browser, "exports");
    });
  });

  test("review round trip: comment as the reviewer, send a response, preview and import it", async () => {
    await step("review", async () => {
      const opener = chooseInFileDialog("Open Review Package", REVIEW);
      await projectMenu(browser, "Open Review Package…");
      await opener;
      await waitForText(browser, "Review package. Comments are separate from the original project.", 20_000);
      const box = await field(browser, "Comment");
      await box.click();
      await box.setValue("Tighten the harbour line.");
      await browser.keys(["Control", "Enter"]);
      await waitForText(browser, "Yours");
      const saver = chooseInFileDialog("Export Response Package", RESPONSE);
      await click(browser, "Export Response Package");
      await saver;
      await waitForText(browser, "Response package exported.", 30_000);
      assert.ok(existsSync(RESPONSE), "the response package exists");
      await click(browser, "Close review");

      await projectMenu(browser, "Import Exchange Package…");
      await pickFile("Choose Package…", "Import Exchange Package", RESPONSE);
      // Preview before apply: the comment is listed (open the Comments group) and nothing is
      // imported yet.
      await click(browser, "Show Comments", { within: '//*[@role="dialog"]', timeout: 20_000 });
      await waitForText(browser, "Tighten the harbour line.", 20_000);
      await inDialog(browser, "Accept All Safe");
      await waitForText(browser, "Imported:", 30_000);
      await inDialog(browser, "Close");
      await assertQuiet(browser, "review");
    });
  });

  test("restore the backup from Application Home: the collision choice opens it as a copy", async () => {
    await step("restore", async () => {
      await closeProject(browser);
      await click(browser, "Restore Backup or Package…");
      await pickFile("Choose Package…", "Open Project Package", BACKUP);
      await waitForText(browser, "This project already exists on this computer", 20_000);
      await waitForText(browser, "The private note in this package will become yours.");
      await waitForText(browser, "Open as Copy");
      await inDialog(browser, "Open as Copy");
      await waitForText(browser, "Opened as a copy.", 60_000);
      await inDialog(browser, "Close");
      await browser.waitUntil(async () => (await browser.execute(() => document.title)) === "Harbour Lights (copy) - OpenFrame Studio", {
        timeout: 20_000,
        timeoutMsg: "the restored copy did not open",
      });
      await assertQuiet(browser, "restore");
    });
  });

  test("a damaged backup is refused and nothing is created", async () => {
    await step("damaged", async () => {
      writeFileSync(DAMAGED, "PK\u0003\u0004 this is not a whole package");
      await closeProject(browser);
      const before = await homeTitles();
      await click(browser, "Restore Backup or Package…");
      await pickFile("Choose Package…", "Open Project Package", DAMAGED);
      await browser.waitUntil(async () => /package|archive/i.test(await pageText(browser)) && (await pageText(browser)).includes("Choose Package…"), {
        timeout: 15_000,
        timeoutMsg: "no message for the damaged package",
      });
      await inDialog(browser, "Cancel");
      assert.deepEqual(await homeTitles(), before, "a damaged package created a project");
    });
  });

  test("relaunch: the original and the restored copy are both listed", async () => {
    await step("relaunch", async () => {
      browser = await relaunch(browser);
      const titles = (await homeTitles()).join("\n");
      assert.match(titles, /Open Harbour Lights \(copy\)/);
      assert.match(titles, /Open Harbour Lights(?! \(copy\))/);
    });
  });
});
