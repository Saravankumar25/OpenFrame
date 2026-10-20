// @e2e-data-root: own
// Appearance in the real app (build plan area "Appearance and themes"; features THEME-001 to THEME-006):
// Settings > Appearance from the project menu and from Home, Light / Dark / System, a built-in theme, a
// pasted theme that is checked, previewed, saved and applied, a hostile theme that changes nothing,
// rename, a relaunch that keeps the choice, and deleting the active theme (falls back to OpenFrame Default).

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, projectMenu, relaunch } from "../lib/flows.mjs";

const THEME = JSON.stringify({
  formatVersion: 1,
  id: "harbour-night",
  name: "Harbour Night",
  description: "Blue-grey surfaces with a lamp-orange accent.",
  author: "E2E",
  seeds: { neutralHue: 230, neutralTint: 0.02, accentHue: 55, accentChroma: 0.14, accentLightnessNudge: 0 },
});
const HOSTILE = JSON.stringify({
  formatVersion: 1,
  id: "evil-theme",
  name: "<img src=x onerror=alert(1)>",
  seeds: { neutralHue: 230, neutralTint: 0.02, accentHue: 55, accentChroma: 0.14, accentLightnessNudge: 0 },
  overrides: { light: { bg: "#fff} body{display:none" } },
});

let browser;
const step = stepper(() => browser, "appearance");

const attrs = () =>
  browser.execute(() => ({ theme: document.documentElement.dataset.theme, mode: document.documentElement.dataset.mode, pref: document.documentElement.dataset.pref }));
/** The colour the window background really has (canvas-normalised to sRGB). */
const bgColour = () =>
  browser.execute(() => {
    const c = document.createElement("canvas").getContext("2d");
    c.fillStyle = getComputedStyle(document.querySelector(".app-root")).backgroundColor;
    c.fillRect(0, 0, 1, 1);
    return [...c.getImageData(0, 0, 1, 1).data.slice(0, 3)];
  });
const until = (pred, msg) => browser.waitUntil(async () => pred(await attrs()), { timeout: 8000, timeoutMsg: msg });

async function pasteTheme(text) {
  await click(browser, "Paste Theme…");
  const area = await browser.$("#ap-paste");
  await area.waitForDisplayed({ timeout: 10_000 });
  await area.setValue(text);
  await inDialog(browser, "Check Theme");
}

describe("appearance", () => {
  before(async () => {
    browser = await launchApp();
    await browser.setWindowSize(1440, 900);
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("mode, theme, import, rename, restart and delete", async () => {
    await step("first start shows OpenFrame Default in System mode", async () => {
      await firstLaunchProject(browser, "Harbour Lights");
      const a = await attrs();
      assert.equal(a.theme, "openframe-default");
      assert.equal(a.pref, "system");
      const logo = await browser.$('//header//*[@role="img"][@aria-label="OpenFrame"]');
      assert.ok(await logo.isDisplayed(), "the OpenFrame logo is in the top bar");
    });

    await step("Settings > Appearance from the project menu; Light and Dark switch at once", async () => {
      await projectMenu(browser, "Appearance");
      await browser.$('//*[@role="radiogroup"][@aria-label="Theme"]').waitForDisplayed({ timeout: 10_000 });
      await click(browser, "Dark");
      await until((a) => a.mode === "dark", "Dark was not applied");
      const dark = await bgColour();
      assert.ok(dark.every((v) => v < 80), `dark background ${dark}`);
      await click(browser, "Light");
      await until((a) => a.mode === "light", "Light was not applied");
      assert.deepEqual(await bgColour(), [0xf3, 0xf1, 0xec], "OpenFrame Default Light is today's background");
      await assertQuiet(browser, "Settings > Appearance");
    });

    await step("a built-in theme from the cards", async () => {
      const card = await browser.$('//*[@role="radiogroup"][@aria-label="Theme"]//*[@role="radio"][starts-with(normalize-space(.),"Midnight Reel")]');
      await card.click();
      await until((a) => a.theme === "midnight-reel", "theme not applied");
      assert.equal(await card.getAttribute("aria-checked"), "true");
    });

    await step("a hostile theme is refused and changes nothing", async () => {
      await pasteTheme(HOSTILE);
      await waitForText(browser, "This theme cannot be used. Nothing was changed.");
      assert.equal(await (await browser.$('//button[normalize-space(.)="Save and Apply"]')).isEnabled(), false);
      await inDialog(browser, "Cancel");
      assert.equal((await attrs()).theme, "midnight-reel");
      assert.equal(await browser.execute(() => getComputedStyle(document.body).display), "block");
    });

    await step("a pasted theme is checked, previewed, saved and applied", async () => {
      await pasteTheme(THEME);
      await waitForText(browser, "This theme is readable in Light and Dark.");
      await browser.$('//*[@aria-label="Dark preview"]').waitForDisplayed({ timeout: 5000 });
      await inDialog(browser, "Save and Apply");
      await until((a) => a.theme === "harbour-night", "imported theme not applied");
      await browser.$('//*[@role="listitem"][@aria-label="Harbour Night"]').waitForDisplayed({ timeout: 5000 });
    });

    await step("rename in My Themes", async () => {
      await click(browser, "Rename Harbour Night");
      const input = await browser.$("#ap-rename-name");
      await input.waitForDisplayed({ timeout: 5000 });
      await input.clearValue();
      await input.setValue("Harbour Lamps");
      await inDialog(browser, "Rename");
      await browser.$('//*[@role="listitem"][@aria-label="Harbour Lamps"]').waitForDisplayed({ timeout: 5000 });
      await click(browser, "Dark");
      await until((a) => a.mode === "dark", "Dark was not applied");
      await closeProject(browser);
    });

    await step("the choice survives a relaunch", async () => {
      browser = await relaunch(browser);
      await browser.setWindowSize(1440, 900);
      await until((a) => a.theme === "harbour-night" && a.mode === "dark", "theme or mode not restored after the relaunch");
    });

    await step("Home > Appearance: deleting the active theme falls back to OpenFrame Default at once", async () => {
      // Appearance is the first tab of Settings on Home (owner change list item 6).
      await click(browser, "Settings");
      await browser.$('//*[@role="dialog"]//*[@role="radiogroup"][@aria-label="Theme"]').waitForDisplayed({ timeout: 10_000 });
      await click(browser, "Delete Harbour Lamps");
      await inDialog(browser, "Delete Theme");
      await until((a) => a.theme === "openframe-default", "no fallback after deleting the active theme");
      await click(browser, "System");
      await until((a) => a.pref === "system", "System was not applied");
      await inDialog(browser, "Done");
      await sleep(300);
      await assertQuiet(browser, "Home after Appearance");
    });
  });
});
