// @e2e-data-root: own
// @e2e-ai-bundle: own
// @e2e-debug-only: reads its AI from its own folder (OPENFRAME_AI_BUNDLE_DIR), which a release build refuses
//
// The bundled Offline AI when its files are missing or altered (ADR-0025 decisions 4 and 5; build
// plan item "Bundle the weights and tokenizer with pinned hashes"). The app reads its AI from this
// spec's own copy of the build's `ai/` folder (run.mjs `@e2e-ai-bundle: own`); files are
// hard-linked from the build and only the ones a test alters are real copies.
//
// Covers: a missing model file turns AI off with a plain "reinstall" message, no download is ever
// offered, and the rest of OpenFrame works; an altered model of the right size is caught by the
// hash check before any engine starts; an altered engine library is caught before launch; once the
// files are whole again AI answers with the real model.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, pageText, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, menuItem, openProject, waitForHome } from "../lib/flows.mjs";
import { ANSWER_TIMEOUT, alterByte, appProcesses, ask, bundledFile, goToTasks, layOutBundle, openAssistant, openTaskTitles, panelText, send } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-bundle");

const CHAT_MODEL = bundledFile((r) => /^models\/gemma[^/]*\/model\.gguf$/.test(r));
const SEARCH_MODEL = bundledFile((r) => /^models\/bge[^/]*\/model\.gguf$/.test(r));
const ENGINE_LIB = bundledFile((r) => /^runtimes\/[^/]+\/ggml-base\.dll$/.test(r));
const REINSTALL = /Reinstall OpenFrame Studio to repair it/;
const engines = () => appProcesses().filter((p) => p.name.toLowerCase() === "llama-server.exe");

/** Send a question when the panel still offers the question box (the check may already have run). */
async function askIfOpen(text) {
  const box = await browser.$('//aside[@aria-label="AI Assistant"]//textarea[@aria-label="Ask or command"]');
  if (await box.waitForDisplayed({ timeout: 5_000 }).catch(() => false)) await send(browser, text);
}

/** Quit cleanly, change the bundle, start again and open the project. */
async function restartWith(layout) {
  await closeProject(browser);
  await browser.deleteSession().catch(() => undefined);
  layout();
  browser = await launchApp();
  await waitForHome(browser, 30_000);
  await openProject(browser, "Night Bus");
}

describe("bundled Offline AI: missing, damaged, whole again", () => {
  before(async () => {
    layOutBundle({ skip: [CHAT_MODEL] });
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("a missing model file turns AI off with a plain message; nothing offers a download", async () => {
    await step("missing", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await openAssistant(browser);
      await browser.waitUntil(async () => REINSTALL.test(await panelText(browser)), { timeout: 15_000, timeoutMsg: "no repair message" });
      const text = await panelText(browser);
      assert.match(text, /missing from this installation/);
      assert.match(text, /Everything else in OpenFrame works normally\./);
      assert.equal(await (await browser.$('//aside[@aria-label="AI Assistant"]//textarea')).isExisting(), false, "no question box while AI is off");
      assert.doesNotMatch(await pageText(browser), /download/i);
      // Settings says the same and still offers no download.
      await click(browser, "Offline AI settings");
      await menuItem(browser, "Offline AI settings");
      await waitForText(browser, "Needs a reinstall");
      assert.doesNotMatch(await pageText(browser), /download offline ai|remove offline ai/i);
      // The rest of OpenFrame is unaffected.
      await goToTasks(browser);
      await click(browser, "New Task");
      const t = await browser.$("#te-title");
      await t.waitForDisplayed({ timeout: 10_000 });
      await t.setValue("Confirm pier permit");
      await click(browser, "Add Task");
      await waitForText(browser, "Added task “Confirm pier permit”");
      assert.deepEqual(await openTaskTitles(browser), ["Confirm pier permit"]);
      assert.equal(engines().length, 0, "no engine was started");
    });
  });

  test("an altered model of the right size is caught before any engine starts", async () => {
    await step("damaged-model", async () => {
      await restartWith(() => {
        layOutBundle({ copy: [SEARCH_MODEL] });
        alterByte(SEARCH_MODEL);
      });
      await openAssistant(browser);
      // The sizes match, so AI looks ready until the hash check before the first start. The answer
      // (or the panel, which switches off as soon as the check fails) says why.
      await askIfOpen("How many scenes are in the script?");
      await browser.waitUntil(async () => /changed or damaged/.test(await panelText(browser)) && REINSTALL.test(await panelText(browser)), {
        timeout: ANSWER_TIMEOUT,
        timeoutMsg: "no repair message after the check",
      });
      assert.doesNotMatch(await pageText(browser), /download/i);
      assert.equal(engines().length, 0, "no engine was started with an altered file");
    });
  });

  test("an altered engine library is caught before the engine is launched", async () => {
    await step("damaged-engine", async () => {
      await restartWith(() => {
        layOutBundle({ copy: [ENGINE_LIB] });
        alterByte(ENGINE_LIB);
      });
      await openAssistant(browser);
      await askIfOpen("How many scenes are in the script?");
      await browser.waitUntil(async () => /changed or damaged/.test(await panelText(browser)) && REINSTALL.test(await panelText(browser)), {
        timeout: ANSWER_TIMEOUT,
        timeoutMsg: "no repair message after the check",
      });
      assert.equal(engines().length, 0, "the altered engine was never launched");
    });
  });

  test("with the files whole again, AI answers with the real model", async () => {
    await step("whole-again", async () => {
      await restartWith(() => layOutBundle());
      await openAssistant(browser);
      // A plain "Create a task to ..." command is answered without the model (AI specification
      // section 19), so the request is worded to need the model.
      const r = await ask(browser, "Scouting the railway station needs a task.");
      assert.ok(["Proposal", "Answer", "Clarify"].includes(r.kind), `${r.kind}: ${r.text}`);
      assert.ok(engines().length >= 1);
    });
  });
});
