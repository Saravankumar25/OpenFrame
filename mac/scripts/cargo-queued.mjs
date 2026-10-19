// Runs one cargo command through a machine-wide build queue.
//
// The development machine has 16 GB of RAM. Several worktrees building the workspace at once ran it
// out of memory (five builds, 52 cargo and rustc processes, 0.4 GB free), so builds swapped and took
// hours. Agents may edit code in parallel, but every memory-heavy cargo command goes through this
// script: it takes one lock shared by all worktrees, runs cargo with a capped job count, and releases
// the lock when cargo ends, on Ctrl+C, or when the holder dies (a stale lock is recovered).
//
//   node C:/OpenFrame/scripts/cargo-queued.mjs <cargo args...>
//   e.g. node C:/OpenFrame/scripts/cargo-queued.mjs test -p openframe-application --test production schedule_depth::
//
// OPENFRAME_CARGO_JOBS sets the job count (default 4); RUST_TEST_THREADS defaults to 4.
// docs/development-storage-policy.md describes the rules.

import { spawn } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";

const LOCK_DIR =
  process.env.OPENFRAME_CARGO_LOCK ??
  "C:/OpenFrame/.build-cache/cargo-queue.lock";
const OWNER = join(LOCK_DIR, "owner.json");
const POLL_MS = 2000;
const args = process.argv.slice(2);

if (args.length === 0) {
  console.error("Usage: node scripts/cargo-queued.mjs <cargo args...>");
  process.exit(2);
}

function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return e.code === "EPERM";
  }
}

function readOwner() {
  try {
    return JSON.parse(readFileSync(OWNER, "utf8"));
  } catch {
    return null;
  }
}

/** True when the lock exists but its holder is gone, so it can be taken over. */
function stale() {
  const owner = readOwner();
  if (owner) return !alive(owner.pid);
  // A lock folder without an owner file: the holder died between mkdir and write. Give a fresh one
  // a moment before treating it as abandoned.
  try {
    return Date.now() - statSync(LOCK_DIR).mtimeMs > 30_000;
  } catch {
    return false;
  }
}

function tryAcquire() {
  try {
    mkdirSync(LOCK_DIR);
  } catch (e) {
    if (e.code !== "EEXIST") throw e;
    if (!stale()) return false;
    console.error(
      `cargo-queue: recovering a stale lock left by ${JSON.stringify(readOwner())}`,
    );
    rmSync(LOCK_DIR, { recursive: true, force: true });
    try {
      mkdirSync(LOCK_DIR);
    } catch {
      return false;
    }
  }
  writeFileSync(
    OWNER,
    JSON.stringify({
      pid: process.pid,
      cwd: process.cwd(),
      args,
      started: new Date().toISOString(),
    }),
  );
  return true;
}

let held = false;
function release() {
  if (!held) return;
  held = false;
  const owner = readOwner();
  if (!owner || owner.pid === process.pid)
    rmSync(LOCK_DIR, { recursive: true, force: true });
}

async function acquire() {
  let lastNote = 0;
  const t0 = Date.now();
  while (!tryAcquire()) {
    if (Date.now() - lastNote > 60_000) {
      const o = readOwner();
      console.error(
        `cargo-queue: waiting for ${o ? `pid ${o.pid} (${o.cwd}: cargo ${o.args.join(" ")})` : "another build"}`,
      );
      lastNote = Date.now();
    }
    await new Promise((r) => setTimeout(r, POLL_MS));
  }
  held = true;
  const waited = Math.round((Date.now() - t0) / 1000);
  if (waited > 0) console.error(`cargo-queue: lock taken after ${waited}s`);
}

await acquire();

// sccache puts every CARGO_* environment variable into its cache key, so a per-worktree
// CARGO_TARGET_DIR or a different CARGO_BUILD_JOBS made every worktree miss the cache (measured
// 2026-10-08: 0 of 30 hits with the variable, 22 of 30 with the flag). Pass both as flags instead.
const BUILD_LIKE = new Set([
  "build",
  "b",
  "check",
  "c",
  "clippy",
  "test",
  "t",
  "run",
  "r",
  "bench",
  "doc",
  "d",
  "rustc",
]);
const env = { ...process.env };
const targetDir = env.CARGO_TARGET_DIR;
delete env.CARGO_TARGET_DIR;
delete env.CARGO_BUILD_JOBS;
env.RUST_TEST_THREADS ??= "4";
env.CARGO_INCREMENTAL ??= "0";
const jobs = process.env.OPENFRAME_CARGO_JOBS ?? "4";
const [sub, ...rest] = args;
const cargoArgs = BUILD_LIKE.has(sub)
  ? [
      sub,
      ...(targetDir && !rest.some((a) => a.startsWith("--target-dir"))
        ? ["--target-dir", targetDir]
        : []),
      ...(rest.some((a) => a === "-j" || a.startsWith("--jobs"))
        ? []
        : ["-j", jobs]),
      ...rest,
    ]
  : args;
if (!BUILD_LIKE.has(sub) && targetDir) env.CARGO_TARGET_DIR = targetDir;

const child = spawn("cargo", cargoArgs, {
  stdio: "inherit",
  env,
  shell: false,
});
for (const sig of ["SIGINT", "SIGTERM", "SIGBREAK", "SIGHUP"]) {
  process.on(sig, () => {
    child.kill();
    release();
    process.exit(130);
  });
}
process.on("exit", release);
child.on("error", (e) => {
  console.error(`cargo-queue: could not start cargo: ${e.message}`);
  release();
  process.exit(1);
});
child.on("exit", (code, signal) => {
  release();
  process.exit(code ?? (signal ? 1 : 0));
});
