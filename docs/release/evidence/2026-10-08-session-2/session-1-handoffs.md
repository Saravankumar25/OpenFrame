# Session 1 handoffs: review and finish

Date: 2026-10-08. Branch `fix/session-1-handoffs`, reviewed in its own worktree. The branch held a checkpoint commit
(`d56d977`) of 48 files the session 1 handoff agent left unreviewed. This file records, per item, what was found, what was
changed, the proof, and the plan box and feature status changes proposed to the lead. The lead owns `masterplan.md`,
`progress-log.md` and `feature-status.md`; none of them was edited here.

`session-2` (at `79a660a`) was merged into the branch first. One conflict, in `apps/desktop/src/workspaces/settings/index.tsx`,
was resolved by keeping both imports (the About card and the breakdown category settings).

## 1. Team password gate of the internal builds (ADR-0024)

**Verdict: needed finishing; now finished in code and tests, not yet run as a real signed internal build.**

Reviewed against ADR-0024 and the security standard:

- **Hash only, never the password.** `apps/desktop/src-tauri/build.rs` reads `OPENFRAME_TEAM_PASSWORD` on the build machine,
  hashes it with Argon2id (64 MiB, 3 passes, 1 lane, 16 bytes of salt from the system's secure random generator) and writes
  only the PHC string to `OUT_DIR/team_gate_hash.txt`. Error messages name the variable, never its value (tested). The build
  script avoids `rerun-if-env-changed` while the variable is set, so Cargo does not keep the value in the target folder.
- **One shared rule file.** `crates/openframe-entitlement/src/team_gate/build_hash.rs` is included by path in the build script
  and compiled in the crate, so the build and the check use the same algorithm and costs.
- **Constant-time check.** `verify_team_password` accepts only `argon2id`, refuses passwords over 1024 bytes without hashing,
  and recomputes the hash with the stored salt and costs. The comparison is `phc::Output`'s `PartialEq`, which uses
  `subtle::ConstantTimeEq` (checked in the `phc` 0.6.1 and `password-hash` 0.6.1 sources).
- **No logging.** `TeamGateUnlockArgs` has a `Debug` that prints no field; `of_invoke` logs the operation name only; the core
  logs "team password accepted" or "refused" with no value. The count file holds numbers only (tested: it never contains the
  test password).
- **Lockout.** 5 wrong tries wait 1 minute, then 2, 4, 8, 16 and 30 minutes from then on; a try during a wait is refused
  unchecked; the count is saved before the slow check; a restart keeps the count and the running wait; a right password
  resets it; a damaged count file starts at the first wait.
- **Store builds.** A `store` build never reads the variable, sets no `openframe_team_gate` cfg and embeds nothing. The core
  holds no gate when `AppConfig::team_password_hash` is `None`.
- **Fail closed.** An internal (`direct`) release build without the variable, with an empty one or with fewer than 12
  characters panics in the build script. Debug builds without the variable run with no gate (development and end-to-end).
- **Core enforcement.** `EntitlementService::check_operation` (inside `Registry::dispatch`) refuses every operation but
  `team_gate.status`, `team_gate.unlock` and `appearance.get` with `permission.team_password_required`. `team_gate.unlock`
  requires the local person (never the assistant or a package), and both operations are hidden from the assistant.

Fixed in this review:

1. **A clock turned back kept the gate shut until the clock caught up.** `wait_left` returned the full wait while the clock
   was before the wait's start, and nothing moved the start, so a clock turned back by a day held the gate for a day. The wait
   now restarts from the earlier time and is saved (`GateRecord::restart_wait_if_clock_went_back`, called from `view` and
   `try_unlock`), so it ends after its own length of real time. New test
   `a_clock_turned_back_during_a_wait_seen_first_by_a_try_restarts_the_wait`; the existing clock test now proves the wait ends
   one minute after the clock went back and that the new start survives a restart.
2. **The shell's `of_reveal_path` opened the data folder and the logs before the password.** It skipped the operation check
   for those two kinds, which the store lock needs. Before the team password it now refuses them too
   (`apps/desktop/src-tauri/src/lib.rs`).
3. **The end-to-end build could pick up the build machine's team password.** `tests/e2e/scripts/build-app.mjs` now removes
   `OPENFRAME_TEAM_PASSWORD` from the environment of the debug build, so the suite never meets the password screen.
4. **Two clippy findings** in the gate (`is_multiple_of`, an identity operation in a test).
5. **Three Vitest files failed** (`TeamGate.test.tsx`, `About.test.tsx`, `DeletedView.test.tsx`): `beforeEach(() =>
   callMock.mockReset())` returns the mock, and Vitest runs a function returned from `beforeEach` as a clean-up hook, which
   called the mock with no operation after each test. The hooks now use braces. No assertion changed.
6. `Cargo.toml` and `crates/openframe-entitlement/Cargo.toml` had been rewritten with LF line endings over CRLF files; the
   original endings are restored so the diff shows only the real lines.

Proof: `crates/openframe-entitlement/src/team_gate/tests.rs` (9 tests), `crates/openframe-application/tests/team_gate.rs`
(4), `apps/desktop/src-tauri/src/lib.rs` tests `a_store_build_embeds_no_team_password_hash` and
`store_packages_are_store_channels_and_everything_else_is_an_internal_build`, `TeamGate.test.tsx` (4). Docs updated:
`docs/engineering/36-entitlement-and-trial.md` section 11 (clock rule, shell commands, test count).

Not run: an internal release build with a test-only password set, launched and unlocked by hand; a release build without the
variable to watch it fail (the rule is unit-tested in `build_plan`, the build itself was not run because a release build of
the desktop crate is long and the shared target is in use by other agents); the internal Mac disk image.

## 2. Release workflow and Cargo changes

**Verdict: finished.** `.github/workflows/release.yml` passes the `OPENFRAME_TEAM_PASSWORD` secret to the internal installer
step only; the Microsoft Store step sets `OPENFRAME_DISTRIBUTION: store` and gets no secret. The verification steps run debug
builds, which need no password. `argon2` 0.6 (default features off, `alloc` and `password-hash`) and `getrandom` 0.3 (build
script only) are added; `ed25519-dalek` and the `issuer` feature leave the application crate. `npm run licenses` reproduces
`THIRD_PARTY_LICENSES.md` exactly.

`cargo deny check`: bans ok, licenses ok, sources ok; **advisories failed** on `rustybuzz` and `ttf-parser` (both marked
unmaintained; the `ttf-parser` advisory is RUSTSEC-2026-0192), which come from the screenplay PDF fonts work already on `session-2`, not from this branch.
Nothing reported touches `argon2`, `blake2`, `password-hash` or `phc`.

## 3. Entitlement crate clean-up (ADR-0023: no installer licence)

**Verdict: finished.** `licence.rs`, `trial.rs`, `DirectInstallerAdapter`, `LicenceCheck`, the `issuer` feature and
`AppConfig::licence_public_key` are gone. A search of code, tests, scripts and living documents finds no reference left; the
only mentions are SUPERSEDED history in `docs/engineering/36-entitlement-and-trial.md`, old evidence in
`docs/release/evidence/2026-10-06-foundation/verify.txt`, and tests in `crates/openframe-application/tests/entitlement.rs`
that assert no `trial.json` is ever written. The internal builds have no adapter (`adapters::for_channel` returns none) and
their state is InternalBuild: no trial, no Buy, no reminder, no lock. A Mac build is the Mac App Store channel only when it is
a `store` build; the internal Mac disk image is an internal build.

## 4. About card in Settings

**Verdict: finished.** `apps/desktop/src/workspaces/settings/About.tsx` shows the version and, on the internal builds,
"Internal build, not for sale." The Licence card says the same. Proof: `About.test.tsx` (2). New end-to-end spec
`tests/e2e/specs/16-about.test.mjs` (not run here).

## 5. Recently Deleted 30-day purge (ADR-0012 amendment)

**Verdict: finished.**

- **Never deletes a non-expired item.** The candidate list uses `deleted_at <= now - 30 days`; inside each transaction the
  record is read again and removed only when `is_expired` still holds, the registered table matches the record and the
  object there still carries `deleted_at`. A restored item, a live row behind a stale or crafted record, or a table without
  `deleted_at` is skipped.
- **Clock.** A delete time later than now counts as now: nothing is removed early and the screens show 30 days.
- **Restart and crash.** Each item is its own `Store::mutate` (quiet, not undoable), so a kill or a full drive leaves each item
  whole or gone; the next open carries on. At most 500 items per open. It runs on project open and on the first open of the
  Global Idea Vault in a session (app start, or after the team password on an internal build). A failure is logged and never
  blocks opening. No network.
- **Screens.** Both Recently Deleted screens show "N days left" in a "Removed in" column; Delete Permanently still works
  earlier.

Proof: `crates/openframe-application/tests/recently_deleted_purge.rs` (7: boundary millisecond, days left, clock back, clock
forward, interrupted purge through injected commit failures and the per-run limit, a record pointing at a live object, purge
on project open and app start), `modules/retention.rs` unit tests (2), `RecentlyDeleted.test.tsx`, `DeletedView.test.tsx`.
New end-to-end spec `tests/e2e/specs/25-recently-deleted-purge.test.mjs` (not run here): deletes two notes and an idea in the
real app, checks "30 days left" on both screens, quits, moves the delete times back in the closed project file with Node's
built-in SQLite module, relaunches, and checks that only what is due is gone, the live note stays, the young note shows
"1 day left", Delete Permanently still works early, and nothing comes back after another relaunch.

## 6. Forced-kill tests and process sweep (areas 3 and 4)

**Verdict: finished.** `crates/openframe-application/tests/forced_kill.rs` now times its last kill after the worker's first
committed operation instead of a fixed 700 ms, so the kill always lands in real work on a slow or loaded machine, with a
generous 180 s bound that only turns a hung worker into a failure. Three panic messages that held raw line breaks inside
string literals now use `\n`. `crates/openframe-platform/src/process.rs`: `stop_if_same` answers NotRunning for a process
that has exited but can still be opened through another handle; the existing test
`stop_if_same_stops_only_the_recorded_process` covers that last step. Both strengthen boxes already ticked; no box changes.

## 7. Gate fixes outside the handoff

The merged `session-2` did not pass the gates on its own. Fixed here so the branch is green: `cargo fmt` over nine files that
were not formatted (`asset_scope.rs`, `distribution.rs`, `engines.rs`, `supervisor.rs` and its test, `failure_injection.rs`,
`migration_failures.rs`, `power.rs`, `shell.rs`, `openframe-security/src/lib.rs`); clippy `assertions_on_constants` in
`crates/openframe-platform/src/drives.rs` (now a `const` assertion); clippy dead code in
`apps/desktop/src-tauri/src/asset_scope.rs` (`AssetRoots::dirs` and `AssetScope::roots` are used only by tests and are now
`#[cfg(test)]`).

## Commands and results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 1901 passed, 0 failed, 6 ignored, 80 test binaries. The doctest step first failed with E0460 because another agent rebuilt `openframe_domain` in the shared target at the same moment; `cargo test --workspace --locked --doc` then passed (12 crates, 0 failed, 2 ignored) |
| `npm run typecheck` | clean |
| `npm --workspace apps/desktop run lint` | clean (the root `npm run lint` also runs clippy, above) |
| `npm run test:web` | 48 files, 506 tests passed |
| `npm run check:governance` | passed; writing standard OK (app 204, crates 481, scripts and CI 29, living docs 41); links OK; 64 governance tests passed |
| `cargo deny check` | advisories failed (rustybuzz, ttf-parser unmaintained, from `session-2`); bans, licenses, sources ok |
| `npm run licenses` | no change to `THIRD_PARTY_LICENSES.md` |

Not run: the end-to-end suite (the lead runs it serially), `graphify update .` (left to the lead so the graph is rebuilt once
after the merges), a release build of the desktop crate.

## Proposed plan boxes (for the lead)

- Area 9, **Recently Deleted 30-day purge**: tick once `25-recently-deleted-purge.test.mjs` passes. Evidence: this file
  section 5.
- Area 18, **Internal installer build**: tick once `16-about.test.mjs` and `15-trial-first-run.test.mjs` pass. Evidence: this
  file sections 3 and 4.
- Area 20, **Team password gate**: the required tests (right, wrong, lockout, restart, missing variable) exist and pass.
  Suggest ticking after one manual check: build an internal release with a test-only password, launch it, see the password
  screen, unlock, and confirm a build without the variable fails. Evidence: this file section 1.

## Proposed feature status changes (for the lead)

- PLAT-016 "Team password gate of the internal builds": test list should read `team_gate/tests.rs (unit, 9)` and add
  `a_clock_turned_back_during_a_wait_seen_first_by_a_try_restarts_the_wait`. Stays Built until a gated build is run.
- CORE-007 "Recently Deleted, Activity": add `25-recently-deleted-purge.test.mjs`; move to Verified when it passes.
- VAULT-004: add `25-recently-deleted-purge.test.mjs` (the Vault's Recently Deleted days left and the idea removed after 40
  days).
- PLAT-012 and PLAT-013: no change (PLAT-013 is already Removed).
- The settings About card: add `16-about.test.mjs` to the row that carries it (PLAT-016 lists About).

## Owner questions

- `cargo deny` reports `rustybuzz` and `ttf-parser` as unmaintained with no safe upgrade. Accept them for now with a dated
  ignore in `deny.toml`, or plan a move to a maintained shaper? This comes from the screenplay PDF fonts work, not from this
  branch.
- The team password must be set as the `OPENFRAME_TEAM_PASSWORD` repository secret before an internal release build, and
  ADR-0024 asks for a new password because the first one was typed in a chat. Only the owner can do that.
