// @e2e-data-root: own
//
// Threat T33 (Windows phases W3 and W4): the web view never talks to the network. The AI specs check
// the app and its engines; this one finds every WebView2 process that belongs to the app (the
// browser process and its renderers, GPU and utility processes), samples every socket they own and
// fails on anything that is not loopback. It runs twice: once through WebDriver with a project open
// and the main workspaces walked, and once with the app started directly, as a person starts it,
// because msedgedriver replaces the webview's arguments with its own and so cannot prove what the
// app passes. Both runs also check that the browser process got the app's arguments
// (tauri.conf.json, `additionalBrowserArgs`). The processes and endpoints go to the artifacts folder.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import net from "node:net";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { APP_WEBVIEW_ARGS, launchApp, openWorkspace, sleep, stepper } from "../lib/app.mjs";
import { firstLaunchProject } from "../lib/flows.mjs";
import { appProcesses, externalConnections, networkUse, webViewProcesses } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "webview-network");

/** Collects every socket of the app's WebView2 processes over several samples. */
function tracer() {
  const use = { tcp: [], udp: [] };
  let procs = [];
  return {
    sample() {
      procs = webViewProcesses();
      const now = networkUse(procs.map((p) => p.pid));
      use.tcp.push(...now.tcp);
      use.udp.push(...now.udp);
    },
    async check(name) {
      assert.ok(procs.length >= 2, `the WebView2 processes of the app were not found: ${JSON.stringify(procs)}`);
      // The socket query must see a listener this spec opens itself, or "nothing seen" proves nothing.
      // (Through WebDriver the browser process also holds a loopback control connection; started
      // directly it may hold no socket at all, which is the goal.)
      const own = net.createServer();
      await new Promise((r) => own.listen(0, "127.0.0.1", r));
      try {
        assert.ok(networkUse([process.pid]).tcp.some((c) => c.state === "Listen"), "the socket query did not see this spec's own listener");
      } finally {
        own.close();
      }
      const appPids = new Set(appProcesses().map((p) => p.pid));
      const main = procs.find((p) => appPids.has(p.parent));
      const outside = externalConnections(use);
      const dir = join(import.meta.dirname, "..", "artifacts");
      mkdirSync(dir, { recursive: true });
      writeFileSync(
        join(dir, `${name}.json`),
        JSON.stringify(
          { at: new Date().toISOString(), browserCommandLine: main?.commandLine, processes: procs.map(({ pid, parent, name }) => ({ pid, parent, name })), use, outside },
          null,
          2,
        ),
      );
      assert.ok(main, "the WebView2 browser process of the app was not found");
      for (const arg of APP_WEBVIEW_ARGS) assert.ok(main.commandLine.includes(arg), `the web view did not get ${arg}: ${main.commandLine}`);
      assert.deepEqual(outside, [], `the web view used the network: ${JSON.stringify(outside)}`);
    },
  };
}

async function waitUntilAppGone(timeoutMs) {
  const until = Date.now() + timeoutMs;
  while (appProcesses().length > 0) {
    if (Date.now() > until) throw new Error("the app under test did not exit");
    await sleep(500);
  }
}

describe("the web view stays off the network", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("through WebDriver, with a project open, no WebView2 process of the app leaves this computer", async () => {
    await step("trace", async () => {
      await firstLaunchProject(browser, "Quiet Room");
      // Sample after every workspace and then every second for 10 s, and keep the union: one late
      // snapshot would miss a short connection that already closed.
      const t = tracer();
      for (const w of ["Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"]) {
        await openWorkspace(browser, w);
        t.sample();
      }
      for (let i = 0; i < 10; i++) {
        await sleep(1000);
        t.sample();
      }
      await t.check("webview-network-trace");
    });
  });

  test("started directly, as a person starts it, the web view stays on this computer for 45 s", async () => {
    await browser.deleteSession().catch(() => undefined);
    browser = undefined;
    await waitUntilAppGone(20_000);
    // The runner's environment carries the spec's own data root, so the app reopens the test data only.
    const app = spawn(process.env.E2E_APP, [], { env: process.env, stdio: "ignore" });
    try {
      // The W3 trace saw the runtime's background connections within 40 s of start.
      const t = tracer();
      for (let i = 0; i < 45; i++) {
        await sleep(1000);
        t.sample();
      }
      await t.check("webview-network-direct-trace");
    } finally {
      spawnSync("taskkill", ["/PID", String(app.pid), "/T", "/F"], { stdio: "ignore" });
      await waitUntilAppGone(20_000).catch(() => undefined);
    }
  });
});
