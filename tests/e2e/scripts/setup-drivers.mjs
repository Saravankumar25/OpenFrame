// Installs the WebDriver pieces for the desktop E2E suite into the gitignored
// tests/e2e/.drivers/ folder (no global installs):
//   * msedgedriver.exe matching the installed WebView2 Runtime version
//     (downloaded from Microsoft's official Edge WebDriver host), and
//   * tauri-driver (cargo install --root tests/e2e/.drivers).
// Re-running is cheap: both are skipped when already present at the right version.

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const E2E_ROOT = join(here, "..");
export const DRIVERS = join(E2E_ROOT, ".drivers");
export const EDGE_DRIVER = join(DRIVERS, "msedgedriver.exe");
export const TAURI_DRIVER = join(DRIVERS, "bin", "tauri-driver.exe");
const TAURI_DRIVER_VERSION = "2.1.0";

const WEBVIEW2_CLIENT = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

/** Installed WebView2 Runtime version (per-machine or per-user install). */
export function webview2Version() {
  const keys = [
    `HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
    `HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
    `HKCU\\Software\\Microsoft\\EdgeUpdate\\Clients\\${WEBVIEW2_CLIENT}`,
  ];
  for (const key of keys) {
    const r = spawnSync("reg", ["query", key, "/v", "pv"], { encoding: "utf8" });
    const m = r.status === 0 ? /pv\s+REG_SZ\s+([\d.]+)/.exec(r.stdout) : null;
    if (m && m[1] !== "0.0.0.0") return m[1];
  }
  throw new Error("The Microsoft Edge WebView2 Runtime is not installed (required by OpenFrame Studio and the E2E suite).");
}

function installedEdgeDriverVersion() {
  if (!existsSync(EDGE_DRIVER)) return null;
  const out = spawnSync(EDGE_DRIVER, ["--version"], { encoding: "utf8" }).stdout ?? "";
  return /([\d]+\.[\d]+\.[\d]+\.[\d]+)/.exec(out)?.[1] ?? null;
}

async function installEdgeDriver(version) {
  if (installedEdgeDriverVersion() === version) {
    console.log(`msedgedriver ${version} already installed.`);
    return;
  }
  const url = `https://msedgedriver.microsoft.com/${version}/edgedriver_win64.zip`;
  console.log(`Downloading Microsoft Edge WebDriver ${version} from ${url}`);
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Download failed (${res.status}) for ${url}`);
  const zip = join(DRIVERS, "edgedriver_win64.zip");
  writeFileSync(zip, Buffer.from(await res.arrayBuffer()));
  const out = join(DRIVERS, "edgedriver");
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  // Windows 10+ ships bsdtar, which extracts zip archives.
  execFileSync("tar", ["-xf", zip, "-C", out], { stdio: "inherit" });
  const exe = join(out, "msedgedriver.exe");
  if (!existsSync(exe)) throw new Error("msedgedriver.exe was not found in the downloaded archive.");
  writeFileSync(EDGE_DRIVER, readFileSync(exe));
  rmSync(zip, { force: true });
  console.log(`Installed ${EDGE_DRIVER} (${installedEdgeDriverVersion()})`);
}

function installTauriDriver() {
  if (existsSync(TAURI_DRIVER)) {
    console.log("tauri-driver already installed.");
    return;
  }
  console.log(`Installing tauri-driver ${TAURI_DRIVER_VERSION} into ${DRIVERS} (cargo install --root)…`);
  const r = spawnSync("cargo", ["install", "tauri-driver", "--version", TAURI_DRIVER_VERSION, "--locked", "--root", DRIVERS], {
    stdio: "inherit",
    shell: false,
  });
  if (r.status !== 0) throw new Error("cargo install tauri-driver failed.");
}

export async function setupDrivers() {
  if (process.platform !== "win32") throw new Error("The desktop E2E suite runs on Windows (WebView2).");
  mkdirSync(DRIVERS, { recursive: true });
  const version = webview2Version();
  console.log(`WebView2 Runtime ${version}`);
  await installEdgeDriver(version);
  installTauriDriver();
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  setupDrivers().catch((e) => {
    console.error(e instanceof Error ? e.message : e);
    process.exit(1);
  });
}
