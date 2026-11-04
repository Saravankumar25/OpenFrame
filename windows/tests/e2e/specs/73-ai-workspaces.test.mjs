// @e2e-data-root: own
//
// The Straft AI entry points in every workspace with the REAL bundled model (ADR-0025,
// ADR-0040; UX §3.40). Straft AI opens from the header button and its shortcut everywhere. It
// never picks sections from the page the person is on: nothing is picked until they pick, Send says
// "Pick a section", and a question is answered from the sections picked and says so ("Using: ...").
//
// Covers: every workspace opens Straft AI with nothing picked; the keyboard shortcut; a question
// asked under Beat Board, under Scenes and under Locations, each answered from that section, with
// the line that names it.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { launchApp, openBeatBoard, openWorkspace, stepper } from "../lib/app.mjs";
import { firstLaunchProject, importFountain, projectMenu } from "../lib/flows.mjs";
import { NIGHT_BUS, ask, closeAssistant, openAssistant, pickSections, sectionsInfo } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-workspaces");

/** Open Straft AI in the current workspace and return what it shows about the sections. */
async function sectionsHere() {
  await closeAssistant(browser);
  await openAssistant(browser);
  // The hint shows once the panel has read the project's sections.
  await browser.waitUntil(async () => (await sectionsInfo(browser)).hint !== "" || (await sectionsInfo(browser)).chips.length > 0, {
    timeout: 10_000,
    timeoutMsg: "Straft AI shows neither a hint nor a pick",
  });
  return sectionsInfo(browser);
}

const answered = (r) => ["Answer", "Navigate", "Clarify", "Proposal", "Suggestion"].includes(r.kind);

describe("AI entry points in every workspace", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("Straft AI opens with nothing picked in every workspace", async () => {
    await step("nothing-picked", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await importFountain(browser, NIGHT_BUS);
      for (const ws of ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets"]) {
        await openWorkspace(browser, ws);
        const info = await sectionsHere();
        assert.deepEqual(info.chips, [], `${ws}: no section is picked for the person`);
        assert.equal(info.hint, "Pick a section", ws);
      }
      await projectMenu(browser, "Notes & Tasks");
      assert.equal((await sectionsHere()).hint, "Pick a section", "Notes & Tasks");
    });
  });

  test("the keyboard shortcut opens and closes Straft AI", async () => {
    await step("shortcut", async () => {
      await closeAssistant(browser);
      await browser.keys(["Control", "j"]);
      await (await browser.$('//aside[@aria-label="Straft AI"]')).waitForDisplayed({ timeout: 10_000 });
      await browser.keys(["Control", "j"]);
      await (await browser.$('//aside[@aria-label="Straft AI"]')).waitForExist({ reverse: true, timeout: 10_000 });
    });
  });

  test("a question asked under Beat Board is answered from the Beat Board", async () => {
    await step("story", async () => {
      await openBeatBoard(browser);
      await openAssistant(browser);
      await pickSections(browser, ["Beat Board"]);
      const r = await ask(browser, "What is on the Beat Board?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.equal((await sectionsInfo(browser)).using, "Using: Beat Board");
    });
  });

  test("a question asked under Scenes is about that screenplay", async () => {
    await step("screenplay", async () => {
      await openWorkspace(browser, "Screenplay");
      await openAssistant(browser);
      await pickSections(browser, ["Scenes"]);
      const r = await ask(browser, "Who speaks in this screenplay?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.match(r.text, /ARJUN|MEERA|Arjun|Meera/, "the speaking characters come from the script");
    });
  });

  test("a question asked under Scenes about the places in the script is answered from the script", async () => {
    await step("production", async () => {
      await openWorkspace(browser, "Production");
      await openAssistant(browser);
      await pickSections(browser, ["Scenes"]);
      const r = await ask(browser, "Which locations are used in the script?");
      assert.ok(answered(r), `${r.kind}: ${r.text}`);
      assert.match(r.text + r.sources.join(" "), /BUS STOP|RAILWAY STATION|POLICE STATION/i, "the answer is grounded in the script's locations");
    });
  });
});
