// @e2e-data-root: own
// @e2e-fake-media
//
// Idea Vault, full flow (build plan: Beat Board and Idea Vault, "Idea Vault end-to-end test"):
// record a voice note with the microphone (WebView2's fake device, see run.mjs) and play it back
// after a relaunch; flag research sources (FSD 107.5) and undo; use the Global Idea Vault and copy
// an idea into the project; send an idea to the Beat Board as a Scene Card and check the original
// stays in the Vault; quit, relaunch and find all of it again.
//
// Drag-in (dropping files from Windows Explorer) is an operating-system drag that WebDriver cannot
// perform; the drop handler and the partial-success path are covered by IdeaVault.test.tsx
// ("drag-in: dropping files from Explorer adds them...") and idea_vault.rs::partial_add_keeps_successful_files.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, inDialog, menuItem, openProject, redo, relaunch, undo } from "../lib/flows.mjs";

const SOURCE = "Police procedure: order of evidence handling at a small station.";
const GLOBAL_IDEA = "A city where every letter is delivered one year late.";
const STORY_IDEA = "Meera finds her own handwriting on an old envelope.";
const VOICE = "Night bus idea";

let browser;
const step = stepper(() => browser, "vault-flow");
const items = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label="Idea Vault items"] [role="button"][data-item-id]')].map((e) => e.getAttribute("aria-label")));
const card = (label) => browser.$(`//*[@aria-label="Idea Vault items"]//*[@role="button"][@data-item-id][starts-with(@aria-label,"${label}")]`);
const cardText = async (label) => (await card(label)).getText();

async function quickCapture(text) {
  await click(browser, "Quick Capture");
  await (await browser.$('//textarea[@aria-label="Idea"]')).setValue(text);
  await inDialog(browser, "Save");
  await browser.waitUntil(async () => (await items()).some((i) => i.startsWith(text.slice(0, 20))), { timeout: 10_000, timeoutMsg: `"${text}" not in the Vault` });
}

async function itemMenu(label, entry) {
  await (await card(label)).click({ button: "right" });
  await menuItem(browser, entry);
}

async function scope(name) {
  const b = await browser.$(`//*[@role="radiogroup"][@aria-label="Which Idea Vault"]//button[normalize-space(.)="${name}"]`);
  await b.waitForClickable({ timeout: 10_000 });
  await b.click();
  await sleep(400);
}

