// @e2e-data-root: own
//
// Real scene numbers and what Home and the call sheet count (fixed behaviours, proved through the
// screens). A script with explicit scene numbers ("#120A#") is imported; a Scene Card links to the
// imported scene, unlinks, links again and keeps the link after the app is closed and opened again;
// Breakdown, Schedule and the call sheet show 120A and not the position; the call sheet shows the
// nearest hospital typed on the location, and a typed override survives Review Update; Project Home
// counts the current draft's scenes (not every draft) and the shooting days without an off day.
// Not covered here: an omitted scene on Home (a scene is only omitted when a numbered scene is
// deleted, which no screen reaches in one step).

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openBeatBoard, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import {
  assertQuiet,
  closeProject,
  firstLaunchProject,
  importFountain,
  inDialog,
  mainText,
  menuItem,
  openProject,
  productionTab,
  relaunch,
  retype,
  setNativeValue,
  waitMain,
} from "../lib/flows.mjs";

const PROJECT = "Corridor Nine";
const CARD = "Kavya sees the bed is empty";
const LOCATION = "Ward Nine Corridor";
const HOSPITAL = "Town Infirmary, Station Road, 14 minutes by car";
const OVERRIDE = "Override: Lakeside Clinic, 6 minutes";
const SCRIPT = [
  "Title: Ward Nine",
  "",
  "INT. HOSPITAL - NIGHT #120A#",
  "",
  "Kavya checks the chart at the end of the bed.",
  "",
  "KAVYA",
  "Bed four is awake.",
  "",
  "EXT. CAR PARK - NIGHT #121#",
  "",
  "Ravi waits by his car in the rain.",
  "",
  "RAVI",
  "Any news?",
  "",
].join("\n");

let browser;
const step = stepper(() => browser, "real-numbers");
let callSheetId = "";

/** Call one application operation through the app's own IPC entry point (setup and reads). */
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

const drawer = '//aside[@aria-label="Scene Card"]';
const drawerText = () => browser.execute(() => document.querySelector('aside[aria-label="Scene Card"]')?.innerText ?? "");
const paper = () => browser.execute(() => document.querySelector('[aria-label="Call sheet document"]')?.innerText ?? "");

async function openCard() {
  await openBeatBoard(browser);
  await (await browser.$(`//*[@aria-label="Scene Card: ${CARD}"]`)).doubleClick();
  await browser.$(drawer).waitForDisplayed({ timeout: 10_000 });
}

/** Choose the imported scene 120A in the drawer's scene select and press Link. */
async function linkScene120A() {
  const select = await browser.$(`${drawer}//select[@aria-label="Screenplay scene"]`);
  await select.waitForDisplayed({ timeout: 10_000 });
  const option = await select.$('.//option[contains(normalize-space(.),"Scene 120A")]');
  await option.waitForExist({ timeout: 10_000, timeoutMsg: "the select does not offer Scene 120A" });
  await select.selectByAttribute("value", await option.getAttribute("value"));
  await click(browser, "Link", { within: drawer });
  await browser.waitUntil(async () => (await drawerText()).includes("Scene 120A"), { timeout: 10_000, timeoutMsg: "the linked banner does not read Scene 120A" });
}

async function createDay(date, { off = false } = {}) {
  await click(browser, off ? "Off Day" : "Create Shooting Day");
  if (date) await setNativeValue(browser, await browser.$("#of-day-date"), date);
  await inDialog(browser, off ? "Add Off Day" : "Create Day");
  await sleep(500);
}

