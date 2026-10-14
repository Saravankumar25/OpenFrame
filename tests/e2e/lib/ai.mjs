// Helpers for the Offline AI specs (72..76): drive the AI Assistant panel the way a user does and
// inspect the real app and engine processes. The app under test carries the real bundled model
// (ADR-0025; build-app.mjs) and runs with deterministic sampling (OPENFRAME_AI_DETERMINISTIC=1,
// set by run.mjs: temperature 0, fixed seed), so the specs assert structure and effects, never the
// wording of an answer. One real answer takes 20 to 40 seconds on a laptop processor.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { closeSync, copyFileSync, existsSync, linkSync, mkdirSync, openSync, readSync, readdirSync, rmSync, statSync, writeSync } from "node:fs";
import { dirname, join } from "node:path";
import { click, sleep } from "./app.mjs";
import { projectMenu } from "./flows.mjs";

/** A three-scene screenplay (Fountain) the AI specs ask about. */
export const NIGHT_BUS = [
  "Title: Night Bus",
  "",
  "INT. POLICE STATION - NIGHT",
  "",
  "Arjun confesses to Inspector Rao.",
  "",
  "ARJUN",
  "I was there that night.",
  "",
  "EXT. BUS STOP - NIGHT",
  "",
  "Arjun meets Meera again at a rainy bus stop.",
  "",
  "MEERA",
  "You came back.",
  "",
  "EXT. RAILWAY STATION - DAY",
  "",
  "Meera waits for the late train with a red umbrella.",
  "",
].join("\n");

/** Generous: model load on first use plus a multi-step answer on a laptop processor. */
export const ANSWER_TIMEOUT = 240_000;

const panel = '//aside[@aria-label="AI Assistant"]';

/** Open the AI Assistant panel from the header (the one entry point in every workspace). */
export async function openAssistant(browser) {
  const open = await browser.$(panel);
  if (await open.isExisting()) return;
  await click(browser, "Assistant");
  await (await browser.$(panel)).waitForDisplayed({ timeout: 10_000 });
}

export async function closeAssistant(browser) {
  if (await (await browser.$(panel)).isExisting()) await click(browser, "Close AI Assistant");
}

/**
 * How the page and the docked AI Assistant share the window (owner decision 2026-10-08: the panel
 * docks beside the page). Returns the page and panel edges and every visible control of the page
 * that ends past the page's right edge without a horizontal scroll area that could bring it into
 * view. With the panel docked, `overlap` is 0 and `hidden` is empty at every supported window size.
 */
export async function dockedLayout(browser) {
  return browser.execute(() => {
    const main = document.querySelector("main.main");
    const aside = document.querySelector('aside[aria-label="AI Assistant"]');
    const mr = main.getBoundingClientRect();
    const ar = aside ? aside.getBoundingClientRect() : null;
    const scrollsSideways = (el) => {
      for (let a = el.parentElement; a && a !== main.parentElement; a = a.parentElement) {
        const o = getComputedStyle(a).overflowX;
        if (o === "auto" || o === "scroll") return true;
      }
      return false;
    };
    const hidden = [];
    for (const el of main.querySelectorAll('button, input, select, textarea, a[href], [role="tab"], [role="button"], [contenteditable="true"]')) {
      const r = el.getBoundingClientRect();
      if (r.width === 0 || r.height === 0 || getComputedStyle(el).visibility === "hidden") continue;
      if (r.right > mr.right + 1 && !scrollsSideways(el)) {
        hidden.push((el.getAttribute("aria-label") || el.textContent || el.tagName).trim().slice(0, 60));
      }
    }
    return {
      mainLeft: Math.round(mr.left),
      mainRight: Math.round(mr.right),
      mainWidth: Math.round(mr.width),
      panelLeft: ar ? Math.round(ar.left) : null,
      panelWidth: ar ? Math.round(ar.width) : 0,
      overlap: ar ? Math.max(0, Math.round(mr.right - ar.left)) : 0,
      viewport: window.innerWidth,
      hidden,
    };
  });
}

