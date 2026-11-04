# 05. Update safety and store standards

The owner's question: when a new version goes to the Microsoft Store or the Mac App Store and users update, does the app stay intact and does every user's data stay intact? This file is the audit of the real code, the gaps, and a standards checklist. Both sessions do the items marked for them.

## A. What the real code already does (read on 2026-11-02, tree `10e5d7b`)

| Area | Evidence | Verdict |
|---|---|---|
| Migrations are append-only and frozen once shipped | `straft-application/src/schema.rs` header rule; 19 project, 4 global, 9 app migrations; `scripts/check-migrations.mjs` | Good. Keep the gate in CI. |
| A safety copy is taken before any upgrade | `store.rs` (about line 389): `pre-migration-v{from}-{time}.sqlite`; if the copy fails the upgrade stops with plain words | Good. See U5 for where the copy lives. |
| A project newer than the app is refused, never opened | `migrate::plan` returns `TooNew`; `project.rs` (about line 1755); the same for vault archives, packages and backups (`vault_archive.rs`, `format.rs`, `backup.rs`) | Good. Messages say "Update Straft". |
| Foreign databases are checked before opening | `untrusted_schema_problems` in `store.rs` | Good. |
| One writer per project | `ProjectLock::acquire` | Good. |
| Store builds never self-update | `distribution.rs`: `in_app_updates_allowed()` is false for Store and for any packaged process; the updater is compiled out under `cfg(straft_store)` | Good. Keep. |
| Windows: private app data is in the package's own data folder; projects and the Global Idea Vault are in Documents | `distribution.rs` (`packaged_app_data_dir`), comments on Documents | Good. Survives an update. Removed on uninstall (see U5). |
| Notice and licence flags tolerate old and new files | `NoticeRecord` uses `#[serde(rename_all = "camelCase", default)]` | Good. New fields get defaults. |
| Mac: the user's work lives inside the sandbox container | `StorageRules.canRelocate(storeBuild:)` is false for the store build | Good. A Store update keeps the container, as long as the bundle ID never changes. |

## B. Gaps and actions

Marked W (Windows session), M (Mac session) or both. Each needs a test or a written checklist, not only a code change.

- **U1. One version source (both).** Define where the app version comes from (tag or one file). Windows: the MSIX version must rise on every submission and the Store reserves the last part, so use `Major.Minor.Build.0` (confirm when the Partner Center record exists). Mac: `CFBundleVersion` must rise on every upload; `CFBundleShortVersionString` is the public number. Make the build scripts read the one source and fail if the number did not rise.
- **U2. Update simulation (both).** Before the first release, freeze fixtures of what a user's data looks like at the first shipped schema (project schema 19, global 4, app 9, plus `notices.json`, settings, recents), using the existing gold project crate (`straft-gold-project`) for projects. From then on, every release adds a test that opens the previous release's fixture with the new build, migrates, and checks the content. Also write a manual run: install version N from a clean machine, create a project with real content, finish the trial or buy, install N+1 over it (Windows: `Add-AppxPackage` of a higher version with the same identity; Mac: install the next build over the old one), confirm projects, settings, appearance, recents, notices flags, downloaded AI runtime and models, and the licence state are all intact.
- **U3. Names that must never change (both).** Windows: MSIX identity name and publisher, package family name, the `Straft` data folder name inside `LocalState`. Mac: bundle ID `app.straftstudio.studio`, Keychain access group, container-relative folder names. A new bundle ID or identity is a different app with an empty container. Add a golden test that fails if `packaged_app_data_dir` output, the Mac data root name or the product IDs change, and a CI check that the Store config identity equals the value recorded at first submission.
- **U4. Config files survive bad input (both).** For every JSON or small file the app keeps (`notices.json`, appearance and settings, recents, the new Mac guard record, any cache): defaults for missing fields, ignore unknown fields, a version number, and on corruption fall back to defaults and keep the bad file as `*.bad` instead of crashing or silently deleting. Add one test per file with a truncated and a garbage copy. (`NoticeRecord` is already tolerant; verify the others.)
- **U5. Where the safety copies live (both).** Check the caller of `Store::open`'s `safety_backup_dir`. They must sit inside the project folder (or the vault folder), not in the package's private app data, so an uninstall on Windows does not remove the very copy that protects the user. If they are in app data, move them and keep the old path readable. Add a test.
- **U6. Update while the app runs (both).** The Store closes the app to update it. The close path must flush the writer and checkpoint, the same as closing a project (`CloseRequested` handling in `lib.rs`; `applicationShouldTerminate` and the window close path on the Mac). Test by killing the process mid-edit and confirming the crash-recovery path restores the work (`RecoveryDialogs.tsx` and the crash detection in `project.rs` exist; prove them end to end).
- **U7. Downloaded AI runtime and models (both).** They live in app data and must survive updates. When a new app version needs a different engine, download the new one beside the old one, verify its hash from `ai-assets.lock.json`, switch, then remove the old one. Never delete a model on update. Check free disk space before a download and make the download resumable. A failed or missing download must never stop the rest of the app.
- **U8. Derived data never blocks opening (both).** The search and AI indexes are derived. After an update that changes them, rebuild in the background with the existing "Preparing project context" notice. Opening and editing never wait for it.
- **U9. Uninstall wording (both).** The data note (mock 193) and tutorial already say the user is responsible for backups. Add one true sentence per platform. Windows: uninstalling removes the app's private data (settings, downloaded AI files) but not projects kept in Documents. Mac: do not promise anything about the container after the app is deleted; say "back up your root folder".
- **U10. Staged rollout (owner step, document it).** Use Partner Center's gradual rollout for each update, and phased release on the Mac if App Store Connect offers it for macOS apps. A Store update cannot be downgraded by the user, so a bad update must be fixed forward; keep `check-migrations` and the update simulation as release gates.
- **U11. Time-bomb check (both).** Search the code for any behaviour that depends on the build date, the install date or a hard-coded year (licence file dates, certificate dates, `APP_STORE_RELEASE_PLACEHOLDER_MS`-style constants). The first one goes away with the Mac server. Make sure nothing else expires.

