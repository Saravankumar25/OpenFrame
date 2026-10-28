// @e2e-data-root: own
// @e2e-debug-only: sets the licence state by hand through entitlement.debug_set, which a release build never registers
//
// Settings > Project settings > About (ADR-0023, ADR-0024; build plan area 18 box "Internal
// installer build"): the app's name and version, and "Internal build, not for sale." on the
// internal team builds. The test build is an internal build. A store channel is then set by hand
// through `entitlement.debug_set`, which exists only in a debug build on an isolated test data root,
// and the About card shows the version only. No screen here names a price.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { installConsoleProbe, launchApp, stepper } from "../lib/app.mjs";
import { assertQuiet, firstLaunchProject, projectMenu } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "about");
const NO_PRICE = /[$€£₹¥]|\bprices?\b|pay once|subscription|lifetime/i;

const invoke = (op, args = {}) => browser.execute((o, a) => window.__TAURI_INTERNALS__.invoke("of_invoke", { op: o, args: a }), op, args);

/** The About card's text, once it shows the version. The text as written, not as drawn: the card
 * heading is set in capitals by the stylesheet, and WebDriver's getText returns the drawn case. */
async function aboutText() {
  const card = await browser.$('//section[@aria-labelledby="about-heading"]');
  await card.waitForDisplayed({ timeout: 15_000, timeoutMsg: "no About card in Project settings" });
  const text = () => browser.execute((el) => el.textContent ?? "", card);
  await browser.waitUntil(async () => /Version \d/.test(await text()), { timeout: 10_000, timeoutMsg: "the About card shows no version" });
  return (await text()).replace(/\s+/g, " ").trim();
}

/** Set the licence state by hand (debug builds only), then reload so the app asks again. */
async function setLicence(channel, state) {
  const r = await invoke("entitlement.debug_set", { channel, state, resetNotices: false });
  assert.equal(r.channel, channel);
  await browser.refresh();
  await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 30_000 });
  await installConsoleProbe(browser);
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
}

describe("about", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the internal build shows its version and says it is not for sale", async () => {
    await step("internal", async () => {
      await firstLaunchProject(browser, "Quiet Pier");
      const { appVersion } = await invoke("app.info");
      assert.match(appVersion, /^\d+\.\d+\.\d+/);
      await projectMenu(browser, "Project settings");
      const text = await aboutText();
      assert.ok(text.includes("About OpenFrame Studio"), text);
      assert.ok(text.includes(`Version ${appVersion}`), `About shows another version: ${text}`);
      assert.ok(text.includes("Internal build, not for sale."), `the internal build is not marked: ${text}`);
      assert.doesNotMatch(text, NO_PRICE, "no price in About");
      // The Licence card says the same in its own words, with nothing to buy.
      assert.equal(await browser.$('//button[normalize-space(.)="Buy full version"]').isExisting(), false);
      await assertQuiet(browser, "about internal");
    });
  });

  test("a store build shows the version only", async () => {
    await step("store", async () => {
      // A store trial: a store build, and no once-only screen in the way (a purchase would first
      // show the data note, which spec 15 covers).
      await setLicence("microsoftStore", { state: "trialActive", days_left: 7 });
      await projectMenu(browser, "Project settings");
      const text = await aboutText();
      assert.ok(text.includes("About OpenFrame Studio"), text);
      assert.ok(!text.includes("Internal build"), `a store build is marked internal: ${text}`);
      assert.doesNotMatch(text, NO_PRICE, "no price in About");
      await assertQuiet(browser, "about store");
      // Leave the build as the internal build.
      await setLicence("directInstaller", { state: "internalBuild" });
      await projectMenu(browser, "Project settings");
      assert.ok((await aboutText()).includes("Internal build, not for sale."));
    });
  });
});
