// Desktop E2E runner (`npm run test:e2e`).
//
//   1. installs msedgedriver (matching WebView2) + tauri-driver into .drivers/ (gitignored);
//   2. builds the real app (debug, embedded UI) unless --no-build;
//   3. runs specs/*.test.mjs in file-name order with node:test, one tauri-driver per data root.
//
// Data roots (OPENFRAME_DATA_ROOT, honoured by debug builds only) live under tests/e2e/.run/<stamp>/:
//
//   * shared/   the journey data root. 01-journey seeds the demo project "Night Bus"; 02-restart,
//               90-visual-baseline and 95-reachability reopen it, in that order. These specs depend
//               on each other only through this root and its order.
//   * <spec>/   a fresh, empty data root for every spec whose header contains the line
//               `// @e2e-data-root: own` (the per-workspace specs 03..89). Such a spec creates its own
//               project and never sees, or changes, the shared root, so the visual baseline (90) is
//               not disturbed by them and any of them can run alone (`--only=<substring>`).
//
// An own-root spec whose header also contains `// @e2e-fake-media` runs with WebView2's fake
// microphone and camera (no device, no permission prompt), so a voice note can be recorded. The
// flags reach the webview through the session capabilities (E2E_WEBVIEW_ARGS, see lib/app.mjs);
// no other spec gets them.
//
// Every spec runs with deterministic Offline AI sampling (OPENFRAME_AI_DETERMINISTIC=1: temperature 0
// and a fixed seed; honoured by debug builds on an isolated data root only).
//
// OpenFrame AI is downloaded after install (ADR-0026). The run starts the local model host
// (lib/model-host.mjs) that every test build is compiled with, and each data root starts with the
// model set installed the way a finished download leaves it, so the AI specs find OpenFrame AI
// ready. A spec whose header contains `// @e2e-ai-models: none` starts with nothing downloaded (the
// download specs). Each data root gets its own host control file and request log
// (E2E_MODEL_HOST_CTL, E2E_MODEL_HOST_LOG).
//
// Options: --no-build (reuse the last build), --only=<substring>[,<substring>...] (filter specs),
// --release=internal | --release=store (drive the release executable of the internal team build or
// of the store channel; see scripts/build-app.mjs. Specs marked `// @e2e-debug-only: <reason>` are
// listed and skipped). --app=<installed openframe-desktop.exe> (needs --release=internal): drive an
// installed build, never build, still preinstall the models in each data root, and skip specs marked
// `// @e2e-needs-test-host: <reason>`.
// Failure screenshots and visual diffs go to tests/e2e/artifacts/ (gitignored).

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import net from "node:net";
import { DRIVERS, E2E_ROOT, EDGE_DRIVER, TAURI_DRIVER, setupDrivers } from "./scripts/setup-drivers.mjs";
import { appBinary, buildApp } from "./scripts/build-app.mjs";
import { preinstallModels, startModelHost, testHost } from "./lib/model-host.mjs";

const args = new Set(process.argv.slice(2));
// --release=internal | --release=store: drive the release executable (build-app.mjs explains both).
const RELEASE = [...args].find((a) => a.startsWith("--release="))?.slice("--release=".length) ?? false;
if (RELEASE && !["internal", "store"].includes(RELEASE)) throw new Error(`Unknown --release=${RELEASE}; use internal or store.`);
// --app=<path to an installed openframe-desktop.exe>: gate G2, "tested as shipped". Never builds.
const APP_ARG = [...args].find((a) => a.startsWith("--app="))?.slice("--app=".length);
if (APP_ARG && RELEASE !== "internal") throw new Error("--app needs --release=internal (the installed build is the internal team build).");
if (APP_ARG && !existsSync(APP_ARG)) throw new Error(`--app: ${APP_ARG} does not exist.`);
const APP_BINARY = APP_ARG ? resolve(APP_ARG) : appBinary(Boolean(RELEASE));
// Specs whose header has `// @e2e-debug-only: <reason>` need a switch the release build refuses
// (ADR-0031); a release run lists them with the reason and skips them.
const DEBUG_ONLY = /^\/\/ @e2e-debug-only: (.+)$/m;
// Specs whose header has `// @e2e-needs-test-host: <reason>` need the local model host to serve
// downloads or need its request log; an installed build points at Hugging Face, so --app skips them.
const NEEDS_HOST = /^\/\/ @e2e-needs-test-host: (.+)$/m;
// E2E_PORT_BASE lets two runs share the machine (the WindowsHTML capture sections are written in
// parallel); each run then uses its own pair of driver ports.
const PORT = Number(process.env.E2E_PORT_BASE ?? 4444);
const NATIVE_PORT = PORT + 1;
// The local model host folder and the current group's control file and log.
const HOST = { dir: "", ctl: "", log: "" };
const OWN_ROOT = /^\/\/ @e2e-data-root: own\s*$/m;
const FAKE_MEDIA = /^\/\/ @e2e-fake-media\s*$/m;
const NO_MODELS = /^\/\/ @e2e-ai-models: none\s*$/m;
// An own-root spec whose header contains `// @e2e-net-log` runs with a Chromium net log of the web
// view written to <root>\webview-netlog.json (E2E_NET_LOG), so it can list every host the web view's
// own network code asked for (spec 05).
const NET_LOG = /^\/\/ @e2e-net-log\s*$/m;
// Chromium's fake media device and auto-accepted prompt, added to the webview's own arguments.
const FAKE_MEDIA_ARGS = "--use-fake-device-for-media-stream --use-fake-ui-for-media-stream";

