// Shared reader for docs/implementation/feature-status.md (used by scripts/check-reachability.mjs
// and scripts/check-feature-status.mjs). Returns the feature rows of the 10-column tables.

import fs from "node:fs";
import path from "node:path";

export const FEATURE_STATUS = "docs/implementation/feature-status.md";
export const MASTERPLAN = "docs/implementation/masterplan.md";
export const HEADER = ["Feature", "Importance", "What exists today", "What is missing", "Build area", "How a user reaches it", "Tests", "Status", "Proof", "Key"];
export const DOC_HEADER = ["Feature", "Build area", "Status", "Key"];

export const IMPORTANCE = ["Must ship", "Should ship", "Can wait", "Removed"];
export const STATUSES = ["Specified", "Designed", "Built", "Reachable", "Verified", "Release-ready"];
/** Position of a status word in the scale; "Not applicable" and "Process" sit outside it. */
export const statusLevel = (status) => STATUSES.indexOf(status);

/**
 * Proof levels of ADR-0030: 1 built, 2 proved on development, 3 proved on the release build, 4 proved on a clean
 * computer, 5 proved by people. A Proof cell is `none`, a level, or a level with its evidence: `3 (ev: <path>)`.
 * Returns { level, evidence } (level 0 for `none`), or null when the cell is malformed.
 */
export function parseProof(cell) {
  if (cell === "none") return { level: 0, evidence: null };
  const m = /^([1-5])(?: \(ev: ([^)]+)\))?$/.exec(cell);
  return m ? { level: Number(m[1]), evidence: m[2] ?? null } : null;
}
/** "Verified" means level 2 until the Windows track reaches phase W9, then level 3 (ADR-0030 decision 1). */
export const VERIFIED_MIN_PROOF = 2;
/** An end-to-end test reference: a journey (J:) or restart (R:) title, or a spec file. */
export const citesEndToEnd = (tests) => /(^|[\s;])(J|R):|\.test\.mjs\b/.test(tests);

export const splitRow = (raw) =>
  raw.trim().replace(/^\|/, "").replace(/\|$/, "").split(/(?<!\\)\|/).map((c) => c.trim());

/** Build areas are the numbered `## N. Title` headings of the build plan, plus "Every area". */
export function readBuildAreas(root) {
  const file = path.join(root, MASTERPLAN);
  if (!fs.existsSync(file)) return null;
  const areas = new Set(["Every area"]);
  for (const line of fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n").split("\n")) {
    const m = /^## \d+\. (.+)$/.exec(line);
    if (m) areas.add(m[1].trim());
  }
  return areas;
}

export function readFeatureStatus(root) {
  const text = fs.readFileSync(path.join(root, FEATURE_STATUS), "utf8").replace(/\r\n/g, "\n");
  const rows = [];
  let inTable = false;
  text.split("\n").forEach((raw, i) => {
    if (!raw.startsWith("|")) {
      inTable = false;
      return;
    }
    const cells = splitRow(raw);
    if (cells[0] === "Feature") {
      inTable = JSON.stringify(cells) === JSON.stringify(HEADER);
      return;
    }
    if (!inTable || /^-+$/.test(cells[0].replace(/:/g, ""))) return;
    rows.push({
      line: i + 1,
      feature: cells[0],
      importance: cells[1],
      area: cells[4],
      route: cells[5],
      tests: cells[6],
      status: cells[7],
      proof: cells[8],
      key: cells[9],
    });
  });
  return rows;
}

/** Shell entry points that exist on every screen (not workspaces). */
export const SHELL_ENTRY_POINTS = ["app-home", "ai-panel", "shell", "project-menu", "search-overlay"];

/** First token of each `;`-separated part of a route cell (`story/outline (tab)` gives `story/outline`). */
export function routeTokens(cell) {
  return cell
    .split(";")
    .map((p) => p.trim().split(/[\s(]/)[0])
    .filter(Boolean);
}

/** Rows a normal user must be able to reach (Reachable and above). */
export const isReachableStatus = (status) => statusLevel(status) >= statusLevel("Reachable");
