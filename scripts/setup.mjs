#!/usr/bin/env node
// OpenFrame Studio developer setup check (Windows 11 first).
//
//   npm run setup                 check prerequisites, then `npm install`
//   node scripts/setup.mjs --check-only   check prerequisites only
//
// Checks: Node >= 20, npm, Rust (rustc/cargo >= workspace rust-version),
// Visual Studio Build Tools "Desktop development with C++" (MSVC + Windows SDK),
// and the Microsoft Edge WebView2 Runtime. Prints the exact fix for anything missing.
// Never installs system software itself.

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const CHECK_ONLY = process.argv.includes("--check-only");
const IS_WIN = process.platform === "win32";

const results = [];
const ok = (name, detail) => results.push({ name, status: "ok", detail });
const warn = (name, detail, fix) => results.push({ name, status: "warn", detail, fix });
const fail = (name, detail, fix) => results.push({ name, status: "fail", detail, fix });

function run(cmd, args) {
  try {
    return execFileSync(cmd, args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], shell: false }).trim();
  } catch {
    return null;
  }
}

function parseVersion(text) {
  const m = /(\d+)\.(\d+)\.(\d+)/.exec(text ?? "");
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}
const gte = (a, b) => a[0] !== b[0] ? a[0] > b[0] : a[1] !== b[1] ? a[1] > b[1] : a[2] >= b[2];

function workspaceRustVersion() {
  try {
    const toml = fs.readFileSync(path.join(ROOT, "Cargo.toml"), "utf8");
    const m = /rust-version\s*=\s*"(\d+)\.(\d+)(?:\.(\d+))?"/.exec(toml);
    if (m) return [Number(m[1]), Number(m[2]), Number(m[3] ?? 0)];
  } catch {
    /* fall through */
  }
  return [1, 85, 0];
}

// ---------------------------------------------------------------- Node / npm
{
  const v = parseVersion(process.versions.node);
  if (v && v[0] >= 20) ok("Node.js", `v${process.versions.node}`);
  else
    fail("Node.js", `v${process.versions.node} (need >= 20)`, "winget install OpenJS.NodeJS.LTS   (or https://nodejs.org, LTS)");
  // npm is a .cmd shim on Windows, which Node only runs through a shell.
  const npmOut = spawnSync("npm --version", { encoding: "utf8", shell: true });
  const npmV = npmOut.status === 0 ? npmOut.stdout.trim() : null;
  if (npmV) ok("npm", npmV);
  else fail("npm", "not found", "npm ships with Node.js; reinstall Node.js LTS.");
}

// ---------------------------------------------------------------- Rust
let cargoCmd = "cargo";
{
  const need = workspaceRustVersion();
  let rustc = run("rustc", ["--version"]);
  let cargo = run("cargo", ["--version"]);
  const cargoBin = path.join(process.env.CARGO_HOME || path.join(os.homedir(), ".cargo"), "bin");
  if (!rustc || !cargo) {
    const exe = IS_WIN ? ".exe" : "";
    const r2 = run(path.join(cargoBin, `rustc${exe}`), ["--version"]);
    const c2 = run(path.join(cargoBin, `cargo${exe}`), ["--version"]);
    if (r2 && c2) {
      rustc = r2;
      cargo = c2;
      cargoCmd = path.join(cargoBin, `cargo${exe}`);
      warn(
        "PATH",
        `Rust is installed in ${cargoBin} but that folder is not on PATH`,
        IS_WIN
          ? `Restart the terminal after installing rustup, or for this session: $env:Path = "$env:USERPROFILE\\.cargo\\bin;$env:Path"`
          : `export PATH="$HOME/.cargo/bin:$PATH"`,
      );
    }
  }
  const rv = parseVersion(rustc);
  if (!rustc || !cargo) {
    fail(
      "Rust toolchain",
      "rustc/cargo not found",
      IS_WIN
        ? "winget install Rustlang.Rustup   then: rustup default stable-msvc   (or https://rustup.rs)"
        : "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
    );
  } else if (rv && !gte(rv, need)) {
    fail("Rust toolchain", `${rustc} (need >= ${need.join(".")})`, "rustup update stable");
  } else {
    ok("Rust toolchain", `${rustc}; ${cargo}`);
    if (IS_WIN) {
      const host = run("rustc", ["-vV"]) ?? run(path.join(path.dirname(cargoCmd), "rustc.exe"), ["-vV"]) ?? "";
      if (host && !/host:\s*\S*-pc-windows-msvc/.test(host)) {
        fail("Rust host target", "not the MSVC toolchain", "rustup default stable-x86_64-pc-windows-msvc");
      }
    }
  }
}

