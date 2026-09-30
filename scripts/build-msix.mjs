#!/usr/bin/env node
// Builds the OpenFrame Studio MSIX package (docs/engineering/27-microsoft-store.md).
//
// Input: the Tauri release build (target/release/openframe-desktop.exe; the frontend is embedded in
// the executable), packaging/msix/AppxManifest.template.xml, packaging/msix/store.config.json and
// the visual assets in packaging/msix/Assets.
// Output (target/msix/): OpenFrame-Studio-<ver>-x64.msix, optional .msixbundle, msix-build.json.
//
//   node scripts/build-msix.mjs --store              Store upload package. Requires the Partner Center
//                                                    identity and an exe built with
//                                                    OPENFRAME_DISTRIBUTION=store. Unsigned: the
//                                                    Store signs it during ingestion.
//   node scripts/build-msix.mjs --dev                Local test package (default): test identity,
//                                                    signed with a throwaway self-signed certificate
//                                                    generated under target/msix/dev-cert (never
//                                                    committed, never added to any certificate store).
//   Options: --exe <path>  --out <dir>  --no-sign  --bundle  --register (dev: Developer Mode loose
//            registration)  --generate-assets (regenerate packaging/msix/Assets from source.png)
//
// Tools: makeappx.exe, makepri.exe and signtool.exe from the newest installed Windows 10/11 SDK
// (C:\Program Files (x86)\Windows Kits\10\bin\<ver>\x64), or WINDOWS_SDK_BIN_DIR.
// The exported helpers are reused by scripts/release-validate.mjs.

import { spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

export const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
export const MSIX_DIR = path.join(ROOT, "packaging", "msix");
export const TEMPLATE_PATH = path.join(MSIX_DIR, "AppxManifest.template.xml");
export const CONFIG_PATH = path.join(MSIX_DIR, "store.config.json");
export const ASSETS_DIR = path.join(MSIX_DIR, "Assets");
export const ICON_SOURCE = path.join(ROOT, "apps", "desktop", "src-tauri", "icons", "source.png");
export const TAURI_CONF = path.join(ROOT, "apps", "desktop", "src-tauri", "tauri.conf.json");
export const EXECUTABLE = "openframe-desktop.exe";
/** Embedded by apps/desktop/src-tauri/src/distribution.rs (CHANNEL_MARKER). */
export const CHANNEL_MARKER_PREFIX = "openframe-distribution-channel:";

const PLACEHOLDER = /REPLACE_/;
const DEV_IDENTITY = {
  identityName: "OpenFrameStudio.LocalTest",
  publisher: "CN=OpenFrame Studio Local Test",
  publisherDisplayName: "OpenFrame Studio (local test build)",
};

// ---------------------------------------------------------------------------------------------
// Visual assets. MRT scale qualifiers 100/125/150/200/400 plus target-size app-list icons (plated,
// unplated and light-unplated) for the taskbar, Start and Explorer. resources.pri (makepri) maps
// the unqualified names used in the manifest to these files.
export const SCALES = [100, 125, 150, 200, 400];
export const TARGET_SIZES = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];
const VISUALS = [
  { base: "Square44x44Logo", width: 44, height: 44, iconFraction: 1 },
  { base: "Square71x71Logo", width: 71, height: 71, iconFraction: 0.66 },
  { base: "Square150x150Logo", width: 150, height: 150, iconFraction: 0.66 },
  { base: "Square310x310Logo", width: 310, height: 310, iconFraction: 0.66 },
  { base: "Wide310x150Logo", width: 310, height: 150, iconFraction: 0.66 },
  { base: "StoreLogo", width: 50, height: 50, iconFraction: 1 },
  { base: "SplashScreen", width: 620, height: 300, iconFraction: 0.5 },
];

export function assetSpec() {
  const out = [];
  for (const v of VISUALS) {
    for (const s of SCALES) {
      out.push({
        file: `${v.base}.scale-${s}.png`,
        width: Math.round((v.width * s) / 100),
        height: Math.round((v.height * s) / 100),
        iconFraction: v.iconFraction,
      });
    }
  }
  for (const t of TARGET_SIZES) {
    for (const alt of ["", "_altform-unplated", "_altform-lightunplated"]) {
      out.push({ file: `Square44x44Logo.targetsize-${t}${alt}.png`, width: t, height: t, iconFraction: 1 });
    }
  }
  return out;
}

