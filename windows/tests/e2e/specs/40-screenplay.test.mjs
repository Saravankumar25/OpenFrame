// @e2e-data-root: own
//
// Screenplay (build plan item end-to-end specs per workspace): start a new screenplay, write a scene with the Enter flow
// (heading → action → character → dialogue), undo and redo typing from inside the editor, create
// a second draft and edit it independently, rename it, delete it and restore it from Recently
// Deleted; then quit, relaunch and find the text again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, editorElements, firstLaunchProject, inDialog, keepNewName, menuItem, openProject, projectMenu, relaunch, retype } from "../lib/flows.mjs";

const SCENE = [
  "scene_heading: INT. GLASS WORKSHOP — NIGHT",
  "action: Asha shapes a lens over the blue flame.",
  "character: ASHA",
  "dialogue: It has to be perfect.",
];

let browser;
const step = stepper(() => browser, "screenplay");
const switcher = async () => (await browser.$('//button[starts-with(@aria-label,"Draft: ")]')).getAttribute("aria-label");
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");
const typeKeys = (text) => browser.keys(text.split(""));
async function waitSaved() {
  await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
}
async function openDraftsDrawer() {
  await click(browser, "Drafts", { within: '//*[@role="toolbar"][@aria-label="Screenplay"]' });
  await browser.$('//aside[@aria-label="Drafts"]').waitForDisplayed({ timeout: 10_000 });
}
async function closeDraftsDrawer() {
  await click(browser, "Close", { within: '//aside[@aria-label="Drafts"]' });
}
const draftRows = () =>
  browser.execute(() =>
    [...document.querySelectorAll('aside[aria-label="Drafts"] button[aria-label^="Actions for "]')].map((b) => ({
      name: b.getAttribute("aria-label").slice("Actions for ".length),
      current: /\bCurrent\b/.test(b.closest(".li")?.innerText ?? ""),
    })),
  );

