// @e2e-data-root: own
//
// Screenplay engine (Screenplay engine and file formats area): write with the element flow,
// insert an explicit page break (Ctrl+Enter), fill the title page and check its preview, choose
// A4 with a header in Page setup, define a custom element under Advanced and use it, see the pages
// in the page view, lock the shooting draft, start a dated revision, edit it and see the changed
// pages, export PDF and FDX, import the FDX back as a draft, print Telugu and Hindi in the PDF, move
// Insert page break to another key and use it, then quit, relaunch and continue.
//
// Exports and the re-import call the same operations the Export and Import dialogs call, with a
// path inside this spec's data root: the native Save and Open dialogs are the only steps skipped.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, openWorkspace, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, editorElements, firstLaunchProject, inDialog, keepNewName, menuItem, openProject, relaunch, retype } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "screenplay-engine");
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
async function viewMenu(item) {
  await click(browser, "View and more");
  await menuItem(browser, item);
}
async function elementMenu(item) {
  const trigger = await browser.$('//button[starts-with(@aria-label,"Element type: ")]');
  await trigger.waitForClickable({ timeout: 10_000 });
  await trigger.click();
  await menuItem(browser, item);
}
async function currentDraftId() {
  const ov = await op("screenplay.overview", {});
  return ov.drafts.find((d) => d.isCurrent).id;
}

