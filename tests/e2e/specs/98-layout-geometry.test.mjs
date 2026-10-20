// Layout geometry in the real app (UX brief 2026-10-20, sections 4 and 11): every workspace and
// production tab, with the AI Assistant closed and open, at 1200x720 (the minimum), 1280x720 and 1920x1080.
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
// 1200 by 720 is the window minimum (tauri.conf.json), the narrowest a person can make it.
const SIZES = [
  [1200, 720],
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
        // Wholly or partly past the edge of its scrolling area: scrolling brings it into view.
        if (r.bottom > pr.bottom + 1 || r.top < pr.top - 1 || r.right > pr.right + 1 || r.left < pr.left - 1) scrolledAway = true;
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
  // A scrolling area that itself runs past the window or under the status bar: the end of its content
  // can never be reached.
  const floor = document.querySelector("footer.status")?.getBoundingClientRect().top ?? vh;
  for (const el of document.querySelectorAll("main *")) {
    const s = getComputedStyle(el);
    if (!/(auto|scroll)/.test(s.overflowY) || el.scrollHeight <= el.clientHeight + 1) continue;
    const r = el.getBoundingClientRect();
    if (r.height < 1 || el.closest("[aria-hidden='true']")) continue;
    if (r.bottom > floor + 1) findings.push({ kind: "scroll area past the window", control: String(el.className).slice(0, 50), by: `bottom ${Math.round(r.bottom)} of ${Math.round(floor)}` });
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

/** Runs in the page on Story > Outline: the outline list is its own scroll area and reaches its last row. */
function probeOutlineScroll() {
  const area = document.querySelector(".story-canvas-outline");
  if (!area) return { ok: false, why: "no outline scroll area" };
  const toolbar = document.querySelector("main .toolbar");
  const top = toolbar?.getBoundingClientRect().top ?? 0;
  const overflowY = getComputedStyle(area).overflowY;
  const scrollable = area.scrollHeight > area.clientHeight + 1;
  area.scrollTop = area.scrollHeight;
  const reached = !scrollable || area.scrollTop > 0;
  const rows = area.querySelectorAll("[role=treeitem]");
  const last = rows[rows.length - 1]?.getBoundingClientRect();
  const lastVisible = !last || last.bottom <= area.getBoundingClientRect().bottom + 1;
  const toolbarStays = !toolbar || Math.abs(toolbar.getBoundingClientRect().top - top) < 1;
  area.scrollTop = 0;
  area.focus();
  return { ok: /(auto|scroll)/.test(overflowY) && reached && lastVisible && toolbarStays, overflowY, scrollable, reached, lastVisible, toolbarStays };
}

/** Runs in the page on Settings > Offline AI with Technical details open: the cards keep usable widths, the component
 * table stays inside its card and no card covers another. */
function probeOfflineAi() {
  const cards = [...document.querySelectorAll("main section.card")].map((el) => ({ name: el.getAttribute("aria-label") || "card", r: el.getBoundingClientRect() }));
  const tech = cards.find((c) => c.name === "Technical details");
  const main = document.querySelector("main.main")?.getBoundingClientRect();
  if (!tech || !main) return { ok: false, why: "no Technical details card" };
  const table = document.querySelector('section[aria-label="Technical details"] table');
  const t = table?.getBoundingClientRect();
  const overlaps = [];
  for (let i = 0; i < cards.length; i++) {
    for (let j = i + 1; j < cards.length; j++) {
      const a = cards[i].r;
      const b = cards[j].r;
      if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1) overlaps.push(cards[i].name + " / " + cards[j].name);
    }
  }
  const narrow = cards.filter((c) => c.r.width < 300).map((c) => c.name + " " + Math.round(c.r.width));
  const tableInside = !t || (t.right <= tech.r.right + 1 && t.left >= tech.r.left - 1);
  const wide = tech.r.width >= Math.min(600, main.width - 60);
  return { ok: overlaps.length === 0 && narrow.length === 0 && tableInside && wide, overlaps, narrow, tableInside, wide, techWidth: Math.round(tech.r.width) };
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
      // The project finishes opening in the core a moment after the page shows: a read that needs an
      // open project is retried until it answers.
      await browser.waitUntil(async () => (await op("story.board", { episodeId: null })) === null, {
        timeout: 20_000,
        timeoutMsg: "the project did not open in the core",
      });
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
          for (const tab of ["Project settings", "Writing", "Keyboard shortcuts", "Templates", "Offline AI", "Advanced", "Appearance"]) {
            await click(browser, tab, { within: '//div[@aria-label="Settings"]' });
            await record(size, ai, `Settings · ${tab}`);
            if (tab === "Offline AI") {
              const summary = await browser.$('//section[@aria-label="Technical details"]//summary');
              if (await summary.isExisting()) {
                await summary.click();
                await sleep(300);
                report[report.length - 1].offlineAi = await browser.execute(probeOfflineAi);
                await summary.click();
              }
            }
          }
        });
      }
    }
    const failures = report.filter(
      (r) => r.pageOverflowX > 1 || r.overlapMainAi > 0 || r.overlapNavMain > 0 || r.findings.length > 0 || (r.outlineScroll && (!r.outlineScroll.ok || r.outlineScroll.keyboard === false)) || (r.offlineAi && !r.offlineAi.ok),
    );
    assert.deepEqual(
      failures.map((r) => `${r.size} AI ${r.ai ? "open" : "closed"} ${r.screen}: x-overflow ${r.pageOverflowX}, AI overlap ${r.overlapMainAi}, ${r.findings.map((f) => `${f.kind} ${f.control}`).join("; ")}${r.outlineScroll && (!r.outlineScroll.ok || r.outlineScroll.keyboard === false) ? ` outline scroll ${JSON.stringify(r.outlineScroll)}` : ""}${r.offlineAi && !r.offlineAi.ok ? ` offline AI ${JSON.stringify(r.offlineAi)}` : ""}`),
      [],
    );
  });
});
