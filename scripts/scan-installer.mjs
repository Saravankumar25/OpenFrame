#!/usr/bin/env node
// String scan of built files before they leave the build computer (ADR-0030 decision 6; Windows phase W9).
//
//   node scripts/scan-installer.mjs <file or folder>...     exit 1 on a finding
//
// Reads every file given (folders recursively), pulls out the printable text runs (ASCII and UTF-16), and refuses:
//   - a secret pattern from scripts/lib/secret-rules.mjs;
//   - a personal path (C:\Users\<name>, /Users/<name>, /home/<name>), which leaks the builder's name and folders;
//     the CI build accounts of third-party binaries (runneradmin on Windows, runner on macOS) are not personal;
//   - the build computer's user name anywhere;
//   - the team password in plain text, when OPENFRAME_TEAM_PASSWORD is set (only its hash may be embedded).
// A compressed installer hides its payload, so run it on the unpacked release folder as well as on the installer.
// Findings name the file, the rule and the byte offset, never the matched text.

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { lineHits } from "./lib/secret-rules.mjs";

// A JavaScript string holds at most about 512 million characters; larger files are refused, not crashed on.
const MAX_BYTES = 400 * 1024 * 1024;
const MIN_RUN = 6;
const PERSONAL_PATH = /[A-Za-z]:[\\/]{1,2}Users[\\/]{1,2}(?!(?:Public|Default|All Users|runneradmin)[\\/])[^\\/\s"]+|\/Users\/(?!Shared\/|runner\/)[^/\s"]+|\/home\/[^/\s"]+/i;
const user = (process.env.OPENFRAME_SCAN_USER ?? os.userInfo().username ?? "").trim();
const password = process.env.OPENFRAME_TEAM_PASSWORD ?? "";

const targets = process.argv.slice(2);
if (targets.length === 0) {
  console.error("usage: node scripts/scan-installer.mjs <file or folder>...");
  process.exit(2);
}

function* walk(p) {
  const stat = fs.statSync(p);
  if (stat.isDirectory()) for (const e of fs.readdirSync(p)) yield* walk(path.join(p, e));
  else if (stat.isFile()) yield p;
}

/** Printable runs as [offset, text]: single-byte ASCII and UTF-16 little-endian. */
function* textRuns(buf) {
  const latin = buf.toString("latin1");
  for (const m of latin.matchAll(new RegExp(`[\\x20-\\x7e]{${MIN_RUN},}`, "g"))) yield [m.index, m[0]];
  for (const m of latin.matchAll(new RegExp(`(?:[\\x20-\\x7e]\\x00){${MIN_RUN},}`, "g"))) yield [m.index, m[0].replace(/\x00/g, "")];
}

const findings = [];
let files = 0;
for (const target of targets) {
  if (!fs.existsSync(target)) {
    findings.push(`${target}  does not exist`);
    continue;
  }
  for (const file of walk(target)) {
    const size = fs.statSync(file).size;
    if (size > MAX_BYTES) {
      findings.push(`${file}  is larger than 400 MB and was not scanned; scan its unpacked contents`);
      continue;
    }
    files++;
    const buf = fs.readFileSync(file);
    if (password && buf.includes(password)) findings.push(`${file}  holds the team password in plain text`);
    if (password && buf.includes(Buffer.from(password, "utf16le"))) findings.push(`${file}  holds the team password in plain UTF-16 text`);
    const lowerUser = user.length >= 3 ? user.toLowerCase() : "";
    for (const [offset, text] of textRuns(buf)) {
      for (const id of lineHits(text)) findings.push(`${file} @${offset}  ${id}`);
      if (PERSONAL_PATH.test(text)) findings.push(`${file} @${offset}  personal path`);
      else if (lowerUser && text.toLowerCase().includes(lowerUser)) findings.push(`${file} @${offset}  build computer user name`);
    }
  }
}

if (findings.length) {
  const shown = findings.slice(0, 200);
  console.error(`scan-installer: ${findings.length} finding(s) in ${files} file(s)`);
  for (const f of shown) console.error(`  ${f}`);
  if (findings.length > shown.length) console.error(`  ... and ${findings.length - shown.length} more`);
  process.exit(1);
}
console.log(`scan-installer: OK (${files} file(s), no secret, personal path, user name or plain team password)`);