describe("idea vault flow", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("record a voice note with the microphone", async () => {
    await step("voice", async () => {
      await firstLaunchProject(browser, "Letter Town");
      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Record Note");
      await waitForText(browser, "Recording…", 15_000);
      await (await browser.$("#vv-rec-name")).setValue(VOICE);
      await sleep(1500);
      await inDialog(browser, "Stop & Save");
      await browser.waitUntil(async () => (await items()).some((i) => i.startsWith(VOICE)), { timeout: 15_000, timeoutMsg: "the voice note was not saved" });
      await assertQuiet(browser, "voice note");
    });
  });

  test("research flags: set, show as words, undo, clear", async () => {
    await step("research", async () => {
      await quickCapture(SOURCE);
      await itemMenu("Police procedure", "Needs Expert Check");
      await waitForText(browser, "as Needs Expert Check");
      await browser.waitUntil(async () => (await cardText("Police procedure")).includes("Needs Expert Check"), { timeout: 10_000, timeoutMsg: "no flag on the card" });
      await itemMenu("Police procedure", "Checked");
      await browser.waitUntil(async () => (await cardText("Police procedure")).includes("Checked") && !(await cardText("Police procedure")).includes("Needs Expert"), {
        timeout: 10_000,
        timeoutMsg: "flag not changed to Checked",
      });
      await undo(browser);
      await browser.waitUntil(async () => (await cardText("Police procedure")).includes("Needs Expert Check"), { timeout: 10_000, timeoutMsg: "undo did not restore the flag" });
      await redo(browser);
      await browser.waitUntil(async () => !(await cardText("Police procedure")).includes("Needs Expert"), { timeout: 10_000, timeoutMsg: "redo did not reapply the flag" });
      // The drawer shows the same flag and the honesty hint (mock 045).
      await (await card("Police procedure")).doubleClick();
      const sel = await browser.$("#vv-research");
      await sel.waitForDisplayed({ timeout: 10_000 });
      assert.equal(await sel.getValue(), "checked");
      await waitForText(browser, "OpenFrame never marks research as true just because you imported it.");
      const close = await browser.$('//aside//button[@aria-label="Close"]');
      await close.click();
      await close.waitForExist({ reverse: true, timeout: 10_000 });
      await assertQuiet(browser, "research flags");
    });
  });

  test("Global Idea Vault: capture there, copy into the project", async () => {
    await step("global", async () => {
      await scope("Global");
      await quickCapture(GLOBAL_IDEA);
      await itemMenu("A city where every letter", "Copy to Project");
      await inDialog(browser, "Copy to Project");
      await waitForText(browser, "to the Project Idea Vault");
      await scope("This Project");
      await browser.waitUntil(async () => (await items()).some((i) => i.startsWith("A city where every letter")), { timeout: 10_000, timeoutMsg: "the copy is not in the project" });
      await assertQuiet(browser, "global vault");
    });
  });

  test("Send to Beat Board as a Scene Card; the Vault original stays", async () => {
    await step("send", async () => {
      await quickCapture(STORY_IDEA);
      await itemMenu("Meera finds her own", "Send to Story");
      await waitForText(browser, "Send to Beat Board");
      await inDialog(browser, "Create Copy in Beat Board");
      await sleep(600);
      assert.ok((await items()).some((i) => i.startsWith("Meera finds her own")), "the Vault original must stay");
      await openWorkspace(browser, "Story");
      // Open the Parking Lot unless it is open already: the toggle says so at once, while the
      // column itself appears only when the board has loaded.
      const toggle = await browser.$('//button[@aria-pressed][normalize-space(.)="Parking Lot"]');
      await toggle.waitForDisplayed({ timeout: 10_000 });
      if ((await toggle.getAttribute("aria-pressed")) !== "true") await toggle.click();
      await browser.waitUntil(
        async () =>
          (await browser.execute(() => [...document.querySelectorAll('[aria-label^="Scene Card:"]')].map((e) => e.getAttribute("aria-label")))).includes(
            `Scene Card: ${STORY_IDEA}`,
          ),
        { timeout: 10_000, timeoutMsg: "no Scene Card from the Vault idea" },
      );
      await assertQuiet(browser, "send to story");
      await openWorkspace(browser, "Idea Vault");
    });
  });

  test("relaunch: voice note plays, flags and copies persist", async () => {
    await step("relaunch", async () => {
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Letter Town");
      await openWorkspace(browser, "Idea Vault");
      await browser.waitUntil(async () => (await items()).some((i) => i.startsWith(VOICE)), { timeout: 15_000, timeoutMsg: "voice note lost" });
      assert.ok((await cardText("Police procedure")).includes("Checked"), "research flag lost after relaunch");
      assert.ok((await items()).some((i) => i.startsWith("A city where every letter")), "project copy lost");
      await (await card(VOICE)).doubleClick();
      const audio = await browser.$("aside audio");
      await audio.waitForExist({ timeout: 10_000 });
      const playable = await browser.waitUntil(
        async () => browser.execute((el) => el.readyState >= 1 && Number.isFinite(el.duration) && el.duration > 0, audio),
        { timeout: 10_000, timeoutMsg: "the voice note does not load for playback" },
      );
      assert.ok(playable);
      const close = await browser.$('//aside//button[@aria-label="Close"]');
      await close.click();
      await scope("Global");
      await browser.waitUntil(async () => (await items()).some((i) => i.startsWith("A city where every letter")), { timeout: 10_000, timeoutMsg: "Global item lost" });
      await assertQuiet(browser, "vault after relaunch");
      await closeProject(browser);
    });
  });
});
