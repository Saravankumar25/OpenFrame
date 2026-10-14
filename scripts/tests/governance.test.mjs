// Self-tests for the governance checks (architecture lint, migration policy, feature status check, reachability).
// Each test copies the files a check reads into a temporary tree, plants one violation and asserts
// that the check fails with the expected rule, so a check that silently stops working is caught.
//
//   node --test scripts/tests/

import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import url from "node:url";

const REPO = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "../..");
const script = (name) => path.join(REPO, "scripts", name);

function fixture(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-governance-"));
  for (const rel of files) {
    const dest = path.join(root, rel);
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.cpSync(path.join(REPO, rel), dest, { recursive: true });
  }
  return root;
}

function run(name, root, args = []) {
  const r = spawnSync(process.execPath, [script(name), ...args], {
    encoding: "utf8",
    env: { ...process.env, OPENFRAME_CHECK_ROOT: root },
  });
  return { status: r.status, out: `${r.stdout}${r.stderr}` };
}

const write = (root, rel, text) => {
  const p = path.join(root, rel);
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, text);
};
const edit = (root, rel, fn) => {
  const p = path.join(root, rel);
  fs.writeFileSync(p, fn(fs.readFileSync(p, "utf8")));
};

// ---------------------------------------------------------------------------- architecture lint

const ARCH_FILES = [
  "crates/openframe-application/src/modules/ai/toolbox.rs",
  "crates/openframe-ai/Cargo.toml",
  "crates/openframe-ai/src/supervisor.rs",
  "apps/desktop/src-tauri/tauri.conf.json",
  "apps/desktop/src-tauri/Cargo.toml",
  "apps/desktop/src-tauri/src/lib.rs",
  "apps/desktop/src-tauri/capabilities",
  "apps/desktop/src/ipc/client.ts",
];

test("architecture lint passes on the repository", () => {
  const r = run("architecture-lint.mjs", REPO);
  assert.equal(r.status, 0, r.out);
});

test("architecture lint passes on the minimal fixture", () => {
  const r = run("architecture-lint.mjs", fixture(ARCH_FILES));
  assert.equal(r.status, 0, r.out);
});

