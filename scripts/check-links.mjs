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
const SKIP_DIRS = new Set(["node_modules", "target", ".git", "graphify-out", ".claude", "dist", "gen"]);
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

if (errors.length) {
  console.error(`check-links: ${errors.length} broken link(s)`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log(`check-links: OK (${checked} documents)`);
