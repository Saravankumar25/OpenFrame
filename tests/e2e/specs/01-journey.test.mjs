// The main desktop journey, in ONE running app, as a filmmaker would do it:
// first launch → project → Idea Vault → Story (act, sequence, cards, keyboard
// reorder, undo) → Build Screenplay → write in the editor → import Fountain →
// Production Source + accept a suggestion → schedule a scene → call sheet →
// search (Ctrl+K) → Recently Deleted → AI panel without a model.
//
// 02-restart.test.mjs relaunches the app on the same data and checks persistence.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import {
  button,
  click,
  launchApp,
  openWorkspace,
  pageText,
  sleep,
  stepper,
  takeConsoleProblems,
  unlabelledControls,
  waitForText,
} from "../lib/app.mjs";

let browser;
const step = stepper(() => browser, "journey");
const $ = (sel) => browser.$(sel);
const main = () => browser.execute(() => document.querySelector("main")?.innerText ?? "");
const dialog = () => browser.$('//*[@role="dialog"]');
const inDialog = (name) => click(browser, name, { within: '//*[@role="dialog"]' });
const menuItem = async (startsWith) => {
  const el = await $(`//*[@role="menuitem"][starts-with(normalize-space(.),"${startsWith}")]`);
  await el.waitForClickable({ timeout: 10_000 });
  await el.click();
};
const cards = () =>
  browser.execute(() => [...document.querySelectorAll('[aria-label^="Scene Card:"]')].map((e) => e.getAttribute("aria-label").replace("Scene Card: ", "")));
const editorElements = () => browser.execute(() => [...document.querySelectorAll(".spx-doc .sp-el")].map((e) => `${e.dataset.type}: ${e.textContent}`));

/**
 * After every step: no console errors/warnings, and every visible control on
 * screen has an accessible name.
 */
async function assertQuietConsole(where) {
  const problems = await takeConsoleProblems(browser);
  assert.deepEqual(problems, [], `console problems in ${where}:\n${problems.join("\n")}`);
  const unlabelled = await unlabelledControls(browser);
  assert.deepEqual(unlabelled, [], `controls without an accessible name in ${where}:\n${unlabelled.join("\n")}`);
  // The window never scrolls sideways (layout overflow).
  const overflow = await browser.execute(() => {
    const m = document.querySelector("main");
    return {
      page: document.documentElement.scrollWidth - window.innerWidth,
      main: m ? m.scrollWidth - m.clientWidth : 0,
    };
  });
  assert.ok(overflow.page <= 1 && overflow.main <= 1, `horizontal overflow in ${where}: ${JSON.stringify(overflow)}`);
}

const FOUNTAIN = [
  "Title: Black Rain",
  "",
  "INT. RAILWAY PLATFORM - DAWN",
  "",
  "Meera waits with a red umbrella.",
  "",
  "MEERA",
  "The last train is gone.",
  "",
  "EXT. LEVEL CROSSING - DAY",
  "",
  "Arjun runs across the tracks.",
  "",
].join("\n");

