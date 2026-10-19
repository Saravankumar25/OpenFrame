// @e2e-data-root: own
//
// Prove the rows writers use every day (Windows phase W4): the Scene Hub, comments, and drafts with
// compare. Import a three-scene screenplay; open the Scene Hub from a scene's context menu and see
// what exists around it; comment on a scene, see the count on the Comments button, and find the
// comment again after a restart; make a second draft, change one line in it, and compare the two
// drafts side by side.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, pageText, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, editorElements, firstLaunchProject, importFountain, inDialog, menuItem, openProject, relaunch, retype } from "../lib/flows.mjs";
import { NIGHT_BUS } from "../lib/ai.mjs";

const TITLE = "Second Look";
const COMMENT = "Is the bus stop lit at night?";
let browser;
const step = stepper(() => browser, "scene-hub-comments-compare");
const toolbar = '//*[@role="toolbar"][@aria-label="Screenplay"]';
const sceneRow = (text) => browser.$(`//nav[@aria-label="Scenes"]//button[contains(normalize-space(.),"${text}")]`);
const commentsButton = () => browser.execute(() => [...document.querySelectorAll('[role="toolbar"][aria-label="Screenplay"] button')].map((b) => b.innerText.trim()).find((t) => t.startsWith("Comments")) ?? "");
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");

describe("Scene Hub, comments, drafts and compare", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the Scene Hub opens from a scene's context menu and lists what exists around the scene", async () => {
    await step("hub", async () => {
      await firstLaunchProject(browser, TITLE);
      await importFountain(browser, NIGHT_BUS);
      await (await sceneRow("BUS STOP")).click({ button: "right" });
      await menuItem(browser, "Scene Hub");
      const hub = await browser.$('//aside[.//*[contains(normalize-space(.),"What exists around this scene?")]]');
      await hub.waitForDisplayed({ timeout: 10_000 });
      assert.match(await hub.getText(), /BUS STOP/);
      await click(browser, "Close", { within: '//aside[.//*[contains(normalize-space(.),"What exists around this scene?")]]' });
      await hub.waitForExist({ reverse: true, timeout: 10_000 });
    });
  });

  test("a comment on a scene is counted on the Comments button and survives a restart", async () => {
    await step("comment", async () => {
      await (await sceneRow("BUS STOP")).click({ button: "right" });
      await menuItem(browser, "Add Comment");
      const box = await browser.$('//textarea[@aria-label="Comment"]');
      await box.waitForDisplayed({ timeout: 10_000 });
      await box.setValue(COMMENT);
      await click(browser, "Comment");
      await browser.waitUntil(async () => (await pageText(browser)).includes(COMMENT), { timeout: 10_000, timeoutMsg: "the comment is not shown" });
      await browser.waitUntil(async () => (await commentsButton()) === "Comments (1)", { timeout: 10_000, timeoutMsg: "the Comments button does not count the comment" });

      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, TITLE);
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(async () => (await commentsButton()) === "Comments (1)", { timeout: 15_000, timeoutMsg: "the comment did not survive the restart" });
      // The panel layout is remembered, so the comments panel may already be open after the restart.
      if (!(await pageText(browser)).includes(COMMENT)) await click(browser, "Comments (1)", { within: toolbar });
      await waitForText(browser, COMMENT);
    });
  });

  test("a second draft takes a change of its own, and Compare shows the two drafts side by side", async () => {
    await step("compare", async () => {
      const first = await browser.execute(() => document.querySelector('button[aria-label^="Draft: "]')?.getAttribute("aria-label") ?? "");
      await click(browser, first);
      await menuItem(browser, "New Draft");
      const name = await browser.$("#nd-name");
      await name.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, name, "Rewrite B");
      await inDialog(browser, "Create Draft");
      await waitForText(browser, "Created draft “Rewrite B”");
      const line = await browser.$("(//div[contains(@class,'spx-doc')]//div[@data-type='dialogue'])[last()]");
      await line.click();
      await browser.keys("End");
      await browser.keys(" Every night.".split(""));
      await browser.waitUntil(async () => (await editorElements(browser)).includes("dialogue: You came back. Every night."), { timeout: 10_000, timeoutMsg: "the edit did not land" });
      await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
      await click(browser, "Compare", { within: toolbar });
      const table = await browser.$('//*[@role="table"][@aria-label="Side-by-side comparison"]');
      await table.waitForDisplayed({ timeout: 15_000 });
      await browser.waitUntil(async () => (await table.getText()).includes("Every night."), { timeout: 10_000, timeoutMsg: "the comparison does not show the changed line" });
    });
  });
});
