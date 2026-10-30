// Renders every Windows screen page (research/WindowsHTML-src) in Chromium at its capture size, 1600x1000:
//   .cache/win-full/NNN-name.png                  light, full size (manual PDF pictures)
//   .cache/win-thumbs/<light|dark>/NNN-name.jpg   480 px wide (tutorial cards; the site swaps light and dark)
// Mac pages already ship their window pictures and thumbnails (research/MacHTML-src/<theme>/assets).
// Run: node scripts/render-screens.mjs   (skips files newer than their page; FORCE=1 redoes all)
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import sharp from 'sharp';

const ROOT = path.resolve(import.meta.dirname, '..');
const SRC = path.join(ROOT, 'research', 'WindowsHTML-src');
const OUT = path.join(ROOT, '.cache');
const force = !!process.env.FORCE;
const fresh = (out, src) => !force && fs.existsSync(out) && fs.statSync(out).mtimeMs >= fs.statSync(src).mtimeMs;

const files = fs.readdirSync(path.join(SRC, 'light')).filter((f) => f.endsWith('.html')).sort();
for (const d of ['win-full', 'win-thumbs/light', 'win-thumbs/dark']) fs.mkdirSync(path.join(OUT, d), { recursive: true });
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
let done = 0;
for (const theme of ['light', 'dark']) {
  for (const f of files) {
    const src = path.join(SRC, theme, f), base = f.replace('.html', '');
    const thumb = path.join(OUT, 'win-thumbs', theme, `${base}.jpg`), full = path.join(OUT, 'win-full', `${base}.png`);
    if (fresh(thumb, src) && (theme === 'dark' || fresh(full, src))) continue;
    await page.goto(pathToFileURL(src).href, { waitUntil: 'load' });
    const buf = await page.screenshot({ type: 'png' });
    await sharp(buf).resize(480).jpeg({ quality: 72, mozjpeg: true }).toFile(thumb);
    if (theme === 'light') fs.writeFileSync(full, buf);
    if (++done % 100 === 0) console.log(`${done} rendered`);
  }
}
await browser.close();
console.log(`rendered ${done} pages (${files.length} screens in light and dark)`);
