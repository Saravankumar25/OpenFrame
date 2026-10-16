#!/usr/bin/env node
// Architecture lint for OpenFrame Studio (build plan item architecture lint, feature PLAT-004).
//
//   node scripts/architecture-lint.mjs          exit 1 on any violation
//   node scripts/architecture-lint.mjs --list   also print every allow-listed site that was found
//
// Enforces the protected invariants in CONTRIBUTING.md §2 and the Windows security
// standard in docs/implementation/masterplan.md §1a mechanically, so a change that
// bypasses them fails CI instead of relying on review:
//
//   W1 webview imports    no filesystem, shell, HTTP, SQL, process or Node modules in the React app
//   W2 single IPC         Tauri `invoke` only in src/ipc/client.ts; no global Tauri object
//   W3 no webview network no fetch / WebSocket / XMLHttpRequest / EventSource / sendBeacon in the React app
//   W4 injection sinks    no dangerouslySetInnerHTML, innerHTML/outerHTML assignment, eval, new Function, window.open
//   C1 capabilities       the window capability grants no fs/shell/http/sql/process/updater permission
//   C2 CSP                strict CSP present (no unsafe-eval, no inline script, IPC-only connect-src)
//   C3 command allow-list the Tauri command list equals the reviewed allow-list
//   C4 desktop plugins    no fs/shell/http/sql/process Tauri plugin crate in the desktop app
//   C5 asset scope        the asset protocol may read managed `assets/` folders only (never cache/, databases)
//   R1 SQLite writes      write transactions and raw connections only in reviewed files
//   R2 AI boundary        the agent, toolbox and retrieval code never accepts or applies a Change Set;
//                         the toolbox keeps its `ai.*` guard; the AI runtime crate has no project-data crate
//   R3 no listeners       no network listener except the reviewed loopback port probe
//   R4 unsafe Rust        `unsafe` only in reviewed files, each with a written reason
//   R5 process spawn      child processes only from the AI sidecar supervisor
//   R6 platform boundary  operating-system specific code (cfg(windows), cfg(target_os), std::os::*, the
//                         windows and libc crates) only in openframe-platform and the desktop shell adapters
//   T1 theme colours      colour literals in the React app only as marked fixed or document colours
//                         (every themeable colour is a token from the generated theme layer; ADR-0014)
//
// Binding drift and AI-coverage drift are checked in CI with `git diff --exit-code` after
// `cargo test` regenerates them (.github/workflows/ci.yml). Every allow-list entry below
// carries the reason it was accepted; adding one is a reviewed, V5 security change.

import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import { walk, rustViews, rustTestRanges, tsViews, lineOf, inRanges } from "./lib/source-scan.mjs";

const ROOT = process.env.OPENFRAME_CHECK_ROOT
  ? path.resolve(process.env.OPENFRAME_CHECK_ROOT)
  : path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), "..");
const LIST = process.argv.includes("--list");
const rel = (p) => path.relative(ROOT, p).split(path.sep).join("/");
const read = (p) => fs.readFileSync(path.join(ROOT, p), "utf8").replace(/\r\n/g, "\n");

const violations = [];
const allowedHits = [];
const fail = (rule, file, line, msg) => violations.push(`${rule} ${file}${line ? `:${line}` : ""}  ${msg}`);

// ---------------------------------------------------------------------------
// Allow-lists (reviewed exceptions). Paths are repository-relative, "/" separated.
// A trailing "/" means the whole directory.
// ---------------------------------------------------------------------------

const SQL_WRITE_ALLOW = {
  "crates/openframe-persistence/": "Persistence layer: connections, migrations, undo triggers and checkpoints.",
  "crates/openframe-application/src/store.rs": "Store::mutate, the single canonical write path (BEGIN IMMEDIATE).",
  "crates/openframe-search/": "Derived FTS5 and vector index, rebuildable from canonical data.",
  "crates/openframe-application/src/modules/ai/intelligence/": "Derived retrieval intelligence index (doc 18), rebuildable.",
  "crates/openframe-application/src/modules/ai/change_set.rs":
    "Groups the undo steps of an applied Change Set into one labelled step (sys_undo bookkeeping only, no canonical rows).",
  "crates/openframe-application/src/modules/project.rs":
    "Operations on a project that is not open (rename, duplicate), under ProjectLock, with an Activity row.",
  "crates/openframe-application/src/modules/packages/backup.rs":
    "Works on a private copy of the database in staging (scrub before export, restore into a new project).",
  "crates/openframe-application/src/modules/packages/coverage.rs":
    "The Full Project Package scrub, on the private staging copy of the database only (scrub_for_transfer).",
  "crates/openframe-application/src/core.rs":
    "Opens app.sqlite (recent projects, local profile; no project content) and migrates it with the shared runner.",
};

