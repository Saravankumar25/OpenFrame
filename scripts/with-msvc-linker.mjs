// Runs a release-grade build command with the Microsoft linker and the release hardening flags.
//
// `.cargo/config.toml` links dev and test builds with rust-lld for speed. Release builds (installer,
// MSIX, `npm run perf`) must keep the Microsoft linker so the binary hardening flags of the security
// standard (masterplan area 6, "Windows and Mac security standard") apply to what ships. Cargo lets an
// environment variable override the per-target linker from the config file, so this wrapper sets it
// and runs the given command unchanged.
//
// Windows phase W3 adds, for these builds only (dev builds stay fast and unchanged):
//   - control-flow guard (`-C control-flow-guard`);
//   - no personal paths in panic text or file!() strings: the repository, the Cargo home and the
//     Rust toolchain folders are remapped to short neutral names (`--remap-path-prefix`).
// Symbols are stripped by the release profile in Cargo.toml (`strip = "symbols"`). Phase W4 checks
// the result on the real release build (scripts/scan-installer.mjs and the binary flags).
//
//   node scripts/with-msvc-linker.mjs <command> [args...]

import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import url from "node:url";

const REPO = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");

/** The environment for a hardened release build, from the current one. */
export function hardenedEnv(env, { platform = process.platform, repo = REPO, home = os.homedir() } = {}) {
  const out = { ...env };
  if (platform !== "win32") return out;
  out.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = "link.exe";
  const cargoHome = env.CARGO_HOME || path.join(home, ".cargo");
  const rustupHome = env.RUSTUP_HOME || path.join(home, ".rustup");
  const flags = [
    ["-C", "control-flow-guard"],
    [`--remap-path-prefix=${repo}=openframe`],
    [`--remap-path-prefix=${cargoHome}=cargo`],
    [`--remap-path-prefix=${rustupHome}=rustup`],
  ].flat();
  // Space-separated RUSTFLAGS cannot carry paths with spaces; the encoded form can (0x1f between flags).
  const existing = env.CARGO_ENCODED_RUSTFLAGS
    ? env.CARGO_ENCODED_RUSTFLAGS.split("\x1f")
    : (env.RUSTFLAGS ?? "").split(/\s+/).filter(Boolean);
  out.CARGO_ENCODED_RUSTFLAGS = [...existing, ...flags].join("\x1f");
  delete out.RUSTFLAGS;
  return out;
}

if (import.meta.url === url.pathToFileURL(process.argv[1] ?? "").href) {
  const [cmd, ...args] = process.argv.slice(2);
  if (!cmd) {
    console.error("Usage: node scripts/with-msvc-linker.mjs <command> [args...]");
    process.exit(2);
  }
  const r = spawnSync(cmd, args, { stdio: "inherit", env: hardenedEnv(process.env), shell: true });
  if (r.error) {
    console.error(`Could not start ${cmd}: ${r.error.message}`);
    process.exit(1);
  }
  process.exit(r.status ?? 1);
}
