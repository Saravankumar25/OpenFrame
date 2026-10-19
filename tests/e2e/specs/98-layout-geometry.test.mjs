// Layout geometry in the real app (UX brief 2026-10-20, sections 4 and 11): every workspace and
// production tab, with the AI Assistant closed and open, at 1280x720 and 1920x1080 window sizes.
//
// Runs in the shared journey data root after 95-reachability and reopens the demo project
// "Night Bus". For each screen it measures, in the page itself:
//   * page-level sideways scroll (the document must never scroll sideways; boards and tables scroll
//     inside their own area);
//   * overlap between the page, the left navigation and the docked AI Assistant (a docked panel
//     narrows the page, it never covers it);
//   * buttons, tabs and fields of the page that sit outside the window, are cut off by a container
//     that cannot scroll, or are covered by another element (a modal dialog is not open here);
//   * text cut off inside its own box with no tooltip or label that carries the full text.
// The measurements go to tests/e2e/artifacts/layout-geometry.json. The spec fails on any finding.

import { after, before, describe, test } from "node:test";
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { click, launchApp, openWorkspace, sleep, stepper, waitForText } from "../lib/app.mjs";

const OUT = join(import.meta.dirname, "..", "artifacts");
const SIZES = [
  [1280, 720],
  [1920, 1080],
];
const WORKSPACES = ["Home", "Idea Vault", "Story", "Screenplay", "Breakdown", "Production", "Call Sheets", "Files"];
const PRODUCTION_TABS = [
  "Overview", "Catalog", "Locations", "Cast & Crew", "Moodboards", "Storyboards", "Shot Lists", "Schedule",
  "Day Out of Days", "Daily View", "Sides", "Reports", "Budget",
];

/** Runs in the page: the geometry findings of the screen as it is now. */
function measure() {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const se = document.scrollingElement;
  const rect = (sel) => document.querySelector(sel)?.getBoundingClientRect() ?? null;
  const overlap = (a, b) =>
    a && b ? Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left)) * Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top)) : 0;
  const main = rect("main.main");
  const ai = rect("aside.ai");
  const nav = rect("nav.nav");
  const label = (el) => (el.getAttribute("aria-label") || el.textContent || el.getAttribute("placeholder") || el.tagName).trim().replace(/\s+/g, " ").slice(0, 50);
  const findings = [];
  for (const el of document.querySelectorAll("main button, main [role=tab], main input, main select, main textarea")) {
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) continue;
    if (getComputedStyle(el).visibility === "hidden") continue;
    // Scrolled out of its own scroll area: reachable by scrolling, not a finding.
    let p = el.parentElement;
    let clipped = null;
    let scrolledAway = false;
    while (p && p !== document.body) {
      const s = getComputedStyle(p);
      const pr = p.getBoundingClientRect();
      const scrolls = /(auto|scroll)/.test(s.overflowX + " " + s.overflowY);
      const hides = /(hidden|clip)/.test(s.overflowX + " " + s.overflowY);
      if (scrolls) {
        if (r.bottom <= pr.top || r.top >= pr.bottom || r.right <= pr.left || r.left >= pr.right) scrolledAway = true;
        break;
      }
      if (hides && !clipped && (r.right > pr.right + 2 || r.left < pr.left - 2 || r.bottom > pr.bottom + 2 || r.top < pr.top - 2)) clipped = p;
      p = p.parentElement;
    }
    if (scrolledAway) continue;
    if (r.right > vw + 1 || r.left < -1 || r.bottom > vh + 1) {
      findings.push({ kind: "outside the window", control: label(el) });
      continue;
    }
    if (clipped) {
      findings.push({ kind: "cut off", control: label(el), by: String(clipped.className).slice(0, 60) });
      continue;
    }
    const x = Math.min(Math.max(r.left + Math.min(r.width / 2, 12), 0), vw - 1);
    const y = Math.min(Math.max(r.top + r.height / 2, 0), vh - 1);
    const hit = document.elementFromPoint(x, y);
    if (hit && !el.contains(hit) && !hit.contains(el) && !hit.closest(".toast, .menu-pop, [role=tooltip]")) {
      findings.push({ kind: "covered", control: label(el), by: String(hit.className || hit.tagName).slice(0, 60) });
    }
  }
  // Text cut off inside its own box (UX brief section 6): a box that hides overflow, holds text of its
  // own and is smaller than that text. An ellipsis is fine when the full text is in a tooltip or label.
  const textCuts = [];
  for (const el of document.querySelectorAll("main *")) {
    if (el.closest(".ProseMirror, svg, canvas, .sr-only")) continue;
    const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    if (!own || el.clientWidth < 1) continue;
    const s = getComputedStyle(el);
    if (s.visibility === "hidden") continue;
    const cutX = /(hidden|clip)/.test(s.overflowX) && el.scrollWidth > el.clientWidth + 2;
    const cutY = /(hidden|clip)/.test(s.overflowY) && el.scrollHeight > el.clientHeight + 2;
    if (!cutX && !cutY) continue;
    const full = el.textContent.trim();
    const tip = el.closest("[title], [aria-label]");
    const tipText = tip ? (tip.getAttribute("title") || tip.getAttribute("aria-label") || "").trim() : "";
    const told = tipText !== "" && (tipText.includes(full.slice(0, 20)) || full.includes(tipText.slice(0, 20)));
    if (told) continue;
    textCuts.push({ kind: "text cut", control: full.replace(/\s+/g, " ").slice(0, 50), by: String(el.className).slice(0, 60) });
    if (textCuts.length >= 12) break;
  }
  findings.push(...textCuts);
  return {
    pageOverflowX: Math.max(0, se.scrollWidth - se.clientWidth),
    pageOverflowY: Math.max(0, se.scrollHeight - se.clientHeight),
    mainWidth: main ? Math.round(main.width) : 0,
    overlapMainAi: Math.round(overlap(main, ai)),
    overlapNavMain: Math.round(overlap(nav, main)),
    findings,
  };
}