const AI_BOUNDARY_EXEMPT = new Set([
  "crates/openframe-application/src/modules/ai/mod.rs", // registers the human-only accept/reject commands
  "crates/openframe-application/src/modules/ai/change_set.rs", // implements them
]);

const LISTENER_ALLOW = {
  "crates/openframe-ai/src/supervisor.rs":
    "Binds 127.0.0.1:0 only to pick a free loopback port for the sidecar, then drops it (no listener remains).",
};

const UNSAFE_ALLOW = {
  "crates/openframe-platform/src/graphics.rs": "DXGI adapter enumeration (COM) and GetSystemDirectoryW for the graphics-card decision (platform boundary, ADR-0029); SAFETY comments in place.",
  "crates/openframe-platform/src/exists.rs": "GetFileAttributesW fast existence check (platform boundary); SAFETY comment in place.",
  "crates/openframe-platform/src/identity.rs": "GetCurrentPackageFamilyName (platform boundary); SAFETY comment in place.",
  "apps/desktop/src-tauri/src/platform/macos_storekit.rs":
    "C ABI calls into the Swift StoreKit bridge (macOS only); every pointer is owned by the bridge and freed with storekit_free; SAFETY comments in place.",
  "crates/openframe-platform/src/process.rs":
    "Suspended creation, Windows Job Object and thread resume, process start times and the stray-engine stop, macOS process group (platform boundary); SAFETY comments in place.",
  "crates/openframe-platform/src/drives.rs":
    "GetDriveTypeW to refuse mapped network drives (platform boundary); reads a NUL-terminated \"X:\\\" buffer only; SAFETY comment in place.",
  "crates/openframe-platform/src/power.rs":
    "PowerRegisterSuspendResumeNotification callback for sleep and wake (platform boundary); the handler box outlives the registration; SAFETY comments in place.",
  "crates/openframe-platform/src/shell.rs":
    "ShellExecuteW and SHOpenFolderAndSelectItems for open and reveal (platform boundary); NUL-terminated buffers, balanced COM init, item id list freed; SAFETY comments in place.",
  "crates/openframe-search/src/vector.rs": "Registers the sqlite-vec extension entry point with SQLite.",
  "crates/openframe-platform/src/spelling.rs":
    "Windows Spell Checking API (COM) for proofing (platform boundary): one worker thread owns its apartment and every COM object, input is bounded before any call, each returned string is read once and freed once with CoTaskMemFree; SAFETY comments in place.",
  "apps/desktop/src-tauri/src/distribution.rs": "GetCurrentPackageFamilyName to detect the Store (MSIX) channel.",
};

const SPAWN_ALLOW = {
  "crates/openframe-ai/src/supervisor.rs": "Starts the verified llama.cpp sidecar from the managed runtime folder.",
  "crates/openframe-platform/src/process.rs":
    "Configures and guards a child process for the supervisor (no window, kill with the app); spawns only in its own tests.",
  "crates/openframe-platform/src/shell.rs":
    "macOS adapter only: runs /usr/bin/open (open a file or link, or `open -R` to reveal) for an absolute path the desktop shell resolved from an id.",
  "apps/desktop/src-tauri/build.rs":
    "Build script, not the app: compiles and links the Swift StoreKit bridge on macOS only (`swift build`, `swift -print-target-info`); runs at build time on the build machine, never in OpenFrame.",
};

