// Probe a captured screen: list its controls, or check one selector.
// node research/probe.mjs win 003-app-home-empty.html            -> list controls with suggested selectors
// node research/probe.mjs mac 011-new-project-sheet.html "[data-id=x]" -> count, rect (relative to the window), text
import { chromium } from 'playwright';
import { pathToFileURL } from 'node:url';
import path from 'node:path';

const [plat, file, sel] = process.argv.slice(2);
const dir = plat === 'mac' ? 'MacHTML-src' : 'WindowsHTML-src';
const url = pathToFileURL(path.resolve(import.meta.dirname, dir, 'light', file)).href;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: plat === 'mac' ? { width: 1400, height: 900 } : { width: 1600, height: 1000 } });
await page.goto(url);
// Check mode uses Playwright selectors (CSS, or role=button[name="Create Project"], or text=...).
if (sel) {
  const root = plat === 'mac' ? page.locator('.win') : page.locator('html');
  const base = await root.boundingBox();
  const loc = root.locator(sel), n = await loc.count(), items = [];
  for (let i = 0; i < Math.min(n, 5); i++) { const b = await loc.nth(i).boundingBox(); items.push({ rect: b && [b.x - base.x, b.y - base.y, b.width, b.height].map(Math.round), label: (await loc.nth(i).innerText().catch(() => '')).slice(0, 60) }); }
  console.log(JSON.stringify({ count: n, items }, null, 1)); await browser.close(); process.exit(0);
}
const out = await page.evaluate(({ plat, sel }) => {
  const win = plat === 'mac' ? document.querySelector('.win') : document.documentElement;
  const base = plat === 'mac' ? win.getBoundingClientRect() : { left: 0, top: 0 };
  const box = (el) => { const r = el.getBoundingClientRect(); return [Math.round(r.left - base.left), Math.round(r.top - base.top), Math.round(r.width), Math.round(r.height)]; };
  const label = (el) => (el.getAttribute('aria-label') || el.getAttribute('title') || el.value || el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 60);
  if (sel) {
    const all = win.querySelectorAll(sel);
    return { count: all.length, items: [...all].slice(0, 5).map((el) => ({ rect: box(el), tag: el.tagName.toLowerCase(), label: label(el) })) };
  }
  const q = plat === 'mac' ? '.el[data-id], .el[aria-label]' : 'button, a, input, textarea, select, [role=button], [role=tab], [role=menuitem], [role=option], [role=treeitem], [contenteditable=true]';
  return [...win.querySelectorAll(q)].filter((el) => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; }).slice(0, 250).map((el) => {
    const id = el.getAttribute('data-id'), al = el.getAttribute('aria-label'), tid = el.getAttribute('data-testid');
    const nm = label(el).replace(/"/g, '\\"'); const s = id ? `[data-id="${id}"]` : tid ? `[data-testid="${tid}"]` : el.id ? `#${CSS.escape(el.id)}` : al ? `${el.tagName.toLowerCase()}[aria-label="${al}"]` : nm ? `role=${el.getAttribute('role') || (el.tagName === 'A' ? 'link' : el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' ? 'textbox' : el.tagName.toLowerCase())}[name="${nm}"]` : null;
    return { sel: s, role: el.getAttribute('data-role') || el.getAttribute('role') || el.tagName.toLowerCase(), label: label(el), rect: box(el) };
  });
}, { plat, sel });
console.log(JSON.stringify(out, null, sel ? 2 : 0).replace(/\},\{/g, '},\n{'));
await browser.close();
