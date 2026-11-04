#!/usr/bin/env node
// Release guard for placeholders and for the Store submission record (ADR-0039 items 14 and 15,
// docs/release/package-2026-11-03/06_PLACEHOLDERS_AND_COMPANY_DETAILS.md sections C to E).
//
//   node scripts/check-release-placeholders.mjs [--root <dir>]
//
// Prints each leftover placeholder (file and token) and exits 1 when any remains; exits 0 otherwise.
// scripts/release-validate.mjs runs it for Store releases only. Debug, test and internal builds
// (node scripts/build-msix.mjs --dev) never call it.
//
// What is scanned
//   - packaging/msix/store.config.json, judged through loadStoreConfig, so the CI variables
//     MSIX_IDENTITY_NAME, MSIX_PUBLISHER and MSIX_PUBLISHER_DISPLAY_NAME count: a real value from
//     the environment is not a failure.
//   - the manifest rendered from packaging/msix/AppxManifest.template.xml with those values.
//   - RAW_FILES, as plain text: the release workflow and the licensing rules.
// A token is REPLACE_WITH_ followed by capitals, digits or underscores. Comments in scanned files
// must not spell that token out.
//
// The same module holds the pure helpers for packaging/msix/submissions.json (see that file).

import fs from "node:fs";
import path from "node:path";
import url from "node:url";

import { loadStoreConfig, renderManifest } from "./build-msix.mjs";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");

export const CONFIG_REL = "packaging/msix/store.config.json";
export const TEMPLATE_REL = "packaging/msix/AppxManifest.template.xml";
export const SUBMISSIONS_REL = "packaging/msix/submissions.json";
/** Scanned as plain text. Missing files fail closed. */
export const RAW_FILES = [".github/workflows/release.yml", "apps/desktop/src/features/licensing/rules.ts"];

const TOKEN = /REPLACE_WITH_[A-Z0-9_]+/g;
const tokens = (text) => [...new Set(String(text).match(TOKEN) ?? [])];

/** Every leftover placeholder as { file, placeholder }. Empty array = clean. */
export function scanPlaceholders({ root = ROOT, env = process.env } = {}) {
  const hits = [];
  const add = (file, text) => tokens(text).forEach((placeholder) => hits.push({ file, placeholder }));

  let cfg;
  try {
    cfg = loadStoreConfig(env, path.join(root, CONFIG_REL));
  } catch (e) {
    hits.push({ file: CONFIG_REL, placeholder: `unreadable: ${e.message}` });
  }
  if (cfg) {
    add(CONFIG_REL, JSON.stringify(cfg));
    try {
      const template = fs.readFileSync(path.join(root, TEMPLATE_REL), "utf8");
      add(`${TEMPLATE_REL} (rendered)`, renderManifest(template, { ...cfg, version: "1.0.0.0" }));
    } catch (e) {
      hits.push({ file: TEMPLATE_REL, placeholder: `cannot render: ${e.message}` });
    }
  }
  for (const rel of RAW_FILES) {
    try {
      add(rel, fs.readFileSync(path.join(root, rel), "utf8"));
    } catch (e) {
      hits.push({ file: rel, placeholder: `unreadable: ${e.message}` });
    }
  }
  return hits;
}

// ---------------------------------------------------------------------------------------------
// Submission record helpers (packaging/msix/submissions.json)

/** Compare dotted numeric versions part by part. Returns -1, 0 or 1; throws on a malformed one. */
export function compareVersions(a, b) {
  const parse = (v) => {
    if (!/^\d+(\.\d+){0,3}$/.test(String(v))) throw new Error(`"${v}" is not a dotted numeric version`);
    return String(v).split(".").map(Number);
  };
  const pa = parse(a);
  const pb = parse(b);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d < 0 ? -1 : 1;
  }
  return 0;
}

/** Problem text when the Store package version is not valid for a new submission, else null. */
export function checkVersionRises(current, lastSubmitted) {
  try {
    if (!/^\d+\.\d+\.\d+\.0$/.test(String(current))) return `package version ${current} must be Major.Minor.Build.0 (the Store reserves the fourth part)`;
    if (lastSubmitted === null || lastSubmitted === undefined) return null;
    if (compareVersions(current, lastSubmitted) <= 0) return `package version ${current} must be higher than the last submitted version ${lastSubmitted} (packaging/msix/submissions.json)`;
    return null;
  } catch (e) {
    return e.message;
  }
}

/** Problem text when the identity differs from the one recorded at first submission, else null. */
export function checkIdentityUnchanged(identity, first) {
  if (first === null || first === undefined) return null;
  const diffs = [];
  if (identity.name !== first.name) diffs.push(`name "${identity.name}" vs recorded "${first.name}"`);
  if (identity.publisher !== first.publisher) diffs.push(`publisher "${identity.publisher}" vs recorded "${first.publisher}"`);
  return diffs.length ? `identity changed since the first submission (${diffs.join("; ")}); a different identity is a different app` : null;
}

/** Reads packaging/msix/submissions.json. Throws when it is missing or malformed (fail closed). */
export function loadSubmissions(root = ROOT) {
  const raw = JSON.parse(fs.readFileSync(path.join(root, SUBMISSIONS_REL), "utf8"));
  const first = raw.firstSubmissionIdentity ?? null;
  if (first !== null && (typeof first.name !== "string" || typeof first.publisher !== "string")) {
    throw new Error("firstSubmissionIdentity must be null or { name, publisher }");
  }
  return { firstSubmissionIdentity: first, lastSubmittedVersion: raw.lastSubmittedVersion ?? null };
}

// ---------------------------------------------------------------------------------------------

function main() {
  const argv = process.argv.slice(2);
  const i = argv.indexOf("--root");
  const root = i >= 0 ? path.resolve(argv[i + 1]) : ROOT;
  const hits = scanPlaceholders({ root });
  if (!hits.length) {
    console.log("No release placeholders remain.");
    return;
  }
  console.error("Release placeholders remain:");
  for (const h of hits) console.error(`  ${h.file}: ${h.placeholder}`);
  console.error("The owner supplies these (package file 06 section D); CI variables count for the identity values.");
  process.exit(1);
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) main();
