#!/usr/bin/env node
// Third-party license inventory for Straft Studio.
//
//   node scripts/license-inventory.mjs            write THIRD_PARTY_LICENSES.md
//   node scripts/license-inventory.mjs --check    also exit 1 if a copyleft/unknown license is found
//   node scripts/license-inventory.mjs --out F    write the inventory to F instead
//
// Sources (production dependencies only):
//   Rust: `cargo metadata --format-version 1 --locked --filter-platform x86_64-pc-windows-msvc`;
//          the resolve graph is walked from the shipped workspace members following
//          *normal* dependency edges (dev- and build-only crates are not redistributed).
//   npm:  package-lock.json (v2/v3 `packages` map) entries that are not dev-only; the
//          license comes from the lockfile's `license` field, falling back to
//          node_modules/<pkg>/package.json when the lockfile has none.
//
// Classification: an SPDX expression is evaluated with OR = best option, AND = worst
// component. Categories: permissive, weak-copyleft (review), copyleft, unknown.
// `--check` fails on copyleft and unknown. The output is deterministic (no timestamps)
// so the committed file only changes when dependencies change.
//
// The same run (without --out) also writes the full licence texts that ship inside the app and that
// Settings > About shows: apps/desktop/src/workspaces/settings/third-party-notices.json. The texts
// are the licence, copying and notice files found in each package's own folder (the cargo registry
// source and node_modules), so `cargo metadata` and `npm ci` must have run first.

import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";

const ROOT = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const TARGET = "x86_64-pc-windows-msvc";

const args = process.argv.slice(2);
const CHECK = args.includes("--check");
const outIdx = args.indexOf("--out");
const OUT = path.resolve(ROOT, outIdx >= 0 && args[outIdx + 1] ? args[outIdx + 1] : "THIRD_PARTY_LICENSES.md");
// Only the default run rewrites the in-app notices, so a check with --out never touches tracked files.
const NOTICES_OUT = outIdx >= 0 ? null : path.join(ROOT, "apps", "desktop", "src", "workspaces", "settings", "third-party-notices.json");

// Crates that are part of the workspace but never shipped in the application.
const NON_SHIPPED_WORKSPACE_CRATES = new Set(["straft-test-support"]);

const PERMISSIVE = new Set([
  "0BSD",
  "Apache-2.0",
  "Apache-2.0-with-LLVM-exception",
  "BSD-1-Clause",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "BSL-1.0",
  "CC0-1.0",
  "CDLA-Permissive-2.0",
  "ISC",
  "MIT",
  "MIT-0",
  "NCSA",
  "Python-2.0",
  "Unicode-3.0",
  "Unicode-DFS-2016",
  "Unlicense",
  "Zlib",
  "BlueOak-1.0.0",
]);
// File-level/weak copyleft or unusual permissive terms: allowed only after human review.
const WEAK = new Set(["MPL-2.0", "EPL-2.0", "CDDL-1.0", "OpenSSL", "CC-BY-4.0", "CC-BY-3.0", "Artistic-2.0"]);
const COPYLEFT_PREFIXES = ["GPL-", "AGPL-", "LGPL-", "SSPL-", "EUPL-", "OSL-", "CC-BY-SA-", "CC-BY-NC", "CPAL-", "RPL-"];

const RANK = { permissive: 0, weak: 1, unknown: 2, copyleft: 3 };
const LABEL = {
  permissive: "permissive",
  weak: "weak copyleft / review",
  unknown: "UNKNOWN",
  copyleft: "COPYLEFT",
};

function classifyId(id) {
  const bare = id.replace(/\+$/, "");
  if (PERMISSIVE.has(bare)) return "permissive";
  if (WEAK.has(bare)) return "weak";
  if (COPYLEFT_PREFIXES.some((p) => bare.startsWith(p))) return "copyleft";
  return "unknown";
}

