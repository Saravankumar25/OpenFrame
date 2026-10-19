// @e2e-data-root: own
//
// Crash safety of AI work with the REAL bundled model (ADR-0019 crash-safe Change Set apply;
// build plan items "Real-model end-to-end test" (restart) and "AI engine lifecycle").
//
// Covers: the app is killed the moment a proposal is applied (as a crash or power loss would);
// after relaunch the project holds either the whole Change Set or none of it, and the review card
// agrees with the project. The app is killed while the engine answers; no engine is left running
// (kill-on-close guard and the startup sweep), and AI works again after relaunch.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { launchApp, sleep, stepper, waitForText } from "../lib/app.mjs";
import { firstLaunchProject, xq } from "../lib/flows.mjs";
import { appProcesses, ask, enginesFromBuild, goToTasks, killApp, openAssistant, openTaskTitles, proposal, proposalButton, results } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-crash");

const engines = () => appProcesses().filter((p) => p.name.toLowerCase() === "llama-server.exe");

/** Launch again after a kill and open the project, choosing the recovery state when offered. */
async function relaunchAfterKill(title) {
  await browser.deleteSession().catch(() => undefined);
  await sleep(1500);
  browser = await launchApp();
  await waitForText(browser, "Your projects", 30_000);
  const row = await browser.$(`//main//button[starts-with(@aria-label,${xq(`Open ${title}`)})]`);
  await row.waitForClickable({ timeout: 15_000 });
  await row.click();
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
  const latest = await browser.$('//button[normalize-space(.)="Open Recovery" and not(@disabled)]');
  if (await latest.waitForDisplayed({ timeout: 5000 }).catch(() => false)) {
    await latest.click();
    await latest.waitForDisplayed({ reverse: true, timeout: 15_000 });
  }
}

describe("AI work survives a crash", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("killed while applying: after relaunch the Change Set is all there or not there at all", async () => {
    await step("kill-apply", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await openAssistant(browser);
      const r = await ask(browser, "Create a task to scout the railway station.");
      assert.equal(r.kind, "Proposal", `${r.kind}: ${r.text}`);
      const card = await proposal(browser);
      const n = Number(/(\d+) changes?/.exec(card.text)?.[1] ?? "0");
      assert.ok(n >= 1, card.text);
      await proposalButton(browser, "Apply Changes");
      killApp();
      await relaunchAfterKill("Night Bus");
      await goToTasks(browser);
      const tasks = await openTaskTitles(browser);
      assert.ok(tasks.length === 0 || tasks.length === n, `all or nothing: ${tasks.length} of ${n} changes after the crash`);
      // The review card tells the same story as the project.
      await openAssistant(browser);
      await browser.waitUntil(async () => (await results(browser)).length >= 1, { timeout: 20_000, timeoutMsg: "the conversation was not restored" });
      const after = await proposal(browser);
      assert.ok(after, "the proposal is still shown");
      if (tasks.length === n) assert.match(after.state, /Applied/, `applied in the project but the card says: ${after.state}`);
      else assert.doesNotMatch(after.state, /Applied/, `nothing in the project but the card says: ${after.state}`);
    });
  });

  test("killed while the engine answers: no engine is left running, and AI works after relaunch", async () => {
    await step("kill-engine", async () => {
      await openAssistant(browser);
      const box = await browser.$('//aside[@aria-label="AI Assistant"]//textarea[@aria-label="Ask or command"]');
      await box.waitForEnabled({ timeout: 30_000 });
      await box.setValue("Which scene happens at the railway station?");
      await browser.keys("Enter");
      await browser.waitUntil(async () => engines().length >= 1, { timeout: 120_000, timeoutMsg: "the engine did not start" });
      killApp();
      // Matched by program path, whoever started them: the app is gone, so nothing has it as parent.
      await browser.waitUntil(async () => enginesFromBuild().length === 0, { timeout: 15_000 }).catch(() => undefined);
      const left = enginesFromBuild();
      assert.deepEqual(left, [], `engines left running: ${JSON.stringify(left)}`);
      await relaunchAfterKill("Night Bus");
      const r = await ask(browser, "Create a task to check the bus timetable.");
      assert.ok(["Proposal", "Answer", "Clarify"].includes(r.kind), `AI works again: ${r.kind}: ${r.text}`);
    });
  });
});