/** Is anything listening on `port` on loopback? msedgedriver listens on IPv4 and IPv6, so both count. */
async function portOpen(port) {
  return (await portOpenOn(port, "127.0.0.1")) || (await portOpenOn(port, "::1"));
}

function portOpenOn(port, host) {
  return new Promise((resolve) => {
    const s = net.connect(port, host);
    s.once("connect", () => {
      s.destroy();
      resolve(true);
    });
    s.once("error", () => {
      s.destroy();
      resolve(false);
    });
  });
}

/** Can a new listener bind `port` on loopback? A port nobody listens on can still refuse a bind
 * for a while after the previous driver stopped, and msedgedriver then fails to start. */
function portBindable(port) {
  return new Promise((resolve) => {
    const srv = net.createServer();
    srv.once("error", () => resolve(false));
    srv.listen({ port, host: "127.0.0.1", exclusive: true }, () => srv.close(() => resolve(true)));
  });
}

/** Wait until `port` is listened on (open = true) or free for the next driver (open = false). */
async function waitForPort(port, open, timeoutMs) {
  const until = Date.now() + timeoutMs;
  const ready = async () => (open ? portOpen(port) : !(await portOpen(port)) && (await portBindable(port)));
  while (!(await ready())) {
    if (Date.now() > until) return false;
    await new Promise((r) => setTimeout(r, 250));
  }
  return true;
}

/** Start tauri-driver for one data root, run the given spec files, stop the driver. */
async function runGroup(files, dataRoot, fakeMedia = false, noModels = false, netLog = false) {
  mkdirSync(dataRoot, { recursive: true });
  if (!noModels) preinstallModels(HOST.dir, join(dataRoot, "appdata"));
  HOST.ctl = join(dataRoot, "model-host-ctl.json");
  HOST.log = join(dataRoot, "model-host-log.jsonl");
  writeFileSync(HOST.ctl, "{}");
  writeFileSync(HOST.log, "");
  console.log(`\nE2E data root: ${dataRoot}\n  specs: ${files.join(", ")}${fakeMedia ? " (fake media devices)" : ""}`);
  const env = {
    ...process.env,
    OPENFRAME_DATA_ROOT: dataRoot,
    OPENFRAME_AI_DETERMINISTIC: "1",
    E2E_APP: APP_BINARY,
    E2E_DATA_ROOT: dataRoot,
    E2E_PORT: String(PORT),
    E2E_MODEL_HOST_DIR: HOST.dir,
    E2E_MODEL_HOST_CTL: HOST.ctl,
    E2E_MODEL_HOST_LOG: HOST.log,
  };
  delete env.E2E_WEBVIEW_ARGS;
  if (RELEASE) env.E2E_RELEASE = RELEASE;
  delete env.E2E_NET_LOG;
  if (fakeMedia) env.E2E_WEBVIEW_ARGS = FAKE_MEDIA_ARGS;
  if (netLog) {
    env.E2E_NET_LOG = join(dataRoot, "webview-netlog.json");
    env.E2E_WEBVIEW_ARGS = [env.E2E_WEBVIEW_ARGS, `--log-net-log=${env.E2E_NET_LOG}`].filter(Boolean).join(" ");
  }
  if (!(await waitForPort(PORT, false, 15_000)) || !(await waitForPort(NATIVE_PORT, false, 15_000))) {
    throw new Error(`Ports ${PORT}/${NATIVE_PORT} are in use by another process; the desktop E2E suite needs them exclusively.`);
  }
  const driver = spawn(TAURI_DRIVER, ["--port", String(PORT), "--native-port", String(NATIVE_PORT), "--native-driver", EDGE_DRIVER], {
    env,
    stdio: ["ignore", "inherit", "inherit"],
    cwd: DRIVERS,
  });
  try {
    if (!(await waitForPort(PORT, true, 20_000))) throw new Error(`tauri-driver did not start on port ${PORT}.`);
    return await new Promise((resolve) => {
      const p = spawn(process.execPath, ["--test", "--test-concurrency=1", "--test-reporter=spec", ...files.map((f) => join("specs", f))], {
        cwd: E2E_ROOT,
        env,
        stdio: "inherit",
      });
      p.on("exit", (c) => resolve(c ?? 1));
    });
  } finally {
    // tauri-driver starts msedgedriver as its child. Stop the whole tree while tauri-driver is
    // still alive (killing it first would leave msedgedriver holding the native port), then wait
    // until both ports are free for the next group.
    spawnSync("taskkill", ["/PID", String(driver.pid), "/T", "/F"], { stdio: "ignore" });
    driver.kill();
    if (!(await waitForPort(PORT, false, 10_000)) || !(await waitForPort(NATIVE_PORT, false, 10_000))) {
      console.warn(`Ports ${PORT}/${NATIVE_PORT} are still in use after the driver stopped.`);
    }
  }
}

