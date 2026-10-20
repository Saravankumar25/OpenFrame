// @e2e-data-root: own
//
// Writing comfort (build plan area "Writing comfort"; SCRIPT-001, SCRIPT-004, SCRIPT-017 to SCRIPT-020):
// smart suggestions while typing (prefix, location, time, character, Create New, Esc), one Enter
// after a character name, new or chosen from the list, that moves on to Dialogue with Undo taking
// each step back, the element flow with Tab and a manual override, spelling underlines with the
// personal dictionary, find and replace in this scene only as one undo step, navigator columns, and
// a custom keyboard shortcut that works in the editor and survives a relaunch, also reachable from Home.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { assertQuiet, closeProject, editorElements, firstLaunchProject, menuItem, openProject, projectMenu, relaunch } from "../lib/flows.mjs";

let browser;
const step = stepper(() => browser, "writing-comfort");
const typeKeys = (text) => browser.keys(text.split(""));
const options = () => browser.execute(() => [...document.querySelectorAll("#sp-suggest-list [role=option]")].map((o) => o.querySelector("span")?.textContent ?? ""));
const listOpen = () => browser.execute(() => !!document.querySelector("#sp-suggest-list"));
const saved = () => browser.execute(() => document.querySelector(".sp-top [aria-live=polite]")?.textContent?.trim() ?? "");
async function waitSaved() {
  await browser.waitUntil(async () => (await saved()) === "Saved", { timeout: 15_000, timeoutMsg: "the editor did not report Saved" });
}
async function waitOption(label) {
  await browser.waitUntil(async () => (await options()).includes(label), { timeout: 10_000, timeoutMsg: `suggestion "${label}" not offered` });
}
/** An empty Dialogue line (the editor shows its "Dialogue" placeholder inside it). */
const isEmptyDialogue = (el) => el === "dialogue: " || el === "dialogue: Dialogue";
/** Beat Board character names, read the way the Characters page reads them. */
const characterNames = () =>
  browser.execute(() =>
    window.__TAURI_INTERNALS__
      .invoke("of_invoke", { op: "story.characters", args: {} })
      .then((r) => r.map((c) => c.name))
      .catch(() => []),
  );
async function waitElements(pred, msg) {
  await browser.waitUntil(async () => pred(await editorElements(browser)), { timeout: 10_000, timeoutMsg: msg });
}

