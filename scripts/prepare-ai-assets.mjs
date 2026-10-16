#!/usr/bin/env node
// Offline AI engine: verify the build inputs against the committed lock and stage the engine for
// the Tauri bundle (ADR-0026 decision 1).
//
// Input:  packaging/ai/ai-assets.lock.json (committed: every engine and model file by path, bytes
//         and SHA-256, plus licence ids) and the git-ignored `.ai-assets/` folder that holds the
//         real files (runtimes/<runtime-id>/..., models/<model-id>/model.gguf).
// Output: .build-cache/ai-bundle/ (git-ignored): exactly the engine files the lock lists for the
//         target platform, in the same layout, plus packaging/ai/notices/. The Tauri overlay config
//         apps/desktop/src-tauri/tauri.bundled-ai.conf.json maps that folder to the `ai/` folder of
//         the app resources. The models are not packaged: the person downloads them after install
//         (ADR-0026), straight from Hugging Face at the revisions this lock pins (ADR-0032).
//
// The build fails closed: a missing file, a wrong size, a wrong hash, a link, an unknown target
// platform or an invalid lock stops it with a plain message and stages nothing. Nothing is ever
// downloaded. Files are hard-linked into the staging folder when the drive allows it, and copied
// otherwise.
//
//   node scripts/prepare-ai-assets.mjs                 verify and stage for this computer
//   node scripts/prepare-ai-assets.mjs --check         verify only, stage nothing
//   node scripts/prepare-ai-assets.mjs --optional      development runs: when the build inputs are
//                                                      absent, stage an empty folder (the app then
//                                                      shows its "Offline AI is missing" state)
//                                                      instead of failing; a wrong file still fails
//   Options: --target <os-arch> (windows-x86_64, macos-aarch64)  --source <dir>  --out <dir>
//
// Where the build inputs are looked up: --source, then OPENFRAME_AI_ASSETS_DIR, then `.ai-assets`
// in this checkout, then `.ai-assets` in the main checkout when this is a git worktree.
//
// Changing a pinned file: put the new file in `.ai-assets`, update its entry in the lock (path,
// bytes, sha256, licence) in a reviewed change, and raise `bundleVersion`. The app embeds the same
// lock and checks the installed engine against it before it first starts it.

import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

export const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
export const LOCK_PATH = path.join(ROOT, "packaging", "ai", "ai-assets.lock.json");
export const NOTICES_DIR = path.join(ROOT, "packaging", "ai", "notices");
export const DEFAULT_OUT = path.join(ROOT, ".build-cache", "ai-bundle");
/** Folder name inside the app resources; must match `RESOURCE_FOLDER` in crates/openframe-ai/src/bundle.rs. */
export const RESOURCE_FOLDER = "ai";

const SAFE_ID = /^[a-z0-9][a-z0-9.-]{0,78}[a-z0-9]$/;
const SAFE_FILE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,120}$/;
const SHA = /^[0-9a-f]{64}$/;
// Windows device names cannot be file or folder names, with or without an extension (same rule as the app).
const DEVICE = /^(con|prn|aux|nul|com[0-9]|lpt[0-9])(\.|$)/i;
const safeName = (n) => typeof n === "string" && !n.includes("..") && !DEVICE.test(n);
const MAX_FILES = 200;

export class AiAssetsError extends Error {}

const fail = (msg) => {
  throw new AiAssetsError(msg);
};

/** `windows-x86_64` / `macos-aarch64` for this computer. */
export function currentTarget() {
  const os = { win32: "windows", darwin: "macos" }[process.platform] ?? process.platform;
  const arch = { x64: "x86_64", arm64: "aarch64" }[process.arch] ?? process.arch;
  return `${os}-${arch}`;
}

function checkFileEntry(f, where) {
  if (!f || typeof f !== "object") fail(`${where}: file entry is missing`);
  if (typeof f.path !== "string" || !SAFE_FILE.test(f.path) || !safeName(f.path)) fail(`${where}: unsafe file path "${f.path}"`);
  if (!Number.isSafeInteger(f.bytes) || f.bytes <= 0) fail(`${where}: ${f.path} has no valid size`);
  if (typeof f.sha256 !== "string" || !SHA.test(f.sha256)) fail(`${where}: ${f.path} has no valid SHA-256`);
}

