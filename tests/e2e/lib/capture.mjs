// Screen capture for WindowsHTML (tests/e2e/specs/zz-capture.test.mjs and
// scripts/windows-html/build-windows-html.mjs).
//
// `capture(name)` records the current state of the real app twice, in Light and in Dark mode of the
// default theme: a full-window screenshot and the page's real DOM (form values written into
// attributes, scroll offsets and the style sheets the page uses). The raw captures go to
// .build-cache/windows-html-capture/ (reproducible, not committed); the generator turns them into
// WindowsHTML/. Nothing here changes how the app behaves: the mode is set through the same operation
// Settings > Appearance uses, and the only thing added to the page is a capture-time style that stops
// the caret and transitions, removed again from the serialized copy.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { sleep } from "./app.mjs";

export const REPO = resolve(import.meta.dirname, "..", "..", "..");
// E2E_CAPTURE_OUT moves the raw captures (a section run writes its own folder).
export const RAW = process.env.E2E_CAPTURE_OUT ? resolve(REPO, process.env.E2E_CAPTURE_OUT) : join(REPO, ".build-cache", "windows-html-capture");
export const MODES = ["light", "dark"];
export const WIDTH = 1600;
export const HEIGHT = 1000;

/** Keep the window at WIDTHxHEIGHT CSS pixels (Windows or the app can resize it, for example when a project opens). */
export async function ensureSize(browser) {
  for (let i = 0; i < 5; i++) {
    const [w, h] = await browser.execute(() => [window.innerWidth, window.innerHeight]);
    if (w === WIDTH && h === HEIGHT) return;
    const outer = await browser.getWindowSize();
    await browser.setWindowSize(outer.width + WIDTH - w, outer.height + HEIGHT - h);
    await sleep(300);
  }
  const [w, h] = await browser.execute(() => [window.innerWidth, window.innerHeight]);
  throw new Error(`the window stays at ${w}x${h}, not ${WIDTH}x${HEIGHT}`);
}

/** Set Light or Dark mode of the default theme through the app's own operations. */
export async function setMode(browser, mode) {
  const r = await browser.execute(
    (m) => {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      return invoke("of_invoke", { op: "appearance.set_mode", args: { mode: m } })
        .then(() => invoke("of_invoke", { op: "appearance.set_theme", args: { themeId: "openframe-default" } }))
        .then(() => ({ ok: true }), (e) => ({ err: JSON.stringify(e) }));
    },
    mode,
  );
  if (r.err) throw new Error(`appearance: ${r.err}`);
  if (mode === "system") return;
  await browser.waitUntil(async () => (await browser.execute(() => document.documentElement.dataset.mode)) === mode, {
    timeout: 10_000,
    timeoutMsg: `the window did not switch to ${mode} mode`,
  });
}

