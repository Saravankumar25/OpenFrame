// User-level flows shared by the per-workspace specs (03..89). Each spec owns a fresh data root
// (`// @e2e-data-root: own`, see run.mjs), so it starts at first launch and creates its own project.

import assert from "node:assert/strict";
import { click, launchApp, openWorkspace, sleep, takeConsoleProblems, unlabelledControls, waitForText } from "./app.mjs";

const xq = (s) => (s.includes('"') ? `concat("${s.replace(/"/g, '", \'"\', "')}")` : `"${s}"`);
export { xq };

/** Text of the current workspace (<main>). */
export async function mainText(browser) {
  return browser.execute(() => document.querySelector("main")?.innerText ?? "");
}

/** Wait until <main> shows (or, with present=false, no longer shows) `text`. */
export async function waitMain(browser, text, { present = true, timeout = 15_000 } = {}) {
  await browser.waitUntil(async () => (await mainText(browser)).includes(text) === present, {
    timeout,
    timeoutMsg: present ? `Not shown in the workspace: "${text}"` : `Still shown in the workspace: "${text}"`,
  });
}

/**
 * After every step: no console errors/warnings, every visible control has an accessible name, and
 * the window never scrolls sideways.
 */
export async function assertQuiet(browser, where) {
  const problems = await takeConsoleProblems(browser);
  assert.deepEqual(problems, [], `console problems in ${where}:\n${problems.join("\n")}`);
  const unlabelled = await unlabelledControls(browser);
  assert.deepEqual(unlabelled, [], `controls without an accessible name in ${where}:\n${unlabelled.join("\n")}`);
  const overflow = await browser.execute(() => {
    const m = document.querySelector("main");
    return { page: document.documentElement.scrollWidth - window.innerWidth, main: m ? m.scrollWidth - m.clientWidth : 0 };
  });
  assert.ok(overflow.page <= 1 && overflow.main <= 1, `horizontal overflow in ${where}: ${JSON.stringify(overflow)}`);
}

export async function inDialog(browser, name) {
  await click(browser, name, { within: '//*[@role="dialog"]' });
}

/** Click a menu item whose text starts with `startsWith` (menu items carry their shortcut after the label). */
export async function menuItem(browser, startsWith) {
  const el = await browser.$(`//*[@role="menuitem"][starts-with(normalize-space(.),${xq(startsWith)})]`);
  await el.waitForClickable({ timeout: 10_000, timeoutMsg: `No menu item "${startsWith}"` });
  await el.click();
}

/** Open the project menu and choose a page or command. */
export async function projectMenu(browser, item) {
  const trigger = await browser.$('//button[starts-with(@aria-label,"Project: ")]');
  await trigger.waitForClickable({ timeout: 10_000 });
  await trigger.click();
  await menuItem(browser, item);
}

/** Global undo / redo with the keyboard, after moving focus off any text field (the shortcut skips fields). */
export async function undo(browser) {
  await blur(browser);
  await browser.keys(["Control", "z"]);
}
export async function redo(browser) {
  await blur(browser);
  await browser.keys(["Control", "y"]);
}
export async function blur(browser) {
  await browser.execute(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });
}


/** Set a native date (or time) input the way the picker does; WebDriver cannot drive the picker. */
export async function setNativeValue(browser, el, value) {
  await browser.execute(
    (node, v) => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(node, v);
      node.dispatchEvent(new Event("input", { bubbles: true }));
      node.dispatchEvent(new Event("change", { bubbles: true }));
    },
    el,
    value,
  );
}

/** Replace the value of a text field (input/textarea) the way a user does: select all, type. */
export async function retype(browser, el, text) {
  await el.click();
  await browser.keys(["Control", "a"]);
  await browser.keys("Backspace");
  await el.setValue(text);
}

/** First launch in a fresh data root: name the user, create a project, land on Project Home. */
export async function firstLaunchProject(browser, title, { type } = {}) {
  await waitForText(browser, "Welcome to OpenFrame Studio", 30_000);
  const name = await browser.$("#dn-name");
  await retype(browser, name, "E2E Tester");
  await inDialog(browser, "Continue");
  await createProject(browser, title, { type, first: true });
}

export async function createProject(browser, title, { type, first = false } = {}) {
  await click(browser, first ? "Create Project" : "New Project");
  await (await browser.$("#np-title")).setValue(title);
  if (type) await click(browser, type, { within: '//*[@role="dialog"]' });
  await inDialog(browser, "Create");
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
  await browser.waitUntil(async () => (await browser.execute(() => document.title)) === `${title} - OpenFrame Studio`, {
    timeout: 15_000,
    timeoutMsg: `the window title does not name "${title}"`,
  });
}

/**
 * Wait for Application Home itself. The words "Your projects" are not enough: other screens can
 * show them (the Offline AI damaged-bundle message ends "Your projects were not changed."), and a
 * spec that moves on while the project is still closing ends the app in the middle of the close.
 */
