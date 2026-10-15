# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-16 (Windows phases W1, W2 and W3).

## State in six lines

1. **W1, W2 and W3 are done at proof level 2** on branch `windows/w1-controls` (local commits, not merged, not pushed).
   Evidence: [W2](../release/evidence/w2/close-handshake-and-data-safety-2026-10-16.md), [W3](../release/evidence/w3/fuzz-2026-10-16.md).
2. W1: proof levels in the status file, team password minimum 8, secret, link and installer gates, archive moves, conflict
   sweep, audience wording, test binaries merged (46 to 6), dev debug level lowered, 39 orphaned worktrees removed with the
   owner's approval (unique files in `.build-cache/salvage-2026-09-30`).
3. W2: the close handshake keeps typed text (40 closes in a row in spec 04), one pending-save registry, cloud-folder warning,
   start-up refusal without Documents, delete and locate tests, migration backup restore check, stranded-replace error.
4. W3: six fuzz targets (about 49 million inputs on Windows, libFuzzer job in CI), two new properties, a central capability
   check with a registry-generated test, release hardening flags, rustls security update, supply chain in `npm run verify`.
5. **Open finding for W4:** the WebView2 runtime opens background connections to Microsoft (threat model section 4 item 8).
   Spec `tests/e2e/specs/05-webview-network.test.mjs` fails on purpose until W4 switches that off.
6. Full Rust suite passed after the last fix (2,035 tests before, 604 application tests rerun after); web 581 tests pass.

## Next items

1. **Owner:** run the 10-minute OneDrive test ([checklist](../testing/onedrive-typing-test.md)); decide on merging the branch.
2. **W4 (needs the owner's word for a release executable):** WebView2 background traffic off (`additionalBrowserArgs`,
   keep wry's defaults), the release-build end-to-end run, `scan-installer` on the release folder, binary flags check.
3. Low review items left: per-rule `secret-scan: allow` marker; `check-links` skips URLs after `#[cfg(test)]`; Mac half
   of "typed text survives closing" (Mac phases).

## Blocked on the owner

- The OneDrive manual test, the word to build a release executable (W4), the macOS version of the Intel MacBook (M1).

## Do not

- Do not push, merge, or build an installer or release executable without the owner's word.
- Do not change the AI prompts or model before the W5 baseline exists (ADR-0028).
- Do not write shell heredocs or `node -e` snippets with backslashes or apostrophes; use the Edit and Write tools.
- Do not edit `docs/archive/`, `docs/audits/` or existing files in `docs/release/evidence/`. The team password is never written to a file.

## Facts a new session cannot see from the code

- The owner is a beginner in Rust, Tauri and Claude Code: plain words, short replies, smallest working change (ponytail).
- Hardware: 15.4 GB memory, 18 logical cores, Intel Core Ultra 5; a full Rust rebuild takes 6 to 10 minutes, an
  end-to-end app build about 10; libFuzzer does not link on this Windows MSVC toolchain.
