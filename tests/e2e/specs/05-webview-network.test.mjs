// @e2e-data-root: own
//
// Threat T33 (Windows phase W3): the web view never talks to the network. The AI specs check the app
// and its engines; this one finds every WebView2 process that belongs to the app (the browser
// process and its renderers, GPU and utility processes), walks the main workspaces with a project
// open, waits, and lists every socket those processes own. Anything that is not loopback fails,
// and the list of processes and endpoints is written to the artifacts folder as evidence.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { launchApp, openWorkspace, sleep, stepper } from "../lib/app.mjs";
import { firstLaunchProject } from "../lib/flows.mjs";
import { externalConnections, networkUse, webViewProcesses } from "../lib/ai.mjs";

let browser;
const step = stepper(() => browser, "webview-network");

describe("the web view stays off the network", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
  });

  test("no WebView2 process of the app holds a connection beyond this computer", async () => {
    await step("trace", async () => {
      await firstLaunchProject(browser, "Quiet Room");
      // Sample after every workspace and then every second for 10 s, and keep the union: one late
      // snapshot would miss a short connection that already closed.
      const use = { tcp: [], udp: [] };
      let procs = [];
      const sample = () => {
        procs = webViewProcesses();
        const now = networkUse(procs.map((p) => p.pid));
        use.tcp.push(...now.tcp);
        use.udp.push(...now.udp);
      };
      for (const w of ["Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"]) {
        await openWorkspace(browser, w);
        sample();
      }
      for (let i = 0; i < 10; i++) {
        await sleep(1000);
        sample();
      }
      assert.ok(procs.length >= 2, `the WebView2 processes of the app were not found: ${JSON.stringify(procs)}`);
      // The browser process always holds its loopback control connection: none at all means the query failed.
      assert.ok(use.tcp.length > 0, "no socket was seen at all; the network query did not work");
      const outside = externalConnections(use);
      const dir = join(import.meta.dirname, "..", "artifacts");
      mkdirSync(dir, { recursive: true });
      writeFileSync(
        join(dir, "webview-network-trace.json"),
        JSON.stringify({ at: new Date().toISOString(), processes: procs.map(({ pid, parent, name }) => ({ pid, parent, name })), use, outside }, null, 2),
      );
      assert.deepEqual(outside, [], `the web view used the network: ${JSON.stringify(outside)}`);
    });
  });
});
