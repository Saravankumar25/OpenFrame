// The local model host of the test suite (ADR-0032 decision 4): serves the pinned model files at
// their Hugging Face paths (<org>/<repo>/resolve/<commit>/<file>, from testHost()) on
// 127.0.0.1:MODEL_HOST_PORT, the address every test build is compiled with
// (OPENFRAME_TEST_MODEL_SOURCE). It honours `Range: bytes=N-` like a real host. Specs steer it through a small
// control file (E2E_MODEL_HOST_CTL) and read what was asked for in its log (E2E_MODEL_HOST_LOG):
//
//   { "cut": { "path": "<rel>", "after": <bytes> } }   the next answer for that file stops early (once)
//   { "corrupt": "<rel>" }                              that file is served with one byte changed
//   { "throttle": <bytes per second> }                  answers are sent slowly (to press Pause)
//   { "down": true }                                    every request is refused (no network)

import { createHash } from "node:crypto";
import http from "node:http";
import { appendFileSync, renameSync, existsSync, linkSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync, copyFileSync, openSync, readSync, closeSync } from "node:fs";
import { dirname, join, normalize, sep } from "node:path";
import { AiAssetsError, ROOT, findSource, readLock, verifyFiles } from "../../../scripts/prepare-ai-assets.mjs";

export const MODEL_HOST_PORT = 47811;
export const MODEL_HOST_URL = `http://127.0.0.1:${MODEL_HOST_PORT}/`;
export const TEST_HOST_DIR = join(ROOT, ".build-cache", "e2e-model-host");

/**
 * The set the app pins, computed from the lock the way the app does
 * (crates/openframe-ai/src/download.rs pins): the version "m-" and the first 12 hex digits of the
 * SHA-256 over the model hashes, and each file's name in app data, host path, size, hash and role.
 */