/** Evaluate an SPDX license expression to a category. */
export function classifyExpression(expr) {
  if (!expr || typeof expr !== "string") return "unknown";
  // Legacy cargo form "MIT/Apache-2.0" means OR.
  const normalized = expr.replace(/\s*\/\s*/g, " OR ").trim();
  if (/^SEE LICENSE/i.test(normalized) || /^UNLICENSED$/i.test(normalized)) return "unknown";
  const tokens = normalized.match(/\(|\)|[^\s()]+/g) ?? [];
  let pos = 0;
  const peek = () => tokens[pos];
  const next = () => tokens[pos++];
  function primary() {
    const t = next();
    if (t === undefined) return "unknown";
    if (t === "(") {
      const v = orExpr();
      if (next() !== ")") return "unknown";
      return v;
    }
    let cat = classifyId(t);
    if (peek() && peek().toUpperCase() === "WITH") {
      next();
      next(); // exception id: keeps the base license's category
    }
    return cat;
  }
  function andExpr() {
    let v = primary();
    while (peek() && peek().toUpperCase() === "AND") {
      next();
      const r = primary();
      if (RANK[r] > RANK[v]) v = r;
    }
    return v;
  }
  function orExpr() {
    let v = andExpr();
    while (peek() && peek().toUpperCase() === "OR") {
      next();
      const r = andExpr();
      if (RANK[r] < RANK[v]) v = r;
    }
    return v;
  }
  const result = orExpr();
  return pos === tokens.length ? result : "unknown";
}

// ------------------------------------------------------------------ Rust

function findCargo() {
  const candidates = ["cargo"];
  const home = process.env.CARGO_HOME || path.join(os.homedir(), ".cargo");
  candidates.push(path.join(home, "bin", process.platform === "win32" ? "cargo.exe" : "cargo"));
  for (const c of candidates) {
    try {
      execFileSync(c, ["--version"], { stdio: "ignore" });
      return c;
    } catch {
      /* try next */
    }
  }
  return null;
}

