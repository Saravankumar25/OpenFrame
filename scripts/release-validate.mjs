#!/usr/bin/env node
// Release gate for OpenFrame Studio. FAILS CLOSED: any missing legal or operational
// prerequisite stops the release (Engineering Package Index §7).
//
//   node scripts/release-validate.mjs --tag v0.1.0            (direct NSIS/MSI release)
//   node scripts/release-validate.mjs --store --tag v0.1.0    (Microsoft Store MSIX release)
//   node scripts/release-validate.mjs --report-only [--store] (print status, always exit 0)
//
// Checks (both channels)
//   1. version consistency: Cargo.toml [workspace.package].version, package.json,
//      apps/desktop/package.json, tauri.conf.json — and the git tag (vX.Y.Z) when given;
//   2. an open-source license has been selected (LICENSE file present, LICENSE-PENDING.md
//      removed, manifests no longer carry the pending placeholder);
//   3. no key material is committed;
//   5. THIRD_PARTY_LICENSES.md exists and the license inventory has no copyleft/unknown entries;
//   6. if the local-AI module exists, the model distribution base URL is configured.
// Direct channel only
//   3a. signing material is present in the environment (never printed):
//      WINDOWS_CERTIFICATE, WINDOWS_CERTIFICATE_PASSWORD, TAURI_SIGNING_PRIVATE_KEY,
//      TAURI_SIGNING_PRIVATE_KEY_PASSWORD, WINDOWS_TIMESTAMP_URL;
//   4. the updater is configured with a public key and HTTPS endpoint(s).
// Store channel only (docs/engineering/27-microsoft-store.md). No signing secrets are needed,
// because Microsoft signs Store packages.
//   7. Partner Center identity filled in (packaging/msix/store.config.json or MSIX_* variables);
//   8. the version maps to a Store package version X.Y.Z.0;
//   9. all MSIX visual assets are present, and the manifest template renders and declares only
//      the expected capabilities;
//  10. the privacy policy URL is configured (https) and its source exists (docs/store/privacy-policy.md).

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

import {
  TEMPLATE_PATH,
  loadStoreConfig,
  missingAssets,
  msixVersion,
  renderManifest,
  validatePrivacyPolicyUrl,
  validateStoreIdentity,
} from "./build-msix.mjs";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const argv = process.argv.slice(2);
const REPORT_ONLY = argv.includes("--report-only");
const STORE = argv.includes("--store");
const tagIdx = argv.indexOf("--tag");
let tag = tagIdx >= 0 ? argv[tagIdx + 1] : undefined;
if (!tag && process.env.GITHUB_REF_TYPE === "tag") tag = process.env.GITHUB_REF_NAME;

const checks = [];
const pass = (name, detail = "") => checks.push({ name, ok: true, detail });
const failCheck = (name, detail) => checks.push({ name, ok: false, detail });

const read = (rel) => fs.readFileSync(path.join(ROOT, rel), "utf8");
const exists = (rel) => fs.existsSync(path.join(ROOT, rel));

// 1. versions -------------------------------------------------------------------------
const versions = {};
try {
  const cargo = read("Cargo.toml");
  const section = /\[workspace\.package\]([\s\S]*?)(\n\[|$)/.exec(cargo);
  versions["Cargo.toml [workspace.package]"] = section ? /\nversion\s*=\s*"([^"]+)"/.exec(section[1])?.[1] : undefined;
  versions["package.json"] = JSON.parse(read("package.json")).version;
  versions["apps/desktop/package.json"] = JSON.parse(read("apps/desktop/package.json")).version;
  versions["apps/desktop/src-tauri/tauri.conf.json"] = JSON.parse(read("apps/desktop/src-tauri/tauri.conf.json")).version;
} catch (e) {
  failCheck("Version files readable", e.message);
}
const distinct = [...new Set(Object.values(versions))];
if (distinct.length === 1 && distinct[0] && /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(distinct[0])) {
  pass("Version consistency", distinct[0]);
} else {
  failCheck(
    "Version consistency",
    Object.entries(versions)
      .map(([k, v]) => `${k}=${v ?? "missing"}`)
      .join(", "),
  );
}
const version = distinct.length === 1 ? distinct[0] : undefined;
if (tag !== undefined) {
  if (version && tag === `v${version}`) pass("Tag matches version", tag);
  else failCheck("Tag matches version", `tag ${tag} vs version v${version ?? "?"}`);
}

