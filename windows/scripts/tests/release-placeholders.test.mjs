// Self-tests for the Store release guards: placeholder scan, submission record compare helpers,
// manifest capabilities and the rule that debug builds never run these checks.
//
//   node --test scripts/tests/release-placeholders.test.mjs

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";

import { TEMPLATE_PATH, cargoWorkspaceVersion, loadStoreConfig, readAppVersion, renderManifest } from "../build-msix.mjs";
import {
  RAW_FILES,
  checkIdentityUnchanged,
  checkVersionRises,
  compareVersions,
  loadSubmissions,
  scanPlaceholders,
} from "../check-release-placeholders.mjs";

const REPO = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "../..");
const CLI = path.join(REPO, "scripts", "check-release-placeholders.mjs");
const FILES = ["packaging/msix/store.config.json", "packaging/msix/AppxManifest.template.xml", ...RAW_FILES];
const NO_ENV = { MSIX_IDENTITY_NAME: "", MSIX_PUBLISHER: "", MSIX_PUBLISHER_DISPLAY_NAME: "" };
const REAL_ENV = { MSIX_IDENTITY_NAME: "Straft.Test", MSIX_PUBLISHER: "CN=ABC", MSIX_PUBLISHER_DISPLAY_NAME: "Test" };

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-placeholders-"));
  for (const rel of FILES) {
    const dest = path.join(root, rel);
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.cpSync(path.join(REPO, rel), dest);
  }
  return root;
}

const cli = (root, env) =>
  spawnSync(process.execPath, [CLI, "--root", root], { encoding: "utf8", env: { ...process.env, ...env } });

// ------------------------------------------------------------------------- placeholder scan

test("default config: the three identity placeholders are found and refused", () => {
  const hits = scanPlaceholders({ root: REPO, env: NO_ENV });
  const found = new Set(hits.map((h) => h.placeholder));
  for (const t of ["REPLACE_WITH_PACKAGE_IDENTITY_NAME", "REPLACE_WITH_PACKAGE_IDENTITY_PUBLISHER", "REPLACE_WITH_PUBLISHER_DISPLAY_NAME"]) {
    assert.ok(found.has(t), `${t} should be reported`);
  }
  assert.ok(hits.some((h) => h.file === "packaging/msix/store.config.json"));
  assert.ok(hits.some((h) => h.file.includes("AppxManifest.template.xml (rendered)")));
  const r = cli(REPO, NO_ENV);
  assert.equal(r.status, 1);
  assert.match(r.stderr, /REPLACE_WITH_PACKAGE_IDENTITY_NAME/);
});

test("environment identity values count: nothing is reported and the command exits 0", () => {
  assert.deepEqual(scanPlaceholders({ root: REPO, env: REAL_ENV }), []);
  const r = cli(REPO, REAL_ENV);
  assert.equal(r.status, 0, r.stderr);
});

test("a raw marker in the workflow or the rules file is found; a missing file fails closed", () => {
  const root = fixture();
  const yml = path.join(root, ".github/workflows/release.yml");
  fs.appendFileSync(yml, "\n# REPLACE_WITH_STORE_PRODUCT_ID\n");
  let hits = scanPlaceholders({ root, env: REAL_ENV });
  assert.deepEqual(hits, [{ file: ".github/workflows/release.yml", placeholder: "REPLACE_WITH_STORE_PRODUCT_ID" }]);
  fs.rmSync(path.join(root, "apps/desktop/src/features/licensing/rules.ts"));
  hits = scanPlaceholders({ root, env: REAL_ENV });
  assert.ok(hits.some((h) => h.file.endsWith("rules.ts") && /unreadable/.test(h.placeholder)));
  assert.equal(cli(root, REAL_ENV).status, 1);
});

test("the word REPLACE_WITH_ in prose, without a token after it, does not trip the scan", () => {
  const root = fixture();
  fs.appendFileSync(path.join(root, ".github/workflows/release.yml"), "\n# a placeholder starts with REPLACE_WITH_ and a name in capitals\n");
  assert.deepEqual(scanPlaceholders({ root, env: REAL_ENV }), []);
});

