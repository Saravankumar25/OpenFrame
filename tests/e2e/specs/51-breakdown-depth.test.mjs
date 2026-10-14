// @e2e-data-root: own
//
// Breakdown depth (masterplan area "Breakdown, catalog, locations, cast and crew"): category colour,
// icon, text and tooltip; Accept Selected as one undo step and Reject Selected; filters; quick reuse;
// Show in script; catalog item to all its scenes; location call-sheet fields and availability dates;
// crew availability; Advanced category colours; then quit, relaunch and find everything again.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import {
  TWO_SCENES,
  assertQuiet,
  closeProject,
  firstLaunchProject,
  importFountain,
  inDialog,
  menuItem,
  openProject,
  productionTab,
  projectMenu,
  relaunch,
  undo,
} from "../lib/flows.mjs";

const PROJECT = "Teak Signal";
const ITEM = "Brass key";
const LOCATION = "Signal Cabin";
const HOSPITAL = "Town Hospital, Station Road";

let browser;
const step = stepper(() => browser, "breakdown-depth");
const aside = (n) => `//aside[@aria-label="Scene ${n} breakdown"]`;
const panelText = () => browser.execute(() => document.querySelector('aside[aria-label$=" breakdown"]')?.innerText ?? "");
const sceneOption = (i) => browser.$(`//*[@role="listbox"][@aria-label="Scenes in the Production Source"]//*[@role="option"][${i}]`);
const visibleScenes = () => browser.execute(() => document.querySelectorAll('[aria-label="Scenes in the Production Source"] [role="option"]').length);

async function addElement(category, name) {
  await click(browser, "Add Element");
  const cat = await browser.$("#add-cat");
  await cat.waitForDisplayed({ timeout: 10_000 });
  await cat.selectByVisibleText(category);
  await (await browser.$("#add-name")).setValue(name);
  await browser.waitUntil(async () => !(await (await browser.$('//*[@role="dialog"]')).getText()).includes("Looking for existing catalog items"), { timeout: 10_000 });
  const add = await browser.$('//*[@role="dialog"]//button[starts-with(normalize-space(.),"Add to Scene")]');
  await add.waitForClickable({ timeout: 10_000 });
  await add.click();
  await waitForText(browser, `Added “${name}”`);
}

async function chooseFilter(label) {
  const trigger = await browser.$('//button[starts-with(@aria-label,"Filter scenes:")]');
  await trigger.waitForClickable({ timeout: 10_000 });
  await trigger.click();
  await menuItem(browser, label);
  await sleep(400);
}

