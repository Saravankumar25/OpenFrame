# What the Windows tree must repeat from the Mac release work of 2026-11-03

Package rule 3: the Rust crates exist in both trees and must not drift. This checkout is the Mac tree. The Windows session applies
the shared changes below and runs its own gates. Nothing here has been run on Windows.

## Shared Rust, copy as they are

- `crates/openframe-entitlement`: `EntitlementState::ClockChanged`, `Notice::ClockLocked`, the new `ChannelAdapter` methods, summaries
  in the new wording, `notices.json.bad`, and the whole `mac/` folder (server removed, offline policy, clock guard).
- `crates/openframe-application/src/modules/entitlement.rs` and `registry.rs`: `next_check`, `check_edit`, `lock_ready`, the status fields
  `trialEndsAtMs` and `lifetimePrice`.
- `crates/openframe-ffi/src/store.rs`, `crates/openframe-platform/src/code_signature.rs`, `crates/openframe-ai` engine signature mode.
- `crates/openframe-application/src/modules/project.rs`: search rebuild off project open, `SEARCH_INDEX_VERSION`.
- `crates/openframe-project-format`: manifest defaults.
- `crates/openframe-application/src/modules/ai/`: sections, pointer, narrative answers, tool filter (ADR-0041). App migration 0010.
- Delete `services/licence-server`, and the addresses and workflow entries for it.

## Windows work these changes create

- **The lock window:** `store_state` opens it for the Mac channel only. On Windows, flush the page on the lock and call
  `entitlement.lock_ready`, then remove the channel condition.
- **`check_edit` is Mac only** because the Windows shell has no read-only editors yet. Decide with package file 02, section 2.
- **Section picker:** the Windows panel sends no picks, so every question gets the "pick a section" reply until its picker exists. Build
  it before shipping this core.
- `apps/desktop/src/features/licensing/`: chip wording, chip on Home, reminder on Home, and everything in package file 03.
- `trial_ends_at_ms` for the Microsoft Store adapter (now plus `TrialTimeRemaining`).
- TLS: `crates/openframe-ai/Cargo.toml` keeps `rustls-tls` on Windows and uses `native-tls` on the Mac.
