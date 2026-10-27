// @e2e-data-root: own
//
// Storage (docs/engineering/40-storage-locations-and-recovery.md, ADR-0038): Settings > Storage
// names every place OpenFrame keeps data and offers the Global Idea Vault actions. An idea is added
// to the Global Idea Vault, the vault is backed up and the backup is inspected, then the open
// project is moved to another folder while the app runs, and still opens with its content.
// The long operations are called the way the screen calls them (a task that reports through app
// events); the spec listens to the same events and waits for the task to end. No system folder
// dialog is needed, because the destinations are folders inside the spec's own data folder.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { click, launchApp, sleep, stepper, takeConsoleProblems } from "../lib/app.mjs";
import { assertQuiet, closeProject, firstLaunchProject, openProject, projectMenu, waitForHome } from "../lib/flows.mjs";

const TITLE = "Harbour Ledger";
const IDEA = "Tide tables for the harbour wall";
const PROJECT_NOTE = "Moved along with the project";
const ROOT = process.env.E2E_DATA_ROOT ?? ".";
const OUT = join(ROOT, "vault-out");
const BACKUP = join(OUT, "Global Idea Vault backup.ofvault");
const MOVED_TO = join(ROOT, "moved-projects");

let browser;
const step = stepper(() => browser, "storage");

/** Call an application operation exactly as the UI does (ipc/client.ts `call`). */
async function op(name, args) {
  return browser
    .execute(
      (n, a) =>
        window.__TAURI_INTERNALS__
          .invoke("of_invoke", { op: n, args: a })
          .then((v) => ({ ok: v }))
          .catch((e) => ({ err: e })),
      name,
      args,
    )
    .then((r) => {
      if (r.err) throw new Error(`${name} failed: ${JSON.stringify(r.err)}`);
      return r.ok;
    });
}

/** Wait until `check` returns true: storage tasks run in the background and the spec watches their
 *  results (a file written, a place changed) instead of listening to task events itself. */
async function until(check, what, timeout = 60_000) {
  await browser.waitUntil(async () => {
    try {
      return await check();
    } catch {
      return false;
    }
  }, { timeout, interval: 500, timeoutMsg: what });
}

/** The same place, whatever the slashes, the case or a Windows long path prefix. */
const place = (p) => p.replace(/^\\\\\?\\/, "").replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();

const settingsTab = (name) => click(browser, name, { within: '//*[@role="tablist"][@aria-label="Settings"]' });
const section = (name) => `//section[@aria-label="${name}"]`;
/** Text and state of every button in one Storage section. */
const buttonsIn = (name) =>
  browser.execute(
    (n) => [...document.querySelectorAll(`section[aria-label="${n}"] button`)].map((b) => ({ text: b.textContent.trim(), disabled: b.disabled })),
    name,
  );
const homeTitles = () =>
  browser.execute(() => [...document.querySelectorAll("main button[aria-label^='Open ']")].map((b) => b.getAttribute("aria-label")));
const projectIdeas = async () => (await op("vault.list", { store: "project", view: { kind: "all" } })).map((i) => i.title);

