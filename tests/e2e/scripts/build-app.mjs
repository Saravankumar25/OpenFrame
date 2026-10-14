// Builds the real desktop app for the E2E suite: a debug build with the web UI
// embedded (no dev server), i.e. `tauri build --debug --no-bundle`.
// Debug builds honour OPENFRAME_DATA_ROOT, which the suite uses to run fully
// isolated from the user's own projects and settings.
//
// The build carries the complete bundled Offline AI (ADR-0025) exactly like a package does:
// scripts/prepare-ai-assets.mjs verifies the pinned runtime and model files against
// packaging/ai/ai-assets.lock.json and stages them, and the Tauri overlay config copies them to
// `ai/` next to the executable. A missing or altered file stops the build (fail closed), so the
// AI specs always run against the real, verified model.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { prepare } from "../../../scripts/prepare-ai-assets.mjs";

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, "..", "..", "..");
// Honour CARGO_TARGET_DIR (parallel worktrees build into their own target folders).
const TARGET_DIR = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(REPO_ROOT, "target");
export const APP_BINARY = join(TARGET_DIR, "debug", "openframe-desktop.exe");
/** The bundled Offline AI next to the test build (the app's resource folder). */
export const APP_AI_DIR = join(TARGET_DIR, "debug", "ai");

export function buildApp() {
  prepare();
  console.log("Building OpenFrame Studio (debug, embedded UI, bundled Offline AI)…");
  // The suite drives the app without the team password screen of the internal builds (ADR-0024),
  // so the test build never sees the build machine's team password, even when it is set there.
  const env = { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "2" };
  delete env.OPENFRAME_TEAM_PASSWORD;
  // npm is a .cmd shim on Windows, so it runs through the shell (fixed command, no user input).
  const r = spawnSync("npm --workspace apps/desktop run tauri -- build --debug --no-bundle --config src-tauri/tauri.bundled-ai.conf.json", {
    cwd: REPO_ROOT,
    stdio: "inherit",
    shell: true,
    env,
  });
  if (r.status !== 0 || !existsSync(APP_BINARY)) throw new Error("Building the desktop app failed.");
  if (!existsSync(join(APP_AI_DIR, "models"))) throw new Error(`The test build has no bundled Offline AI in ${APP_AI_DIR}.`);
  return APP_BINARY;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    buildApp();
  } catch (e) {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  }
}
