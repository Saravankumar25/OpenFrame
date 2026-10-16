#!/usr/bin/env node
// The OpenFrame AI download list (ADR-0026 decisions 4 and 8).
//
// Builds a model host folder: the model files the lock pins, under <version>/, and manifest.json,
// the list the app downloads first and checks with the public key compiled into it
// (OPENFRAME_MANIFEST_PUBLIC_KEY). The host serves this folder as it is, over https.
//
//   node scripts/model-manifest.mjs new-key --out <private-key.pem>
//       The owner, once, on an offline computer: makes the Ed25519 key pair, writes the private key
//       to that file (keep it offline, never in this repository) and prints the public key to set
//       as OPENFRAME_MANIFEST_PUBLIC_KEY on the build computer.
//   node scripts/model-manifest.mjs publish --key <private-key.pem> --out <host folder> --sequence <n> [--version <v>] [--source <dir>]
//       Copies the pinned model files from the build inputs (.ai-assets, or --source) after checking
//       them against the lock, and writes the signed manifest.json. --sequence must be higher than
//       every list signed before (the app refuses an older list than the one it uses). --version
//       defaults to the lock's bundleVersion; a new version makes the app download the set again.
//
// The test suite uses testHost(): a throwaway key made in .build-cache/e2e-model-host (git-ignored),
// never the production key, and the same publish step.

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import { AiAssetsError, ROOT, findSource, readLock, verifyFiles } from "./prepare-ai-assets.mjs";

export const MANIFEST_NAME = "manifest.json";
export const ENVELOPE_FORMAT = "openframe-ai-manifest";
/** The model profile the app runs (crates/openframe-ai/src/model_download.rs PROFILE_VERSION). */
export const PROFILE_VERSION = 2;
export const TEST_HOST_DIR = path.join(ROOT, ".build-cache", "e2e-model-host");
const SAFE = /^[A-Za-z0-9][A-Za-z0-9._-]{0,118}[A-Za-z0-9]$/;

/** The 32 raw bytes of an Ed25519 public key, in standard base64 (what the app compiles in). */
export function publicKeyBase64(key) {
  const jwk = crypto.createPublicKey(key).export({ format: "jwk" });
  return Buffer.from(jwk.x, "base64url").toString("base64");
}

/** The signed envelope around `payload` (an object), as the app reads it. */
export function envelope(payload, privateKey, keyId = "openframe") {
  const bytes = Buffer.from(JSON.stringify(payload));
  return JSON.stringify({
    format: ENVELOPE_FORMAT,
    keyId,
    payload: bytes.toString("base64"),
    signature: crypto.sign(null, bytes, privateKey).toString("base64"),
  });
}

function linkOrCopy(src, dst) {
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.rmSync(dst, { force: true });
  try {
    fs.linkSync(src, dst);
  } catch {
    fs.copyFileSync(src, dst);
  }
}

/** The file name a model has on the host and in the app's models folder. */
export const modelFileName = (m) => `${m.modelId}.gguf`;

/**
 * Lay out the host folder `out` for the lock's models and sign its list with `privateKey`.
 * Returns { version, files: [{ name, path, bytes, sha256, role }] }.
 */