describe("breakdown depth", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("source, and every category shows colour with icon, text and tooltip", async () => {
    await step("identity", async () => {
      await firstLaunchProject(browser, PROJECT);
      await importFountain(browser, TWO_SCENES);
      await openWorkspace(browser, "Breakdown");
      await click(browser, "Select Source");
      const draft = await browser.$('//*[@role="dialog"]//*[@role="radiogroup"][@aria-label="Screenplay drafts"]//*[@role="radio"]');
      await draft.waitForClickable({ timeout: 10_000 });
      await draft.click();
      await inDialog(browser, "Set as Production Source");
      await browser.$(aside(1)).waitForDisplayed({ timeout: 10_000 });
      await addElement("Props", ITEM);
      const mark = await browser.$(`${aside(1)}//section[@aria-label="Props"]//*[contains(@class,"catmark")]`);
      await mark.waitForDisplayed({ timeout: 10_000 });
      assert.match(await mark.getAttribute("title"), /^Props: /, "the category has a tooltip");
      assert.equal(await (await mark.$(".cat-label")).getText(), "Props", "the category has its text");
      assert.ok(await (await mark.$("svg")).isExisting(), "the category has an icon");
      const color = await browser.execute((el) => getComputedStyle(el.querySelector(".cat-dot")).backgroundColor, mark);
      assert.notEqual(color, "rgba(0, 0, 0, 0)", "the category has a colour");
      await assertQuiet(browser, "category identity");
    });
  });

  test("Accept Selected is one undo step; Reject Selected removes the rest; Show in script", async () => {
    await step("bulk", async () => {
      await click(browser, "Suggest Elements");
      await waitForText(browser, "Nothing is added to production until you accept.");
      await click(browser, "Review…");
      const accept = await browser.$('//*[@role="dialog"]//button[starts-with(normalize-space(.),"Accept Selected")]');
      await accept.waitForClickable({ timeout: 10_000 });
      const label = await accept.getText();
      const n = Number(/\((\d+)\)/.exec(label)?.[1] ?? "0");
      assert.ok(n >= 2, `several suggestions to accept (${label})`);
      await accept.click();
      await waitForText(browser, "Accepted ");
      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await panelText()).includes("Suggested elements:"), { timeout: 10_000, timeoutMsg: "suggestions still waiting after Accept Selected" });
      await undo(browser);
      await browser.waitUntil(async () => (await panelText()).includes(`Suggested elements: ${n}`), { timeout: 10_000, timeoutMsg: "one undo did not bring every suggestion back" });
      // Keep the first suggestion out of the batch and reject the others in one step.
      await click(browser, "Review…");
      const first = await browser.$('//*[@role="dialog"]//input[@type="checkbox"]');
      await first.waitForClickable({ timeout: 10_000 });
      await first.click();
      await inDialog(browser, `Reject Selected (${n - 1})`);
      await waitForText(browser, "Rejected");
      await browser.keys("Escape");
      await browser.waitUntil(async () => (await panelText()).includes("Suggested elements: 1"), { timeout: 10_000, timeoutMsg: "Reject Selected did not leave exactly the kept suggestion" });
      // Accept the kept one and jump to the words that triggered it.
      const acceptOne = await browser.$(`${aside(1)}//button[starts-with(@aria-label,"Accept ")]`);
      await acceptOne.waitForClickable({ timeout: 10_000 });
      const name = (await acceptOne.getAttribute("aria-label")).slice("Accept ".length);
      await acceptOne.click();
      await waitForText(browser, `Accepted “${name}”`);
      await click(browser, `Actions for ${name}`);
      await menuItem(browser, "Show in script");
      await browser.$("mark.jump").waitForDisplayed({ timeout: 10_000, timeoutMsg: "the source phrase is not highlighted" });
      await assertQuiet(browser, "bulk actions");
    });
  });

  test("filters: only missing a breakdown, then quick reuse fills scene 2", async () => {
    await step("filters", async () => {
      await chooseFilter("Only missing a breakdown");
      await browser.waitUntil(async () => (await visibleScenes()) === 1, { timeout: 10_000, timeoutMsg: "the missing filter did not keep only scene 2" });
      await (await sceneOption(1)).click();
      await browser.$(aside(2)).waitForDisplayed({ timeout: 10_000 });
      await click(browser, `Reuse ${ITEM} (Props) in Scene 2`);
      await waitForText(browser, `Reused “${ITEM}” in Scene 2`);
      await browser.waitUntil(async () => (await visibleScenes()) === 0, { timeout: 10_000, timeoutMsg: "scene 2 still listed as missing a breakdown" });
      await chooseFilter("All scenes");
      await browser.waitUntil(async () => (await visibleScenes()) === 2, { timeout: 10_000 });
      await assertQuiet(browser, "filters and quick reuse");
    });
  });

  test("catalog item to all its scenes", async () => {
    await step("navigate", async () => {
      await productionTab(browser, "Catalog");
      // A catalog item is a table row that opens its drawer when clicked (or with Enter).
      const row = await browser.$(`//main//tr[.//b[normalize-space(.)="${ITEM}"]]`);
      await row.waitForClickable({ timeout: 10_000, timeoutMsg: `no catalog row for ${ITEM}` });
      await row.click();
      await click(browser, "Open Scenes", { within: '//aside[contains(@class,"drawer")]' });
      await waitForText(browser, `scenes that use ${ITEM}`);
      assert.equal(await visibleScenes(), 2, "both scenes that use the item are listed");
      await click(browser, "Show all scenes");
      await assertQuiet(browser, "navigation");
    });
  });

  test("location call-sheet fields, availability dates, and crew availability", async () => {
    await step("people-places", async () => {
      await productionTab(browser, "Locations");
      await click(browser, "Add Location");
      await (await browser.$("#nl-name")).setValue(LOCATION);
      await (await browser.$("#nl-hospital")).setValue(HOSPITAL);
      await inDialog(browser, "Add Location");
      await waitForText(browser, `Added location “${LOCATION}”`);
      const hospital = await browser.$("#loc-hospital");
      await hospital.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await hospital.getValue(), HOSPITAL);
      await browser.execute(() => {
        const set = (id, v) => {
          const el = document.getElementById(id);
          const proto = Object.getPrototypeOf(el);
          Object.getOwnPropertyDescriptor(proto, "value").set.call(el, v);
          el.dispatchEvent(new Event("input", { bubbles: true }));
          el.dispatchEvent(new Event("change", { bubbles: true }));
        };
        set("av-location-start", "2027-06-20");
        set("av-location-end", "2027-06-22");
        set("av-location-note", "Track repairs");
      });
      await click(browser, "Add Dates");
      await waitForText(browser, `Updated availability of ${LOCATION}`);
      await waitForText(browser, "Track repairs");

      await productionTab(browser, "Cast & Crew");
      await click(browser, "Crew");
      await click(browser, "Add Person");
      const person = await browser.$("#nc-name");
      await person.waitForDisplayed({ timeout: 10_000 });
      await person.setValue("Imran Sheikh");
      await (await browser.$("#nc-role")).setValue("Cinematographer");
      await inDialog(browser, "Add to Crew");
      await waitForText(browser, "Added Imran Sheikh to the crew");
      const avail = await browser.$("#cr-avail");
      await avail.waitForDisplayed({ timeout: 10_000 });
      await avail.setValue("Weekends only");
      await browser.keys("Tab");
      // Saved: close the drawer, open the person again and read the note back.
      await click(browser, "Close", { within: '//aside[contains(@class,"drawer")]' });
      const crewRow = await browser.$('//main//tr[.//b[normalize-space(.)="Imran Sheikh"]]');
      await crewRow.waitForClickable({ timeout: 10_000 });
      await crewRow.click();
      await browser.waitUntil(async () => (await (await browser.$("#cr-avail")).getValue()) === "Weekends only", {
        timeout: 10_000,
        timeoutMsg: "the crew availability note was not saved",
      });
      await assertQuiet(browser, "locations and crew");
    });
  });

  test("Advanced settings change a category colour; everything survives a relaunch", async () => {
    await step("advanced-relaunch", async () => {
      await projectMenu(browser, "Project settings");
      const tab = await browser.$('//*[@role="tablist"][@aria-label="Settings"]//*[@role="tab"][normalize-space(.)="Advanced"]');
      await tab.waitForClickable({ timeout: 10_000 });
      await tab.click();
      const red = await browser.$('//*[@role="group"][@aria-label="Props colour"]//button[@aria-label="Red"]');
      await red.waitForClickable({ timeout: 10_000 });
      await red.click();
      await waitForText(browser, "Changed the look of Props");
      await assertQuiet(browser, "advanced settings");

      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, PROJECT);
      await projectMenu(browser, "Project settings");
      const tab2 = await browser.$('//*[@role="tablist"][@aria-label="Settings"]//*[@role="tab"][normalize-space(.)="Advanced"]');
      await tab2.waitForClickable({ timeout: 10_000 });
      await tab2.click();
      const pressed = await browser.$('//*[@role="group"][@aria-label="Props colour"]//button[@aria-label="Red"]');
      await pressed.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await pressed.getAttribute("aria-pressed"), "true", "the category colour was not kept");
      await productionTab(browser, "Locations");
      // A location is a card that opens its drawer.
      const card = await browser.$(`//main//button[contains(@class,"vcard")][.//*[contains(@class,"vt")][normalize-space(.)="${LOCATION}"]]`);
      await card.waitForClickable({ timeout: 10_000, timeoutMsg: `no card for ${LOCATION}` });
      await card.click();
      const hospital = await browser.$("#loc-hospital");
      await hospital.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await hospital.getValue(), HOSPITAL);
      await waitForText(browser, "Track repairs");
      await openWorkspace(browser, "Breakdown");
      await browser.$(aside(1)).waitForDisplayed({ timeout: 10_000 });
      assert.ok((await panelText()).includes(ITEM), "breakdown lost after relaunch");
      await assertQuiet(browser, "after relaunch");
      await closeProject(browser);
    });
  });
});