// R6: the only places that may know the operating system (build plan area "Platform adapter layer").
const OS_ALLOW = {
  "crates/openframe-platform/": "The platform boundary crate: Windows adapter and macOS adapter live here.",
  "apps/desktop/src-tauri/src/distribution.rs":
    "Desktop shell: MSIX package data folder redirection and the WebView2 missing-runtime dialog (Windows installer channel).",
  "crates/openframe-ai/src/hardware.rs":
    "DXGI adapter enumeration for the hardware check; to move behind openframe-platform when the Mac (Metal) check is written.",
  "apps/desktop/src-tauri/src/platform/":
    "Desktop shell macOS adapter: the StoreKit 2 bridge over a C ABI, compiled only for target_os = macos; Windows builds get the no-op.",
};

const TAURI_COMMANDS = ["of_invoke", "of_open_asset", "of_reveal_path", "of_open_url", "of_close_window"];

const IPC_CLIENT = "apps/desktop/src/ipc/client.ts";

// ---------------------------------------------------------------------------

const allowed = (map, file) => Object.keys(map).find((k) => (k.endsWith("/") ? file.startsWith(k) : file === k));

function scanRust() {
  const roots = ["crates", "apps"].map((d) => path.join(ROOT, d));
  const files = roots.flatMap((d) => walk(d, (n) => n.endsWith(".rs")));
  for (const abs of files) {
    const file = rel(abs);
    // Test-only code (Cargo integration tests and benches, the test-support crate) may open raw
    // connections, bind sockets and so on. Only the Cargo-defined locations are exempt.
    if (/^crates\/[^/]+\/(tests|benches)\//.test(file) || file.startsWith("crates/openframe-test-support/")) continue;
    const src = fs.readFileSync(abs, "utf8").replace(/\r\n/g, "\n");
    const { noComments, code } = rustViews(src);
    const tests = rustTestRanges(code);
    const each = (view, re, cb) => {
      re.lastIndex = 0;
      let m;
      while ((m = re.exec(view))) if (!inRanges(tests, m.index)) cb(m, lineOf(view, m.index));
    };

    // R1 SQLite writes
    const sqlRe =
      /BEGIN\s+IMMEDIATE|BEGIN\s+EXCLUSIVE|\.transaction\(\)|transaction_with_behavior|unchecked_transaction|Connection::open|open_with_flags|open_connection\(|\bConnection\s+as\s+\w+/g;
    each(noComments, sqlRe, (m, line) => {
      if (allowed(SQL_WRITE_ALLOW, file)) allowedHits.push(`R1 ${file}:${line} ${m[0]}`);
      else
        fail("R1", file, line, `\`${m[0]}\` outside the persistence layer, Store and reviewed derived indexes. Use Registry::command + Store::mutate.`);
    });

    // R2 AI boundary
    if (file.startsWith("crates/openframe-application/src/modules/ai/") && !AI_BOUNDARY_EXEMPT.has(file)) {
      const aiRe = /change_set::accept|change_set::apply|accept_with\s*\(|change_set_accept|"ai\.change_set\.(accept|apply)"/g;
      each(noComments, aiRe, (m, line) =>
        fail(
          "R2",
          file,
          line,
          `AI agent/tool code references \`${m[0]}\`. Only a local user action may accept or apply a Change Set (CONTRIBUTING §2 invariant 2).`,
        ),
      );
    }

    // R3 listeners
    const lisRe = /\b(TcpListener|UdpSocket)\b|\bhyper::server\b|\baxum::serve\b|\bwarp::serve\b|\btiny_http\b/g;
    each(code, lisRe, (m, line) => {
      if (allowed(LISTENER_ALLOW, file)) allowedHits.push(`R3 ${file}:${line} ${m[0]}`);
      else fail("R3", file, line, `network socket \`${m[0]}\`. OpenFrame has no network listener (ADR-0007, the security standard in the build plan).`);
    });

    // R4 unsafe
    each(code, /\bunsafe\b/g, (m, line) => {
      if (allowed(UNSAFE_ALLOW, file)) allowedHits.push(`R4 ${file}:${line} unsafe`);
      else fail("R4", file, line, "`unsafe` Rust without a reviewed reason. Add the file to UNSAFE_ALLOW with the reason, and a SAFETY comment.");
    });

    // R6 platform boundary
    const osRe =
      /\bcfg!?\s*\((?:[^)]*\b)?(?:windows|unix|target_(?:os|family|env|vendor)|macos)\b|\bcfg_attr\s*\(\s*(?:windows|unix|target_os)|\bstd::(?:os::(?:windows|unix|macos|fd|linux)|env::consts::OS)\b|\b(?:windows(?:_sys|_core)?|nix|objc2?|core_foundation|cocoa|winreg|libc|winapi)::|#\[link\(/g;
    each(noComments, osRe, (m, line) => {
      if (allowed(OS_ALLOW, file)) allowedHits.push(`R6 ${file}:${line} ${m[0]}`);
      else
        fail("R6", file, line, `operating-system specific code \`${m[0]}\`. Call openframe-platform instead; feature code never learns which OS it runs on.`);
    });

    // R5 process spawn
    const spawnRe =
      /\bCommand::new\b|\bprocess::\{[^}]*\bCommand\b[^}]*\}|\bprocess::Command\s+as\b|\bCreateProcess\w*\b|\bShellExecute\w*\b|\bWinExec\b/g;
    each(code, spawnRe, (m, line) => {
      if (allowed(SPAWN_ALLOW, file)) allowedHits.push(`R5 ${file}:${line} ${m[0]}`);
      else fail("R5", file, line, `child process \`${m[0]}\`. Only the AI sidecar supervisor may start a process.`);
    });
  }

  // R2 guard sentinels
  const toolbox = "crates/openframe-application/src/modules/ai/toolbox.rs";
  if (!/!\s*op\.starts_with\("ai\."\)/.test(rustViews(read(toolbox)).noComments)) {
    fail("R2", toolbox, 0, 'the toolbox guard `!op.starts_with("ai.")` is missing; no tool may touch an `ai.*` operation.');
  }
  const aiCargo = read("crates/openframe-ai/Cargo.toml");
  for (const forbidden of ["openframe-persistence", "openframe-application", "openframe-search", "rusqlite"]) {
    if (new RegExp(`^\\s*${forbidden}\\b`, "m").test(aiCargo)) {
      fail("R2", "crates/openframe-ai/Cargo.toml", 0, `the AI runtime crate depends on \`${forbidden}\`; the sidecar layer has no project-data authority.`);
    }
  }
  const supervisor = read("crates/openframe-ai/src/supervisor.rs");
  if (/TcpListener::bind/.test(supervisor) && !/TcpListener::bind\(\("127\.0\.0\.1",\s*0\)\)/.test(supervisor)) {
    fail("R3", "crates/openframe-ai/src/supervisor.rs", 0, "the port probe must bind 127.0.0.1:0 only.");
  }
}

function scanWebview() {
  const dir = path.join(ROOT, "apps/desktop/src");
  const files = walk(dir, (n) => /\.(ts|tsx|js|jsx|mts|mjs)$/.test(n));
  // Tauri modules the webview may import (everything else under @tauri-apps is refused), and
  // Node or network modules that must never reach the bundle.
  const allowedTauri = new Set([
    "@tauri-apps/plugin-dialog",
    "@tauri-apps/api/core",
    "@tauri-apps/api/event",
    "@tauri-apps/api/window",
    "@tauri-apps/api/webview",
  ]);
  const forbiddenModule =
    /^(?:node:.*|fs|fs\/promises|path|child_process|net|http|https|os|vm|worker_threads|better-sqlite3|sql\.js|axios|node-fetch|undici|ws|got|superagent)$/;
  const isForbidden = (mod) => (mod.startsWith("@tauri-apps/") ? !allowedTauri.has(mod) : forbiddenModule.test(mod));
  for (const abs of files) {
    const file = rel(abs);
    if (
      file.startsWith("apps/desktop/src/ipc/generated/") ||
      /\.test\.(ts|tsx)$/.test(file) ||
      file.startsWith("apps/desktop/src/test/")
    )
      continue;
    const src = fs.readFileSync(abs, "utf8").replace(/\r\n/g, "\n");
    const { noComments, code } = tsViews(src);

    // W1 imports (static, re-export, dynamic, require, side-effect)
    const impRe = /(?:\bimport\s[^;]*?\bfrom\s*|\bexport\s[^;]*?\bfrom\s*|\bimport\s*\(\s*|\brequire\s*\(\s*|\bimport\s+)(["'])([^"']+)\1/g;
    let m;
    while ((m = impRe.exec(noComments))) {
      if (isForbidden(m[2])) {
        fail(
          "W1",
          file,
          lineOf(noComments, m.index),
          `imports \`${m[2]}\`. The webview has no filesystem, shell, network or database authority; call an application operation through src/ipc/client.ts.`,
        );
      }
    }

    // W2 single IPC entry point
    if (file !== IPC_CLIENT) {
      const ipcMsg = "Tauri IPC outside src/ipc/client.ts. All calls go through the single client (ADR-0004).";
      // Outside the client, @tauri-apps/api/core may supply only convertFileSrc, and the event
      // channel and the bare @tauri-apps/api barrel are not imported at all.
      const coreRe = /\b(?:import|export)\s+([^;]*?)\s+from\s*["']@tauri-apps\/api(\/core)?["']/g;
      while ((m = coreRe.exec(noComments))) {
        const names = /^\{([^}]*)\}$/.exec(m[1].trim());
        const onlyConvert = m[2] && names && names[1].split(",").every((n) => /^\s*(type\s+)?convertFileSrc\s*$/.test(n) || !n.trim());
        if (!onlyConvert) fail("W2", file, lineOf(noComments, m.index), ipcMsg);
      }
      const invRe = /["']@tauri-apps\/api\/event["']|\b__TAURI_INTERNALS__\b|\b__TAURI__\b|\b__TAURI_IPC__\b|\b__TAURI_INTERNALS__/g;
      while ((m = invRe.exec(noComments))) fail("W2", file, lineOf(noComments, m.index), ipcMsg);
    }

    // W4 script-injection sinks (doc 20 T1; security baseline P12). React escapes text; these bypass it.
    const sinkRe =
      /\bdangerouslySetInnerHTML\b|\.(innerHTML|outerHTML)\s*=(?!=)|\binsertAdjacentHTML\b|(?<![\w$])eval\b|(?<![\w$.])Function\s*\(|\bwindow\.open\s*\(|\bdocument\.write(ln)?\s*\(/g;
    while ((m = sinkRe.exec(code))) {
      fail("W4", file, lineOf(code, m.index), `\`${m[0].trim()}\` bypasses React escaping or opens a window; render text, and open links through ipc/client.ts.`);
    }

    // W3 network primitives
    const netRe =
      /(?<![\w$])fetch\b(?!\s*:)|\bWebSocket\b|\bXMLHttpRequest\b|\bEventSource\b|\bsendBeacon\b|\bRTCPeerConnection\b|\bnew\s+(Shared)?Worker\b|\bnew\s+Request\b/g;
    while ((m = netRe.exec(code))) {
      fail(
        "W3",
        file,
        lineOf(code, m.index),
        `network call \`${m[0].trim()}\` in the webview. No silent network use; downloads run in Rust with user approval.`,
      );
    }
  }
}

function scanConfig() {
  // C1 capabilities
  const capDir = path.join(ROOT, "apps/desktop/src-tauri/capabilities");
  for (const abs of walk(capDir, (n) => n.endsWith(".json"))) {
    const file = rel(abs);
    const cap = JSON.parse(fs.readFileSync(abs, "utf8"));
    if (cap.remote) fail("C1", file, 0, "capability has a `remote` section; no remote origin may reach IPC.");
    for (const p of cap.permissions ?? []) {
      const id = typeof p === "string" ? p : p.identifier;
      if (/^(fs|shell|http|sql|process|updater|upload|websocket|store|opener|persisted-scope):/.test(id)) {
        fail("C1", file, 0, `capability grants \`${id}\` to the webview. Least privilege: these run in Rust only.`);
      }
    }
  }

  // C2 CSP
  const confFile = "apps/desktop/src-tauri/tauri.conf.json";
  const conf = JSON.parse(read(confFile));
  const sec = conf.app?.security ?? {};
  const csp = typeof sec.csp === "string" ? sec.csp : null;
  if (!csp) fail("C2", confFile, 0, "no Content-Security-Policy string.");
  else {
    const dir = Object.fromEntries(
      csp
        .split(";")
        .map((d) => d.trim())
        .filter(Boolean)
        .map((d) => {
          const [k, ...v] = d.split(/\s+/);
          return [k, v];
        }),
    );
    const scriptSrc = dir["script-src"] ?? dir["default-src"];
    if (!scriptSrc) fail("C2", confFile, 0, "CSP has neither script-src nor default-src.");
    for (const bad of ["'unsafe-eval'", "'unsafe-inline'", "*", "http:", "https:", "data:", "blob:"]) {
      if ((scriptSrc ?? []).includes(bad)) fail("C2", confFile, 0, `script-src allows ${bad}.`);
    }
    for (const k of ["img-src", "media-src", "font-src", "style-src", "default-src"]) {
      for (const bad of ["*", "http:", "https:"]) {
        if ((dir[k] ?? []).includes(bad)) fail("C2", confFile, 0, `${k} allows ${bad}; the webview loads no remote content.`);
      }
    }
    for (const c of dir["connect-src"] ?? []) {
      if (!["ipc:", "http://ipc.localhost", "'self'"].includes(c)) {
        fail("C2", confFile, 0, `connect-src allows \`${c}\`; the webview talks only to IPC.`);
      }
    }
    for (const [k, want] of [
      ["object-src", "'none'"],
      ["frame-src", "'none'"],
      ["base-uri", "'none'"],
      ["form-action", "'none'"],
    ]) {
      if (!(dir[k] ?? []).includes(want)) fail("C2", confFile, 0, `${k} must be ${want}.`);
    }
  }
  if (sec.freezePrototype !== true) fail("C2", confFile, 0, "app.security.freezePrototype must be true.");
  if (conf.app?.withGlobalTauri === true) fail("C2", confFile, 0, "app.withGlobalTauri must stay off.");
  if (sec.dangerousDisableAssetCspModification) fail("C2", confFile, 0, "dangerousDisableAssetCspModification must not be set.");

  // C3 command allow-list
  const libFile = "apps/desktop/src-tauri/src/lib.rs";
  const lib = rustViews(read(libFile)).code;
  const handlers = [...lib.matchAll(/generate_handler!\s*\[([^\]]*)\]/g)];
  if (handlers.length !== 1) fail("C3", libFile, 0, `expected exactly one generate_handler!, found ${handlers.length}.`);
  for (const h of handlers) {
    const names = h[1]
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean)
      .sort();
    const want = [...TAURI_COMMANDS].sort();
    if (JSON.stringify(names) !== JSON.stringify(want)) {
      fail("C3", libFile, lineOf(lib, h.index), `Tauri commands [${names.join(", ")}] differ from the reviewed allow-list [${want.join(", ")}].`);
    }
  }

  // C5 asset protocol scope: only managed media folders. Databases, cache/ (intelligence index,
  // package staging copies), recovery/ and backups/ must never be readable by the webview.
  const libStrings = rustViews(read(libFile)).noComments;
  for (const m of libStrings.matchAll(/\b(allow_directory|allow_file)\s*\(([\s\S]*?),\s*(true|false)\s*\)\s*;/g)) {
    const arg = m[2].replace(/\s+/g, "");
    // Exactly `<path>.assets()` or `<path>.join("assets")`: nothing may be chained after it.
    if (!/^[\w.]+(\.assets\(\)|\.join\("assets"\))$/.test(arg)) {
      fail("C5", libFile, lineOf(libStrings, m.index), `asset scope \`${m[1]}(${arg})\` must name an \`assets\` folder only.`);
    }
  }
  if (/\b(allow_directory|allow_file)\s*\(/.test(libStrings.replace(/\b(allow_directory|allow_file)\s*\(([\s\S]*?),\s*(true|false)\s*\)\s*;/g, ""))) {
    fail("C5", libFile, 0, "an asset-scope call has an unexpected form; use allow_directory(<path>.assets(), true).");
  }
  const staticScope = conf.app?.security?.assetProtocol?.scope;
  if (staticScope !== undefined && !(Array.isArray(staticScope) && staticScope.length === 0)) {
    fail("C5", confFile, 0, "app.security.assetProtocol.scope must stay [] (the scope is set at runtime to assets/ folders only).");
  }
  // Tauri's own asset scope can only grow, so a closed project would stay readable. Media is
  // served by the `ofasset` protocol (`asset_scope.rs`), whose folders are replaced on every
  // project change; the built-in protocol stays off.
  if (conf.app?.security?.assetProtocol?.enable === true) {
    fail("C5", confFile, 0, "app.security.assetProtocol.enable must stay false; managed media is served by the ofasset protocol.");
  }

  // C4 desktop plugins
  const desktopCargo = "apps/desktop/src-tauri/Cargo.toml";
  const dc = read(desktopCargo);
  // Plain, table-form (`[dependencies.x]`) and renamed (`package = "x"`) dependencies.
  const plug =
    /(?:^\s*|^\[[^\]\n]*dependencies\.|package\s*=\s*")(tauri-plugin-(?:fs|shell|http|sql|process|upload|websocket|store|persisted-scope))\b/gm;
  let m;
  while ((m = plug.exec(dc))) {
    fail("C4", desktopCargo, lineOf(dc, m.index), `\`${m[1]}\` would give the webview filesystem, shell or network reach.`);
  }
}

// ---------------------------------------------------------------------------
// T1 theme colours (ADR-0014; docs/engineering/30-appearance-themes.md section 7): every themeable
// UI colour is a var(--token) from the generated theme layer. A colour literal in the React app's
// CSS or TSX must be a deliberately fixed or document colour and say so on its line or the line
// above ("fixed colour: <reason>" or "document colour: <reason>"). Only neutral black shadows are
// allowed unmarked; a tinted shadow is a token (--shadow, --shadow-s, --shadow-side) or marked.
function scanThemeColours() {
  const dir = path.join(ROOT, "apps/desktop/src");
  const files = walk(dir, (n) => /\.(css|tsx|ts)$/.test(n));
  const literal = /#[0-9a-fA-F]{3,8}\b|\brgba?\([^)]*\)|\bhsla?\([^)]*\)/g;
  const marker = /(fixed|document) colour:/;
  const neutralShadow = /^(#0{3,4}[0-9a-fA-F]?|#0{6}[0-9a-fA-F]{2}|rgba?\(\s*0\s*,?\s*0\s*,?\s*0\b[^)]*\)|rgb\(0 0 0\s*\/[^)]*\))$/;
  for (const abs of files) {
    const file = rel(abs);
    if (file.startsWith("apps/desktop/src/ipc/generated/") || /\.test\.(ts|tsx)$/.test(file) || file.startsWith("apps/desktop/src/test/")) continue;
    const lines = fs.readFileSync(abs, "utf8").replace(/\r\n/g, "\n").split("\n");
    lines.forEach((line, i) => {
      const hits = [...line.matchAll(literal)].map((m) => m[0]);
      if (!hits.length || marker.test(line) || (i > 0 && marker.test(lines[i - 1]))) return;
      const inShadow = /(box-shadow|text-shadow|--shadow[a-z-]*)\s*:/.test(line);
      const bad = hits.filter((h) => !(inShadow && neutralShadow.test(h)));
      if (bad.length) fail("T1", file, i + 1, `colour literal ${bad.join(", ")}: use a theme token (var(--...)), or mark it "fixed colour: <reason>" / "document colour: <reason>".`);
    });
  }
}

scanRust();
scanWebview();
scanConfig();
scanThemeColours();

if (LIST) for (const h of allowedHits) console.log(`allowed  ${h}`);
if (violations.length) {
  console.error(`architecture-lint: ${violations.length} violation(s)\n`);
  for (const v of violations) console.error(`  ${v}`);
  console.error("\nRules and allow-lists: scripts/architecture-lint.mjs. CONTRIBUTING.md §2, the security standard in masterplan.md.");
  process.exit(1);
}
console.log(`architecture-lint: OK (${allowedHits.length} allow-listed sites)`);