const archCases = [
  ["W1", "apps/desktop/src/workspaces/x/index.tsx", 'import { readFile } from "@tauri-apps/plugin-fs";\n'],
  ["W1", "apps/desktop/src/workspaces/x/index.tsx", 'const m = await import("node:child_process");\n'],
  ["W2", "apps/desktop/src/workspaces/x/index.tsx", 'import { invoke } from "@tauri-apps/api/core";\n'],
  ["W3", "apps/desktop/src/workspaces/x/index.tsx", 'export const f = () => fetch("https://example.com");\n'],
  ["W4", "apps/desktop/src/workspaces/x/index.tsx", "export const f = (h: string) => <div dangerouslySetInnerHTML={{ __html: h }} />;\n"],
  ["W4", "apps/desktop/src/workspaces/x/util.ts", "export const g = (e: HTMLElement, h: string) => { e.innerHTML = h; };\n"],
  // Bypasses found by the 2026-10-05 security review of the lint itself.
  ["W1", "apps/desktop/src/workspaces/x/index.tsx", 'import { openUrl } from "@tauri-apps/plugin-opener";\n'],
  ["W2", "apps/desktop/src/workspaces/x/index.tsx", 'import * as core from "@tauri-apps/api/core";\ncore.invoke("x");\n'],
  ["W2", "apps/desktop/src/workspaces/x/index.tsx", 'import { core } from "@tauri-apps/api";\n'],
  ["W2", "apps/desktop/src/workspaces/x/index.tsx", 'import { listen } from "@tauri-apps/api/event";\n'],
  ["W3", "apps/desktop/src/workspaces/x/index.tsx", "const f = fetch;\nexport const g = () => f('x');\n"],
  ["W4", "apps/desktop/src/workspaces/x/index.tsx", "export const g = (s: string) => window.eval(s);\n"],
  ["W4", "apps/desktop/src/workspaces/x/index.tsx", 'export const g = Function("return 1");\n'],
  ["R5", "crates/openframe-application/src/x.rs", 'use std::process::Command as C;\nfn f() { C::new("cmd"); }\n'],
  ["R5", "crates/openframe-application/src/x.rs", "fn f() { unsafe { ShellExecuteExW(std::ptr::null_mut()); } }\n"],
  ["R1", "crates/openframe-application/src/x.rs", 'use rusqlite::Connection as Db;\nfn f() { let _ = Db::open("x"); }\n'],
  ["R1", "crates/openframe-application/src/modules/story/x.rs", 'fn f(c: &rusqlite::Connection) { c.execute_batch("BEGIN IMMEDIATE").unwrap(); }\n'],
  ["R2", "crates/openframe-application/src/modules/ai/agent_x.rs", "fn f() { crate::modules::ai::change_set::accept(core, actor, id); }\n"],
  ["R3", "crates/openframe-application/src/modules/x.rs", 'fn f() { std::net::TcpListener::bind("0.0.0.0:80").unwrap(); }\n'],
  ["R4", "crates/openframe-domain/src/x.rs", "fn f() { unsafe { std::hint::unreachable_unchecked() } }\n"],
  ["R5", "crates/openframe-application/src/x.rs", 'fn f() { std::process::Command::new("cmd").spawn().unwrap(); }\n'],
  ["R6", "crates/openframe-application/src/modules/x.rs", "#[cfg(windows)]\nfn f() {}\n"],
  ["R6", "crates/openframe-application/src/modules/x.rs", "fn f() { let _ = windows::Win32::Foundation::HANDLE::default(); }\n"],
  ["R6", "crates/openframe-project-format/src/x.rs", "use std::os::windows::ffi::OsStrExt;\nfn f() {}\n"],
  ["R6", "crates/openframe-application/src/x.rs", '#[cfg(target_os = "macos")]\nfn f() {}\n'],
  ["R6", "crates/openframe-application/src/x.rs", "fn f() -> bool { cfg!(windows) }\n"],
  ["R6", "crates/openframe-application/src/x.rs", '#[cfg(all(windows, target_env = "msvc"))]\nfn f() {}\n'],
  ["R6", "crates/openframe-application/src/x.rs", "fn f() -> &'static str { std::env::consts::OS }\n"],
  ["R6", "crates/openframe-application/src/x.rs", "#[cfg_attr(windows, derive(Debug))]\nstruct S;\n"],
  ["R6", "crates/openframe-application/src/x.rs", '#[link(name = "kernel32")]\nextern "system" {}\n'],
  ["R6", "apps/desktop/src-tauri/build.rs", 'fn main() { if cfg!(target_os = "windows") {} }\n'],
  // Theme colours (ADR-0014): literals need a token or a written classification.
  ["T1", "apps/desktop/src/workspaces/x/x.css", ".card { background: #fff; }\n"],
  ["T1", "apps/desktop/src/workspaces/x/x.css", ".card { color: rgba(10, 20, 30, .5); }\n"],
  ["T1", "apps/desktop/src/workspaces/x/x.css", ".card { box-shadow: 0 0 0 3px #d9822b33; }\n"],
  // A tinted shadow is a token or marked, never allowed as if it were neutral.
  ["T1", "apps/desktop/src/workspaces/x/x.css", ".drawer { box-shadow: -8px 0 26px rgba(20,24,38,.16); }\n"],
  ["T1", "apps/desktop/src/workspaces/x/index.tsx", 'export const s = { background: "#faf9f6" };\n'],
];

test("architecture lint T1 accepts tokens, marked fixed or document colours and neutral shadows", () => {
  const root = fixture(ARCH_FILES);
  write(
    root,
    "apps/desktop/src/workspaces/x/x.css",
    [
      ".a { background: var(--panel); color: var(--ink); }",
      ".b { background: #fff3b0; } /* fixed colour: stripboard day and night convention */",
      "/* document colour: printed call-sheet paper */",
      ".c { color: #111; }",
      ".d { box-shadow: 0 2px 12px #0003, 0 1px 3px rgb(0 0 0/.12), var(--shadow-side); }",
      "",
    ].join("\n"),
  );
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 0, r.out);
});

for (const [rule, file, body] of archCases) {
  test(`architecture lint rule ${rule} catches ${path.basename(file)}: ${body.trim().slice(0, 50)}`, () => {
    const root = fixture(ARCH_FILES);
    write(root, file, body);
    const r = run("architecture-lint.mjs", root);
    assert.equal(r.status, 1, r.out);
    assert.match(r.out, new RegExp(`\\b${rule} `));
  });
}

