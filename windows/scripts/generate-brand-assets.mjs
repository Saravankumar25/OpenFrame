#!/usr/bin/env node
// Regenerates every Straft Studio icon file from one source drawing.
//
//   node scripts/generate-brand-assets.mjs          (from the repository root, Windows)
//
// Input:  apps/desktop/src-tauri/icons/source.svg (1024 square tile with the Straft mark).
// Output: all PNGs in apps/desktop/src-tauri/icons under their existing names and pixel sizes
//         (source.png is 1024), icon.ico (PNG entries 16 to 256), icon.icns (PNG entries 16 to 1024
//         with the @2x variants), then the MSIX visual assets in packaging/msix/Assets.
// Method: the installed Tauri CLI (`tauri icon --png`) rasterises the SVG, this script writes the
//         .ico and .icns itself, then reads both back and checks them. No new dependency, and no
//         installer or release build is started: the MSIX step only runs generate-assets.ps1 with the
//         size list exported by scripts/build-msix.mjs (the --generate-assets flag of that script
//         goes on to build a package, so it is not used here).
// The report and print logo is a separate, wide lockup and is not made here.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";
import { ASSETS_DIR, MSIX_DIR, ROOT, assetSpec, missingAssets } from "./build-msix.mjs";

const ICONS = path.join(ROOT, "apps", "desktop", "src-tauri", "icons");
const SVG = path.join(ICONS, "source.svg");
const TAURI_CLI = path.join(ROOT, "node_modules", "@tauri-apps", "cli", "tauri.js");

/** Existing file name in the icons folder to its pixel size. */
const PNG_FILES = {
  "32x32.png": 32,
  "64x64.png": 64,
  "128x128.png": 128,
  "128x128@2x.png": 256,
  "icon.png": 512,
  "source.png": 1024,
  "StoreLogo.png": 50,
  ...Object.fromEntries([30, 44, 71, 89, 107, 142, 150, 284, 310].map((n) => [`Square${n}x${n}Logo.png`, n])),
};
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];
/** ICNS type code to pixel size (the @2x types hold the doubled size of the point size). */
const ICNS_TYPES = [
  ["icp4", 16], ["icp5", 32], ["icp6", 64], ["ic07", 128], ["ic08", 256], ["ic09", 512], ["ic10", 1024],
  ["ic11", 32], ["ic12", 64], ["ic13", 256], ["ic14", 512],
];

export function buildIco(pngs) {
  const sizes = Object.keys(pngs).map(Number).sort((a, b) => a - b);
  const head = Buffer.alloc(6 + 16 * sizes.length);
  head.writeUInt16LE(1, 2);
  head.writeUInt16LE(sizes.length, 4);
  let offset = head.length;
  sizes.forEach((s, i) => {
    const e = 6 + 16 * i;
    head[e] = s === 256 ? 0 : s; // 0 means 256 in an ICO directory
    head[e + 1] = s === 256 ? 0 : s;
    head.writeUInt16LE(1, e + 4); // planes
    head.writeUInt16LE(32, e + 6); // bits per pixel
    head.writeUInt32LE(pngs[s].length, e + 8);
    head.writeUInt32LE(offset, e + 12);
    offset += pngs[s].length;
  });
  return Buffer.concat([head, ...sizes.map((s) => pngs[s])]);
}

export function buildIcns(pngs) {
  const parts = ICNS_TYPES.map(([type, size]) => {
    const h = Buffer.alloc(8);
    h.write(type, 0, "ascii");
    h.writeUInt32BE(8 + pngs[size].length, 4);
    return Buffer.concat([h, pngs[size]]);
  });
  const body = Buffer.concat(parts);
  const h = Buffer.alloc(8);
  h.write("icns", 0, "ascii");
  h.writeUInt32BE(8 + body.length, 4);
  return Buffer.concat([h, body]);
}

function pngSize(buf) {
  if (buf.readUInt32BE(0) !== 0x89504e47) throw new Error("not a PNG");
  return [buf.readUInt32BE(16), buf.readUInt32BE(20)];
}

