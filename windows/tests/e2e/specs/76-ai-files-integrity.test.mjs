// @e2e-data-root: own
// @e2e-needs-test-host: "Download again" fetches from the local model host and the spec reads its request log
//
// The Offline AI files when they are missing or damaged (ADR-0026 decisions 5 and 7; Windows phase
// W8). The models were downloaded (run.mjs installs the set the way a finished download leaves it);
// the engine is in the app. Release-capable: no test switch. A model file is removed or changed in
// this spec's own app data; the engine is changed in this spec's own copy of the build (the
// executable and its `ai/` folder, launched in place of the build).
//
// Covers: a missing model file turns AI off and offers the download again, and the rest of OpenFrame
// works; "Download again" restores it from the host; an altered model of the right size is caught
// by the hash check before any engine starts; an altered engine library is caught before launch
// and asks for a reinstall, never a download.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, pageText, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, menuItem, openProject, waitForHome } from "../lib/flows.mjs";
import { ANSWER_TIMEOUT, alterFile, appProcesses, ask, copyApp, goToTasks, modelFile, openAssistant, openTaskTitles, panelText, restoreModels, send } from "../lib/ai.mjs";
import { hostRequests, pins } from "../lib/model-host.mjs";
import { readFileSync } from "node:fs";

// The processor engine the app starts (the Vulkan build runs only after a card passes its check).
const LOCK = JSON.parse(readFileSync(new URL("../../../packaging/ai/ai-assets.lock.json", import.meta.url), "utf8"));
const CPU_ENGINE = LOCK.runtimes.find((r) => r.backend === "cpu" && r.os === "windows").runtimeId;

let browser;
const step = stepper(() => browser, "ai-files");
const BUILD_APP = process.env.E2E_APP;

const DOWNLOAD_AGAIN = /An OpenFrame AI file is missing or damaged/;
const REINSTALL = /Reinstall OpenFrame Studio to repair it/;
const panel = '//aside[@aria-label="AI Assistant"]';
const engines = () => appProcesses().filter((p) => p.name.toLowerCase() === "llama-server.exe");

/** Send a question when the panel still offers the question box (the check may already have run). */
async function askIfOpen(text) {
  const box = await browser.$(`${panel}//textarea[@aria-label="Ask or command"]`);
  if (await box.waitForDisplayed({ timeout: 5_000 }).catch(() => false)) await send(browser, text);
}

/** Quit cleanly, change the files, start again (optionally another copy of the app), open the project. */
async function restartWith(change, app = BUILD_APP) {
  await closeProject(browser);
  await browser.deleteSession().catch(() => undefined);
  change();
  process.env.E2E_APP = app;
  browser = await launchApp();
  await waitForHome(browser, 30_000);
  await openProject(browser, "Night Bus");
}

describe("Offline AI files: missing, damaged, whole again", () => {
  before(async () => {
    alterFile(modelFile("chat"), { remove: true });
    browser = await launchApp();
  });
  after(async () => {
    process.env.E2E_APP = BUILD_APP;
    await browser?.deleteSession().catch(() => undefined);
  });

  test("a missing model file turns AI off and offers the download again; the rest of OpenFrame works", async () => {
    await step("missing", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await openAssistant(browser);
      await browser.waitUntil(async () => DOWNLOAD_AGAIN.test(await panelText(browser)), { timeout: 15_000, timeoutMsg: "no download-again message" });
      const text = await panelText(browser);
      assert.match(text, /Download OpenFrame AI again/i); // the heading is shown in capitals
      assert.doesNotMatch(text, REINSTALL, "a model problem never asks for a reinstall");
      assert.equal(await (await browser.$(`${panel}//textarea`)).isExisting(), false, "no question box while AI is off");
      // Settings says the same.
      await click(browser, "Offline AI settings");
      await menuItem(browser, "Offline AI settings");
      await waitForText(browser, "Needs a new download");
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
      assert.deepEqual(hostRequests(), [], "nothing was downloaded without a click");
    });
  });

  test("Download again restores the files from the host and AI answers with the real model", async () => {
    await step("download-again", async () => {
      await openAssistant(browser);
      await click(browser, "Download again");
      await (await browser.$(`${panel}//textarea[@aria-label="Ask or command"]`)).waitForDisplayed({
        timeout: ANSWER_TIMEOUT,
        timeoutMsg: "AI did not become ready after the download",
      });
      assert.ok(hostRequests().some((r) => r.path === pins().files[0].path), "the pinned files were fetched again");
      // A plain "Create a task to ..." command is answered without the model (AI specification
      // section 19), so the request is worded to need the model.
      const r = await ask(browser, "Scouting the railway station needs a task.");
      assert.ok(["Proposal", "Answer", "Clarify"].includes(r.kind), `${r.kind}: ${r.text}`);
      assert.ok(engines().length >= 1);
    });
  });

  test("an altered model of the right size is caught before any engine starts", async () => {
    await step("damaged-model", async () => {
      await restartWith(() => alterFile(modelFile("embedding")));
      await openAssistant(browser);
      // The sizes match, so AI looks ready until the hash check before the first start. The answer
      // (or the panel, which switches off as soon as the check fails) says why.
      await askIfOpen("How many scenes are in the script?");
      await browser.waitUntil(async () => DOWNLOAD_AGAIN.test(await panelText(browser)), {
        timeout: ANSWER_TIMEOUT,
        timeoutMsg: "no download-again message after the check",
      });
      assert.doesNotMatch(await pageText(browser), REINSTALL);
      assert.equal(engines().length, 0, "no engine was started with an altered file");
    });
  });

  test("an altered engine library is caught before launch and asks for a reinstall", async () => {
    await step("damaged-engine", async () => {
      const copy = copyApp({ alter: `runtimes/${CPU_ENGINE}/ggml-base.dll` });
      // The models are whole again and this copy of the app carries a changed engine library.
      await restartWith(() => restoreModels(), copy);
      await openAssistant(browser);
      await askIfOpen("How many scenes are in the script?");
      await browser.waitUntil(async () => REINSTALL.test(await panelText(browser)), {
        timeout: ANSWER_TIMEOUT,
        timeoutMsg: "no reinstall message after the engine check",
      });
      assert.doesNotMatch(await panelText(browser), /Download OpenFrame AI/, "an engine problem never offers a download");
      assert.equal(engines().length, 0, "the altered engine was never launched");
    });
  });
});
