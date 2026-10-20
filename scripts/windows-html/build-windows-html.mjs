// Builds WindowsHTML/ from the raw captures of tests/e2e/specs/zz-capture.test.mjs
// (.build-cache/windows-html-capture/, see tests/e2e/lib/capture.mjs).
//
//   node scripts/windows-html/build-windows-html.mjs
//
// Every captured state becomes WindowsHTML/light/NNN-name.html and WindowsHTML/dark/NNN-name.html:
// the app's real markup with form values and scroll offsets kept, the app's own style sheets stored
// once in WindowsHTML/assets/ (named by content hash), canvas and app-asset images replaced by the
// same pixels cut from the screenshot, no scripts and nothing loaded from the network. Controls that
// lead to another captured screen get an invisible link over them. WindowsHTML/index.html lists the
// screens with a Light and Dark switch made in CSS only.

import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, extname, join, resolve } from "node:path";
import { decodePng, encodePng } from "../../tests/e2e/lib/png.mjs";

const REPO = resolve(import.meta.dirname, "..", "..");
export const RAW = join(REPO, ".build-cache", "windows-html-capture");
export const OUT = join(REPO, "WindowsHTML");
let raw = RAW;
let out = OUT;
const DIST = join(REPO, "apps", "desktop", "dist");
const MODES = ["light", "dark"];

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const hash = (s) => createHash("sha256").update(s).digest("hex").slice(0, 12);
const title = (name) => name.replace(/-/g, " ").replace(/^./, (c) => c.toUpperCase());

