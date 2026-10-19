// Visual baseline capture (build plan item visual baseline, feature THEME-002) and visual regression (build plan item screen visual comparison).
//
// Runs after 01-journey and 02-restart in the shared journey data root, reopens the invented demo
// project "Night Bus" and walks the same 15 representative screens at a fixed 1440x900 window.
//
// Modes
//   default                    COMPARE each screen with the committed baseline PNG in
//                              docs/release/evidence/M0/M0.20-visual-baseline/ (tolerance below).
//                              On failure the screenshot and a diff PNG go to tests/e2e/artifacts/visual/.
//   E2E_UPDATE_BASELINE=1      deliberately REGENERATE the committed baseline (review the new PNGs
//                              before committing; a changed baseline needs a reason in the progress log).
//   E2E_VISUAL_UPDATE=<names>  regenerate ONLY the named screens (comma separated, file names without
//                              .png, for example "15-project-settings") and compare every other screen
//                              as in the default mode. Use it when one screen changed on purpose.
//   E2E_VISUAL_BASELINE=<dir>  capture the 15 screens into another folder (the original visual baseline capture).
//   E2E_VISUAL_PERTURB_CSS=... harness self-test only: inject this CSS before every capture to prove a
//                              colour change is caught (example: ":root{--accent:#1f6feb !important}").
//
// Tolerance (applied per screen, in device pixels):
//   * a pixel differs when any of its R, G or B channels differs by more than PIXEL_THRESHOLD (of 255),
//     which absorbs anti-aliasing noise but not a colour change;
//   * a screen fails when more than MAX_DIFF_RATIO of its compared (non-ignored) pixels differ;
//   * a screen fails outright when its size differs from the baseline (window size or display scale
//     changed: the visual baseline baseline is 2160x1350, i.e. 1440x900 at 150 % Windows display scaling).
//
// Ignore regions (IGNORE below) cover only content that changes from run to run by design:
// wall-clock and relative times, the per-run data-root path, and the transient recovery toast that
// the visual baseline capture caught on screens 02 to 04. Toasts are dismissed before every capture so they are
// never part of a new capture. Everything else is compared.
//
// Deterministic state: the Project Home "Continue" card (screen 02) and the draft the Screenplay opens
// on (screen 06) come from the writer's remembered place, which the screenplay saves on timers
// (ScreenplayView.tsx: the place 2 s after the cursor settles, the layout 500 ms after a change; a
// pending save is dropped when the workspace unmounts). The earlier specs leave that place in a
// timing-dependent state, so this spec sets it the same way on every run before the first capture,
// and every capture waits until no save indicator says "Saving…".

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { comparePixels, decodePng, encodePng } from "../lib/png.mjs";

const REPO = resolve(import.meta.dirname, "..", "..", "..");
const BASELINE_DIR = join(REPO, "docs", "release", "evidence", "M0", "M0.20-visual-baseline");
const DIFF_DIR = join(import.meta.dirname, "..", "artifacts", "visual");

export const PIXEL_THRESHOLD = 24;
export const MAX_DIFF_RATIO = 0.001; // 0.1 % of the compared pixels

const CAPTURE_DIR = process.env.E2E_VISUAL_BASELINE;
const MODE = CAPTURE_DIR ? "capture" : process.env.E2E_UPDATE_BASELINE === "1" ? "update" : "compare";
/** Screens to regenerate in compare mode (E2E_VISUAL_UPDATE); the rest are still compared. */
const UPDATE_ONLY = new Set(
  (process.env.E2E_VISUAL_UPDATE ?? "")
    .split(",")
    .map((n) => n.trim())
    .filter(Boolean),
);
const outDir = CAPTURE_DIR ? (isAbsolute(CAPTURE_DIR) ? CAPTURE_DIR : resolve(REPO, CAPTURE_DIR)) : BASELINE_DIR;
const PERTURB_CSS = process.env.E2E_VISUAL_PERTURB_CSS ?? "";

/** Wall-clock and relative times as the UI words them (app/home/format.ts: ago, dayAndTime, dayWord). */
const TIME_TEXT = "(just now|\\b\\d+ (minute|hour|day)s? ago\\b|\\b(today|yesterday)\\b|\\b\\d{1,2}:\\d{2}\\b)";

