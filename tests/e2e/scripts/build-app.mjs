// Builds the real desktop app for the E2E suite: a debug build with the web UI
// embedded (no dev server), i.e. `tauri build --debug --no-bundle`.
// Debug builds honour OPENFRAME_DATA_ROOT, which the suite uses to run fully
// isolated from the user's own projects and settings.

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, "..", "..", "..");
export const APP_BINARY = join(REPO_ROOT, "target", "debug", "openframe-desktop.exe");

export function buildApp() {
  console.log("Building OpenFrame Studio (debug, embedded UI)…");
  // npm is a .cmd shim on Windows, so it runs through the shell (fixed command, no user input).
  const r = spawnSync("npm --workspace apps/desktop run tauri -- build --debug --no-bundle", {
    cwd: REPO_ROOT,
    stdio: "inherit",
    shell: true,
    env: { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "2" },
  });
  if (r.status !== 0 || !existsSync(APP_BINARY)) throw new Error("Building the desktop app failed.");
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