/** Rewrite url(...) in a style sheet: files of the built app are copied to assets/files/. */
function rewriteUrls(text, sheetHref) {
  return text.replace(/url\((["']?)([^"')]+)\1\)/g, (all, _q, url) => {
    if (url.startsWith("data:") || url.startsWith("#")) return all;
    let path;
    try {
      path = new URL(url, sheetHref ?? "http://app.local/").pathname;
    } catch {
      return all;
    }
    const file = join(DIST, decodeURIComponent(path));
    if (!existsSync(file)) throw new Error(`style sheet refers to ${url}, not found at ${file}`);
    mkdirSync(join(out, "assets", "files"), { recursive: true });
    copyFileSync(file, join(out, "assets", "files", basename(file)));
    return `url("files/${basename(file)}")`;
  });
}

const MIME = { ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg", ".gif": "image/gif", ".webp": "image/webp", ".svg": "image/svg+xml", ".bmp": "image/bmp" };

/**
 * The original file behind an app image (the ofasset protocol URL carries the file path), as a data
 * URI, so a picture under a dialog is not dimmed twice. Null when it is not a readable image file.
 */
function assetFile(src) {
  const m = /^(?:http:\/\/ofasset\.localhost|ofasset:\/\/localhost)\/(.+)$/.exec(src ?? "");
  if (!m) return null;
  const file = decodeURIComponent(m[1].split("?")[0]);
  const mime = MIME[extname(file).toLowerCase()];
  if (!mime || !existsSync(file)) return null;
  return `data:${mime};base64,${readFileSync(file).toString("base64")}`;
}

/** A linked style sheet as the built app ships it (apps/desktop/dist, the build the tour ran). */
function builtSheet(href) {
  const file = join(DIST, decodeURIComponent(new URL(href).pathname));
  if (!existsSync(file)) throw new Error(`style sheet ${href} is not in the built app (${file}); rebuild before generating`);
  return readFileSync(file, "utf8");
}

/** Cut a CSS-pixel rectangle out of a decoded screenshot, as a PNG data URI. */
function crop(png, r, dpr) {
  const x0 = Math.max(0, Math.round(r.x * dpr));
  const y0 = Math.max(0, Math.round(r.y * dpr));
  const x1 = Math.min(png.width, Math.round((r.x + r.w) * dpr));
  const y1 = Math.min(png.height, Math.round((r.y + r.h) * dpr));
  const width = Math.max(1, x1 - x0);
  const height = Math.max(1, y1 - y0);
  const data = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    const from = ((y0 + y) * png.width + x0) * 4;
    png.data.copy(data, y * width * 4, from, from + Math.min(width, png.width - x0) * 4);
  }
  return `data:image/png;base64,${encodePng({ width, height, data }).toString("base64")}`;
}

function overlay(href, r, label) {
  const style = `position:fixed;left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px;z-index:2147483647`;
  return `<a data-cap-link href="${esc(href)}" style="${style}" aria-label="${esc(label)}"></a>`;
}

/** Build every page. Returns the manifest of the screens written. */
export function build({ commit = "unknown", from = RAW, to = OUT } = {}) {
  raw = from;
  out = to;
  const manifest = JSON.parse(readFileSync(join(raw, "manifest.json"), "utf8"));
  for (const d of [...MODES, "assets"]) rmSync(join(out, d), { recursive: true, force: true });
  for (const d of [...MODES, "assets"]) mkdirSync(join(out, d), { recursive: true });
  const homes = new Map();
  for (const s of manifest.screens) if (s.home && !homes.has(s.home)) homes.set(s.home, s.id);
  const byId = new Map(manifest.screens.map((s) => [s.id, s]));
  const sheetFiles = new Set();

  for (const s of manifest.screens) {
    for (const mode of MODES) {
      const dom = JSON.parse(readFileSync(join(raw, mode, `${s.id}.json`), "utf8"));
      const links = [];
      for (const sheet of dom.sheets) {
        const text = rewriteUrls(sheet.href ? builtSheet(sheet.href) : sheet.text, sheet.href);
        const name = `${hash(text)}.css`;
        if (!sheetFiles.has(name)) {
          writeFileSync(join(out, "assets", name), text);
          sheetFiles.add(name);
        }
        links.push(`<link rel="stylesheet" href="../assets/${name}"${sheet.media ? ` media="${esc(sheet.media)}"` : ""}>`);
      }
      let html = dom.html.replace(/<meta http-equiv[^>]*>/gi, "");
      if (dom.raster.length) {
        let png;
        dom.raster.forEach((r, i) => {
          let src = assetFile(r.src);
          if (!src) src = crop((png ??= decodePng(readFileSync(join(raw, mode, `${s.id}.png`)))), r, dom.dpr);
          html = html.replace(`data-cap-raster="${i}"`, `data-cap-raster="${i}" src="${src}"`);
        });
      }
      const overlays = [];
      for (const l of manifest.links.filter((l) => l.from === s.id)) overlays.push(overlay(`${l.to}.html`, l, `Open ${title(byId.get(l.to).name)}`));
      for (const n of dom.nav) {
        const target = homes.get(n.label);
        if (target && target !== s.id) overlays.push(overlay(`${target}.html`, n, `Open ${n.label}`));
      }
      const note = `<!-- OpenFrame Studio for Windows, screen ${s.id}, ${mode === "light" ? "Light" : "Dark"} mode, captured from commit ${commit} at ${dom.width}x${dom.height}. Generated by scripts/windows-html/build-windows-html.mjs; do not edit by hand. -->`;
      html = html.replace(/<head>/i, `<head>\n${links.join("\n")}\n`).replace(/<\/body>/i, `${overlays.join("")}</body>`);
      writeFileSync(join(out, mode, `${s.id}.html`), `<!doctype html>\n${note}\n${html}\n`);
    }
  }
  writeFileSync(join(out, "index.html"), indexPage(manifest, commit));
  return manifest;
}

function indexPage(manifest, commit) {
  const rows = manifest.screens
    .map(
      (s) =>
        `<li><span class="n">${s.id.slice(0, 3)}</span><a class="l" href="light/${s.id}.html">${esc(title(s.name))}</a><a class="d" href="dark/${s.id}.html">${esc(title(s.name))}</a></li>`,
    )
    .join("\n");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>OpenFrame Studio for Windows, every screen</title>
<style>
:root{color-scheme:light dark;--bg:#f6f6f4;--ink:#1d1d1b;--muted:#6b6b66;--line:#deddd8;--accent:#3b5bdb}
@media (prefers-color-scheme:dark){:root{--bg:#18181a;--ink:#ececea;--muted:#9a9a95;--line:#2e2e31;--accent:#8ea2ff}}
body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.45 "Segoe UI",system-ui,sans-serif}
main{max-width:1100px;margin:0 auto;padding:24px 16px}
h1{font-size:20px;margin:0 0 4px}
p{color:var(--muted);margin:0 0 16px}
.switch{display:inline-flex;border:1px solid var(--line);border-radius:8px;overflow:hidden;margin-bottom:16px}
.switch label{padding:6px 14px;cursor:pointer}
#m-light:checked~.switch label[for=m-light],#m-dark:checked~.switch label[for=m-dark]{background:var(--accent);color:#fff}
input[name=mode]{position:absolute;opacity:0}
ol{list-style:none;padding:0;margin:0;columns:3 280px;column-gap:24px}
li{break-inside:avoid;padding:3px 0;border-bottom:1px solid var(--line);display:flex;gap:8px}
.n{color:var(--muted);font-variant-numeric:tabular-nums;width:2.6em}
a{color:var(--ink);text-decoration:none}a:hover{color:var(--accent);text-decoration:underline}
#m-light:checked~ol .d,#m-dark:checked~ol .l{display:none}
</style>
</head>
<body>
<main>
<h1>OpenFrame Studio for Windows, every screen</h1>
<p>${manifest.screens.length} screens and states, each in Light and Dark mode, captured from the real app at commit ${esc(commit)}. See README.md for how they were made.</p>
<input type="radio" name="mode" id="m-light" checked><input type="radio" name="mode" id="m-dark">
<div class="switch" role="radiogroup" aria-label="Mode"><label for="m-light">Light</label><label for="m-dark">Dark</label></div>
<ol>
${rows}
</ol>
</main>
</body>
</html>
`;
}

if (process.argv[1] && resolve(process.argv[1]) === resolve(import.meta.filename)) {
  const m = build({ commit: process.argv[2] ?? "unknown" });
  console.log(`WindowsHTML: ${m.screens.length} screens in ${MODES.length} modes, ${m.links.length} links`);
}
