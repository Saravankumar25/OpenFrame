#!/usr/bin/env node
// Generates docs/roadmap/mac-parity-matrix.md: the Mac track checklist.
// Reads the Must-ship rows of docs/implementation/feature-status.md and the scenario files in
// tests/e2e/scenarios. Mac status and Mac evidence cells already in the matrix are kept on re-run.
// Run: npm run mac:parity
import { readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const STATUS_FILE = join(root, "docs/implementation/feature-status.md");
const SCENARIO_DIR = join(root, "tests/e2e/scenarios");
const OUT = join(root, "docs/roadmap/mac-parity-matrix.md");

const NOT_STARTED = "Not started";
const NA = "Not applicable on the Mac";

// Scenarios that cannot apply to the Mac, with the reason shown in the matrix.
const SCENARIO_NA = {
  "05-webview-network.md": "tests the Windows WebView2 process; the Mac shell has no web view (the Mac equivalent is the network check in M9)",
  "17-internal-build-release.md": "checks the Windows NSIS and MSI installers; the Mac disk image has its own checks in M9",
};

// Features that cannot apply to the Mac, matched on the feature name, with the reason shown in the matrix.
const FEATURE_NA = [
  [/Windows High Contrast/, "Windows High Contrast; the Mac equivalent is Increase Contrast, covered by the accessibility row in M9"],
  [/WebView2/, "the Mac shell has no web view"],
  [/Supported Windows versions|Windows 11 package manifest/, "Windows packaging; the Mac minimum version is set in M9"],
  [/Store MSIX release candidate|WACK passes/, "Microsoft Store packaging; the Mac App Store package is a Chapter 3 row"],
  [/Clean-machine install, upgrade/, "Windows installer test; the clean Mac test is in M9"],
  [/Windows installer \(\.exe\)/, "Windows installer; the Mac disk image is covered in M9"],
  [/Partner Center identity/, "Microsoft Partner Center; the Apple Developer account is an owner task"],
  [/CI: Windows runner/, "Windows runner; Mac CI is the Mac core check workflow"],
];

// Mac phase: decided by build area first, then by words in the feature name.
const NAME_PHASE = [
  [/password gate|Project create, open|Single-writer|App shell|Offline operation|Global search|Roles and capability|End-user licence/i, "M3"],
  [/Idea Vault|Quick Capture|Project vault|voice note|Drag-in files/i, "M5"],
  [/Recently Deleted|Activity|Files cabinet|notes and lightweight tasks/i, "M6"],
  [/Keyboard-only|accessib/i, "M9"],
];

function featurePhase(name, area) {
  for (const [re, phase] of NAME_PHASE) if (re.test(name)) return phase;
  if (/export|import|FDX|pdf/i.test(name) && !/Windows|Store/i.test(area)) return "M7";
  if (/Screenplay engine|Writing comfort/i.test(area)) return "M4";
  if (/Packages and backup/i.test(area)) return "M7";
  if (/AI assistant/i.test(area)) return "M8";
  if (/Story Board and Idea Vault|Breakdown/i.test(area)) return "M5";
  if (/Shooting schedule|Call sheets, budget/i.test(area)) return "M6";
  return "M9";
}

function scenarioPhase(file) {
  const n = parseInt(file, 10);
  if (n === 0 || n === 1 || n === 2 || n === 4 || n === 5 || n === 10 || n === 15 || n === 16 || n === 60 || n === 91) return "M3";
  if (n === 17 || n === 26 || n === 90 || n === 95 || n === 96 || n === 97) return "M9";
  if (n >= 20 && n <= 31) return "M5";
  if (n >= 40 && n <= 44) return "M4";
  if (n === 50 || n === 51) return "M5";
  if (n === 61 || n === 62 || n === 80 || n === 85 || n === 86 || n === 25) return "M6";
  if (n === 63 || n === 70) return "M7";
  if (n >= 72 && n <= 78) return "M8";
  return "M9";
}

const cell = (s) => String(s).replace(/\s+/g, " ").trim().replace(/\|/g, "\\|");

function readOld() {
  const features = new Map();
  const scenarios = new Map();
  if (!existsSync(OUT)) return { features, scenarios };
  let table = 0;
  for (const line of readFileSync(OUT, "utf8").split("\n")) {
    if (/^## Table 1/.test(line)) table = 1;
    else if (/^## Table 2/.test(line)) table = 2;
    if (!line.startsWith("|")) continue;
    const p = line.split(/(?<!\\)\|/).slice(1, -1).map((x) => x.trim());
    if (table === 1 && p.length === 6 && p[0] !== "Feature" && !/^-+$/.test(p[0])) features.set(p[0], { status: p[4], evidence: p[5] });
    if (table === 2 && p.length === 5 && p[0] !== "File" && !/^-+$/.test(p[0])) scenarios.set(p[0].replace(/`/g, ""), { status: p[3], test: p[4] });
  }
  return { features, scenarios };
}

const old = readOld();

// Feature rows: table columns are Feature, Importance, Exists, Missing, Area, Route, Tests, Status, Proof, Key.
const features = [];
for (const line of readFileSync(STATUS_FILE, "utf8").split("\n")) {
  if (!line.startsWith("|")) continue;
  const p = line.split(/(?<!\\)\|/).slice(1, -1).map((x) => x.trim());
  if (p.length !== 10 || p[1] !== "Must ship") continue;
  features.push({ name: p[0].replace(/\*\*/g, ""), area: p[4], status: p[7], proof: p[8], key: p[9] });
}

const featureRows = features.map((f) => {
  const na = FEATURE_NA.find(([re]) => re.test(f.name));
  const prev = old.features.get(cell(f.name));
  let status = prev?.status || (na ? `${NA}: ${na[1]}` : NOT_STARTED);
  if (na && (!prev || prev.status === NOT_STARTED)) status = `${NA}: ${na[1]}`;
  const phase = na ? "-" : featurePhase(f.name, f.area);
  return { ...f, phase, macStatus: status, evidence: prev?.evidence || "" };
});

// Scenario rows.
const scenarioRows = readdirSync(SCENARIO_DIR)
  .filter((f) => /^\d+-.*\.md$/.test(f))
  .sort()
  .map((file) => {
    const text = readFileSync(join(SCENARIO_DIR, file), "utf8");
    const title = (text.match(/^#\s+(.+)$/m) || [, file])[1].trim();
    const items = (text.match(/^## /gm) || []).length;
    const prev = old.scenarios.get(file);
    const naReason = SCENARIO_NA[file];
    let status = prev?.status || (naReason ? `${NA}: ${naReason}` : NOT_STARTED);
    if (naReason && (!prev || prev.status === NOT_STARTED)) status = `${NA}: ${naReason}`;
    return { file, title, items, phase: naReason ? "-" : scenarioPhase(file), macStatus: status, test: prev?.test || "" };
  });

const isNA = (s) => s.startsWith(NA);
const naFeatures = featureRows.filter((r) => isNA(r.macStatus)).length;
const naScenarios = scenarioRows.filter((r) => isNA(r.macStatus)).length;
const scenarioItems = scenarioRows.reduce((a, r) => a + r.items, 0);

const out = [];
out.push(`# Mac parity matrix

> Generated by \`npm run mac:parity\` (\`scripts/gen-mac-parity.mjs\`). Edit only the Mac status, Mac evidence and Mac test
> cells by hand. The script keeps them when it runs again and refreshes everything else.

This is the checklist of the Mac track. The Mac app is a native SwiftUI shell over the same Rust core as the Windows app
([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). Every Windows Must-ship feature and every Windows end-to-end
scenario has a row. The phases M3 to M10 are in [chapter-1b-mac-phases.md](chapter-1b-mac-phases.md). The Mac track closes in
phase M10, when no row says Not started.

Mac status words:

| Mac status | Meaning |
|---|---|
| Not started | Nothing is built on the Mac for this row. |
| Built | The Mac code exists and passes its own tests. |
| Proved on Apple silicon at level 2 | The Mac scenario or test passes on the debug build on an Apple silicon Mac ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). Evidence names the test. |
| Proved on the Mac release build | The same scenario passes on the release \`.dmg\` build. Evidence names the file in \`docs/release/evidence/\`. |
| Not applicable on the Mac | The row cannot apply to the Mac. The reason follows the status in the same cell. |

Counts: ${featureRows.length} feature rows (${naFeatures} not applicable) and ${scenarioRows.length} scenario files holding ${scenarioItems} scenarios (${naScenarios} files not applicable).

## Table 1: Windows Must-ship features

| Feature | Area | Windows status and proof | Mac phase | Mac status | Mac evidence |
|---|---|---|---|---|---|`);
for (const r of featureRows) {
  out.push(`| ${cell(r.name)} | ${cell(r.area)} | ${cell(`${r.status}, proof ${r.proof}`)} | ${r.phase} | ${cell(r.macStatus)} | ${cell(r.evidence)} |`);
}
out.push(`
## Table 2: Windows end-to-end scenarios

The scenario count is the number of second-level headings in the file, one per scenario.

| File | Title | Mac phase | Mac status | Mac test |
|---|---|---|---|---|`);
for (const r of scenarioRows) {
  out.push(`| \`${r.file}\` | ${cell(`${r.title} (${r.items} scenarios)`)} | ${r.phase} | ${cell(r.macStatus)} | ${cell(r.test)} |`);
}
writeFileSync(OUT, out.join("\n") + "\n");
console.log(`mac-parity: ${featureRows.length} features (${naFeatures} n/a), ${scenarioRows.length} scenario files (${naScenarios} n/a) -> ${OUT.replace(root + "/", "")}`);