/** The panel's visible text. */
export async function panelText(browser) {
  return browser.execute(() => document.querySelector('aside[aria-label="AI Assistant"]')?.innerText ?? "");
}

/** The "Using: <scope>" line and the selected scope. */
export async function scopeInfo(browser) {
  return browser.execute(() => {
    const aside = document.querySelector('aside[aria-label="AI Assistant"]');
    const using = [...(aside?.querySelectorAll(".xs.muted") ?? [])].map((e) => e.textContent ?? "").find((t) => t.startsWith("Using: ")) ?? "";
    const select = aside?.querySelector("#ai-scope");
    const selected = select ? (select.textContent ?? "").trim() : "";
    return { using, selected };
  });
}

/** All assistant results in the conversation, oldest first. */
export async function results(browser) {
  return browser.execute(() =>
    [...document.querySelectorAll('aside[aria-label="AI Assistant"] .msg.a[data-kind]')].map((m) => ({
      kind: m.getAttribute("data-kind"),
      text: m.innerText,
      sources: [...m.querySelectorAll(".ai-sources .chip, .ai-sources button")].map((c) => c.textContent?.trim() ?? ""),
      exact: m.innerText.includes("Exact · from project data"),
      inferred: m.innerText.includes("Inferred · written by Offline AI"),
    })),
  );
}

/** Type a question into the panel and send it (Enter), without waiting for the answer. */
export async function send(browser, text) {
  await openAssistant(browser);
  const box = await browser.$(`${panel}//textarea[@aria-label="Ask or command"]`);
  await box.waitForEnabled({ timeout: 30_000 });
  await box.click();
  await box.setValue(text);
  await browser.keys("Enter");
}

/**
 * Ask the assistant through the panel (type, Enter) and wait for the new result. Returns it as
 * { kind, text, sources, exact, inferred }.
 */
export async function ask(browser, text, { timeout = ANSWER_TIMEOUT } = {}) {
  await openAssistant(browser);
  const before = (await results(browser)).length;
  await send(browser, text);
  // The newest result must answer this request: the conversation restored when the panel opens
  // can arrive after `before` was counted, and its last result belongs to an earlier question.
  const answered = () =>
    browser.execute((q) => {
      const aside = document.querySelector('aside[aria-label="AI Assistant"]');
      if (!aside || aside.querySelector('.msg.a.muted[role="status"]')) return false;
      const all = [...aside.querySelectorAll(".msg.a[data-kind]")];
      const asked = all[all.length - 1]?.previousElementSibling;
      return !!asked && asked.matches(".msg.u") && (asked.textContent ?? "").trim() === q.trim();
    }, text);
  await browser.waitUntil(async () => (await results(browser)).length > before && (await answered()), {
    timeout,
    interval: 1_000,
    timeoutMsg: `no answer to "${text}" within ${timeout / 1000} s`,
  });
  const all = await results(browser);
  return all[all.length - 1];
}

/** The newest "Proposed Changes" card: its text and the buttons it offers. */
export async function proposal(browser) {
  return browser.execute(() => {
    const cards = [...document.querySelectorAll('aside[aria-label="AI Assistant"] section[aria-label="Proposed Changes"]')];
    const card = cards[cards.length - 1];
    if (!card) return null;
    let n = card.nextElementSibling;
    const after = [];
    while (n && !n.matches(".msg")) {
      after.push(n);
      n = n.nextElementSibling;
    }
    const buttons = after.flatMap((e) => [...e.querySelectorAll("button")]).map((b) => b.textContent?.trim() ?? "");
    const state = after.map((e) => (e.matches("[role=status],[role=alert]") ? e.textContent?.trim() ?? "" : "")).filter(Boolean).join(" ");
    return { text: card.innerText, stale: card.classList.contains("ai-stale"), buttons, state };
  });
}

