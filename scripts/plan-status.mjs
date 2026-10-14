#!/usr/bin/env node
// Build plan progress counter.
//
//   node scripts/plan-status.mjs            print checked and total boxes per area and overall
//   node scripts/plan-status.mjs --json     the same as JSON
//   node scripts/plan-status.mjs --check    exit 1 when a checked box has no `(ev: ...)` evidence
//
// Reads docs/implementation/masterplan.md. An area is a `## N. Title` heading. A box is a list line that starts
// with `- [ ]` or `- [x]`. Boxes outside numbered areas (for example the release sign-off table) are not counted.
//
// A box tagged `(Later: store publishing)` belongs to the later store publishing step (owner decision 2026-10-08).
// The "Current goal" line counts only the boxes without that tag; the "Later: store publishing" line counts the rest.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const PLAN = "docs/implementation/masterplan.md";
const LATER_TAG = "(Later: store publishing)";

const lines = fs.readFileSync(path.join(ROOT, PLAN), "utf8").replace(/\r\n/g, "\n").split("\n");
const areas = [];
let current = null;
const missingEvidence = [];

lines.forEach((raw, i) => {
  const h = /^## (\d+)\. (.+)$/.exec(raw);
  if (h) {
    current = { number: Number(h[1]), title: h[2].trim(), done: 0, total: 0, later: 0, laterDone: 0 };
    areas.push(current);
    return;
  }
  if (/^## /.test(raw)) {
    current = null;
    return;
  }
  const box = /^- \[( |x)\] (.*)$/.exec(raw);
  if (!box || !current) return;
  current.total++;
  const later = box[2].includes(LATER_TAG);
  if (later) current.later++;
  if (box[1] === "x") {
    current.done++;
    if (later) current.laterDone++;
    if (!/\(ev: /.test(box[2])) missingEvidence.push(`${PLAN}:${i + 1}  checked box has no (ev: ...) evidence`);
  }
});

const done = areas.reduce((n, a) => n + a.done, 0);
const total = areas.reduce((n, a) => n + a.total, 0);
const pct = (d, t) => (t ? Math.round((d / t) * 1000) / 10 : 0);
const percent = pct(done, total);
const later = areas.reduce((n, a) => n + a.later, 0);
const laterDone = areas.reduce((n, a) => n + a.laterDone, 0);
const currentGoal = { done: done - laterDone, total: total - later };
currentGoal.percent = pct(currentGoal.done, currentGoal.total);

if (process.argv.includes("--json")) {
  console.log(JSON.stringify({ done, total, percent, currentGoal, laterStorePublishing: later, areas }, null, 2));
} else {
  for (const a of areas) console.log(`${String(a.number).padStart(2)}. ${a.title}: ${a.done} of ${a.total}`);
  console.log(`Overall: ${done} of ${total} (${percent}%)`);
  console.log(`Current goal: ${currentGoal.done} of ${currentGoal.total} (${currentGoal.percent}%)`);
  console.log(`Later: store publishing: ${later} boxes`);
}

if (process.argv.includes("--check") && missingEvidence.length) {
  console.error(`plan-status: ${missingEvidence.length} problem(s)`);
  for (const e of missingEvidence) console.error(`  ${e}`);
  process.exit(1);
}
