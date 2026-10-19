// @e2e-data-root: own
//
// Shooting schedule and Day Out of Days (masterplan area "Shooting schedule and Day Out of Days"):
// create the schedule from a Production Source, drag a scene into a day (undo and redo), move a
// scene with the menu, add a break, an off day, day times and the timeline, conflicts (Keep anyway),
// grouping suggestions that move nothing, filters that change nothing, the call sheet stale review,
// the Day Out of Days with a manual correction, reproducible exports, then quit, relaunch and find
// everything again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import {
  TWO_SCENES,
  assertQuiet,
  blur,
  closeProject,
  firstLaunchProject,
  importFountain,
  inDialog,
  mainText,
  menuItem,
  openProject,
  productionTab,
  redo,
  relaunch,
  retype,
  setNativeValue,
  undo,
  waitMain,
} from "../lib/flows.mjs";

const PROJECT = "Signal Yard";
const OUT = mkdtempSync(join(tmpdir(), "openframe-e2e-schedule-"));

let browser;
const step = stepper(() => browser, "schedule");

/** Call one application operation through the app's own IPC entry point (export paths only). */
async function op(name, args) {
  return browser.execute(
    (o, a) =>
      window.__TAURI_INTERNALS__
        .invoke("of_invoke", { op: o, args: a })
        .then((r) => ({ ok: r }))
        .catch((e) => ({ err: String(e?.message ?? JSON.stringify(e)) })),
    name,
    args,
  );
}

/** Text of a day column on the stripboard. */
async function dayColumn(label) {
  return browser.execute((l) => {
    for (const col of document.querySelectorAll(".daycol")) {
      if (col.querySelector(".of-dayname")?.textContent?.startsWith(l)) return col.innerText;
    }
    return "";
  }, label);
}

async function poolText() {
  return browser.execute(() => document.querySelector(".of-board .pool")?.innerText ?? "");
}

/** A real pointer drag (dnd-kit needs movement past its 6 px activation distance). */
async function drag(from, to) {
  const a = await from.getLocation();
  const as = await from.getSize();
  const b = await to.getLocation();
  const bs = await to.getSize();
  const sx = Math.round(a.x + as.width / 2);
  const sy = Math.round(a.y + as.height / 2);
  const tx = Math.round(b.x + bs.width / 2);
  const ty = Math.round(b.y + Math.min(bs.height - 10, 60));
  const moves = [];
  for (let i = 1; i <= 12; i++) {
    moves.push({ type: "pointerMove", duration: 40, x: Math.round(sx + ((tx - sx) * i) / 12), y: Math.round(sy + ((ty - sy) * i) / 12) });
  }
  await browser.performActions([
    {
      type: "pointer",
      id: "mouse",
      parameters: { pointerType: "mouse" },
      actions: [
        { type: "pointerMove", duration: 0, x: sx, y: sy },
        { type: "pointerDown", button: 0 },
        { type: "pointerMove", duration: 60, x: sx + 10, y: sy + 4 },
        ...moves,
        { type: "pause", duration: 150 },
        { type: "pointerUp", button: 0 },
      ],
    },
  ]);
  await browser.releaseActions();
}

async function createDay(date, { off = false } = {}) {
  await click(browser, off ? "Off Day" : "Create Shooting Day");
  if (date) await setNativeValue(browser, await browser.$("#of-day-date"), date);
  await inDialog(browser, off ? "Add Off Day" : "Create Day");
  await sleep(500);
}

async function acceptAllSuggestions(scene) {
  for (let i = 0; i < 12; i++) {
    const accept = await browser.$(`//aside[@aria-label="Scene ${scene} breakdown"]//button[starts-with(@aria-label,"Accept ")]`);
    if (!(await accept.isExisting())) return;
    await accept.click();
    await sleep(400);
  }
}

