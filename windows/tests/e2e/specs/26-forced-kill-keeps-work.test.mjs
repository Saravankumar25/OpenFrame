// @e2e-data-root: own
//
// Prove the rows writers use every day (Windows phase W4): work survives a forced kill. Add an idea
// to the Idea Vault, an act and a Scene Card on the Story Board, a character, a project note and a
// screenplay line; wait until each one says it is saved; then end the app abruptly, as a crash or a
// power cut would (the process tree is killed, nothing gets to close). Start again, open the
// project (taking the recovery state when it is offered), and find every piece of work. Then edit
// once more to prove the project is writable after the recovery.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, pageText, sleep, stepper, waitForText } from "../lib/app.mjs";
import { editorElements, firstLaunchProject, inDialog, menuItem, projectMenu, xq } from "../lib/flows.mjs";
import { killApp } from "../lib/ai.mjs";

const TITLE = "Hard Stop";
const IDEA = "A ferry that only sails when nobody is waiting.";
const CARD = "The ferry leaves without its captain";
const CHARACTER = "Ilse";
const NOTE = "Ferry timetable";
const LINE = "The engine coughs twice and dies.";
const AFTER = "The captain swims after it.";

let browser;
const step = stepper(() => browser, "forced-kill");
const storyTab = async (name) => {
  const t = await browser.$(`//*[@role="tablist"]//*[@role="tab"][normalize-space(.)="${name}"]`);
  await t.waitForClickable({ timeout: 10_000 });
  await t.click();
  await sleep(300);
};
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label^="Scene Card:"]')].map((e) => e.getAttribute("aria-label").replace("Scene Card: ", "")));
const vaultItems = () => browser.execute(() => document.querySelector("main")?.innerText ?? "");
const notes = () => browser.execute(() => [...document.querySelectorAll('section[aria-label="Project Notes"] li button')].map((b) => b.innerText.trim()));
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");

describe("work survives a forced kill", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("add an idea, an act and a card, a character, a note and a screenplay line; each one saves", async () => {
    await step("work", async () => {
      await firstLaunchProject(browser, TITLE);

      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Quick Capture");
      await (await browser.$('//textarea[@aria-label="Idea"]')).setValue(IDEA);
      await inDialog(browser, "Save");
      await waitForText(browser, "Saved to the Project Idea Vault");

      await openWorkspace(browser, "Story");
      await click(browser, "Add Act");
      await (await browser.$('//input[@aria-label="Act title"]')).setValue("Act One");
      await browser.keys("Enter");
      await click(browser, "Add");
      await menuItem(browser, "Scene Card");
      await (await browser.$('//textarea[@aria-label="Short description"]')).setValue(CARD);
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await cards()).includes(CARD), { timeout: 10_000, timeoutMsg: "the card was not created" });

      await storyTab("Characters");
      await click(browser, "Add Character");
      await (await browser.$("#nc-name")).setValue(CHARACTER);
      await inDialog(browser, "Create Character");
      await waitForText(browser, `Added character ${CHARACTER}`);
      await storyTab("Board");

      await projectMenu(browser, "Notes & Tasks");
      await click(browser, "New Note");
      const title = await browser.$("#ne-title");
      await title.waitForDisplayed({ timeout: 10_000 });
      await title.setValue(NOTE);
      const close = await browser.$('//aside//button[@aria-label="Close"]');
      await close.click();
      await close.waitForExist({ reverse: true, timeout: 10_000 });
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith(NOTE)), { timeout: 10_000, timeoutMsg: "the note is not listed" });

      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      await browser.keys(LINE.split(""));
      await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
    });
  });

  test("the app is killed; after a new start every piece of work is there", async () => {
    await step("kill-and-recover", async () => {
      killApp();
      await browser.deleteSession().catch(() => undefined);
      await sleep(1500);
      browser = await launchApp();
      await waitForText(browser, "Your projects", 30_000);
      const row = await browser.$(`//main//button[starts-with(@aria-label,${xq(`Open ${TITLE}`)})]`);
      await row.waitForClickable({ timeout: 15_000 });
      await row.click();
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      const latest = await browser.$('//button[normalize-space(.)="Open Recovery" and not(@disabled)]');
      if (await latest.waitForDisplayed({ timeout: 5000 }).catch(() => false)) {
        await latest.click();
        await latest.waitForDisplayed({ reverse: true, timeout: 15_000 });
      }

      await openWorkspace(browser, "Idea Vault");
      await browser.waitUntil(async () => (await vaultItems()).includes("A ferry that only sails"), { timeout: 10_000, timeoutMsg: "the idea was lost" });
      await openWorkspace(browser, "Story");
      await browser.waitUntil(async () => (await cards()).includes(CARD), { timeout: 10_000, timeoutMsg: "the Scene Card was lost" });
      await storyTab("Characters");
      await browser.waitUntil(async () => (await pageText(browser)).includes(CHARACTER), { timeout: 10_000, timeoutMsg: "the character was lost" });
      await storyTab("Board");
      await projectMenu(browser, "Notes & Tasks");
      await browser.waitUntil(async () => (await notes()).some((n) => n.startsWith(NOTE)), { timeout: 10_000, timeoutMsg: "the note was lost" });
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).some((e) => e.endsWith(LINE)), { timeout: 15_000, timeoutMsg: "the screenplay line was lost" });
    });
  });

  test("after the recovery the project takes new work and saves it", async () => {
    await step("write-again", async () => {
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[last()]")).click();
      await browser.keys(["Control", "End"]);
      await browser.keys("End");
      await browser.keys("Enter");
      await browser.keys(["Control", "2"]);
      await browser.keys(AFTER.split(""));
      await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved after the recovery" });
      assert.ok((await editorElements(browser)).some((e) => e.endsWith(AFTER)));
    });
  });
});
