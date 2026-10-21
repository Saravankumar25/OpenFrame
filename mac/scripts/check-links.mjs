#!/usr/bin/env node
// Markdown link check.
//
//   node scripts/check-links.mjs     exit 1 when a relative link in a living document points at a missing file
//
// Scans every .md file except records: docs/audits, docs/archive, docs/release/evidence and the earlier-entries part of the
// logs are history and may point at files that were renamed since. Web links and anchors are not checked.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
// OpenFrameMac and changes are local, uncommitted reference folders the owner supplies (listed in .git/info/exclude).
const SKIP_DIRS = new Set(["node_modules", "target", ".git", "graphify-out", ".claude", ".build-cache", "dist", "gen", "OpenFrameMac", "changes"]);
const RECORD_PREFIXES = ["docs/audits/", "docs/archive/", "docs/release/evidence/"];
// Documents whose older sections are kept as written; links after the marker line are not checked.
const HISTORY_MARKERS = {
  "docs/implementation/progress-log.md": "## Earlier entries",
  "docs/implementation/reconciliation-log.md": "## Earlier records",
};

function* walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIRS.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* walk(full);
    else if (entry.name.endsWith(".md")) yield full;
  }
}

let checked = 0;
const errors = [];
for (const file of walk(ROOT)) {
  const rel = path.relative(ROOT, file).split(path.sep).join("/");
  if (RECORD_PREFIXES.some((p) => rel.startsWith(p))) continue;
  checked++;
  let text = fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n");
  const marker = HISTORY_MARKERS[rel];
  if (marker && text.includes(marker)) text = text.slice(0, text.indexOf(marker));
  text.split("\n").forEach((line, i) => {
    for (const m of line.matchAll(/\]\(([^)#\s]+)(#[^)]*)?\)/g)) {
      const target = m[1];
      if (/^(https?:|mailto:)/.test(target)) continue;
      let decoded = target;
      try {
        decoded = decodeURI(target);
      } catch {
        /* keep the raw target */
      }
      if (!fs.existsSync(path.normalize(path.join(path.dirname(file), decoded)))) errors.push(`${rel}:${i + 1}  broken link ${target}`);
    }
  });
}

// ---------------------------------------------------------------------------- app addresses
// Every web address in the shipped app source must be on the allow-list in scripts/app-addresses.json,
// and every listed address must still appear (no stale entries). Test files and generated bindings are skipped.

const ADDRESS_LIST = "scripts/app-addresses.json";
const APP_SOURCES = [
  "apps/desktop/src",
  "apps/desktop/src-tauri/src",
  "apps/desktop/src-tauri/capabilities",
  // The gold project builder is a development tool, never shipped: its addresses are film research links in project data.
  ...(fs.existsSync(path.join(ROOT, "crates"))
    ? fs.readdirSync(path.join(ROOT, "crates")).filter((c) => c !== "openframe-gold-project").map((c) => `crates/${c}/src`)
    : []),
];
const APP_FILES = ["apps/desktop/src-tauri/tauri.conf.json", "apps/desktop/index.html"];
const SOURCE_EXT = /\.(ts|tsx|rs|json|html)$/;
const NOT_SHIPPED = /(\.test\.tsx?$|\/tests?\/|\/tests\.rs$|\/test_[^/]*\.rs$|\/ipc\/generated\/|\/benches\/|\/fuzz\/)/;
const normalize = (a) => a.replace(/[./]+$/, "");

function* sourceFiles(dir) {
  if (!fs.existsSync(dir)) return;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (SKIP_DIRS.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) yield* sourceFiles(full);
    else if (SOURCE_EXT.test(entry.name)) yield full;
  }
}

const allowed = new Map();
try {
  for (const a of JSON.parse(fs.readFileSync(path.join(ROOT, ADDRESS_LIST), "utf8")).addresses) {
    if (!a.address || !a.why) errors.push(`${ADDRESS_LIST}  every entry needs an address and a why`);
    else allowed.set(a.prefix ? a.address : normalize(a.address), a);
  }
} catch (e) {
  errors.push(`${ADDRESS_LIST}  cannot be read: ${e.message}`);
}
const seen = new Set();
let sources = 0;
const appFiles = [...APP_SOURCES.flatMap((d) => [...sourceFiles(path.join(ROOT, d))]), ...APP_FILES.map((f) => path.join(ROOT, f))];
for (const file of appFiles) {
  const rel = path.relative(ROOT, file).split(path.sep).join("/");
  if (NOT_SHIPPED.test(`/${rel}`) || !fs.existsSync(file)) continue;
  sources++;
  let lines = fs.readFileSync(file, "utf8").split(/\r?\n/);
  // An inline Rust test module is not shipped; by convention it closes the file.
  const testModule = rel.endsWith(".rs") ? lines.findIndex((l) => /^\s*#\[cfg\(test\)\]/.test(l)) : -1;
  if (testModule >= 0) lines = lines.slice(0, testModule);
  lines.forEach((line, i) => {
    if (/^\s*(\/\/|\/?\*)/.test(line)) return; // comments are read by people, never opened by the app
    for (const m of line.matchAll(/\bhttps?:\/\/[A-Za-z0-9][A-Za-z0-9._~:/?#@!$&*+=%-]*/g)) {
      const address = normalize(m[0]);
      const prefix = [...allowed.values()].find((a) => a.prefix && address.startsWith(a.address));
      if (prefix) seen.add(prefix.address);
      else if (allowed.has(address)) seen.add(address);
      else errors.push(`${rel}:${i + 1}  app address ${address} is not in ${ADDRESS_LIST}`);
    }
  });
}
for (const address of allowed.keys()) {
  if (!seen.has(address)) errors.push(`${ADDRESS_LIST}  ${address} no longer appears in the app; remove it from the list`);
}

if (process.argv.includes("--online")) {
  for (const a of allowed.values()) {
    if (!a.online) continue;
    try {
      const r = await fetch(a.address, { method: "GET", redirect: "follow", signal: AbortSignal.timeout(15000) });
      if (r.status >= 400) errors.push(`${ADDRESS_LIST}  ${a.address} answered ${r.status}`);
    } catch (e) {
      errors.push(`${ADDRESS_LIST}  ${a.address} could not be reached: ${e.cause?.code ?? e.message}`);
    }
  }
}

if (errors.length) {
  console.error(`check-links: ${errors.length} problem(s)`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
const online = process.argv.includes("--online") ? ", online addresses fetched" : "";
console.log(`check-links: OK (${checked} documents; ${seen.size} app addresses in ${sources} source files${online})`);
