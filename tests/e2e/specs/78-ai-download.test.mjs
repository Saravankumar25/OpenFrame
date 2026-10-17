// @e2e-data-root: own
// @e2e-ai-models: none
//
// Download OpenFrame AI after install (ADR-0026 decisions 3, 5, 7 and 10; Windows phase W8), on the
// release executable as it ships: the app is compiled with the local test host (run.mjs) and starts
// with nothing downloaded. Release-capable: no test switch.
//
// Covers the decision 10 cases a test computer can show: a first start with nothing downloaded
// shows the not-installed state and asks the host nothing; the first download with a network; an
// interrupted download resumed with a range request; a pause and a resume; a damaged file refused
// and set aside; AI answers with the host down after the download; a restart with the host down
// finds the AI and asks the host nothing; Remove takes the AI away. Install with no network and
// uninstall are installer checks (W9).

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, menuItem, openProject, waitForHome } from "../lib/flows.mjs";
import { ANSWER_TIMEOUT, ask, openAssistant } from "../lib/ai.mjs";
import { hostRequests, pins, steerHost } from "../lib/model-host.mjs";

let browser;
const step = stepper(() => browser, "ai-download");
const panel = '//aside[@aria-label="AI Assistant"]';

// The pinned set: the chat and embedding files at their Hugging Face paths on the test host.
const { files } = pins();
const CHAT = files.find((f) => f.role === "chat").path;

/** The download card wherever it shows (panel or Settings). */
const cardText = () =>
  browser.execute(() => document.querySelector('section[aria-label="Download OpenFrame AI"]')?.textContent?.replace(/\s+/g, " ").trim() ?? "");

/** Wait until the panel offers the question box: OpenFrame AI is ready. */
async function ready(timeout = ANSWER_TIMEOUT) {
  await openAssistant(browser);
  await (await browser.$(`${panel}//textarea[@aria-label="Ask or command"]`)).waitForDisplayed({
    timeout,
    timeoutMsg: `OpenFrame AI was not ready; the card says: ${await cardText()}`,
  });
}

async function waitForCard(re, timeout = 120_000) {
  await browser.waitUntil(async () => re.test(await cardText()), { timeout, interval: 250, timeoutMsg: `the download card never showed ${re}: ${await cardText()}` });
}

describe("download OpenFrame AI after install", () => {
  before(async () => {
    steerHost({});
    browser = await launchApp();
  });
  after(async () => {
    steerHost({});
    await browser?.deleteSession().catch(() => undefined);
  });

  test("before the click: the not-installed state with the size, and no request to the host", async () => {
    await step("not-installed", async () => {
      await firstLaunchProject(browser, "Harbour Lights");
      await openAssistant(browser);
      await waitForCard(/Download OpenFrame AI/, 15_000);
      const text = await cardText();
      assert.match(text, /A one-time download of [\d.]+ GB\. It needs [\d.]+ GB of free space/);
      assert.match(text, /uses the internet once/);
      assert.equal(await (await browser.$(`${panel}//textarea`)).isExisting(), false, "no question box before the download");
      assert.deepEqual(hostRequests(), [], "nothing is asked of the host before the click");
    });
  });

  test("an interrupted download resumes where it stopped and AI becomes ready", async () => {
    await step("interrupted", async () => {
      steerHost({ cut: { path: CHAT, after: 300 * 1024 * 1024 } });
      await click(browser, "Download OpenFrame AI");
      await waitForCard(/The OpenFrame AI download stopped/);
      await click(browser, "Try again");
      await ready();
      const asked = hostRequests().filter((r) => r.path === CHAT);
      assert.equal(asked[0].from, null, "the first request asks for the whole file");
      assert.ok(asked.some((r) => r.from > 0), `the second request resumes with a range: ${JSON.stringify(asked)}`);
      assert.deepEqual([...new Set(hostRequests().map((r) => r.path))].sort(), files.map((f) => f.path).sort(), "only the pinned files");
      // The app names the set after the model hashes exactly as the harness computes it.
      const active = JSON.parse(readFileSync(join(process.env.E2E_DATA_ROOT, "appdata", "models", "active.json"), "utf8"));
      assert.equal(active.version, pins().version);
    });
  });

  test("with the host down, AI answers on this computer", async () => {
    await step("offline-answer", async () => {
      steerHost({ down: true });
      const before = hostRequests().length;
      const r = await ask(browser, "Summarise this project in one sentence.");
      assert.ok(r.text.length > 0, "an answer");
      assert.equal(hostRequests().length, before, "answering asks the host nothing");
    });
  });

  test("a restart with the host down finds OpenFrame AI and asks the host nothing", async () => {
    await step("restart-offline", async () => {
      const before = hostRequests().length;
      await closeProject(browser);
      await browser.deleteSession().catch(() => undefined);
      browser = await launchApp();
      await waitForHome(browser, 30_000);
      await openProject(browser, "Harbour Lights");
      await ready(30_000);
      assert.equal(hostRequests().length, before);
    });
  });

  test("Remove takes the AI away; a damaged file is set aside; Pause and Resume continue the download", async () => {
    await step("remove-damaged-pause", async () => {
      steerHost({});
      await click(browser, "Offline AI settings");
      await menuItem(browser, "Offline AI settings");
      await click(browser, "Remove OpenFrame AI…");
      await click(browser, "Remove");
      await waitForText(browser, "OpenFrame AI was removed.");
      await waitForText(browser, "Not downloaded");
      // The host sends the language model with a changed byte: it is refused and set aside.
      steerHost({ corrupt: CHAT });
      await click(browser, "Download OpenFrame AI");
      await waitForCard(/damaged and has been set aside/, ANSWER_TIMEOUT);
      // Slowly this time, so the download can be paused, then resumed where it stopped.
      steerHost({ throttle: 40 * 1024 * 1024 });
      await click(browser, "Try again");
      // Some bytes of this download have arrived (the engine figure, not the rounded text).
      await browser.waitUntil(
        () =>
          browser.execute(() =>
            window.__TAURI_INTERNALS__
              .invoke("of_invoke", { op: "ai.download.status", args: {} })
              .then((s) => s.phase === "Downloading" && s.doneBytes > 0)
              .catch(() => false),
          ),
        { timeout: 120_000, interval: 250, timeoutMsg: "the slow download never started" },
      );
      const mark = hostRequests().length;
      await click(browser, "Pause");
      await waitForCard(/Paused at/);
      steerHost({});
      await click(browser, "Resume download");
      await waitForText(browser, "AI Ready", ANSWER_TIMEOUT);
      const chat = hostRequests()
        .slice(mark)
        .filter((r) => r.path === CHAT && r.from > 0);
      assert.ok(chat.length >= 1, `the resume asked for the rest only: ${JSON.stringify(hostRequests())}`);
      await ready(30_000);
    });
  });
});
