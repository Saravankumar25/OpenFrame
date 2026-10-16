// The local model host of the test suite (ADR-0026 decision 10): serves the folder made by
// scripts/model-manifest.mjs testHost() on 127.0.0.1:MODEL_HOST_PORT, the address every test build
// is compiled with. It honours `Range: bytes=N-` like a real host. Specs steer it through a small
// control file (E2E_MODEL_HOST_CTL) and read what was asked for in its log (E2E_MODEL_HOST_LOG):
//
//   { "cut": { "path": "<rel>", "after": <bytes> } }   the next answer for that file stops early (once)
//   { "corrupt": "<rel>" }                              that file is served with one byte changed
//   { "throttle": <bytes per second> }                  answers are sent slowly (to press Pause)
//   { "down": true }                                    every request is refused (no network)

import http from "node:http";
import { appendFileSync, renameSync, existsSync, linkSync, mkdirSync, readFileSync, statSync, writeFileSync, copyFileSync, openSync, readSync, closeSync } from "node:fs";
import { dirname, join, normalize, sep } from "node:path";

export const MODEL_HOST_PORT = 47811;
export const MODEL_HOST_URL = `http://127.0.0.1:${MODEL_HOST_PORT}/`;

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
  server.listen(MODEL_HOST_PORT, "127.0.0.1");
  return { close: () => new Promise((r) => server.close(() => r())) };
}

/**
 * Install the host's model set in an app data folder the way a finished download leaves it:
 * <appData>/models/<version>/<files> (hard links, never written by the app), set.json (the signed
 * list) and active.json. Specs that alter a model must copy it first (see lib/ai.mjs).
 */
export function preinstallModels(hostDir, appDataDir) {
  const envelope = readFileSync(join(hostDir, "manifest.json"));
  const payload = JSON.parse(Buffer.from(JSON.parse(envelope).payload, "base64").toString("utf8"));
  const version = payload.manifestVersion;
  const set = join(appDataDir, "models", version);
  mkdirSync(set, { recursive: true });
  for (const f of payload.tiers[0].files) {
    const dst = join(set, f.name);
    if (existsSync(dst)) continue;
    try {
      linkSync(join(hostDir, f.path), dst);
    } catch {
      copyFileSync(join(hostDir, f.path), dst);
    }
  }
  writeFileSync(join(set, "set.json"), envelope);
  mkdirSync(dirname(join(appDataDir, "models", "active.json")), { recursive: true });
  writeFileSync(join(appDataDir, "models", "active.json"), JSON.stringify({ version, tier: "small", previous: null, activatedAt: 0 }));
  return { version, files: payload.tiers[0].files };
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