describe("real scene numbers, nearest hospital and Home counts", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("a Scene Card links to an imported scene, unlinks, links again and keeps the link", async () => {
    await step("link", async () => {
      await firstLaunchProject(browser, PROJECT);
      await importFountain(browser, SCRIPT);
      await openBeatBoard(browser);
      await click(browser, "Add Act");
      await (await browser.$('//input[@aria-label="Act title"]')).setValue("Act One");
      await browser.keys("Enter");
      await click(browser, "Add");
      await menuItem(browser, "Scene Card");
      await (await browser.$('//textarea[@aria-label="Short description"]')).setValue(CARD);
      await browser.keys("Enter");
      await (await browser.$(`//*[@aria-label="Scene Card: ${CARD}"]`)).waitForDisplayed({ timeout: 10_000 });
      await openCard();
      assert.ok((await drawerText()).includes("Not linked to a screenplay scene"), "a new card should start unlinked");
      await linkScene120A();
      await click(browser, "Unlink", { within: drawer });
      await browser.waitUntil(async () => (await drawerText()).includes("Not linked to a screenplay scene"), { timeout: 10_000, timeoutMsg: "Unlink did not clear the link" });
      await linkScene120A();
      await assertQuiet(browser, "scene link");
    });
    await step("link-after-reopen", async () => {
      await click(browser, "Close", { within: drawer });
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, PROJECT);
      await openCard();
      await browser.waitUntil(async () => (await drawerText()).includes("Scene 120A"), { timeout: 10_000, timeoutMsg: "the link was lost after the project was opened again" });
      await click(browser, "Close", { within: drawer });
      await assertQuiet(browser, "scene link after reopen");
    });
  });

  test("Breakdown and Schedule show Scene 120A, not its position", async () => {
    await step("breakdown", async () => {
      await openWorkspace(browser, "Breakdown");
      await click(browser, "Select Source");
      const draft = await browser.$('//*[@role="dialog"]//*[@role="radiogroup"][@aria-label="Screenplay drafts"]//*[@role="radio"]');
      await draft.waitForClickable({ timeout: 10_000 });
      await draft.click();
      await inDialog(browser, "Set as Production Source");
      await browser.$('//aside[@aria-label="Scene 120A breakdown"]').waitForDisplayed({ timeout: 10_000, timeoutMsg: "the breakdown panel does not name Scene 120A" });
      const list = await browser.execute(() => document.querySelector('[aria-label="Scenes in the Production Source"]')?.innerText ?? "");
      assert.ok(list.includes("120A") && list.includes("121"), `the scene list does not show the real numbers: ${list}`);
      // Tag the location of scene 120A, so the call sheet has a location to show a hospital for.
      await click(browser, "Add Element");
      const cat = await browser.$("#add-cat");
      await cat.waitForDisplayed({ timeout: 10_000 });
      await cat.selectByVisibleText("Location / Set");
      await (await browser.$("#add-name")).setValue(LOCATION);
      await browser.waitUntil(async () => !(await (await browser.$('//*[@role="dialog"]')).getText()).includes("Looking for existing catalog items"), { timeout: 10_000 });
      const add = await browser.$('//*[@role="dialog"]//button[starts-with(normalize-space(.),"Add to Scene")]');
      await add.waitForClickable({ timeout: 10_000 });
      await add.click();
      await waitForText(browser, `Added “${LOCATION}”`);
      await assertQuiet(browser, "breakdown numbers");
    });
    await step("schedule", async () => {
      await productionTab(browser, "Schedule");
      await click(browser, "Create Shooting Schedule");
      await inDialog(browser, "Create Schedule");
      await waitMain(browser, "2 unscheduled");
      await createDay("2027-06-14");
      await click(browser, "Scene 120A actions");
      await menuItem(browser, "Shoot Day 1");
      await browser.waitUntil(async () => /1 scene\b/.test(await mainText(browser)), { timeout: 10_000, timeoutMsg: "Scene 120A was not scheduled" });
      const text = await mainText(browser);
      assert.ok(text.includes("120A") && text.includes("121"), `the schedule does not show the real numbers: ${text}`);
      for (const position of ["Scene 1 actions", "Scene 2 actions"]) {
        assert.equal(await browser.$(`//button[@aria-label="${position}"]`).isExisting(), false, `the schedule still numbers a scene by its position (${position})`);
      }
      await assertQuiet(browser, "schedule numbers");
    });
  });

  test("the call sheet shows Scene 120A and the nearest hospital typed on the location", async () => {
    await step("hospital-on-location", async () => {
      await productionTab(browser, "Locations");
      const card = await browser.$(`//main//button[contains(@class,"vcard")][.//*[contains(@class,"vt")][normalize-space(.)="${LOCATION}"]]`);
      await card.waitForClickable({ timeout: 10_000, timeoutMsg: `no location card for ${LOCATION}` });
      await card.click();
      const hospital = await browser.$("#loc-hospital");
      await hospital.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, hospital, HOSPITAL);
      await browser.keys("Tab");
      await sleep(600);
      await click(browser, "Close", { within: '//aside[contains(@class,"drawer")]' });
      await assertQuiet(browser, "hospital typed");
    });
    await step("sheet", async () => {
      await openWorkspace(browser, "Call Sheets");
      await click(browser, "Create Call Sheet");
      await inDialog(browser, "Create Call Sheet");
      await browser.$('//*[@aria-label="Call sheet document"]').waitForDisplayed({ timeout: 15_000 });
      assert.ok((await paper()).includes("120A"), "the call sheet does not show Scene 120A");
      const safety = await browser.$(`//input[@aria-label="${LOCATION} · Nearest hospital"]`);
      await safety.waitForDisplayed({ timeout: 10_000, timeoutMsg: "the safety block has no hospital field for the location" });
      assert.equal(await safety.getValue(), HOSPITAL, "the safety block does not show the hospital typed on the location");
      await assertQuiet(browser, "call sheet");
    });
    await step("override", async () => {
      // The "call sheet created" toast sits over the safety block for a few seconds; wait it out.
      await browser.waitUntil(async () => !(await browser.$("button.toast-action").isExisting()), { timeout: 20_000, timeoutMsg: "the toast did not go away" });
      await retype(browser, await browser.$(`//input[@aria-label="${LOCATION} · Nearest hospital"]`), OVERRIDE);
      await browser.execute(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
      callSheetId = (await op("callsheets.list", {}))[0].id;
      await browser.waitUntil(async () => (await op("callsheets.get", { id: callSheetId })).document.locations[0].hospital === OVERRIDE, {
        timeout: 10_000,
        timeoutMsg: "the typed hospital was not saved",
      });
      // Put the second scene on the day: the sheet needs a refresh, and Apply Update keeps what was typed.
      const view = await op("schedule.get", {});
      const day = view.days[0].id;
      await op("schedule.move_strip", { stripId: view.unscheduled[0].id, dayId: day });
      await browser.waitUntil(async () => (await mainText(browser)).includes("Needs refresh"), { timeout: 15_000, timeoutMsg: "the sheet was not flagged" });
      await click(browser, "Review Update");
      await waitForText(browser, "Update call sheet: review changes");
      await browser.waitUntil(async () => /scene/i.test(await browser.execute(() => document.querySelector('[role="dialog"]')?.innerText ?? "")), {
        timeout: 10_000,
        timeoutMsg: "the preview does not list the changed scenes",
      });
      await inDialog(browser, "Apply Update");
      await browser.waitUntil(async () => !(await mainText(browser)).includes("Needs refresh."), { timeout: 10_000, timeoutMsg: "refresh did not clear the flag" });
      assert.ok((await paper()).includes("121"), "the refreshed sheet does not show Scene 121");
      const after = await browser.$(`//input[@aria-label="${LOCATION} · Nearest hospital"]`);
      assert.equal(await after.getValue(), OVERRIDE, "Review Update replaced the typed hospital");
      await assertQuiet(browser, "override survives refresh");
    });
  });

  test("Project Home counts the current draft's scenes and the shooting days without an off day", async () => {
    await step("home", async () => {
      await productionTab(browser, "Schedule");
      await createDay(null, { off: true });
      await openWorkspace(browser, "Screenplay");
      await browser.$('//button[starts-with(@aria-label,"Draft: ")]').waitForDisplayed({ timeout: 15_000, timeoutMsg: "the draft switcher did not show" });
      const switcher = await browser.execute(() => document.querySelector('button[aria-label^="Draft: "]')?.getAttribute("aria-label") ?? "");
      await click(browser, switcher);
      await menuItem(browser, "New Draft");
      const name = await browser.$("#nd-name");
      await name.waitForDisplayed({ timeout: 10_000 });
      await retype(browser, name, "Second pass");
      await inDialog(browser, "Create Draft");
      await waitForText(browser, "Created draft “Second pass”");
      await openWorkspace(browser, "Home");
      const counts = () =>
        browser.execute(() =>
          Object.fromEntries(
            [...document.querySelectorAll('section[aria-label="Production status"] .row')].map((r) => [r.querySelector(".grow")?.textContent?.trim() ?? "", Number(r.querySelector("b")?.textContent)]),
          ),
        );
      // Two drafts of two scenes each: Home counts the current draft (2), not both (4).
      await browser.waitUntil(async () => (await counts())["Screenplay drafts"] === 2, { timeout: 10_000, timeoutMsg: "Home does not count two drafts" });
      assert.equal((await counts())["Screenplay scenes"], 2, `Home added up the drafts: ${JSON.stringify(await counts())}`);
      // One shoot day and one off day: the off day is not a shooting day.
      assert.equal((await counts())["Shooting days"], 1, `Home counted the off day: ${JSON.stringify(await counts())}`);
      await assertQuiet(browser, "home counts");
    });
  });
});
