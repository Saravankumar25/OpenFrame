// @e2e-data-root: own
//
// Trial, purchase and first-run safety screens (owner mocks 182 to 198; feature rows PLAT-009 and
// PLAT-015; ADR-0022 and ADR-0023). The test build is the internal installer build: no trial, no
// chip, no Buy, no popup. The store states are then set by hand through `entitlement.debug_set`,
// which exists only in a debug build on an isolated test data root (the shell refuses it in a
// release build, see `release_builds_never_expose_the_hand_set_licence_state`). The page is
// reloaded after each change so the app asks the core again, the way a launch would.
//
// The store screens are driven with the store channel set by hand on Windows. There is no store
// behind them, so "Buy full version" must say so; the stores' own purchase windows are not tested.
// The lock after the trial cannot be closed and the core refuses every project operation under
// it; the licence check screen can be closed and the projects stay open. No screen names a price.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, installConsoleProbe, launchApp, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, firstLaunchProject } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "trial");

/** Set the licence state by hand (debug builds only), then reload so the app asks again. A lock
 * refuses `project.current`, so the project screen is not waited for then. */
async function setLicence(channel, state, resetNotices = false, locked = false) {
  const r = await browser.execute(
    (c, s, reset) =>
      window.__TAURI_INTERNALS__.invoke("of_invoke", { op: "entitlement.debug_set", args: { channel: c, state: s, resetNotices: reset } }),
    channel,
    state,
    resetNotices,
  );
  assert.equal(r.channel, channel);
  await browser.refresh();
  await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 30_000 });
  await installConsoleProbe(browser);
  if (!locked) await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
}

const dialog = (title) => browser.$(`//*[@role="dialog"][.//h2[normalize-space(.)="${title}"]]`);
const dialogs = () => browser.execute(() => [...document.querySelectorAll('[role="dialog"] h2')].map((h) => h.textContent.trim()));
const chip = () =>
  browser.execute(() => {
    const g = document.querySelector('[role="group"][aria-label="Free trial"]');
    if (!g) return null;
    const c = g.querySelector(".chip");
    return { text: c.querySelector("[aria-hidden]").textContent, amber: c.classList.contains("y"), spoken: c.querySelector(".sr-only").textContent };
  });
const NO_PRICE = /[$€£₹¥]|\bprices?\b|pay once|subscription|lifetime/i;