test("architecture lint ignores patterns in comments, strings and test modules", () => {
  const root = fixture(ARCH_FILES);
  write(
    root,
    "crates/openframe-application/src/modules/x.rs",
    [
      "// unsafe { } and Command::new in a comment",
      // SQL patterns (R1) are matched inside strings on purpose: SQL only ever appears in strings.
      'const MSG: &str = "an unsafe path; Command::new; TcpListener";',
      "#[cfg(test)]",
      "mod tests {",
      '    fn t() { std::net::TcpListener::bind("127.0.0.1:0").unwrap(); let _ = "}"; }',
      "}",
      // R6 ignores OS-specific code in comments and in test modules.
      "// #[cfg(windows)] and std::os::windows::ffi in a comment",
      "#[cfg(test)]",
      "mod os_tests {",
      "    #[cfg(windows)]",
      "    fn t() { let _ = windows::Win32::Foundation::HANDLE::default(); }",
      "}",
      "",
    ].join("\n"),
  );
  write(root, "apps/desktop/src/workspaces/x/index.tsx", '// fetch("x")\nconst s = "new WebSocket(";\nq.refetch();\n');
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 0, r.out);
});

test("architecture lint catches a weakened CSP and an extra Tauri command", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/tauri.conf.json", (s) => s.replace("script-src 'self'", "script-src 'self' 'unsafe-eval'"));
  edit(root, "apps/desktop/src-tauri/src/lib.rs", (s) => s.replace("of_open_url]", "of_open_url, of_run_shell]"));
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C2 .*unsafe-eval/);
  assert.match(r.out, /C3 /);
});

test("architecture lint rule C5 catches the project cache folder in the asset scope", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/src/lib.rs", (s) =>
    s.replace(
      "scope.allow_directory(p.layout.assets(), true);",
      "let _ = scope.allow_directory(p.layout.assets(), true);\n        let _ = scope.allow_directory(p.layout.cache(), true);",
    ),
  );
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C5 .*cache/);
});

test("architecture lint rule C5 catches a path chained after assets()", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/src/lib.rs", (s) =>
    s.replace("scope.allow_directory(p.layout.assets(), true);", "scope.allow_directory(p.layout.assets().parent().unwrap(), true);"),
  );
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C5 /);
});

test("architecture lint rule C5 catches the built-in asset protocol switched back on", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/tauri.conf.json", (s) => s.replace('"enable": false', '"enable": true'));
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C5 .*assetProtocol\.enable/);
});

