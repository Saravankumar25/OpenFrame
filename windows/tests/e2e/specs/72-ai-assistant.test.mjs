// @e2e-data-root: own
//
// Straft AI assistant with the REAL bundled model (ADR-0025; AI spec; agentic RAG spec §5, §21,
// §30, §36; build plan items "Real-model end-to-end test" and "No-network test").
//
// Covers: AI ready once its models are installed (preinstalled here as a finished download leaves
// them); Settings > Straft AI shows the installed parts (components, licences, hashes, read-only
// engine folder), the privacy promise and Remove, and offers no download while installed; an
// exact answer from project data with its sources, and a source that opens its place; a meaning
// question answered from retrieved project context; a navigation command; no network use by the
// app or the engines while AI works (loopback only); Stop Straft AI frees the engines and the
// next question starts them again; the conversation survives a relaunch; the panel, docked beside
// the page, opens again after the window loads again when it was open and stays closed when closed.
//
// Deterministic sampling (run.mjs): the checks are about structure and effects, never wording.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, installConsoleProbe, launchApp, openWorkspace, pageText, sleep, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, importFountain, menuItem, openProject, relaunch } from "../lib/flows.mjs";
import { BUILD_AI_DIR, NIGHT_BUS, appProcesses, ask, closeAssistant, externalConnections, networkUse, openAssistant, panelText, pickSections, results } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "ai-assistant");

const engines = () => appProcesses().filter((p) => p.name.toLowerCase() === "llama-server.exe");
// The chat engine (the Gemma model). The embedding engine (started with --embedding) may index the
// project as soon as it opens (AI specification section 4: "the embedding sidecar starts when the
// background indexer first needs embeddings"); the chat engine starts on the first request only.
const chatEngines = () => engines().filter((p) => !/\s--embedding\b/.test(p.commandLine ?? ""));

// Nothing is picked for the person: a question is asked under the sections they chose (spec
// ai-section-scoped-assistant). These specs read the screenplay, so they pick Scenes.
const pickScenes = () => pickSections(browser, ["Scenes"]);

