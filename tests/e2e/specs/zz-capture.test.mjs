// @e2e-data-root: own
// WindowsHTML capture tour. Skipped unless E2E_CAPTURE=1:
//
//   E2E_CAPTURE=1 node tests/e2e/run.mjs --only=zz-capture --no-build
//
// Seeds the invented demo project "Night Bus" in its own data root (tests/e2e/capture/seed.mjs),
// runs every section in tests/e2e/capture/sections/ in file-name order at a fixed 1600x1000 window,
// and records each state in Light and Dark mode (tests/e2e/lib/capture.mjs). Then
// scripts/windows-html/build-windows-html.mjs writes the pages, and every page is opened in this
// same WebView2 at the same size and compared with its screenshot; a page fails above
// MAX_DIFF_RATIO differing pixels. A full run writes WindowsHTML/ and its README table.
//
// Working on one section (several can run at once on one machine):
//   E2E_CAPTURE=1 E2E_CAPTURE_ONLY=40-screenplay E2E_PORT_BASE=4460 \
//   E2E_CAPTURE_OUT=.build-cache/capture-40 node tests/e2e/run.mjs --only=zz-capture --no-build
// A section run writes its pages next to its raw captures (<E2E_CAPTURE_OUT>/html), never WindowsHTML/.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { launchApp, sleep } from "../lib/app.mjs";
import { HEIGHT, MODES, RAW, REPO, WIDTH, captureSession, ensureSize } from "../lib/capture.mjs";
import { comparePixels, decodePng, encodePng } from "../lib/png.mjs";
import { makeKit } from "../capture/kit.mjs";
import { seed } from "../capture/seed.mjs";
import { OUT, build } from "../../../scripts/windows-html/build-windows-html.mjs";
import { writeReadme } from "../../../scripts/windows-html/readme.mjs";

const ENABLED = process.env.E2E_CAPTURE === "1";
const ONLY = (process.env.E2E_CAPTURE_ONLY ?? "").split(",").map((s) => s.trim()).filter(Boolean);
const FULL = ONLY.length === 0;
const HTML_OUT = FULL ? OUT : join(RAW, "html");
export const PIXEL_THRESHOLD = 24;
export const MAX_DIFF_RATIO = 0.005;
const MIN_SCREENS = 400;
const SECTIONS_DIR = join(import.meta.dirname, "..", "capture", "sections");
const DIFF_DIR = join(RAW, "diff");

let browser;
const cap = ENABLED ? captureSession(() => browser) : null;
const t = ENABLED ? makeKit(() => browser, cap) : null;

async function verify() {
  const results = [];
  for (const s of cap.manifest.screens) {
    for (const mode of MODES) {
      await browser.url(pathToFileURL(join(HTML_OUT, mode, `${s.id}.html`)).href);
      await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 15_000 });
      await ensureSize(browser);
      await sleep(250);
      const actualPng = Buffer.from(await browser.takeScreenshot(), "base64");
      const r = comparePixels(decodePng(readFileSync(join(RAW, mode, `${s.id}.png`))), decodePng(actualPng), { threshold: PIXEL_THRESHOLD });
      const row = { id: s.id, section: s.section, mode, ratio: r.sizeMismatch ? 1 : r.ratio, ok: !r.sizeMismatch && r.ratio <= MAX_DIFF_RATIO };
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
    const files = readdirSync(SECTIONS_DIR).filter((f) => f.endsWith(".mjs")).sort();
    const chosen = FULL ? files : files.filter((f) => ONLY.some((o) => f.includes(o)));
    assert.ok(chosen.length, `no section matches E2E_CAPTURE_ONLY=${ONLY.join(",")}`);

    cap.setSection("seed");
    await seed(t, { dataRoot: process.env.E2E_DATA_ROOT });
    for (const f of chosen) {
      const name = f.replace(/\.mjs$/, "");
      cap.setSection(name);
      console.log(`  section ${name}`);
      const run = (await import(pathToFileURL(join(SECTIONS_DIR, f)).href)).default;
      try {
        await run(t);
      } catch (e) {
        await cap.fail(`${name}-section-stopped`, e);
      }
      await t.reset().catch(() => undefined);
    }
    await cap.finish();

    const commit = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: REPO, encoding: "utf8" }).trim();
    build({ commit, from: RAW, to: HTML_OUT });
    const results = await verify();
    writeFileSync(join(RAW, "verify.json"), JSON.stringify(results, null, 1));
    const bad = results.filter((r) => !r.ok);
    for (const r of bad) console.log(`  verify FAIL ${r.mode} ${r.id}: ${(r.ratio * 100).toFixed(3)} %`);
    const m = cap.manifest;
    console.log(`  ${m.screens.length} screens captured, ${m.failed.length} failed, ${m.unreachable.length} unreachable, ${bad.length} of ${results.length} pages over the limit`);
    if (FULL) writeReadme({ manifest: m, results, commit, threshold: PIXEL_THRESHOLD, maxRatio: MAX_DIFF_RATIO, width: WIDTH, height: HEIGHT });

    assert.deepEqual(m.failed.map((f) => `${f.name}: ${f.error}`), [], "states the tour could not capture");
    assert.deepEqual(bad.map((r) => `${r.mode}/${r.id}`), [], `pages differ from their screenshots (see ${DIFF_DIR})`);
    if (FULL) assert.ok(m.screens.length >= MIN_SCREENS, `only ${m.screens.length} screens; the owner asked for at least ${MIN_SCREENS}`);
  });
});