/** Click a button that belongs to the newest Proposed Changes card. */
export async function proposalButton(browser, name) {
  const cards = await browser.$$(`${panel}//section[@aria-label="Proposed Changes"]`);
  assert.ok(cards.length > 0, "no Proposed Changes card");
  const b = await browser.$(
    `(${panel}//section[@aria-label="Proposed Changes"])[last()]/following-sibling::*[not(contains(@class,"msg"))]//button[normalize-space(.)="${name}"]`,
  );
  await b.waitForClickable({ timeout: 15_000, timeoutMsg: `the proposal offers no "${name}" button` });
  await b.click();
}

/** Open tasks listed in Notes & Tasks (the Tasks section). */
export async function openTaskTitles(browser) {
  return browser.execute(() =>
    [...document.querySelectorAll('section[aria-label="Tasks"] input[aria-label^="Mark "]')].map((i) => (i.getAttribute("aria-label") ?? "").replace(/^Mark “/, "").replace(/” done$/, "")),
  );
}

/** Open Notes & Tasks and wait for the Tasks section. The assistant panel lies over the right of
 * the page, where the Tasks column and its buttons are, so it is closed first, as a person would. */
export async function goToTasks(browser) {
  await closeAssistant(browser);
  await projectMenu(browser, "Notes & Tasks");
  await (await browser.$('//section[@aria-label="Tasks"]')).waitForDisplayed({ timeout: 15_000 });
  await sleep(500);
}

// ------------------------------------------------------------------ processes (Windows)

function ps(script) {
  return execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], {
    encoding: "utf8",
    windowsHide: true,
    env: { ...process.env, E2E_AI_ENGINE_DIR: SPEC_AI_DIR || BUILD_AI_DIR },
  });
}

/** The app under test and the Offline AI engines it started: [{ pid, parent, name, path, commandLine }]. */
export function appProcesses() {
  const app = process.env.E2E_APP;
  const out = ps(
    `$app = Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $env:E2E_APP }; ` +
      `$ids = @($app | ForEach-Object { $_.ProcessId }); ` +
      `$eng = Get-CimInstance Win32_Process -Filter "Name='llama-server.exe'" | Where-Object { $ids -contains $_.ParentProcessId }; ` +
      `@($app) + @($eng) | Where-Object { $_ } | ForEach-Object { [pscustomobject]@{ pid = $_.ProcessId; parent = $_.ParentProcessId; name = $_.Name; path = $_.ExecutablePath; commandLine = $_.CommandLine } } | ConvertTo-Json -Compress`,
  ).trim();
  if (!out) return [];
  const v = JSON.parse(out);
  assert.ok(app, "E2E_APP is not set");
  return Array.isArray(v) ? v : [v];
}

/** Every TCP connection and UDP endpoint owned by the given processes. */
export function networkUse(pids) {
  if (pids.length === 0) return { tcp: [], udp: [] };
  const list = pids.join(",");
  const out = ps(
    `$p = @(${list}); ` +
      `$t = @(Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $p -contains $_.OwningProcess } | ForEach-Object { [pscustomobject]@{ pid = $_.OwningProcess; local = $_.LocalAddress; remote = $_.RemoteAddress; state = "$($_.State)" } }); ` +
      `$u = @(Get-NetUDPEndpoint -ErrorAction SilentlyContinue | Where-Object { $p -contains $_.OwningProcess } | ForEach-Object { [pscustomobject]@{ pid = $_.OwningProcess; local = $_.LocalAddress } }); ` +
      `[pscustomobject]@{ tcp = $t; udp = $u } | ConvertTo-Json -Compress -Depth 4`,
  ).trim();
  const v = JSON.parse(out || '{"tcp":[],"udp":[]}');
  const arr = (x) => (x == null ? [] : Array.isArray(x) ? x : [x]);
  return { tcp: arr(v.tcp), udp: arr(v.udp) };
}

const LOOPBACK = new Set(["127.0.0.1", "::1", "0.0.0.0", "::"]);