export function publish({ privateKey, out, version, source, keyId, sequence } = {}) {
  const lock = readLock();
  const v = version ?? lock.bundleVersion;
  if (!SAFE.test(v) || ["staging", "quarantine", "active.json", "set.json"].includes(v.toLowerCase())) throw new AiAssetsError(`The version ${v} is not a safe folder name.`);
  if (!Number.isSafeInteger(sequence) || sequence < 1 || sequence > 4294967295) throw new AiAssetsError("--sequence must be a whole number from 1, higher than every list signed before.");
  const src = findSource(source);
  if (!src) throw new AiAssetsError("The model files are not in .ai-assets (see packaging/ai/ai-assets.lock.json).");
  const pinned = lock.models.map((m) => ({ rel: `models/${m.modelId}/${m.file.path}`, bytes: m.file.bytes, sha256: m.file.sha256 }));
  const problems = verifyFiles(src, pinned);
  if (problems.length) throw new AiAssetsError(`The model files do not match the lock:\n  - ${problems.join("\n  - ")}`);
  const files = lock.models.map((m) => {
    const name = modelFileName(m);
    linkOrCopy(path.join(src, "models", m.modelId, m.file.path), path.join(out, v, name));
    return { name, path: `${v}/${name}`, bytes: m.file.bytes, sha256: m.file.sha256, role: m.role };
  });
  const cpu = lock.runtimes.find((r) => r.backend === "cpu");
  const payload = {
    manifestVersion: v,
    sequence,
    profileVersion: PROFILE_VERSION,
    tiers: [{ id: "small", label: "Standard", minMemoryGb: 8, requiresRuntime: cpu.version, files }],
    runtimes: lock.runtimes.map((r) => ({ id: r.runtimeId, version: r.version, os: r.os, arch: r.arch, backend: r.backend })),
  };
  fs.writeFileSync(path.join(out, MANIFEST_NAME), envelope(payload, privateKey, keyId));
  return { version: v, files, envelope: fs.readFileSync(path.join(out, MANIFEST_NAME)) };
}

/**
 * The test suite's model host folder with a throwaway key (made once, kept in the git-ignored build
 * cache so a reused build keeps working). Returns { dir, publicKey, privateKey, version, files }.
 */
export function testHost() {
  const keyFile = path.join(TEST_HOST_DIR, "test-key.pem");
  fs.mkdirSync(TEST_HOST_DIR, { recursive: true });
  if (!fs.existsSync(keyFile)) {
    const { privateKey } = crypto.generateKeyPairSync("ed25519");
    fs.writeFileSync(keyFile, privateKey.export({ format: "pem", type: "pkcs8" }));
  }
  const privateKey = crypto.createPrivateKey(fs.readFileSync(keyFile));
  const r = publish({ privateKey, out: TEST_HOST_DIR, keyId: "e2e-test", sequence: 1 });
  return { dir: TEST_HOST_DIR, publicKey: publicKeyBase64(privateKey), privateKey, ...r };
}

function parseArgs(argv) {
  const [cmd, ...rest] = argv;
  const a = { cmd };
  for (let i = 0; i < rest.length; i++) {
    const k = rest[i];
    if (["--out", "--key", "--version", "--source", "--sequence"].includes(k)) a[k.slice(2)] = rest[++i];
    else throw new AiAssetsError(`unknown argument ${k} (see the header of scripts/model-manifest.mjs)`);
  }
  return a;
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) {
  try {
    const a = parseArgs(process.argv.slice(2));
    if (a.cmd === "new-key") {
      if (!a.out) throw new AiAssetsError("new-key needs --out <private-key.pem>");
      if (path.resolve(a.out).startsWith(ROOT + path.sep)) throw new AiAssetsError("Keep the private key outside this repository.");
      if (fs.existsSync(a.out)) throw new AiAssetsError(`${a.out} exists; a key is never overwritten.`);
      const { privateKey } = crypto.generateKeyPairSync("ed25519");
      fs.writeFileSync(a.out, privateKey.export({ format: "pem", type: "pkcs8" }), { mode: 0o600 });
      console.log(`Private key written to ${a.out}. Keep it offline.\nOPENFRAME_MANIFEST_PUBLIC_KEY=${publicKeyBase64(privateKey)}`);
    } else if (a.cmd === "publish") {
      if (!a.key || !a.out || !a.sequence) throw new AiAssetsError("publish needs --key <private-key.pem>, --out <host folder> and --sequence <n>");
      const r = publish({ privateKey: crypto.createPrivateKey(fs.readFileSync(a.key)), out: path.resolve(a.out), version: a.version, source: a.source, sequence: Number(a.sequence) });
      console.log(`Signed list for version ${r.version} with ${r.files.length} files written to ${path.join(a.out, MANIFEST_NAME)}.`);
    } else {
      throw new AiAssetsError("Use new-key or publish (see the header of scripts/model-manifest.mjs).");
    }
  } catch (e) {
    console.error(`\n${e instanceof AiAssetsError ? e.message : (e.stack ?? e)}`);
    process.exit(1);
  }
}
