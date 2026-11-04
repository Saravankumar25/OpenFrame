// Downloads the home hero photographs once and writes the small square WebP files the site serves itself (the CSP allows
// images from this site only). Run by hand when src/content/hero-photos.mjs changes: node scripts/make-hero-photos.mjs
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';
import { HERO_PHOTOS, FEATURED, DOUBLE } from '../src/content/hero-photos.mjs';

const OUT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'src', 'static', 'hero');
fs.mkdirSync(OUT, { recursive: true });

async function download(url) {
  for (let tries = 1; ; tries++) {
    try {
      const r = await fetch(url);
      if (!r.ok) throw new Error(`status ${r.status}`);
      return Buffer.from(await r.arrayBuffer());
    } catch (e) {
      if (tries === 4) throw new Error(`${url}: ${e.message}`);
      await new Promise((ok) => setTimeout(ok, tries * 2000));
    }
  }
}

const keep = new Set();
for (const [i, p] of HERO_PHOTOS.entries()) {
  const widths = i < FEATURED ? [320, 640, 1000] : i < FEATURED + DOUBLE ? [320, 640] : [320, 480];
  const names = widths.map((w) => `${p.short}-${w}.webp`);
  names.forEach((n) => keep.add(n));
  if (names.every((n) => fs.existsSync(path.join(OUT, n)))) continue;
  const src = await download(`https://images.unsplash.com/photo-${p.id}?fit=crop&w=1000&h=1000&q=85&fm=jpg`);
  for (const [n, w] of widths.entries()) {
    await sharp(src).resize(w, w).webp({ quality: w > 640 ? 68 : 60 }).toFile(path.join(OUT, names[n]));
  }
}
// only after every photograph is in place: files of photographs dropped from the list go
for (const f of fs.readdirSync(OUT)) if (!keep.has(f)) fs.rmSync(path.join(OUT, f));
const kb = fs.readdirSync(OUT).reduce((a, f) => a + fs.statSync(path.join(OUT, f)).size, 0) / 1024;
console.log(`${HERO_PHOTOS.length} photographs, ${keep.size} files, ${Math.round(kb)} KB in ${OUT}`);
