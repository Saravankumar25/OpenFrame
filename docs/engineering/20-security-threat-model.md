# 20 — Security Threat Model & Secure Storage

> **Status legend:** ✅ implemented (with test) · 🟡 implemented, test missing · 🚧 in development · 📋 planned.
> Authority: Security, Privacy & Data Ownership spec (digest `docs/spec-digest/security.md`); ESD §13.
> Decisions: ADR-0003, ADR-0004, ADR-0006, ADR-0007, ADR-0009.

## 1. Scope and assets

**Assets:** the user's project content (scripts, story, production data, media), Private Notes, the local profile,
integrity of exported documents, and the integrity of downloaded runtime, model and update binaries.

**Trust boundaries:**

```text
[ untrusted input files: imports, packages, media, URLs ]      [ untrusted network: model CDN, update endpoint (planned) ]
                    │ user-chosen path                                       │ HTTPS + signature verify
                    ▼                                                        ▼
[ WebView (React) — untrusted renderer ] ──of_invoke──► [ Rust core — trusted ] ──► [ project folder / app data ]
                                                               │ loopback + token (planned)
                                                               ▼
                                                    [ llama.cpp sidecar — untrusted output ]
```

**Out of scope / removed:** real-time LAN collaboration was removed by product-owner decision (ADR-0007).
OpenFrame opens **no listening socket**, so network listener, discovery, pairing and session-authorization threats do
not apply. There are no accounts or servers, so credential theft and server compromise do not apply. Malware running
as the same Windows user is out of scope: it can read the user's files regardless (the Security spec relies on OS
account protection, and encryption at rest is later scope).

## 2. Threats → controls → code → tests

