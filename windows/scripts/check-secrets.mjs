#!/usr/bin/env node
// Secret scan of the working tree and the whole git history (ADR-0030 decision 6).
//
//   node scripts/check-secrets.mjs     exit 1 when a secret pattern or a secret file is found
//
// Working tree: every tracked file and every untracked file that .gitignore does not hide.
// History: every line added by any commit on any branch, and every path any commit touched.
// Rules live in scripts/lib/secret-rules.mjs. A hit prints the place and the rule, never the value.

import { execFileSync, spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import readline from "node:readline";
import url from "node:url";
import { isSecretFile, lineHits } from "./lib/secret-rules.mjs";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const MAX_BYTES = 5 * 1024 * 1024;
const hits = [];

// Lines already in history that are not secrets, reviewed one by one. History is never rewritten for them.
const REVIEWED_HISTORY = [
  { commit: "eeacccb", file: "scripts/tests/governance.test.mjs", rule: "team-password-value", why: "a test passes the variable name with a test value it builds at run time" },
];
const reviewed = (commit, file, rule) => REVIEWED_HISTORY.some((r) => commit.startsWith(r.commit) && r.file === file && r.rule === rule);

// ---------------------------------------------------------------------------- working tree

const files = execFileSync("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard"], {
  cwd: ROOT,
  maxBuffer: 256 * 1024 * 1024,
})
  .toString("utf8")
  .split("\0")
  .filter(Boolean);

let scanned = 0;
for (const rel of files) {
  if (isSecretFile(rel)) hits.push(`${rel}  secret file (keys, certificates and .env files are never committed)`);
  const full = path.join(ROOT, rel);
  let stat;
  try {
    stat = fs.statSync(full);
  } catch {
    continue; // listed by git but deleted in the working tree
  }
  if (!stat.isFile() || stat.size > MAX_BYTES) continue;
  const buf = fs.readFileSync(full);
  if (buf.subarray(0, 8000).includes(0)) continue; // binary
  scanned++;
  buf
    .toString("utf8")
    .split(/\r?\n/)
    .forEach((line, i) => {
      for (const id of lineHits(line)) hits.push(`${rel}:${i + 1}  ${id}`);
    });
}

// ---------------------------------------------------------------------------- history

const git = spawn("git", ["-c", "core.quotepath=false", "log", "--all", "-p", "--no-color", "--no-ext-diff", "--no-textconv", "--unified=0", "--format=commit %h"], {
  cwd: ROOT,
});
let gitError = "";
git.stderr.on("data", (d) => (gitError += d));
let commit = "";
let file = "";
let line = 0;
// A "+++ " line is a file header only right after a "--- " header; inside a hunk it is an added line.
let afterMinusHeader = false;
let commits = 0;
const seenPaths = new Set();
for await (const raw of readline.createInterface({ input: git.stdout, crlfDelay: Infinity })) {
  if (raw.startsWith("commit ")) {
    commit = raw.slice(7);
    commits++;
  } else if (raw.startsWith("diff --git ")) {
    file = "";
    afterMinusHeader = false;
  } else if (!file && raw.startsWith("--- ")) {
    afterMinusHeader = true;
  } else if (afterMinusHeader && raw.startsWith("+++ ")) {
    afterMinusHeader = false;
    file = raw === "+++ /dev/null" ? "" : raw.slice(6);
    if (file && isSecretFile(file) && !seenPaths.has(file)) {
      seenPaths.add(file);
      hits.push(`history ${commit} ${file}  secret file was committed`);
    }
  } else if (raw.startsWith("@@ ")) {
    line = Number(/\+(\d+)/.exec(raw)?.[1] ?? 0);
  } else if (raw.startsWith("+") && file) {
    for (const id of lineHits(raw.slice(1))) if (!reviewed(commit, file, id)) hits.push(`history ${commit} ${file}:${line}  ${id}`);
    line++;
  }
}
const code = await new Promise((resolve) => (git.exitCode === null ? git.on("close", resolve) : resolve(git.exitCode)));
if (code !== 0) {
  console.error(`check-secrets: git log failed (${code}): ${gitError.trim()}`);
  process.exit(2);
}

if (hits.length) {
  console.error(`check-secrets: ${hits.length} finding(s). Remove the secret, rotate it, and for history ask the owner before any rewrite.`);
  for (const h of hits) console.error(`  ${h}`);
  process.exit(1);
}
console.log(`check-secrets: OK (${scanned} files in the working tree, ${commits} commits of history)`);
