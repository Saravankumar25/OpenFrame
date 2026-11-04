// One-off: draws src/static/og.png (1200x630) for link previews from the brand mark and a real app screen.
// Re-run when the brand line changes. Needs .cache/win-full from scripts/render-screens.mjs.
import sharp from 'sharp';
import path from 'node:path';
import { MARK_D, STRAFT_D, STUDIO_D, LOGO_VIEWBOX } from '../src/lib/icons.mjs';
const ROOT = path.resolve(import.meta.dirname, '..');
const shot = path.join(ROOT, '.cache', 'win-full', '016-shell-project-home.png');
const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630">
<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1d1a2e"/><stop offset="1" stop-color="#0e0d15"/></linearGradient>
<linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FF5E6C"/><stop offset="0.5" stop-color="#FF9A45"/><stop offset="1" stop-color="#FFC13D"/></linearGradient></defs>
<rect width="1200" height="630" fill="url(#bg)"/>
<svg x="72" y="62" width="360" height="80" viewBox="${LOGO_VIEWBOX}"><g transform="translate(70 26.4) scale(0.3)"><path fill-rule="evenodd" fill="url(#g)" d="${MARK_D}"/></g><path fill="#fff" d="${STRAFT_D}"/><path fill="#FF9A45" d="${STUDIO_D}"/></svg>
<text x="72" y="232" font-family="Georgia, serif" font-size="58" fill="#fff">A film production software</text>
<text x="72" y="300" font-family="Georgia, serif" font-size="58" fill="#fff">for all your needs.</text>
<text x="72" y="366" font-family="Segoe UI, Arial, sans-serif" font-size="26" fill="#cfc9e2">Ideate, write, plan and shoot in one app.</text>
<text x="72" y="406" font-family="Segoe UI, Arial, sans-serif" font-size="26" fill="#cfc9e2">Windows 11 and Mac. 7-day free trial.</text>
</svg>`;
const bg = await sharp(Buffer.from(svg)).png().toBuffer();
const pic = await sharp(shot).resize(620).png().toBuffer();
await sharp(bg).composite([{ input: pic, left: 560, top: 330 }]).png({ compressionLevel: 9 }).toFile(path.join(ROOT, 'src', 'static', 'og.png'));
console.log('og.png written');