export async function waitForHome(browser, timeout = 15_000) {
  await browser.$('//main[@aria-label="Your projects"]').waitForExist({ timeout, timeoutMsg: "Application Home did not appear" });
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout, reverse: true, timeoutMsg: "the project shell is still open" });
}

/** Close the open project cleanly through the project menu (lands on Application Home). */
export async function closeProject(browser) {
  await projectMenu(browser, "Close project");
  await waitForHome(browser);
}

/**
 * Quit the app (end the WebDriver session) and launch it again on the same data root. Returns the
 * new session, on Application Home.
 */
export async function relaunch(browser) {
  await browser.deleteSession().catch(() => undefined);
  await sleep(1000);
  const next = await launchApp();
  await waitForHome(next, 30_000);
  return next;
}

/** Open a project from Application Home; a clean close before means no recovery question. */
export async function openProject(browser, title, { expectRecovery = false } = {}) {
  const row = await browser.$(`//main//button[starts-with(@aria-label,${xq(`Open ${title}`)})]`);
  await row.waitForClickable({ timeout: 15_000 });
  await row.click();
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
  await sleep(800);
  const recovery = (await browser.execute(() => document.body.innerText)).includes("We found a recovery state");
  assert.equal(recovery, expectRecovery, expectRecovery ? "recovery was not offered" : "unexpected recovery offer after a clean close");
}

/** Paste Fountain text through Create › Import Screenplay and open the imported draft. */
export async function importFountain(browser, text) {
  await click(browser, "Create");
  await menuItem(browser, "Import Screenplay");
  const paste = await browser.$("#ix-paste");
  await paste.waitForDisplayed({ timeout: 10_000 });
  await paste.setValue(text);
  await inDialog(browser, "Preview");
  await waitForText(browser, "Import Screenplay · Preview");
  await inDialog(browser, "Import");
  await waitForText(browser, "Import complete");
  await inDialog(browser, "Open Screenplay");
  await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
  await browser.waitUntil(async () => (await editorElements(browser)).length > 0, { timeout: 15_000, timeoutMsg: "the imported draft did not open" });
}

/** Screenplay editor elements as "type: text". */
export async function editorElements(browser) {
  return browser.execute(() => [...document.querySelectorAll(".spx-doc .sp-el")].map((e) => `${e.dataset.type}: ${e.textContent}`));
}

/**
 * After typing a character name that the project does not know yet, the suggestion list offers
 * "Create New: NAME", and Enter would add the character and move on to Dialogue in one press
 * (Writing comfort, SCRIPT-018). A spec that only writes a scene closes the list with Esc, which
 * keeps the name as typed without adding a character, so the next Enter moves on to Dialogue.
 */
export async function keepNewName(browser, name) {
  const label = `Create New: ${name}`;
  await browser.waitUntil(
    async () => (await browser.execute(() => [...document.querySelectorAll("#sp-suggest-list [role=option] span")].map((s) => s.textContent ?? ""))).includes(label),
    { timeout: 10_000, timeoutMsg: `suggestion "${label}" not offered` },
  );
  await browser.keys("Escape");
  await browser.waitUntil(async () => !(await browser.execute(() => !!document.querySelector("#sp-suggest-list"))), {
    timeout: 5_000,
    timeoutMsg: "Esc did not close the suggestion list",
  });
}

/** Breakdown › Select Source › choose the draft whose label starts with `draftLabel` › Set as Production Source. */
export async function setProductionSource(browser, draftLabel) {
  await openWorkspace(browser, "Breakdown");
  await click(browser, "Select Source");
  const opt = await browser.$(`//*[@role="dialog"]//button[starts-with(normalize-space(.),${xq(draftLabel)})]`);
  await opt.waitForClickable({ timeout: 10_000 });
  await opt.click();
  await inDialog(browser, "Set as Production Source");
  await waitForText(browser, `Production Source: ${draftLabel}`);
}

/** Production › tab. */
export async function productionTab(browser, label) {
  const nav = await browser.$('//nav[@aria-label="Production"]');
  if (!(await nav.isExisting())) await openWorkspace(browser, "Production");
  const b = await browser.$(`//nav[@aria-label="Production"]//button[normalize-space(.)=${xq(label)}]`);
  await b.waitForClickable({ timeout: 10_000 });
  await b.click();
  await sleep(400);
}

/** A short two-scene screenplay used by specs that need scenes. */
export const TWO_SCENES = [
  "Title: Night Shift",
  "",
  "INT. CONTROL ROOM - NIGHT",
  "",
  "Kavya studies a flickering monitor and lifts a torch.",
  "",
  "KAVYA",
  "Someone is in the tunnel.",
  "",
  "EXT. TUNNEL MOUTH - NIGHT",
  "",
  "Ravi drags a heavy toolbox through the rain.",
  "",
  "RAVI",
  "Hold the gate.",
  "",
].join("\n");
