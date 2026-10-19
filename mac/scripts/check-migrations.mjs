#!/usr/bin/env node
// Migration policy check (build plan item migration policy; docs/engineering/23-release-migration.md §4).
//
//   node scripts/check-migrations.mjs            check (exit 1 on a violation)
//   node scripts/check-migrations.mjs --record   record checksums of new files in migrations/frozen.json
//   node scripts/check-migrations.mjs --base <git-ref>   also compare frozen.json with the base branch (CI)
//
// Rules:
//   1. Linear: each of migrations/{project,global,app} holds NNNN_<name>.sql numbered 1..N with no gap
//      and no duplicate number (two branches that both add 0011 fail here).
//   2. Embedded: crates/openframe-application/src/schema.rs lists exactly those files, in order, with
//      the matching version numbers. A file on disk that the app does not run is an error.
//   3. Frozen: every file recorded in migrations/frozen.json must keep its SHA-256 (line endings
//      normalised to LF). While `"frozen": false` (before the first external build) a changed hash is a
//      warning and `--record` refreshes it; once the owner sets `"frozen": true` a changed or deleted
//      file fails and `--record` refuses to rewrite recorded hashes. New files are always appended.

import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const RECORD = process.argv.includes("--record");
const SETS = ["project", "global", "app"];
const SCHEMA = "crates/openframe-application/src/schema.rs";
const FROZEN = "migrations/frozen.json";

const errors = [];
const warnings = [];
const sha = (text) => crypto.createHash("sha256").update(text.replace(/\r\n/g, "\n"), "utf8").digest("hex");

// 1. Linear numbering
const onDisk = {};
for (const set of SETS) {
  const dir = path.join(ROOT, "migrations", set);
  const files = fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => f.endsWith(".sql")).sort() : [];
  onDisk[set] = files;
  const seen = new Map();
  for (const f of files) {
    const m = /^(\d{4})_[a-z0-9_]+\.sql$/.exec(f);
    if (!m) {
      errors.push(`migrations/${set}/${f}: name must be NNNN_lower_snake.sql`);
      continue;
    }
    const n = Number(m[1]);
    if (seen.has(n)) errors.push(`migrations/${set}: number ${m[1]} used twice (${seen.get(n)}, ${f}); renumber the newer one`);
    seen.set(n, f);
  }
  const nums = [...seen.keys()].sort((a, b) => a - b);
  nums.forEach((n, i) => {
    if (n !== i + 1) {
      const want = String(i + 1).padStart(4, "0");
      errors.push(`migrations/${set}: expected ${want}, found ${String(n).padStart(4, "0")} (numbering must be 1..N with no gap)`);
    }
  });
}

// 2. Embedded list matches the files
const schemaPath = path.join(ROOT, SCHEMA);
if (fs.existsSync(schemaPath)) {
  const schema = fs.readFileSync(schemaPath, "utf8");
  const embedded = { project: [], global: [], app: [] };
  for (const m of schema.matchAll(/m!\(\s*(\d+)\s*,\s*"[^"]*"\s*,\s*"[^"]*migrations\/(project|global|app)\/([^"]+)"\s*,?\s*\)/g)) {
    embedded[m[2]].push({ version: Number(m[1]), file: m[3] });
  }
  for (const set of SETS) {
    const want = onDisk[set];
    const got = embedded[set].map((e) => e.file);
    if (JSON.stringify(want) !== JSON.stringify(got)) {
      errors.push(`${SCHEMA}: ${set} migrations [${got.join(", ")}] do not match migrations/${set} [${want.join(", ")}]`);
    }
    embedded[set].forEach((e, i) => {
      const fileNum = Number(e.file.slice(0, 4));
      if (e.version !== i + 1 || e.version !== fileNum) {
        errors.push(`${SCHEMA}: ${set} entry ${e.file} has version ${e.version}; expected ${i + 1} and the file number ${fileNum}`);
      }
    });
  }
} else {
  errors.push(`${SCHEMA} not found`);
}

// 3. Frozen checksums
const frozenPath = path.join(ROOT, FROZEN);
const frozen = fs.existsSync(frozenPath)
  ? JSON.parse(fs.readFileSync(frozenPath, "utf8"))
  : { frozen: false, policy: "", files: {} };
let changed = false;
const present = new Set();
for (const set of SETS) {
  for (const f of onDisk[set]) {
    const key = `${set}/${f}`;
    present.add(key);
    const hash = sha(fs.readFileSync(path.join(ROOT, "migrations", set, f), "utf8"));
    const recorded = frozen.files[key];
    if (!recorded) {
      if (RECORD) {
        frozen.files[key] = hash;
        changed = true;
      } else {
        warnings.push(`${key}: not recorded yet; run \`node scripts/check-migrations.mjs --record\` in the PR that adds it`);
      }
    } else if (recorded !== hash) {
      if (frozen.frozen) {
        errors.push(`${key}: edited after the freeze. Shipped migrations are append-only; add a new numbered file instead (doc 23 §4)`);
      } else if (RECORD) {
        frozen.files[key] = hash;
        changed = true;
        warnings.push(`${key}: pre-freeze edit recorded`);
      } else {
        warnings.push(`${key}: differs from the recorded checksum (allowed only before the freeze; run --record)`);
      }
    }
  }
}
for (const key of Object.keys(frozen.files)) {
  if (!present.has(key)) errors.push(`${key}: recorded in ${FROZEN} but missing on disk; migrations are never deleted`);
}

// 4. Against the base branch (CI passes --base): a pull request cannot loosen the freeze by editing
//    frozen.json in the same change. Recorded keys are never removed; once the base is frozen, the
//    flag stays true and every file and hash recorded there is unchanged.
const baseIdx = process.argv.indexOf("--base");
if (baseIdx >= 0) {
  const ref = process.argv[baseIdx + 1];
  if (!ref || !/^[\w./~^-]+$/.test(ref)) {
    errors.push(`--base needs a git ref (got ${JSON.stringify(ref)})`);
  } else {
    const show = spawnSync("git", ["show", `${ref}:${FROZEN}`], { cwd: ROOT, encoding: "utf8" });
    if (show.status !== 0) {
      warnings.push(`${FROZEN} does not exist at ${ref}; base comparison skipped`);
    } else {
      const base = JSON.parse(show.stdout);
      if (base.frozen === true && frozen.frozen !== true) errors.push(`${FROZEN}: "frozen" was true at ${ref} and cannot be turned off`);
      for (const [key, hash] of Object.entries(base.files ?? {})) {
        if (!frozen.files[key]) errors.push(`${key}: recorded at ${ref} but removed from ${FROZEN}`);
        else if (base.frozen === true && frozen.files[key] !== hash) errors.push(`${key}: frozen at ${ref}; its recorded checksum may not change`);
      }
    }
  }
}
if (changed && errors.length === 0) {
  const sorted = Object.fromEntries(Object.entries(frozen.files).sort(([a], [b]) => a.localeCompare(b)));
  fs.writeFileSync(frozenPath, JSON.stringify({ ...frozen, files: sorted }, null, 2) + "\n");
}

for (const w of warnings) console.warn(`check-migrations: warning: ${w}`);
if (errors.length) {
  console.error(`check-migrations: ${errors.length} violation(s)`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
const counts = SETS.map((s) => `${s} ${onDisk[s].length}`).join(", ");
console.log(`check-migrations: OK (${counts}; ${frozen.frozen ? "frozen" : "not frozen yet"})`);