describe("screenplay engine", () => {
  const exportDir = join(process.env.E2E_DATA_ROOT ?? ".", "exports");

  before(async () => {
    browser = await launchApp();
    mkdirSync(exportDir, { recursive: true });
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("write with the element flow and insert a page break", async () => {
    await step("write", async () => {
      await firstLaunchProject(browser, "Signal Box");
      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      await typeKeys("INT. SIGNAL BOX - NIGHT");
      await browser.keys("Enter");
      await typeKeys("The telephone rings twice.");
      await browser.keys("Enter");
      await typeKeys("ASHA");
      await keepNewName(browser, "ASHA");
      await browser.keys("Enter");
      await typeKeys("Who is there?");
      await browser.keys(["Control", "Enter"]);
      await typeKeys("Nobody answers.");
      await browser.waitUntil(async () => (await editorElements(browser)).some((e) => e.startsWith("page_break")), { timeout: 10_000, timeoutMsg: "no page break" });
      const els = await editorElements(browser);
      const pb = els.findIndex((e) => e.startsWith("page_break"));
      assert.equal(els[pb - 1], "dialogue: Who is there?");
      assert.equal(els[pb + 1], "dialogue: Nobody answers.");
      await waitSaved();
      // The status line counts two pages: the page break starts page 2.
      await waitForText(browser, "2 pages");
      await assertQuiet(browser, "write with page break");
    });
  });

  test("title page fields with a live preview", async () => {
    await step("title page", async () => {
      await viewMenu("Title page");
      const date = await browser.$("#tp-date");
      await date.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, date, "4 October 2026");
      await retype(browser, await browser.$("#tp-phone"), "+91 98765 43210");
      await browser.waitUntil(
        async () => (await browser.execute(() => document.querySelector(".spx-tp-preview")?.innerText ?? "")).includes("4 October 2026"),
        { timeout: 10_000, timeoutMsg: "the title page preview did not show the date" },
      );
      await inDialog(browser, "Save");
      await waitForText(browser, "Saved the title page");
    });
  });

  test("page setup: A4 with a header", async () => {
    await step("page setup", async () => {
      await viewMenu("Page setup");
      const paper = await browser.$("#ps-paper");
      await paper.waitForDisplayed({ timeout: 10_000 });
      await paper.selectByAttribute("value", "A4");
      await retype(browser, await browser.$("#ps-header"), "{title} {draft}");
      await inDialog(browser, "Save");
      await waitForText(browser, "Changed the page setup");
      const p = await op("screenplay.pagination", { draftId: await currentDraftId() });
      assert.equal(p.paper, "A4");
      assert.equal(p.linesPerPage, 59);
    });
  });

  test("a custom element under Advanced, used in the script", async () => {
    await step("custom element", async () => {
      await viewMenu("Custom elements");
      await click(browser, "Add custom element");
      await retype(browser, await browser.$("#ce-name"), "Telephone");
      await (await browser.$("#ce-look")).selectByAttribute("value", "dialogue");
      await (await browser.$("#ce-short")).selectByAttribute("value", "Ctrl+Alt+1");
      await inDialog(browser, "Add");
      await waitForText(browser, "Added the custom element “Telephone”");
      await inDialog(browser, "Done");
      // Turn the action line into the custom element from the element menu.
      const line = await browser.$("//div[contains(@class,'spx-doc')]//div[@data-type='action'][contains(.,'telephone rings')]");
      await line.click();
      await elementMenu("Telephone");
      await browser.waitUntil(async () => (await editorElements(browser)).includes("custom: The telephone rings twice."), { timeout: 10_000, timeoutMsg: "the element did not become Telephone" });
      // Tab leaves a custom element as it is (only the element menu and shortcuts change it).
      await browser.keys("Tab");
      await browser.pause(300);
      assert.ok((await editorElements(browser)).includes("custom: The telephone rings twice."), "Tab kept the custom element");
      await waitSaved();
      await viewMenu("Page view");
      await browser.$('//aside[@aria-label="Page view"]').waitForDisplayed({ timeout: 10_000 });
      await browser.waitUntil(
        async () => (await browser.execute(() => document.querySelector('aside[aria-label="Page view"]')?.innerText ?? "")).includes("The telephone rings twice."),
        { timeout: 10_000, timeoutMsg: "the page view did not show the custom element" },
      );
      const text = await browser.execute(() => document.querySelector('aside[aria-label="Page view"]')?.innerText ?? "");
      assert.ok(text.includes("Page 2"), "the page view shows the page break's second page");
      await click(browser, "Close", { within: '//aside[@aria-label="Page view"]' });
      await assertQuiet(browser, "custom element");
    });
  });

  test("lock, start a dated revision, edit it and see the changed pages", async () => {
    await step("lock and revise", async () => {
      await click(browser, "Lock", { within: '//*[@role="toolbar"][@aria-label="Screenplay"]' });
      await inDialog(browser, "Lock as Shooting Draft");
      await waitForText(browser, "as the Shooting Draft");
      await click(browser, "Revisions", { within: '//*[@role="toolbar"][@aria-label="Screenplay"]' });
      await click(browser, "Start Revision", { within: '//aside[@aria-label="Revisions"]' });
      await (await browser.$("#rv-date")).waitForDisplayed({ timeout: 10_000 });
      await browser.execute(() => {
        const el = document.querySelector("#rv-date");
        const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set;
        set.call(el, "2026-10-04");
        el.dispatchEvent(new Event("input", { bubbles: true }));
      });
      await inDialog(browser, "Start Revision");
      await waitForText(browser, "stays locked");
      const dialogue = await browser.$("//div[contains(@class,'spx-doc')]//div[@data-type='dialogue'][contains(.,'Nobody answers')]");
      await dialogue.click();
      await browser.keys("End");
      await typeKeys(" Again.");
      await waitSaved();
      const draftId = await currentDraftId();
      const pages = await op("screenplay.revision_pages", { draftId });
      assert.deepEqual(pages.changedPages, [2], "only the second page changed");
      assert.equal(pages.revisionDate, "2026-10-04");
      await click(browser, "Revisions", { within: '//*[@role="toolbar"][@aria-label="Screenplay"]' });
      await waitForText(browser, "Changed pages:");
      await click(browser, "Close", { within: '//aside[@aria-label="Revisions"]' });
      await assertQuiet(browser, "revision");
    });
  });

  test("export PDF and FDX, import the FDX back as a draft", async () => {
    await step("export and import", async () => {
      const draftId = await currentDraftId();
      const pdf = join(exportDir, "signal-box.pdf");
      const fdx = join(exportDir, "signal-box.fdx");
      const r = await op("screenplay.export", { draftId, format: "pdf", path: pdf, options: { titlePage: true, sceneNumbers: true } });
      assert.equal(r.pages, 2);
      assert.ok(existsSync(pdf) && statSync(pdf).size > 500);
      assert.ok(!readFileSync(pdf, "latin1").includes("OpenFrame"), "screenplay PDF carries no branding");
      await op("screenplay.export", { draftId, format: "fdx", path: fdx, options: { titlePage: true, sceneNumbers: true } });
      const xml = readFileSync(fdx, "utf8");
      assert.ok(xml.includes('Type="TELEPHONE"') && xml.includes('StartsNewPage="Yes"') && xml.includes('Number="1"'));
      const ov = await op("screenplay.overview", {});
      const prev = await op("screenplay.import_preview", { path: fdx });
      const rep = await op("screenplay.import_apply", { previewId: prev.previewId, mode: "new_draft", screenplayId: ov.screenplay.id, draftName: "From Final Draft" });
      const doc = await op("screenplay.document", { draftId: rep.draftId });
      const els = doc.scenes.flatMap((s) => s.elements);
      assert.ok(els.some((e) => e.elementType === "custom" && e.text === "The telephone rings twice."));
      assert.ok(els.some((e) => e.elementType === "page_break"));
      assert.equal(doc.scenes[0].label, "1");
    });
  });

  test("Telugu and Hindi print in the PDF; the page break follows a remapped shortcut", async () => {
    await step("indian scripts and shortcuts", async () => {
      const draftId = await currentDraftId();
      const doc = await op("screenplay.document", { draftId });
      const sceneId = doc.scenes[0].id;
      await op("screenplay.apply_edits", { draftId, ops: [
        { op: "insertElement", sceneId, elementType: "action", text: "మీరా రైలు స్టేషన్‌లో వేచి ఉంది. मीरा ने कहा, “क्षमा करें।”" },
      ] });
      await browser.waitUntil(async () => (await editorElements(browser)).some((e) => e.includes("మీరా రైలు")), { timeout: 10_000, timeoutMsg: "the Telugu line did not appear" });
      const pdf = join(exportDir, "signal-box-te.pdf");
      await op("screenplay.export", { draftId, format: "pdf", path: pdf, options: { titlePage: false } });
      const raw = readFileSync(pdf, "latin1");
      assert.ok(raw.includes("NotoSansTelugu") && raw.includes("NotoSansDevanagari"), "Indian-script fonts are embedded");
      // Move Insert page break to Ctrl+Shift+Enter in Settings' store, then use it in the editor.
      await op("app.set_shortcuts", { changes: [{ commandId: "screenplay.page_break", keys: ["Mod+Shift+Enter"] }] });
      const before = (await editorElements(browser)).filter((e) => e.startsWith("page_break")).length;
      await (await browser.$("//div[contains(@class,'spx-doc')]//div[@data-type='action'][contains(.,'మీరా రైలు')]")).click();
      await browser.keys("End");
      await browser.keys(["Control", "Enter"]);
      await browser.pause(400);
      assert.equal((await editorElements(browser)).filter((e) => e.startsWith("page_break")).length, before, "the old key no longer inserts a page break");
      await browser.keys(["Control", "Shift", "Enter"]);
      await browser.waitUntil(async () => (await editorElements(browser)).filter((e) => e.startsWith("page_break")).length === before + 1, { timeout: 10_000, timeoutMsg: "the remapped key did not insert a page break" });
      await op("app.set_shortcuts", { changes: [{ commandId: "screenplay.page_break", keys: null }] });
      await waitSaved();
      await assertQuiet(browser, "indian scripts and shortcuts");
    });
  });

  test("relaunch: page setup, custom element and revision text persist; continue writing", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Signal Box");
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: Nobody answers. Again."), { timeout: 15_000, timeoutMsg: "the revision text did not persist" });
      const ov = await op("screenplay.overview", {});
      const setup = await op("screenplay.page_setup", { screenplayId: ov.screenplay.id });
      assert.equal(setup.setup.paper, "A4");
      assert.equal(setup.setup.header, "{title} {draft}");
      // Continue: the custom element's shortcut still works.
      const dialogue = await browser.$("//div[contains(@class,'spx-doc')]//div[@data-type='dialogue'][contains(.,'Again')]");
      await dialogue.click();
      await browser.keys("End");
      await browser.keys("Enter");
      await typeKeys("Static on the line.");
      await browser.keys(["Control", "Alt", "1"]);
      await browser.waitUntil(async () => (await editorElements(browser)).includes("custom: Static on the line."), { timeout: 10_000, timeoutMsg: "Ctrl+Alt+1 did not apply Telephone" });
      await waitSaved();
      await assertQuiet(browser, "after relaunch");
      await closeProject(browser);
    });
  });
});
