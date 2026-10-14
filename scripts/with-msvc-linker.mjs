// Runs a release-grade build command with the Microsoft linker.
//
// `.cargo/config.toml` links dev and test builds with rust-lld for speed. Release builds (installer,
// MSIX, `npm run perf`) must keep the Microsoft linker so the binary hardening flags of the security
// standard (masterplan area 6, "Windows and Mac security standard") apply to what ships. Cargo lets an
// environment variable override the per-target linker from the config file, so this wrapper sets it
// and runs the given command unchanged.
//
//   node scripts/with-msvc-linker.mjs <command> [args...]

import { spawnSync } from "node:child_process";

const [cmd, ...args] = process.argv.slice(2);
if (!cmd) {
  console.error("Usage: node scripts/with-msvc-linker.mjs <command> [args...]");
  process.exit(2);
}

const env = { ...process.env };
if (process.platform === "win32") env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = "link.exe";

const r = spawnSync(cmd, args, { stdio: "inherit", env, shell: true });
if (r.error) {
  console.error(`Could not start ${cmd}: ${r.error.message}`);
  process.exit(1);
}
process.exit(r.status ?? 1);