describe("layout geometry", () => {
  let browser;
  const step = stepper(() => browser);
  const report = [];

  before(async () => {
    browser = await launchApp();
  });
  after(async () => {
    mkdirSync(OUT, { recursive: true });
    writeFileSync(join(OUT, "layout-geometry.json"), JSON.stringify(report, null, 2));
    await browser?.deleteSession().catch(() => undefined);
  });

  test("every workspace fits beside the navigation and the docked AI Assistant", async () => {
    await step("open Night Bus", async () => {
      await browser.setWindowSize(1440, 900);
      await waitForText(browser, "Your projects");
      const row = await browser.$('//main//button[starts-with(@aria-label,"Open Night Bus")]');
      await row.waitForClickable({ timeout: 15_000 });
      await row.click();
      await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 20_000 });
      const keep = await browser.$('//button[normalize-space(.)="Keep Saved Version" and not(@disabled)]');
      if (await keep.waitForDisplayed({ timeout: 4000 }).catch(() => false)) await keep.click();
    });
    await step("add stress text (UX brief section 6.2)", async () => {
      const op = (name, args) =>
        browser.execute((n, a) => window.__TAURI_INTERNALS__.invoke("of_invoke", { op: n, args: a }).then(() => null, (e) => JSON.stringify(e)), name, args);
      const long = "A long card title that keeps going so that it is far wider than any column, ".repeat(4).trim();
      const unbroken = "https://example.invalid/" + "x".repeat(176);
      const mixed = "రైలు స్టేషన్ ரயில் நிலையம் रेलवे स्टेशन 🚉 Café naïve";
      for (const text of [long, unbroken, mixed]) {
        const err = await op("story.create_card", { shortDescription: text, parent: null, longDescription: null, notes: null, index: null });
        assert.equal(err, null, `story.create_card: ${err}`);
      }
      const err = await op("story.create_character", { name: "Venkata Satyanarayana Lakshmi Narasimha Rao Garu of Pedda Revu", episodeId: null, imagePath: null });
      assert.equal(err, null, `story.create_character: ${err}`);
    });
    const record = async (size, ai, screen) => {
      await sleep(400);
      const m = await browser.execute(measure);
      report.push({ size: size.join("x"), ai, screen, ...m });
    };
    const setAi = async (open) => {
      const btn = await browser.$('//header//button[@aria-label="Assistant"]');
      const pressed = (await btn.getAttribute("aria-pressed")) === "true";
      if (pressed !== open) await btn.click();
      await sleep(300);
    };
    for (const size of SIZES) {
      for (const ai of [false, true]) {
        await step(`${size.join("x")} AI ${ai ? "open" : "closed"}`, async () => {
          await browser.setWindowSize(size[0], size[1]);
          await setAi(ai);
          for (const ws of WORKSPACES) {
            await openWorkspace(browser, ws);
            if (ws === "Production") {
              for (const tab of PRODUCTION_TABS) {
                await click(browser, tab, { within: '//nav[@aria-label="Production"]' });
                await record(size, ai, `Production · ${tab}`);
              }
            } else if (ws === "Story") {
              for (const tab of ["Board", "Outline", "Characters", "Timeline"]) {
                await click(browser, tab, { within: '//div[@aria-label="Story views"]' });
                await record(size, ai, `Story · ${tab}`);
              }
              await click(browser, "Board", { within: '//div[@aria-label="Story views"]' });
            } else {
              await record(size, ai, ws);
            }
          }
          await click(browser, "Settings", { within: "//header" });
          await record(size, ai, "Settings");
        });
      }
    }
    const failures = report.filter((r) => r.pageOverflowX > 1 || r.overlapMainAi > 0 || r.overlapNavMain > 0 || r.findings.length > 0);
    assert.deepEqual(
      failures.map((r) => `${r.size} AI ${r.ai ? "open" : "closed"} ${r.screen}: x-overflow ${r.pageOverflowX}, AI overlap ${r.overlapMainAi}, ${r.findings.map((f) => `${f.kind} ${f.control}`).join("; ")}`),
      [],
    );
  });
});
