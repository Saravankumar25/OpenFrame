// @e2e-data-root: own
//
// Story page (owner requests 2026-10-20 and 2026-10-27): the Story tab of the Story workspace is one
// writing page on an A4 sheet, the tab Story opens on, saved in the project and not linked to
// anything else. Type a few paragraphs, format a word (bold, size), zoom the page, see the page
// saved, relaunch the app and find the text again, then export it as plain text, PDF and Word.
// The exports call the same operation the Export dialog calls, with a path in the spec's own data
// folder, so no system file dialog is needed (as in spec 63); the dialog itself is opened and
// checked for its three formats.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, openProject, relaunch } from "../lib/flows.mjs";

const PARAGRAPHS = [
  "The last bus leaves at midnight.",
  "Meera runs, one shoe in her hand, and the driver pretends not to see her.",
  "By the time the doors close she has already forgiven him.",
];
const TEXT = PARAGRAPHS.join("\n\n");

let browser;
const step = stepper(() => browser, "story-page");
const dir = join(process.env.E2E_DATA_ROOT ?? ".", "exports");

const storyTab = async (name) => {
  const t = await browser.$(`//*[@role="tablist"]//*[@role="tab"][normalize-space(.)="${name}"]`);
  await t.waitForClickable({ timeout: 10_000 });
  await t.click();
  await sleep(300);
};
const LAST = "Last line, typed just before leaving.";
const FULL = [TEXT, LAST].join("\n\n");

/** Type paragraphs key by key; two Enter keys separate them. */
async function typeParagraphs(paragraphs) {
  for (const [i, p] of paragraphs.entries()) {
    if (i > 0) await browser.keys(["Enter", "Enter"]);
    await browser.keys(p.split(""));
  }
}
const area = () => browser.$('//*[@role="textbox"][@aria-label="Story text"]');
/** The page text: one line per paragraph of the editor. */
const pageValue = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label="Story text"] > p')].map((p) => p.textContent ?? "").join("\n"));
const saveState = () => browser.execute(() => document.querySelector(".story-write-foot [role=status]")?.textContent?.trim() ?? "");
const tabNames = () =>
  browser.execute(() => [...document.querySelectorAll('[role="tablist"][aria-label="Story views"] [role="tab"]')].map((t) => t.textContent.trim()));

/** Call an application operation exactly as the UI does (ipc/client.ts `call`). */
async function op(name, args) {
  return browser
    .execute(
      (n, a) =>
        window.__TAURI_INTERNALS__
          .invoke("of_invoke", { op: n, args: a })
          .then((v) => ({ ok: v }))
          .catch((e) => ({ err: e })),
      name,
      args,
    )
    .then((r) => {
      if (r.err) throw new Error(`${name} failed: ${JSON.stringify(r.err)}`);
      return r.ok;
    });
}