export function missingAssets(dir = ASSETS_DIR) {
  return assetSpec()
    .map((a) => a.file)
    .filter((f) => !fs.existsSync(path.join(dir, f)));
}

// ---------------------------------------------------------------------------------------------
// Configuration and validation

export function loadStoreConfig(env = process.env) {
  const cfg = JSON.parse(fs.readFileSync(CONFIG_PATH, "utf8"));
  const pick = (key, fallback) => (env[key] && env[key].trim() ? env[key].trim() : fallback);
  return {
    identityName: pick("MSIX_IDENTITY_NAME", cfg.identity?.name ?? ""),
    publisher: pick("MSIX_PUBLISHER", cfg.identity?.publisher ?? ""),
    publisherDisplayName: pick("MSIX_PUBLISHER_DISPLAY_NAME", cfg.identity?.publisherDisplayName ?? ""),
    displayName: pick("MSIX_DISPLAY_NAME", cfg.displayName ?? ""),
    shortName: cfg.shortName ?? "",
    description: cfg.description ?? "",
    privacyPolicyUrl: pick("STORE_PRIVACY_POLICY_URL", cfg.privacyPolicyUrl ?? ""),
    supportUrl: cfg.supportUrl ?? "",
    minVersion: cfg.minVersion ?? "10.0.17763.0",
    maxVersionTested: cfg.maxVersionTested ?? "10.0.26100.0",
    fileTypeAssociations: cfg.fileTypeAssociations ?? { enabled: false },
  };
}

/** Problems with the Partner Center identity values (empty array = OK). */
export function validateStoreIdentity(c) {
  const problems = [];
  if (!c.identityName || PLACEHOLDER.test(c.identityName)) problems.push("Package/Identity/Name is not set (Partner Center > Product identity)");
  else if (!/^[A-Za-z0-9.-]{3,50}$/.test(c.identityName)) problems.push(`Package/Identity/Name "${c.identityName}" must be 3-50 characters of A-Z, a-z, 0-9, '.' or '-'`);
  if (!c.publisher || PLACEHOLDER.test(c.publisher)) problems.push("Package/Identity/Publisher is not set (Partner Center > Product identity, e.g. CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX)");
  else if (!/^CN=\S/.test(c.publisher) || c.publisher.length > 8192) problems.push(`Package/Identity/Publisher "${c.publisher}" must be a distinguished name starting with CN=`);
  if (!c.publisherDisplayName || PLACEHOLDER.test(c.publisherDisplayName)) problems.push("PublisherDisplayName is not set (Partner Center > Product identity)");
  else if (c.publisherDisplayName.length > 256) problems.push("PublisherDisplayName is longer than 256 characters");
  if (!c.displayName || PLACEHOLDER.test(c.displayName) || c.displayName.length > 256) problems.push("displayName must be the reserved Store name (1-256 characters)");
  return problems;
}

export function validatePrivacyPolicyUrl(u) {
  if (!u || !u.trim()) return "privacy policy URL is not configured (store.config.json privacyPolicyUrl or STORE_PRIVACY_POLICY_URL); Store policy 10.5.1 requires one for Win32 apps";
  if (!/^https:\/\/[^\s/]+\.[^\s]+$/.test(u.trim())) return `privacy policy URL "${u}" must be a public https:// URL`;
  return null;
}

/** tauri.conf.json version X.Y.Z -> MSIX X.Y.Z.0 (the Store reserves the fourth part; must be 0). */
export function msixVersion(version) {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(version ?? "");
  if (!m) throw new Error(`version "${version}" must be X.Y.Z (no pre-release suffix) to map to an MSIX version`);
  const parts = m.slice(1).map(Number);
  if (parts.some((p) => p > 65535)) throw new Error(`version "${version}": each part must be <= 65535`);
  if (parts[0] === 0 && parts[1] === 0 && parts[2] === 0) throw new Error("version 0.0.0 is not a valid package version");
  return `${parts.join(".")}.0`;
}

export function readAppVersion() {
  return JSON.parse(fs.readFileSync(TAURI_CONF, "utf8")).version;
}

const xmlEscape = (s) =>
  String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&apos;");

