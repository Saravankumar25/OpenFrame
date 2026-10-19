#!/usr/bin/env node
// Software bill of materials for the Mac app (CycloneDX 1.5 JSON on standard output).
//
//   node apps/macos/scripts/mac-sbom.mjs "<path to OpenFrame Studio.app>"
//
// Components: the Rust crates linked into the core (normal dependency edges from openframe-ffi
// for aarch64-apple-darwin, from `cargo metadata --locked`), the offline AI engine files inside
// the bundle with their SHA-256 (pinned in packaging/ai/ai-assets.lock.json), and the app
// executable itself. The Swift package has no third-party dependencies. Apple system frameworks
// are not listed (they ship with macOS).

import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "../../..");
const app = process.argv[2];
if (!app || !fs.existsSync(app)) {
  console.error("usage: node apps/macos/scripts/mac-sbom.mjs <OpenFrame Studio.app>");
  process.exit(2);
}

const sha256 = (file) => crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const meta = JSON.parse(
  execFileSync("cargo", ["metadata", "--format-version", "1", "--locked", "--filter-platform", "aarch64-apple-darwin"], {
    cwd: ROOT,
    maxBuffer: 256 * 1024 * 1024,
    // A Mac without sccache turns the wrapper of .cargo/config.toml off, as the build scripts do.
    env: { ...process.env, RUSTC_WRAPPER: process.env.RUSTC_WRAPPER ?? "", PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}` },
  }).toString(),
);
const byId = new Map(meta.packages.map((p) => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const root = meta.packages.find((p) => p.name === "openframe-ffi");
const seen = new Set();
const stack = [root.id];
while (stack.length) {
  const id = stack.pop();
  if (seen.has(id)) continue;
  seen.add(id);
  for (const dep of nodes.get(id)?.deps ?? []) {
    if (dep.dep_kinds.some((k) => k.kind === null)) stack.push(dep.pkg);
  }
}
const crates = [...seen]
  .map((id) => byId.get(id))
  .filter((p) => p && !p.name.startsWith("openframe-"))
  .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version))
  .map((p) => ({
    type: "library",
    name: p.name,
    version: p.version,
    purl: `pkg:cargo/${p.name}@${p.version}`,
    licenses: p.license ? [{ expression: p.license }] : [],
  }));

const files = [];
const aiDir = path.join(app, "Contents/Resources/ai");
if (fs.existsSync(aiDir)) {
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile()) files.push({ type: "file", name: path.relative(app, p), hashes: [{ alg: "SHA-256", content: sha256(p) }] });
    }
  };
  walk(aiDir);
}
const exe = path.join(app, "Contents/MacOS/OpenFrameStudio");
const version = execFileSync("/usr/libexec/PlistBuddy", ["-c", "Print :CFBundleShortVersionString", path.join(app, "Contents/Info.plist")]).toString().trim();

const sbom = {
  bomFormat: "CycloneDX",
  specVersion: "1.5",
  version: 1,
  metadata: {
    component: {
      type: "application",
      name: "OpenFrame Studio",
      version,
      hashes: [{ alg: "SHA-256", content: sha256(exe) }],
    },
  },
  components: [...crates, ...files],
};
process.stdout.write(JSON.stringify(sbom, null, 2) + "\n");
