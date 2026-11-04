// Golden test for the names that must never change after the first App Store release (package file
// 05, U3), and for the release placeholder guard (file 06).

import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import url from "node:url";

import { SCANNED, buildNumberRises, findPlaceholders } from "../check-release-placeholders.mjs";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "../..");
const read = (p) => fs.readFileSync(path.join(ROOT, p), "utf8");
const identity = JSON.parse(read("apps/macos/packaging/store-identity.json"));

test("the bundle id, product ids and Keychain names match what the first release recorded", () => {
  assert.equal(identity.bundleId, "app.straftstudio.studio");
  assert.equal(identity.trialProductId, "app.straftstudio.studio.trial7");
  assert.equal(identity.lifetimeProductId, "app.straftstudio.studio.lifetime");
  const plist = read("apps/macos/packaging/Info-store.plist");
  assert.match(plist, new RegExp(`<key>CFBundleIdentifier</key><string>${identity.bundleId}</string>`));
  const bridge = read("crates/straft-entitlement/src/mac/bridge.rs");
  assert.ok(bridge.includes(`TRIAL_PRODUCT_ID: &str = "${identity.trialProductId}"`));
  assert.ok(bridge.includes(`LIFETIME_PRODUCT_ID: &str = "${identity.lifetimeProductId}"`));
  const source = read("apps/macos/Sources/StraftUI/Store/StoreKitSource.swift");
  assert.ok(source.includes(`trialProductID = "${identity.trialProductId}"`));
  assert.ok(source.includes(`lifetimeProductID = "${identity.lifetimeProductId}"`));
  const keychain = read("apps/macos/Sources/StraftUI/Store/GuardKeychain.swift");
  assert.ok(keychain.includes(`.${identity.keychainGroupSuffix}"`));
  assert.ok(keychain.includes(`service = "${identity.keychainService}"`));
  const ent = read("apps/macos/packaging/StraftStudio-store.entitlements");
  assert.ok(ent.includes(`.${identity.keychainGroupSuffix}</string>`));
  const storekit = JSON.parse(read("apps/macos/packaging/StraftStudio.storekit"));
  const byId = Object.fromEntries(storekit.products.map((p) => [p.productID, p]));
  assert.equal(byId[identity.trialProductId].localizations[0].displayName, "7-day Trial");
  assert.equal(byId[identity.trialProductId].displayPrice, "0.00");
  assert.equal(byId[identity.lifetimeProductId].localizations[0].displayName, "Lifetime Access");
  for (const p of storekit.products) {
    assert.equal(p.familyShareable, false);
    assert.equal(p.type, "NonConsumable");
  }
});

test("the store build's sandbox keeps its small list of rights", () => {
  const ent = read("apps/macos/packaging/StraftStudio-store.entitlements").replace(/<!--[\s\S]*?-->/g, "");
  const keys = [...ent.matchAll(/<key>([^<]+)<\/key>/g)].map((m) => m[1]).sort();
  assert.deepEqual(keys, [
    "com.apple.application-identifier",
    "com.apple.developer.team-identifier",
    "com.apple.security.app-sandbox",
    "com.apple.security.device.audio-input",
    "com.apple.security.files.bookmarks.app-scope",
    "com.apple.security.files.user-selected.read-write",
    "com.apple.security.network.client",
    "com.apple.security.network.server",
    "com.apple.security.print",
    "keychain-access-groups",
  ]);
});

test("the privacy manifest declares no tracking and no collected data", () => {
  const m = read("apps/macos/packaging/PrivacyInfo.xcprivacy");
  assert.match(m, /<key>NSPrivacyTracking<\/key><false\/>/);
  assert.match(m, /<key>NSPrivacyCollectedDataTypes<\/key><array\/>/);
});

test("the guard finds placeholders in the store files and ignores the comments that explain them", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "placeholders-"));
  fs.writeFileSync(path.join(dir, "a.plist"), "<!-- REPLACE_WITH_NOTE --><string>REPLACE_WITH_TEAM_ID</string>");
  fs.writeFileSync(path.join(dir, "b.swift"), '// REPLACE_WITH_NOTE\nlet x = "REPLACE_WITH_THING"\n');
  fs.writeFileSync(path.join(dir, "c.txt"), "REPLACE_WITH_NOT_SCANNED");
  const found = findPlaceholders(dir, [{ dir: ".", exts: [".plist", ".swift"] }]).map((f) => f.name).sort();
  assert.deepEqual(found, ["REPLACE_WITH_TEAM_ID", "REPLACE_WITH_THING"]);
  fs.rmSync(dir, { recursive: true });
});

test("a scanned file that is missing fails, and the Mac scan leaves out the Windows file", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "placeholders-"));
  const found = findPlaceholders(dir, [{ file: "gone.plist" }]);
  assert.deepEqual(found, [{ file: "gone.plist", name: "the file is missing" }]);
  fs.rmSync(dir, { recursive: true });
  const mac = findPlaceholders(ROOT, SCANNED.filter((t) => !t.windows)).map((f) => f.file);
  assert.ok(!mac.some((f) => f.includes("msix")));
});

test("today a release build fails: the owner's values are still placeholders", () => {
  const names = findPlaceholders().map((f) => f.name);
  assert.ok(names.includes("REPLACE_WITH_TEAM_ID"));
  assert.ok(names.includes("REPLACE_WITH_APP_CATEGORY"));
});

test("the build number must rise above the last upload", () => {
  assert.equal(buildNumberRises(11, { lastUploadedBuild: 10 }), true);
  assert.equal(buildNumberRises(10, { lastUploadedBuild: 10 }), false);
  assert.equal(buildNumberRises(Number("x"), { lastUploadedBuild: 0 }), false);
});

test("no licence server is left in the shipped addresses or the workflows", () => {
  const addresses = read("scripts/app-addresses.json");
  assert.ok(!/licence\.straft|licence-sandbox/.test(addresses));
  assert.ok(addresses.includes("https://openframestudio.app/contact/"));
  for (const wf of ["release.yml", "mac-core.yml", "ci.yml"]) {
    assert.ok(!/LICENCE_SIGNING|licence-server/.test(read(`.github/workflows/${wf}`)), wf);
  }
});
