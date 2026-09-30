// Shared helpers for the desktop E2E specs: launch the real app through
// tauri-driver, find controls the way a user does (visible labels / accessible
// names), capture console problems and failure screenshots.

import { remote } from "webdriverio";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

const ARTIFACTS = join(import.meta.dirname, "..", "artifacts");

export async function launchApp() {
  if (!process.env.E2E_APP) throw new Error("Run the suite with `npm run test:e2e` (E2E_APP is not set).");
  const browser = await remote({
    hostname: "127.0.0.1",
    port: Number(process.env.E2E_PORT ?? 4444),
    logLevel: "warn",
    connectionRetryTimeout: 120_000,
    capabilities: {
      browserName: "wry",
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": { application: process.env.E2E_APP },
    },
  });
  browser.setTimeout({ implicit: 0 });
  await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 30_000 });
  await installConsoleProbe(browser);
  return browser;
}

/** Record console errors/warnings and uncaught errors inside the webview. */
export async function installConsoleProbe(browser) {
  await browser.execute(() => {
    const w = window;
    if (w.__e2eProblems) return;
    w.__e2eProblems = [];
    const push = (kind, args) => {
      const text = args
        .map((a) => {
          if (a instanceof Error) return `${a.name}: ${a.message}`;
          if (typeof a === "string") return a;
          try {
            return JSON.stringify(a);
          } catch {
            return String(a);
          }
        })
        .join(" ");
      w.__e2eProblems.push(`${kind}: ${text}`.slice(0, 2000));
    };
    for (const kind of ["error", "warn"]) {
      const orig = console[kind].bind(console);
      console[kind] = (...args) => {
        push(kind, args);
        orig(...args);
      };
    }
    window.addEventListener("error", (e) => push("uncaught", [e.message]));
    window.addEventListener("unhandledrejection", (e) => push("unhandledrejection", [e.reason]));
  });
}

/** Console problems recorded since the last call (and clears them). */
export async function takeConsoleProblems(browser) {
  return browser.execute(() => {
    const p = window.__e2eProblems ?? [];
    window.__e2eProblems = [];
    return p;
  });
}

/**
 * Visible form controls and buttons without an accessible name (aria-label,
 * aria-labelledby, title, <label>, or visible text for buttons).
 */
export async function unlabelledControls(browser) {
  return browser.execute(() => {
    const visible = (el) => {
      const r = el.getBoundingClientRect();
      return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== "hidden" && !el.closest("[aria-hidden=true]");
    };
    const named = (el) => {
      if (el.getAttribute("aria-label")?.trim() || el.getAttribute("title")?.trim()) return true;
      const by = el.getAttribute("aria-labelledby");
      if (by && by.split(/\s+/).some((id) => document.getElementById(id)?.textContent?.trim())) return true;
      if (el.id && document.querySelector(`label[for="${CSS.escape(el.id)}"]`)?.textContent?.trim()) return true;
      if (el.closest("label")?.textContent?.trim()) return true;
      if (el.matches("button, [role=button], [role=menuitem], [role=tab], [role=option], a") && el.textContent?.trim()) return true;
      return false;
    };
    return [...document.querySelectorAll("button, input:not([type=hidden]), textarea, select, [role=button], [role=menuitem], [role=tab], [contenteditable=true]")]
      .filter((el) => visible(el) && !el.matches(".ProseMirror *") && !named(el) && !(el.matches("[contenteditable=true]") && el.getAttribute("role") === "textbox" && el.getAttribute("aria-label")))
      .map((el) => el.outerHTML.slice(0, 160));
  });
}

export async function screenshot(browser, name) {
  mkdirSync(ARTIFACTS, { recursive: true });
  const file = join(ARTIFACTS, `${name.replace(/[^a-z0-9-_]+/gi, "_").slice(0, 80)}.png`);
  try {
    await browser.saveScreenshot(file);
  } catch {
    // The window may already be gone.
  }
  return file;
}

/**
 * A named step: on failure, saves a screenshot (and the visible text) to
 * tests/e2e/artifacts/ before rethrowing.
 */
export function stepper(browser, prefix) {
  let n = 0;
  return async (name, fn) => {
    n += 1;
    try {
      await fn();
    } catch (e) {
      const file = await screenshot(browser(), `${prefix}-${String(n).padStart(2, "0")}-${name}`);
      const msg = e instanceof Error ? e.message : String(e);
      throw new Error(`${name} failed: ${msg}\n  screenshot: ${file}`, { cause: e });
    }
  };
}

const xq = (s) => (s.includes('"') ? `concat("${s.replace(/"/g, '", \'"\', "')}")` : `"${s}"`);

/** A button by visible text or accessible name (aria-label / title). */
export async function button(browser, name, { timeout = 10_000, within = "" } = {}) {
  const sel =
    `${within}//button[not(@disabled)][normalize-space(.)=${xq(name)} or @aria-label=${xq(name)} or @title=${xq(name)}]` +
    ` | ${within}//*[@role="menuitem" or @role="tab" or @role="option"][normalize-space(.)=${xq(name)} or @aria-label=${xq(name)}]`;
  const el = await browser.$(sel);
  await el.waitForDisplayed({ timeout, timeoutMsg: `No visible button "${name}"` });
  return el;
}

export async function click(browser, name, opts) {
  const el = await button(browser, name, opts);
  await el.scrollIntoView({ block: "center" }).catch(() => undefined);
  await el.click();
}

/** A text field by its label, aria-label or placeholder. */
export async function field(browser, label, { timeout = 10_000 } = {}) {
  const l = xq(label);
  const sel =
    `//input[@aria-label=${l} or @placeholder=${l}] | //textarea[@aria-label=${l} or @placeholder=${l}]` +
    ` | //label[normalize-space(.)=${l} or normalize-space(text())=${l}]//input | //label[normalize-space(text())=${l}]//textarea` +
    ` | //*[@id=//label[normalize-space(.)=${l}]/@for]`;
  const el = await browser.$(sel);
  await el.waitForDisplayed({ timeout, timeoutMsg: `No visible field "${label}"` });
  return el;
}

export async function type(browser, label, text) {
  const el = await field(browser, label);
  await el.click();
  await el.clearValue().catch(() => undefined);
  await el.setValue(text);
}

/** Wait until the visible page text contains `text`. */
export async function waitForText(browser, text, timeout = 15_000) {
  await browser.waitUntil(async () => (await browser.execute(() => document.body.innerText)).includes(text), {
    timeout,
    timeoutMsg: `Text not shown: "${text}"`,
  });
}

export async function pageText(browser) {
  return browser.execute(() => document.body.innerText);
}

/** Press a shortcut, e.g. keys(browser, ["Control", "k"]). */
export async function keys(browser, combo) {
  await browser.keys(combo);
}

/** Go to a workspace through the left navigation. */
export async function openWorkspace(browser, label) {
  const el = await browser.$(`//nav[@aria-label="Workspaces"]//button[normalize-space(.)=${xq(label)}]`);
  await el.waitForClickable({ timeout: 10_000 });
  await el.click();
  await browser.$(`//main[@aria-label=${xq(label)}]`).waitForExist({ timeout: 10_000 });
}

export async function sleep(ms) {
  await new Promise((r) => setTimeout(r, ms));
}