test("architecture lint catches a static asset scope, a remote capability and a table-form plugin", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/tauri.conf.json", (s) => s.replace(/"scope":\s*\[\s*\]/, '"scope": ["$APPDATA/**"]'));
  edit(root, "apps/desktop/src-tauri/capabilities/default.json", (s) => s.replace('"windows": ["main"],', '"windows": ["main"],\n  "remote": { "urls": ["https://example.com"] },'));
  edit(root, "apps/desktop/src-tauri/Cargo.toml", (s) => s + '\n[dependencies.tauri-plugin-shell]\nversion = "2"\n');
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C5 .*assetProtocol\.scope/);
  assert.match(r.out, /C1 .*remote/);
  assert.match(r.out, /C4 .*tauri-plugin-shell/);
});

test("architecture lint catches a filesystem capability for the webview", () => {
  const root = fixture(ARCH_FILES);
  edit(root, "apps/desktop/src-tauri/capabilities/default.json", (s) => s.replace('"dialog:allow-open",', '"dialog:allow-open",\n    "fs:allow-read-file",'));
  const r = run("architecture-lint.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /C1 /);
});

// ---------------------------------------------------------------------------- migrations

const MIG_FILES = ["migrations", "crates/openframe-application/src/schema.rs"];

test("migration check passes on the repository", () => {
  const r = run("check-migrations.mjs", fixture(MIG_FILES));
  assert.equal(r.status, 0, r.out);
});

test("migration check catches a gap and a duplicate number", () => {
  const root = fixture(MIG_FILES);
  // Far past the latest migration, so this stays a gap as migrations are added.
  write(root, "migrations/project/0099_gap.sql", "SELECT 1;\n");
  write(root, "migrations/global/0003_duplicate.sql", "SELECT 1;\n");
  const r = run("check-migrations.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /no gap/);
  assert.match(r.out, /used twice/);
});

test("migration check catches a file the app does not embed", () => {
  const root = fixture(MIG_FILES);
  const n = fs.readdirSync(path.join(root, "migrations/project")).filter((f) => f.endsWith(".sql")).length;
  write(root, `migrations/project/${String(n + 1).padStart(4, "0")}_not_embedded.sql`, "SELECT 1;\n");
  const r = run("check-migrations.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /schema\.rs: project migrations/);
});

test("migration check: edits warn before the freeze and fail after it", () => {
  const root = fixture(MIG_FILES);
  edit(root, "migrations/project/0001_core.sql", (s) => s + "\n-- edited\n");
  const before = run("check-migrations.mjs", root);
  assert.equal(before.status, 0, before.out);
  assert.match(before.out, /differs from the recorded checksum/);
  edit(root, "migrations/frozen.json", (s) => s.replace('"frozen": false', '"frozen": true'));
  const after = run("check-migrations.mjs", root);
  assert.equal(after.status, 1, after.out);
  assert.match(after.out, /edited after the freeze/);
});

test("migration check --base refuses turning the freeze off or dropping a record in the same change", () => {
  const root = fixture(MIG_FILES);
  const git = (...a) => spawnSync("git", a, { cwd: root, encoding: "utf8" });
  edit(root, "migrations/frozen.json", (s) => s.replace('"frozen": false', '"frozen": true'));
  git("init", "-q");
  git("-c", "user.email=ci@example.invalid", "-c", "user.name=ci", "add", "-A");
  git("-c", "user.email=ci@example.invalid", "-c", "user.name=ci", "commit", "-q", "-m", "base");
  edit(root, "migrations/frozen.json", (s) =>
    s.replace('"frozen": true', '"frozen": false').replace(/\s*"app\/0001_app\.sql": "[0-9a-f]+",?/, ""),
  );
  const r = run("check-migrations.mjs", root, ["--base", "HEAD"]);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /cannot be turned off/);
  assert.match(r.out, /removed from migrations\/frozen\.json/);
});

test("migration check catches a deleted migration", () => {
  const root = fixture(MIG_FILES);
  fs.rmSync(path.join(root, "migrations/app/0001_app.sql"));
  const r = run("check-migrations.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /missing on disk/);
});

// ---------------------------------------------------------------------------- feature status

const STATUS_FILE = "docs/implementation/feature-status.md";
const PLAN_FILE = "docs/implementation/masterplan.md";

test("feature status check passes on the repository", () => {
  const r = run("check-feature-status.mjs", REPO);
  assert.equal(r.status, 0, r.out);
});

const ROW_HEAD =
  "| Feature | Importance | What exists today | What is missing | Build area | How a user reaches it | Tests | Status | Key |\n|---|---|---|---|---|---|---|---|---|\n";
const statusFileWith = (rows, source = "Source: FSD §1.\n\n") =>
  `# Feature status\n\n## Project foundation\n\n${source}${ROW_HEAD}${rows.join("\n")}\n`;
const PLAN = "# Plan\n\n## 1. Data safety and crash recovery\n\n## 2. Writing comfort\n";
const AREA = "Data safety and crash recovery";

const statusCases = [
  ["Built without tests", `| Thing | Must ship | Built | none | ${AREA} | home | pending | Built | CORE-001 |`, /needs passing tests/],
  ["Reachable without a route", `| Thing | Must ship | Built | none | ${AREA} | pending | core.rs::t | Reachable | CORE-001 |`, /means reachable/],
  ["an unverified tag", `| Thing | Must ship | Built [R] | none | ${AREA} | home | core.rs::t | Built | CORE-001 |`, /unverified/],
  ["an unknown build area", "| Thing | Must ship | Built | none | Made up area | home | core.rs::t | Built | CORE-001 |", /Build area "Made up area"/],
  ["an invalid status", `| Thing | Must ship | Built | none | ${AREA} | home | core.rs::t | done | CORE-001 |`, /Status "done"/],
  ["an invalid importance", `| Thing | Urgent | Built | none | ${AREA} | home | core.rs::t | Built | CORE-001 |`, /Importance "Urgent"/],
  ["an empty route", `| Thing | Must ship | Built | none | ${AREA} |  | core.rs::t | Designed | CORE-001 |`, /How a user reaches it is empty/],
];
for (const [label, row, re] of statusCases) {
  test(`feature status check catches ${label}`, () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-status-"));
    write(root, PLAN_FILE, PLAN);
    write(root, STATUS_FILE, statusFileWith([row]));
    const r = run("check-feature-status.mjs", root);
    assert.equal(r.status, 1, r.out);
    assert.match(r.out, re);
  });
}

test("feature status check catches a duplicate key and a section without Source", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-status-"));
  const row = `| Thing | Must ship | Built | none | ${AREA} | home | core.rs::t | Built | CORE-001 |`;
  write(root, STATUS_FILE, statusFileWith([row, row], ""));
  const r = run("check-feature-status.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /duplicate Key/);
  assert.match(r.out, /no `Source:` line/);
});

test("feature status check accepts a complete row and a removed row", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-status-"));
  write(root, PLAN_FILE, PLAN);
  write(
    root,
    STATUS_FILE,
    statusFileWith([
      `| Thing | Must ship | Built | none | ${AREA} | home | core.rs::t | Reachable | CORE-001 |`,
      "| Gone | Removed | Removed by ADR | none | Every area | none: removed | none | Not applicable | CORE-002 |",
    ]),
  );
  const r = run("check-feature-status.mjs", root);
  assert.equal(r.status, 0, r.out);
});

// ---------------------------------------------------------------------------- reachability

const REACH_FILES = ["apps/desktop/src/app/routes.tsx", "apps/desktop/src/workspaces", STATUS_FILE];

test("reachability check passes on the repository", () => {
  const r = run("check-reachability.mjs", REPO);
  assert.equal(r.status, 0, r.out);
});

test("reachability check catches a route that does not exist and a reachable row without a route", () => {
  const root = fixture(REACH_FILES);
  edit(root, STATUS_FILE, (s) =>
    s
      .replace("story; story/outline", "story; story/nonexistent")
      .replace("| search-overlay (Ctrl+K) |", "| somewhere in the app |"),
  );
  const r = run("check-reachability.mjs", root);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /story\/nonexistent does not exist/);
  assert.match(r.out, /names no workspace route or shell entry point/);
});

// ---------------------------------------------------------------------------- plan status

const PLAN_FIXTURE = `# Plan

## 1. First area

- [x] **Done now.** (ev: test)
- [ ] **Open now.**
- [x] **Done for the stores.** (ev: test) (Later: store publishing)
- [ ] **Open for the stores.** (Later: store publishing) (Needs Mac hardware)
- [ ] **Open on a Mac.** (Needs Mac hardware)

## Release sign-off

- [ ] Not counted (Later: store publishing)
`;

test("plan status passes its check on the repository and prints the current goal", () => {
  const r = run("plan-status.mjs", REPO, ["--check"]);
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /^Overall: \d+ of \d+ \(/m);
  assert.match(r.out, /^Current goal: \d+ of \d+ \(/m);
  assert.match(r.out, /^Later: store publishing: \d+ boxes$/m);
});

test("plan status leaves boxes tagged for store publishing out of the current goal", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-governance-"));
  write(root, PLAN_FILE, PLAN_FIXTURE);
  const r = run("plan-status.mjs", root);
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /^ 1\. First area: 2 of 5$/m);
  assert.match(r.out, /^Overall: 2 of 5 \(40%\)$/m);
  assert.match(r.out, /^Current goal: 1 of 3 \(33\.3%\)$/m);
  assert.match(r.out, /^Later: store publishing: 2 boxes$/m);
  const json = JSON.parse(run("plan-status.mjs", root, ["--json"]).out);
  assert.deepEqual(json.currentGoal, { done: 1, total: 3, percent: 33.3 });
  assert.equal(json.laterStorePublishing, 2);
});

test("plan status --check catches a checked box without evidence, tagged or not", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "of-governance-"));
  write(root, PLAN_FILE, PLAN_FIXTURE.replace("(ev: test) (Later", "(Later"));
  const r = run("plan-status.mjs", root, ["--check"]);
  assert.equal(r.status, 1, r.out);
  assert.match(r.out, /masterplan\.md:7 {2}checked box has no \(ev: \.\.\.\) evidence/);
});
