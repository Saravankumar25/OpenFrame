// Desktop E2E runner (`npm run test:e2e`).
//
//   1. installs msedgedriver (matching WebView2) + tauri-driver into .drivers/ (gitignored);
//   2. builds the real app (debug, embedded UI) unless --no-build;
//   3. starts tauri-driver and runs specs/*.test.mjs sequentially with node:test.
//
// The app runs fully isolated in a fresh tests/e2e/.run/<stamp>/ data root
// (OPENFRAME_DATA_ROOT, honoured by debug builds only). Failure screenshots go
// to tests/e2e/artifacts/ (gitignored).

import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
import net from "node:net";
import { DRIVERS, E2E_ROOT, EDGE_DRIVER, TAURI_DRIVER, setupDrivers } from "./scripts/setup-drivers.mjs";
import { APP_BINARY, buildApp } from "./scripts/build-app.mjs";

const args = new Set(process.argv.slice(2));
const PORT = 4444;
const NATIVE_PORT = 4445;

function waitForPort(port, timeoutMs) {
  const until = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const tryOnce = () => {
      const s = net.connect(port, "127.0.0.1");
      s.once("connect", () => {
        s.destroy();
        resolve();
      });
      s.once("error", () => {
        s.destroy();
        if (Date.now() > until) reject(new Error(`tauri-driver did not start on port ${port}.`));
        else setTimeout(tryOnce, 250);
      });
    };
    tryOnce();
  });
}

async function main() {
  await setupDrivers();
  if (!args.has("--no-build") || !existsSync(APP_BINARY)) buildApp();

  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const dataRoot = join(E2E_ROOT, ".run", stamp);
  mkdirSync(dataRoot, { recursive: true });
  mkdirSync(join(E2E_ROOT, "artifacts"), { recursive: true });
  console.log(`E2E data root: ${dataRoot}`);

  const env = { ...process.env, OPENFRAME_DATA_ROOT: dataRoot, E2E_APP: APP_BINARY, E2E_DATA_ROOT: dataRoot, E2E_PORT: String(PORT) };
  const driver = spawn(TAURI_DRIVER, ["--port", String(PORT), "--native-port", String(NATIVE_PORT), "--native-driver", EDGE_DRIVER], {
    env,
    stdio: ["ignore", "inherit", "inherit"],
    cwd: DRIVERS,
  });
  let code = 1;
  try {
    await waitForPort(PORT, 20_000);
    const only = [...args].find((a) => a.startsWith("--only="))?.slice("--only=".length);
    const specs = readdirSync(join(E2E_ROOT, "specs"))
      .filter((f) => f.endsWith(".test.mjs") && (!only || f.includes(only)))
      .sort()
      .map((f) => join("specs", f));
    code = await new Promise((resolve) => {
      const p = spawn(process.execPath, ["--test", "--test-concurrency=1", "--test-reporter=spec", ...specs], {
        cwd: E2E_ROOT,
        env,
        stdio: "inherit",
      });
      p.on("exit", (c) => resolve(c ?? 1));
    });
  } finally {
    driver.kill();
  }
  process.exit(code);
}

main().catch((e) => {
  console.error(e instanceof Error ? e.stack : e);
  process.exit(1);
});
