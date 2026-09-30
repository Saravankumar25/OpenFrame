# Internal security review: 2026-09-30

**Scope:** the code on `master` at `a31ee7f`: Tauri adapter and capabilities, every operation that takes a filesystem
path, archive/XML/PDF parsing, IPC deserialization, SQL construction, the Private Notes boundary, local AI (manifest,
downloads, sidecar, tool calls, Change Sets), package import/export, logs/diagnostics, release/Store scripts, the
frontend, dependencies and secrets.
**Method:** manual code review across all crates and the frontend, with focused sweeps per area, then a fix and a
regression test for every finding. The full findings table is in
[`20-security-threat-model.md` §5](20-security-threat-model.md#5-internal-review-2026-09-30-findings).
**Model:** OpenFrame is a local-first single-user app with no listening sockets. The review treats the **webview as
untrusted** (threat T1). **Imported documents and received packages are treated as hostile**, because they come from
other people. A local attacker running as the same Windows user is out of scope (§1 of the threat model).

## Outcome

- **31 findings: 6 High, 11 Medium, 12 Low, 2 Info.** Every High is fixed.
- **Medium:** every Medium is fixed except PKG-04, which is accepted (justification below).
- **Low:** DES-01, AI-05, WEB-04, PKG-05 and the two SUP-01 warnings are accepted with justification. The rest are
  fixed.
- **Regression tests added:**
  - `crates/openframe-security/tests/security_paths.rs`
  - `crates/openframe-import-export/tests/security_parsers.rs`
  - `crates/openframe-ai/tests/security_ai.rs`
  - `crates/openframe-application/tests/security_review.rs`
  - `apps/desktop/src/workspaces/screenplay/lock.security.test.ts`
  - new unit tests in `openframe-ai::manifest`

### High (all fixed)

| ID | What an attacker could do | Fix |
|---|---|---|
| PKG-01 | A received project package makes OpenFrame connect to `\\attacker\share` when it is only inspected (NTLM hash leak), or launch an executable disguised as a picture when the user clicks "Open" | Linked paths are only touched when they are on a local disk; executable types are revealed in Explorer instead of launched; links to network shares are refused |
| PKG-03 | A received project database runs its own triggers/views, or injects SQL through a crafted table name, including `ATTACH` to drop files anywhere | Schema screening on open and on import; `trusted_schema=OFF`; `SQLITE_LIMIT_ATTACHED=0`; identifiers always quoted |
| AI-03 | `ai.remove_model("C:")` recursively deletes the process's current directory | Ids must be safe single components directly under `models/` |
| REL-01 | Public releases would trust the development manifest-signing key | The release gate requires a distinct `OPENFRAME_MANIFEST_PUBLIC_KEY`; the workflow passes it; an untrusted embedded manifest is never used |
| PARSE-01 | One DOCX/FDX tag with millions of attributes hangs the import | quick-xml 0.42 (RUSTSEC-2026-0194/0195); O(n²) duplicate check off; ≤ 128 attributes per element |
| PARSE-02 | A small PDF crashes the app: a decompression bomb, `[[[[…` stack overflow, or content amplification | lopdf 0.45 (RUSTSEC-2026-0187) with a per-stream decompression limit; pre-flight nesting/inflation limits; OpenFrame-controlled page-content decompression with a budget; large-stack parse thread |

### Accepted risks (justification)

- **PKG-04 (Medium): full-project packages are not authenticated.** A hostile package can reference local files that
  a *later* export with "include linked files" would copy, or plant private notes shown to the recipient.
  - Exploiting it needs the user to open a full project from an untrusted party *and* to export it again with linked
    files included.
  - The export preview lists every linked file by path.
  - PKG-01 removes every silent network access and every file launch.
  - Signing packages would need key distribution, which is out of scope with no accounts. It is tracked as open
    item 5 in the threat model.
- **PKG-05 (Low): exchange packages are not authenticated either.** Author ids can be spoofed, and leaving out
  `base_snapshot` hides the "Stale" banner. This is file-based collaboration by design: every change is listed for
  review, the operations are fixed by the planner, and per-change conflicts are still computed.
- **DES-01 (Low): four nested argument types lack `deny_unknown_fields`.**
  - Two are internally tagged enums; ts-rs can't emit the attribute for them. The other two are JSON stored in the
    database and must stay forward-compatible.
  - Unknown keys are dropped and never acted on.
  - All top-level Args structs deny unknown fields.
- **AI-05 (Low): sidecar hardening gaps.** There is no orphan sweep, Job Object assignment failure is silent, and
  nothing is re-verified at launch. Exploiting any of these requires a same-user local attacker (out of scope).
- **WEB-04 (Low): CSP `style-src 'unsafe-inline'`.** React style props need it. `script-src` is `'self'` only;
  `object-src` and `frame-src` are `'none'`.
- **SUP-01 warnings (Low):** `cargo audit` still reports two warnings. `proc-macro-error` is unmaintained, but it is
  a build-time procedural macro. `glib` is unsound, but it is a Linux GTK dependency of Tauri that is not compiled
  for Windows. Neither runs in the shipped Windows application.

## What was checked and found sound

- **Tauri surface:**
  - One allow-listed data command, `of_invoke`; unknown ops are rejected.
  - Capabilities grant only event, window, app-version and dialog permissions. There is no fs, shell or http plugin
    (enforced by ESLint).
  - CSP is strict.
  - The asset-protocol scope is now limited to media folders (WEB-02).
  - `of_open_asset` takes an id, never a path.
- **Frontend:**
  - No `dangerouslySetInnerHTML`, `innerHTML`, `eval`, `window.open` or `target=_blank`.
  - User URLs are never rendered as `href`; they go through `of_open_url`.
  - Pasted HTML in the screenplay editor is reduced by the ProseMirror schema.
  - `convertFileSrc` only receives paths resolved in Rust.
  - `localStorage` holds view preferences only.
- **Archives:** zip-slip, absolute/drive/UNC names, symlink entries, ratio/size/entry caps. The real decompressed
  stream is capped, and checksums are verified before any write into a project.
- **XML:** no DTD or external-entity expansion (billion-laughs test), depth ≤ 256, node budget.
- **Images:** decoding limits of 20000 × 20000 and 512 MiB; ingest reads the header only.
- **SQL:**
  - Parameterized everywhere.
  - Dynamic identifiers come from constants, allow-lists or (now always quoted) `sqlite_master`.
  - FTS queries quote every term.
  - `LIKE` with user text is escaped.
- **Private Notes:**
  - Every read filters by owner (search, AI tools and scope, trash listings, list/update/delete).
  - Exchange packages refuse them.
  - Document exports never read them.
  - Trash titles are a fixed label.
- **AI:**
  - Signed manifest verified over the exact bytes; SHA-256 per artifact before rename or extract.
  - Sidecar spawned without a shell, on 127.0.0.1 with a random port and a per-launch key passed through the
    environment, with `CREATE_NO_WINDOW` and a kill-on-close Job Object.
  - Tool calls are schema-constrained and unknown tools rejected.
  - Change Sets are replayed only through allow-listed, registered commands with the applying user's capabilities,
    and are revalidated for staleness and role changes.
- **Logs:** operation names, codes and timings only. Panics are now redacted and truncated.
- **Updater / Store:**
  - No in-app updater is compiled in.
  - `build.rs` sets `cfg(openframe_store)`.
  - The release gate fails closed: license, versions, signing material, updater configuration, Store identity and
    assets, AI distribution URL, and now the manifest key.

## Dependency advisories and secrets

- `npm audit --omit=dev`: **0 vulnerabilities**.
- `cargo audit` (cargo-audit 0.22.2, installed into the user's cargo directory):
  - **Before:** 3 vulnerabilities. RUSTSEC-2026-0187 is a lopdf stack overflow on nested objects.
    RUSTSEC-2026-0194 is quadratic quick-xml duplicate-attribute checking, and RUSTSEC-2026-0195 is unbounded
    quick-xml `NsReader` namespace allocation. These are the same issues as PARSE-01/02, found independently.
  - **Fix:** quick-xml was upgraded 0.38 → 0.42 and lopdf 0.36 → 0.45. Only `openframe-import-export` uses them.
    `THIRD_PARTY_LICENSES.md` was regenerated; every new crate has a permissive license (brotli-decompressor, weezl,
    RustCrypto 0.11 crates).
  - **After:** 0 vulnerabilities, with 2 accepted warnings (see above).
- The release workflow also runs `cargo deny --locked check` as a blocking step.
- **Secret sweep:**
  - Regex scan of the working tree and of `git log -p --all` for private keys, AWS/GitHub/GitLab/npm/Slack/Google
    tokens, `sk-` keys and `password|secret|token|api_key = "…"` literals.
  - No `.pfx/.p12/.pem/.key/.env` file is tracked now or was ever added.
  - **Nothing found.** The only key material is the development manifest *public* key
    (`crates/openframe-ai/keys/manifest-dev.pub`).

## Verification

- `cargo test -j 2 --workspace --exclude openframe-desktop`: green (1,156 tests).
- `cargo clippy --all-targets -- -D warnings`: clean on every touched crate, including `openframe-desktop`.
- `cargo fmt --check`: clean.
- `tsc --noEmit`, `eslint src --max-warnings 0` and `vitest run`: clean.
- `node scripts/release-validate.mjs --report-only`, checked three ways:
  - no key set: FAIL;
  - the development key set: FAIL;
  - any other valid 32-byte key set: PASS.

## Follow-ups (not blocking)

1. An adapter-level test for the asset-protocol scope (T5), and a "no content in logs" test (T20).
2. An ADR on package authenticity (PKG-04/PKG-05) if signed packages become a requirement.
3. Sidecar hardening (AI-05): create the process suspended, assign it to the Job Object, then resume; sweep for
   orphans at startup.
