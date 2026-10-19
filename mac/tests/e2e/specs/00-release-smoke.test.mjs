// @e2e-data-root: own
//
// Release smoke (Windows phase W4): the short check that runs first on every build, and the first
// thing a release run proves on the hardened executable. Start, create a project, type in a
// screenplay, close, start again, reopen and find the text, ask the Offline AI a question it can
// answer from the project, and check that the app, its AI engines and its web view hold no socket
// outside this computer: the engines listen on 127.0.0.1 only. If this spec fails, the rest of a
// release run is not worth reading.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { launchApp, openWorkspace, stepper } from "../lib/app.mjs";
import { closeProject, editorElements, firstLaunchProject, importFountain, openProject, relaunch } from "../lib/flows.mjs";
import { NIGHT_BUS, appProcesses, ask, externalConnections, networkUse, webViewProcesses } from "../lib/ai.mjs";

const TITLE = "Smoke Night";
const LINE = "The driver waits one more minute.";
let browser;
const step = stepper(() => browser, "release-smoke");
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");

describe("release smoke", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("start, create a project, type in the screenplay until it says Saved, close the project", async () => {
    await step("create-and-type", async () => {
      await firstLaunchProject(browser, TITLE);
      await importFountain(browser, NIGHT_BUS);
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[last()]")).click();
      await browser.keys(["Control", "End"]);
      await browser.keys("End");
      await browser.keys("Enter");
      await browser.keys(["Control", "2"]);
      await browser.keys(LINE.split(""));
      await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
      await closeProject(browser);
    });
  });

  test("start again, reopen the project, the typed line is there", async () => {
    await step("reopen", async () => {
      browser = await relaunch(browser);
      await openProject(browser, TITLE);
      await openWorkspace(browser, "Screenplay");
      await browser.waitUntil(async () => (await editorElements(browser)).some((e) => e.endsWith(LINE)), {
        timeout: 15_000,
        timeoutMsg: `"${LINE}" did not survive the restart`,
      });
    });
  });

  test("the Offline AI answers from the project, and nothing leaves this computer", async () => {
    await step("ai-and-network", async () => {
      const answer = await ask(browser, "Which scenes take place at night?");
      assert.ok(answer.text.trim().length > 0, "the AI gave an empty answer");
      assert.ok(!["Failed", "Unavailable"].includes(answer.kind), `the AI did not answer (${answer.kind}): ${answer.text}`);
      const procs = [...appProcesses(), ...webViewProcesses()];
      const engines = procs.filter((p) => p.name.toLowerCase() === "llama-server.exe");
      assert.ok(engines.length > 0, "no Offline AI engine ran for the answer");
      const use = networkUse(procs.map((p) => p.pid));
      const outside = externalConnections(use);
      const listening = use.tcp.filter((c) => c.state === "Listen");
      const dir = join(import.meta.dirname, "..", "artifacts");
      mkdirSync(dir, { recursive: true });
      writeFileSync(
        join(dir, "release-smoke-network.json"),
        JSON.stringify({ at: new Date().toISOString(), release: process.env.E2E_RELEASE ?? "debug", processes: procs.map(({ pid, parent, name }) => ({ pid, parent, name })), use, outside }, null, 2),
      );
      assert.deepEqual(outside, [], `a socket left this computer: ${JSON.stringify(outside)}`);
      for (const l of listening) assert.ok(["127.0.0.1", "::1"].includes(l.local), `something listens beyond loopback: ${JSON.stringify(l)}`);
    });
  });
});
