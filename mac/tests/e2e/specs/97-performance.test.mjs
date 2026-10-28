// @e2e-data-root: own
//
// Performance with the Offline AI running (Windows phase W4; docs/engineering/21-performance-budgets.md).
// Measured on whatever build the run drives; the numbers that count are the release run's
// (`--release=store`). With the shipped 36-scene "Example Feature" script:
//   P1  start again, process launch to Application Home shown (WebDriver start-up included, so an upper bound);
//   P2  open the project from Application Home until the workspaces show;
//   P3  typing latency in the screenplay, keydown to next paint, from the page's own Event Timing entries;
//   P6  private working set of the app and its web view with no AI, and again with the AI engines loaded;
//   P5  processor use over 30 s with the project open and the AI idle;
//   AI  time to the first answer, engine load included, and the engines' own memory.
// The numbers go to tests/e2e/artifacts/performance.json. P1, P2 and P6 (without AI) are gates;
// P3 (the app's own work per key) and P5 (idle without AI) are gates too, on the release build (owner,
// 2026-10-16: every test from a release-ready point of view). The AI figures are recorded.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";
import { closeProject, firstLaunchProject, inDialog, menuItem, openProject, waitForHome } from "../lib/flows.mjs";
import { appProcesses, ask, webViewProcesses } from "../lib/ai.mjs";

const TITLE = "Measured Feature";
let browser;
const step = stepper(() => browser, "performance");
const numbers = { at: new Date().toISOString(), release: process.env.E2E_RELEASE ?? "debug", budgets: { P1: 3000, P2: 1500, P3: 16, P5: 0.5, P6: 400 } };

function ps(script) {
  return execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8", windowsHide: true, timeout: 120_000 }).trim();
}
/** Private working set in MB of the given processes, summed (Task Manager's "Memory" column). */
function privateMb(pids) {
  if (pids.length === 0) return 0;
  const out = ps(`$p = @(${pids.join(",")}); (Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $p -contains [int]$_.IDProcess } | Measure-Object WorkingSetPrivate -Sum).Sum`);
  return Math.round(Number(out || 0) / 1048576);
}
/** Processor time used by the given processes over `seconds`, as a percentage of one core. */
function cpuPercent(pids, seconds) {
  const sum = "(Get-Process -Id $p -ErrorAction SilentlyContinue | ForEach-Object { $_.TotalProcessorTime.TotalMilliseconds } | Measure-Object -Sum).Sum";
  return Number(ps(`$p = @(${pids.join(",")}); $a = ${sum}; Start-Sleep ${seconds}; $b = ${sum}; [math]::Round(($b - $a) / (${seconds} * 10), 3)`));
}
/**
 * Processor use of `procs` ([{ pid, name }]) over `windows` windows of `seconds` each: per process
 * (name and command-line role) a list of percents of one core, and the average of their sum.
 */
function idleSeries(procs, windows, seconds) {
  const ids = procs.map((p) => p.pid).join(",");
  const out = ps(
    `$ids = @(${ids}); $r = @(); for ($w = 0; $w -le ${windows}; $w++) { ` +
      `$r += ,@($ids | ForEach-Object { $p = Get-Process -Id $_ -ErrorAction SilentlyContinue; if ($p) { $p.TotalProcessorTime.TotalMilliseconds } else { 0 } }); ` +
      `if ($w -lt ${windows}) { Start-Sleep ${seconds} } }; ConvertTo-Json -Compress -Depth 3 $r`,
  ).trim();
  const samples = JSON.parse(out);
  const role = (p) => /--type=(\w[\w-]*)/.exec(p.commandLine ?? "")?.[1] ?? p.name.replace(/\.exe$/i, "");
  const byProcess = {};
  let totalMs = 0;
  procs.forEach((p, i) => {
    // A process that ended or started during the windows reads as zero there: count only the
    // windows in which it ran from start to end, never a negative difference.
    const deltas = Array.from({ length: windows }, (_, w) => (samples[w][i] > 0 && samples[w + 1][i] >= samples[w][i] ? samples[w + 1][i] - samples[w][i] : 0));
    totalMs += deltas.reduce((a, b) => a + b, 0);
    byProcess[`${role(p)} ${p.pid}`] = deltas.map((d) => Math.round((d / (seconds * 10)) * 1000) / 1000);
  });
  const total = totalMs / (windows * seconds * 10);
  return { byProcess, total: Math.round(total * 1000) / 1000 };
}
const appTree = () => [...appProcesses().filter((p) => p.name.toLowerCase() !== "llama-server.exe"), ...webViewProcesses()].map((p) => p.pid);
const engines = () => appProcesses().filter((p) => p.name.toLowerCase() === "llama-server.exe").map((p) => p.pid);
const pct = (xs, q) => {
  const s = [...xs].sort((a, b) => a - b);
  return s.length ? Math.round(s[Math.min(s.length - 1, Math.floor(q * s.length))]) : null;
};