/** Runs in the page: the DOM as static markup plus what a static page needs to look the same. */
function serialize() {
  const freeze = document.getElementById("cap-freeze");
  const all = [...document.documentElement.querySelectorAll("*")];
  const copy = document.documentElement.cloneNode(true);
  const twins = [...copy.querySelectorAll("*")];
  const scroll = [];
  const raster = [];
  const rectOf = (el) => {
    const b = el.getBoundingClientRect();
    return { x: b.left, y: b.top, w: b.width, h: b.height };
  };
  all.forEach((el, i) => {
    const c = twins[i];
    if (el instanceof HTMLInputElement) {
      if (el.type === "checkbox" || el.type === "radio") el.checked ? c.setAttribute("checked", "") : c.removeAttribute("checked");
      else if (el.type !== "file") c.setAttribute("value", el.value);
    } else if (el instanceof HTMLTextAreaElement) {
      c.textContent = el.value;
    } else if (el instanceof HTMLSelectElement) {
      [...el.options].forEach((o, j) => (o.selected ? c.options[j].setAttribute("selected", "") : c.options[j].removeAttribute("selected")));
    }
    // A static page keeps a scroll offset without script through `scroll-initial-target`: an
    // invisible 1 px marker sits where the view was scrolled to (start aligned, so it never widens
    // the scroll area), and the browser opens the page scrolled to it.
    if (el.scrollTop || el.scrollLeft) {
      const positioned = getComputedStyle(el).position !== "static";
      if (!positioned) c.style.position = "relative";
      c.style.scrollPadding = "0";
      const m = document.createElement("span");
      m.setAttribute("data-cap-mark", "");
      m.setAttribute("aria-hidden", "true");
      m.style.cssText = `position:absolute;top:${el.scrollTop}px;left:${el.scrollLeft}px;width:1px;height:1px;margin:0;scroll-margin:0;scroll-snap-align:start;scroll-initial-target:nearest;pointer-events:none;visibility:hidden`;
      c.appendChild(m);
      scroll.push({ top: el.scrollTop, left: el.scrollLeft, positioned });
    }
    // Pixels a static page cannot rebuild from markup: canvas, video, and images from the app's own
    // asset protocol or blob URLs. They become an <img> the generator fills with the same pixels cut
    // from the screenshot.
    const src = el instanceof HTMLImageElement ? el.currentSrc || el.src : "";
    if (el instanceof HTMLCanvasElement || el instanceof HTMLVideoElement || (src && !src.startsWith("data:"))) {
      let img = c;
      if (!(el instanceof HTMLImageElement)) {
        img = document.createElement("img");
        for (const a of c.attributes) img.setAttribute(a.name, a.value);
        img.style.width = `${el.getBoundingClientRect().width}px`;
        img.style.height = `${el.getBoundingClientRect().height}px`;
        c.replaceWith(img);
      }
      img.removeAttribute("src");
      img.removeAttribute("srcset");
      img.setAttribute("data-cap-raster", String(raster.length));
      raster.push({ tag: el.tagName.toLowerCase(), src, ...rectOf(el) });
    }
  });
  copy.querySelectorAll("script, noscript, link, style, template").forEach((n) => n.remove());
  copy.querySelectorAll("*").forEach((n) => {
    for (const a of [...n.attributes]) if (/^on/i.test(a.name)) n.removeAttribute(a.name);
  });
  const sheets = [];
  const take = (s) => {
    if (s.ownerNode === freeze) return;
    // The source text, not cssRules: Chromium serializes a rule that mixes a var() shorthand with
    // a longhand (the app's kbd rule) as empty declarations. A linked sheet is read from the built
    // app by the generator; a <style> keeps its own text, and only a sheet built in script falls
    // back to cssRules.
    let text = s.href ? "" : s.ownerNode?.textContent ?? "";
    if (!s.href && !text.trim()) {
      try {
        text = [...s.cssRules].map((r) => r.cssText).join("\n");
      } catch {
        text = "";
      }
    }
    sheets.push({ href: s.href, media: s.media?.mediaText ?? "", text });
  };
  [...document.styleSheets].forEach(take);
  (document.adoptedStyleSheets ?? []).forEach(take);
  const nav = [...document.querySelectorAll('nav[aria-label="Workspaces"] button')].map((b) => ({
    label: (b.getAttribute("aria-label") || b.textContent || "").trim(),
    ...rectOf(b),
  }));
  return {
    html: copy.outerHTML,
    sheets,
    scroll,
    raster,
    nav,
    title: document.title,
    width: window.innerWidth,
    height: window.innerHeight,
    dpr: window.devicePixelRatio || 1,
    mode: document.documentElement.dataset.mode,
  };
}

/**
 * A capture session. `capture(name, opts)` records one state in both modes; `noteClick(el)` before
 * a click records which control leads to the next captured state, so the generated pages can link
 * screens the way the app does (only when exactly one noted click separates two captures).
 */
