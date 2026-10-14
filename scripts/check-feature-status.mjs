#!/usr/bin/env node
// Feature status check (build plan item feature status check in CI; feature-status.md "How to read" rule 5).
//
//   node scripts/check-feature-status.mjs     exit 1 when an in-scope row is incomplete or claims more than its evidence
//
// Reads docs/implementation/feature-status.md. Every section that holds a feature table must have a `Source:` line
// naming the specification sections the rows come from. Every row of a table whose header is
//   | Feature | Importance | What exists today | What is missing | Build area | How a user reaches it | Tests | Status | Key |
// is checked. A row is in scope unless its Importance is `Removed` or its Status is `Not applicable`.
//
//   Key                     unique, AREA-001 or AREA-WORD-001 (stable lookup key; wording may change, keys do not)
//   Importance              Must ship, Should ship or Can wait (or Removed)
//   Build area              the title of a numbered area in masterplan.md, or `Every area`
//   How a user reaches it   a route (`home`, `story`, `production/schedule`, `app-home`, `ai-panel`, `menu: ...`), or
//                           `none: <reason>` for a feature with no UI of its own, or `pending`
//   Tests                   test references (`file.rs::test_name`, `file.rs`, `spec.test.mjs`, `component.test.tsx`), or `pending`
//   Status                  Specified, Designed, Built, Reachable, Verified, Release-ready (or `Process` in the documentation section)
//
// Evidence rules:
//   Built and above need Tests that are not `pending`                  (Built = implemented and passes its own tests)
//   Reachable and above need a route that is not `pending`             (Reachable = a normal user can use it)
//   No row may cite an unverified [R], [A] or [S] tag; the feature status check replaced them with evidence or a corrected status.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import {
  DOC_HEADER,
  FEATURE_STATUS,
  HEADER,
  IMPORTANCE,
  STATUSES,
  readBuildAreas,
  splitRow,
  statusLevel,
} from "./lib/feature-status.mjs";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);

const text = fs.readFileSync(path.join(ROOT, FEATURE_STATUS), "utf8").replace(/\r\n/g, "\n");
const lines = text.split("\n");
const areas = readBuildAreas(ROOT);
const errors = [];
const err = (line, key, msg) => errors.push(`${FEATURE_STATUS}:${line}${key ? ` ${key}` : ""}  ${msg}`);

const keys = new Map();
let section = null;
let sectionHasSource = false;
let sectionHasTable = false;
let header = null;
let rows = 0;
const counts = {};

const closeSection = (line) => {
  if (section && sectionHasTable && !sectionHasSource) err(line, null, `section "${section}" has no \`Source:\` line`);
};

lines.forEach((raw, i) => {
  const n = i + 1;
  if (/^## /.test(raw)) {
    closeSection(n);
    section = raw.slice(3).trim();
    sectionHasSource = false;
    sectionHasTable = false;
    header = null;
    return;
  }
  if (/^Source:\s*\S/.test(raw)) sectionHasSource = true;
  if (!raw.startsWith("|")) {
    header = null;
    return;
  }
  const c = splitRow(raw);
  if (c[0] === "Feature") {
    header = c;
    if (same(c, HEADER) || same(c, DOC_HEADER)) sectionHasTable = true;
    else err(n, null, `feature table header must be | ${HEADER.join(" | ")} |`);
    return;
  }
  if (!header || /^-+$/.test(c[0].replace(/:/g, ""))) return;
  if (!(same(header, HEADER) || same(header, DOC_HEADER))) return;

  const row = Object.fromEntries(header.map((h, k) => [h, c[k] ?? ""]));
  const key = row.Key;
  rows++;
  if (c.length !== header.length) err(n, key, `has ${c.length} cells, header has ${header.length}`);
  if (!/^[A-Z]+(-[A-Z]+)?-\d{3}$/.test(key)) err(n, key, "Key must look like AREA-001 or AREA-WORD-001");
  if (keys.has(key)) err(n, key, `duplicate Key (also line ${keys.get(key)})`);
  keys.set(key, n);

  const status = row.Status;
  if (row.Importance === "Removed" || status === "Not applicable") return;
  counts[status] = (counts[status] ?? 0) + 1;

  if (!row.Feature) err(n, key, "Feature is empty");
  if (areas && !areas.has(row["Build area"])) {
    err(n, key, `Build area "${row["Build area"]}" is not a numbered area of masterplan.md (or "Every area")`);
  }

  if (same(header, DOC_HEADER)) {
    if (!(STATUSES.includes(status) || status === "Process")) err(n, key, `Status "${status}" must be one of ${STATUSES.join(", ")} or Process`);
    return;
  }

  if (!IMPORTANCE.includes(row.Importance)) err(n, key, `Importance "${row.Importance}" must be Must ship, Should ship or Can wait`);
  if (!STATUSES.includes(status)) err(n, key, `Status "${status}" must be one of ${STATUSES.join(", ")}`);
  const level = statusLevel(status);
  const route = row["How a user reaches it"];
  const tests = row.Tests;
  if (!route) err(n, key, "How a user reaches it is empty (use a route, `none: <reason>` or `pending`)");
  if (!tests) err(n, key, "Tests is empty (use test references or `pending`)");
  if (level >= statusLevel("Built") && /^pending\b/.test(tests)) {
    err(n, key, `Status ${status} needs passing tests; Tests says pending (lower the status or add tests)`);
  }
  if (level >= statusLevel("Reachable") && /^pending\b/.test(route)) err(n, key, `Status ${status} means reachable; the route says pending`);
  if (level >= statusLevel("Reachable") && /^none:/.test(route) && !/^none: (backend|process|release|platform|packaging)/.test(route)) {
    err(n, key, `Status ${status} with no screen needs a reason starting backend, process, release, platform or packaging`);
  }
  if (/\[(R|A|S)\]/.test(row["What exists today"])) {
    err(n, key, "What exists today cites an unverified [R]/[A]/[S] tag; replace it with evidence or a corrected status");
  }
});
closeSection(lines.length);

if (rows === 0) err(0, null, "no feature rows found");
if (errors.length) {
  console.error(`check-feature-status: ${errors.length} problem(s)`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
const summary = STATUSES.concat(["Process"])
  .filter((s) => counts[s])
  .map((s) => `${s} ${counts[s]}`)
  .join(", ");
console.log(`check-feature-status: OK (${keys.size} rows; ${summary})`);
