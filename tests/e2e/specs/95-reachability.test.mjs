// Reachability test generated from the feature status file (build plan item reachability test).
//
// `scripts/check-reachability.mjs` reads every feature row at Reachable or above and returns the canonical
// UI routes they name (workspaces, sub-tabs and shell entry points), each already checked against
// the source. This spec clicks to every one of those routes in the real app the way a user does,
// and fails when a route has no click recipe here, when the control is missing, when the selected
// tab is not announced as current (aria-selected / aria-current), or when the console reports a
// problem. Runs after the journey and relaunch specs (same isolated data root).

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { join } from "node:path";
import { checkRoutes } from "../../../scripts/check-reachability.mjs";
import { button, click, launchApp, openWorkspace, sleep, stepper, takeConsoleProblems, waitForText } from "../lib/app.mjs";

const REPO = join(import.meta.dirname, "..", "..", "..");

const WORKSPACE_LABEL = {
  home: "Home",
  vault: "Idea Vault",
  story: "Story",
  screenplay: "Screenplay",
  breakdown: "Breakdown",
  production: "Production",
  callsheets: "Call Sheets",
  files: "Files",
};
const PROJECT_PAGE = { trash: "Recently Deleted", activity: "Activity", notes: "Notes & Tasks", settings: "Project settings" };
const SUB_LABEL = {
  production: {
    overview: "Overview", catalog: "Catalog", locations: "Locations", "cast-crew": "Cast & Crew", moodboards: "Moodboards",
    storyboards: "Storyboards", shots: "Shot Lists", schedule: "Schedule", daily: "Daily View", sides: "Sides",
    reports: "Reports", budget: "Budget",
  },
  story: { board: "Board", outline: "Outline", characters: "Characters", timeline: "Timeline", episodes: "Episodes" },
  screenplay: { compare: "Compare", review: "Review" },
  vault: { deleted: "Recently Deleted" },
  settings: { templates: "Templates", "offline-ai": "Offline AI", writing: "Writing", shortcuts: "Keyboard shortcuts", advanced: "Advanced", appearance: "Appearance" },
};
/** Sub-routes that replace the page rather than select a tab: the button that opens them goes
 * away, and the page shows this button instead. */
const PAGE_SWITCH = { "vault/deleted": "Back to Vault" };
/** Routes that only exist in an episodic or series project. */
const EPISODIC_ONLY = new Set(["story/episodes"]);

let browser;
const step = stepper(() => browser, "reachability");

async function quiet(label) {
  const problems = await takeConsoleProblems(browser);
  assert.deepEqual(problems, [], `${label}: ${problems.join("\n")}`);
}

async function projectMenu(item) {
  const trigger = await browser.$('//button[starts-with(@aria-label,"Project: ")]');
  await trigger.waitForClickable({ timeout: 10_000 });
  await trigger.click();
  const el = await browser.$(`//*[@role="menuitem"][starts-with(normalize-space(.),"${item}")]`);
  await el.waitForClickable({ timeout: 10_000 });
  await el.click();
}

async function assertCurrent(el, label) {
  const selected = await el.getAttribute("aria-selected");
  const current = await el.getAttribute("aria-current");
  assert.ok(selected === "true" || (current && current !== "false"), `${label} is not announced as the current tab`);
}

async function visit(route) {
  const [ws, sub] = route.split("/");
  if (route === "shell") {
    await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 10_000 });
    return;
  }
  if (route === "search-overlay") {
    await browser.keys(["Control", "k"]);
    await browser.$('//*[@role="dialog"]').waitForDisplayed({ timeout: 10_000 });
    await browser.keys(["Escape"]);
    return;
  }
  if (route === "ai-panel") {
    await browser.keys(["Control", "j"]);
    const panel = await browser.$('//aside[@aria-label="AI Assistant"]');
    await panel.waitForDisplayed({ timeout: 10_000, timeoutMsg: "Ctrl+J did not open the AI Assistant" });
    await browser.keys(["Control", "j"]);
    await panel.waitForExist({ reverse: true, timeout: 10_000, timeoutMsg: "Ctrl+J did not close the AI Assistant" });
    return;
  }
  if (route === "project-menu") {
    const trigger = await browser.$('//button[starts-with(@aria-label,"Project: ")]');
    await trigger.click();
    await browser.$('//*[@role="menu"]').waitForDisplayed({ timeout: 10_000 });
    await browser.keys(["Escape"]);
    return;
  }
  if (PROJECT_PAGE[ws]) {
    await projectMenu(PROJECT_PAGE[ws]);
    await sleep(500);
  } else {
    assert.ok(WORKSPACE_LABEL[ws], `no click recipe for workspace route ${route}; add it to 95-reachability.test.mjs`);
    await openWorkspace(browser, WORKSPACE_LABEL[ws]);
  }
  if (sub) {
    const label = SUB_LABEL[ws]?.[sub];
    assert.ok(label, `no click recipe for sub-route ${route}; add it to 95-reachability.test.mjs`);
    const el = await button(browser, label);
    await el.scrollIntoView({ block: "center" }).catch(() => undefined);
    await el.click();
    await sleep(500);
    if (PAGE_SWITCH[route]) {
      await button(browser, PAGE_SWITCH[route]);
    } else if (ws !== "screenplay") {
      await assertCurrent(await button(browser, label), route);
    }
  }
}

describe("reachability from the feature status file", () => {
  const { errors, canonical } = checkRoutes(REPO);

  before(async () => {
    browser = await launchApp();
    await browser.setWindowSize(1440, 900);
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("the feature status routes are valid against the source", () => {
    assert.deepEqual(errors, []);
    assert.ok(canonical.length >= 20, `only ${canonical.length} canonical routes`);
  });

  test("every canonical route opens in the real app", async () => {
    await step("open project", async () => {
      await waitForText(browser, "Your projects");
      const row = await browser.$('//main//button[starts-with(@aria-label,"Open Night Bus")]');
      await row.waitForClickable({ timeout: 10_000 });
      await row.click();
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      const keep = await browser.$('//button[normalize-space(.)="Keep Saved Version" and not(@disabled)]');
      if (await keep.waitForDisplayed({ timeout: 4000 }).catch(() => false)) await keep.click();
      await sleep(500);
      await quiet("open project");
    });
    for (const route of canonical.filter((r) => r !== "app-home" && !EPISODIC_ONLY.has(r))) {
      await step(`route ${route}`, async () => {
        await visit(route);
        await quiet(route);
      });
    }
    await step("app-home", async () => {
      await projectMenu("Close project");
      await waitForText(browser, "Your projects");
      await quiet("app-home");
    });
    const episodic = canonical.filter((r) => EPISODIC_ONLY.has(r));
    if (episodic.length) {
      await step("episodic project", async () => {
        await click(browser, "New Project");
        await (await browser.$("#np-title")).setValue("Reach Series");
        await click(browser, "Episodic / Series");
        await click(browser, "Create", { within: '//*[@role="dialog"]' });
        await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
        for (const route of episodic) await visit(route);
        await quiet("episodic routes");
        await projectMenu("Close project");
        await waitForText(browser, "Your projects");
      });
    }
  });
});