/** Connections that leave this computer (anything not on loopback). */
export function externalConnections(use) {
  return [
    ...use.tcp.filter((c) => !LOOPBACK.has(c.remote) || (c.state === "Listen" && !["127.0.0.1", "::1"].includes(c.local))),
    ...use.udp.filter((u) => !["127.0.0.1", "::1"].includes(u.local)),
  ];
}

/** End the app under test abruptly (as a crash or power loss would), engines included. */
export function killApp() {
  ps(
    `Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -eq $env:E2E_APP } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`,
  );
}

// ------------------------------------------------------------------ bundled AI copies

/** The bundled AI of the test build (`ai/` next to the executable). */
export const BUILD_AI_DIR = join(dirname(process.env.E2E_APP ?? "."), "ai");

/** The AI folder this spec's app reads (run.mjs sets it for `// @e2e-ai-bundle: own` specs). */
export const SPEC_AI_DIR = process.env.E2E_AI_BUNDLE_DIR ?? "";

function walk(dir, base = dir, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) walk(p, base, out);
    else out.push(p.slice(base.length + 1));
  }
  return out;
}

/**
 * Lay out a bundle for SPEC_AI_DIR from the test build's: `copy` lists relative paths that get a
 * real copy (so they can be altered), everything else is hard-linked (no second gigabyte).
 * `skip` lists relative paths left out (a missing file).
 */
export function layOutBundle({ copy = [], skip = [] } = {}) {
  assert.ok(SPEC_AI_DIR, "this spec needs `// @e2e-ai-bundle: own`");
  assert.ok(existsSync(BUILD_AI_DIR), `the test build has no bundled AI in ${BUILD_AI_DIR}`);
  rmSync(SPEC_AI_DIR, { recursive: true, force: true });
  for (const rel of walk(BUILD_AI_DIR)) {
    const norm = rel.replace(/\\/g, "/");
    if (skip.includes(norm)) continue;
    const src = join(BUILD_AI_DIR, rel);
    const dst = join(SPEC_AI_DIR, rel);
    mkdirSync(dirname(dst), { recursive: true });
    if (copy.includes(norm)) copyFileSync(src, dst);
    else {
      try {
        linkSync(src, dst);
      } catch {
        copyFileSync(src, dst);
      }
    }
  }
}

/** Relative path (with `/`) of the first bundled file matching `test`. */
export function bundledFile(test) {
  const rel = walk(BUILD_AI_DIR)
    .map((r) => r.replace(/\\/g, "/"))
    .find(test);
  assert.ok(rel, "no such bundled file");
  return rel;
}

export function fileSize(rel) {
  return statSync(join(SPEC_AI_DIR, rel)).size;
}

/** Flip one byte in the middle of a copied bundle file (never a hard link into the test build). */
export function alterByte(rel) {
  const p = join(SPEC_AI_DIR, rel);
  assert.equal(statSync(p).nlink, 1, `${rel} must be a copy before it is altered`);
  const fd = openSync(p, "r+");
  try {
    const pos = Math.floor(statSync(p).size / 2);
    const b = Buffer.alloc(1);
    readSync(fd, b, 0, 1, pos);
    b[0] ^= 0x5a;
    writeSync(fd, b, 0, 1, pos);
  } finally {
    closeSync(fd);
  }
}

/** Every engine program running from the test build's bundled AI, whoever started it. */
export function enginesFromBuild() {
  const out = ps(
    `$dir = $env:E2E_AI_ENGINE_DIR; Get-CimInstance Win32_Process -Filter "Name='llama-server.exe'" | ` +
      `Where-Object { $_.ExecutablePath -and $_.ExecutablePath.ToLower().StartsWith($dir.ToLower()) } | ` +
      `ForEach-Object { [pscustomobject]@{ pid = $_.ProcessId; path = $_.ExecutablePath } } | ConvertTo-Json -Compress`,
  ).trim();
  if (!out) return [];
  const v = JSON.parse(out);
  return Array.isArray(v) ? v : [v];
}
