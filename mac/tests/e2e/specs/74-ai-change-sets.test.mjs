// @e2e-data-root: own
//
// AI Change Sets with the REAL bundled model (ADR-0025; agentic spec §7, §40; FSD §42.7–§42.15;
// build plan item "Real-model end-to-end test": propose, apply or reject, stale).
//
// Covers: a project change asked for in plain words arrives as ONE Proposed Changes card and
// changes nothing until Apply; Apply makes exactly the reviewed change and one Undo reverses all
// of it (Redo brings it back); Reject leaves the project unchanged; a proposal whose target
// changed after it was prepared is never applied blindly and must be re-checked.
//
// The effects are checked in Notes & Tasks, never in the wording of the answer.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, sleep, stepper, waitForText } from "../lib/app.mjs";
import { firstLaunchProject, redo, undo } from "../lib/flows.mjs";
import { ask, goToTasks, openAssistant, openTaskTitles, proposal, proposalButton } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-change-sets");

/** Number of changes the newest card says it will apply ("1 change" / "3 changes"). */
function changesIn(text) {
  const m = /(\d+) changes?/.exec(text);
  assert.ok(m, `the card states how many changes it applies: ${text}`);
  return Number(m[1]);
}

async function openTasks() {
  await goToTasks(browser);
  return openTaskTitles(browser);
}

describe("AI Change Sets: review, apply, undo, reject, stale", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  let proposed = 0;

  test("a requested change arrives as a proposal and nothing changes before Apply", async () => {
    await step("propose", async () => {
      await firstLaunchProject(browser, "Night Bus");
      assert.deepEqual(await openTasks(), []);
      await openAssistant(browser);
      const r = await ask(browser, "Create a task to scout the railway station.");
      assert.equal(r.kind, "Proposal", `${r.kind}: ${r.text}`);
      const card = await proposal(browser);
      assert.ok(card, "a Proposed Changes card is shown");
      assert.match(card.text, /Notes & Tasks/, "the affected area is named");
      assert.deepEqual(card.buttons.filter((b) => b === "Apply Changes" || b === "Reject").sort(), ["Apply Changes", "Reject"]);
      proposed = changesIn(card.text);
      assert.ok(proposed >= 1);
      assert.deepEqual(await openTasks(), [], "nothing changes before Apply");
    });
  });

  test("Apply makes the reviewed change; one Undo reverses all of it; Redo brings it back", async () => {
    await step("apply-undo", async () => {
      await openAssistant(browser);
      await proposalButton(browser, "Apply Changes");
      await browser.waitUntil(async () => /Applied \d+ changes?\./.test((await proposal(browser))?.state ?? ""), {
        timeout: 20_000,
        timeoutMsg: "the card does not say the change was applied",
      });
      let tasks = await openTasks();
      assert.equal(tasks.length, proposed, `exactly the reviewed change: ${tasks.join(" | ")}`);
      assert.ok(tasks.every((t) => /railway station/i.test(t)), tasks.join(" | "));
      await undo(browser);
      await browser.waitUntil(async () => (await openTaskTitles(browser)).length === 0, { timeout: 15_000, timeoutMsg: "one Undo did not reverse the whole Change Set" });
      await redo(browser);
      await browser.waitUntil(async () => (await openTaskTitles(browser)).length === proposed, { timeout: 15_000, timeoutMsg: "Redo did not bring the change back" });
      tasks = await openTaskTitles(browser);
      assert.equal(tasks.length, proposed);
    });
  });

  test("Reject leaves the project unchanged", async () => {
    await step("reject", async () => {
      const before = await openTasks();
      await openAssistant(browser);
      const r = await ask(browser, "Create a task to book the bus for the night shoot.");
      assert.equal(r.kind, "Proposal", `${r.kind}: ${r.text}`);
      await proposalButton(browser, "Reject");
      await browser.waitUntil(async () => ((await proposal(browser))?.state ?? "").includes("Rejected. Nothing was changed."), {
        timeout: 15_000,
        timeoutMsg: "the card does not say it was rejected",
      });
      assert.deepEqual(await openTasks(), before);
    });
  });

  test("a proposal whose target changed is never applied blindly and must be re-checked", async () => {
    await step("stale", async () => {
      await goToTasks(browser);
      await click(browser, "New Task");
      const t = await browser.$("#te-title");
      await t.waitForDisplayed({ timeout: 10_000 });
      await t.setValue("Confirm pier permit");
      await click(browser, "Add Task");
      await waitForText(browser, "Added task “Confirm pier permit”");
      await openAssistant(browser);
      const r = await ask(browser, "Mark the task Confirm pier permit as done.");
      assert.equal(r.kind, "Proposal", `${r.kind}: ${r.text}`);
      // The user completes the task by hand before applying the proposal.
      await goToTasks(browser);
      const box = await browser.$('//section[@aria-label="Tasks"]//input[@aria-label="Mark “Confirm pier permit” done"]');
      await box.waitForDisplayed({ timeout: 10_000 });
      await box.click();
      await waitForText(browser, "Completed task “Confirm pier permit”");
      await openAssistant(browser);
      await proposalButton(browser, "Apply Changes");
      await browser.waitUntil(async () => (await proposal(browser))?.stale === true, { timeout: 20_000, timeoutMsg: "the out-of-date proposal was not flagged" });
      const card = await proposal(browser);
      assert.ok(!card.buttons.includes("Apply Changes"), "a stale proposal offers no Apply");
      assert.ok(card.buttons.includes("Re-check & Review"), card.buttons.join(", "));
      await sleep(200);
    });
  });
});