describe("shooting schedule and Day Out of Days", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("create the schedule from the Production Source", async () => {
    await step("create", async () => {
      await firstLaunchProject(browser, PROJECT);
      await importFountain(browser, TWO_SCENES);
      await openWorkspace(browser, "Breakdown");
      await click(browser, "Select Source");
      const draft = await browser.$('//*[@role="dialog"]//*[@role="radiogroup"][@aria-label="Screenplay drafts"]//*[@role="radio"]');
      await draft.waitForClickable({ timeout: 10_000 });
      await draft.click();
      await inDialog(browser, "Set as Production Source");
      await waitForText(browser, "Production Source:");
      for (const n of [1, 2]) {
        await (await browser.$(`//*[@role="listbox"][@aria-label="Scenes in the Production Source"]//*[@role="option"][${n}]`)).click();
        await browser.$(`//aside[@aria-label="Scene ${n} breakdown"]`).waitForDisplayed({ timeout: 10_000 });
        await click(browser, "Suggest Elements");
        await sleep(800);
        await acceptAllSuggestions(n);
      }
      await productionTab(browser, "Schedule");
      await click(browser, "Create Shooting Schedule");
      await inDialog(browser, "Create Schedule");
      await waitMain(browser, "2 unscheduled");
      await assertQuiet(browser, "schedule created");
    });
  });

  test("days, off day, drag a scene into a day (undo, redo), move with the menu", async () => {
    await step("drag", async () => {
      await createDay("2027-06-14");
      await createDay("2027-06-15");
      await createDay(null, { off: true });
      await waitMain(browser, "Shoot Day 2 · Tue 15 Jun 2027");
      const strip = await browser.$('//div[contains(@class,"pool")]//div[@role="button"][starts-with(@aria-label,"Scene 1,")]');
      const target = await browser.$(`//div[contains(@class,"daycol")][.//button[starts-with(normalize-space(.),"Shoot Day 1")]]`);
      await strip.waitForDisplayed({ timeout: 10_000 });
      await drag(strip, target);
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("1 scene"), { timeout: 10_000, timeoutMsg: "drag did not schedule scene 1" });
      await undo(browser);
      await browser.waitUntil(async () => /unscheduled/i.test(await poolText()) && (await dayColumn("Shoot Day 1")).includes("No scenes scheduled"), {
        timeout: 10_000,
        timeoutMsg: "undo did not return scene 1 to Unscheduled",
      });
      await redo(browser);
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("1 scene"), { timeout: 10_000, timeoutMsg: "redo did not schedule scene 1 again" });
      // Non-drag alternative: the strip menu.
      await click(browser, "Scene 2 actions");
      await menuItem(browser, "Shoot Day 1");
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("2 scenes"), { timeout: 10_000, timeoutMsg: "menu move failed" });
      assert.ok((await dayColumn("Off Day")).includes("Off day"), "the off day column is missing");
      await assertQuiet(browser, "drag and move");
    });
  });

  test("a meal break, day times and the computed timeline", async () => {
    await step("timeline", async () => {
      await click(browser, "Shoot Day 1 actions");
      await menuItem(browser, "Meal Break");
      // A break counts toward the day only with a duration (the field starts empty).
      const duration = await browser.$("#of-mk-dur");
      await duration.waitForDisplayed({ timeout: 10_000 });
      await duration.setValue("45m");
      await inDialog(browser, "Add");
      await browser.waitUntil(async () => /meal break/i.test(await dayColumn("Shoot Day 1")), { timeout: 10_000, timeoutMsg: "meal break not on the day" });
      await click(browser, "Shoot Day 1 · Mon 14 Jun 2027");
      await waitMain(browser, "What exactly are we doing on this day?");
      const crew = await browser.$('//input[@aria-label="Crew call for Shoot Day 1"]');
      await retype(browser, crew, "07:00");
      const start = await browser.$('//input[@aria-label="Shooting start for Shoot Day 1"]');
      await retype(browser, start, "08:00");
      await blur(browser);
      await browser.$('//ol[@aria-label="Timeline of Shoot Day 1"]').waitForDisplayed({ timeout: 10_000 });
      const tl = await browser.execute(() => document.querySelector('ol[aria-label="Timeline of Shoot Day 1"]')?.innerText ?? "");
      for (const want of ["07:00", "Crew call", "08:00", "Shooting starts", "Meal Break", "Estimated wrap"]) assert.ok(tl.includes(want), `timeline misses ${want}: ${tl}`);
      // A 30 minute target is shorter than the meal break alone, so the day is over its target:
      // the next step needs a real conflict to keep.
      await retype(browser, await browser.$('//input[@aria-label="Day target length"]'), "30");
      await blur(browser);
      await waitMain(browser, "Over target");
      await assertQuiet(browser, "day timeline");
      await click(browser, "Back to schedule");
    });
  });

  test("conflicts are advice: Keep anyway; grouping suggestions move nothing", async () => {
    await step("advice", async () => {
      const placed = await dayColumn("Shoot Day 1");
      assert.ok(placed.includes("2 scenes"), "both scenes are on Shoot Day 1");
      const chip = await browser.$('//button[contains(normalize-space(.),"warning")]');
      assert.ok(!/^0 warnings/.test((await chip.getText()).trim()), "the over-target day shows no warning");
      await chip.click();
      await browser.$('//aside[@aria-label="Schedule assistance"]').waitForDisplayed({ timeout: 10_000 });
      const keep = await browser.$('//aside[@aria-label="Schedule assistance"]//button[normalize-space(.)="Keep anyway"]');
      await keep.waitForClickable({ timeout: 10_000 });
      await keep.click();
      await browser.$('//aside[@aria-label="Schedule assistance"]//*[contains(normalize-space(.),"Kept anyway")]').waitForDisplayed({ timeout: 10_000 });
      await click(browser, "Close", { within: '//aside[@aria-label="Schedule assistance"]' });
      const before = await dayColumn("Shoot Day 1");
      assert.ok(before.includes("2 scenes"), "keeping a warning moved a scene");
      await click(browser, "Suggest Grouping");
      await browser.$('//aside[@aria-label="Grouping suggestions"]').waitForDisplayed({ timeout: 10_000 });
      await sleep(600);
      await click(browser, "Close", { within: '//aside[@aria-label="Grouping suggestions"]' });
      assert.equal(await dayColumn("Shoot Day 1"), before, "advice changed the schedule");
      await assertQuiet(browser, "advice");
    });
  });

  test("filters only change what is shown", async () => {
    await step("filters", async () => {
      await click(browser, "List");
      const scheduled = await browser.$('//select[@aria-label="Filter by scheduled state"]');
      await scheduled.selectByVisibleText("Unscheduled");
      await waitMain(browser, "No matching scenes.");
      await waitMain(browser, "Showing 0 of 2 scenes");
      await click(browser, "Clear filters");
      await waitMain(browser, "No matching scenes.", { present: false });
      await click(browser, "Board");
      assert.ok((await dayColumn("Shoot Day 1")).includes("2 scenes"), "a filter changed the schedule");
      await assertQuiet(browser, "filters");
    });
  });

  test("call sheet stale review follows the day times", async () => {
    await step("stale", async () => {
      await click(browser, "Shoot Day 1 actions");
      await menuItem(browser, "Create Call Sheet");
      await sleep(1500);
      await productionTab(browser, "Schedule");
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("Call sheet: Draft"), { timeout: 15_000, timeoutMsg: "call sheet chip missing" });
      await click(browser, "Shoot Day 1 · Mon 14 Jun 2027");
      const wrap = await browser.$('//input[@aria-label="Target wrap for Shoot Day 1"]');
      await retype(browser, wrap, "18:00");
      await blur(browser);
      await sleep(800);
      await click(browser, "Back to schedule");
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("Call sheet: Needs refresh"), { timeout: 10_000, timeoutMsg: "the call sheet was not flagged" });
      await undo(browser);
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("Call sheet: Draft"), { timeout: 10_000, timeoutMsg: "undo did not clear the flag" });
      await redo(browser);
      await assertQuiet(browser, "stale review");
    });
  });

  test("Day Out of Days: derived, corrected by hand, undo", async () => {
    await step("dood", async () => {
      await productionTab(browser, "Day Out of Days");
      await browser.$('//table[@aria-label="Day Out of Days"]').waitForDisplayed({ timeout: 10_000 });
      const cell = await browser.$('//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"Shoot Day 1")]');
      const label = await cell.getAttribute("aria-label");
      assert.ok(/: (SWF|SW|W)/.test(label), `cast does not work on Day 1: ${label}`);
      const off = await browser.$('//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"Shoot Day 2")]');
      await off.click();
      await menuItem(browser, "T");
      await browser.waitUntil(async () => (await mainText(browser)).includes("1 corrected by hand"), { timeout: 10_000, timeoutMsg: "correction not shown" });
      await undo(browser);
      await browser.waitUntil(async () => !(await browser.execute(() => document.querySelector('table[aria-label="Day Out of Days"]')?.innerText ?? "")).includes("T*"), {
        timeout: 10_000,
        timeoutMsg: "undo did not remove the correction",
      });
      await redo(browser);
      await browser.waitUntil(async () => (await mainText(browser)).includes("1 corrected by hand"), { timeout: 10_000, timeoutMsg: "redo lost the correction" });
      // A note on the correction: kept with the cell, read out by its label.
      await (await browser.$('//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"Shoot Day 2")]')).click();
      await menuItem(browser, "Add a note");
      await retype(browser, await browser.$("#of-dood-note"), "Flies in the night before");
      await inDialog(browser, "Save");
      await browser.waitUntil(
        async () => (await browser.$('//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"note: Flies in the night before")]').isExisting()),
        { timeout: 10_000, timeoutMsg: "the correction note is not on the cell" },
      );
      await assertQuiet(browser, "dood");
    });
  });

  test("exports: dialogs open, files are byte-reproducible", async () => {
    await step("export", async () => {
      await click(browser, "Export");
      await waitForText(browser, "Export Day Out of Days");
      await browser.keys("Escape");
      await productionTab(browser, "Schedule");
      await click(browser, "Export");
      await waitForText(browser, "Export Shooting Schedule");
      await browser.keys("Escape");
      for (const [name, file] of [
        ["schedule.export_schedule", "schedule"],
        ["schedule.export_dood", "dood"],
      ]) {
        for (const fmt of ["pdf", "xlsx"]) {
          const a = join(OUT, `${file}-a.${fmt}`);
          const b = join(OUT, `${file}-b.${fmt}`);
          const ra = await op(name, { format: fmt, path: a });
          assert.ok(ra.ok, `${name} ${fmt}: ${ra.err}`);
          await sleep(1100);
          const rb = await op(name, { format: fmt, path: b });
          assert.ok(rb.ok, `${name} ${fmt}: ${rb.err}`);
          assert.ok(existsSync(a) && existsSync(b));
          assert.ok(readFileSync(a).equals(readFileSync(b)), `${name} ${fmt} differs between two exports`);
        }
      }
      await assertQuiet(browser, "exports");
    });
  });

  test("relaunch: days, scenes, breaks, times and corrections persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, PROJECT);
      await productionTab(browser, "Schedule");
      await browser.waitUntil(async () => (await dayColumn("Shoot Day 1")).includes("2 scenes"), { timeout: 15_000, timeoutMsg: "scenes lost after relaunch" });
      const col = await dayColumn("Shoot Day 1");
      assert.ok(/meal break/i.test(col), "break lost after relaunch");
      assert.ok(col.includes("wrap"), "day times lost after relaunch");
      await productionTab(browser, "Day Out of Days");
      await browser.waitUntil(async () => (await mainText(browser)).includes("1 corrected by hand"), { timeout: 10_000, timeoutMsg: "correction lost after relaunch" });
      assert.ok(
        await browser.$('//table[@aria-label="Day Out of Days"]//tbody//button[contains(@aria-label,"note: Flies in the night before")]').isExisting(),
        "correction note lost after relaunch",
      );
      await assertQuiet(browser, "after relaunch");
      await closeProject(browser);
    });
  });
});