/**
 * Ignore regions per screen. `css` selects candidate elements and `text` (a case-insensitive regex)
 * keeps the innermost ones whose text matches; `rect` is a fixed region in CSS pixels. Every entry
 * names why the content is dynamic.
 */
const TOAST_BAND = {
  rect: { x: 352, y: 814, w: 736, h: 68 },
  why: "transient toast (the visual baseline capture caught the recovery toast; its timing varies run to run)",
};
const IGNORE = {
  "01-application-home": [
    { css: "main .xs.muted", text: "^Last modified ", why: "relative time (Last modified just now / N minutes ago)" },
    { css: "main .pcard .pv", why: "card preview pattern is picked from the project's random id (AppHome.tsx previewFor)" },
  ],
  "02-project-home": [
    { css: "main *", text: "Last opened ", why: "wall-clock time (Last opened Today hh:mm)" },
    { css: "main .xs.muted", text: TIME_TEXT, why: "relative time of each Recent activity entry" },
    TOAST_BAND,
  ],
  "03-idea-vault": [{ css: "main *", text: `· ${TIME_TEXT}`, why: "relative day on the idea card (Note · today)" }, TOAST_BAND],
  "04-story-board": [TOAST_BAND],
  "12-files": [{ css: "main *", text: `^${TIME_TEXT}$`, why: "relative day in the Modified column" }],
  "15-project-settings": [
    { css: 'aside[aria-label="Where this project is stored"] .selectable-text', why: "per-run data-root folder in the project path" },
    { css: 'aside[aria-label="Where this project is stored"] .xs.muted', text: "^(Created|Last changed) ", why: "wall-clock time (Created / Last changed Today hh:mm)" },
  ],
};

let browser;
const step = stepper(() => browser, "visual");
const results = [];
/** Screens written to the baseline by E2E_VISUAL_UPDATE in this run. */
const updated = [];

/**
 * E2E_VISUAL_IGNORE=<css>[;<css>...] (review aid, never in CI): also ignore these elements on every
 * screen, to prove that a deliberate change (for example the 2026-10-07 logo) is the only difference
 * before its screens are regenerated with E2E_VISUAL_UPDATE.
 */
const EXTRA_IGNORE = (process.env.E2E_VISUAL_IGNORE ?? "")
  .split(";")
  .map((css) => css.trim())
  .filter(Boolean)
  .map((css) => ({ css, why: "E2E_VISUAL_IGNORE review aid" }));

/** Resolve a screen's ignore regions to device-pixel rectangles in the current window. */
async function ignoreRects(name) {
  const rules = [...(IGNORE[name] ?? []), ...EXTRA_IGNORE];
  return browser.execute(
    (rules) => {
      const dpr = window.devicePixelRatio || 1;
      const out = [];
      for (const r of rules) {
        if (r.rect) {
          out.push({ x: r.rect.x * dpr, y: r.rect.y * dpr, w: r.rect.w * dpr, h: r.rect.h * dpr });
          continue;
        }
        const re = r.text ? new RegExp(r.text, "i") : null;
        const els = [...document.querySelectorAll(r.css)].filter((el) => {
          const b = el.getBoundingClientRect();
          if (b.width === 0 || b.height === 0) return false;
          if (!re) return true;
          if (!re.test(el.textContent ?? "")) return false;
          // innermost match only
          return ![...el.children].some((c) => re.test(c.textContent ?? ""));
        });
        for (const el of els) {
          const b = el.getBoundingClientRect();
          let right = b.right;
          // Inline text can grow ("just now" → "12 minutes ago"): extend to the containing block's edge.
          if (getComputedStyle(el).display.startsWith("inline") && el.parentElement) right = Math.max(right, el.parentElement.getBoundingClientRect().right);
          out.push({ x: b.left * dpr, y: b.top * dpr, w: (right - b.left) * dpr, h: b.height * dpr });
        }
      }
      return out;
    },
    rules.map((r) => ({ css: r.css, text: r.text, rect: r.rect })),
  );
}

/** Wait until no save indicator (status bar, screenplay header) says "Saving…". */
async function settleSaves() {
  await browser.waitUntil(async () => !(await browser.execute(() => document.body.innerText.includes("Saving…"))), {
    timeout: 15_000,
    timeoutMsg: "a save indicator still says Saving… before the capture",
  });
}