describe("performance with the AI running", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
    const dir = join(import.meta.dirname, "..", "artifacts");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "performance.json"), JSON.stringify(numbers, null, 2));
  });

  test("a project with the 36-scene Example Feature", async () => {
    await step("project", async () => {
      await firstLaunchProject(browser, TITLE);
      await click(browser, "Create");
      await menuItem(browser, "Import Screenplay");
      const sample = await browser.$('//*[@role="radiogroup"][@aria-label="Sample script"]//button[normalize-space(.)="Example Feature"]');
      await sample.waitForClickable({ timeout: 10_000 });
      await sample.click();
      await inDialog(browser, "Preview");
      await waitForText(browser, "Import Screenplay · Preview");
      await inDialog(browser, "Import");
      await waitForText(browser, "Import complete", 60_000);
      await inDialog(browser, "Open Screenplay");
      await browser.$('//main[@aria-label="Screenplay"]').waitForExist({ timeout: 15_000 });
      await closeProject(browser);
    });
  });

  test("P1 start and P2 project open stay inside their budgets", async () => {
    await step("start-and-open", async () => {
      await browser.deleteSession().catch(() => undefined);
      await sleep(1500);
      const t0 = Date.now();
      browser = await launchApp();
      await waitForHome(browser, 30_000);
      numbers.P1_start_ms = Date.now() - t0;
      const t1 = Date.now();
      await openProject(browser, TITLE);
      numbers.P2_open_ms = Date.now() - t1 - 800; // openProject waits 800 ms for a recovery offer after the shell shows
      numbers.P6_no_ai_mb = privateMb(appTree());
      assert.ok(numbers.P1_start_ms < numbers.budgets.P1, `start took ${numbers.P1_start_ms} ms`);
      assert.ok(numbers.P2_open_ms < numbers.budgets.P2, `project open took ${numbers.P2_open_ms} ms`);
      assert.ok(numbers.P6_no_ai_mb < numbers.budgets.P6, `memory without AI: ${numbers.P6_no_ai_mb} MB`);
    });
  });

  test("P3 typing in scene 20 keeps the app's own work per key under 16 ms (p95), and P5 idle stays under budget", async () => {
    await step("typing", async () => {
      await openWorkspace(browser, "Screenplay");
      // For every key: input delay plus the app's handlers (capture start to the end of the event
      // loop task, measured with a zero-delay message), and the time until the next frame starts.
      // The frame time alone cannot go below one frame; a plain text field in the same window gives
      // the floor, recorded beside it.
      await browser.execute(() => {
        window.__e2eKeys = [];
        const ch = new MessageChannel();
        let pending = null;
        ch.port1.onmessage = () => {
          if (!pending) return;
          pending.done = performance.now();
          pending = null;
        };
        document.addEventListener("keydown", (e) => {
          const k = { at: e.timeStamp, done: 0, frame: 0 };
          window.__e2eKeys.push(k);
          pending = k;
          ch.port2.postMessage(0);
          requestAnimationFrame((t) => (k.frame = t));
        }, true);
      });
      const el = await browser.$("(//div[contains(@class,'spx-doc')]//div[@data-type='action'])[20]");
      await el.scrollIntoView({ block: "center" });
      await el.click();
      await browser.keys("End");
      const text = " The rain gets heavier as the night goes on and nobody leaves the bus stop.".split("");
      for (const ch of text) {
        await browser.keys(ch);
        await sleep(110); // about 100 words a minute
      }
      await sleep(1000);
      const editor = await browser.execute(() => window.__e2eKeys.splice(0));
      // The floor: the same keys into a plain text field added for the measurement.
      await browser.execute(() => {
        const i = document.createElement("input");
        i.id = "e2e-plain";
        i.setAttribute("aria-label", "plain");
        document.body.appendChild(i);
        i.focus();
      });
      for (const ch of text.slice(0, 30)) {
        await browser.keys(ch);
        await sleep(110);
      }
      await sleep(500);
      const plain = await browser.execute(() => {
        document.getElementById("e2e-plain")?.remove();
        return window.__e2eKeys.splice(0);
      });
      const work = (ks) => ks.filter((k) => k.done).map((k) => k.done - k.at);
      const frame = (ks) => ks.filter((k) => k.frame).map((k) => k.frame - k.at);
      numbers.P3_typing = {
        keys: editor.length,
        work_p50_ms: pct(work(editor), 0.5),
        work_p95_ms: pct(work(editor), 0.95),
        to_next_frame_p95_ms: pct(frame(editor), 0.95),
        plain_field_to_next_frame_p95_ms: pct(frame(plain), 0.95),
      };
      assert.ok(numbers.P3_typing.work_p95_ms < numbers.budgets.P3, `typing: the app's work per key is ${numbers.P3_typing.work_p95_ms} ms at p95`);
      // P5: no AI running, project open, no input: under half a percent of one core over 60 s.
      await sleep(5000);
      // Six windows of 10 s; each process's share of one core per window, so the record shows where
      // the time goes and whether it is steady or settling.
      const tree = [...appProcesses().filter((p) => p.name.toLowerCase() !== "llama-server.exe"), ...webViewProcesses()];
      const series = idleSeries(tree, 6, 10);
      numbers.P5_idle_no_ai_by_process = series.byProcess;
      numbers.P5_idle_no_ai_percent_of_one_core = series.total;
      assert.ok(numbers.P5_idle_no_ai_percent_of_one_core < numbers.budgets.P5, `idle without AI: ${numbers.P5_idle_no_ai_percent_of_one_core} percent of one core`);
    });
  });

  test("the AI answers, and memory and processor use with the AI loaded are recorded", async () => {
    await step("ai", async () => {
      const t0 = Date.now();
      const answer = await ask(browser, "Which scenes take place at night?");
      numbers.AI_first_answer_ms = Date.now() - t0;
      assert.ok(answer.text.trim().length > 0, "the AI gave no answer");
      numbers.AI_engines_mb = privateMb(engines());
      numbers.P6_with_ai_mb = privateMb(appTree()) + numbers.AI_engines_mb;
      await sleep(5000);
      numbers.P5_idle_ai_loaded_percent_of_one_core = cpuPercent([...appTree(), ...engines()], 30);
    });
  });
});