test("a debug or internal build never reaches the placeholder check", () => {
  const build = fs.readFileSync(path.join(REPO, "scripts/build-msix.mjs"), "utf8");
  assert.doesNotMatch(build, /check-release-placeholders|submissions\.json/, "build-msix.mjs (including --dev) must not call the Store checks");
  const validate = fs.readFileSync(path.join(REPO, "scripts/release-validate.mjs"), "utf8");
  const storeStart = validate.search(/if \(STORE\) \{\r?\n  let cfg;/);
  const call = validate.indexOf("scanPlaceholders()");
  const record = validate.indexOf("loadSubmissions(");
  const report = validate.indexOf("// report ---");
  assert.ok(storeStart > 0 && call > storeStart && call < report, "scanPlaceholders runs only inside the Store block");
  assert.ok(record > storeStart && record < report, "the submission record is read only inside the Store block");
});

// ------------------------------------------------------------------------- version and identity

test("version compare is numeric, part by part", () => {
  assert.equal(compareVersions("1.0.10.0", "1.0.9.0"), 1);
  assert.equal(compareVersions("1.2.0.0", "1.10.0.0"), -1);
  assert.equal(compareVersions("1.0.0.0", "1.0.0.0"), 0);
  assert.throws(() => compareVersions("1.0.x", "1.0.0.0"));
});

test("Store version rises: null passes, higher passes, equal and lower fail, fourth part must be 0", () => {
  assert.equal(checkVersionRises("1.0.0.0", null), null);
  assert.equal(checkVersionRises("1.0.1.0", "1.0.0.0"), null);
  assert.equal(checkVersionRises("1.10.0.0", "1.9.0.0"), null);
  assert.match(checkVersionRises("1.0.0.0", "1.0.0.0"), /higher than the last submitted/);
  assert.match(checkVersionRises("1.0.0.0", "1.0.1.0"), /higher than the last submitted/);
  assert.match(checkVersionRises("1.0.1.2", null), /fourth part/);
  assert.match(checkVersionRises("1.0.1.2", "1.0.0.0"), /fourth part/);
  assert.match(checkVersionRises("1.0.1.0", "not-a-version"), /not a dotted numeric version/);
});

test("Store identity unchanged: unset passes, equal passes, a different name or publisher fails", () => {
  const id = { name: "A.B", publisher: "CN=X" };
  assert.equal(checkIdentityUnchanged(id, null), null);
  assert.equal(checkIdentityUnchanged(id, { name: "A.B", publisher: "CN=X" }), null);
  assert.match(checkIdentityUnchanged(id, { name: "A.C", publisher: "CN=X" }), /name/);
  assert.match(checkIdentityUnchanged(id, { name: "A.B", publisher: "CN=Y" }), /publisher/);
});

test("the committed submissions.json loads and starts empty", () => {
  assert.deepEqual(loadSubmissions(REPO), { firstSubmissionIdentity: null, lastSubmittedVersion: null });
});

// ------------------------------------------------------------------------- manifest

test("the manifest renders both capabilities, general first, restricted second", () => {
  const cfg = { ...loadStoreConfig(REAL_ENV), version: "1.0.0.0" };
  const xml = renderManifest(fs.readFileSync(TEMPLATE_PATH, "utf8"), cfg);
  const caps = [...xml.matchAll(/<(?:rescap:|uap\d*:)?Capability\s+Name="([^"]+)"/g)].map((m) => m[1]);
  assert.deepEqual(caps, ["internetClient", "runFullTrust"]);
  assert.match(xml, /<Capability Name="internetClient" \/>\s*<rescap:Capability Name="runFullTrust" \/>/);
  assert.match(xml, /ProcessorArchitecture="x64"/);
  for (const m of xml.matchAll(/<!--([\s\S]*?)-->/g)) assert.ok(!m[1].includes("--"), "an XML comment must not contain a double hyphen");
});

// ------------------------------------------------------------------------- one version source (U1)

test("the version the package is built with equals Cargo.toml [workspace.package].version", () => {
  const cargo = cargoWorkspaceVersion();
  assert.match(cargo, /^\d+\.\d+\.\d+$/);
  assert.equal(readAppVersion(), cargo, "build-msix reads tauri.conf.json, which must equal the source");
  for (const f of ["package.json", "apps/desktop/package.json"]) {
    assert.equal(JSON.parse(fs.readFileSync(path.join(REPO, f), "utf8")).version, cargo, f);
  }
  const text = ["[workspace]", "members = []", "", "[workspace.package]", 'version = "1.2.3"', 'edition = "2024"', "", "[dependencies]", 'version = "9.9.9"', ""].join("\n");
  assert.equal(cargoWorkspaceVersion(text), "1.2.3", "only the workspace.package section counts");
});
