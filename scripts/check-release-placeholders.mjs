#!/usr/bin/env node
// Release guard: a store or release build fails while a REPLACE_WITH_ placeholder remains in the
// files below (package file 06). A debug, test or internal build never runs this script, so those
// builds ignore placeholders. Each placeholder in a file carries a comment saying who supplies it;
// the full table is docs/release/mac-store-placeholders.md.
//
//   node scripts/check-release-placeholders.mjs                  fail while one remains
//   node scripts/check-release-placeholders.mjs --report-only    print them, exit 0
//   node scripts/check-release-placeholders.mjs --build-number N fail unless N rises above the last
//                                                                upload in store-identity.json

import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
export const PLACEHOLDER = /REPLACE_WITH_[A-Z0-9_]+/g;

/** Text files of the store build. Directories are walked; `exts` limits what is read in them. */
export const SCANNED = [
  { dir: "apps/macos/packaging", exts: [".entitlements", ".plist", ".json", ".xcprivacy", ".storekit"] },
  { dir: "apps/macos/Sources/OpenFrameUI/Store", exts: [".swift"] },
  { file: ".github/workflows/release.yml" },
  { file: "packaging/msix/store.config.json" },
];

function walk(dir, exts) {
  const out = [];
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) out.push(...walk(p, exts));
    else if (exts.includes(path.extname(e.name))) out.push(p);
  }
  return out;
}

/** Comments that explain a placeholder do not count, or this guard could never go green. */
function codeOnly(file, text) {
  if (file.endsWith(".plist") || file.endsWith(".entitlements") || file.endsWith(".xcprivacy")) {
    return text.replace(/<!--[\s\S]*?-->/g, "");
  }
  if (file.endsWith(".swift")) return text.replace(/^\s*\/\/.*$/gm, "");
  if (file.endsWith(".yml")) return text.replace(/^\s*#.*$/gm, "");
  return text;
}

export function findPlaceholders(root = ROOT, targets = SCANNED) {
  const found = [];
  for (const t of targets) {
    const files = t.dir
      ? (fs.existsSync(path.join(root, t.dir)) ? walk(path.join(root, t.dir), t.exts) : [])
      : (fs.existsSync(path.join(root, t.file)) ? [path.join(root, t.file)] : []);
    for (const f of files) {
      const text = codeOnly(f, fs.readFileSync(f, "utf8"));
      for (const m of text.matchAll(PLACEHOLDER)) found.push({ file: path.relative(root, f), name: m[0] });
    }
  }
  return found;
}

export function buildNumberRises(next, identity) {
  const last = Number(identity.lastUploadedBuild ?? 0);
  return Number.isInteger(next) && next > last;
}

if (process.argv[1] === url.fileURLToPath(import.meta.url)) {
  const argv = process.argv.slice(2);
  const buildAt = argv.indexOf("--build-number");
  if (buildAt >= 0) {
    const identity = JSON.parse(fs.readFileSync(path.join(ROOT, "apps/macos/packaging/store-identity.json"), "utf8"));
    const next = Number(argv[buildAt + 1]);
    if (!buildNumberRises(next, identity)) {
      console.error(`The build number ${argv[buildAt + 1]} did not rise above the last upload, ${identity.lastUploadedBuild}.`);
      process.exit(1);
    }
    console.log(`Build number ${next} rises above ${identity.lastUploadedBuild}.`);
    process.exit(0);
  }
  const found = findPlaceholders();
  for (const { file, name } of found) console.log(`${file}: ${name}`);
  if (found.length && !argv.includes("--report-only")) {
    console.error(`${found.length} placeholder(s) remain. See docs/release/mac-store-placeholders.md.`);
    process.exit(1);
  }
  if (!found.length) console.log("No placeholders remain.");
}
