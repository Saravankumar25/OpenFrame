// One-off: draws src/static/og.png (1200x630) for link previews from the brand mark and a real app screen.
// Re-run when the brand line changes. Needs .cache/win-full from scripts/render-screens.mjs.
import sharp from 'sharp';
import path from 'node:path';
const ROOT = path.resolve(import.meta.dirname, '..');
const shot = path.join(ROOT, '.cache', 'win-full', '016-shell-project-home.png');
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1d1a2e"/><stop offset="1" stop-color="#0e0d15"/></linearGradient>
<linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ff5d6c"/><stop offset=".55" stop-color="#ff9a3d"/><stop offset="1" stop-color="#ffc23d"/></linearGradient></defs>
<rect width="1200" height="630" fill="url(#bg)"/>
<g transform="translate(72 66) scale(2.5)" fill="none" stroke="url(#g)" stroke-width="1.6" stroke-linecap="round"><rect x="2" y="2" width="20" height="20" rx="2.2"/><path d="M7 2v20M17 2v20M2 12h20M2 7h5M2 17h5M22 7h-5M22 17h-5"/></g>
<text x="148" y="113" font-family="Manrope, Segoe UI, Arial, sans-serif" font-size="44" fill="#fff" xml:space="preserve"><tspan font-weight="700">Straft</tspan><tspan font-weight="400" fill="#ff9a3d"> Studio</tspan></text>
<text x="72" y="232" font-family="Georgia, serif" font-size="58" fill="#fff">A film production software</text>
<text x="72" y="300" font-family="Georgia, serif" font-size="58" fill="#fff">for all your needs.</text>
<text x="72" y="366" font-family="Segoe UI, Arial, sans-serif" font-size="26" fill="#cfc9e2">Ideate, write, plan and shoot in one app.</text>
<text x="72" y="406" font-family="Segoe UI, Arial, sans-serif" font-size="26" fill="#cfc9e2">Windows 11 and Mac. 7-day free trial.</text>
</svg>`;
const bg = await sharp(Buffer.from(svg)).png().toBuffer();
const pic = await sharp(shot).resize(620).png().toBuffer();
await sharp(bg).composite([{ input: pic, left: 560, top: 330 }]).png({ compressionLevel: 9 }).toFile(path.join(ROOT, 'src', 'static', 'og.png'));
console.log('og.png written');
