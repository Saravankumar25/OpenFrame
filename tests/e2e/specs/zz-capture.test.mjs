// @e2e-data-root: own
// WindowsHTML capture tour. Skipped unless E2E_CAPTURE=1:
//
//   E2E_CAPTURE=1 node tests/e2e/run.mjs --only=zz-capture --no-build
//
// Seeds the invented demo project "Night Bus" in its own data root, walks every screen and state
// it can reach at a fixed 1600x1000 window, and records each one in Light and Dark mode
// (tests/e2e/lib/capture.mjs). Then scripts/windows-html/build-windows-html.mjs writes WindowsHTML/,
// and every generated page is opened in this same WebView2 at the same size and compared with its
// screenshot; a page fails above MAX_DIFF_RATIO differing pixels. WindowsHTML/README.md gets the table.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { firstLaunchProject, inDialog, menuItem, retype } from "../lib/flows.mjs";
import { HEIGHT, MODES, RAW, REPO, WIDTH, captureSession } from "../lib/capture.mjs";
import { comparePixels, decodePng, encodePng } from "../lib/png.mjs";
import { OUT, build } from "../../../scripts/windows-html/build-windows-html.mjs";

const ENABLED = process.env.E2E_CAPTURE === "1";
const PIXEL_THRESHOLD = 24;
const MAX_DIFF_RATIO = 0.005;
const DIFF_DIR = join(import.meta.dirname, "..", "artifacts", "windows-html");

let browser;
const step = stepper(() => browser, "capture");
const cap = ENABLED ? captureSession(() => browser) : null;
const shot = (name, opts) => cap.capture(name, opts);

/** Click a control and note it as the way to the next captured screen. */
async function go(el) {
  await el.waitForClickable({ timeout: 10_000 });
  await cap.noteClick(el);
  await el.click();
}
const $ = (sel) => browser.$(sel);
const btn = (name, within = "") =>
  $(`${within}//button[not(@disabled)][normalize-space(.)="${name}" or @aria-label="${name}" or @title="${name}"] | ${within}//*[@role="menuitem" or @role="tab" or @role="option"][normalize-space(.)="${name}" or @aria-label="${name}"]`);

async function seed() {
  await step("first launch", async () => {
    await waitForText(browser, "Welcome to OpenFrame Studio", 30_000);
    await shot("first-launch-welcome");
    await firstLaunchProject(browser, "Night Bus");
    await shot("project-home-new", { home: "Home" });
  });
  await step("import the example screenplay", async () => {
    await click(browser, "Create");
    await menuItem(browser, "Import Screenplay");
    await (await $('//*[@role="radiogroup"][@aria-label="Sample script"]//*[@role="radio"][1]')).click();
    await shot("import-screenplay-sample-chosen");
    await inDialog(browser, "Preview");
    await waitForText(browser, "Import Screenplay · Preview");
    await inDialog(browser, "Import");
    await waitForText(browser, "Import complete");
    await inDialog(browser, "Open Screenplay");
    await $('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
    await sleep(800);
    await shot("screenplay-editor", { home: "Screenplay" });
    await browser.execute(() => {
      let el = document.querySelector(".spx-doc");
      while (el && el.scrollHeight <= el.clientHeight + 4) el = el.parentElement;
      if (el) el.scrollTop = 900;
    });
    await shot("screenplay-editor-scrolled");
  });
}

async function verify() {
  const results = [];
  for (const s of cap.manifest.screens) {
    for (const mode of MODES) {
      await browser.url(pathToFileURL(join(OUT, mode, `${s.id}.html`)).href);
      await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 15_000 });
      await sleep(250);
      const actualPng = Buffer.from(await browser.takeScreenshot(), "base64");
      const r = comparePixels(decodePng(readFileSync(join(RAW, mode, `${s.id}.png`))), decodePng(actualPng), { threshold: PIXEL_THRESHOLD });
      const row = { id: s.id, mode, ratio: r.sizeMismatch ? 1 : r.ratio, ok: !r.sizeMismatch && r.ratio <= MAX_DIFF_RATIO };
      if (!row.ok) {
        mkdirSync(DIFF_DIR, { recursive: true });
        writeFileSync(join(DIFF_DIR, `${mode}-${s.id}.actual.png`), actualPng);
        if (r.diff) writeFileSync(join(DIFF_DIR, `${mode}-${s.id}.diff.png`), encodePng(r.diff));
      }
      results.push(row);
    }
  }
  return results;
}

describe("WindowsHTML capture tour", { skip: !ENABLED && "set E2E_CAPTURE=1 to capture WindowsHTML" }, () => {
  before(async () => {
    browser = await launchApp();
    await browser.setWindowSize(WIDTH, HEIGHT);
  });
  after(async () => {
    await cap?.finish().catch(() => undefined);
    await browser?.deleteSession().catch(() => undefined);
  });

  test("capture, generate and verify", async () => {
    const size = await browser.execute(() => [window.innerWidth, window.innerHeight, window.devicePixelRatio, navigator.userAgent]);
    console.log(`  viewport ${size[0]}x${size[1]} at ${size[2]}x, ${size[3]}`);
    await seed();
    await cap.finish();
    const commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: REPO, encoding: "utf8" }).trim();
    build({ commit });
    const results = await verify();
    for (const r of results) console.log(`  ${r.ok ? "ok  " : "FAIL"} ${r.mode} ${r.id}: ${(r.ratio * 100).toFixed(3)} %`);
    writeFileSync(join(RAW, "verify.json"), JSON.stringify(results, null, 1));
    assert.deepEqual(results.filter((r) => !r.ok).map((r) => `${r.mode}/${r.id}`), [], "pages differ from their screenshots (see tests/e2e/artifacts/windows-html/)");
  });
});
