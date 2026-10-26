// @e2e-data-root: own
//
// Call sheets, budget, templates, samples and exports (masterplan area "Call sheets, budget,
// templates, samples and exports", box "End-to-end test"): open a shipped sample script, plan a
// shooting day, create a call sheet with its six default sections, add a section, see "Needs
// input", change the schedule and review the update, finalize (a frozen snapshot), start a new
// revision; budget Planned and Actual; a call sheet made from the Indie Call Sheet template;
// sides (pages of the script, no brand), a production report, the moodboard, storyboard sheet and
// shot list exports with the OpenFrame brand line; notes and tasks; then quit, relaunch and find everything again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import {
  assertQuiet,
  blur,
  closeProject,
  firstLaunchProject,
  inDialog,
  mainText,
  menuItem,
  openProject,
  productionTab,
  projectMenu,
  relaunch,
  retype,
  undo,
  waitMain,
} from "../lib/flows.mjs";

const PROJECT = "Lost Property Unit";
const OUT = mkdtempSync(join(tmpdir(), "openframe-e2e-callsheets-"));
const BRAND = "Created using OpenFrame";

let browser;
const step = stepper(() => browser, "callsheets");
let dayId = "";
let callSheetId = "";

/** Call one application operation through the app's own IPC entry point (setup and export paths). */
async function op(name, args) {
  const r = await browser.execute(
    (o, a) =>
      window.__TAURI_INTERNALS__
        .invoke("of_invoke", { op: o, args: a })
        .then((v) => ({ ok: v }))
        .catch((e) => ({ err: String(e?.message ?? JSON.stringify(e)) })),
    name,
    args,
  );
  if (r.err) throw new Error(`${name}: ${r.err}`);
  return r.ok;
}

/** A PDF's raw bytes as Latin-1 text (the brand line is plain WinAnsi text in the content). */
function pdfText(path) {
  return readFileSync(path).toString("latin1");
}

async function exportAndCheck(name, args, file, branded = true) {
  const path = join(OUT, file);
  await op(name, { ...args, path });
  assert.ok(existsSync(path), `${name} wrote no file`);
  const head = readFileSync(path).subarray(0, 5).toString("latin1");
  assert.equal(head, "%PDF-", `${name} did not write a PDF`);
  assert.equal(pdfText(path).includes(BRAND), branded, `${name} PDF: brand line expected ${branded}`);
  return path;
}

