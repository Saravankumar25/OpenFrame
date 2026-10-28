// Clean-machine proof (level 4) for the Windows installer, driven from the HOST.
// The app is installed and run inside the VirtualBox guest; the host talks WebDriver to a
// tauri-driver that runs in the guest, so the guest never gets Node, Rust or git.
// Read README.md in this folder first. `--help` lists the options, `--dry-run` prints the plan.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO = join(import.meta.dirname, "..", "..", "..");
const EVIDENCE = join(REPO, "docs", "release", "evidence", "w9", "vm");
const E2E = join(REPO, "tests", "e2e");
const RELEASE_OUT = join(REPO, "release-out");
const VBOX = String.raw`C:\Program Files\Oracle\VirtualBox\VBoxManage.exe`;
const VM = "OpenFrame-W9-Clean";
const SNAPSHOT = "clean";
const USER = "tester";
const PW_FILE = String.raw`C:\w9-vm\guest-account.txt`;
const HOST_STASH = String.raw`C:\w9-vm`;
const PREVIOUS = "OpenFrame-Studio-0.1.0-internal-d271894";
const FINAL = "OpenFrame-Studio-0.1.0-internal-995bfac";
const PROJECT = "Clean Machine Test";
const PORT_HOST = 14444; // host side of the NAT forward; the guest portproxy sends it on to tauri-driver (4444)
const T = String.raw`C:\w9tools`;
const HOME = String.raw`C:\Users\tester`;
const INSTALL_DIR = String.raw`${HOME}\AppData\Local\Programs\OpenFrame Studio`;
const APP_EXE = String.raw`${INSTALL_DIR}\openframe-desktop.exe`;
const APPDATA = String.raw`${HOME}\AppData\Local\OpenFrame`;
const OUT_DIR = String.raw`${HOME}\Documents\w9out`;
const DOCS_OF = String.raw`${HOME}\Documents\OpenFrame`;
const WEBVIEW2_KEY = String.raw`HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}`;

const argv = process.argv.slice(2);
const flag = (n) => argv.includes(n);
const opt = (n, d) => (argv.includes(n) ? argv[argv.indexOf(n) + 1] : d);
const DRY = flag("--dry-run");
const KEEP_GOING = flag("--keep-going");
const OFFLINE_METHOD = opt("--offline", "auto"); // link | firewall | auto

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ---- secrets: read at run time, never printed, never written to the repo ----
let _guestPw;
const guestPw = () => {
  if (_guestPw) return _guestPw;
  const m = /^password:\s*(.+?)\s*$/m.exec(readFileSync(PW_FILE, "utf8"));
  if (!m) throw new Error(`No "password: <value>" line in ${PW_FILE}`);
  return (_guestPw = m[1]);
};
const redact = (s) => {
  let t = String(s ?? "");
  if (_guestPw) t = t.split(_guestPw).join("***");
  return t;
};

// ---- VBoxManage and guest helpers ----
export function vbox(args, { check = true } = {}) {
  const r = spawnSync(VBOX, args, { encoding: "utf8", maxBuffer: 256 * 1024 * 1024 });
  const out = { code: r.status ?? 1, stdout: r.stdout ?? "", stderr: r.stderr ?? "" };
  if (check && out.code !== 0) {
    const shown = args.map((a, i) => (args[i - 1] === "--password" ? "***" : a)).join(" ");
    throw new Error(redact(`VBoxManage ${shown} failed (${out.code}): ${out.stderr.trim() || out.stdout.trim()}`));
  }
  return out;
}
const auth = () => ["--username", USER, "--password", guestPw()];

