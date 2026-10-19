// @e2e-data-root: own
//
// The AI Assistant entry points in every workspace with the REAL bundled model (ADR-0025; AI
// spec §7.2 scope inheritance; UX §3.40). The assistant opens from the header button and its
// shortcut everywhere, inherits the scope of the place the user is in, and answers a question
// asked there with that scope.
//
// Covers: Home and Idea Vault → Whole Project; Story → Story Board; Screenplay → Current
// Scene / Current Screenplay; Breakdown, Production and Call Sheets → Production Workspace;
// Notes & Tasks → Whole Project; the keyboard shortcut; a scoped question answered in the Story
// Board, Screenplay and Production workspaces.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { launchApp, openWorkspace, stepper } from "../lib/app.mjs";
import { firstLaunchProject, importFountain, projectMenu } from "../lib/flows.mjs";
import { NIGHT_BUS, ask, closeAssistant, openAssistant, scopeInfo } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-workspaces");

/** Open the assistant in the current workspace and return its "Using: …" line. */
async function scopeHere() {
  await closeAssistant(browser);
  await openAssistant(browser);
  // The line shows once the panel has read the assistant's state.
  await browser.waitUntil(async () => (await scopeInfo(browser)).using.startsWith("Using: "), {
    timeout: 10_000,
    timeoutMsg: "the assistant shows no scope line",
  });
  return (await scopeInfo(browser)).using;
}

const answered = (r) => ["Answer", "Navigate", "Clarify", "Proposal", "Suggestion"].includes(r.kind);

describe("AI entry points in every workspace", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the assistant inherits the scope of each workspace", async () => {
    await step("scopes", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await importFountain(browser, NIGHT_BUS);
      const expected = [
        ["Home", "Using: Whole Project"],
        ["Idea Vault", "Using: Whole Project"],
        ["Story", "Using: Story Board"],
        ["Breakdown", "Using: Production Workspace"],
        ["Production", "Using: Production Workspace"],
        ["Call Sheets", "Using: Production Workspace"],
      ];
      for (const [ws, using] of expected) {
        await openWorkspace(browser, ws);
        assert.equal(await scopeHere(), using, ws);
      }
      await openWorkspace(browser, "Screenplay");
      assert.match(await scopeHere(), /^Using: Current (Scene|Screenplay)$/, "Screenplay");
      await projectMenu(browser, "Notes & Tasks");
      assert.equal(await scopeHere(), "Using: Whole Project", "Notes & Tasks");
    });
  });

  test("the keyboard shortcut opens and closes the assistant", async () => {
    await step("shortcut", async () => {
      await closeAssistant(browser);
      await browser.keys(["Control", "j"]);
      await (await browser.$('//aside[@aria-label="AI Assistant"]')).waitForDisplayed({ timeout: 10_000 });
      await browser.keys(["Control", "j"]);
      await (await browser.$('//aside[@aria-label="AI Assistant"]')).waitForExist({ reverse: true, timeout: 10_000 });
    });
  });

  test("a question asked in the Story Board uses the Story Board", async () => {
    await step("story", async () => {
      await openWorkspace(browser, "Story");
      await openAssistant(browser);
      const r = await ask(browser, "What is on the Story Board?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.equal((await scopeInfo(browser)).using, "Using: Story Board");
    });
  });

  test("a question asked in the Screenplay is about that screenplay", async () => {
    await step("screenplay", async () => {
      await openWorkspace(browser, "Screenplay");
      await openAssistant(browser);
      const sel = await browser.$('//aside[@aria-label="AI Assistant"]//select[@id="ai-scope"]');
      await sel.selectByVisibleText("Current Screenplay");
      const r = await ask(browser, "Who speaks in this screenplay?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.match(r.text, /ARJUN|MEERA|Arjun|Meera/, "the speaking characters come from the script");
    });
  });

  test("a question asked in Production uses the Production Workspace", async () => {
    await step("production", async () => {
      await openWorkspace(browser, "Production");
      await openAssistant(browser);
      const r = await ask(browser, "Which locations are used in the script?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.match(r.text + r.sources.join(" "), /BUS STOP|RAILWAY STATION|POLICE STATION/i, "the answer is grounded in the script's locations");
    });
  });
});