describe("desktop journey", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("first launch → name → create project → Project Home", async () => {
    await step("first-launch", async () => {
      await waitForText(browser, "Welcome to OpenFrame Studio");
      const name = await $("#dn-name");
      await name.setValue("E2E Tester");
      await inDialog("Continue");
      await click(browser, "Create Project");
      await (await $("#np-title")).setValue("Night Bus");
      await inDialog("Create");
      await waitForText(browser, "Start with your ideas, build your story, or open a screenplay.");
      await browser.waitUntil(async () => (await browser.execute(() => document.title)) === "OpenFrame Studio — Night Bus", {
        timeout: 10_000,
        timeoutMsg: "window title does not name the open project",
      });
      await assertQuietConsole("home");
    });
  });

  test("Idea Vault: add a note with Quick Capture", async () => {
    await step("vault-note", async () => {
      await openWorkspace(browser, "Idea Vault");
      await click(browser, "Quick Capture");
      await (await $('//textarea[@aria-label="Idea"]')).setValue("Arjun meets Meera again at a rainy bus stop.");
      await inDialog("Save");
      await waitForText(browser, "Arjun meets Meera again at a rainy bus stop.");
      await assertQuietConsole("idea vault");
    });
  });

  test("Story: act, sequence, two cards (one with Ctrl+N), keyboard reorder, undo/redo", async () => {
    await step("story", async () => {
      await openWorkspace(browser, "Story");
      await click(browser, "Add Act");
      await (await $('//input[@aria-label="Act title"]')).setValue("Act 1");
      await browser.keys("Enter");
      await waitForText(browser, "ACT 1");
      await click(browser, "Add");
      await menuItem("Sequence");
      await (await $('//input[contains(@placeholder,"Sequence name")]')).setValue("The return");
      await browser.keys("Enter");
      await waitForText(browser, "The return");
      await click(browser, "Add");
      await menuItem("Scene Card");
      await (await $('//textarea[@aria-label="Short description"]')).setValue("Bus stop reunion in the rain");
      await browser.keys("Enter");
      await sleep(500);
      // Ctrl+N (outside text fields) = New Scene Card.
      await (await $('//h1[normalize-space(.)="Story Board"]')).click();
      await browser.keys(["Control", "n"]);
      const second = await $('//textarea[@aria-label="Short description"]');
      await second.waitForDisplayed({ timeout: 10_000 });
      await second.setValue("Arjun confesses at the police station");
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await cards()).length === 2, { timeout: 10_000 });
      assert.deepEqual(await cards(), ["Bus stop reunion in the rain", "Arjun confesses at the police station"]);

      // Keyboard reorder (Alt+↑), then undo and redo with the global shortcuts.
      await (await $('//*[@aria-label="Scene Card: Arjun confesses at the police station"]')).click();
      await browser.keys(["Alt", "ArrowUp"]);
      await browser.waitUntil(async () => (await cards())[0] === "Arjun confesses at the police station", { timeout: 10_000 });
      await browser.keys(["Control", "z"]);
      await browser.waitUntil(async () => (await cards())[0] === "Bus stop reunion in the rain", { timeout: 10_000, timeoutMsg: "undo did not restore the order" });
      await browser.keys(["Control", "y"]);
      await browser.waitUntil(async () => (await cards())[0] === "Arjun confesses at the police station", { timeout: 10_000, timeoutMsg: "redo did not reapply the move" });
      await assertQuietConsole("story");
    });
  });

  test("Build Screenplay lands in the new screenplay at the first scene", async () => {
    await step("build", async () => {
      await click(browser, "Build Screenplay");
      await (await $('//input[@aria-label="Heading for scene 1"]')).setValue("INT. POLICE STATION — NIGHT");
      await (await $('//input[@aria-label="Heading for scene 2"]')).setValue("EXT. BUS STOP — NIGHT");
      await inDialog("Build Screenplay");
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
      await browser.waitUntil(async () => (await editorElements()).length > 0, { timeout: 15_000 });
      const els = await editorElements();
      assert.deepEqual(els.slice(0, 3), [
        "scene_heading: INT. POLICE STATION — NIGHT",
        "note: Arjun confesses at the police station",
        "action: ",
      ]);
      assert.equal(els.filter((e) => e.startsWith("scene_heading")).length, 2, "one heading per scene, no duplicates");
      await assertQuietConsole("build screenplay");
    });
  });

  test("type in the screenplay editor (autosaved)", async () => {
    await step("write", async () => {
      await (await $('(//div[contains(@class,"spx-doc")]//div[@data-type="action"])[1]')).click();
      await browser.keys("Arjun enters carrying a pistol.".split(""));
      await browser.keys("Enter");
      await browser.keys("ARJUN".split(""));
      await browser.keys("Enter");
      await browser.keys("I was there that night.".split(""));
      await browser.waitUntil(async () => (await editorElements()).includes("dialogue: I was there that night."), { timeout: 10_000 });
      assert.deepEqual((await editorElements()).slice(2, 5), [
        "action: Arjun enters carrying a pistol.",
        "character: ARJUN",
        "dialogue: I was there that night.",
      ]);
      // Autosave settles ("Saved" in the editor toolbar).
      await browser.waitUntil(async () => /\bSaved\b/.test(await main()), { timeout: 15_000 });
      await assertQuietConsole("screenplay editor");
    });
  });

  test("import pasted Fountain from the quick actions menu → opens the new draft", async () => {
    await step("import", async () => {
      await click(browser, "Create");
      await menuItem("Import Screenplay");
      const paste = await $("#ix-paste");
      await paste.waitForDisplayed({ timeout: 10_000 });
      await paste.setValue(FOUNTAIN);
      await inDialog("Preview");
      await waitForText(browser, "Import Screenplay · Preview");
      const preview = await (await dialog()).getText();
      assert.match(preview, /INT\. RAILWAY PLATFORM - DAWN/);
      assert.match(preview, /EXT\. LEVEL CROSSING - DAY/);
      await inDialog("Import");
      await waitForText(browser, "Import complete");
      await inDialog("Open Screenplay");
      await browser.waitUntil(async () => (await editorElements())[0] === "scene_heading: INT. RAILWAY PLATFORM - DAWN", {
        timeout: 15_000,
        timeoutMsg: "the imported draft was not opened",
      });
      const els = await editorElements();
      assert.ok(!els.some((e) => e.includes("Title:")), `the title page must not become script text: ${els}`);
      await assertQuietConsole("import");
    });
  });

  test("Breakdown: select the Production Source and accept a suggestion", async () => {
    await step("breakdown", async () => {
      await openWorkspace(browser, "Breakdown");
      await click(browser, "Select Source");
      await (await $('//*[@role="dialog"]//button[starts-with(normalize-space(.),"Draft 1 — from Story Board")]')).click();
      await inDialog("Set as Production Source");
      await waitForText(browser, "Production Source: Draft 1 — from Story Board");
      await click(browser, "Suggest Elements");
      await click(browser, "Accept Pistol");
      await click(browser, "Accept Arjun");
      await browser.waitUntil(async () => /Props\s*\n?\s*1\s*\n?\s*Pistol/.test(await main()), {
        timeout: 10_000,
        timeoutMsg: "Pistol was not confirmed",
      });
      await assertQuietConsole("breakdown");
    });
  });

  test("Schedule: all scenes start unscheduled, one moves into a shooting day", async () => {
    await step("schedule", async () => {
      await openWorkspace(browser, "Production");
      await (await $('//nav[@aria-label="Production"]//button[normalize-space(.)="Schedule"]')).click();
      await click(browser, "Create Shooting Schedule");
      await inDialog("Create Schedule");
      await waitForText(browser, "2 scenes (0 scheduled, 2 unscheduled)");
      await click(browser, "Create Shooting Day");
      // Native date pickers are not scriptable through WebDriver: set the value
      // the way the picker does and let React see the input event.
      const date = await $('//*[@role="dialog"]//input[@type="date"]');
      await browser.execute((el) => {
        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(el, "2027-06-14");
        el.dispatchEvent(new Event("input", { bubbles: true }));
        el.dispatchEvent(new Event("change", { bubbles: true }));
      }, date);
      await inDialog("Create Day");
      await waitForText(browser, "Shoot Day 1");
      await click(browser, "Scene 1 actions");
      await menuItem("Shoot Day 1");
      await waitForText(browser, "2 scenes (1 scheduled, 1 unscheduled)");
      // Choosing a menu item must not also open the scene's details drawer.
      assert.ok(!(await main()).includes("SCHEDULED SCENE"), "strip drawer opened by the menu click");
      // The unscheduled pool never scrolls sideways.
      const overflow = await browser.execute(() => {
        const p = document.querySelector(".of-board .pool");
        return p ? p.scrollWidth - p.clientWidth : 0;
      });
      assert.ok(overflow <= 1, `unscheduled pool overflows by ${overflow}px`);
      await assertQuietConsole("schedule");
    });
  });

  test("Call sheet from the shooting day", async () => {
    await step("call-sheet", async () => {
      await click(browser, "Shoot Day 1 actions");
      await menuItem("Create Call Sheet");
      await waitForText(browser, "Call Sheet — Day 1");
      const text = await main();
      assert.match(text, /INT\. POLICE STATION — NIGHT/);
      assert.match(text, /ARJUN/);
      await assertQuietConsole("call sheet");
    });
  });

  test("Schedule strip → Open in Screenplay opens the production draft at that scene", async () => {
    await step("strip-to-script", async () => {
      await openWorkspace(browser, "Production");
      await (await $('//nav[@aria-label="Production"]//button[normalize-space(.)="Schedule"]')).click();
      const strip = await $('//div[contains(@class,"daycol")]//div[@role="button" and contains(@class,"strip")][.//span[@class="no" and normalize-space(.)="1"]]');
      await strip.waitForClickable({ timeout: 10_000 });
      await strip.click();
      await click(browser, "Open in Screenplay");
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 10_000 });
      // The imported draft was open last; the strip's scene lives in Draft 1.
      await browser.waitUntil(
        async () =>
          (await main()).includes("Draft 1 — from Story Board") &&
          (await browser.execute(() => document.querySelector(".spx-doc .sp-el.cur")?.textContent ?? "")) === "INT. POLICE STATION — NIGHT",
        { timeout: 15_000, timeoutMsg: "the strip's scene was not opened in its draft" },
      );
      await assertQuietConsole("strip to screenplay");
    });
  });

  test("Search (Ctrl+K) finds the scene and navigates to it", async () => {
    await step("search", async () => {
      await browser.keys(["Control", "k"]);
      await browser.keys("pistol".split(""));
      const hit = await $('//*[@role="option"][contains(normalize-space(.),"INT. POLICE STATION")]');
      await hit.waitForDisplayed({ timeout: 10_000 });
      await hit.click();
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 10_000 });
      await browser.waitUntil(
        async () => (await browser.execute(() => document.querySelector(".spx-doc .sp-el.cur")?.textContent ?? "")) === "INT. POLICE STATION — NIGHT",
        { timeout: 10_000, timeoutMsg: "the scene was not focused" },
      );
      await assertQuietConsole("search");
    });
  });

  test("Recently Deleted: delete a Scene Card and restore it", async () => {
    await step("trash", async () => {
      await openWorkspace(browser, "Story");
      await (await $('//*[@aria-label="Scene Card: Bus stop reunion in the rain"]')).click();
      await browser.keys("Delete");
      // Deleting a card that already built a screenplay scene asks first.
      const confirm = await $('//*[@role="dialog"]');
      if (await confirm.isExisting()) {
        const btn = await $('//*[@role="dialog"]//button[contains(@class,"danger") or starts-with(normalize-space(.),"Delete") or starts-with(normalize-space(.),"Move")]');
        await btn.click();
      }
      await browser.waitUntil(async () => !(await cards()).includes("Bus stop reunion in the rain"), { timeout: 10_000 });
      await click(browser, "Project: Night Bus. Switch or manage projects");
      await menuItem("Recently Deleted");
      await click(browser, "Restore Bus stop reunion in the rain");
      await openWorkspace(browser, "Story");
      await browser.waitUntil(async () => (await cards()).includes("Bus stop reunion in the rain"), { timeout: 10_000, timeoutMsg: "card not restored" });
      await assertQuietConsole("recently deleted");
    });
  });

  test("AI panel says Offline AI is not installed (no network needed)", async () => {
    await step("ai", async () => {
      await click(browser, "Assistant");
      await waitForText(browser, "Offline AI isn't installed on this computer.");
      assert.match(await pageText(browser), /Everything else in OpenFrame works normally\./);
      await button(browser, "Download Offline AI");
      await assertQuietConsole("ai panel");
    });
  });

  test("every workspace and production tab renders without console problems or unlabelled controls", async () => {
    await step("tour", async () => {
      await browser.keys(["Control", "j"]); // close the AI panel
      // At the smallest supported window size (tauri.conf.json minWidth/minHeight).
      await browser.setWindowSize(1200, 720);
      await sleep(500);
      const vw = await browser.execute(() => window.innerWidth);
      assert.ok(vw <= 1200, `window was not resized (viewport ${vw}px)`);
      for (const ws of ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"]) {
        await openWorkspace(browser, ws);
        await sleep(800);
        await assertQuietConsole(ws);
      }
      await openWorkspace(browser, "Production");
      const tabs = await browser.execute(() => [...document.querySelectorAll('nav[aria-label="Production"] button')].map((b) => b.textContent.trim()));
      for (const t of tabs) {
        await (await $(`//nav[@aria-label="Production"]//button[normalize-space(.)="${t}"]`)).click();
        await sleep(800);
        await assertQuietConsole(`Production › ${t}`);
      }
      for (const t of ["Outline", "Characters", "Timeline"]) {
        await openWorkspace(browser, "Story");
        await (await $(`//*[@role="tablist"]//*[@role="tab"][normalize-space(.)="${t}"]`)).click();
        await sleep(800);
        await assertQuietConsole(`Story › ${t}`);
      }
      for (const page of ["Notes & Tasks", "Activity", "Recently Deleted", "Project settings"]) {
        await click(browser, "Project: Night Bus. Switch or manage projects");
        await menuItem(page);
        await sleep(800);
        await assertQuietConsole(page);
      }
    });
  });

  test("close the project cleanly → Application Home lists it", async () => {
    await step("close", async () => {
      await click(browser, "Project: Night Bus. Switch or manage projects");
      await menuItem("Close project");
      await waitForText(browser, "Your projects");
      assert.match(await pageText(browser), /Night Bus/);
      await assertQuietConsole("close project");
    });
  });
});
