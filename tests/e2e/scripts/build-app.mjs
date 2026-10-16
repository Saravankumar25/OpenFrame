// Builds the real desktop app for the E2E suite, with the web UI embedded (no dev server):
//
//   * debug (default): `tauri build --debug --no-bundle`, the build every spec can drive, with the
//     debug-only test switches (deterministic AI, own AI folder, hand-set licence state);
//   * release (`--release=gated` or `--release=open`, Windows phase W4, ADR-0030 proof level 3):
//     `tauri build --no-bundle`, the hardened executable that goes into the installer, built through
//     scripts/with-msvc-linker.mjs. Never an installer or bundle. "gated" is the internal team build
//     with the team password screen, made with a throwaway password that exists only in this run's
//     memory (never the real team password); "open" is the same executable built as the store
//     channel, which has no team password screen. Both honour OPENFRAME_DATA_ROOT for file
//     locations only (ADR-0031).
//
// Every build goes through the machine-wide build queue (scripts/cargo-queued.cmd as Tauri's cargo
// runner). The build carries the complete Offline AI exactly like today's package does:
// scripts/prepare-ai-assets.mjs verifies the pinned runtime and model files against
// packaging/ai/ai-assets.lock.json and stages them, and the Tauri overlay config copies them to
// `ai/` next to the executable. A missing or altered file stops the build (fail closed).

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { prepare } from "../../../scripts/prepare-ai-assets.mjs";

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, "..", "..", "..");
// Honour CARGO_TARGET_DIR (the shared build folder under .build-cache).
const TARGET_DIR = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(REPO_ROOT, "target");
const QUEUE_RUNNER = join(REPO_ROOT, "scripts", "cargo-queued.cmd");

/** The executable a build of this kind produces. */
export function appBinary(release = false) {
  return join(TARGET_DIR, release ? "release" : "debug", "openframe-desktop.exe");
}
/** The Offline AI next to the test build (the app's resource folder). */
export function appAiDir(release = false) {
  return join(TARGET_DIR, release ? "release" : "debug", "ai");
}
export const APP_BINARY = appBinary(false);
export const APP_AI_DIR = appAiDir(false);

/**
 * Build the app. `release` is false (debug), "gated" or "open"; `gatePassword` is the throwaway
 * password of a gated build.
 */
export function buildApp({ release = false, gatePassword } = {}) {
  prepare();
  console.log(`Building OpenFrame Studio (${release ? `release, ${release}` : "debug"}, embedded UI, Offline AI)…`);
  const env = { ...process.env };
  // The debug and open builds never see the build machine's team password, even when it is set.
  delete env.OPENFRAME_TEAM_PASSWORD;
  delete env.OPENFRAME_DISTRIBUTION;
  if (release === "gated") {
    if (!gatePassword) throw new Error("A gated release build needs its throwaway test password.");
    env.OPENFRAME_TEAM_PASSWORD = gatePassword; // secret-scan: allow (a random value made in run.mjs for this run only)
  } else if (release === "open") {
    env.OPENFRAME_DISTRIBUTION = "store";
  }
  const tauri = `npm --workspace apps/desktop run tauri -- build ${release ? "" : "--debug "}--no-bundle --runner "${QUEUE_RUNNER}" --config src-tauri/tauri.bundled-ai.conf.json`;
  // npm is a .cmd shim on Windows, so it runs through the shell (fixed command, no user input).
  const command = release ? `node scripts/with-msvc-linker.mjs ${tauri}` : tauri;
  const r = spawnSync(command, { cwd: REPO_ROOT, stdio: "inherit", shell: true, env });
  const binary = appBinary(Boolean(release));
  if (r.status !== 0 || !existsSync(binary)) throw new Error("Building the desktop app failed.");
  if (!existsSync(join(appAiDir(Boolean(release)), "models"))) throw new Error(`The test build has no Offline AI in ${appAiDir(Boolean(release))}.`);
  return binary;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    buildApp();
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  }
}