describe("trial and first-run screens", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the internal build shows no chip, no Buy and no popup; a Store trial shows the chip", async () => {
    await step("trial-chip", async () => {
      await firstLaunchProject(browser, "Night Ferry");
      await sleep(1500);
      assert.equal(await chip(), null, "no trial in the internal build");
      assert.equal(await browser.$('//button[normalize-space(.)="Buy full version"]').isExisting(), false);
      assert.deepEqual(await dialogs(), []);
      await setLicence("microsoftStore", { state: "trialActive", days_left: 7 }, true);
      await browser.waitUntil(async () => (await chip()) !== null, { timeout: 15_000, timeoutMsg: "no trial chip" });
      assert.deepEqual(await chip(), { text: "Trial: 7 days left", amber: false, spoken: "Free trial, 7 days left." });
      await (await browser.$('//button[normalize-space(.)="Buy full version"]')).waitForDisplayed();
      assert.deepEqual(await dialogs(), []);
      assert.doesNotMatch(await browser.execute(() => document.body.innerText), NO_PRICE, "no price anywhere");
      await assertQuiet(browser, "trial chip");
    });
  });

  test("day 5 reminder shows once that day; Buy says when the Store is not connected", async () => {
    await step("reminder", async () => {
      await setLicence("microsoftStore", { state: "trialActive", days_left: 3 }, true);
      const d = await dialog("Trial reminder");
      await d.waitForDisplayed({ timeout: 15_000 });
      await waitForText(browser, "Your free trial ends in 3 days");
      assert.doesNotMatch(await d.getText(), NO_PRICE);
      assert.deepEqual(await chip(), {
        text: "Trial: 3 days left",
        amber: true,
        spoken: "Free trial, 3 days left. Your trial ends soon.",
      });
      await assertQuiet(browser, "reminder");
      await click(browser, "Continue trial", { within: '//*[@role="dialog"]' });
      await d.waitForExist({ reverse: true, timeout: 10_000 });
      // Same calendar day after a reload: no second reminder.
      await setLicence("microsoftStore", { state: "trialActive", days_left: 3 });
      await sleep(1500);
      assert.deepEqual(await dialogs(), []);
      // A state set by hand has no Store behind it.
      await click(browser, "Buy full version");
      await waitForText(browser, "The Microsoft Store connection is not available in this build.");
      assert.deepEqual(await dialogs(), []);
    });
  });

  test("after a purchase: data note, 4-step tutorial and welcome, once, keyboard only, cannot be skipped", async () => {
    await step("first-run", async () => {
      await setLicence("microsoftStore", { state: "purchased" });
      const note = await dialog("Your data, your responsibility");
      await note.waitForDisplayed({ timeout: 15_000 });
      assert.equal(await chip(), null, "the chip is gone after the purchase");
      const ok = await browser.$('//*[@role="dialog"]//button[normalize-space(.)="OK, I understand"]');
      assert.equal(await ok.isEnabled(), false, "OK starts disabled");
      await browser.keys(["Escape"]);
      await sleep(300);
      assert.ok(await note.isDisplayed(), "Escape does not close the data note");
      // Read to the end with the keyboard: focus the list and press End.
      const list = await browser.$('//*[@role="region"][@aria-label="Six points about your data"]');
      await list.click();
      await browser.keys(["End"]);
      await browser.execute(() => {
        const el = document.querySelector('[role="region"][aria-label="Six points about your data"]');
        el.scrollTop = el.scrollHeight;
        el.dispatchEvent(new Event("scroll"));
      });
      const box = await browser.$('//*[@role="dialog"]//input[@type="checkbox"]');
      await browser.waitUntil(() => box.isEnabled(), { timeout: 5_000, timeoutMsg: "the box stays disabled after reading" });
      await box.click();
      await browser.waitUntil(() => ok.isEnabled(), { timeout: 5_000 });
      await waitForText(browser, "Show in File Explorer");
      await assertQuiet(browser, "data note");
      await ok.click();

      const tut = await dialog("File safety tutorial");
      await tut.waitForDisplayed({ timeout: 10_000 });
      await browser.keys(["Escape"]);
      await sleep(300);
      assert.ok(await tut.isDisplayed(), "Escape does not close the tutorial");
      for (const n of [2, 3, 4]) {
        await click(browser, "Next", { within: '//*[@role="dialog"]' });
        await waitForText(browser, `Step ${n} of 4`);
      }
      const finish = await browser.$('//*[@role="dialog"]//button[normalize-space(.)="Finish"]');
      assert.equal(await finish.isEnabled(), false, "Finish needs all three boxes");
      const boxes = await browser.$$('//*[@role="dialog"]//input[@type="checkbox"]');
      assert.equal(boxes.length, 3);
      for (const b of boxes) {
        await b.click();
      }
      await browser.waitUntil(() => finish.isEnabled(), { timeout: 5_000 });
      await assertQuiet(browser, "tutorial");
      await finish.click();

      const welcome = await dialog("Welcome to the OpenFrame Studio family");
      await welcome.waitForDisplayed({ timeout: 10_000 });
      await waitForText(browser, "Be a part of our community.");
      await assertQuiet(browser, "welcome");
      await click(browser, "OK", { within: '//*[@role="dialog"]' });
      await welcome.waitForExist({ reverse: true, timeout: 10_000 });

      // Shown once: a reload brings nothing back.
      await setLicence("microsoftStore", { state: "purchased" });
      await sleep(1500);
      assert.deepEqual(await dialogs(), []);
      assert.equal(await chip(), null);
    });
  });

  test("a check that cannot answer can be closed and the project stays open", async () => {
    await step("check-needed", async () => {
      await setLicence("microsoftStore", { state: "needsOnlineCheck" });
      const d = await dialog("Checking your purchase");
      await d.waitForDisplayed({ timeout: 15_000 });
      await waitForText(browser, "Your projects stay open and can be exported. Nothing is deleted.");
      await click(browser, "Not now", { within: '//*[@role="dialog"]' });
      await d.waitForExist({ reverse: true, timeout: 10_000 });
      await browser.$('//nav[@aria-label="Workspaces"]').waitForDisplayed({ timeout: 10_000 });
      await click(browser, "Check purchase", { within: '//*[@role="group"][@aria-label="Licence"]' });
      await d.waitForDisplayed({ timeout: 10_000 });
      await browser.keys(["Escape"]);
      await d.waitForExist({ reverse: true, timeout: 10_000 });
      await assertQuiet(browser, "check needed");
    });
  });

  for (const [channel, store] of [
    ["macAppStore", "The App Store"],
    ["microsoftStore", "The Microsoft Store"],
  ]) {
    test(`${channel}: an ended trial locks the app with only Buy full version and Restore purchase`, async () => {
      await step(`lock-${channel}`, async () => {
        await setLicence(channel, { state: "trialEnded" }, false, true);
        const d = await dialog("Your free trial has ended");
        await d.waitForDisplayed({ timeout: 15_000 });
        const labels = await browser.execute(() => [...document.querySelectorAll('[role="dialog"] button')].map((b) => b.textContent.trim()));
        assert.deepEqual(labels, ["Buy full version", "Restore purchase"]);
        await browser.keys(["Escape"]);
        await browser.keys(["Control", "k"]);
        await sleep(400);
        assert.deepEqual(await dialogs(), ["Your free trial has ended"], "no way around the lock");
        // The core refuses project operations underneath the screen, not only the screen.
        const refused = await browser.execute(() =>
          window.__TAURI_INTERNALS__
            .invoke("of_invoke", { op: "project.current", args: {} })
            .then(() => "ran", (e) => JSON.stringify(e)),
        );
        assert.match(refused, /permission\.licence_required/);
        assert.doesNotMatch(await d.getText(), NO_PRICE);
        await click(browser, "Buy full version", { within: '//*[@role="dialog"]' });
        await waitForText(browser, `${store} connection is not available in this build.`);
      });
    });
  }

  test("Mac first launch cannot be closed and shows no price", async () => {
    await step("mac-first-launch", async () => {
      await setLicence("macAppStore", { state: "trialNotStarted" });
      const d = await dialog("Welcome to OpenFrame Studio");
      await d.waitForDisplayed({ timeout: 15_000 });
      await waitForText(browser, "7 days. Everything included. No credit card.");
      assert.doesNotMatch(await browser.execute(() => document.body.innerText), NO_PRICE, "no price on the first-launch screen");
      await browser.keys(["Escape"]);
      await sleep(300);
      assert.ok(await d.isDisplayed(), "Escape does not close the first-launch screen");
      await assertQuiet(browser, "mac first launch");
      // Leave the build as the internal build for the next spec.
      await setLicence("directInstaller", { state: "internalBuild" });
    });
  });
});
