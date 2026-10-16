// @e2e-data-root: own
//
// Keyboard and screen reader pass (Windows phase W4) on the eight main workspaces: Home, Idea Vault,
// Story, Screenplay, Breakdown, Production, Call Sheets and Files, with a three-scene screenplay in
// the project. For each workspace:
//   * keyboard: press Tab from the top of the page until focus wraps past the last control (the web
//     view has no browser bar, so focus then rests on the page itself) or 80 presses; every stop must
//     be a visible element with an accessible name and a visible focus mark (an outline or ring on the
//     element or on the field box around it, or a caret in an editor); an editor that uses Tab for
//     itself (the screenplay's element flow) must say in its accessible description that Escape leaves
//     it, and Escape then Tab must move on (WCAG 2.1.2, no keyboard trap);
//   * screen reader: read the window through Windows UI Automation, the tree Narrator reads, and list
//     every button, field, box, tab, link and menu item without a name.
// Findings go to tests/e2e/artifacts/keyboard-and-screen-reader.json; any finding fails the spec.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { launchApp, openWorkspace, sleep, stepper } from "../lib/app.mjs";
import { firstLaunchProject, importFountain } from "../lib/flows.mjs";
import { NIGHT_BUS, appProcesses } from "../lib/ai.mjs";

const WORKSPACES = ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"];
const TABS = 80;
let browser;
const step = stepper(() => browser, "keyboard-and-screen-reader");
const findings = {};

/** What has focus now: a short description and whether it is named, visible and visibly focused. */
const focused = () =>
  browser.execute(() => {
    const el = document.activeElement;
    if (!el || el === document.body || el === document.documentElement) return { body: true };
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const ringOn = (s) => (s.outlineStyle !== "none" && parseFloat(s.outlineWidth) > 0) || (s.boxShadow && s.boxShadow !== "none");
    // A field inside a styled box (label.input) shows the ring on the box (.input:focus-within).
    const box = el.parentElement?.closest(".input");
    const ring = ringOn(cs) || (box && ringOn(getComputedStyle(box)));
    const by = el.getAttribute("aria-labelledby");
    const name =
      el.getAttribute("aria-label")?.trim() ||
      (by && by.split(/\s+/).map((id) => document.getElementById(id)?.textContent?.trim()).filter(Boolean).join(" ")) ||
      (el.id && document.querySelector(`label[for="${CSS.escape(el.id)}"]`)?.textContent?.trim()) ||
      el.closest("label")?.textContent?.trim() ||
      el.getAttribute("title")?.trim() ||
      el.getAttribute("placeholder")?.trim() ||
      (el.matches("button, a, [role=button], [role=tab], [role=menuitem], [role=option], [role=treeitem], summary") ? el.textContent?.trim() : "") ||
      "";
    const desc = `${el.tagName.toLowerCase()}${el.getAttribute("role") ? `[role=${el.getAttribute("role")}]` : ""} "${(name || el.outerHTML).slice(0, 70)}"`;
    // ProseMirror and the screenplay editor show a caret, not a ring, which is the platform's own focus mark.
    const caret = el.isContentEditable;
    return { desc, named: !!name, visible: r.width > 0 && r.height > 0, ring: ring || caret, hint: el.getAttribute("aria-description") ?? "" };
  });

/** Controls in the app window without a name, as Windows UI Automation (and so Narrator) sees them. */
function unnamedForScreenReader(pid) {
  const script = [
    "Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes",
    "$A = [System.Windows.Automation.AutomationElement]",
    `$cond = New-Object System.Windows.Automation.PropertyCondition($A::ProcessIdProperty, ${pid})`,
    "$win = $A::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)",
    "if (-not $win) { 'NO WINDOW'; exit }",
    "$types = @('Button','Edit','ComboBox','CheckBox','RadioButton','MenuItem','TabItem','Hyperlink','Slider','ListItem','TreeItem')",
    "$all = $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)",
    "$n = 0; $named = 0",
    "foreach ($e in $all) { $n++; if ($n -gt 4000) { break }; $c = $e.Current; $t = $c.ControlType.ProgrammaticName -replace 'ControlType.',''; if ($types -contains $t -and -not $c.IsOffscreen) { if ([string]::IsNullOrWhiteSpace($c.Name)) { \"$t|$($c.AutomationId)|$($c.ClassName)\" } else { $named++ } } }",
    "\"NAMED $named\"",
  ].join("; ");
  const out = execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8", windowsHide: true, timeout: 120_000 });
  const lines = out.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  assert.ok(!lines.includes("NO WINDOW"), "the app window was not found through UI Automation");
  // The tree must show the page's named controls, or "no unnamed control" proves nothing.
  const named = Number(lines.find((l) => l.startsWith("NAMED "))?.slice(6) ?? 0);
  assert.ok(named >= 5, `UI Automation saw only ${named} named controls; the page's tree was not read`);
  return lines.filter((l) => !l.startsWith("NAMED "));
}

describe("keyboard and screen reader pass", () => {
  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    await browser?.deleteSession().catch(() => undefined);
    const dir = join(import.meta.dirname, "..", "artifacts");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "keyboard-and-screen-reader.json"), JSON.stringify({ at: new Date().toISOString(), release: process.env.E2E_RELEASE ?? "debug", findings }, null, 2));
  });

  test("a project with a screenplay to walk through", async () => {
    await step("project", async () => {
      await firstLaunchProject(browser, "Quiet Keys");
      await importFountain(browser, NIGHT_BUS);
    });
  });

  for (const name of WORKSPACES) {
    test(`${name}: every Tab stop is named and visibly focused, and the window has no unnamed control`, async () => {
      await step(name, async () => {
        await openWorkspace(browser, name);
        await sleep(800);
        await browser.execute(() => {
          document.activeElement?.blur();
          window.scrollTo(0, 0);
        });
        const issues = [];
        let last = null;
        let repeats = 0;
        for (let i = 0; i < TABS; i++) {
          await browser.keys("Tab");
          const f = await focused();
          // Past the last control focus rests on the page: the cycle is complete.
          if (f.body) break;
          if (!f.visible) issues.push(`Tab ${i + 1}: focus is on a hidden element: ${f.desc}`);
          if (!f.named) issues.push(`Tab ${i + 1}: no accessible name: ${f.desc}`);
          if (!f.ring) issues.push(`Tab ${i + 1}: no visible focus mark: ${f.desc}`);
          repeats = f.desc === last ? repeats + 1 : 0;
          if (repeats >= 3) {
            if (!/escape/i.test(f.hint)) {
              issues.push(`Tab ${i + 1}: focus is trapped on ${f.desc} and its description names no way out`);
              break;
            }
            await browser.keys("Escape");
            await browser.keys("Tab");
            const out = await focused();
            if (!out.body && out.desc === f.desc) {
              issues.push(`Tab ${i + 1}: Escape then Tab did not leave ${f.desc}`);
              break;
            }
            repeats = 0;
            last = out.desc ?? null;
            continue;
          }
          last = f.desc;
        }
        const app = appProcesses().find((p) => p.name.toLowerCase() === "openframe-desktop.exe");
        const unnamed = unnamedForScreenReader(app.pid).map((u) => `screen reader: unnamed ${u}`);
        findings[name] = [...new Set([...issues, ...unnamed])];
        assert.deepEqual(findings[name], [], `${name}:\n  ${findings[name].join("\n  ")}`);
      });
    });
  }
});
