// @e2e-data-root: own
//
// Protect writers' work (Windows phase W2): text typed just before the window closes is never lost.
// Type a sentence in the screenplay, close the window within 100 ms of the last key, by the window's
// close button and by Alt+F4, reopen the project and find the sentence. Before the close handler
// existed the editor still held the sentence in its 400 ms save delay and the close threw it away.
// The round repeats E2E_CLOSE_ROUNDS times (20 by default) for each way of closing.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, stepper } from "../lib/app.mjs";
import { editorElements, firstLaunchProject, openProject, relaunch } from "../lib/flows.mjs";
import { appRunning, prepareClose } from "../lib/window.mjs";

const TITLE = "Last Words";
const ROUNDS = Number(process.env.E2E_CLOSE_ROUNDS ?? 20);
const CLOSE_WITHIN_MS = 100;

let browser;
const step = stepper(() => browser, "close-keeps-typing");
const typeKeys = (text) => browser.keys(text.split(""));
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");
const hasLine = (line) => async () => (await editorElements(browser)).some((e) => e.endsWith(line));

/**
 * Start a new Action element at the end of the screenplay. Enter can make a Character element,
 * whose name suggestions would take the typed sentence; Ctrl+2 makes the new element an Action
 * first, so every round types into a plain Action line and no suggestion list ever opens.
 */
async function newLineAtEnd() {
  await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[last()]")).click();
  // The click can land inside a long line; End puts the caret after its last character, so Enter
  // never splits the earlier sentence.
  await browser.keys(["Control", "End"]);
  await browser.keys("End");
  await browser.keys("Enter");
  await browser.keys(["Control", "2"]);
}

async function waitForExit() {
  const deadline = Date.now() + 15_000;
  while (appRunning()) {
    if (Date.now() > deadline) throw new Error("the app did not end after the close");
    await new Promise((r) => setTimeout(r, 200));
  }
}

describe("closing the window keeps the last typed text", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test(`type, close within ${CLOSE_WITHIN_MS} ms by the close button and by Alt+F4, reopen, find the text (${ROUNDS} rounds)`, async () => {
    await step("project", async () => {
      await firstLaunchProject(browser, TITLE);
      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      await typeKeys("The last bus leaves at midnight.");
      await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
    });

    for (let round = 1; round <= ROUNDS; round++) {
      for (const method of ["button", "alt-f4"]) {
        const sentence = `Round ${round} ends by ${method === "button" ? "the close button" : "Alt F4"}.`;
        await step(`round ${round} ${method}`, async () => {
          const closer = await prepareClose(method);
          await newLineAtEnd();
          await typeKeys(sentence);
          const typedAt = Date.now();
          await closer.go();
          const elapsed = Date.now() - typedAt;
          assert.ok(elapsed < CLOSE_WITHIN_MS, `the close was sent ${elapsed} ms after the last key, not within ${CLOSE_WITHIN_MS} ms`);
          await waitForExit();

          browser = await relaunch(browser);
          await openProject(browser, TITLE);
          await openWorkspace(browser, "Screenplay");
          await browser.waitUntil(hasLine(sentence), {
            timeout: 15_000,
            timeoutMsg: `"${sentence}" was lost when the window closed (${method})`,
          });
        });
      }
    }
  });
});
