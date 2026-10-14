# Security review of openframe-platform and openframe-entitlement, 2026-10-06

> Read-only review by `ecc:security-reviewer` at commit `d30b588+dirty`, before the fixes. Findings are recorded as
> observed. The fixes applied the same day are listed in [INDEX.md](INDEX.md) row F-18; items that need the owner are in
> `docs/implementation/owner-decisions.md`. "Fixed" below means fixed on 2026-10-06 in the same session.

## High

1. `licence.rs`: no revocation or replay control; a leaked licence works for everyone, on any machine, forever. **Open: owner decision** (signed denylist, expiry, machine binding). `licence_id` and `schema` claims were added so a denylist can follow without a new format.
2. `licence.rs` and `adapters.rs`: the future-date check could lock out a paying customer with a wrong clock, and every error (including a transient IO error) became `NotPurchased`. **Fixed:** no clock check at verification; IO errors answer `Unknown`; only a parsed file that fails verification is `NotPurchased`.

## Medium

3. Canonical bytes were a re-serialisation of the parsed struct; a field or serde change would invalidate issued licences; no domain separation. **Fixed:** the signature covers the stored payload bytes verbatim with the prefix `openframe-licence-v1\0`.
4. Use `verify_strict`. **Fixed.**
5. Unbounded reads before the size checks in `trial.rs` and `adapters.rs`. **Fixed:** `take(cap + 1)`.
6. A transient read error destroyed the trial record (treated as tampering and overwritten). **Fixed:** IO errors are returned and never overwrite.
7. One bad clock reading (clock briefly far in the future) ends the trial for good. **Accepted, documented** in `trial.rs`: ignoring forward jumps would allow an unlimited trial; the record is not secret and can be deleted.
8. A write failure made every launch look like a first launch. **Fixed:** `LoadedTrial { record, persisted }`; the caller sees `persisted == false`.
9. `i64` overflow on hostile `started_at_ms`. **Fixed:** saturating arithmetic and believable-time range checks in `is_intact`.
10. macOS guard could signal the wrong process group. **Fixed:** `attach` requires `pgid == pid`, Drop re-checks before `killpg`.
11. The supervisor only warned when the Job Object could not be created. **Fixed:** on Windows it kills the child and refuses to run.
12. `replace_from_staged` did not sync and deleted any path on a lexical parent mismatch. **Fixed:** pattern check, canonical parent compare, sync before rename, foreign files never deleted.
13. No retry on a Windows rename sharing violation. **Fixed:** 5 attempts with backoff on error 32, 33 and PermissionDenied.
14. `remove_stale_temps` deleted live temps and had no callers. **Fixed:** exact pattern and 10-minute age; called at project open and app start.
15. Silent fallback to the temp folder in `dirs.rs`. **Fixed in the crate** (returns `Option`); the `openframe-project-format` wrappers keep the old fallback. **Open: owner decision** (refuse to start or ask).
16. The trial checksum is a keyless hash: editing, deleting or a new user account restarts the trial. **Documented honestly** in `trial.rs`; store channels use their own licence service. **Open: owner decision** for the installer channel.

## Low

17. Replacement loses the destination's ACL or mode. Open (noted in `37-platform-boundary.md`).
18. Long destination names and a panic in the writer leaked the temp file. **Fixed:** name cut to 120 characters, drop guard.
19. Relative paths returned false instead of falling back. **Fixed.**
20. Family-name validation too loose. **Fixed:** 3 to 50 character name, 13 character lower-case publisher id.
21. Unsalted e-mail hash; Unicode lower-casing. **Partly fixed:** ASCII-only normalisation; salting is the issuing service's decision (owner).
22. `issue` and `SigningKey` compiled into the app. **Fixed:** behind the `issuer` feature and tests.
23. `lib.rs` pointed to a document that did not exist. **Fixed:** `docs/engineering/36-entitlement-and-trial.md` written.

## Lint R6

The regex missed `cfg!(windows)`, `cfg(all(windows, ..))`, `cfg_attr`, `target_env`, `std::os::fd`, `std::env::consts::OS`, `windows_core`, `nix`, `objc2`, `core_foundation` and raw `#[link(`; `apps/**/build.rs` was not scanned. **Fixed:** widened regex, `apps` scanned, 10 self-test cases. The three `std::env::consts::OS` reads found by the wider rule are allow-listed with reasons and listed as work to move onto `current_os()`.

No path or e-mail leaks in logs or errors; no em dashes in user-facing strings.
