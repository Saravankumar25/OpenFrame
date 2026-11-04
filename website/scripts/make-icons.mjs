// Draws the site icon set into src/static from the Straft frame mark: favicon.ico (16, 32, 48), PNG favicons,
// the Apple touch icon, the web app icons (192, 512) and a maskable icon. Re-run when the mark changes.
// Run: node scripts/make-icons.mjs
import fs from 'node:fs';
import path from 'node:path';
import sharp from 'sharp';
import { MARK_D } from '../src/lib/icons.mjs';

const OUT = path.resolve(import.meta.dirname, '..', 'src', 'static');
const TILE = '#1a1d26';
// The mark on the app icon tile. radius: corner of the tile; inset: space around the mark (fraction of the tile).
const tile = (size, { radius = 0.22, inset = 0.14 } = {}) => {
  const s = size, m = s * inset, k = (s - 2 * m) / 1024;
  return Buffer.from(`<svg xmlns="http://www.w3.org/2000/svg" width="${s}" height="${s}" viewBox="0 0 ${s} ${s}">
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FF5E6C"/><stop offset="0.5" stop-color="#FF9A45"/><stop offset="1" stop-color="#FFC13D"/></linearGradient></defs>
<rect width="${s}" height="${s}" rx="${s * radius}" fill="${TILE}"/>
<g transform="translate(${m} ${m}) scale(${k})"><path fill-rule="evenodd" fill="url(#g)" d="${MARK_D}"/></g></svg>`);
};
const png = (size, opts) => sharp(tile(size * 4, opts)).resize(size, size).png({ compressionLevel: 9 }).toBuffer();
const write = (name, buf) => { fs.writeFileSync(path.join(OUT, name), buf); console.log(name, buf.length + ' bytes'); };

write('favicon-16x16.png', await png(16, { inset: 0.1 }));
write('favicon-32x32.png', await png(32, { inset: 0.1 }));
write('apple-touch-icon.png', await png(180, { radius: 0, inset: 0.17 })); // iOS rounds the corners itself
write('icon-192.png', await png(192));
write('icon-512.png', await png(512));
write('icon-maskable-512.png', await png(512, { radius: 0, inset: 0.26 })); // mark inside the 80 percent safe zone

// favicon.ico: PNG images inside the ICO container.
const sizes = [16, 32, 48];
const imgs = await Promise.all(sizes.map((n) => png(n, { inset: 0.1 })));
const head = Buffer.alloc(6); head.writeUInt16LE(0, 0); head.writeUInt16LE(1, 2); head.writeUInt16LE(sizes.length, 4);
let offset = 6 + 16 * sizes.length;
const dir = Buffer.concat(sizes.map((n, i) => {
  const e = Buffer.alloc(16);
  e.writeUInt8(n, 0); e.writeUInt8(n, 1); e.writeUInt16LE(1, 4); e.writeUInt16LE(32, 6);
  e.writeUInt32LE(imgs[i].length, 8); e.writeUInt32LE(offset, 12); offset += imgs[i].length;
  return e;
}));
write('favicon.ico', Buffer.concat([head, dir, ...imgs]));
