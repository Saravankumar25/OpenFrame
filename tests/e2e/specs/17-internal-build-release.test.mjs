// @e2e-data-root: own
//
// The internal team build as it ships (Windows phase W4; owner 2026-10-16: every test from a
// release-ready point of view). Specs 15 and 16 set store licence states by hand and so run only on
// a debug build; this spec proves, on the release executable and without any test switch, what the
// internal build itself shows: no trial chip, no Buy button and no licence popup, and About with the
// version and "Internal build, not for sale.". The store states are proved on a packaged store
// build in Chapter 3.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { launchApp, sleep, stepper } from "../lib/app.mjs";
import { assertQuiet, firstLaunchProject, projectMenu } from "../lib/flows.mjs";

const NO_PRICE = /₹|\$|€|rupee|dollar|euro|price/i;
let browser;
const step = stepper(() => browser, "internal-build-release");

describe("the internal build as it ships", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("no trial chip, no Buy and no popup, and About says internal build, not for sale", async () => {
    await step("internal", async () => {
      await firstLaunchProject(browser, "Quiet Pier");
      await sleep(1500);
      assert.equal(await browser.$('//*[contains(@aria-label,"Free trial")]').isExisting(), false, "no trial chip");
      assert.equal(await browser.$('//button[normalize-space(.)="Buy full version"]').isExisting(), false, "nothing to buy");
      assert.equal(await browser.$('//*[@role="dialog"]').isExisting(), false, "no licence popup");
      const appVersion = await browser.executeAsync((done) => {
        window.__TAURI_INTERNALS__.invoke("of_invoke", { op: "app.info", args: {} }).then((v) => done(v.appVersion)).catch(() => done(""));
      });
      assert.match(appVersion, /^\d+\.\d+\.\d+/);
      await projectMenu(browser, "Project settings");
      // The text as written, not as drawn (the card heading is set in capitals by the stylesheet).
      const card = await browser.$('//section[@aria-labelledby="about-heading"]');
      await card.waitForDisplayed({ timeout: 15_000, timeoutMsg: "no About card in Project settings" });
      const read = async () => (await browser.execute((el) => el.textContent ?? "", card)).replace(/\s+/g, " ").trim();
      await browser.waitUntil(async () => /Version \d/.test(await read()), { timeout: 10_000, timeoutMsg: "the About card shows no version" });
      const text = await read();
      assert.ok(text.includes(`Version ${appVersion}`), `About shows another version: ${text}`);
      assert.ok(text.includes("Internal build, not for sale."), `the internal build is not marked: ${text}`);
      assert.doesNotMatch(await browser.execute(() => document.body.innerText), NO_PRICE, "no price anywhere");
      await assertQuiet(browser, "internal build");
    });
  });
});