// 2. license ----------------------------------------------------------------------------
{
  const licenseFiles = ["LICENSE", "LICENSE.md", "LICENSE.txt", "LICENSE-MIT", "LICENSE-APACHE", "COPYING"].filter(exists);
  const problems = [];
  if (exists("LICENSE-PENDING.md")) problems.push("LICENSE-PENDING.md is still present (no license selected)");
  if (licenseFiles.length === 0) problems.push("no LICENSE file");
  try {
    if (/LicenseRef-OpenFrame-Pending/.test(read("Cargo.toml"))) problems.push("Cargo.toml license is the pending placeholder");
    for (const pj of ["package.json", "apps/desktop/package.json"]) {
      if (/LICENSE-PENDING/.test(JSON.parse(read(pj)).license ?? "")) problems.push(`${pj} license points at LICENSE-PENDING.md`);
    }
  } catch (e) {
    problems.push(e.message);
  }
  if (problems.length) failCheck("Open-source license selected", problems.join("; "));
  else pass("Open-source license selected", licenseFiles.join(", "));
}

// 3. signing material (direct channel only; the Store signs MSIX packages itself) --------------
if (!STORE) {
  const required = [
    "WINDOWS_CERTIFICATE",
    "WINDOWS_CERTIFICATE_PASSWORD",
    "TAURI_SIGNING_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
    "WINDOWS_TIMESTAMP_URL",
  ];
  const missing = required.filter((k) => !process.env[k] || !process.env[k].trim());
  if (missing.length) failCheck("Signing material in environment", `missing: ${missing.join(", ")}`);
  else pass("Signing material in environment", "all present (values not shown)");
  const ts = process.env.WINDOWS_TIMESTAMP_URL;
  if (ts && !/^https?:\/\/\S+$/.test(ts)) failCheck("Timestamp URL format", "WINDOWS_TIMESTAMP_URL is not a URL");
}
{
  // Never allow key material to be committed.
  const committed = ["pfx", "p12", "key"]
    .flatMap((ext) => walk(ROOT).filter((f) => f.endsWith(`.${ext}`)))
    .map((f) => path.relative(ROOT, f));
  if (committed.length) failCheck("No key material in repository", committed.join(", "));
  else pass("No key material in repository");
}