/** Parse and validate the lock. Throws AiAssetsError with a plain message. */
export function readLock(lockPath = LOCK_PATH) {
  let lock;
  try {
    lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
  } catch (e) {
    fail(`The Offline AI lock file ${path.relative(ROOT, lockPath)} cannot be read: ${e.message}`);
  }
  if (lock.lockVersion !== 1) fail("The Offline AI lock file has an unsupported lockVersion (expected 1).");
  if (lock.bundleId !== "openframe-local-ai-v1" || !SAFE_ID.test(lock.bundleVersion ?? "")) fail("The Offline AI lock file names an unknown bundle.");
  if (!Array.isArray(lock.runtimes) || !Array.isArray(lock.models)) fail("The Offline AI lock file lists no runtimes or models.");
  const ids = new Set();
  for (const r of lock.runtimes) {
    const where = `runtime ${r.runtimeId}`;
    if (!SAFE_ID.test(r.runtimeId ?? "") || !safeName(r.runtimeId) || ids.has(r.runtimeId)) fail(`${where}: invalid or duplicate id`);
    ids.add(r.runtimeId);
    if (!["windows", "macos"].includes(r.os) || !["x86_64", "aarch64"].includes(r.arch)) fail(`${where}: unknown platform ${r.os}-${r.arch}`);
    if (!["cpu", "vulkan"].includes(r.backend)) fail(`${where}: unknown engine build ${r.backend} (processor or Vulkan only, ADR-0029)`);
    if (typeof r.licenseId !== "string" || !r.licenseId.trim()) fail(`${where}: licence id missing`);
    if (!Array.isArray(r.files) || r.files.length === 0 || r.files.length > MAX_FILES) fail(`${where}: file list missing or too long`);
    const names = new Set();
    for (const f of r.files) {
      checkFileEntry(f, where);
      if (names.has(f.path.toLowerCase())) fail(`${where}: duplicate file ${f.path}`);
      names.add(f.path.toLowerCase());
    }
    if (!names.has(String(r.executable).toLowerCase())) fail(`${where}: the server program ${r.executable} is not in its file list`);
  }
  const roles = { chat: 0, embedding: 0, multilingualEmbedding: 0 };
  for (const m of lock.models) {
    const where = `model ${m.modelId}`;
    if (!SAFE_ID.test(m.modelId ?? "") || !safeName(m.modelId) || ids.has(m.modelId)) fail(`${where}: invalid or duplicate id`);
    ids.add(m.modelId);
    if (!(m.role in roles)) fail(`${where}: unknown role ${m.role}`);
    roles[m.role] += 1;
    if (typeof m.licenseId !== "string" || !m.licenseId.trim()) fail(`${where}: licence id missing`);
    if (m.tokenizer !== "embedded") fail(`${where}: the tokenizer must be embedded in the model file (or listed as its own file)`);
    checkFileEntry(m.file, where);
  }
  if (roles.chat !== 1 || roles.embedding !== 1 || roles.multilingualEmbedding > 1) fail("The Offline AI lock file must list exactly one language model, one search model and at most one multilingual search model.");
  return lock;
}

/** Every file the target platform packages (the engine): [{ rel: "runtimes/<id>/<file>", bytes, sha256 }]. */
export function bundledFiles(lock, target) {
  const [os, arch] = target.split("-");
  const runtimes = lock.runtimes.filter((r) => r.os === os && r.arch === arch);
  // Exactly one processor build per platform; a Vulkan build beside it is optional (ADR-0029).
  if (runtimes.filter((r) => r.backend === "cpu").length !== 1 || runtimes.filter((r) => r.backend === "vulkan").length > 1) {
    fail(
      `No Offline AI engine is pinned for ${target}. Every package contains the Offline AI engine (ADR-0026 decision 1), so this build stops. ` +
        "Add the engine for this platform to packaging/ai/ai-assets.lock.json and its files to .ai-assets.",
    );
  }
  return runtimes.flatMap((r) => r.files.map((f) => ({ rel: `runtimes/${r.runtimeId}/${f.path}`, bytes: f.bytes, sha256: f.sha256 })));
}

function sha256File(file) {
  const h = crypto.createHash("sha256");
  const fd = fs.openSync(file, "r");
  try {
    const buf = Buffer.allocUnsafe(1 << 20);
    for (;;) {
      const n = fs.readSync(fd, buf, 0, buf.length, null);
      if (n === 0) break;
      h.update(buf.subarray(0, n));
    }
  } finally {
    fs.closeSync(fd);
  }
  return h.digest("hex");
}

/** Problems with the files under `dir` (empty array = every file present, regular, right size and hash). */
export function verifyFiles(dir, files) {
  const problems = [];
  for (const f of files) {
    const p = path.join(dir, ...f.rel.split("/"));
    let st;
    try {
      st = fs.lstatSync(p);
    } catch {
      problems.push(`${f.rel} is missing`);
      continue;
    }
    if (st.isSymbolicLink() || !st.isFile()) {
      problems.push(`${f.rel} is not a regular file`);
      continue;
    }
    if (st.size !== f.bytes) {
      problems.push(`${f.rel} has ${st.size} bytes, the lock expects ${f.bytes}`);
      continue;
    }
    if (sha256File(p) !== f.sha256) problems.push(`${f.rel} does not match its pinned SHA-256`);
  }
  return problems;
}