function checkIco(file) {
  const b = fs.readFileSync(file);
  const count = b.readUInt16LE(4);
  if (b.readUInt16LE(0) !== 0 || b.readUInt16LE(2) !== 1 || count !== ICO_SIZES.length) throw new Error("icon.ico header is wrong");
  const got = [];
  for (let i = 0; i < count; i++) {
    const e = 6 + 16 * i;
    const size = b.readUInt32LE(e + 8);
    const off = b.readUInt32LE(e + 12);
    const [w, h] = pngSize(b.subarray(off, off + size));
    const dirW = b[e] || 256;
    if (w !== h || w !== dirW) throw new Error(`icon.ico entry ${i} says ${dirW} but holds ${w}x${h}`);
    got.push(w);
  }
  if (got.join() !== ICO_SIZES.join()) throw new Error(`icon.ico sizes are ${got.join()}`);
  return got;
}

function checkIcns(file) {
  const b = fs.readFileSync(file);
  if (b.toString("ascii", 0, 4) !== "icns" || b.readUInt32BE(4) !== b.length) throw new Error("icon.icns length does not match its header");
  const got = [];
  for (let off = 8; off < b.length; ) {
    const len = b.readUInt32BE(off + 4);
    const [w] = pngSize(b.subarray(off + 8, off + len));
    got.push(`${b.toString("ascii", off, off + 4)}=${w}`);
    off += len;
  }
  const want = ICNS_TYPES.map(([t, s]) => `${t}=${s}`);
  if (got.join() !== want.join()) throw new Error(`icon.icns entries are ${got.join()}`);
  return { bytes: b.length, entries: got };
}

function main() {
  const sizes = [...new Set([...Object.values(PNG_FILES), ...ICO_SIZES, ...ICNS_TYPES.map((t) => t[1])])].sort((a, b) => a - b);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "straft-icons-"));
  try {
    const r = spawnSync(process.execPath, [TAURI_CLI, "icon", SVG, "-o", tmp, "--png", sizes.join(",")], { cwd: ROOT, encoding: "utf8" });
    if (r.status !== 0) throw new Error(`tauri icon failed:\n${r.stdout}${r.stderr}`);
    const raster = {};
    for (const s of sizes) {
      raster[s] = fs.readFileSync(path.join(tmp, `${s}x${s}.png`));
      const [w, h] = pngSize(raster[s]);
      if (w !== s || h !== s) throw new Error(`${s}x${s}.png came out ${w}x${h}`);
    }
    for (const [name, size] of Object.entries(PNG_FILES)) fs.writeFileSync(path.join(ICONS, name), raster[size]);
    fs.writeFileSync(path.join(ICONS, "icon.ico"), buildIco(Object.fromEntries(ICO_SIZES.map((s) => [s, raster[s]]))));
    fs.writeFileSync(path.join(ICONS, "icon.icns"), buildIcns(raster));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
  // The CLI can also emit android/ and ios/ folders; with --png it does not, but never keep them.
  for (const d of ["android", "ios"]) fs.rmSync(path.join(ICONS, d), { recursive: true, force: true });

  console.log(`icon.ico  entries ${checkIco(path.join(ICONS, "icon.ico")).join(", ")}`);
  const icns = checkIcns(path.join(ICONS, "icon.icns"));
  console.log(`icon.icns ${icns.bytes} bytes, entries ${icns.entries.join(" ")}`);

  const specPath = path.join(os.tmpdir(), `straft-msix-asset-spec-${process.pid}.json`);
  fs.writeFileSync(specPath, JSON.stringify(assetSpec(), null, 2));
  try {
    fs.rmSync(ASSETS_DIR, { recursive: true, force: true });
    const ps = spawnSync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", path.join(MSIX_DIR, "generate-assets.ps1"),
        "-Source", path.join(ICONS, "source.png"), "-SpecPath", specPath, "-OutDir", ASSETS_DIR],
      { encoding: "utf8" },
    );
    if (ps.status !== 0) throw new Error(`generate-assets.ps1 failed:\n${ps.stdout}${ps.stderr}`);
    console.log(ps.stdout.trim());
  } finally {
    fs.rmSync(specPath, { force: true });
  }
  const missing = missingAssets();
  if (missing.length) throw new Error(`missing MSIX assets: ${missing.join(", ")}`);
  console.log("Brand assets regenerated.");
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (e) {
    console.error(`generate-brand-assets failed: ${e.message}`);
    process.exit(1);
  }
}
