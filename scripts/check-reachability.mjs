#!/usr/bin/env node
// Reachability check generated from the feature status file (build plan item reachability test).
//
//   node scripts/check-reachability.mjs          exit 1 when a reachable row has no valid UI route
//   node scripts/check-reachability.mjs --json   print the canonical routes (used by the E2E spec)
//
// Every feature row at Reachable or above ("a normal user can use it") must name at least one canonical
// entry point in its "How a user reaches it" cell: a workspace route `id` or `id/sub` that exists in the source,
// or a shell entry point (app-home, ai-panel, shell, project-menu, search-overlay). Any token that
// looks like a workspace route must exist. The source of truth for routes is the app itself:
//   workspaces   apps/desktop/src/app/routes.tsx  ({ id: "...", ..., component: load("<dir>") })
//   sub-routes   literals the workspace compares or navigates to (`route.sub === "x"`, `sub: "x"`),
//                Story tab ids, and Production tabs (`export const tab = { id: "..." }`)
// tests/e2e/specs/95-reachability.test.mjs then clicks to every canonical route in the real app.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import { readFeatureStatus, routeTokens, isReachableStatus, SHELL_ENTRY_POINTS } from "./lib/feature-status.mjs";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");

export function sourceRoutes(root = ROOT) {
  const src = path.join(root, "apps/desktop/src");
  const routesTsx = fs.readFileSync(path.join(src, "app/routes.tsx"), "utf8");
  const workspaces = new Map(); // id -> dir
  for (const m of routesTsx.matchAll(/\{\s*id:\s*"([a-z-]+)"[^}]*?component:\s*load\("([a-z-]+)"\)/g)) workspaces.set(m[1], m[2]);
  const subs = new Map();
  for (const [id, dir] of workspaces) {
    const set = new Set();
    const files = [];
    const walk = (d) => {
      for (const e of fs.existsSync(d) ? fs.readdirSync(d, { withFileTypes: true }) : []) {
        if (e.isDirectory()) walk(path.join(d, e.name));
        else if (/\.tsx?$/.test(e.name) && !/\.test\.tsx?$/.test(e.name)) files.push(path.join(d, e.name));
      }
    };
    walk(path.join(src, "workspaces", dir));
    for (const f of files) {
      const t = fs.readFileSync(f, "utf8");
      for (const m of t.matchAll(/route\.sub\s*===\s*"([a-z-]+)"/g)) set.add(m[1]);
      for (const m of t.matchAll(/workspace:\s*"([a-z-]+)",\s*sub:\s*"([a-z-]+)"/g)) if (m[1] === id) set.add(m[2]);
      for (const m of t.matchAll(/export const tab\s*=\s*\{\s*id:\s*"([a-z-]+)"/g)) set.add(m[1]);
      if (id === "story") for (const m of t.matchAll(/\{\s*id:\s*"([a-z-]+)",\s*label:/g)) set.add(m[1]);
      // Tab unions such as `type Tab = "board" | "outline" | "episodes";`.
      for (const m of t.matchAll(/type\s+Tab\s*=\s*([^;]+);/g)) for (const lit of m[1].matchAll(/"([a-z-]+)"/g)) set.add(lit[1]);
    }
    subs.set(id, set);
  }
  return { workspaces, subs };
}

export function checkRoutes(root = ROOT) {
  const { workspaces, subs } = sourceRoutes(root);
  const errors = [];
  const canonical = new Set();
  for (const row of readFeatureStatus(root)) {
    if (!isReachableStatus(row.status)) continue;
    let valid = 0;
    for (const tok of routeTokens(row.route)) {
      if (SHELL_ENTRY_POINTS.includes(tok)) {
        valid++;
        canonical.add(tok);
        continue;
      }
      const m = /^([a-z-]+)(?:\/([a-z-]+))?$/.exec(tok);
      if (!m || !workspaces.has(m[1])) continue; // free-text detail, not a route token
      if (m[2] && !subs.get(m[1]).has(m[2])) {
        errors.push(`${row.key} (line ${row.line}): route ${tok} does not exist; known ${m[1]} sub-routes: ${[...subs.get(m[1])].sort().join(", ")}`);
        continue;
      }
      valid++;
      canonical.add(tok);
    }
    if (valid === 0) {
      errors.push(`${row.key} (line ${row.line}): ${row.status} but the route "${row.route}" names no workspace route or shell entry point (${SHELL_ENTRY_POINTS.join(", ")})`);
    }
  }
  return { errors, canonical: [...canonical].sort() };
}

if (process.argv[1] && path.resolve(process.argv[1]) === url.fileURLToPath(import.meta.url)) {
  const { errors, canonical } = checkRoutes();
  if (process.argv.includes("--json")) {
    console.log(JSON.stringify(canonical));
  } else if (errors.length) {
    console.error(`check-reachability: ${errors.length} problem(s)`);
    for (const e of errors) console.error(`  ${e}`);
    process.exit(1);
  } else {
    console.log(`check-reachability: OK (${canonical.length} canonical routes from the feature status file)`);
  }
}