async function shot(name) {
  // Let lazy workspaces, fonts and transitions settle so captures are comparable run to run.
  await browser.execute((css) => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    if (css && !document.getElementById("e2e-perturb")) {
      const s = document.createElement("style");
      s.id = "e2e-perturb";
      s.textContent = css;
      document.head.appendChild(s);
    }
  }, PERTURB_CSS);
  await sleep(1200);
  await settleSaves();
  if (MODE !== "capture") {
    // Toasts are transient: dismiss them so a capture never depends on their timing.
    await browser.execute(() => document.querySelectorAll('.toast button[aria-label="Dismiss"]').forEach((b) => b.click()));
    await sleep(400);
  }
  if (MODE !== "compare" || UPDATE_ONLY.has(name)) {
    await browser.saveScreenshot(join(outDir, `${name}.png`));
    if (MODE === "compare") updated.push(name);
    return;
  }
  const actualPng = Buffer.from(await browser.takeScreenshot(), "base64");
  const baselineFile = join(BASELINE_DIR, `${name}.png`);
  if (!existsSync(baselineFile)) {
    results.push({ name, error: `no baseline ${baselineFile} (create it with E2E_UPDATE_BASELINE=1)` });
    return;
  }
  const baseline = decodePng(readFileSync(baselineFile));
  const actual = decodePng(actualPng);
  const ignore = await ignoreRects(name);
  const r = comparePixels(baseline, actual, { threshold: PIXEL_THRESHOLD, ignore });
  const res = { name, ignored: ignore.length, differing: r.differing, compared: r.compared, ratio: r.ratio };
  if (r.sizeMismatch) {
    res.error = `size ${actual.width}x${actual.height} differs from the baseline ${baseline.width}x${baseline.height} (window size or display scaling changed)`;
  } else if (r.ratio > MAX_DIFF_RATIO) {
    res.error = `${r.differing} of ${r.compared} pixels differ (${(r.ratio * 100).toFixed(3)} % > ${(MAX_DIFF_RATIO * 100).toFixed(3)} %)`;
  }
  if (res.error) {
    mkdirSync(DIFF_DIR, { recursive: true });
    writeFileSync(join(DIFF_DIR, `${name}.actual.png`), actualPng);
    if (r.diff) writeFileSync(join(DIFF_DIR, `${name}.diff.png`), encodePng(r.diff));
    res.artifacts = join(DIFF_DIR, `${name}.{actual,diff}.png`);
  }
  results.push(res);
}

async function projectPage(label) {
  await click(browser, "Project: Night Bus. Switch or manage projects");
  const el = await browser.$(`//*[@role="menuitem"][starts-with(normalize-space(.),"${label}")]`);
  await el.waitForClickable({ timeout: 10_000 });
  await el.click();
}

/** Set the appearance through the app's own operations, as Settings > Appearance does. */
async function setAppearance(mode, themeId) {
  const r = await browser.execute(
    (m, t) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      return invoke("of_invoke", { op: "appearance.set_mode", args: { mode: m } })
        .then(() => invoke("of_invoke", { op: "appearance.set_theme", args: { themeId: t } }))
        .then(() => ({ ok: true }), (e) => ({ err: JSON.stringify(e) }));
    },
    mode,
    themeId,
  );
  if (r.err) throw new Error(`appearance: ${r.err}`);
  // data-mode is the resolved mode (light or dark); System resolves to whatever Windows uses.
  if (mode !== "system") {
    await browser.waitUntil(async () => (await browser.execute(() => document.documentElement.dataset.mode)) === mode, {
      timeout: 10_000,
      timeoutMsg: `the window did not switch to ${mode} mode`,
    });
  }
}