## C. Standards checklist before submission (both stores)

Items marked (confirm) are from general practice and must be checked against the store's current policy text during submission. The owner does the portal parts; you make the app ready for them.

**Both**
- Privacy policy URL live and accurate: `https://straftstudio.com/legal/privacy/`. It must state: the app is offline, no accounts, no analytics, no data sent from the app; the optional AI model download comes from the model host (Hugging Face) and what the host can see (the download request); the feedback path is the website contact page and its email. The contact page already says the website has no form and collects nothing; keep that true.
- Support URL live: `https://straftstudio.com/contact/`, showing `support@straftstudio.com`. Company name and address on the website (the separate website prompt covers it).
- Third-party notices: Gemma terms of use and its prohibited-use notice, llama.cpp licence, every Rust and JS dependency (`license-inventory.mjs`, `THIRD_PARTY_LICENSES.md`). Show them in an About or Licences screen and ship the file. The listings must disclose that an AI model is downloaded after install.
- Accurate store description and screenshots of the real app. No claim that the app does not do (no cloud sync, no collaboration over the internet unless it does it).
- Age rating questionnaire answered truthfully (owner).
- No external payment link, no licence key entry, no price typed in the app, no "pay on our website".
- A visible Restore purchase on the Mac; on Windows the Store's own flow plus `Check again`.
- Accessibility: keyboard reachable, screen reader names, contrast in all 15 themes for the new screens.
- Language: English only for now; set the store listings to English.

**Microsoft Store** (confirm)
- Package passes the Windows App Certification Kit.
- Minimum capabilities declared; `runFullTrust` explained in the submission notes.
- WebView2 requirement stated in the notes (Windows 11 ships it; the app explains it if missing).
- Free trial set to 7 days in Pricing and availability.
- Publisher display name is a name the company owns (placeholder; the owner chooses).
- The app does not install or update anything outside the Store.

**Mac App Store** (confirm)
- App Sandbox with the minimum entitlements, privacy manifest, correct export-compliance answer, usage description strings, a 1024 px icon and the required screenshot sizes.
- Review notes with a demo path and the sandbox purchase steps (file 04, M6).
- The Guideline 3.1.1 trial disclosure on the first-launch screen (file 04, M5).
- Hardened runtime and notarization apply to the internal disk image, not to the App Store package.
- The AI engine helper is signed, sandboxed (inherits), and listens only on `127.0.0.1`.
- No download of executable code after install except the model weights (data), which the user starts. Check that the engine binary is bundled in the app, not downloaded; if it is downloaded, that is a review risk and must be changed to bundled. (Today `fetch-macos-runtime.sh` fetches the runtime at build time into the bundle; confirm that stays true for the store build.)

## D. Done when
- U1 to U11 each has a test, a gate or a written manual step, or the handoff says why not.
- A release candidate can be installed over the previous candidate on a clean Windows PC and a clean Mac with every piece of user data intact.