describe("screenplay", () => {
  let firstDraft = "";

  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("new screenplay: write a scene with the Enter flow (autosaved)", async () => {
    await step("write", async () => {
      await firstLaunchProject(browser, "Glass Harbour");
      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
      firstDraft = /^Draft: (.+)\. Switch draft$/.exec(await switcher())?.[1] ?? "";
      assert.ok(firstDraft, "draft switcher has no draft name");
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      await typeKeys("INT. GLASS WORKSHOP — NIGHT");
      await browser.keys("Enter");
      await typeKeys("Asha shapes a lens over the blue flame.");
      await browser.keys("Enter");
      await typeKeys("ASHA");
      await keepNewName(browser, "ASHA");
      await browser.keys("Enter");
      await typeKeys("It has to be perfect.");
      await browser.waitUntil(async () => (await editorElements(browser)).includes(SCENE[3]), { timeout: 10_000, timeoutMsg: "dialogue not typed" });
      assert.deepEqual((await editorElements(browser)).slice(0, 4), SCENE);
      await waitSaved();
      await assertQuiet(browser, "screenplay write");
    });
  });

  test("undo and redo typing from inside the editor", async () => {
    await step("undo", async () => {
      await sleep(4500); // typing more than 4 s apart is a separate undo step
      await browser.keys(" Now.".split(""));
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now."), { timeout: 10_000 });
      await waitSaved();
      await browser.keys(["Control", "z"]);
      await browser.waitUntil(async () => (await editorElements(browser)).includes(SCENE[3]), { timeout: 10_000, timeoutMsg: "undo did not remove the last typing" });
      await browser.keys(["Control", "y"]);
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now."), { timeout: 10_000, timeoutMsg: "redo did not restore the typing" });
      await waitSaved();
      await assertQuiet(browser, "editor undo");
    });
  });

  test("a second draft is edited independently, renamed, deleted and restored", async () => {
    await step("drafts", async () => {
      await click(browser, `Draft: ${firstDraft}. Switch draft`);
      await menuItem(browser, "New Draft");
      const name = await browser.$("#nd-name");
      await name.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, name, "Rewrite A");
      await inDialog(browser, "Create Draft");
      await waitForText(browser, "Created draft “Rewrite A”");
      await browser.waitUntil(async () => (await switcher()) === "Draft: Rewrite A. Switch draft", { timeout: 10_000, timeoutMsg: "the new draft did not open" });
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now."), { timeout: 10_000, timeoutMsg: "the new draft is not a copy" });
      // Edit only the new draft.
      const last = await browser.$("(//div[contains(@class,'spx-doc')]//div[@data-type='dialogue'])[last()]");
      await last.click();
      await browser.keys("End");
      await typeKeys(" Again.");
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now. Again."), { timeout: 10_000 });
      await waitSaved();
      await click(browser, "Draft: Rewrite A. Switch draft");
      await menuItem(browser, firstDraft);
      await browser.waitUntil(async () => (await switcher()) === `Draft: ${firstDraft}. Switch draft`, { timeout: 10_000, timeoutMsg: "could not switch back" });
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now."), { timeout: 10_000, timeoutMsg: "first draft text changed" });

      // Rename the new draft in the Drafts drawer.
      await openDraftsDrawer();
      await click(browser, "Actions for Rewrite A");
      await menuItem(browser, "Rename");
      const rn = await browser.$("#rd-name");
      await rn.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, rn, "Rewrite B");
      await inDialog(browser, "Save");
      await browser.waitUntil(async () => (await draftRows()).some((r) => r.name === "Rewrite B"), { timeout: 10_000, timeoutMsg: "draft not renamed" });

      // Only a draft that is not current can be deleted.
      const rows = await draftRows();
      if (rows.find((r) => r.name === "Rewrite B")?.current) {
        await click(browser, `Actions for ${firstDraft}`);
        await menuItem(browser, "Mark current");
        await browser.waitUntil(async () => (await draftRows()).find((r) => r.name === firstDraft)?.current, { timeout: 10_000 });
      }
      await click(browser, "Actions for Rewrite B");
      await menuItem(browser, "Delete");
      await inDialog(browser, "Delete Draft");
      await waitForText(browser, "Moved “Rewrite B” to Recently Deleted");
      await browser.waitUntil(async () => !(await draftRows()).some((r) => r.name === "Rewrite B"), { timeout: 10_000, timeoutMsg: "draft not deleted" });
      await closeDraftsDrawer();
      await projectMenu(browser, "Recently Deleted");
      await click(browser, "Restore Rewrite B");
      await waitForText(browser, "Restored “Rewrite B”");
      await openWorkspace(browser, "Screenplay");
      await click(browser, `Draft: ${firstDraft}. Switch draft`);
      await menuItem(browser, "Rewrite B");
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: It has to be perfect. Now. Again."), { timeout: 15_000, timeoutMsg: "the restored draft lost its text" });
      await assertQuiet(browser, "drafts");
    });
  });

  test("relaunch: both drafts and their text persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Glass Harbour");
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).length >= 4, { timeout: 15_000, timeoutMsg: "the screenplay did not open after relaunch" });
      const label = await switcher();
      const other = label === "Draft: Rewrite B. Switch draft" ? firstDraft : "Rewrite B";
      const text = label === "Draft: Rewrite B. Switch draft" ? "dialogue: It has to be perfect. Now. Again." : "dialogue: It has to be perfect. Now.";
      assert.ok((await editorElements(browser)).includes(text), `unexpected text in ${label}`);
      await click(browser, label);
      await menuItem(browser, other);
      const otherText = other === "Rewrite B" ? "dialogue: It has to be perfect. Now. Again." : "dialogue: It has to be perfect. Now.";
      await browser.waitUntil(async () => (await editorElements(browser)).includes(otherText), { timeout: 15_000, timeoutMsg: `${other} lost its text after relaunch` });
      assert.deepEqual((await editorElements(browser)).slice(0, 3), SCENE.slice(0, 3));
      await assertQuiet(browser, "screenplay after relaunch");
      await closeProject(browser);
    });
  });
});