| # | Threat | Control | Code | Tests / status |
|---|---|---|---|---|
| T1 | Webview compromise (XSS via imported or pasted content) reads or writes arbitrary files | No fs/shell/http plugins, least-privilege capability, strict CSP (`script-src 'self'`, `connect-src ipc:`), `freezePrototype`, React escaping (no `dangerouslySetInnerHTML` of content) | `tauri.conf.json`, `capabilities/default.json`, `eslint.config.js` | ✅ config; 📋 CI grep for `dangerouslySetInnerHTML` |
| T2 | Webview invokes operations that aren't meant to exist | Single `of_invoke` + registry allow-list; unknown op → `security.unknown_operation`; `deny_unknown_fields` args | `registry.rs`, `src-tauri/src/lib.rs` | 🟡 (no test for unknown-op rejection; add one to `core_pipeline.rs`) |
| T3 | Path traversal via stored or supplied relative paths (`..`, absolute, UNC, drive, ADS `:`, reserved names, symlink escape) | `validate_relative` + `confine` (canonicalized prefix check); assets resolved by id in Rust | `openframe-security/src/lib.rs`, `util::asset_file_path` | ✅ `relative_paths_cannot_escape`, `confine_joins_inside_root`, `sanitize_handles_hostile_names` |
| T4 | Opening or revealing arbitrary files via IPC | `of_open_asset` takes an asset id only; `of_reveal_path` limited to known kinds; `exported` must be absolute and exist | `src-tauri/src/lib.rs` | 🟡 (adapter not covered by tests; 📋 extract path resolution into a testable fn) |
| T5 | Asset protocol reads outside the project | Static scope empty; runtime scope = open project + Global Idea Vault only | `sync_asset_scope` | 🟡 |
| T6 | Malicious ZIP (package/DOCX): zip-slip, absolute paths, symlinks, zip bomb, lying headers | `archive::inspect` pre-flight (names, symlinks, sizes, ratio > 200, entry count); `read_entry` caps the actual stream; `extract_all` inspects first, extracts only into a new directory, caps every stream, and removes the partial directory on any failure | `openframe-security/src/archive.rs` | ✅ `zip_slip_rejected_and_nothing_written`, `absolute_and_drive_paths_rejected`, `oversized_entry_rejected`, `garbage_is_rejected_humanely` |
| T7 | Oversized or non-regular input files (DoS, device files) | `validate_input_file` (regular file, size cap; managed copy ≤ 8 GiB); text length limits | `security::validate_input_file`, `util::{required_text,body_text}` | ✅ `text_validation`; 🟡 file-size cap |
| T8 | Hostile import content mutates the project without consent | validate → preview → Change Set → apply; `Import`/`ApplyChangeSet` capabilities | import/package modules | 🚧 |
| T9 | Permission bypass (Viewer/Commenter mutating via UI, import, AI, or package response) | Capability check before any transaction; object-state checks (locked drafts); roles in `Role::allows` | `store.rs::mutate`, `auth.rs` | ✅ `permissions_are_enforced_before_any_mutation`, `viewer_cannot_mutate`, `commenter_cannot_edit_core_content`, `only_owner_manages_permissions_and_hosts` |
| T10 | Private Notes leak to other users (search, packages, AI context) | Owner filter in every query; search `owner_user_id` filter; packages exclude private notes by default | `search.rs::run`; notes module | 🟡 search filter (test to add with notes module); 🚧 packages/AI |
| T11 | FTS query injection | Every term quoted and prefix-matched; ≤ 12 terms; ≤ 500 chars | `search::fts_query` | ✅ `user_text_cannot_inject_fts_syntax` |
| T12 | SQL injection | Parameterized SQL everywhere; dynamic identifiers only from code constants or the catalog, quoted with `quote_ident`; column allow-lists in `update_fields` | `persistence/rows.rs`, `undo.rs` | ✅ `update_respects_allow_list_and_revision` |
| T13 | Two writers corrupt a project (second window, network share) | Exclusive OS lock; single-instance plugin; UNC refused | `ProjectLock`, `is_network_path` | ✅ `lock_is_exclusive`, `project_is_single_writer`, `network_paths_detected` |
| T14 | Corrupt or too-new database is "repaired" and destroyed | `quick_check` before use, refuse without modifying; too-new refused; safety backup before migration; atomic migration | `Store::open`, `migrate.rs` | ✅ `corrupt_database_is_refused_without_modification`, `too_new_database_is_refused`, `failed_migration_rolls_back_completely` |
| T15 | Undo/restore overwrites newer work | Full-row stale check | `undo::check_applicable` | ✅ `undo_refuses_to_overwrite_later_changes`, `undo_never_overwrites_later_work_by_someone_else` |
| T16 | Tampered model/runtime download (CDN or DNS compromise) | Ed25519-signed manifest with a compiled-in public key; SHA-256 per artifact; `.partial` → verify → atomic rename; keep last known-good | `security::{verify_ed25519, sha256_file}`; AI module | ✅ `signatures_verify_and_reject_tampering`, `hashing_is_stable`; 🚧 download manager |
| T17 | Prompt injection via project text ("ignore previous instructions…") | Retrieved content marked as data; strict tool schema; application-side authorization; mutations only as previewed Change Sets | AI module (ADR-0006) | 🚧 (prompt-injection suite is a model release gate) |
| T18 | Local AI sidecar reachable by other local processes | Bind 127.0.0.1, random port, per-launch token, Job Object kill-on-close | AI module | 🚧 |
| T19 | Tampered update | Tauri updater signature (minisign key; the private key exists only as a CI secret); Authenticode-signed installers; user approval before install | `release.yml`, `24-windows-packaging-updater.md` | 📋 (updater plugin not yet integrated) |
| T20 | Personal data or content leaks through logs or diagnostics | Logs contain op names, codes and timings only; `AppError.detail` never includes content; panic text redacted (home paths, e-mails, tokens); local only, no upload | `init_logging`, `security::redact` | ✅ `redaction_removes_personal_data`; 🟡 no automated "no content in logs" test |
| T21 | Link opening abuse (`file:`, `javascript:`, custom schemes) | `of_open_url` only accepts `http(s)` ≤ 4096 chars, on explicit user action | `src-tauri/src/lib.rs` | 🟡 |
| T22 | Supply-chain compromise or license contamination | `cargo deny` (advisories, sources, licenses), `npm audit --omit=dev`, lockfiles (`--locked`, `npm ci`), license inventory, SBOM | `deny.toml`, `ci.yml`, `release.yml`, `scripts/license-inventory.mjs` | ✅ CI jobs (non-blocking in CI, blocking in release) |
| T23 | Signing keys leaked from the repository | `*.pfx`/`*.p12`/`*.key`/`.env*` gitignored; `release-validate` fails if key files are committed; secrets only in CI; certificate removed from the runner after the build | `.gitignore`, `scripts/release-validate.mjs`, `release.yml` | ✅ |
| T24 | Silent network egress (privacy) | No telemetry code; CSP blocks webview egress; outbound HTTP only for user-approved downloads | ESD §11, invariant 10 | 🟡 📋 test: no HTTP client use outside downloader modules (`cargo tree -i reqwest` review) |

## 3. Secure storage

- **At rest:** project data is plain SQLite + files, protected by Windows account ACLs (the user profile). OpenFrame
  does **not** claim encryption (Security spec: "only claimed once implemented and tested"). Optional encrypted
  projects and packages are later scope and need their own ADR (key management, recovery).
- **Secrets:** v1 has none. No accounts, API keys or tokens are stored. The AI sidecar token is ephemeral and held in
  memory only. If a future feature needs a secret, it goes in Windows Credential Manager behind an adapter (ESD §14),
  never in SQLite or plain files.
- **Temporary files:** writes use temp siblings in the same folder (`*.tmp`, `*.partial`, `*.importing`), renamed
  atomically and removed on failure. Nothing is written to `%TEMP%` with content, except OS dialogs.
- **Deletion:** "delete" is recoverable. Permanent purge removes rows and, after commit, the managed files it
  exclusively owns. Project deletion goes to the Recycle Bin. No secure-erase claims are made.

## 4. Open items (tracked)

1. Add tests for T2 (unknown op), T4/T5/T21 (adapter path logic, after extraction to a testable module), T10 (private
   search filter), T20 (log content check).
2. Foundation cleanup after ADR-0007: remove `Capability::{HostSession, JoinSession}` and `ActorOrigin::Lan`, and fix
   the `project_format.network_location` message that still suggests a local-network session.
3. `history.undo`/`redo` should require `Edit` (defence in depth, ADR-0009).
4. Integrate the Tauri updater plugin with a pinned public key before the first public release (T19).