describe("story page", () => {
  before(async () => {
    browser = await launchApp();
    mkdirSync(dir, { recursive: true });
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("Story opens on the Story tab, which is first, on an A4 sheet that starts empty", async () => {
    await step("open", async () => {
      await firstLaunchProject(browser, "Night Pages");
      await openWorkspace(browser, "Story");
      await waitForText(browser, "Write your story freely. This page is not linked to anything else.");
      assert.deepEqual(await tabNames(), ["Story", "Board", "Outline", "Characters", "Timeline"]);
      assert.equal(await pageValue(), "");
      assert.equal(await (await area()).getAttribute("data-placeholder"), "Start writing your story here.");
      const sheet = await browser.execute(() => {
        const r = document.querySelector(".story-a4")?.getBoundingClientRect();
        return r ? r.height / r.width : 0;
      });
      assert.ok(sheet > 1.38 && sheet < 1.44, `the page is not an A4 sheet (height / width = ${sheet})`);
      const text = await browser.execute(() => document.querySelector("main")?.innerText ?? "");
      for (const gone of ["Parking Lot", "Build Screenplay", "Search / filter cards", "No scene numbers"]) {
        assert.ok(!text.includes(gone), `Beat Board control "${gone}" shows on the Story tab`);
      }
      await assertQuiet(browser, "story page empty");
    });
  });

  test("type paragraphs: the page saves them and counts the words", async () => {
    await step("type", async () => {
      await (await area()).click();
      await typeParagraphs(PARAGRAPHS);
      await browser.waitUntil(async () => (await saveState()) === "Saved", { timeout: 10_000, timeoutMsg: "the page did not report Saved" });
      assert.equal(await pageValue(), TEXT);
      const words = TEXT.split(/\s+/).length;
      await waitForText(browser, `${words} words`);
      await assertQuiet(browser, "story page typed");
    });
  });

  test("format: a selected word becomes bold and bigger, and the page zooms", async () => {
    await step("format", async () => {
      // Select the first word of the page with the keyboard and format it.
      await (await area()).click();
      await browser.keys(["Control", "Home"]);
      await browser.keys(["Control", "Shift", "ArrowRight"]);
      await click(browser, "Bold");
      const size = await browser.$('//select[@aria-label="Font size"]');
      await size.selectByAttribute("value", "24");
      await browser.waitUntil(async () => (await saveState()) === "Saved", { timeout: 10_000, timeoutMsg: "the formatting was not saved" });
      const p = await op("story.page.get", {});
      assert.equal(p.text, TEXT, "formatting changed the text");
      const runs = JSON.parse(p.format)[0];
      assert.equal(runs[0].b, true, `the first word is not bold: ${p.format}`);
      assert.equal(runs[0].s, 24, `the first word is not 24 pt: ${p.format}`);
      // Zoom in with the button, then back to 100%.
      await click(browser, "Zoom in");
      await waitForText(browser, "110%");
      await (await browser.$('//button[@title="Reset the page zoom"]')).click();
      await waitForText(browser, "100%");
      await assertQuiet(browser, "story page formatted");
    });
  });

  test("switching tabs keeps the text, and typed text is stored when leaving", async () => {
    await step("switch", async () => {
      await (await area()).click();
      await browser.keys(["Control", "End"]);
      await typeParagraphs(["", LAST]);
      await storyTab("Board");
      await waitForText(browser, "Your Beat Board is empty.");
      await storyTab("Story");
      await browser.waitUntil(async () => (await pageValue()) === FULL, { timeout: 10_000, timeoutMsg: "the text typed before leaving was lost" });
    });
  });

  test("relaunch: the text is still there", async () => {
    await step("relaunch", async () => {
      await browser.waitUntil(async () => (await saveState()) === "Saved", { timeout: 10_000 });
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Night Pages");
      await openWorkspace(browser, "Story");
      await browser.waitUntil(async () => (await pageValue()).startsWith(PARAGRAPHS[0]), { timeout: 15_000, timeoutMsg: "the Story page text is missing after the relaunch" });
      assert.equal(await pageValue(), FULL);
      await assertQuiet(browser, "story page after relaunch");
    });
  });

  test("export: the dialog offers Plain text, PDF and Word, and the files hold the text", async () => {
    await step("export", async () => {
      await click(browser, "Export");
      await waitForText(browser, "Export Story");
      const dialog = await browser.execute(() => document.querySelector('[role="dialog"]')?.innerText ?? "");
      for (const label of ["Plain text (.txt)", "PDF", "Word (.docx)"]) assert.ok(dialog.includes(label), `the Export dialog does not offer ${label}`);
      await click(browser, "Cancel", { within: '//*[@role="dialog"]' });

      const txt = join(dir, "story.txt");
      await op("story.page.export", { format: "txt", path: txt });
      const got = readFileSync(txt, "utf8").replace(/^﻿/, "");
      assert.equal(got, FULL.replace(/\n/g, "\r\n"), "the text file is not the page with Windows line endings");

      const pdf = join(dir, "story.pdf");
      await op("story.page.export", { format: "pdf", path: pdf });
      assert.ok(readFileSync(pdf, "latin1").startsWith("%PDF-"), "the PDF is not a PDF");

      const docx = join(dir, "story.docx");
      await op("story.page.export", { format: "docx", path: docx });
      assert.ok(existsSync(docx) && statSync(docx).size > 500, "the Word file was not written");
      assert.equal(readFileSync(docx).toString("latin1", 0, 2), "PK", "the Word file is not a zip package");
      await closeProject(browser);
    });
  });
});
