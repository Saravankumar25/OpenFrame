// Quit and relaunch the real app on the same data (01-journey.test.mjs ran
// first): the project is listed on Application Home, reopens, and everything
// typed in the previous session is still there.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { launchApp, openWorkspace, stepper, takeConsoleProblems, waitForText } from "../lib/app.mjs";

let browser;
const step = stepper(() => browser, "restart");

describe("relaunch", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("reopen the project and find the screenplay text, story and call sheet", async () => {
    await step("reopen", async () => {
      await waitForText(browser, "Your projects");
      const row = await browser.$('//main//button[starts-with(@aria-label,"Open Night Bus")]');
      await row.waitForClickable({ timeout: 10_000 });
      await row.click();
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      // The previous session closed the project cleanly: no recovery question.
      await browser.pause(1000);
      assert.ok(!(await browser.execute(() => document.body.innerText)).includes("recovery state"), "unexpected recovery offer");

      // Search the text typed last session and jump to it.
      await browser.keys(["Control", "k"]);
      await browser.keys("there that night".split(""));
      const hit = await browser.$('//*[@role="option"][contains(normalize-space(.),"INT. POLICE STATION")]');
      await hit.waitForDisplayed({ timeout: 10_000 });
      await hit.click();
      await browser.waitUntil(
        async () =>
          (await browser.execute(() => [...document.querySelectorAll(".spx-doc .sp-el")].map((e) => e.textContent))).includes("I was there that night."),
        { timeout: 15_000, timeoutMsg: "the typed dialogue did not persist" },
      );

      await openWorkspace(browser, "Story");
      await browser.waitUntil(
        async () => (await browser.execute(() => document.querySelectorAll('[aria-label^="Scene Card:"]').length)) === 2,
        { timeout: 15_000, timeoutMsg: "the two Scene Cards are not on the board after relaunch" },
      );

      await openWorkspace(browser, "Call Sheets");
      await waitForText(browser, "Day 1");
      const problems = await takeConsoleProblems(browser);
      assert.deepEqual(problems, [], problems.join("\n"));
    });
  });
});