function fileTypeAssociationsXml(fta) {
  if (!fta?.enabled) return "";
  const exts = (fta.extensions ?? []).filter((e) => /^\.[a-z0-9]{1,16}$/.test(e));
  if (!exts.length) throw new Error("fileTypeAssociations.enabled is true but no valid extensions are listed");
  if (!/^[a-z0-9._-]{1,100}$/.test(fta.name ?? "")) throw new Error("fileTypeAssociations.name must be lowercase [a-z0-9._-]");
  const types = exts.map((e) => `              <uap:FileType>${e}</uap:FileType>`).join("\n");
  return `
      <Extensions>
        <uap:Extension Category="windows.fileTypeAssociation">
          <uap:FileTypeAssociation Name="${xmlEscape(fta.name)}">
            <uap:DisplayName>${xmlEscape(fta.displayName ?? fta.name)}</uap:DisplayName>
            <uap:SupportedFileTypes>
${types}
            </uap:SupportedFileTypes>
          </uap:FileTypeAssociation>
        </uap:Extension>
      </Extensions>`;
}

/** Fill the manifest template. Throws when a token is left unreplaced. */
export function renderManifest(template, v) {
  const values = {
    IDENTITY_NAME: xmlEscape(v.identityName),
    IDENTITY_PUBLISHER: xmlEscape(v.publisher),
    PUBLISHER_DISPLAY_NAME: xmlEscape(v.publisherDisplayName),
    VERSION: v.version,
    DISPLAY_NAME: xmlEscape(v.displayName),
    SHORT_NAME: xmlEscape(v.shortName || v.displayName).slice(0, 40),
    DESCRIPTION: xmlEscape(v.description),
    MIN_VERSION: v.minVersion,
    MAX_VERSION_TESTED: v.maxVersionTested,
    EXECUTABLE,
    APPLICATION_EXTENSIONS: fileTypeAssociationsXml(v.fileTypeAssociations),
  };
  const out = template.replace(/\{\{([A-Z_]+)\}\}/g, (m, k) => {
    if (!(k in values)) throw new Error(`unknown manifest token ${m}`);
    return values[k];
  });
  const left = /\{\{[A-Z_]+\}\}/.exec(out);
  if (left) throw new Error(`manifest token ${left[0]} was not replaced`);
  return out;
}

// ---------------------------------------------------------------------------------------------
// Windows SDK tools

export function findSdkTool(name) {
  const candidates = [];
  if (process.env.WINDOWS_SDK_BIN_DIR) candidates.push(path.join(process.env.WINDOWS_SDK_BIN_DIR, name));
  const kits = path.join(process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)", "Windows Kits", "10", "bin");
  let versions = [];
  try {
    versions = fs.readdirSync(kits).filter((d) => /^10\.0\.\d+\.\d+$/.test(d));
  } catch {
    /* no SDK */
  }
  const num = (v) => v.split(".").map(Number);
  versions.sort((a, b) => {
    const [x, y] = [num(a), num(b)];
    for (let i = 0; i < 4; i++) if (x[i] !== y[i]) return y[i] - x[i];
    return 0;
  });
  for (const v of versions) candidates.push(path.join(kits, v, "x64", name));
  const hit = candidates.find((c) => fs.existsSync(c));
  if (!hit) {
    throw new Error(`${name} not found. Install the Windows 10/11 SDK (App packaging tools) or set WINDOWS_SDK_BIN_DIR to its bin\\<version>\\x64 folder.`);
  }
  return hit;
}

function run(exe, args, { env, quiet } = {}) {
  const r = spawnSync(exe, args, { encoding: "utf8", env: env ? { ...process.env, ...env } : process.env, windowsHide: true });
  const output = `${r.stdout ?? ""}${r.stderr ?? ""}`.trim();
  if (r.error || r.status !== 0) {
    throw new Error(`${path.basename(exe)} failed (exit ${r.status ?? r.error?.message}):\n${output}`);
  }
  if (!quiet && output) console.log(output.split(/\r?\n/).map((l) => `    ${l}`).join("\n"));
  return output;
}