export function captureSession(getBrowser) {
  rmSync(RAW, { recursive: true, force: true });
  for (const m of MODES) mkdirSync(join(RAW, m), { recursive: true });
  const manifest = { screens: [], links: [], unreachable: [], failed: [] };
  let section = "";
  const names = new Set();
  let current = "light";
  let clicks = [];

  async function freeze(browser, on) {
    await browser.execute((enable) => {
      let s = document.getElementById("cap-freeze");
      if (!enable) return s?.remove();
      if (s) return;
      s = document.createElement("style");
      s.id = "cap-freeze";
      s.textContent = "*,*::before,*::after{caret-color:transparent!important;transition:none!important}";
      document.head.appendChild(s);
    }, on);
  }

  async function settle(browser) {
    await browser
      .waitUntil(async () => !(await browser.execute(() => document.body.innerText.includes("Saving…"))), { timeout: 15_000 })
      .catch(() => undefined);
  }

  return {
    manifest,
    async noteClick(el) {
      const browser = getBrowser();
      const r = await browser.execute((e) => {
        const b = e.getBoundingClientRect();
        return { x: b.left, y: b.top, w: b.width, h: b.height };
      }, el);
      clicks.push(r);
    },
    unreachable(name, why) {
      manifest.unreachable.push({ name, why, section });
    },
    /** The section now running (recorded with each screen). */
    setSection(name) {
      section = name;
    },
    /** A state the tour tried and could not capture: the error and a screenshot of the window. */
    async fail(name, error) {
      const msg = error instanceof Error ? error.message : String(error);
      const file = join(RAW, "failed", `${name}.png`);
      mkdirSync(join(RAW, "failed"), { recursive: true });
      await getBrowser().saveScreenshot(file).catch(() => undefined);
      const line = msg.split("\n")[0].slice(0, 300);
      manifest.failed.push({ name, section, error: line, screenshot: file });
      clicks = [];
      console.log(`  capture FAILED ${name}: ${line}`);
      writeFileSync(join(RAW, "manifest.json"), JSON.stringify(manifest, null, 1));
    },
    /**
     * Record the current state. Options: `blur` (default true) moves focus off fields so no caret
     * or focus ring is captured; `keepToasts` keeps toasts that are the subject of the screen;
     * `wait` (ms) lets lazy content settle; `home` marks the screen the workspace button leads to.
     */
    async capture(name, { blur = true, keepToasts = false, wait = 600, home } = {}) {
      const browser = getBrowser();
      if (names.has(name)) throw new Error(`capture name used twice: ${name}`);
      names.add(name);
      const id = `${String(manifest.screens.length + 1).padStart(3, "0")}-${name}`;
      if (blur) {
        await browser.execute(() => {
          if (document.activeElement instanceof HTMLElement && !document.activeElement.closest('[role="menu"],[role="listbox"]')) document.activeElement.blur();
        });
      }
      if (!keepToasts) {
        await browser.execute(() => document.querySelectorAll('.toast button[aria-label="Dismiss"]').forEach((b) => b.click()));
      }
      await ensureSize(browser);
      await freeze(browser, true);
      await sleep(wait);
      await settle(browser);
      const order = current === "light" ? ["light", "dark"] : ["dark", "light"];
      const sizes = {};
      for (const mode of order) {
        if (mode !== current) {
          await setMode(browser, mode);
          current = mode;
          await sleep(350);
        }
        let png;
        let dom;
        // The window can still be resized between the size check and the screenshot: take it again.
        for (let i = 0; i < 3; i++) {
          png = Buffer.from(await browser.takeScreenshot(), "base64");
          dom = await browser.execute(serialize);
          if (dom.width === WIDTH && dom.height === HEIGHT) break;
          await ensureSize(browser);
          await sleep(400);
        }
        if (dom.width !== WIDTH || dom.height !== HEIGHT) throw new Error(`captured at ${dom.width}x${dom.height}, not ${WIDTH}x${HEIGHT}`);
        writeFileSync(join(RAW, mode, `${id}.png`), png);
        writeFileSync(join(RAW, mode, `${id}.json`), JSON.stringify(dom));
        sizes[mode] = `${dom.width}x${dom.height}`;
      }
      await freeze(browser, false);
      if (clicks.length === 1 && manifest.screens.length) manifest.links.push({ from: manifest.screens.at(-1).id, to: id, ...clicks[0] });
      clicks = [];
      manifest.screens.push({ id, name, section, home: home ?? null, size: sizes.light });
      console.log(`  captured ${id}`);
      writeFileSync(join(RAW, "manifest.json"), JSON.stringify(manifest, null, 1));
      return id;
    },
    async finish() {
      const browser = getBrowser();
      if (browser) await setMode(browser, "system").catch(() => undefined);
      writeFileSync(join(RAW, "manifest.json"), JSON.stringify(manifest, null, 1));
    },
  };
}
