#!/usr/bin/env node
// Writing standard check (docs/engineering/35-writing-standard.md).
//
//   node scripts/check-writing-standard.mjs                 ratchet: fail when a group has more hits than the baseline
//   node scripts/check-writing-standard.mjs --strict        fail on any hit (the target state)
//   node scripts/check-writing-standard.mjs --update        write the current counts to scripts/writing-standard-baseline.json
//   node scripts/check-writing-standard.mjs --list [group]  print every hit (optionally for one group)
//
// A hit is an em dash (U+2014), an en dash (U+2013) used with spaces around it, or a banned phrase. Groups are sets of
// folders so a cleanup pull request can lower one count at a time. Audits, archives and evidence folders are records and
// are never scanned.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const BASELINE = path.join(ROOT, "scripts/writing-standard-baseline.json");

const EM = "—";
const EN_SPACED = / – /;
const BANNED = [
  /\bdelve[sd]?\b/i,
  /\bseamless(ly)?\b/i,
  /\bleverag(e|es|ed|ing)\b/i,
  /\bgame-changer\b/i,
  /\bcutting-edge\b/i,
  /\bunlock the power\b/i,
  /\bharness the power\b/i,
  /\bsupercharge[sd]?\b/i,
  /\bstreamline your workflow\b/i,
  /\btestament to\b/i,
  /\btapestry\b/i,
  /\bnavigate the complexities\b/i,
  /fast-paced world/i,
  /it(?:'s| is) (?:important|worth) (?:to )?not(?:e|ing)/i,
];

// Group name -> folders or files (relative to the repository root). First match wins.
const GROUPS = [
  ["app", ["apps"]],
  ["crates", ["crates", "migrations"]],
  ["scripts-and-ci", ["scripts", ".github", "packaging", "tests"]],
  ["root-documents", ["README.md", "CONTRIBUTING.md", "DEVELOPMENT.md", "CLAUDE.md", "LICENSE-PENDING.md", "THIRD_PARTY_LICENSES.md"]],
  ["mockups", ["HTML_Mockups"]],
  ["living-docs", ["docs"]],
];
const SKIP_DIRS = new Set(["node_modules", "target", ".git", "graphify-out", "dist", "gen", "artifacts", "build", ".drivers", ".run"]);
const SKIP_PREFIXES = [
  "docs/audits/",
  "docs/archive/",
  "docs/release/evidence/",
  "docs/specs/",

  "docs/manual/",
  "tests/fixtures/",
  "scripts/check-writing-standard.mjs",
  "docs/engineering/35-writing-standard.md",
  // These two prompts quote the banned phrases as a list of words to avoid.
  "docs/website/prompts/premium-website-design-motion-prompt.md",
  "docs/website/prompts/production-readiness-anti-ai-slop-prompt.md",
  "THIRD_PARTY_LICENSES.md",
];
const TEXT_EXT = new Set([".md", ".ts", ".tsx", ".js", ".mjs", ".rs", ".html", ".css", ".json", ".yml", ".yaml", ".toml", ".sql", ".txt", ".xml"]);

function* walk(dir, rel) {
  for (const entry of fs.readdirSync(path.join(ROOT, dir), { withFileTypes: true })) {
    const r = `${rel}/${entry.name}`;
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      yield* walk(path.join(dir, entry.name), r);
    } else if (TEXT_EXT.has(path.extname(entry.name).toLowerCase())) {
      yield r;
    }
  }
}

function scan() {
  const counts = {};
  const hits = [];
  for (const [group, members] of GROUPS) {
    counts[group] = 0;
    for (const m of members) {
      const full = path.join(ROOT, m);
      if (!fs.existsSync(full)) continue;
      const files = fs.statSync(full).isDirectory() ? [...walk(m, m)] : [m];
      for (const file of files) {
        if (SKIP_PREFIXES.some((p) => file === p || file.startsWith(p))) continue;
        const lines = fs.readFileSync(path.join(ROOT, file), "utf8").split(/\r?\n/);
        lines.forEach((line, i) => {
          const emCount = line.split(EM).length - 1;
          const kinds = [];
          if (emCount) kinds.push(`${emCount} em dash`);
          if (EN_SPACED.test(line)) kinds.push("spaced en dash");
          for (const re of BANNED) if (re.test(line)) kinds.push(`banned phrase ${re.source}`);
          if (kinds.length) {
            counts[group] += Math.max(1, emCount);
            hits.push({ group, file, line: i + 1, kinds });
          }
        });
      }
    }
  }
  return { counts, hits };
}

const args = process.argv.slice(2);
const { counts, hits } = scan();
const total = Object.values(counts).reduce((a, b) => a + b, 0);

if (args.includes("--list")) {
  const only = args[args.indexOf("--list") + 1];
  for (const h of hits) if (!only || only.startsWith("--") || h.group === only) console.log(`${h.file}:${h.line}  ${h.kinds.join(", ")}`);
  process.exit(0);
}
if (args.includes("--update")) {
  fs.writeFileSync(BASELINE, JSON.stringify({ note: "Counts of writing standard hits per group. Only ever lower these.", counts }, null, 2) + "\n");
  console.log(`check-writing-standard: baseline written (${total} hits)`);
  process.exit(0);
}

const summary = Object.entries(counts).map(([g, n]) => `${g} ${n}`).join(", ");
if (args.includes("--strict")) {
  if (total) {
    console.error(`check-writing-standard: ${total} hit(s) (${summary}). Run with --list to see them.`);
    process.exit(1);
  }
  console.log("check-writing-standard: OK, no hits");
  process.exit(0);
}

const base = fs.existsSync(BASELINE) ? JSON.parse(fs.readFileSync(BASELINE, "utf8")).counts : {};
const errors = [];
for (const [g, n] of Object.entries(counts)) {
  const allowed = base[g] ?? 0;
  if (n > allowed) errors.push(`${g}: ${n} hits, baseline allows ${allowed}. New text must follow docs/engineering/35-writing-standard.md.`);
}
if (errors.length) {
  console.error(`check-writing-standard: ${errors.length} group(s) over the baseline`);
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
const lowered = Object.entries(counts).filter(([g, n]) => n < (base[g] ?? 0)).map(([g]) => g);
console.log(`check-writing-standard: OK (${summary})${lowered.length ? `; lower the baseline for ${lowered.join(", ")} with --update` : ""}`);