function powershell(script, env) {
  return run("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script], { env, quiet: true });
}

// ---------------------------------------------------------------------------------------------
// Steps

function generateAssets() {
  const specPath = path.join(ROOT, "target", "msix-asset-spec.json");
  fs.mkdirSync(path.dirname(specPath), { recursive: true });
  fs.writeFileSync(specPath, JSON.stringify(assetSpec(), null, 2));
  fs.rmSync(ASSETS_DIR, { recursive: true, force: true });
  const out = run(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", path.join(MSIX_DIR, "generate-assets.ps1"), "-Source", ICON_SOURCE, "-SpecPath", specPath, "-OutDir", ASSETS_DIR],
    { quiet: true },
  );
  console.log(`  ${out}`);
}

function writePriConfig(makepri, outDir) {
  const cfg = path.join(outDir, "priconfig.xml");
  run(makepri, ["createconfig", "/cf", cfg, "/dq", "en-US", "/pv", "10.0.0", "/o"], { quiet: true });
  // A single resources.pri: drop automatic resource-package splitting (only meaningful for bundles
  // with resource packages, which OpenFrame does not ship).
  const xml = fs.readFileSync(cfg, "utf8").replace(/<packaging>[\s\S]*?<\/packaging>/, "");
  fs.writeFileSync(cfg, xml);
  return cfg;
}

function ensureDevCertificate(certDir, subject) {
  fs.mkdirSync(certDir, { recursive: true });
  const pfx = path.join(certDir, "OpenFrame-LocalTest.pfx");
  const cer = path.join(certDir, "OpenFrame-LocalTest.cer");
  const meta = path.join(certDir, "certificate.json");
  if (fs.existsSync(pfx) && fs.existsSync(cer) && fs.existsSync(meta)) {
    const m = JSON.parse(fs.readFileSync(meta, "utf8"));
    if (m.subject === subject && Date.parse(m.notAfter) > Date.now() + 86400000) return { pfx, cer, password: m.password, reused: true };
  }
  const password = crypto.randomBytes(24).toString("base64url");
  // In-memory self-signed code-signing certificate (EKU 1.3.6.1.5.5.7.3.3). It is written only to
  // target/msix/dev-cert and never imported into a certificate store by this script.
  const ps = `
$ErrorActionPreference = 'Stop'
$rsa = [System.Security.Cryptography.RSA]::Create(2048)
$req = New-Object System.Security.Cryptography.X509Certificates.CertificateRequest($env:OF_SUBJECT, $rsa, [System.Security.Cryptography.HashAlgorithmName]::SHA256, [System.Security.Cryptography.RSASignaturePadding]::Pkcs1)
$req.CertificateExtensions.Add((New-Object System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension($false, $false, 0, $true)))
$req.CertificateExtensions.Add((New-Object System.Security.Cryptography.X509Certificates.X509KeyUsageExtension([System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::DigitalSignature, $true)))
$eku = New-Object System.Security.Cryptography.OidCollection
[void]$eku.Add((New-Object System.Security.Cryptography.Oid('1.3.6.1.5.5.7.3.3')))
$req.CertificateExtensions.Add((New-Object System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension($eku, $false)))
$cert = $req.CreateSelfSigned([DateTimeOffset]::UtcNow.AddDays(-1), [DateTimeOffset]::UtcNow.AddDays(180))
[IO.File]::WriteAllBytes($env:OF_PFX, $cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx, $env:OF_PASSWORD))
[IO.File]::WriteAllBytes($env:OF_CER, $cert.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert))
Write-Output ($cert.NotAfter.ToUniversalTime().ToString('o') + '|' + $cert.Thumbprint)
`;
  const [notAfter, thumbprint] = powershell(ps, { OF_SUBJECT: subject, OF_PFX: pfx, OF_CER: cer, OF_PASSWORD: password }).trim().split("|");
  fs.writeFileSync(meta, JSON.stringify({ subject, notAfter, thumbprint, password }, null, 2));
  return { pfx, cer, password, reused: false };
}

function sha256(file) {
  return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
}

function copyDir(src, dst) {
  fs.mkdirSync(dst, { recursive: true });
  for (const e of fs.readdirSync(src, { withFileTypes: true })) {
    const s = path.join(src, e.name);
    const d = path.join(dst, e.name);
    if (e.isDirectory()) copyDir(s, d);
    else fs.copyFileSync(s, d);
  }
}

function parseArgs(argv) {
  const a = { mode: "dev", sign: true, bundle: false, register: false, generateAssets: false, exe: undefined, out: undefined };
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i];
    if (k === "--store") a.mode = "store";
    else if (k === "--dev") a.mode = "dev";
    else if (k === "--no-sign") a.sign = false;
    else if (k === "--bundle") a.bundle = true;
    else if (k === "--register") a.register = true;
    else if (k === "--generate-assets") a.generateAssets = true;
    else if (k === "--exe") a.exe = argv[++i];
    else if (k === "--out") a.out = argv[++i];
    else if (k === "--help" || k === "-h") a.help = true;
    else throw new Error(`unknown argument ${k} (see the header of scripts/build-msix.mjs)`);
  }
  if (a.mode === "store") a.sign = false;
  return a;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log(fs.readFileSync(url.fileURLToPath(import.meta.url), "utf8").split("\n").filter((l) => l.startsWith("//")).join("\n"));
    return;
  }
  if (process.platform !== "win32") throw new Error("MSIX packaging requires Windows (makeappx/makepri/signtool).");

  console.log(`\nOpenFrame Studio: MSIX build (${args.mode})\n`);
  if (args.generateAssets) {
    console.log("• Generating visual assets from apps/desktop/src-tauri/icons/source.png");
    generateAssets();
  }
  const missing = missingAssets();
  if (missing.length) throw new Error(`missing ${missing.length} asset(s) in packaging/msix/Assets (run with --generate-assets): ${missing.slice(0, 5).join(", ")}…`);

  const appVersion = readAppVersion();
  const version = msixVersion(appVersion);
  const cfg = loadStoreConfig();
  let identity;
  if (args.mode === "store") {
    const problems = validateStoreIdentity(cfg);
    if (problems.length) throw new Error(`Store identity incomplete:\n  - ${problems.join("\n  - ")}`);
    identity = { identityName: cfg.identityName, publisher: cfg.publisher, publisherDisplayName: cfg.publisherDisplayName, displayName: cfg.displayName };
  } else {
    identity = { ...DEV_IDENTITY, displayName: `${cfg.displayName || "OpenFrame Studio"} (Test)` };
  }

  const targetDir = process.env.CARGO_TARGET_DIR ? path.resolve(process.env.CARGO_TARGET_DIR) : path.join(ROOT, "target");
  const exe = path.resolve(args.exe ?? path.join(targetDir, "release", EXECUTABLE));
  if (!fs.existsSync(exe)) throw new Error(`${exe} not found. Build first: npm run build (Store: $env:OPENFRAME_DISTRIBUTION='store'; npm run build -- --no-bundle)`);
  const exeBytes = fs.readFileSync(exe);
  const channel = ["store", "direct"].find((c) => exeBytes.includes(Buffer.from(CHANNEL_MARKER_PREFIX + c)));
  if (args.mode === "store" && channel !== "store") {
    throw new Error(`${path.basename(exe)} was built for the "${channel ?? "unknown"}" channel. Rebuild with OPENFRAME_DISTRIBUTION=store so the in-app updater is compiled out.`);
  }
  console.log(`• Executable: ${path.relative(ROOT, exe)} (channel: ${channel ?? "unknown"}, ${(exeBytes.length / 1048576).toFixed(1)} MB)`);
  console.log(`• Version: ${appVersion} -> ${version}`);
  console.log(`• Identity: ${identity.identityName} / ${identity.publisher}`);

  const outDir = path.resolve(args.out ?? path.join(targetDir, "msix"));
  const stage = path.join(outDir, "stage");
  fs.rmSync(stage, { recursive: true, force: true });
  fs.mkdirSync(stage, { recursive: true });
  fs.copyFileSync(exe, path.join(stage, EXECUTABLE));
  copyDir(ASSETS_DIR, path.join(stage, "Assets"));
  for (const f of ["LICENSE", "LICENSE.md", "LICENSE.txt", "THIRD_PARTY_LICENSES.md"]) {
    if (fs.existsSync(path.join(ROOT, f))) fs.copyFileSync(path.join(ROOT, f), path.join(stage, f));
  }
  const manifest = renderManifest(fs.readFileSync(TEMPLATE_PATH, "utf8"), {
    ...identity,
    version,
    shortName: cfg.shortName,
    description: cfg.description,
    minVersion: cfg.minVersion,
    maxVersionTested: cfg.maxVersionTested,
    fileTypeAssociations: cfg.fileTypeAssociations,
  });
  fs.writeFileSync(path.join(stage, "AppxManifest.xml"), manifest);
  console.log(`• Staged package layout: ${path.relative(ROOT, stage)}`);

  const makepri = findSdkTool("makepri.exe");
  const makeappx = findSdkTool("makeappx.exe");
  console.log(`• Windows SDK: ${path.dirname(makeappx)}`);
  const priConfig = writePriConfig(makepri, outDir);
  run(makepri, ["new", "/pr", stage, "/cf", priConfig, "/mn", path.join(stage, "AppxManifest.xml"), "/of", path.join(stage, "resources.pri"), "/o"], { quiet: true });
  console.log("• resources.pri generated (scale/target-size asset variants)");

  const suffix = args.mode === "dev" ? "-localtest" : "";
  const msix = path.join(outDir, `OpenFrame-Studio-${appVersion}-x64${suffix}.msix`);
  fs.rmSync(msix, { force: true });
  run(makeappx, ["pack", "/d", stage, "/p", msix, "/o"], { quiet: true });
  console.log(`• makeappx pack: ${path.relative(ROOT, msix)} (manifest schema and file references validated)`);

  let cert;
  if (args.sign) {
    cert = ensureDevCertificate(path.join(outDir, "dev-cert"), identity.publisher);
    const signtool = findSdkTool("signtool.exe");
    run(signtool, ["sign", "/fd", "SHA256", "/f", cert.pfx, "/p", cert.password, msix], { quiet: true });
    console.log(`• Signed with ${cert.reused ? "existing" : "new"} local test certificate ${path.relative(ROOT, cert.cer)}`);
  }

  let bundle;
  if (args.bundle) {
    const bundleDir = path.join(outDir, "bundle-input");
    fs.rmSync(bundleDir, { recursive: true, force: true });
    fs.mkdirSync(bundleDir, { recursive: true });
    fs.copyFileSync(msix, path.join(bundleDir, path.basename(msix)));
    bundle = path.join(outDir, `OpenFrame-Studio-${appVersion}${suffix}.msixbundle`);
    run(makeappx, ["bundle", "/d", bundleDir, "/p", bundle, "/bv", version, "/o"], { quiet: true });
    if (args.sign) run(findSdkTool("signtool.exe"), ["sign", "/fd", "SHA256", "/f", cert.pfx, "/p", cert.password, bundle], { quiet: true });
    console.log(`• makeappx bundle: ${path.relative(ROOT, bundle)}`);
  }

  const summary = {
    mode: args.mode,
    appVersion,
    packageVersion: version,
    identityName: identity.identityName,
    publisher: identity.publisher,
    channel: channel ?? "unknown",
    msix: path.relative(ROOT, msix),
    msixSha256: sha256(msix),
    bundle: bundle ? path.relative(ROOT, bundle) : null,
    signed: Boolean(args.sign),
    devCertificate: cert ? path.relative(ROOT, cert.cer) : null,
  };
  fs.writeFileSync(path.join(outDir, "msix-build.json"), JSON.stringify(summary, null, 2));
  console.log(`\n  ${path.basename(msix)}  sha256 ${summary.msixSha256}`);

  if (args.mode === "dev") {
    console.log(`
Install the local test package (one-time trust needs an elevated PowerShell; it changes the
machine's certificate trust, so do it yourself and only on a development machine):
  Import-Certificate -FilePath "${cert ? cert.cer : "<signed build needed>"}" -CertStoreLocation Cert:\\LocalMachine\\TrustedPeople
  Add-AppxPackage -Path "${msix}"
Or, with Windows Developer Mode on, register the unsigned layout without any certificate:
  node scripts/build-msix.mjs --dev --register
Remove it again with: Get-AppxPackage ${DEV_IDENTITY.identityName} | Remove-AppxPackage`);
  } else {
    console.log("\nUpload the .msix in Partner Center > Packages. Microsoft signs it during certification.");
  }

  if (args.register) {
    if (args.mode !== "dev") throw new Error("--register is only for local test packages (--dev)");
    powershell(`$ErrorActionPreference='Stop'; Add-AppxPackage -Register "${path.join(stage, "AppxManifest.xml")}"`);
    console.log(`• Registered loose layout ${path.relative(ROOT, stage)} (Developer Mode). Start "OpenFrame Studio (Test)" from Start.`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(`\nMSIX build failed: ${e.message}`);
    process.exit(1);
  });
}
