#!/usr/bin/env node
// Builds the internal Windows installer (unsigned NSIS .exe for the team) and its evidence files.
//
//   npm run installer:internal [-- --allow-dirty]
//
// Output goes to release-out/ (git-ignored): OpenFrame-<n>.exe (the next number; the highest is the latest), a .sha256,
// a .json record with the version and commit, and two SBOMs.
// The team password comes from OPENFRAME_TEAM_PASSWORD or the Windows user variable of the same name.
// It is passed only to the child builds and scans, and is never printed or written.

import { spawnSync, execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { prepare } from "./prepare-ai-assets.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TARGET_DIR = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(ROOT, "target");
const OUT_DIR = join(ROOT, "release-out");
const RELEASE_DIR = join(TARGET_DIR, "release");
const allowDirty = process.argv.includes("--allow-dirty");

const fail = (m) => {
  console.error(`installer:internal: ${m}`);
  process.exit(1);
};
const out = (cmd, args) => execFileSync(cmd, args, { cwd: ROOT, encoding: "utf8", shell: false }).trim();
const sha256 = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");
function run(command, env, what) {
  const r = spawnSync(command, { cwd: ROOT, stdio: "inherit", shell: true, env });
  if (r.status !== 0) fail(`${what} failed.`);
}

// 1. Clean tree.
const dirty = out("git", ["status", "--porcelain"]) !== "";
if (dirty && !allowDirty) fail("the working tree has changes. Commit them, or pass --allow-dirty (the record is marked dirty).");

// 2. Team password, never printed.
let password = process.env.OPENFRAME_TEAM_PASSWORD ?? "";
if (!password) {
  try {
    password = out("powershell", ["-NoProfile", "-Command", "[Environment]::GetEnvironmentVariable('OPENFRAME_TEAM_PASSWORD','User')"]);
  } catch {
    password = "";
  }
}
if (password.length < 8) fail("the team password is missing or shorter than 8 characters. Set OPENFRAME_TEAM_PASSWORD (ADR-0030).");

// 3. Build environment: the shipped build uses Hugging Face and the real folders.
const env = { ...process.env, OPENFRAME_TEAM_PASSWORD: password }; // secret-scan: allow (a variable, not a literal)
for (const k of ["OPENFRAME_TEST_MODEL_SOURCE", "OPENFRAME_TEST_BUILD", "OPENFRAME_DISTRIBUTION", "OPENFRAME_DATA_ROOT"]) delete env[k];

// 4. Build.
prepare();
const runner = join(ROOT, "scripts", "cargo-queued.cmd");
run(
  `node scripts/with-msvc-linker.mjs npm --workspace apps/desktop run tauri -- build --bundles nsis --runner "${runner}" --config src-tauri/tauri.bundled-ai.conf.json`,
  env,
  "the Tauri build",
);

// 5. Find and copy the installer.
const nsisDir = join(RELEASE_DIR, "bundle", "nsis");
const found = existsSync(nsisDir)
  ? readdirSync(nsisDir).filter((f) => f.endsWith(".exe")).map((f) => join(nsisDir, f)).sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)
  : [];
if (found.length === 0) fail(`no installer found in ${nsisDir}.`);
const version = JSON.parse(readFileSync(join(ROOT, "apps", "desktop", "src-tauri", "tauri.conf.json"), "utf8")).version;
const commit = out("git", ["rev-parse", "HEAD"]);
mkdirSync(OUT_DIR, { recursive: true });
// Installers are numbered in build order (owner, 2026-10-20): the highest number is the latest. The commit is in the record.
const built = readdirSync(OUT_DIR).map((f) => /^OpenFrame-(\d+)\.exe$/.exec(f)?.[1]).filter(Boolean).map(Number);
const name = `OpenFrame-${Math.max(0, ...built) + 1}`;
const installer = join(OUT_DIR, `${name}.exe`);
copyFileSync(found[0], installer);

// 6. Hash and record.
const hash = sha256(installer);
writeFileSync(`${installer}.sha256`, `${hash}  ${name}.exe\n`);
const appExe = join(RELEASE_DIR, "openframe-desktop.exe");
const record = {
  file: `${name}.exe`,
  sha256: hash,
  sizeBytes: statSync(installer).size,
  version,
  commit,
  branch: out("git", ["rev-parse", "--abbrev-ref", "HEAD"]),
  dirty,
  builtAtUtc: new Date().toISOString(),
  rustc: out("rustc", ["--version"]),
  node: process.version,
  appExeSha256: sha256(appExe),
};
writeFileSync(join(OUT_DIR, `${name}.json`), `${JSON.stringify(record, null, 2)}\n`);

// 7. SBOMs.
const manifestDir = join(ROOT, "apps", "desktop", "src-tauri");
run(
  `cargo cyclonedx --manifest-path "${join(manifestDir, "Cargo.toml")}" --format json --all-features --target x86_64-pc-windows-msvc --override-filename openframe-desktop.rust`,
  env,
  "the Rust SBOM",
);
// cargo cyclonedx also writes one file into every workspace crate it describes; keep the app's, delete the rest.
const sbomFile = "openframe-desktop.rust.json";
const rustSbom = join(manifestDir, sbomFile);
if (!existsSync(rustSbom)) fail("cargo cyclonedx wrote no SBOM file.");
renameSync(rustSbom, join(OUT_DIR, `${name}.rust.cdx.json`));
for (const crate of readdirSync(join(ROOT, "crates"))) rmSync(join(ROOT, "crates", crate, sbomFile), { force: true });
const npmSbom = spawnSync("npm sbom --sbom-format cyclonedx --omit dev --workspace apps/desktop", { cwd: ROOT, shell: true, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
if (npmSbom.status !== 0) fail("the npm SBOM failed.");
writeFileSync(join(OUT_DIR, `${name}.npm.cdx.json`), npmSbom.stdout);

// 8. Scans. scan-installer reads the password from its own environment.
run(`node scripts/check-binary.mjs "${appExe}"`, env, "the binary hardening check");
run(`node scripts/scan-installer.mjs "${installer}" "${appExe}" "${join(RELEASE_DIR, "ai")}"`, env, "the installer scan");

console.log(`\nInternal installer ready:\n  ${installer}\n  size   ${record.sizeBytes} bytes\n  sha256 ${hash}\n  dirty  ${dirty}`);
