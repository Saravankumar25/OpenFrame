// One-off: draws src/static/og.png (1200x630) for link previews. Re-run when the brand line changes.
import sharp from 'sharp';
import path from 'node:path';
const ROOT = path.resolve(import.meta.dirname, '..');
const shot = path.join(ROOT, 'screens', 'win', '003-app-home-empty-1600.webp');
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
<rect width="1200" height="630" fill="#1d1a2e"/>
<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff5d6c"/><stop offset=".55" stop-color="#ff9a3d"/><stop offset="1" stop-color="#ffc23d"/></linearGradient></defs>
<g transform="translate(72 74) scale(2.6)" fill="none" stroke="url(#g)" stroke-width="1.6" stroke-linecap="round"><rect x="2" y="2" width="20" height="20" rx="2.2"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></g>
<text x="150" y="122" font-family="Segoe UI, Arial, sans-serif" font-size="46" fill="#fff"><tspan font-weight="300">Open</tspan><tspan font-weight="700">Frame</tspan><tspan font-weight="700" fill="#ff8a5c" font-size="22" dx="14">STUDIO</tspan></text>
<text x="72" y="250" font-family="Segoe UI, Arial, sans-serif" font-size="54" font-weight="700" fill="#fff">Ideate. Write. Plan. Shoot.</text>
<text x="72" y="310" font-family="Segoe UI, Arial, sans-serif" font-size="27" fill="#cfc9e2">One local-first app for writers and film production teams.</text>
<text x="72" y="352" font-family="Segoe UI, Arial, sans-serif" font-size="27" fill="#cfc9e2">Windows 11 and Apple silicon Macs.</text>
</svg>`;
const bg = await sharp(Buffer.from(svg)).png().toBuffer();
const pic = await sharp(shot).resize(560).toBuffer();
await sharp(bg).composite([{ input: pic, left: 600, top: 300 }]).png().toFile(path.join(ROOT, 'src', 'static', 'og.png'));
console.log('og.png written');