function rustInventory() {
  const cargo = findCargo();
  if (!cargo) throw new Error("cargo was not found. Install Rust (see DEVELOPMENT.md) or add %USERPROFILE%\\.cargo\\bin to PATH.");
  const raw = execFileSync(
    cargo,
    ["metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET],
    { cwd: ROOT, maxBuffer: 512 * 1024 * 1024, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );
  const meta = JSON.parse(raw);
  const pkgById = new Map(meta.packages.map((p) => [p.id, p]));
  const nodeById = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const workspace = new Set(meta.workspace_members);
  const roots = meta.workspace_members.filter((id) => !NON_SHIPPED_WORKSPACE_CRATES.has(pkgById.get(id)?.name));
  const seen = new Set();
  const stack = [...roots];
  while (stack.length) {
    const id = stack.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    const node = nodeById.get(id);
    if (!node) continue;
    for (const dep of node.deps ?? []) {
      const normal = (dep.dep_kinds ?? []).some((k) => k.kind === null || k.kind === undefined);
      if (normal && !seen.has(dep.pkg)) stack.push(dep.pkg);
    }
  }
  const out = [];
  for (const id of seen) {
    if (workspace.has(id)) continue;
    const p = pkgById.get(id);
    if (!p) continue;
    const license = p.license ?? (p.license_file ? `SEE LICENSE FILE (${p.license_file})` : null);
    out.push({
      ecosystem: "cargo",
      name: p.name,
      version: p.version,
      license: license ?? "NOASSERTION",
      category: classifyExpression(p.license),
      source: p.repository ?? (p.source ? p.source.replace(/^registry\+/, "") : ""),
      dir: path.dirname(p.manifest_path),
    });
  }
  return out;
}

// ------------------------------------------------------------------ npm

function npmLicenseField(pkg) {
  if (!pkg) return null;
  if (typeof pkg.license === "string") return pkg.license;
  if (pkg.license && typeof pkg.license.type === "string") return pkg.license.type;
  if (Array.isArray(pkg.licenses) && pkg.licenses.length) {
    return pkg.licenses.map((l) => (typeof l === "string" ? l : l.type)).filter(Boolean).join(" OR ");
  }
  return null;
}

function npmInventory() {
  const lockPath = path.join(ROOT, "package-lock.json");
  if (!fs.existsSync(lockPath)) throw new Error("package-lock.json not found; run `npm install` first.");
  const lock = JSON.parse(fs.readFileSync(lockPath, "utf8"));
  const packages = lock.packages ?? {};
  const out = [];
  const seen = new Set();
  for (const [key, entry] of Object.entries(packages)) {
    if (!key.startsWith("node_modules/") && !key.includes("/node_modules/")) continue; // root/workspaces
    if (entry.link) continue; // workspace symlink (first-party)
    if (entry.dev) continue; // dev-only
    const name = entry.name ?? key.slice(key.lastIndexOf("node_modules/") + "node_modules/".length);
    const version = entry.version ?? "";
    const ident = `${name}@${version}`;
    if (seen.has(ident)) continue;
    seen.add(ident);
    let installed = null;
    const pj = path.join(ROOT, key, "package.json");
    if (fs.existsSync(pj)) {
      try {
        installed = JSON.parse(fs.readFileSync(pj, "utf8"));
      } catch {
        installed = null;
      }
    }
    // Lockfile first so the output is identical with or without node_modules installed;
    // the installed package.json fills gaps (older lockfiles omit `license`).
    const license = npmLicenseField(entry) ?? npmLicenseField(installed);
    const repo = installed?.repository;
    out.push({
      ecosystem: "npm",
      name,
      version,
      license: license ?? "NOASSERTION",
      category: classifyExpression(license),
      source: entry.resolved ?? (typeof repo === "string" ? repo : repo?.url) ?? "",
      optional: Boolean(entry.optional),
      dir: path.join(ROOT, key),
    });
  }
  return out;
}

// ------------------------------------------------------------------ Straft AI components

// Pinned with their hashes in packaging/ai/ai-assets.lock.json. The engine ships in the package; the
// models are downloaded after install from Hugging Face at the pinned revisions (ADR-0026, ADR-0032).
// Listed so their notices ship with the product.
const AI_LOCK = path.join(ROOT, "packaging", "ai", "ai-assets.lock.json");
const AI_NOTICES_DIR = path.join(ROOT, "packaging", "ai", "notices");
const BACKEND_LABEL = { cpu: "processor build", vulkan: "graphics card build, Vulkan", metal: "Apple silicon build, Metal" };

function aiComponents() {
  if (!fs.existsSync(AI_LOCK)) return [];
  const lock = JSON.parse(fs.readFileSync(AI_LOCK, "utf8"));
  const rows = [];
  for (const r of lock.runtimes ?? []) {
    rows.push({
      component: `${r.engine} server (engine in the package, ${r.os} ${r.arch}, ${BACKEND_LABEL[r.backend] ?? r.backend})`,
      version: r.version,
      license: r.licenseId,
      source: r.source.url,
      notes: r.files.some((f) => f.path === "libomp.dll")
        ? "Includes the LLVM OpenMP runtime (Apache-2.0 WITH LLVM-exception); its licence text ships next to it."
        : "",
    });
  }
  for (const md of lock.models ?? []) {
    const gemma = md.licenseId === "Gemma-Terms-of-Use";
    rows.push({
      component: `${md.displayName} (${md.role === "chat" ? "language model" : "search/embedding model"}, ${md.quantization} GGUF, tokenizer embedded, downloaded after install)`,
      version: md.version,
      license: md.licenseId,
      source: md.source.url,
      notes: gemma
        ? `Terms: ${md.licenseUrl}. Required notice: "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms." Use is subject to the Gemma Prohibited Use Policy (ai.google.dev/gemma/prohibited_use_policy). Distribution must pass the use restrictions on in the end-user agreement and give each recipient a copy of the Gemma Terms of Use.`
        : `Licence: ${md.licenseUrl}.`,
    });
  }
  return rows;
}

function aiTable(rows) {
  const lines = ["| Component | Version | License | Source | Notes |", "|---|---|---|---|---|"];
  for (const r of rows) lines.push(`| ${esc(r.component)} | ${esc(r.version)} | ${esc(r.license)} | ${esc(r.source)} | ${esc(r.notes)} |`);
  return lines.join("\n");
}

// ------------------------------------------------------------------ bundled fonts

// Compiled into the app for screenplay and document PDFs (ADR-0016), listed with their hashes in
// crates/straft-import-export/fonts/fonts.json (checked by tests/fonts_manifest.rs).
const FONT_MANIFEST = path.join(ROOT, "crates", "straft-import-export", "fonts", "fonts.json");

function bundledFonts() {
  if (!fs.existsSync(FONT_MANIFEST)) return [];
  return JSON.parse(fs.readFileSync(FONT_MANIFEST, "utf8")).fonts ?? [];
}

function fontTable(rows) {
  const lines = ["| Font file | Family and style | Version | License | Source | SHA-256 |", "|---|---|---|---|---|---|"];
  for (const r of rows) {
    lines.push(
      `| ${esc(r.file)} | ${esc(`${r.family} ${r.style}`)} | ${esc(r.version)} | ${esc(r.license)} (text: \`crates/straft-import-export/fonts/${esc(r.licenseFile)}\`) | ${esc(r.source)} | \`${esc(r.sha256)}\` |`,
    );
  }
  return lines.join("\n");
}

// ------------------------------------------------------------------ report

const esc = (s) => String(s ?? "").replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
const byName = (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version);

function table(rows) {
  const lines = ["| Package | Version | License | Category | Source |", "|---|---|---|---|---|"];
  for (const r of rows) {
    const src = r.source.replace(/^git\+/, "").replace(/\.git$/, "");
    lines.push(`| ${esc(r.name)} | ${esc(r.version)} | ${esc(r.license)} | ${LABEL[r.category]} | ${esc(src)} |`);
  }
  return lines.join("\n");
}

function summary(rows) {
  const counts = new Map();
  for (const r of rows) counts.set(r.license, (counts.get(r.license) ?? 0) + 1);
  const lines = ["| License expression | Packages |", "|---|---:|"];
  for (const [lic, n] of [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) {
    lines.push(`| ${esc(lic)} | ${n} |`);
  }
  return lines.join("\n");
}

function render(rust, npm) {
  const all = [...rust, ...npm];
  const flagged = all.filter((r) => r.category === "copyleft" || r.category === "unknown").sort(byName);
  const review = all.filter((r) => r.category === "weak").sort(byName);
  const parts = [];
  parts.push("# Third-Party Licenses: Straft Studio");
  parts.push("");
  parts.push(
    "<!-- GENERATED FILE. Do not edit by hand. Regenerate with `npm run licenses` (scripts/license-inventory.mjs). -->",
  );
  parts.push("");
  parts.push(
    "This is an inventory of the third-party packages that are compiled into, or shipped with, the Straft Studio " +
      `desktop application (Rust target \`${TARGET}\`, normal dependencies only; npm production dependencies only). ` +
      "Build tools, test tooling and dev-only packages are excluded. This list names each package; the full licence " +
      "texts are in the app, under Settings > About > Third-party licences.",
  );
  parts.push("");
  parts.push("Straft Studio's own license has **not been selected yet**. See `LICENSE-PENDING.md`.");
  parts.push("");
  parts.push("## Summary");
  parts.push("");
  parts.push(`- Rust crates: **${rust.length}**`);
  parts.push(`- npm packages: **${npm.length}**`);
  parts.push(`- Copyleft or unknown (blocking until resolved): **${flagged.length}**`);
  parts.push(`- Weak copyleft / needs review: **${review.length}**`);
  parts.push("");
  const ai = aiComponents();
  if (ai.length) {
    parts.push("## Straft AI components (engine in the package, models downloaded after install)");
    parts.push("");
    parts.push(
      "Every file is pinned by size and SHA-256 in `packaging/ai/ai-assets.lock.json`. The engine ships in the package " +
        "and the build checks it against the lock (ADR-0026). The models are not packaged: the person downloads them " +
        "once, after install, straight from the public Hugging Face repositories at the commit revisions the lock pins, " +
        "and the app uses a file only when its size and SHA-256 match (ADR-0032). They run only on the user's computer. " +
        "Their notices ship in the app's `ai/notices` folder and are shown in Settings > About > Third-party licences.",
    );
    parts.push("");
    parts.push(aiTable(ai));
    parts.push("");
  }
  const fontRows = bundledFonts();
  if (fontRows.length) {
    parts.push("## Bundled fonts (compiled into the app)");
    parts.push("");
    parts.push(
      "Screenplay and document PDFs embed subsets of these fonts (ADR-0016) so text in Telugu, Hindi, Tamil, " +
        "Malayalam, Kannada and other scripts prints without depending on the computer's fonts. The files ship " +
        "unmodified under the SIL Open Font License 1.1; the licence texts sit next to them and are shown in " +
        "Settings > About > Third-party licences.",
    );
    parts.push("");
    parts.push(fontTable(fontRows));
    parts.push("");
  }
  parts.push("## Flagged: copyleft or unknown license");
  parts.push("");
  parts.push(flagged.length ? table(flagged) : "None.");
  parts.push("");
  parts.push("## Needs review: weak copyleft or unusual terms");
  parts.push("");
  parts.push(review.length ? table(review) : "None.");
  parts.push("");
  parts.push("## License expressions in use");
  parts.push("");
  parts.push(summary(all));
  parts.push("");
  parts.push("## Rust crates");
  parts.push("");
  parts.push(table([...rust].sort(byName)));
  parts.push("");
  parts.push("## npm packages (production)");
  parts.push("");
  parts.push(table([...npm].sort(byName)));
  parts.push("");
  return { text: parts.join("\n"), flagged, review };
}

// ------------------------------------------------------------------ in-app notices (full texts)

const LICENCE_FILE = /^(licen[cs]e|copying|notice|copyright|unlicense)([-._ ].*)?$/i;

/** The licence, copying and notice files at the top of a package folder, sorted by name. */
export function licenceFiles(dir) {
  let names;
  try {
    names = fs.readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  return names
    .filter((d) => d.isFile() && LICENCE_FILE.test(d.name))
    .map((d) => d.name)
    .sort()
    .map((name) => ({ name, text: tidy(fs.readFileSync(path.join(dir, name), "utf8")) }))
    .filter((f) => f.text);
}

/** Line endings, trailing spaces and runs of blank lines differ between copies of one licence. */
const tidy = (s) =>
  s
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((l) => l.trimEnd())
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();

/** Packages, AI notices and font licences with their texts; each distinct text is stored once. */
function noticesData(rust, npm) {
  const texts = {};
  const ref = (text) => {
    const id = crypto.createHash("sha256").update(text).digest("hex").slice(0, 12);
    texts[id] = text;
    return id;
  };
  const missing = [];
  const packages = [...rust, ...npm].sort(byName).map((r) => {
    const files = licenceFiles(r.dir);
    if (!files.length) missing.push(`${r.ecosystem} ${r.name}@${r.version}`);
    return { name: r.name, version: r.version, license: r.license, source: r.source.replace(/^git\+/, "").replace(/\.git$/, ""), texts: files.map((f) => ref(f.text)) };
  });
  const ai = fs.existsSync(AI_NOTICES_DIR)
    ? fs.readdirSync(AI_NOTICES_DIR).sort().map((name) => ({ name, text: ref(tidy(fs.readFileSync(path.join(AI_NOTICES_DIR, name), "utf8"))) }))
    : [];
  const fonts = bundledFonts().map((f) => ({
    file: f.file,
    family: `${f.family} ${f.style}`,
    license: f.license,
    source: f.source,
    text: ref(tidy(fs.readFileSync(path.join(path.dirname(FONT_MANIFEST), f.licenseFile), "utf8"))),
  }));
  return { data: { generatedBy: "scripts/license-inventory.mjs (do not edit by hand)", ai, fonts, packages, texts }, missing };
}

function main() {
  let rust;
  let npm;
  try {
    rust = rustInventory();
    npm = npmInventory();
  } catch (e) {
    console.error(`license-inventory: ${e.message}`);
    process.exit(2);
  }
  const { text, flagged, review } = render(rust, npm);
  fs.writeFileSync(OUT, text, "utf8");
  console.log(
    `license-inventory: ${rust.length} Rust crates, ${npm.length} npm packages → ${path.relative(ROOT, OUT)}`,
  );
  if (NOTICES_OUT) {
    const { data, missing } = noticesData(rust, npm);
    fs.writeFileSync(NOTICES_OUT, `${JSON.stringify(data, null, 1)}\n`, "utf8");
    console.log(`license-inventory: notices for ${data.packages.length} packages, ${Object.keys(data.texts).length} distinct texts -> ${path.relative(ROOT, NOTICES_OUT)}`);
    for (const m of missing) console.log(`  no licence file in the package folder: ${m}`);
  }
  for (const r of review) console.log(`  review   ${r.ecosystem} ${r.name}@${r.version}: ${r.license}`);
  for (const r of flagged) console.log(`  FLAGGED  ${r.ecosystem} ${r.name}@${r.version}: ${r.license}`);
  if (CHECK && flagged.length) {
    console.error(`license-inventory: ${flagged.length} package(s) with copyleft or unknown licenses. Resolve before release.`);
    process.exit(1);
  }
}

if (import.meta.url === url.pathToFileURL(process.argv[1] ?? "").href) main();