// 4. updater (direct channel only: Store builds are updated by the Microsoft Store and compile the
//    in-app updater out, see apps/desktop/src-tauri/src/distribution.rs) ----------------------
if (!STORE) {
  try {
  const conf = JSON.parse(read("apps/desktop/src-tauri/tauri.conf.json"));
  const up = conf.plugins?.updater;
  const problems = [];
  if (!up) problems.push("plugins.updater is not configured in tauri.conf.json");
  else {
    if (!up.pubkey || !String(up.pubkey).trim()) problems.push("plugins.updater.pubkey missing");
    const eps = Array.isArray(up.endpoints) ? up.endpoints : [];
    if (!eps.length) problems.push("plugins.updater.endpoints missing");
    if (eps.some((e) => !/^https:\/\//.test(e))) problems.push("updater endpoints must be https://");
  }
  if (conf.bundle?.createUpdaterArtifacts === false) problems.push("bundle.createUpdaterArtifacts is false");
  if (problems.length) failCheck("Updater configured", problems.join("; "));
  else pass("Updater configured", `${up.endpoints.length} endpoint(s)`);
  } catch (e) {
    failCheck("Updater configured", e.message);
  }
}

// 5. third-party licenses ---------------------------------------------------------------------
if (!exists("THIRD_PARTY_LICENSES.md")) {
  failCheck("THIRD_PARTY_LICENSES.md", "missing — run `npm run licenses`");
} else {
  fs.mkdirSync(path.join(ROOT, "target"), { recursive: true });
  const inv = spawnSync(process.execPath, [path.join(ROOT, "scripts", "license-inventory.mjs"), "--check", "--out", path.join("target", "release-license-check.md")], {
    cwd: ROOT,
    encoding: "utf8",
  });
  if (inv.status === 0) {
    pass("License inventory", "no copyleft/unknown licenses");
    // The committed inventory must match the current dependency set.
    try {
      const fresh = fs.readFileSync(path.join(ROOT, "target", "release-license-check.md"), "utf8");
      const committedInv = read("THIRD_PARTY_LICENSES.md");
      const norm = (s) => s.replace(/\r\n/g, "\n").trim();
      if (norm(fresh) === norm(committedInv)) pass("THIRD_PARTY_LICENSES.md up to date");
      else failCheck("THIRD_PARTY_LICENSES.md up to date", "dependencies changed — run `npm run licenses` and commit");
    } catch (e) {
      failCheck("THIRD_PARTY_LICENSES.md up to date", e.message);
    }
  } else {
    failCheck("License inventory", (inv.stderr || inv.stdout || `exit ${inv.status}`).trim().split("\n").slice(-3).join(" "));
  }
}

// 6. local AI distribution ----------------------------------------------------------------------
if (exists("crates/openframe-application/src/modules/ai")) {
  const base = process.env.MODEL_DISTRIBUTION_BASE_URL;
  if (base && /^https:\/\//.test(base)) pass("Model distribution base URL", "configured");
  else failCheck("Model distribution base URL", "MODEL_DISTRIBUTION_BASE_URL must be an https:// URL for builds that include local AI");
}

// 7-10. Microsoft Store --------------------------------------------------------------------------
if (STORE) {
  let cfg;
  try {
    cfg = loadStoreConfig();
  } catch (e) {
    failCheck("Store configuration readable", `packaging/msix/store.config.json: ${e.message}`);
  }
  if (cfg) {
    const idProblems = validateStoreIdentity(cfg);
    if (idProblems.length) failCheck("Store identity (Partner Center)", idProblems.join("; "));
    else pass("Store identity (Partner Center)", `${cfg.identityName} / ${cfg.publisher}`);

    let pkgVersion;
    try {
      pkgVersion = msixVersion(version);
      pass("Store package version", `${version} -> ${pkgVersion}`);
    } catch (e) {
      failCheck("Store package version", e.message);
    }

    const missing = missingAssets();
    if (missing.length) failCheck("MSIX visual assets", `${missing.length} missing (node scripts/build-msix.mjs --generate-assets): ${missing.slice(0, 4).join(", ")}`);
    else pass("MSIX visual assets", "all scale and target-size variants present");

    try {
      const xml = renderManifest(fs.readFileSync(TEMPLATE_PATH, "utf8"), { ...cfg, version: pkgVersion ?? "1.0.0.0" });
      const caps = [...xml.matchAll(/<(?:rescap:|uap\d*:)?Capability\s+Name="([^"]+)"/g)].map((m) => m[1]).sort();
      const expected = ["internetClient", "runFullTrust"];
      const problems = [];
      if (caps.join(",") !== expected.join(",")) problems.push(`capabilities are [${caps.join(", ")}], expected [${expected.join(", ")}] (a new capability needs a documented justification)`);
      if (!/<TargetDeviceFamily Name="Windows\.Desktop"/.test(xml)) problems.push("TargetDeviceFamily must be Windows.Desktop");
      if (!/EntryPoint="Windows\.FullTrustApplication"/.test(xml)) problems.push("Application must use EntryPoint=Windows.FullTrustApplication");
      if (problems.length) failCheck("MSIX manifest template", problems.join("; "));
      else pass("MSIX manifest template", "renders; capabilities runFullTrust + internetClient");
    } catch (e) {
      failCheck("MSIX manifest template", e.message);
    }

    // The Store updates the app; an in-app updater must be compiled out of Store builds.
    try {
      const lib = read("apps/desktop/src-tauri/src/lib.rs");
      const build = read("apps/desktop/src-tauri/build.rs");
      const problems = [];
      if (!/rustc-cfg=openframe_store/.test(build)) problems.push("build.rs no longer sets cfg(openframe_store) for OPENFRAME_DISTRIBUTION=store");
      if (/tauri_plugin_updater/.test(lib) && !/cfg\(not\(openframe_store\)\)/.test(lib)) problems.push("lib.rs registers tauri_plugin_updater without #[cfg(not(openframe_store))]");
      if (problems.length) failCheck("No in-app updater in Store builds", problems.join("; "));
      else pass("No in-app updater in Store builds", /tauri_plugin_updater/.test(lib) ? "updater compiled out by cfg(openframe_store)" : "no updater plugin integrated");
    } catch (e) {
      failCheck("No in-app updater in Store builds", e.message);
    }

    const privacy = validatePrivacyPolicyUrl(cfg.privacyPolicyUrl);
    if (privacy) failCheck("Privacy policy URL", privacy);
    else pass("Privacy policy URL", cfg.privacyPolicyUrl);
    if (!exists("docs/store/privacy-policy.md")) failCheck("Privacy policy source", "docs/store/privacy-policy.md is missing");
  }
}

// report -------------------------------------------------------------------------------------
console.log(`\nOpenFrame Studio — release validation (${STORE ? "Microsoft Store MSIX" : "direct NSIS/MSI"})\n`);
for (const c of checks) console.log(`  [${c.ok ? "PASS" : "FAIL"}] ${c.name}${c.detail ? ` — ${c.detail}` : ""}`);
const failures = checks.filter((c) => !c.ok);
console.log("");
if (failures.length) {
  console.error(`${failures.length} release check(s) failed. The release is blocked (fail closed).`);
  if (!REPORT_ONLY) process.exit(1);
} else {
  console.log("All release checks passed.");
}

function walk(dir, out = []) {
  const skip = new Set(["node_modules", "target", ".git", "dist", "gen"]);
  let entries = [];
  try {
    entries = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return out;
  }
  for (const e of entries) {
    if (e.isDirectory()) {
      if (!skip.has(e.name)) walk(path.join(dir, e.name), out);
    } else out.push(path.join(dir, e.name));
  }
  return out;
}