// ---------------------------------------------------------------- Windows-only prerequisites
if (IS_WIN) {
  // Visual Studio Build Tools with the C++ workload (MSVC linker + Windows SDK).
  const pf86 = process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)";
  const vswhere = path.join(pf86, "Microsoft Visual Studio", "Installer", "vswhere.exe");
  let vcPath = null;
  if (fs.existsSync(vswhere)) {
    vcPath = run(vswhere, [
      "-latest",
      "-products",
      "*",
      "-requires",
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      "-property",
      "installationPath",
    ]);
  }
  const vcFix =
    'winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"';
  if (vcPath) ok("VS Build Tools (C++)", vcPath);
  else fail("VS Build Tools (C++)", "MSVC C++ build tools not found", vcFix);

  const kitsRoot = path.join(pf86, "Windows Kits", "10", "Include");
  if (fs.existsSync(kitsRoot) && fs.readdirSync(kitsRoot).length > 0) ok("Windows 10/11 SDK", kitsRoot);
  else warn("Windows 10/11 SDK", "not found", `${vcFix}  (the VCTools workload with --includeRecommended installs the SDK)`);

  // WebView2 Runtime (preinstalled on Windows 11; needed to run `npm run dev`).
  const key = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
  const hives = [
    `HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\${key}`,
    `HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\${key}`,
    `HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients\\${key}`,
  ];
  let wv = null;
  for (const h of hives) {
    const out = spawnSync("reg", ["query", h, "/v", "pv"], { encoding: "utf8" });
    const m = /pv\s+REG_SZ\s+(\S+)/.exec(out.stdout ?? "");
    if (out.status === 0 && m && m[1] !== "0.0.0.0") {
      wv = m[1];
      break;
    }
  }
  if (wv) ok("WebView2 Runtime", wv);
  else
    fail(
      "WebView2 Runtime",
      "not found",
      "winget install Microsoft.EdgeWebView2Runtime   (or https://developer.microsoft.com/microsoft-edge/webview2/)",
    );
} else {
  warn(
    "Platform",
    `${process.platform}: OpenFrame v1 targets Windows 11`,
    "Rust/TypeScript checks and tests work here; building the desktop bundle requires Windows (see DEVELOPMENT.md).",
  );
}

// ---------------------------------------------------------------- report
const icon = { ok: "OK  ", warn: "WARN", fail: "FAIL" };
console.log("\nOpenFrame Studio — development prerequisites\n");
for (const r of results) {
  console.log(`  [${icon[r.status]}] ${r.name}: ${r.detail}`);
  if (r.fix) console.log(`         fix: ${r.fix}`);
}
const failed = results.filter((r) => r.status === "fail");
console.log("");
if (failed.length) {
  console.error(`${failed.length} prerequisite(s) missing. Fix them, open a new terminal, and run \`npm run setup\` again.`);
  process.exit(1);
}
if (CHECK_ONLY) {
  console.log("All required prerequisites found.");
  process.exit(0);
}

console.log("Installing npm dependencies (npm install)…\n");
const npm = spawnSync("npm install", { cwd: ROOT, stdio: "inherit", shell: true });
if (npm.status !== 0) {
  console.error("\n`npm install` failed. See the output above.");
  process.exit(npm.status ?? 1);
}
console.log("\nSetup complete. Next: `npm run dev` (first Rust build takes several minutes).");
