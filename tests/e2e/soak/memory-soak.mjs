// Memory soak for release gate P1: `node tests/e2e/soak/memory-soak.mjs --app=<exe> --minutes=120 [--dry-run]`.
// Drives an installed (gated) openframe-desktop.exe through tauri-driver: writes in the screenplay, switches
// workspaces every 10 loops, closes and reopens the project every 50 loops, and samples the working set and
// private bytes of the app and of its msedgewebview2.exe descendants every 60 s. Output goes to
// tests/e2e/artifacts/soak-<stamp>.csv and soak-<stamp>.json. PASS: over the last 90 minutes (the whole run if
// shorter) app private bytes slope < 20 MB/hour, webview total slope < 40 MB/hour, and no crash.

import { spawn, spawnSync, execFileSync } from "node:child_process";
import net from "node:net";
import { existsSync, mkdirSync, readFileSync, writeFileSync, appendFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { DRIVERS, E2E_ROOT, EDGE_DRIVER, TAURI_DRIVER, setupDrivers } from "../scripts/setup-drivers.mjs";
import { teamPassword } from "../lib/team-password.mjs";
import { click, launchApp, openWorkspace, sleep } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, menuItem, openProject } from "../lib/flows.mjs";

const arg = (n) => process.argv.find((a) => a.startsWith(`--${n}=`))?.slice(n.length + 3);
const APP = arg("app") ? resolve(arg("app")) : "";
const MINUTES = Number(arg("minutes") ?? 120);
const DRY = process.argv.includes("--dry-run");
const SAMPLE_MS = 60_000;
const WINDOW_MIN = 90;
const LIMIT = { appMbPerHour: 20, webMbPerHour: 40 };
const TITLE = "Soak Feature";
const PORT = 4444;
const NATIVE_PORT = 4445;

/** Memory budgets of the performance spec, printed next to the result. */
function budgets() {
  const m = /budgets: (\{[^}]*\})/.exec(readFileSync(join(E2E_ROOT, "specs", "97-performance.test.mjs"), "utf8"));
  return m ? m[1] : "(not found)";
}

/** Least-squares slope of y over x (x in hours). */
function slope(xs, ys) {
  const n = xs.length;
  if (n < 2) return 0;
  const mx = xs.reduce((a, b) => a + b, 0) / n;
  const my = ys.reduce((a, b) => a + b, 0) / n;
  let num = 0;
  let den = 0;
  for (let i = 0; i < n; i++) {
    num += (xs[i] - mx) * (ys[i] - my);
    den += (xs[i] - mx) ** 2;
  }
  return den === 0 ? 0 : num / den;
}

const ps = (script) =>
  execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8", windowsHide: true, timeout: 60_000 }).trim();