export function pins() {
  const lock = readLock();
  const version = `m-${createHash("sha256").update(lock.models.map((m) => m.file.sha256).join("")).digest("hex").slice(0, 12)}`;
  const files = lock.models.map((m) => {
    const path = m.source.url.replace(/^https:\/\/huggingface\.co\//, "");
    if (path === m.source.url) throw new AiAssetsError(`model ${m.modelId} has no Hugging Face address`);
    return { name: `${m.modelId}.gguf`, path, bytes: m.file.bytes, sha256: m.file.sha256, role: m.role };
  });
  return { version, files };
}

/**
 * Lay out the host folder: each pinned model at its Hugging Face path, hard-linked from the build
 * inputs after they are checked against the lock (copied when the drive cannot link). The folder is
 * made again each time, so files of an older lock never linger. Returns { dir, version, files }.
 */
export function testHost() {
  const lock = readLock();
  const src = findSource();
  if (!src) throw new AiAssetsError("The model files are not in .ai-assets (see packaging/ai/ai-assets.lock.json).");
  const problems = verifyFiles(src, lock.models.map((m) => ({ rel: `models/${m.modelId}/${m.file.path}`, bytes: m.file.bytes, sha256: m.file.sha256 })));
  if (problems.length) throw new AiAssetsError(`The model files do not match the lock:\n  - ${problems.join("\n  - ")}`);
  rmSync(TEST_HOST_DIR, { recursive: true, force: true, maxRetries: 3 });
  const p = pins();
  lock.models.forEach((m, i) => {
    const from = join(src, "models", m.modelId, m.file.path);
    const to = join(TEST_HOST_DIR, ...p.files[i].path.split("/"));
    mkdirSync(dirname(to), { recursive: true });
    try {
      linkSync(from, to);
    } catch {
      copyFileSync(from, to);
    }
  });
  return { dir: TEST_HOST_DIR, ...p };
}

/** The control file, written whole by steerHost (temp file and rename). A file that does not
 * parse is a broken test, never "normal": it fails loudly. */
const readCtl = (file) => {
  const text = readFileSync(file, "utf8");
  return text.trim() ? JSON.parse(text) : {};
};

/** Start the host on `dir`. `ctlAt` and `logAt` give the current control file and log. Returns { close() }. */
export function startModelHost(dir, ctlAt, logAt) {
  const server = http.createServer((req, res) => {
    const ctlFile = ctlAt();
    const logFile = logAt();
    const ctl = readCtl(ctlFile);
    let rel;
    try {
      rel = decodeURIComponent((req.url ?? "/").split("?")[0]).replace(/^\/+/, "");
    } catch {
      res.writeHead(400, { "Content-Length": 0, Connection: "close" }).end();
      return;
    }
    const range = /^bytes=(\d+)-$/.exec(req.headers.range ?? "");
    const from = range ? Number(range[1]) : null;
    appendFileSync(logFile, `${JSON.stringify({ path: rel, from, at: Date.now() })}\n`);
    const file = normalize(join(dir, rel));
    if (ctl.down || !file.startsWith(dir + sep) || !existsSync(file) || !statSync(file).isFile()) {
      res.writeHead(ctl.down ? 503 : 404, { "Content-Length": 0, Connection: "close" }).end();
      return;
    }
    const size = statSync(file).size;
    if (from !== null && from >= size) {
      // Like a real host: nothing left to send from there.
      res.writeHead(416, { "Content-Range": `bytes */${size}`, "Content-Length": 0, Connection: "close" }).end();
      return;
    }
    const start = Math.min(from ?? 0, size);
    const headers = { "Content-Length": size - start, Connection: "close" };
    if (from !== null) headers["Content-Range"] = `bytes ${start}-${size - 1}/${size}`;
    res.writeHead(from !== null ? 206 : 200, headers);
    const cut = ctl.cut?.path === rel ? ctl.cut.after : Infinity;
    if (cut !== Infinity) writeFileSync(ctlFile, JSON.stringify({ ...ctl, cut: undefined }));
    const fd = openSync(file, "r");
    let closed = false;
    const release = () => {
      if (!closed) {
        closed = true;
        closeSync(fd);
      }
    };
    const chunk = 1 << 20;
    let pos = start;
    let sent = 0;
    const pump = () => {
      // The client went away (a pause): stop, never read from a closed file.
      if (closed) return;
      while (pos < size) {
        if (sent >= cut) {
          release();
          res.socket?.end(); // the connection drops mid-file, after what was sent so far
          return;
        }
        const n = Math.min(chunk, size - pos, cut - sent);
        const buf = Buffer.alloc(n);
        readSync(fd, buf, 0, n, pos);
        // A corrupted copy: the middle byte of the file differs, the size does not.
        const mid = Math.floor(size / 2);
        if (ctl.corrupt === rel && mid >= pos && mid < pos + n) buf[mid - pos] ^= 0x5a;
        pos += n;
        sent += n;
        const more = res.write(buf);
        if (ctl.throttle) {
          setTimeout(pump, Math.max(1, (n / ctl.throttle) * 1000));
          return;
        }
        if (!more) {
          res.once("drain", pump);
          return;
        }
      }
      release();
      res.end();
    };
    res.on("close", release);
    pump();
  });
  // Two runs on one machine (E2E_PORT_BASE) share the first run's host: it serves the same pinned files.
  server.on("error", (e) => {
    if (e.code !== "EADDRINUSE" || !process.env.E2E_PORT_BASE) throw e;
    console.warn(`model host port ${MODEL_HOST_PORT} is in use; this run shares the host already running`);
  });
  server.listen(MODEL_HOST_PORT, "127.0.0.1");
  return { close: () => new Promise((r) => server.close(() => r())) };
}

/**
 * Install the pinned set in an app data folder the way a finished download leaves it:
 * <appData>/models/<version>/<model-id>.gguf (hard links from the host folder, never written by the
 * app) and active.json. Specs that alter a model must copy it first (see lib/ai.mjs).
 */
export function preinstallModels(hostDir, appDataDir) {
  const { version, files } = pins();
  const set = join(appDataDir, "models", version);
  mkdirSync(set, { recursive: true });
  for (const f of files) {
    const dst = join(set, f.name);
    rmSync(dst, { force: true }); // never trust a file left by an earlier run
    const src = join(hostDir, ...f.path.split("/"));
    try {
      linkSync(src, dst);
    } catch {
      copyFileSync(src, dst);
    }
  }
  writeFileSync(join(appDataDir, "models", "active.json"), JSON.stringify({ version, tier: "small", previous: null, activatedAt: 0 }));
  return { version, files };
}

/** What the host was asked for since the log was made: [{ path, from }]. */
export function hostRequests(logFile = process.env.E2E_MODEL_HOST_LOG) {
  if (!logFile || !existsSync(logFile)) return [];
  return readFileSync(logFile, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((l) => JSON.parse(l));
}

/** Steer the host (see the header); `{}` puts it back to normal. */
export function steerHost(ctl, ctlFile = process.env.E2E_MODEL_HOST_CTL) {
  writeFileSync(`${ctlFile}.tmp`, JSON.stringify(ctl));
  renameSync(`${ctlFile}.tmp`, ctlFile);
}