/** Run a program in the guest and wait. Not elevated, no desktop. For elevated or interactive needs use task(). */
export function guestRun(exe, args = [], { check = false, timeoutMs = 0 } = {}) {
  const t = timeoutMs ? ["--timeout", String(timeoutMs)] : [];
  return vbox(["guestcontrol", VM, "run", "--exe", exe, ...auth(), ...t, "--wait-stdout", "--wait-stderr", "--", exe, ...args], { check });
}
const cmd = (line, o) => guestRun(String.raw`C:\Windows\System32\cmd.exe`, ["/c", line], o);
const ps = (script, o) => guestRun(String.raw`C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`, ["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script], o);

export function copyTo(hostPath, guestDir) {
  cmd(`if not exist "${guestDir}" mkdir "${guestDir}"`);
  vbox(["guestcontrol", VM, "copyto", ...auth(), "--target-directory", guestDir, hostPath]);
}
export function copyFrom(guestPath, hostDir) {
  mkdirSync(hostDir, { recursive: true });
  vbox(["guestcontrol", VM, "copyfrom", ...auth(), "--target-directory", hostDir, guestPath]);
}

/**
 * A scheduled task in the guest that runs as `tester` on the interactive desktop (/it).
 * elevated: run with the highest privileges. wait: poll for the done file; the command's output
 * comes back as a string. The command goes through a small .cmd file, so there are no quoting problems.
 */
export async function task(name, line, { elevated = false, wait = true, timeoutMs = 600_000 } = {}) {
  const dir = String.raw`${T}\tasks`;
  const tmp = mkdtempSync(join(tmpdir(), "w9task-"));
  const file = join(tmp, `${name}.cmd`);
  writeFileSync(file, `@echo off\r\n(${line}) > "${dir}\\${name}.out" 2>&1\r\necho %errorlevel% > "${dir}\\${name}.done"\r\n`);
  copyTo(file, dir);
  cmd(`del /q "${dir}\\${name}.out" "${dir}\\${name}.done" 2>nul`);
  const tn = `w9-${name}`;
  const schtasks = String.raw`C:\Windows\System32\schtasks.exe`;
  guestRun(schtasks, ["/delete", "/tn", tn, "/f"]);
  const created = guestRun(schtasks, ["/create", "/tn", tn, "/tr", `${dir}\\${name}.cmd`, "/sc", "once", "/st", "00:00", "/rl", elevated ? "highest" : "limited", "/it", "/ru", USER, "/rp", guestPw(), "/f"]);
  if (created.code !== 0) throw new Error(redact(`schtasks /create ${tn} failed: ${created.stderr || created.stdout}`));
  const ran = guestRun(schtasks, ["/run", "/tn", tn]);
  if (ran.code !== 0) throw new Error(redact(`schtasks /run ${tn} failed: ${ran.stderr || ran.stdout}`));
  if (!wait) return "";
  const until = Date.now() + timeoutMs;
  while (Date.now() < until) {
    if (cmd(`if exist "${dir}\\${name}.done" echo yes`).stdout.includes("yes")) break;
    await sleep(2000);
  }
  if (Date.now() >= until) throw new Error(`Guest task ${name} did not finish within ${timeoutMs / 1000} s`);
  return cmd(`type "${dir}\\${name}.out"`).stdout;
}

function nextShotNumber() {
  mkdirSync(EVIDENCE, { recursive: true });
  const nums = readdirSync(EVIDENCE).map((f) => /^(\d\d)-/.exec(f)?.[1]).filter(Boolean).map(Number);
  return Math.max(0, ...nums) + 1;
}
export function shot(name) {
  const file = join(EVIDENCE, `${String(nextShotNumber()).padStart(2, "0")}-${name.replace(/[^a-z0-9-_]+/gi, "_")}.png`);
  const r = vbox(["controlvm", VM, "screenshotpng", file], { check: false });
  if (r.code !== 0) console.warn(`  screenshot failed: ${r.stderr.trim()}`);
  return file;
}
export const typeKeys = (text) => vbox(["controlvm", VM, "keyboardputstring", text]);
export const pressKey = (scancodes) => vbox(["controlvm", VM, "keyboardputscancode", ...scancodes.split(" ")]);
const ENTER = "1c 9c";
const CTRL_A = "1d 1e 9e 9d";

// ---- offline: the link goes down; if the driver then stops answering, fall back to a guest firewall rule ----
let _driverUp = false;
async function driverAnswers() {
  try {
    await fetch(`http://127.0.0.1:${PORT_HOST}/status`, { signal: AbortSignal.timeout(4000) });
    return true;
  } catch {
    return false;
  }
}
const FW_RULE = "w9-offline";
const FW_BLOCK = "0.0.0.0-10.0.1.255,10.0.3.0-126.255.255.255,128.0.0.0-255.255.255.255"; // all but loopback and the NAT subnet
async function firewallOffline(on) {
  const del = `netsh advfirewall firewall delete rule name=${FW_RULE}`;
  await task("fw-offline", on ? `${del} & netsh advfirewall firewall add rule name=${FW_RULE} dir=out action=block remoteip=${FW_BLOCK}` : del, { elevated: true });
}
export async function setNetwork(on) {
  const state = readState();
  if (on) {
    vbox(["controlvm", VM, "setlinkstate1", "on"]);
    if (state.offlineMethod === "firewall") await firewallOffline(false);
    writeState({ offline: false, offlineMethod: null });
    return;
  }
  let method = OFFLINE_METHOD === "firewall" ? "firewall" : "link";
  if (method === "link") {
    vbox(["controlvm", VM, "setlinkstate1", "off"]);
    await sleep(3000);
    if (OFFLINE_METHOD === "auto" && _driverUp && !(await driverAnswers())) {
      console.warn("  the driver is unreachable with the link down; using the guest firewall instead");
      vbox(["controlvm", VM, "setlinkstate1", "on"]);
      method = "firewall";
    }
  }
  if (method === "firewall") await firewallOffline(true);
  writeState({ offline: true, offlineMethod: method });
  const probe = ps("(Test-NetConnection huggingface.co -Port 443 -WarningAction SilentlyContinue).TcpTestSucceeded");
  if (/True/i.test(probe.stdout)) throw new Error("The guest still reaches huggingface.co:443 after going offline.");
}

// ---- state between steps (no secrets) and the step log ----
const STATE = join(EVIDENCE, "state.json");
function readState() {
  try {
    return JSON.parse(readFileSync(STATE, "utf8"));
  } catch {
    return {};
  }
}
function writeState(patch) {
  mkdirSync(EVIDENCE, { recursive: true });
  writeFileSync(STATE, JSON.stringify({ ...readState(), ...patch }, null, 2));
}
const LOG = join(EVIDENCE, "steps.json");
function appendLog(entry) {
  let all = [];
  try {
    all = JSON.parse(readFileSync(LOG, "utf8"));
  } catch {
    // first entry
  }
  all.push(entry);
  writeFileSync(LOG, JSON.stringify(all, null, 2));
}

export async function waitGuestReady(maxMs = 15 * 60_000) {
  const until = Date.now() + maxMs;
  while (Date.now() < until) {
    const r = cmd("echo ready");
    if (r.code === 0 && r.stdout.includes("ready")) break;
    await sleep(10_000);
  }
  // Auto-logon must have reached the desktop: the interactive tasks need it.
  while (Date.now() < until) {
    if (/explorer\.exe/i.test(cmd(`tasklist /fi "imagename eq explorer.exe"`).stdout)) return;
    await sleep(5000);
  }
  throw new Error("The guest did not become ready (no guest control or no desktop) within the time limit.");
}

// ---- tauri-driver in the guest and the WebDriver connection from the host ----
async function startDriver() {
  if (await driverAnswers()) {
    _driverUp = true;
    return;
  }
  cmd("taskkill /f /im tauri-driver.exe /im msedgedriver.exe");
  await task("driver", `${T}\\tauri-driver.exe --port 4444 --native-port 4445 --native-driver ${T}\\msedgedriver.exe`, { wait: false });
  const until = Date.now() + 60_000;
  while (Date.now() < until && !(await driverAnswers())) await sleep(1000);
  if (!(await driverAnswers())) throw new Error(`tauri-driver in the guest did not answer on 127.0.0.1:${PORT_HOST} (port forward, portproxy or firewall rule missing? run the prepare step)`);
  _driverUp = true;
}
async function stopDriver() {
  guestRun(String.raw`C:\Windows\System32\schtasks.exe`, ["/end", "/tn", "w9-driver"]);
  cmd("taskkill /f /im tauri-driver.exe /im msedgedriver.exe");
  _driverUp = false;
}
const killApp = () => cmd("taskkill /f /im openframe-desktop.exe /im llama-server.exe");

/** Start the installed app through the guest's tauri-driver. Same capabilities shape as tests/e2e/lib/app.mjs. */
async function connect(appPath = APP_EXE) {
  const { remote } = await import("webdriverio");
  const { APP_WEBVIEW_ARGS, installConsoleProbe } = await import("../lib/app.mjs");
  const browser = await remote({
    hostname: "127.0.0.1",
    port: PORT_HOST,
    logLevel: "warn",
    strictSelectors: false,
    connectionRetryTimeout: 180_000,
    capabilities: {
      browserName: "wry",
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": { application: appPath, webviewOptions: { additionalBrowserArguments: APP_WEBVIEW_ARGS } },
    },
  });
  browser.setTimeout({ implicit: 0 });
  await browser.waitUntil(async () => (await browser.execute(() => document.readyState)) === "complete", { timeout: 60_000, timeoutMsg: "the app window did not finish loading" });
  await installConsoleProbe(browser);
  return browser;
}
async function endApp(browser) {
  await Promise.race([browser?.deleteSession().catch(() => undefined), sleep(20_000)]);
  killApp();
}
async function withApp(fn) {
  await startDriver();
  const browser = await connect();
  try {
    return await fn(browser);
  } finally {
    await endApp(browser);
  }
}
const lib = async () => ({ app: await import("../lib/app.mjs"), flows: await import("../lib/flows.mjs"), ai: await import("../lib/ai.mjs") });
const assert = (cond, msg) => {
  if (!cond) throw new Error(msg);
};

/** Search the open project for a line (Ctrl+K as spec 02 does), jump to it and assert it is in the editor. */
async function assertLine(browser, text) {
  const { flows } = await lib();
  await browser.keys(["Control", "k"]);
  await browser.keys(text.split(""));
  const hit = await browser.$(`//*[@role="option"][contains(normalize-space(.),${flows.xq(text.slice(0, 24))})]`);
  await hit.waitForDisplayed({ timeout: 15_000, timeoutMsg: `search found nothing for "${text}"` });
  await hit.click();
  await browser.waitUntil(async () => (await flows.editorElements(browser)).some((e) => e.endsWith(text)), { timeout: 20_000, timeoutMsg: `the line is not in the screenplay: "${text}"` });
}
/** Open the project from Application Home; returns the recovery text when the app offers it. */
async function openFromHome(browser, title) {
  const { flows } = await lib();
  await flows.waitForHome(browser, 45_000);
  const row = await browser.$(`//main//button[starts-with(@aria-label,${flows.xq(`Open ${title}`)})]`);
  await row.waitForClickable({ timeout: 20_000, timeoutMsg: `"${title}" is not listed on Application Home` });
  await row.click();
  await browser.$('//nav[@aria-label="Workspaces"]').waitForExist({ timeout: 30_000 });
  await sleep(1500);
  const text = await browser.execute(() => document.body.innerText);
  const at = text.search(/recovery state/i);
  return at >= 0 ? text.slice(Math.max(0, at - 200), at + 400) : "";
}

// ---- steps ----
const STEPS = [];
const step = (name, plan, run) => STEPS.push({ name, plan, run });

async function installerRun(base, notes) {
  const exe = `${T}\\${base}.exe`;
  // A limited token on the interactive desktop: if the installer needed admin it would stop at a UAC prompt.
  const out = await task(`install-${base.slice(-7)}`, `whoami /groups | findstr /i "Mandatory" & "${exe}" /S`, { elevated: false, timeoutMs: 300_000 });
  notes[`token-${base.slice(-7)}`] = out.trim();
  const until = Date.now() + 120_000;
  while (Date.now() < until && !cmd(`if exist "${APP_EXE}" echo yes`).stdout.includes("yes")) await sleep(2000);
  assert(cmd(`if exist "${APP_EXE}" echo yes`).stdout.includes("yes"), `${APP_EXE} not found after the silent install`);
  const want = JSON.parse(readFileSync(join(RELEASE_OUT, `${base}.json`), "utf8")).appExeSha256;
  const got = ps(`(Get-FileHash -Algorithm SHA256 '${APP_EXE}').Hash`).stdout.trim().toLowerCase();
  notes.appExeSha256 = got;
  assert(got === want, `installed app hash ${got} does not match the build record ${want} (${base})`);
}

step("prepare", [
  "VBoxManage controlvm poweroff (if running or paused), snapshot restore clean, startvm --type headless",
  "wait for guest control and the desktop (auto-logon), up to 15 min",
  "record Windows version, WebView2 version, VC++ runtime (system-wide); assert git, node, cargo, rustc are absent",
  "copy tauri-driver.exe, the msedgedriver matching the guest WebView2, Procmon64.exe and both installers to C:\\w9tools",
  "controlvm natpf1 w9wd tcp 127.0.0.1:14444 -> guest 14444; guest (elevated task): netsh portproxy 0.0.0.0:14444 -> 127.0.0.1:4444, firewall allow 14444",
  "start Process Monitor elevated: Procmon64.exe /AcceptEula /Quiet /Minimized /BackingFile C:\\w9tools\\session.pml",
  "Microsoft Defender: MpCmdRun.exe -Scan -ScanType 3 -File <installer> -DisableRemediation, for both installers -> defender-<hash>.txt",
], async (notes) => {
  const info = vbox(["showvminfo", VM, "--machinereadable"]).stdout;
  if (/VMState="(running|paused)"/.test(info)) vbox(["controlvm", VM, "poweroff"]);
  await sleep(4000);
  vbox(["snapshot", VM, "restore", SNAPSHOT]);
  vbox(["startvm", VM, "--type", "headless"]);
  writeState({ offline: false, offlineMethod: null, lastLine: null, powerLine: null });
  await waitGuestReady();
  shot("prepare-desktop");
  notes.windows = cmd("ver").stdout.trim();
  notes.webview2 = (/pv\s+REG_SZ\s+([\d.]+)/.exec(cmd(`reg query "${WEBVIEW2_KEY}" /v pv`).stdout) ?? [])[1] ?? "not found";
  assert(notes.webview2 !== "not found", "WebView2 runtime is not installed in the guest");
  const vc = cmd(String.raw`reg query HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64 /v Installed`).stdout;
  notes.vcRuntimeSystemWide = /0x1/.test(vc) ? "installed (the clean snapshot should not have it)" : "not installed";
  notes.vcDllInSystem32 = cmd(String.raw`if exist C:\Windows\System32\vcruntime140.dll echo yes`).stdout.includes("yes");
  const present = ["git", "node", "cargo", "rustc"].filter((t) => /\\/.test(cmd(`where ${t}`).stdout));
  notes.devToolsPresent = present;
  assert(present.length === 0, `developer tools found in the guest: ${present.join(", ")}`);

  // drivers: tauri-driver from the host's cache, msedgedriver matching the GUEST WebView2
  const tauriDriver = join(E2E, ".drivers", "bin", "tauri-driver.exe");
  assert(existsSync(tauriDriver), `${tauriDriver} missing: run npm run test:e2e once so setup-drivers installs it`);
  const edgeDir = join(HOST_STASH, "tools", "edgedriver", notes.webview2);
  const edgeExe = join(edgeDir, "msedgedriver.exe");
  if (!existsSync(edgeExe)) {
    mkdirSync(edgeDir, { recursive: true });
    const res = await fetch(`https://msedgedriver.microsoft.com/${notes.webview2}/edgedriver_win64.zip`);
    assert(res.ok, `msedgedriver ${notes.webview2} download failed (${res.status})`);
    const zip = join(edgeDir, "edgedriver_win64.zip");
    writeFileSync(zip, Buffer.from(await res.arrayBuffer()));
    // bsdtar by full path: a GNU tar from Git Bash cannot read zip files
    const tar = spawnSync(join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe"), ["-xf", zip, "-C", edgeDir], { encoding: "utf8" });
    assert(tar.status === 0 && existsSync(edgeExe), `could not unpack msedgedriver: ${tar.stderr}`);
  }
  for (const f of [tauriDriver, edgeExe, String.raw`${HOST_STASH}\tools\procmon\Procmon64.exe`, join(RELEASE_OUT, `${PREVIOUS}.exe`), join(RELEASE_OUT, `${FINAL}.exe`)]) {
    assert(existsSync(f), `missing on the host: ${f}`);
    copyTo(f, T);
  }

  // reach tauri-driver from the host
  vbox(["controlvm", VM, "natpf1", "delete", "w9wd"], { check: false });
  vbox(["controlvm", VM, "natpf1", `w9wd,tcp,127.0.0.1,${PORT_HOST},,${PORT_HOST}`]);
  await task(
    "netsetup",
    `netsh interface portproxy delete v4tov4 listenaddress=0.0.0.0 listenport=${PORT_HOST} & netsh interface portproxy add v4tov4 listenaddress=0.0.0.0 listenport=${PORT_HOST} connectaddress=127.0.0.1 connectport=4444 & netsh advfirewall firewall delete rule name=w9-driver & netsh advfirewall firewall add rule name=w9-driver dir=in action=allow protocol=TCP localport=${PORT_HOST}`,
    { elevated: true },
  );

  await task("procmon", `${T}\\Procmon64.exe /AcceptEula /Quiet /Minimized /BackingFile ${T}\\session.pml`, { elevated: true, wait: false });
  await sleep(8000);
  notes.procmonRunning = /Procmon64/i.test(cmd(`tasklist /fi "imagename eq Procmon64.exe"`).stdout);
  assert(notes.procmonRunning, "Process Monitor did not start in the guest");

  for (const base of [PREVIOUS, FINAL]) {
    const id = base.slice(-7);
    const line = `powershell -NoProfile -Command "& (Get-ChildItem 'C:\\ProgramData\\Microsoft\\Windows Defender\\Platform\\*\\MpCmdRun.exe' | Sort-Object FullName | Select-Object -Last 1).FullName -Scan -ScanType 3 -File '${T}\\${base}.exe' -DisableRemediation"`;
    const out = await task(`defender-${id}`, line, { elevated: true, timeoutMs: 20 * 60_000 });
    writeFileSync(join(EVIDENCE, `defender-${id}.txt`), out);
    notes[`defender-${id}`] = /found no threats/i.test(out) ? "no threats" : out.trim().split(/\r?\n/).slice(-3).join(" | ");
  }
});

step("install-previous", [
  "silent install of d271894 (installer /S) as a limited-token interactive task, so no admin is possible",
  "assert: under %LOCALAPPDATA%\\Programs, no machine-wide uninstall entry, ai\\ holds the engine and its VC++ DLLs",
  "assert the installed app hash equals the build record",
], async (notes) => {
  await installerRun(PREVIOUS, notes);
  assert(INSTALL_DIR.toLowerCase().startsWith(`${HOME}\\AppData\\Local`.toLowerCase()), "install folder is not per-user");
  notes.installFolder = cmd(`dir /b "${INSTALL_DIR}"`).stdout.trim().split(/\r?\n/);
  const ai = cmd(`dir /b "${INSTALL_DIR}\\ai"`).stdout.trim().split(/\r?\n/);
  notes.aiFolder = ai;
  assert(ai.some((f) => /\.exe$/i.test(f)), "no engine executable in ai\\");
  notes.vcDllsNextToEngine = ai.filter((f) => /^(vcruntime|msvcp|concrt|vcomp).*\.dll$/i.test(f));
  assert(notes.vcDllsNextToEngine.length > 0, "the VC++ DLLs are not next to the engine in ai\\");
  notes.hklmUninstall = cmd(String.raw`reg query HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall /s /f "OpenFrame"`).stdout.includes("OpenFrame") ? "present (unexpected)" : "none";
  notes.hkcuUninstall = cmd(String.raw`reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall /s /f "OpenFrame"`).stdout.includes("OpenFrame") ? "present" : "none";
  assert(notes.hklmUninstall === "none", "the installer wrote a machine-wide uninstall entry");
  shot("install-previous");
});

step("journey", [
  "launch; first-launch name, create project, import a 2-scene screenplay, type an action line, a character name and dialogue",
  "close the project, quit, relaunch, assert the typed text is there and no recovery was offered",
], async (notes) => {
  const unique = `Cold start in the clean room ${Date.now().toString(36)}.`;
  writeState({ lastLine: unique });
  await withApp(async (b) => {
    const { flows } = await lib();
    await flows.firstLaunchProject(b, PROJECT);
    await flows.importFountain(b, flows.TWO_SCENES);
    await (await b.$('(//div[contains(@class,"spx-doc")]//div[@data-type="action"])[1]')).click();
    await b.keys("Ravi pauses at the gate.".split(""));
    await b.keys("Enter");
    await b.keys("ARJUN".split(""));
    await flows.keepNewName(b, "ARJUN");
    await b.keys("Enter");
    await b.keys(unique.split(""));
    await sleep(3000);
    await assertLine(b, unique);
    shot("journey-typed");
    await flows.closeProject(b);
  });
  await withApp(async (b) => {
    notes.recoveryOffered = await openFromHome(b, PROJECT);
    assert(!notes.recoveryOffered, "a recovery offer after a clean close");
    await assertLine(b, unique);
    shot("journey-after-relaunch");
  });
});

step("upgrade", [
  "silent install of 995bfac over the previous build",
  "launch, assert the project and the typed line survived",
  "record the new app hash (both builds are 0.1.0, so the hash tells them apart)",
], async (notes) => {
  const unique = readState().lastLine;
  assert(unique, "no saved line: run journey first");
  await installerRun(FINAL, notes);
  notes.versionInfo = ps(`(Get-Item '${APP_EXE}').VersionInfo.ProductVersion`).stdout.trim();
  await withApp(async (b) => {
    notes.recoveryOffered = await openFromHome(b, PROJECT);
    await assertLine(b, unique);
    shot("upgrade-project-survived");
  });
});

step("ai-download", [
  "launch, open the project, open the AI Assistant, click Download OpenFrame AI",
  "poll the card every 20 s until the question box appears (up to 90 min); record the time and the size of %LOCALAPPDATA%\\OpenFrame\\models",
  "setNetwork(false): controlvm setlinkstate1 off (guest firewall block if the driver stops answering); assert huggingface.co is unreachable",
  "ask 'Summarise this project in one sentence.' (CPU only, up to 10 min); record the time to the first answer; network stays off",
], async (notes) => {
  await withApp(async (b) => {
    const { app, ai, flows } = await lib();
    await openFromHome(b, PROJECT);
    await ai.openAssistant(b);
    const card = () => b.execute(() => document.querySelector('section[aria-label="Download OpenFrame AI"]')?.textContent?.replace(/\s+/g, " ").trim() ?? "");
    await b.waitUntil(async () => /Download OpenFrame AI/.test(await card()), { timeout: 30_000, timeoutMsg: "no download card (is the AI already installed?)" });
    notes.cardBefore = (await card()).slice(0, 300);
    shot("ai-before-download");
    const box = '//aside[@aria-label="AI Assistant"]//textarea[@aria-label="Ask or command"]';
    await app.click(b, "Download OpenFrame AI");
    const t0 = Date.now();
    let lastLog = 0;
    await b.waitUntil(
      async () => {
        if (Date.now() - lastLog > 300_000) {
          lastLog = Date.now();
          console.log(`  ${Math.round((Date.now() - t0) / 60000)} min: ${(await card()).slice(0, 160)}`);
        }
        const ready = await b.$(box).isDisplayed();
        if (!ready && /stopped|damaged|not enough/i.test(await card())) throw new Error(`the download card reports a problem: ${(await card()).slice(0, 300)}`);
        return ready;
      },
      { timeout: 90 * 60_000, interval: 20_000, timeoutMsg: "OpenFrame AI was not ready after 90 minutes" },
    );
    notes.downloadSeconds = Math.round((Date.now() - t0) / 1000);
    shot("ai-ready");
    notes.modelsBytes = Number(ps(`(Get-ChildItem -Recurse -File '${APPDATA}\\models' | Measure-Object Length -Sum).Sum`).stdout.trim());
    notes.modelFiles = cmd(`dir /s /b "${APPDATA}\\models"`).stdout.trim().split(/\r?\n/).length;

    await setNetwork(false);
    shot("network-off");
    const t1 = Date.now();
    const r = await ai.ask(b, "Summarise this project in one sentence.", { timeout: 10 * 60_000 });
    notes.firstAnswerSeconds = Math.round((Date.now() - t1) / 1000);
    notes.answerKind = r.kind;
    notes.answerLength = r.text.length;
    assert(r.text.length > 0, "an empty answer");
    shot("ai-answer-offline");
    await flows.closeProject(b);
  });
});

/** Fill the native Save or Open dialog that is open in the guest: select the name box text, type the full path, Enter. */
async function pickInDialog(path) {
  await sleep(3000); // the dialog needs a moment to open and take focus
  shot("native-dialog");
  pressKey(CTRL_A);
  typeKeys(path);
  await sleep(500);
  pressKey(ENTER);
  await sleep(1500);
}

step("offline-export-backup", [
  "network stays off (re-applied)",
  "mkdir C:\\Users\\tester\\Documents\\w9out; Export > Export… (PDF) through the native Save dialog (Ctrl+A, typeKeys path, Enter)",
  "Project menu > Create Backup… > Browse… (Save dialog) > Create Backup; assert both files with cmd /c dir and the %PDF- header",
  "Home > Restore Backup or Package… > Choose Package… (Open dialog) > Open as Copy; assert the copy opens with the typed line",
], async (notes) => {
  await setNetwork(false);
  cmd(`if not exist "${OUT_DIR}" mkdir "${OUT_DIR}"`);
  const pdf = `${OUT_DIR}\\w9-screenplay.pdf`;
  const backup = `${OUT_DIR}\\w9-backup.ofbackup`;
  const unique = readState().lastLine;
  assert(unique, "no saved line: run journey first");
  await withApp(async (b) => {
    const { app, flows } = await lib();
    await openFromHome(b, PROJECT);
    await assertLine(b, unique); // opens the screenplay
    await app.click(b, "Export");
    await app.waitForText(b, "Export Screenplay");
    await flows.inDialog(b, "Export…");
    await pickInDialog(pdf);
    await app.waitForText(b, "Export complete", 60_000);
    shot("export-complete");
    await flows.inDialog(b, "Close");

    await flows.projectMenu(b, "Create Backup…");
    await app.waitForText(b, "A recoverable copy of supported project state.");
    await flows.inDialog(b, "Browse…");
    await pickInDialog(backup);
    await b.waitUntil(async () => (await (await b.$('//input[@aria-label="Backup location"]')).getValue()) === backup, { timeout: 15_000, timeoutMsg: "the chosen backup location is not shown" });
    await flows.inDialog(b, "Create Backup");
    await app.waitForText(b, "Backup created.", 90_000);
    shot("backup-created");
    await flows.inDialog(b, "Close");

    const listing = cmd(`dir "${OUT_DIR}"`).stdout;
    notes.outFolder = listing.trim().split(/\r?\n/).slice(-6);
    assert(/w9-screenplay\.pdf/i.test(listing), "the PDF was not written");
    assert(/w9-backup\.ofbackup/i.test(listing), "the backup was not written");
    const head = ps(`[System.Text.Encoding]::ASCII.GetString((Get-Content -Encoding Byte -TotalCount 5 '${pdf}'))`).stdout.trim();
    assert(head === "%PDF-", `the exported file is not a PDF (starts with "${head}")`);

    await flows.closeProject(b);
    await app.click(b, "Restore Backup or Package…");
    await flows.inDialog(b, "Choose Package…");
    await pickInDialog(backup);
    await app.waitForText(b, "This project already exists on this computer", 30_000);
    await flows.inDialog(b, "Open as Copy");
    await app.waitForText(b, "Opened as a copy.", 90_000);
    await flows.inDialog(b, "Close");
    await b.waitUntil(async () => (await b.execute(() => document.title)) === `${PROJECT} (copy) - OpenFrame Studio`, { timeout: 30_000, timeoutMsg: "the restored copy did not open" });
    await assertLine(b, unique);
    shot("restored-copy-open");
    notes.restored = "opened as a copy with the typed line";
    await flows.closeProject(b);
  });
});

step("power-loss", [
  "open the project, type a new unique line, wait 3 s (autosave), VBoxManage controlvm poweroff (hard)",
  "startvm --type headless, wait ready, network off again, relaunch",
  "assert the project opens and the line is present; record any recovery message or dialog",
], async (notes) => {
  const line = `Power cut line ${Date.now().toString(36)}.`;
  writeState({ powerLine: line });
  const old = readState().lastLine;
  assert(old, "no saved line: run journey first");
  await startDriver();
  const b = await connect();
  const { flows } = await lib();
  await openFromHome(b, PROJECT);
  await assertLine(b, old);
  const el = await b.$(`//div[contains(@class,"spx-doc")]//div[contains(@class,"sp-el")][contains(normalize-space(.),${flows.xq(old.slice(0, 20))})]`);
  await el.click();
  await b.keys(["Control", "End"]);
  await b.keys("Enter");
  await b.keys(line.split(""));
  await sleep(3000);
  assert((await flows.editorElements(b)).some((e) => e.endsWith(line)), "the new line is not in the editor before the power cut");
  shot("before-power-cut");
  vbox(["controlvm", VM, "poweroff"]);
  _driverUp = false;
  b.deleteSession().catch(() => undefined);
  await sleep(5000);
  vbox(["startvm", VM, "--type", "headless"]);
  await waitGuestReady();
  await setNetwork(false);
  await withApp(async (b2) => {
    notes.recoveryMessage = (await openFromHome(b2, PROJECT)) || "none shown";
    shot("after-power-cut");
    assert(!/error|could not|damaged|corrupt/i.test(notes.recoveryMessage), `a recovery error after the power cut: ${notes.recoveryMessage}`);
    const dialog = await b2.$('//*[@role="dialog"]');
    if (await dialog.isExisting()) notes.recoveryDialog = (await dialog.getText()).slice(0, 400);
    await assertLine(b2, line);
    notes.lineSurvived = true;
  });
});

step("uninstall-keeps-projects", [
  "run uninstall.exe (not silent) through an interactive task",
  "screenshot every 5 s; press Enter on the pages (confirm page, the message box saying projects are kept, finish); the 'delete app data' box is left at its default",
  "assert the program is gone and every entry of Documents\\OpenFrame is still there; list what remains in %LOCALAPPDATA%\\OpenFrame",
], async (notes) => {
  killApp();
  await stopDriver();
  const before = cmd(`dir /b "${DOCS_OF}"`).stdout.trim().split(/\r?\n/).filter(Boolean);
  notes.documentsBefore = before;
  assert(before.length > 0, "Documents\\OpenFrame is empty before the uninstall (nothing to keep)");
  await task("uninstall", `"${INSTALL_DIR}\\uninstall.exe"`, { wait: false });
  const gone = () => !cmd(`if exist "${APP_EXE}" echo yes`).stdout.includes("yes");
  let presses = 0;
  for (let i = 0; i < 24; i++) {
    await sleep(5000);
    shot(`uninstall-page-${i + 1}`);
    const running = /uninstall\.exe|Un_[A-Za-z]\.exe/i.test(cmd("tasklist").stdout);
    if (!running && gone()) break;
    if (running && presses < 4) {
      pressKey(ENTER);
      presses++;
    }
  }
  notes.enterPresses = presses;
  assert(gone(), "the program is still installed after the uninstaller ran (see the uninstall screenshots)");
  const after = cmd(`dir /b "${DOCS_OF}"`).stdout.trim().split(/\r?\n/).filter(Boolean);
  notes.documentsAfter = after;
  assert(before.every((f) => after.includes(f)), `Documents\\OpenFrame lost entries: ${before.filter((f) => !after.includes(f)).join(", ")}`);
  notes.appDataRemaining = cmd(`dir /b "${APPDATA}"`).stdout.trim().split(/\r?\n/);
  notes.installFolderRemaining = cmd(`dir /b "${INSTALL_DIR}"`).stdout.trim() || "gone";
  shot("uninstall-done");
});

step("reinstall-offline-start", [
  "network off; silent install of 995bfac",
  "launch, assert the project is listed on Home and opens with the typed line",
  "assert the AI models are still present, or record why not (the delete-app-data box)",
], async (notes) => {
  await setNetwork(false);
  await installerRun(FINAL, notes);
  await withApp(async (b) => {
    notes.recoveryOffered = await openFromHome(b, PROJECT);
    await assertLine(b, readState().powerLine ?? readState().lastLine);
    shot("reinstall-project-open");
  });
  const models = cmd(`dir /s /b "${APPDATA}\\models"`);
  notes.modelsStillThere = models.stdout.trim().length > 0 && !/not found/i.test(models.stdout + models.stderr);
  if (!notes.modelsStillThere) notes.modelsNote = "models folder absent: the delete-app-data box of the uninstaller removed it, or the uninstaller did (see the uninstall screenshots)";
});

step("finish", [
  "Procmon64.exe /Terminate (elevated); Procmon64.exe /OpenLog session.pml /SaveAs session.csv",
  "copy the CSV to C:\\w9-vm\\procmon\\session.csv (not into the repo)",
  "network back on; run procmon-report.mjs, which writes docs/release/evidence/w9/vm/procmon-summary.md",
], async (notes) => {
  await task("procmon-stop", `${T}\\Procmon64.exe /Terminate`, { elevated: true, timeoutMs: 120_000 });
  await sleep(5000);
  await task("procmon-csv", `${T}\\Procmon64.exe /AcceptEula /Quiet /Minimized /OpenLog ${T}\\session.pml /SaveAs ${T}\\session.csv`, { elevated: true, timeoutMs: 30 * 60_000 });
  const until = Date.now() + 20 * 60_000;
  while (Date.now() < until && /Procmon/i.test(cmd(`tasklist /fi "imagename eq Procmon64.exe"`).stdout)) await sleep(5000);
  cmd("taskkill /f /im Procmon64.exe");
  assert(cmd(`if exist ${T}\\session.csv echo yes`).stdout.includes("yes"), "session.csv was not written");
  notes.pmlBytes = Number(ps(`(Get-Item ${T}\\session.pml).Length`).stdout.trim());
  copyFrom(`${T}\\session.csv`, String.raw`${HOST_STASH}\procmon`);
  await setNetwork(true);
  const r = spawnSync(process.execPath, [join(import.meta.dirname, "procmon-report.mjs")], { encoding: "utf8" });
  notes.report = (r.stdout + r.stderr).trim().slice(-400);
  assert(r.status === 0, `procmon-report.mjs failed: ${r.stderr}`);
});

// ---- runner ----
function help() {
  console.log(`Clean-machine proof for the Windows installer (VirtualBox guest ${VM}).
  node tests/e2e/clean-machine/vm-proof.mjs --dry-run          print every step and its commands, run nothing
  node tests/e2e/clean-machine/vm-proof.mjs --all              prepare, then every step in order
  node tests/e2e/clean-machine/vm-proof.mjs --step <name>      run one step (prepare must have run once)
Options: --keep-going (continue after a failed step), --offline link|firewall|auto (default auto)
Steps: ${STEPS.map((s) => s.name).join(", ")}
It restores the guest snapshot and drives the guest keyboard: do not run it while the host needs the keyboard or windows.`);
}

async function runStep(s) {
  const entry = { name: s.name, start: new Date().toISOString(), end: null, result: "running", notes: {} };
  console.log(`\n== ${s.name}`);
  try {
    await s.run(entry.notes);
    entry.result = "pass";
  } catch (e) {
    entry.result = "fail";
    entry.notes.error = redact(e instanceof Error ? e.message : String(e));
    try {
      shot(`FAIL-${s.name}`);
    } catch {
      // the guest may be down
    }
  }
  entry.end = new Date().toISOString();
  appendLog(entry);
  console.log(`   ${entry.result}${entry.result === "fail" ? `: ${entry.notes.error}` : ""}`);
  return entry.result === "pass";
}

async function main() {
  if (flag("--help") || flag("-h")) return help();
  if (DRY) {
    console.log(`DRY RUN. VBoxManage: ${VBOX}\nVM ${VM}, snapshot ${SNAPSHOT}, guest user ${USER}; evidence: ${EVIDENCE}\nNothing below is executed.\n`);
    for (const s of STEPS) console.log(`${s.name}\n${s.plan.map((p) => `    - ${p}`).join("\n")}`);
    console.log(`\nHelpers: guestcontrol run/copyto/copyfrom, controlvm screenshotpng/keyboardputstring/keyboardputscancode/setlinkstate1/natpf1/poweroff, schtasks /create /rl highest|limited /it /ru ${USER} /rp ***`);
    return;
  }
  const chosen = flag("--all") ? STEPS : STEPS.filter((s) => s.name === opt("--step", ""));
  if (!chosen.length) {
    help();
    process.exitCode = 2;
    return;
  }
  let ok = true;
  try {
    for (const s of chosen) {
      if (!(await runStep(s))) {
        ok = false;
        if (!KEEP_GOING) break;
      }
    }
  } finally {
    await stopDriver().catch(() => undefined);
  }
  process.exitCode = ok ? 0 : 1;
}

main().catch((e) => {
  console.error(redact(e instanceof Error ? e.stack : e));
  process.exitCode = 1;
});