function mainCheckoutAssets() {
  const r = spawnSync("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], { cwd: ROOT, encoding: "utf8" });
  if (r.status !== 0) return null;
  return path.join(path.dirname(r.stdout.trim()), ".ai-assets");
}

/** The folder holding the build inputs, or null when none exists. */
export function findSource(explicit) {
  const candidates = [explicit, process.env.OPENFRAME_AI_ASSETS_DIR, path.join(ROOT, ".ai-assets"), mainCheckoutAssets()].filter(Boolean);
  for (const c of candidates) {
    const abs = path.resolve(c);
    if (fs.existsSync(abs) && fs.statSync(abs).isDirectory()) return abs;
    if (c === explicit || c === process.env.OPENFRAME_AI_ASSETS_DIR) fail(`The Offline AI build inputs folder ${abs} does not exist.`);
  }
  return null;
}

function linkOrCopy(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  try {
    fs.linkSync(src, dst);
  } catch {
    fs.copyFileSync(src, dst);
  }
}

function resetOut(out, src) {
  // Only ever clear a folder inside this checkout's git-ignored build cache, never the build
  // inputs or anything around them.
  const cache = path.join(ROOT, ".build-cache") + path.sep;
  const abs = path.resolve(out);
  if (!abs.startsWith(cache) || (src && (abs + path.sep).startsWith(path.resolve(src) + path.sep)) || (src && (path.resolve(src) + path.sep).startsWith(abs + path.sep))) {
    fail(`The staging folder must be inside ${path.relative(ROOT, cache)} and apart from the build inputs (got ${abs}).`);
  }
  fs.rmSync(abs, { recursive: true, force: true });
  fs.mkdirSync(out, { recursive: true });
}

/**
 * Verify the build inputs and stage them. Returns { out, files, bytes, staged }.
 * `optional`: stage an empty folder when no build inputs exist (development runs only).
 */
export function prepare({ target = currentTarget(), source, out = DEFAULT_OUT, check = false, optional = false, log = console.log } = {}) {
  const lock = readLock();
  const files = bundledFiles(lock, target);
  const bytes = files.reduce((s, f) => s + f.bytes, 0);
  const src = findSource(source);
  if (!src) {
    const msg =
      "The Offline AI build inputs are missing. Every package contains the Offline AI engine (ADR-0026 decision 1), so the build stops. " +
      "Put the pinned files in .ai-assets (see packaging/ai/ai-assets.lock.json) or set OPENFRAME_AI_ASSETS_DIR.";
    if (!optional) fail(msg);
    log(`Offline AI: build inputs not found; staging an empty folder for this development run (${path.relative(ROOT, out)}).`);
    if (!check) resetOut(out, null);
    return { out, files, bytes, staged: false };
  }
  log(`Offline AI: verifying ${files.length} pinned files (${(bytes / 1048576).toFixed(0)} MB) for ${target} in ${src}`);
  const problems = verifyFiles(src, files);
  if (problems.length) {
    fail(`The Offline AI build inputs do not match the lock, so the build stops (nothing was staged):\n  - ${problems.join("\n  - ")}`);
  }
  if (check) {
    log("Offline AI: every pinned file matches the lock.");
    return { out, files, bytes, staged: false };
  }
  resetOut(out, src);
  for (const f of files) linkOrCopy(path.join(src, ...f.rel.split("/")), path.join(out, ...f.rel.split("/")));
  for (const n of fs.readdirSync(NOTICES_DIR)) linkOrCopy(path.join(NOTICES_DIR, n), path.join(out, "notices", n));
  // The staged files are checked again in full (a copy that failed half way, or a hard-linked input
  // changed after its check, never reaches the bundle).
  const staged = verifyFiles(out, files);
  if (staged.length) fail(`Staging the Offline AI failed, so the build stops:\n  - ${staged.join("\n  - ")}`);
  log(`Offline AI: staged in ${path.relative(ROOT, out) || out}`);
  return { out, files, bytes, staged: true };
}

function parseArgs(argv) {
  const a = {};
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === "--check") a.check = true;
    else if (k === "--optional") a.optional = true;
    else if (k === "--target") a.target = argv[++i];
    else if (k === "--source") a.source = argv[++i];
    else if (k === "--out") a.out = path.resolve(argv[++i]);
    else throw new AiAssetsError(`unknown argument ${k} (see the header of scripts/prepare-ai-assets.mjs)`);
  }
  return a;
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) {
  try {
    prepare(parseArgs(process.argv.slice(2)));
  } catch (e) {
    console.error(`\n${e instanceof AiAssetsError ? e.message : e.stack ?? e}`);
    process.exit(1);
  }
}