describe("Straft AI assistant with the bundled model", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("AI is ready once its models are installed: no setup, no model names", async () => {
    await step("ready", async () => {
      await firstLaunchProject(browser, "Night Bus");
      await importFountain(browser, NIGHT_BUS);
      await openAssistant(browser);
      await (await browser.$('//aside[@aria-label="Straft AI"]//textarea[@aria-label="Ask or command"]')).waitForDisplayed({ timeout: 15_000 });
      await waitForText(browser, "Ready");
      const text = await pageText(browser);
      assert.doesNotMatch(text, /download|install offline ai|gemma|gguf|llama|q8_0|embedding/i);
      assert.equal(chatEngines().length, 0, "the chat engine does not run before the first question");
    });
  });

  test("Settings > Straft AI shows the installed components and Remove, and offers no download while installed", async () => {
    await step("settings", async () => {
      await click(browser, "Straft AI settings");
      await menuItem(browser, "Straft AI settings");
      await (await browser.$('//section[@aria-label="Straft AI"]')).waitForDisplayed({ timeout: 15_000 });
      await waitForText(browser, "Straft AI runs on this computer after a one-time download");
      await (await browser.$('//section[@aria-label="Technical details"]//summary')).click();
      await (await browser.$('//table[@aria-label="Straft AI components"]')).waitForDisplayed({ timeout: 10_000 });
      const rows = await browser.execute(() => [...document.querySelectorAll('table[aria-label="Straft AI components"] tbody tr')].map((r) => r.innerText));
      assert.equal(rows.length, 4, rows.join("\n")); // the engine and the three models
      assert.ok(rows.some((r) => r.includes("Gemma-Terms-of-Use")), "the Gemma terms are named");
      assert.ok(rows.every((r) => /[0-9a-f]{64}/.test(r)), "every component shows its pinned SHA-256");
      const details = await browser.execute(() => document.querySelector('section[aria-label="Technical details"]')?.innerText ?? "");
      assert.match(details, /read-only/);
      assert.ok(await (await browser.$('//button[normalize-space(.)="Remove Straft AI…"]')).isDisplayed(), "Remove is offered");
      for (const gone of ["Download Straft AI", "Download Straft AI", "Update Straft AI"]) {
        assert.equal(await (await browser.$(`//button[normalize-space(.)="${gone}"]`)).isExisting(), false, `"${gone}" must not exist while installed`);
      }
    });
  });

  test("an exact answer comes from project data, with sources that open their place", async () => {
    await step("exact", async () => {
      await openWorkspace(browser, "Screenplay");
      await openAssistant(browser);
      await pickScenes();
      const r = await ask(browser, "How many scenes are in the script?");
      assert.equal(r.kind, "Answer", r.text);
      // Since ADR-0037 code counts the scenes and the model writes the sentence, and code checks that
      // it states the counted number (as a digit or a word). "Exact · from project data" marks only
      // the fact sentence code shows after three failed passes, so the guarantee is the number and
      // the counted record behind "See sources", not the mark.
      assert.match(r.text, /\b(3|three)\b/i, r.text);
      assert.ok(r.sources.length > 0, `the answer names its sources: ${r.text}`);
    });
  });

  test("a meaning question is answered from retrieved project context", async () => {
    await step("retrieval", async () => {
      await pickScenes();
      // "Which scene happens at the bus stop?" is now an exact scene search; this asks for meaning.
      const r = await ask(browser, "What happens at the bus stop?");
      assert.ok(["Answer", "Navigate"].includes(r.kind), `${r.kind}: ${r.text}`);
      assert.ok(r.sources.some((s) => /BUS STOP/i.test(s)), `the bus stop scene is a source: ${r.sources.join(" | ")}`);
      // The full check of the model files ran before the first engine start; the chat engine runs now.
      assert.ok(chatEngines().length >= 1, "the chat engine runs after a question that needs the model");
      // A source chip opens the scene it names; the chips sit under the answer's "See sources".
      await openWorkspace(browser, "Home");
      const details = await browser.$('(//aside[@aria-label="Straft AI"]//details[contains(@class,"ai-details")]/summary)[last()]');
      await details.waitForClickable({ timeout: 10_000 });
      await details.click();
      const chip = await browser.$('(//aside[@aria-label="Straft AI"]//*[contains(@class,"ai-sources")]//button[contains(normalize-space(.),"BUS STOP")])[last()]');
      await chip.waitForClickable({ timeout: 10_000 });
      await chip.click();
      await (await browser.$('//main[@aria-label="Screenplay"]')).waitForExist({ timeout: 15_000 });
    });
  });

  test("a navigation command opens the place it names", async () => {
    await step("navigate", async () => {
      await openWorkspace(browser, "Home");
      await openAssistant(browser);
      const r = await ask(browser, "Open Scene 2.");
      assert.equal(r.kind, "Navigate", `${r.kind}: ${r.text}`);
      await (await browser.$('//main[@aria-label="Screenplay"]')).waitForExist({ timeout: 15_000 });
    });
  });

  test("no network: the app and the engines use loopback only while AI works", async () => {
    await step("no-network", async () => {
      const procs = appProcesses();
      const eng = procs.filter((p) => p.name.toLowerCase() === "llama-server.exe");
      assert.ok(eng.length >= 1, "an engine is running");
      for (const e of eng) {
        assert.ok(e.path.toLowerCase().startsWith(BUILD_AI_DIR.toLowerCase()), `the engine runs from the bundled AI: ${e.path}`);
        assert.match(e.commandLine, /--host 127\.0\.0\.1/);
        assert.match(e.commandLine, /--offline/);
        assert.doesNotMatch(e.commandLine, /api[_-]?key/i, "the per-launch key is never on the command line");
      }
      // Ask once more while measuring, then check every socket the app and its engines own.
      await pickScenes();
      const r = await ask(browser, "How many scenes are in the script?");
      assert.equal(r.kind, "Answer");
      const use = networkUse(procs.map((p) => p.pid));
      const outside = externalConnections(use);
      assert.deepEqual(outside, [], `network use outside this computer: ${JSON.stringify(outside)}`);
    });
  });

  test("Stop Straft AI frees the engines; the next question starts them again", async () => {
    await step("stop", async () => {
      await openAssistant(browser);
      await click(browser, "Straft AI settings");
      await menuItem(browser, "Stop Straft AI");
      await waitForText(browser, "Straft AI stopped");
      await browser.waitUntil(async () => engines().length === 0, { timeout: 30_000, timeoutMsg: "the engines did not stop" });
      const r = await ask(browser, "How many scenes are in the script?");
      assert.equal(r.kind, "Answer", r.text);
      assert.ok(engines().length >= 1);
    });
  });

  test("the conversation is still there after a relaunch", async () => {
    await step("history", async () => {
      const before = (await results(browser)).length;
      assert.ok(before >= 4, `${before} answers so far`);
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Night Bus");
      await openAssistant(browser);
      await browser.waitUntil(async () => (await results(browser)).length >= before, { timeout: 20_000, timeoutMsg: "the conversation was not restored" });
      assert.match(await panelText(browser), /Ready/);
      await sleep(200);
    });
  });

  test("the docked panel remembers whether it is open when the window loads again", async () => {
    await step("panel-state", async () => {
      // Open or closed is a layout choice kept in the window's local storage, like the Beat Board
      // zoom. Each test session starts with fresh window storage, so a reload checks it here.
      const aside = () => browser.$('//aside[@aria-label="Straft AI"]');
      const reload = async () => {
        await browser.refresh();
        await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 30_000 });
        await installConsoleProbe(browser);
        await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
        await sleep(800);
      };
      assert.ok(await (await aside()).isExisting(), "the panel is open before the reload");
      await reload();
      await (await aside()).waitForDisplayed({ timeout: 15_000, timeoutMsg: "the open Straft AI did not open again" });
      await closeAssistant(browser);
      await reload();
      assert.equal(await (await aside()).isExisting(), false, "the closed Straft AI opened again");
    });
  });
});