describe("storage", () => {
  before(async () => {
    mkdirSync(OUT, { recursive: true });
    mkdirSync(MOVED_TO, { recursive: true });
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("Settings > Storage names the five places and the vault actions", async () => {
    await step("settings", async () => {
      await firstLaunchProject(browser, TITLE);
      await projectMenu(browser, "Project settings");
      await settingsTab("Storage");
      for (const name of ["Your OpenFrame library", "Folder for new projects", "Global Idea Vault", "This project", "App data"]) {
        await (await browser.$(section(name))).waitForDisplayed({ timeout: 15_000, timeoutMsg: `no "${name}" section in Settings > Storage` });
      }
      const vault = await buttonsIn("Global Idea Vault");
      assert.deepEqual(
        vault.map((b) => b.text),
        ["Open Folder", "Back Up…", "Export Package…", "Restore…", "Move…", "Locate Existing Vault…"],
        "the Global Idea Vault buttons are not the expected six",
      );
      assert.ok(!vault.find((b) => b.text === "Restore…").disabled, "Restore is not available");
      assert.ok(!vault.find((b) => b.text === "Locate Existing Vault…").disabled, "Locate Existing Vault is not available");
      const project = (await buttonsIn("This project")).map((b) => b.text);
      assert.ok(project.includes("Open Folder") && project.includes("Move Project…"), `the project buttons are ${project.join(", ")}`);
      await assertQuiet(browser, "storage settings");
    });
  });

  test("an idea in the Global Idea Vault is backed up, and the backup is inspected", async () => {
    await step("backup", async () => {
      await op("vault.create", { store: "global", itemType: "note", title: IDEA, body: "High water is at 06:40 on the harbour wall." });
      assert.equal((await op("storage.info", {})).vaultExists, true, "the Global Idea Vault was not created");

      const started = await op("vault.backup", { path: BACKUP });
      assert.ok(started.taskId, `the backup returned no task: ${JSON.stringify(started)}`);
      // The file appears in one step (atomic write) and can then be read back.
      await until(async () => existsSync(BACKUP) && (await op("vault.inspect", { path: BACKUP })).ideaCount >= 1, "the backup file was not written");
      assert.equal(readFileSync(BACKUP).subarray(0, 2).toString("latin1"), "PK", "the backup is not a zip package");

      const seen = await op("vault.inspect", { path: BACKUP });
      assert.equal(seen.kind, "backup");
      assert.ok(seen.ideaCount >= 1, `the backup holds ${seen.ideaCount} ideas`);

      // The buttons that need a vault are on now: the screen reads the places again when it opens.
      await settingsTab("Appearance");
      await settingsTab("Storage");
      await browser.waitUntil(async () => (await buttonsIn("Global Idea Vault")).every((b) => !b.disabled), {
        timeout: 15_000,
        timeoutMsg: "the vault buttons are still off after the vault exists",
      });
      await assertQuiet(browser, "vault backup");
    });
  });

  test("move the open project to another folder: it keeps its content", async () => {
    await step("move", async () => {
      await op("vault.create", { store: "project", itemType: "note", title: PROJECT_NOTE, body: "A note that must survive the move." });
      const before = (await op("storage.info", {})).projectDir;
      assert.ok(before && existsSync(before), `the project folder is ${before}`);
      const target = join(MOVED_TO, basename(before));

      const started = await op("project.move", { destParent: MOVED_TO });
      assert.ok(started.taskId, `the move returned no task: ${JSON.stringify(started)}`);
      // Done when the open project is the copy in the new place and the old folder is gone.
      await until(
        async () => !existsSync(before) && place((await op("storage.info", {})).projectDir ?? "") === place(target),
        "the project did not finish moving",
      );
      assert.ok(existsSync(target), "the project folder is not in the new place");
      assert.ok(!existsSync(before), "the old project folder is still there");

      const after = await op("storage.info", {});
      assert.equal(place(after.projectDir), place(target), "the open project is not at the new place");
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000, timeoutMsg: "the project did not stay open" });
      assert.equal((await op("project.current", {})).title, TITLE);
      assert.ok((await projectIdeas()).includes(PROJECT_NOTE), "the project lost its note in the move");
      // The move closes and reopens the project: the screen drops its event listeners while the
      // move's progress events still arrive, and Tauri warns about those callbacks. That warning is
      // expected here; any other console message still fails the step.
      const problems = (await takeConsoleProblems(browser)).filter((p) => !p.includes("Couldn't find callback id"));
      assert.deepEqual(problems, [], "console problems in project move");
      await assertQuiet(browser, "project move");
    });
  });

  test("Home lists the moved project under Recent projects, and it opens again", async () => {
    await step("home", async () => {
      await closeProject(browser);
      await (await browser.$(section("Recent projects"))).waitForExist({ timeout: 15_000, timeoutMsg: "Home shows no Recent projects" });
      assert.ok((await homeTitles()).some((t) => t.startsWith(`Open ${TITLE}`)), `Home lists ${(await homeTitles()).join(", ")}`);
      // "Found on this PC" is only for projects in the projects folder that Recent projects does not list.
      await sleep(1000);
      assert.equal(await (await browser.$(section("Found on this PC"))).isExisting(), false, "Found on this PC lists a project that is in Recent projects");
      assert.ok(!(await op("project.discover", {})).some((p) => p.title === TITLE), "project discovery lists the moved project");

      // The Home settings dialog has the same tab; with no project open it has no "This project" place.
      await click(browser, "Settings");
      await settingsTab("Storage");
      await (await browser.$(section("Global Idea Vault"))).waitForDisplayed({ timeout: 15_000, timeoutMsg: "no vault section in the Home settings dialog" });
      assert.equal(await (await browser.$(section("This project"))).isExisting(), false, "the Home dialog shows a project place");
      await browser.keys("Escape");
      await waitForHome(browser);

      await openProject(browser, TITLE);
      assert.ok((await projectIdeas()).includes(PROJECT_NOTE), "the project opened from Home has lost its note");
      await closeProject(browser);
      await assertQuiet(browser, "storage home");
    });
  });
});