describe(`visual ${MODE === "compare" ? "regression against the visual baseline baseline" : `baseline ${MODE}`}`, () => {
  before(async () => {
    mkdirSync(outDir, { recursive: true });
    browser = await launchApp();
    await browser.setWindowSize(1440, 900);
    // The default mode is System, which follows this computer's Windows setting; the baseline is
    // the default theme in Light mode, so every run captures that, whatever the machine uses.
    await setAppearance("light", "openframe-default");
  });
  after(async () => {
    if (browser) await setAppearance("system", "openframe-default").catch(() => undefined);
    await browser?.deleteSession().catch(() => undefined);
  });

  test("15 representative screens", async () => {
    await step("app home", async () => {
      await waitForText(browser, "Your projects");
      await shot("01-application-home");
      const row = await browser.$('//main//button[starts-with(@aria-label,"Open Night Bus")]');
      await row.waitForClickable({ timeout: 10_000 });
      await row.click();
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      // 02-restart quits without closing the project, so this launch may offer recovery.
      // Keep the confirmed save so every capture starts from the same state.
      const keep = await browser.$('//button[normalize-space(.)="Keep Saved Version" and not(@disabled)]');
      if (await keep.waitForDisplayed({ timeout: 4000 }).catch(() => false)) {
        await keep.click();
        await keep.waitForDisplayed({ reverse: true, timeout: 10_000 });
      }
    });
    await step("remembered place", async () => {
      // Jump to Draft 1's first scene through search (the way 02-restart does), stay until the place
      // and the layout are saved, then every run captures 02 with the same Continue card and 06 on
      // the same draft.
      await browser.keys(["Control", "k"]);
      await browser.keys("there that night".split(""));
      const hit = await browser.$('//*[@role="option"][contains(normalize-space(.),"INT. POLICE STATION")]');
      await hit.waitForDisplayed({ timeout: 10_000 });
      await hit.click();
      await browser.waitUntil(
        async () =>
          (await browser.execute(() => document.querySelector("main")?.innerText.includes("Draft 1 from Story Board") ?? false)) &&
          (await browser.execute(() => document.querySelector(".spx-doc .sp-el.cur")?.textContent ?? "")) === "INT. POLICE STATION \u2014 NIGHT",
        { timeout: 15_000, timeoutMsg: "search did not open Draft 1 at the police station scene" },
      );
      await sleep(2600);
      await settleSaves();
    });
    await step("workspaces", async () => {
      const screens = [
        ["02-project-home", "Home"],
        ["03-idea-vault", "Idea Vault"],
        ["04-story-board", "Story"],
        ["06-screenplay-editor", "Screenplay"],
        ["07-breakdown", "Breakdown"],
        ["08-production-overview", "Production"],
        ["11-call-sheets", "Call Sheets"],
        ["12-files", "Files"],
      ];
      for (const [name, ws] of screens) {
        await openWorkspace(browser, ws);
        await shot(name);
        if (ws === "Story") {
          await (await browser.$('//*[@role="tablist"]//*[@role="tab"][normalize-space(.)="Outline"]')).click();
          await shot("05-story-outline");
        }
        if (ws === "Production") {
          for (const [file, tab] of [
            ["09-production-catalog", "Catalog"],
            ["10-production-schedule", "Schedule"],
          ]) {
            const b = await browser.$(`//nav[@aria-label="Production"]//button[starts-with(normalize-space(.),"${tab}")]`);
            await b.waitForClickable({ timeout: 10_000 });
            await b.click();
            await shot(file);
          }
        }
      }
    });
    await step("project pages", async () => {
      for (const [name, page] of [
        ["13-notes-and-tasks", "Notes & Tasks"],
        ["14-recently-deleted", "Recently Deleted"],
        ["15-project-settings", "Project settings"],
      ]) {
        await projectPage(page);
        await shot(name);
      }
    });

    if (MODE === "compare") {
      for (const name of updated) console.log(`  visual updated ${name}: written to the baseline (E2E_VISUAL_UPDATE)`);
      const unknown = [...UPDATE_ONLY].filter((n) => !updated.includes(n));
      assert.deepEqual(unknown, [], "E2E_VISUAL_UPDATE names a screen this spec does not capture");
      for (const r of results) {
        const pct = r.compared ? `${((r.ratio ?? 0) * 100).toFixed(4)} %` : "-";
        console.log(`  visual ${r.error ? "FAIL" : "ok  "} ${r.name}: ${r.differing ?? "-"} px differ (${pct}), ${r.ignored ?? 0} ignore regions${r.error ? `: ${r.error}; ${r.artifacts ?? ""}` : ""}`);
      }
      assert.equal(results.length + updated.length, 15, "expected 15 compared or updated screens");
      const failed = results.filter((r) => r.error);
      assert.deepEqual(failed.map((r) => `${r.name}: ${r.error}`), [], "screens differ from the visual baseline baseline (see artifacts/visual/)");
    }
  });
});