/** One sample: MB working set and private bytes of the app and of all its msedgewebview2.exe descendants. */
function sample(exe) {
  const path = exe.replace(/'/g, "''");
  const out = ps(
    `$all = Get-CimInstance Win32_Process; ` +
      `$app = @($all | Where-Object { $_.ExecutablePath -eq '${path}' }); ` +
      `$ids = New-Object System.Collections.Generic.HashSet[int]; $app | ForEach-Object { [void]$ids.Add([int]$_.ProcessId) }; ` +
      `do { $n = $ids.Count; $all | Where-Object { $ids.Contains([int]$_.ParentProcessId) } | ForEach-Object { [void]$ids.Add([int]$_.ProcessId) } } while ($ids.Count -gt $n); ` +
      `$web = @($all | Where-Object { $_.Name -eq 'msedgewebview2.exe' -and $ids.Contains([int]$_.ProcessId) }); ` +
      `$mem = { param($set) $p = @($set | ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }); ` +
      `@{ n = $p.Count; ws = [double](($p | Measure-Object WorkingSet64 -Sum).Sum); pb = [double](($p | Measure-Object PrivateMemorySize64 -Sum).Sum) } }; ` +
      `ConvertTo-Json -Compress @{ app = (& $mem $app); web = (& $mem $web) }`,
  );
  const j = JSON.parse(out);
  const mb = (b) => Math.round(((b ?? 0) / 1048576) * 10) / 10;
  return { appN: j.app.n, appWs: mb(j.app.ws), appPb: mb(j.app.pb), webN: j.web.n, webWs: mb(j.web.ws), webPb: mb(j.web.pb) };
}

function listening(port) {
  return new Promise((r) => {
    const s = net.connect(port, "127.0.0.1");
    s.once("connect", () => {
      s.destroy();
      r(true);
    });
    s.once("error", () => r(false));
  });
}

function plan() {
  return [
    `app: ${APP || "(missing --app)"}  minutes: ${MINUTES}  sample every ${SAMPLE_MS / 1000} s`,
    "start tauri-driver + msedgedriver (WebView2 version check as run.mjs), fresh data root tests/e2e/.run/soak-<stamp> (OPENFRAME_DATA_ROOT)",
    "unlock the team gate (OPENFRAME_TEAM_PASSWORD env or Windows User scope), create project, open Screenplay, New Screenplay",
    "loop: scene heading, action, character, dialogue; every 10 loops Story, Idea Vault, Breakdown, Screenplay; every 50 loops close and reopen the project",
    "sample app + msedgewebview2.exe descendants (working set, private bytes) -> artifacts/soak-<stamp>.csv, summary -> soak-<stamp>.json",
    `PASS if, over the last ${WINDOW_MIN} min, app private slope < ${LIMIT.appMbPerHour} MB/h and webview total private slope < ${LIMIT.webMbPerHour} MB/h and no crash`,
    `existing memory budgets (specs/97-performance): ${budgets()} (P6 = MB private, no AI)`,
  ];
}

async function main() {
  if (!APP) throw new Error("Usage: node tests/e2e/soak/memory-soak.mjs --app=<openframe-desktop.exe> --minutes=120 [--dry-run]");
  if (DRY) {
    console.log(plan().join("\n"));
    return;
  }
  if (!existsSync(APP)) throw new Error(`--app: ${APP} does not exist.`);
  await setupDrivers();
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const root = join(E2E_ROOT, ".run", `soak-${stamp}`);
  const art = join(E2E_ROOT, "artifacts");
  mkdirSync(root, { recursive: true });
  mkdirSync(art, { recursive: true });
  const csv = join(art, `soak-${stamp}.csv`);
  writeFileSync(csv, "minute,loops,appN,appWsMB,appPrivateMB,webN,webWsMB,webPrivateMB\n");

  Object.assign(process.env, { OPENFRAME_DATA_ROOT: root, E2E_APP: APP, E2E_DATA_ROOT: root, E2E_PORT: String(PORT), E2E_RELEASE: "gated", E2E_TEAM_PASSWORD: teamPassword() });
  delete process.env.OPENFRAME_TEAM_PASSWORD;
  if ((await listening(PORT)) || (await listening(NATIVE_PORT))) throw new Error(`Ports ${PORT}/${NATIVE_PORT} are in use.`);
  const driver = spawn(TAURI_DRIVER, ["--port", String(PORT), "--native-port", String(NATIVE_PORT), "--native-driver", EDGE_DRIVER], {
    env: process.env,
    stdio: ["ignore", "inherit", "inherit"],
    cwd: DRIVERS,
  });
  const rows = [];
  const errors = [];
  let loops = 0;
  let crashed = false;
  let browser;
  const t0 = Date.now();
  const EL = "//div[contains(@class,'spx-doc')]//div[contains(@class,'sp-el')]";
  try {
    for (let i = 0; i < 80 && !(await listening(PORT)); i++) await sleep(250);
    browser = await launchApp();
    await firstLaunchProject(browser, TITLE);
    const typeKeys = (t) => browser.keys(t.split(""));
    // Open Screenplay, make sure a screenplay exists, and put the cursor at the end of the last element.
    const place = async () => {
      await openWorkspace(browser, "Screenplay");
      if ((await browser.$$(EL)).length === 0) {
        await click(browser, "New Screenplay");
        await browser.waitUntil(async () => (await browser.$$(EL)).length > 0, { timeout: 15_000, timeoutMsg: "the editor did not open" });
      }
      const all = await browser.$$(EL);
      await all[all.length - 1].click();
      await browser.keys(["Control", "End"]);
    };
    await place();
    let lastSample = 0;
    const takeSample = () => {
      const s = sample(APP);
      const row = { minute: Math.round(((Date.now() - t0) / 60000) * 100) / 100, loops, ...s };
      rows.push(row);
      appendFileSync(csv, `${row.minute},${loops},${s.appN},${s.appWs},${s.appPb},${s.webN},${s.webWs},${s.webPb}\n`);
      if (s.appN === 0) crashed = true;
    };
    takeSample();
    while (Date.now() - t0 < MINUTES * 60_000 && !crashed) {
      loops += 1;
      try {
        await browser.keys("Enter");
        const trigger = await browser.$('//button[starts-with(@aria-label,"Element type: ")]');
        await trigger.waitForClickable({ timeout: 10_000 });
        await trigger.click();
        await menuItem(browser, "Scene Heading");
        await typeKeys(`INT. SOAK ROOM ${loops} - NIGHT`);
        await browser.keys("Enter");
        await typeKeys("Asha checks the gauge and writes the number down.");
        await browser.keys("Enter");
        await typeKeys("ASHA");
        await browser.keys("Escape"); // closes the name suggestions, keeps the name as typed
        await browser.keys("Enter");
        await typeKeys("Still holding.");
        if (loops % 10 === 0) {
          for (const w of ["Story", "Idea Vault", "Breakdown"]) await openWorkspace(browser, w);
          await place();
        }
        if (loops % 50 === 0) {
          await sleep(3000); // let autosave settle before the close
          await closeProject(browser);
          await openProject(browser, TITLE);
          await place();
        }
      } catch (e) {
        errors.push(`loop ${loops}: ${e instanceof Error ? e.message.split("\n")[0] : e}`);
        if (errors.length > 20 && errors.length > loops / 2) break; // the app is not usable any more
        await browser.keys("Escape").catch(() => undefined);
      }
      if (Date.now() - lastSample >= SAMPLE_MS) {
        lastSample = Date.now();
        takeSample();
      }
      await sleep(1000);
    }
    takeSample();
  } catch (e) {
    errors.push(`fatal: ${e instanceof Error ? e.message.split("\n")[0] : e}`);
    try {
      if (sample(APP).appN === 0) crashed = true;
    } catch {
      /* keep the error above */
    }
  } finally {
    await browser?.deleteSession().catch(() => undefined);
    spawnSync("taskkill", ["/PID", String(driver.pid), "/T", "/F"], { stdio: "ignore" });
  }

  const endMin = rows.at(-1)?.minute ?? 0;
  const win = rows.filter((r) => r.minute >= endMin - WINDOW_MIN);
  const hrs = win.map((r) => r.minute / 60);
  const appSlope = Math.round(slope(hrs, win.map((r) => r.appPb)) * 10) / 10;
  const webSlope = Math.round(slope(hrs, win.map((r) => r.webPb)) * 10) / 10;
  const summary = {
    app: APP,
    minutesRequested: MINUTES,
    minutesRun: endMin,
    loops,
    errors,
    crashed,
    start: rows[0],
    end: rows.at(-1),
    peakAppPrivateMB: Math.max(0, ...rows.map((r) => r.appPb)),
    peakWebPrivateMB: Math.max(0, ...rows.map((r) => r.webPb)),
    windowMinutes: Math.min(WINDOW_MIN, endMin),
    appPrivateSlopeMBperHour: appSlope,
    webPrivateSlopeMBperHour: webSlope,
    limits: LIMIT,
    budgets97: budgets(),
  };
  const pass = !crashed && rows.length >= 2 && appSlope < LIMIT.appMbPerHour && webSlope < LIMIT.webMbPerHour;
  summary.result = pass ? "PASS" : "FAIL";
  writeFileSync(join(art, `soak-${stamp}.json`), JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));
  console.log(`\nMemory budgets in specs/97-performance: ${budgets()}`);
  console.log(`\n${summary.result}: app ${appSlope} MB/h (limit ${LIMIT.appMbPerHour}), webview ${webSlope} MB/h (limit ${LIMIT.webMbPerHour}), crashed: ${crashed}, loops: ${loops}, errors: ${errors.length}`);
  process.exit(pass ? 0 : 1);
}

main().catch((e) => {
  console.error(e instanceof Error ? e.stack : e);
  process.exit(1);
});