describe("writing comfort", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("smart suggestions build a scene without retyping and never change text silently", async () => {
    await step("suggest", async () => {
      await firstLaunchProject(browser, "Bus Depot");
      await openWorkspace(browser, "Screenplay");
      await click(browser, "New Screenplay");
      await waitElements((els) => els.length > 0, "the editor did not open");
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[1]")).click();
      // Prefix: type to filter, Enter accepts.
      await typeKeys("IN");
      await waitOption("INT.");
      await browser.keys("Enter");
      await typeKeys("BUS DEPOT - ");
      // Time of day: filtered list, Enter accepts.
      await typeKeys("NI");
      await waitOption("NIGHT");
      await browser.keys("Enter");
      await waitElements((els) => els[0] === "scene_heading: INT. BUS DEPOT - NIGHT", "heading not built from suggestions");
      await browser.keys("Enter");
      await typeKeys("Meera waits for teh last bus.");
      await browser.keys("Enter");
      // A new character name: one Enter adds it to the Beat Board characters, keeps the text as
      // typed and moves on to Dialogue (owner decision 2026-10-08, like Final Draft).
      await typeKeys("MEERA");
      await waitOption("Create New: MEERA");
      await browser.keys("Enter");
      await waitForText(browser, "Added MEERA to Characters");
      await waitElements((els) => els.at(-2) === "character: MEERA" && isEmptyDialogue(els.at(-1)), "one Enter did not keep the name and move on to Dialogue");
      await typeKeys("Is it late?");
      await browser.keys("Enter");
      await browser.keys("Enter"); // empty action: Enter starts a new scene
      await typeKeys("EXT. ROAD - LATER");
      await browser.keys("Escape"); // close the list, keep the text
      await browser.waitUntil(async () => !(await listOpen()), { timeout: 5_000, timeoutMsg: "Esc did not close the list" });
      await browser.keys("Enter");
      await typeKeys("The bus is late.");
      await browser.keys("Enter");
      // A known name is offered from the script: one Enter fills it and moves on to Dialogue.
      await typeKeys("ME");
      await waitOption("MEERA");
      await browser.keys("Enter");
      await waitElements(
        (els) => els.filter((e) => e === "character: MEERA").length === 2 && els.at(-2) === "character: MEERA" && isEmptyDialogue(els.at(-1)),
        "one Enter did not fill the known name and move on to Dialogue",
      );
      await typeKeys("Late.");
      await waitSaved();
      const els = await editorElements(browser);
      assert.deepEqual(els.slice(0, 5), [
        "scene_heading: INT. BUS DEPOT - NIGHT",
        "action: Meera waits for teh last bus.",
        "character: MEERA",
        "dialogue: Is it late?",
        "scene_heading: EXT. ROAD - LATER",
      ]);
      await assertQuiet(browser, "suggestions");
    });
  });

  test("element flow: Tab moves through the dialogue block and a manual override sticks", async () => {
    await step("flow", async () => {
      await browser.keys("Enter"); // dialogue → action
      await browser.keys("Tab"); // action → character
      await typeKeys("RAVI");
      await browser.keys(["Control", "2"]); // override back to Action
      await waitElements((els) => els.at(-1) === "action: RAVI", "Ctrl+2 did not turn the line into Action");
      await typeKeys(" runs.");
      await waitElements((els) => els.at(-1) === "action: RAVI runs.", "override did not stick while typing");
      await waitSaved();
      await assertQuiet(browser, "element flow");
    });
  });

  test("one Enter after a new character name creates it and moves on to Dialogue; Undo takes each step back", async () => {
    await step("create-and-move-on", async () => {
      await browser.keys("Enter"); // action → character
      await typeKeys("NISHA");
      await waitOption("Create New: NISHA");
      await browser.keys("Enter");
      await waitForText(browser, "Added NISHA to Characters");
      await waitElements((els) => els.at(-2) === "character: NISHA" && isEmptyDialogue(els.at(-1)), "one Enter did not move on to Dialogue");
      assert.ok((await characterNames()).includes("NISHA"), "NISHA is not a Beat Board character");
      await waitSaved();
      // The first undo takes back only the Dialogue step; the character stays.
      await browser.keys(["Control", "z"]);
      await waitElements((els) => els.at(-1) === "character: NISHA", "the first undo did not take back the Dialogue line");
      await waitSaved();
      assert.ok((await characterNames()).includes("NISHA"), "the first undo also removed the character");
      // The second undo takes back the created character; the name typed in the script stays.
      await browser.keys(["Control", "z"]);
      await browser.waitUntil(async () => !(await characterNames()).includes("NISHA"), { timeout: 10_000, timeoutMsg: "the second undo did not remove the created character" });
      assert.equal((await editorElements(browser)).at(-1), "character: NISHA", "the typed name must stay until its own undo");
      await assertQuiet(browser, "create and move on");
    });
  });

  test("one Enter on a known name from the list fills it and moves on to Dialogue; Undo takes the Dialogue step back first", async () => {
    await step("known-and-move-on", async () => {
      const meera = (els) => els.filter((e) => e === "character: MEERA").length;
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')])[last()]")).click();
      await browser.keys("End");
      await sleep(300);
      // NISHA is no longer a project character, so the list offers Create New: keep the name as typed.
      if (await listOpen()) await browser.keys("Escape");
      await browser.keys("Enter"); // character → dialogue
      await browser.keys("Enter"); // empty dialogue → action
      await browser.keys("Tab"); // action → character
      const before = meera(await editorElements(browser));
      await typeKeys("ME");
      await waitOption("MEERA");
      await browser.keys("Enter");
      await waitElements((els) => els.at(-2) === "character: MEERA" && isEmptyDialogue(els.at(-1)), "one Enter did not fill the known name and move on to Dialogue");
      await waitSaved();
      await browser.keys(["Control", "z"]);
      await waitElements((els) => els.at(-1) === "character: MEERA", "the first undo did not take back only the Dialogue line");
      await waitSaved();
      await browser.keys(["Control", "z"]);
      await waitElements((els) => meera(els) === before, "the second undo did not take back the filled name");
      await assertQuiet(browser, "known name and move on");
    });
  });

  test("spelling: unknown words are underlined; Add to dictionary removes the underline", async () => {
    await step("spelling", async () => {
      const underlined = () => browser.execute(() => [...document.querySelectorAll(".spx-doc .sp-miss")].map((e) => e.textContent));
      const hasDictionary = await browser.waitUntil(async () => (await underlined()).includes("teh"), { timeout: 10_000 }).then(
        () => true,
        () => false,
      );
      if (!hasDictionary) {
        // No English dictionary on this machine: nothing may be underlined at all.
        assert.deepEqual(await underlined(), []);
        return;
      }
      assert.ok(!(await underlined()).includes("MEERA"), "character names are never underlined");
      const word = await browser.$("//div[contains(@class,'spx-doc')]//*[contains(@class,'sp-miss')][normalize-space(.)='teh']");
      await word.click({ button: "right" });
      await menuItem(browser, "Add to dictionary");
      await waitForText(browser, "Added “teh” to your dictionary");
      await browser.waitUntil(async () => !(await underlined()).includes("teh"), { timeout: 10_000, timeoutMsg: "the underline stayed after adding the word" });
      await assertQuiet(browser, "spelling");
    });
  });

  test("find and replace in this scene only, undone in one step", async () => {
    await step("find", async () => {
      const road = await browser.$("(//div[contains(@class,'spx-doc')]//div[@data-type='action'])[2]");
      await road.click();
      // The caret is in scene 2 once the editor marks that line as current (as a writer sees it).
      await browser.waitUntil(async () => (await browser.execute(() => document.querySelector(".spx-doc .sp-el.cur")?.textContent ?? "")) === "The bus is late.", {
        timeout: 5_000,
        timeoutMsg: "the click did not put the caret in scene 2",
      });
      await browser.keys(["Control", "f"]);
      await click(browser, "This scene");
      const find = await browser.$('//input[@aria-label="Find"]');
      await find.setValue("bus");
      const replace = await browser.$('//input[@aria-label="Replace with"]');
      await replace.setValue("van");
      await click(browser, "Replace All");
      await waitForText(browser, "Replaced 1 match in Scene 2");
      await waitElements((els) => els.includes("action: The van is late.") && els.includes("action: Meera waits for teh last bus."), "replace was not limited to the scene");
      await click(browser, "Close find (Esc)");
      await waitSaved();
      // Focus is back in the page: Ctrl+Z there is the project's undo.
      await browser.keys(["Control", "z"]);
      await waitElements((els) => els.includes("action: The bus is late."), "one undo did not restore the scene");
      await assertQuiet(browser, "find scope");
    });
  });

  test("navigator columns can be shown and are remembered", async () => {
    await step("columns", async () => {
      await click(browser, "Navigator columns");
      await menuItem(browser, "Page");
      await browser.$('//nav[@aria-label="Scenes"]//*[@aria-label="Page 1"]').waitForExist({ timeout: 10_000, timeoutMsg: "no Page column" });
      await click(browser, "Navigator columns");
      await menuItem(browser, "Notes");
      await browser.$('//nav[@aria-label="Scenes"]//*[contains(@class,"spx-nav-head")][contains(.,"Notes")]').waitForExist({ timeout: 10_000, timeoutMsg: "no Notes column" });
      await assertQuiet(browser, "navigator columns");
    });
  });

  test("a custom shortcut works in the editor and survives a relaunch; restore all", async () => {
    await step("shortcuts", async () => {
      await projectMenu(browser, "Project settings");
      await click(browser, "Keyboard shortcuts");
      await click(browser, "Add a shortcut for Find in script");
      await browser.keys("F3");
      await waitForText(browser, "Find in script: F3");
      await assertQuiet(browser, "shortcut settings");
      await closeProject(browser);
      browser = await relaunch(browser);
      await openProject(browser, "Bus Depot");
      await openWorkspace(browser, "Screenplay");
      await waitElements((els) => els.length >= 5, "the screenplay did not reopen");
      await browser.$('//nav[@aria-label="Scenes"]//*[@aria-label="Page 1"]').waitForExist({ timeout: 10_000, timeoutMsg: "columns were not remembered" });
      await (await browser.$("(//div[contains(@class,'spx-doc')]//div[@data-type='action'])[1]")).click();
      await browser.keys("F3");
      await browser.$('//*[@role="search"][@aria-label="Find in script"]').waitForDisplayed({ timeout: 10_000, timeoutMsg: "F3 did not open Find" });
      await click(browser, "Close find (Esc)");
      await projectMenu(browser, "Project settings");
      await click(browser, "Keyboard shortcuts");
      await click(browser, "Restore all defaults");
      const dialog = '//*[@role="dialog"]';
      await click(browser, "Restore all defaults", { within: dialog });
      await waitForText(browser, "All shortcuts are back to their defaults");
      await sleep(300);
      await assertQuiet(browser, "restore all");
      await closeProject(browser);
    });
  });

  test("Home reaches the same shortcut table from Settings, and its tooltips name the keys", async () => {
    await step("home-shortcuts", async () => {
      const newProject = await browser.$('//button[normalize-space()="New Project"]');
      await newProject.waitForDisplayed({ timeout: 10_000, timeoutMsg: "Home did not show New Project" });
      assert.equal(await newProject.getAttribute("title"), "New Project (Ctrl+N)");
      // The Settings gear replaced the old shortcuts button (owner change list item 6).
      await click(browser, "Settings");
      const dialog = '//*[@role="dialog"]';
      await click(browser, "Keyboard shortcuts", { within: dialog });
      await browser.$(`${dialog}//th[normalize-space()="Command"]`).waitForDisplayed({ timeout: 10_000, timeoutMsg: "the shortcut table did not open" });
      await click(browser, "Add a shortcut for New Project", { within: dialog });
      // Escape belongs to the key recorder while it waits: the dialog stays open.
      await browser.keys("Escape");
      await sleep(300);
      assert.ok(await browser.$(dialog).isDisplayed(), "Escape closed the dialog while recording keys");
      await browser.keys("Escape");
      await browser.$(dialog).waitForDisplayed({ reverse: true, timeout: 10_000, timeoutMsg: "Escape did not close the dialog" });
      await assertQuiet(browser, "home shortcuts");
    });
  });
});