describe("call sheets, budget, templates, samples and exports", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("open a sample script that ships with the app", async () => {
    await step("sample", async () => {
      await firstLaunchProject(browser, PROJECT);
      await click(browser, "Create");
      await menuItem(browser, "Import Screenplay");
      const sample = await browser.$('//*[@role="radiogroup"][@aria-label="Sample script"]//button[normalize-space(.)="Example Short"]');
      await sample.waitForClickable({ timeout: 10_000 });
      await sample.click();
      await inDialog(browser, "Preview");
      await waitForText(browser, "Import Screenplay · Preview");
      // The preview's "Scenes" figure: the sample holds ten scenes.
      await browser.waitUntil(async () => /\bScenes\s*\n\s*10\b/.test(await (await browser.$('//*[@role="dialog"]')).getText()), {
        timeout: 15_000,
        timeoutMsg: "the preview does not count 10 scenes",
      });
      await inDialog(browser, "Import");
      await waitForText(browser, "Import complete");
      await inDialog(browser, "Open Screenplay");
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
      await assertQuiet(browser, "sample import");
    });
  });

  test("plan two shooting days from the Production Source", async () => {
    await step("plan", async () => {
      const sources = await op("interchange.sources", {});
      const draft = sources.screenplays[0].drafts[0].id;
      await op("production.set_source", { draftId: draft, reason: null });
      const sched = await op("schedule.create", {});
      const scheduleId = sched.id ?? sched;
      dayId = await op("schedule.create_day", { scheduleId, date: "2027-06-14" });
      await op("schedule.create_day", { scheduleId, date: "2027-06-15" });
      const view = await op("schedule.get", {});
      const strips = view.unscheduled.slice(0, 3);
      for (const s of strips.slice(0, 2)) await op("schedule.move_strip", { stripId: s.id, dayId });
      // Break the two scheduled scenes down (suggest, then accept), as the Breakdown screen does,
      // so their cast reaches the call sheet and the Day Out of Days.
      for (const s of strips.slice(0, 2)) {
        await op("breakdown.suggest", { sceneId: s.sceneId });
        const detail = await op("breakdown.scene", { sceneId: s.sceneId });
        const ids = detail.groups.flatMap((g) => g.suggestions.map((e) => e.id));
        if (ids.length) await op("breakdown.accept_many", { ids });
      }
      const cast = await op("schedule.dood", {});
      assert.ok(cast.rows.length > 0, "the planned scenes have no cast");
      await op("schedule.add_marker", { dayId, markerType: "Meal", durationMinutes: 45, index: 1 });
      await op("schedule.set_day_times", { dayId, crewCall: 420, shootingStart: 480, targetWrap: 1140 });
      await productionTab(browser, "Schedule");
      await waitMain(browser, "Shoot Day 1");
      await assertQuiet(browser, "planning");
    });
  });

  test("create a call sheet: six default sections, Needs input, Add Section", async () => {
    await step("create", async () => {
      await openWorkspace(browser, "Call Sheets");
      await click(browser, "Create Call Sheet");
      await inDialog(browser, "Create Call Sheet");
      await browser.$('//*[@aria-label="Call sheet document"]').waitForDisplayed({ timeout: 15_000 });
      const paper = async () => browser.execute(() => document.querySelector('[aria-label="Call sheet document"]')?.innerText ?? "");
      const text = await paper();
      for (const s of ["PRODUCTION HEADER", "DAY AND TIMES", "LOCATIONS", "SCENE SCHEDULE", "TALENT", "PRACTICAL NOTES"]) {
        assert.ok(text.toUpperCase().includes(s), `default section ${s} is missing`);
      }
      assert.ok(!text.toUpperCase().includes("WEATHER"), "an optional section shows before it is added");
      const need = await browser.$$('//*[@aria-label="Call sheet document"]//*[@placeholder="Needs input"]');
      assert.ok(need.length > 0, "values nobody entered do not read Needs input");
      const crew = await browser.$('//input[@aria-label="Crew call"]');
      assert.equal(await crew.getValue(), "07:00", "the crew call does not come from the day");
      // Keyboard: open Add Section with Enter and pick Weather.
      const add = await browser.$('//button[normalize-space(.)="Add Section"]');
      await add.click();
      await menuItem(browser, "Weather");
      await browser.waitUntil(async () => (await paper()).toUpperCase().includes("WEATHER"), { timeout: 10_000, timeoutMsg: "Weather was not added" });
      await retype(browser, await browser.$('//input[@aria-label="Forecast"]'), "Light rain after 18:00");
      await blur(browser);
      await sleep(900);
      const url = await browser.execute(() => window.location.hash);
      callSheetId = (await op("callsheets.list", {}))[0].id;
      assert.ok(callSheetId, `no call sheet listed (${url})`);
      await assertQuiet(browser, "call sheet created");
    });
  });

  test("a schedule change marks the sheet Needs refresh and the preview lists what changed", async () => {
    await step("refresh", async () => {
      const view = await op("schedule.get", {});
      const third = view.unscheduled[0];
      await op("schedule.move_strip", { stripId: third.id, dayId });
      await browser.waitUntil(async () => (await mainText(browser)).includes("Needs refresh"), { timeout: 15_000, timeoutMsg: "the sheet was not flagged" });
      await click(browser, "Review Update");
      await waitForText(browser, "Update call sheet: review changes");
      const dialog = await browser.execute(() => document.querySelector('[role="dialog"]')?.innerText ?? "");
      assert.ok(/scene/i.test(dialog), "the preview does not list the changed scenes");
      await inDialog(browser, "Apply Update");
      await browser.waitUntil(async () => !(await mainText(browser)).includes("Needs refresh."), { timeout: 10_000, timeoutMsg: "refresh did not clear the flag" });
      const forecast = await browser.$('//input[@aria-label="Forecast"]');
      assert.equal(await forecast.getValue(), "Light rain after 18:00", "the refresh lost a typed value");
      await assertQuiet(browser, "refresh");
    });
  });

  test("finalize freezes a snapshot; a new revision follows the schedule", async () => {
    await step("finalize", async () => {
      await click(browser, "Finalize…");
      await inDialog(browser, "Finalize");
      // "Final" alone also matches the Finalize button; the sheet says when it was finalized.
      await browser.waitUntil(async () => /\bFinalized \d/.test(await mainText(browser)), { timeout: 10_000, timeoutMsg: "not final" });
      const crew = await browser.$('//input[@aria-label="Crew call"]');
      assert.equal(await crew.isEnabled(), false, "a final sheet is still editable");
      await exportAndCheck("callsheets.export_pdf", { id: callSheetId, format: "pdf" }, "call-sheet.pdf");
      await op("schedule.set_day_times", { dayId, crewCall: 450, shootingStart: 480, targetWrap: 1140 });
      await browser.waitUntil(async () => (await mainText(browser)).includes("Start New Revision"), { timeout: 15_000, timeoutMsg: "no new revision offered" });
      assert.equal(await (await browser.$('//input[@aria-label="Crew call"]')).getValue(), "07:00", "the final sheet changed with the schedule");
      await click(browser, "Start New Revision");
      await inDialog(browser, "Create New Revision");
      await waitMain(browser, "v2");
      assert.equal(await (await browser.$('//input[@aria-label="Crew call"]')).getValue(), "07:30");
      await assertQuiet(browser, "revision");
    });
  });

  test("budget keeps Planned and Actual with the difference", async () => {
    await step("budget", async () => {
      await productionTab(browser, "Budget");
      await click(browser, "Start budget");
      await click(browser, "Add Line");
      await retype(browser, await browser.$("#of-bl-desc"), "Lead actor");
      await retype(browser, await browser.$("#of-bl-amt"), "50000");
      await retype(browser, await browser.$("#of-bl-act"), "55000");
      await inDialog(browser, "Add Line");
      await waitMain(browser, "over plan");
      await undo(browser);
      await waitMain(browser, "No lines yet");
      await assertQuiet(browser, "budget");
    });
  });

  test("a call sheet from the Indie Call Sheet template is an independent copy", async () => {
    await step("template", async () => {
      const listing = await op("templates.list", {});
      const names = listing.builtin.map((t) => t.name);
      for (const n of ["Feature Film Screenplay", "Short Film Screenplay", "Episodic / Series", "Simple Beat Board", "Indie Call Sheet", "Shot List", "Moodboard", "Lightweight Budget"]) {
        assert.ok(names.includes(n), `built-in template ${n} is missing`);
      }
      const view = await op("schedule.get", {});
      const second = view.days.find((d) => !d.callSheet && !d.isOffDay);
      await op("templates.apply", { scope: "builtin", id: "builtin:call_sheet", dayId: second.id });
      const sheets = await op("callsheets.list", {});
      const made = await op("callsheets.get", { id: sheets.find((s) => s.dayId === second.id).id });
      assert.notEqual(made.document.optional.weather, null, "the template's Weather section is missing");
      assert.notEqual(made.document.optional.crew, null, "the template's Crew section is missing");
      await assertQuiet(browser, "template");
    });
  });

  test("a report and the visual planning exports carry the brand line, sides do not", async () => {
    await step("exports", async () => {
      // Sides are pages of the writer's script: no mark and no brand line (owner, 2026-10-27).
      await exportAndCheck("sides.export_pdf", { format: "pdf", dayId, includeCover: true }, "sides.pdf", false);
      await exportAndCheck("reports.export_report", { format: "pdf", reportType: "scene" }, "scene-report.pdf");
      await exportAndCheck("schedule.export_schedule", { format: "pdf" }, "schedule.pdf");
      await exportAndCheck("schedule.export_dood", { format: "pdf" }, "dood.pdf");
      await exportAndCheck("breakdown.export_report", { format: "pdf" }, "breakdown.pdf");
      const scenes = await op("visual.scenes", {});
      const sceneId = scenes.scenes[0].sceneId;
      await op("templates.apply", { scope: "builtin", id: "builtin:shot_list", sceneId });
      await exportAndCheck("shot.export_list", { format: "pdf" }, "shots.pdf");
      const board = await op("moodboard.create", { name: "Look" });
      await op("moodboard.add_note", { moodboardId: board.id ?? board, text: "Wet asphalt at night" });
      await exportAndCheck("moodboard.export_pdf", { format: "pdf", moodboardId: board.id ?? board }, "moodboard.pdf");
      const sb = await op("storyboard.create", { name: "Opening", sceneId });
      await op("storyboard.add_panel", { storyboardId: sb.id ?? sb, visual: "placeholder", description: "Rain on the platform roof" });
      await exportAndCheck("storyboard.export_sheet", { format: "pdf", storyboardIds: [sb.id ?? sb] }, "storyboard.pdf");
      // Screenplay files stay clean.
      const sources = await op("interchange.sources", {});
      const draftId = sources.screenplays[0].drafts[0].id;
      const sp = join(OUT, "script.pdf");
      await op("screenplay.export", { draftId, format: "pdf", path: sp, options: null });
      assert.ok(!pdfText(sp).includes(BRAND), "the screenplay PDF is branded");
      await assertQuiet(browser, "exports");
    });
  });

  test("notes and tasks for the shoot", async () => {
    await step("notes", async () => {
      await op("notes.create", { title: "Rain cover", body: "Keep the tarps on the truck.", pinned: false });
      await op("tasks.create", { title: "Confirm the railway permit", dueAt: null, ownerUserId: null, notes: null, related: null });
      // Notes & Tasks opens from the project menu (spec 80).
      await projectMenu(browser, "Notes & Tasks");
      await waitMain(browser, "Rain cover");
      await waitMain(browser, "Confirm the railway permit");
      await assertQuiet(browser, "notes and tasks");
    });
  });

  test("quit, relaunch and find everything again", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, PROJECT);
      const sheets = await op("callsheets.list", {});
      assert.ok(sheets.some((s) => s.revision === 2), "the second revision is gone");
      assert.ok(sheets.some((s) => s.status === "Final"), "the final revision is gone");
      await openWorkspace(browser, "Call Sheets");
      await waitMain(browser, "Shoot Day 1");
      await assertQuiet(browser, "after relaunch");
    });
  });
});