async function main() {
  await setupDrivers();
  if (!APP_ARG && (!args.has("--no-build") || !existsSync(APP_BINARY))) buildApp({ release: RELEASE });
  if (RELEASE === "internal" && !APP_ARG) {
    // The internal release executable carries no secret, personal path or user name (ADR-0030 decision 6).
    const scan = spawnSync(process.execPath, [join(E2E_ROOT, "..", "..", "scripts", "scan-installer.mjs"), APP_BINARY], { stdio: "inherit" });
    if (scan.status !== 0) throw new Error("The internal release executable failed the string scan.");
  }

  // The local model host for the whole run; each group points it at its own control file and log.
  Object.assign(HOST, testHost());
  const server = startModelHost(HOST.dir, () => HOST.ctl, () => HOST.log);
  const stamp = `${new Date().toISOString().replace(/[:.]/g, "-")}-${process.pid}`;
  const runRoot = join(E2E_ROOT, ".run", stamp);
  mkdirSync(join(E2E_ROOT, "artifacts"), { recursive: true });

  const only = [...args].find((a) => a.startsWith("--only="))?.slice("--only=".length);
  const specs = readdirSync(join(E2E_ROOT, "specs"))
    .filter((f) => f.endsWith(".test.mjs") && (!only || only.split(",").some((o) => f.includes(o))))
    .sort();
  // Consecutive shared-root specs run together; every own-root spec runs alone in its own root.
  const groups = [];
  const skipped = [];
  for (const f of specs) {
    const head = readFileSync(join(E2E_ROOT, "specs", f), "utf8").slice(0, 4000);
    const debugOnly = head.match(DEBUG_ONLY);
    if (RELEASE && debugOnly) {
      skipped.push(`${f}: ${debugOnly[1]}`);
      continue;
    }
    const needsHost = head.match(NEEDS_HOST);
    if (APP_ARG && needsHost) {
      skipped.push(`${f}: needs the test model host: ${needsHost[1]}`);
      continue;
    }
    const own = OWN_ROOT.test(head);
    const last = groups.at(-1);
    if (!own && last && !last.own) last.files.push(f);
    else
      groups.push({
        own,
        fakeMedia: own && FAKE_MEDIA.test(head),
        noModels: own && NO_MODELS.test(head),
        netLog: own && NET_LOG.test(head),
        files: [f],
        root: own ? join(runRoot, f.replace(/\.test\.mjs$/, "")) : join(runRoot, "shared"),
      });
  }
  const failed = [];
  for (const g of groups) {
    const code = await runGroup(g.files, g.root, g.fakeMedia, g.noModels, g.netLog);
    if (code !== 0) failed.push(...g.files);
  }
  await server.close();
  const ran = specs.length - skipped.length;
  if (skipped.length) console.log(`\nSkipped on the release build (debug-only switch or test model host):\n  ${skipped.join("\n  ")}`);
  console.log(failed.length ? `\nE2E FAILED: ${failed.join(", ")}` : `\nE2E passed: ${ran} spec files${RELEASE ? ` on the release build (${RELEASE})` : ""}.`);
  process.exit(failed.length ? 1 : 0);
}

main().catch((e) => {
  console.error(e instanceof Error ? e.stack : e);
  process.exit(1);
});
