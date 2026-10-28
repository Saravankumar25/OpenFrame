# 23: Release, Migration & Compatibility

> **Current decisions (2026-10-15).** The release flow is Windows track, internal `.exe`, team pilot, then Mac track, internal `.dmg`, team pilot, then website, then stores ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The team password gate was removed on 2026-10-27; internal builds open without a password. Models download after install ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)); a changed model is a manifest version change ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)).

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `persistence/src/migrate.rs`, `application/src/{schema.rs,store.rs}`, `project-format`,
> `openframe-domain` version constants. Related: 11-physical-database.md §7, ADR-0002, ADR-0003.

## 1. Version identifiers

| Identifier | Where | Current | Changes when |
|---|---|---|---|
| App version (SemVer) | `Cargo.toml [workspace.package].version`, `package.json`, `apps/desktop/package.json`, `tauri.conf.json` | 0.1.0 | Every release. All four must match; `release-validate` checks this and the git tag `vX.Y.Z`. |
| Project schema version | `PRAGMA user_version` = number of project migrations; mirrored in `openframe.json.schemaVersion` | 19 (`0001_core` to `0019_story_page_format`; 10 corrected from 9 on 2026-10-05, 11 added by the crash-safe AI changes item of the build plan, 12 to 14 by the Beat Board, screenplay engine and production depth areas, 15 by the shooting schedule area, 16 by the call sheets, budget, templates, samples and exports area, 17 by the Story page, 18 by the assistant's memory, 19 by the Story page formatting) | A migration is appended |
| Project format version | `PROJECT_FORMAT_VERSION`, `openframe.json.formatVersion` | 1 | Folder layout or manifest changes incompatibly |
| Package format version | `PACKAGE_FORMAT_VERSION` (oldest read: `OLDEST_PACKAGE_FORMAT_VERSION`) | 2 | Package container or manifest changes; add a `format-N` fixture folder |
| AI compatibility version | `AI_COMPATIBILITY_VERSION`, `openframe.json.aiCompatibilityVersion` | 1 | AI tool or Change Set schemas change incompatibly |
| Global vault / app DB schema | `user_version` of `vault.sqlite` / `app.sqlite` | 3 / 1 (global corrected from 2 on 2026-10-05: `0001_core`, `0002_idea_vault`, `0003_performance`) | A migration is appended |

## 2. Compatibility policy

- **Forward (old app, new project): refused safely.** A project with a newer `formatVersion` or schema is **never
  modified**. The message is "This project was saved by a newer version of OpenFrame. Update OpenFrame to open it.
  The project has not been changed." (`project_format.too_new`) ✅. Read-only opening of newer projects is not
  supported in the first release.
- **Backward (new app, old project): always supported.** Every shipped schema version must open and migrate in every
  later release. There are no "too old" cut-offs without an ADR.
- **Packages:** an app reads packages with `formatVersion ≤` its own. Newer packages are refused with guidance to
  update. Packages written by the app always carry the current version. Each format has committed fixtures
  (`crates/openframe-application/tests/fixtures/packages/format-N/`); a backup fixture holds the database at the
  schema it was written with, so restoring it also exercises every later migration.
- **Downgrade** after migration is not supported. The pre-migration safety copy in `backups/` is the rollback path,
  and it opens in the old version.

## 3. Migration procedure ✅ (on project open)

```text
quick_check ──fail──► refuse (project_format.corrupt), no changes
   │
plan: Fresh | UpToDate | Upgrade{from,to} | TooNew
   │ TooNew ──► refuse (project_format.too_new), no changes
   │ Upgrade
   ▼
safety backup: backups/pre-migration-v<from>-<ms>.sqlite   (online backup → .partial → quick_check → rename)
   ▼
BEGIN; foreign_keys=OFF; apply pending migrations in order; record in _schema_migrations (checksum)
foreign_key_check + quick_check  ──fail──► ROLLBACK (database exactly as before; foreign_keys=ON restored)
user_version = to; COMMIT
   ▼
manifest.schemaVersion/appVersion updated atomically; search index rebuilt; open continues
```

The global vault and app databases use the same runner. The app database has no safety backup: it holds no user
content, and recents can be rebuilt by opening projects.

## 4. Rules for writing migrations

1. **Append only.** Once a release containing migration N ships, `migrations/*/000N_*.sql` is frozen. Checksums are
   recorded, and drift is logged at open. **Freeze point (decided 2026-10-05, reconciliation C10):** files `0001` to
   `0010` (project), `0001` to `0003` (global) and `0001` (app) become frozen when the **first build is given to anyone
   outside the development team** (internal release candidate or beta). Until then edits are allowed but discouraged.
   Every later schema change is a new numbered file, linear on `master`, with a fixture test from the previous version.

   **Enforcement (build plan: migration policy, 2026-10-05).**
   - `migrations/frozen.json` records the SHA-256 (LF-normalised) of every migration file and a `frozen` flag.
   - `scripts/check-migrations.mjs` (part of `npm run check:governance`, blocking in CI) fails when numbering in any
     set is not `1..N` without gaps or duplicates, when `crates/openframe-application/src/schema.rs` does not embed
     exactly the files on disk in order, or when a recorded file is deleted. While `frozen` is `false` an edited
     file is a warning; once `frozen` is `true` it is an error.
   - The pull request that adds a migration runs `node scripts/check-migrations.mjs --record` and commits the
     updated `frozen.json`.
   - The change that produces the first build for anyone outside the development team sets `"frozen": true`.
     `scripts/release-validate.mjs` fails closed until it is set ("Migration freeze" check).
   - **Allocation:** new numbers are taken in merge order on `master`. Parallel branches reserve a number in the
     build plan item (for example the Beat Board long scene description takes project `0011`) and the second branch to merge renumbers; the check
     catches duplicates. Allocations: project `0011` was taken by the ADR-0019 apply journal (crash-safe AI changes, merged first);
     the Beat Board long scene description took project `0012` (`0012_story_card_and_vault_research.sql`, which also
     adds the Idea Vault research flag; the Global Idea Vault gets the same column in global `0004_vault_research.sql`);
     the Revision 1 screenplay schema (Screenplay engine and file formats area) took `0013`; production depth took `0014`;
     the shooting schedule day timeline and Day Out of Days took `0015` (`0015_schedule_timeline.sql`), call sheets and the budget took `0016` (`0016_callsheets_budget.sql`), the Story page took `0017` (`0017_story_page.sql`), the assistant's memory took `0018` (`0018_ai_memory.sql`) and the Story page formatting took `0019` (`0019_story_page_format.sql`).
2. Each migration is plain SQL executed inside the runner's transaction. Do not use `BEGIN`/`COMMIT` or
   `PRAGMA foreign_keys` inside it.
3. Table rebuilds: `CREATE TABLE new_x … ; INSERT INTO new_x SELECT … ; DROP TABLE x; ALTER TABLE new_x RENAME TO x;`
   then recreate indexes and triggers. Keep `id`, `rev`, `created_at` and `updated_at` values. Never renumber ids.
4. Hub contract columns (11-physical-database.md §5) are never renamed or removed. Add a new column and deprecate
   the old one.
5. New canonical tables follow the conventions: `id TEXT PK`, no BLOB, no cascade, `rev`, timestamps, `deleted_at`
   where the user can delete. The `schema.rs` test enforces undo-trackability.
6. Data migrations must be deterministic and idempotent within their transaction, and must never drop user content.
   Obsolete data is kept in a renamed column or table until a later major version, with an ADR.
7. **Tests:** every new migration adds a fixture database at the previous version (📋 `tests/fixtures/schema-vN.sqlite`)
   and a test that opens, migrates, verifies content and verifies that the safety backup exists.
   Failure paths (full drive, read-only file, newer schema, damaged database, unwritable safety backup) are covered
   by `crates/openframe-application/tests/migration_failures.rs`, and an interrupted migration by `forced_kill.rs`.

## 5. Release process

1. Bump the version in all four files (one commit). Update the changelog. Run `npm run licenses` if dependencies
   changed.
2. `node scripts/release-validate.mjs --report-only` locally.
3. Tag `vX.Y.Z` on `master` → `.github/workflows/release.yml`:
   - **validate** job (fail closed): versions/tag, end-user licence agreement in place, signing secrets present, updater configured,
     license inventory clean, `cargo deny check`;
   - **build** job: full verification, Authenticode import, `tauri build` (NSIS + MSI, updater artifacts),
     signature verification, CycloneDX SBOMs, license inventory, SHA256SUMS, **draft** GitHub release.
4. A human reviews the draft (installers on clean VMs; the must-ship acceptance, migration, performance, crash and recovery, accessibility, installer and offline gates in [22-qa-test-strategy.md](22-qa-test-strategy.md)), then publishes it and the update manifest.

**Current blockers** (release-validate report): no end-user licence agreement yet (`LICENSE-PENDING.md` is still present), no signing certificate or
updater key configured, updater plugin not configured in `tauri.conf.json`.

## 6. Channels and updates 📋

- One `stable` channel in the first release. Pre-releases use SemVer pre-release tags (`v0.2.0-rc.1`) and are published as GitHub
  pre-releases, never offered to stable users.
- In-app updates: notify, then download and install only after explicit approval (Engineering Index). See
  24-windows-packaging-updater.md.
- Model updates ship as new pins in a new app version (ADR-0032); an update that keeps the same models keeps the
  download. The runtime
  is part of the app version (ADR-0020). Local AI spec §15 describes the download flow.

## 7. Offline AI release prerequisites ⛔

Design (ADR-0026 and ADR-0032): the package holds the app and the AI runtime. The Gemma 3 1B chat model and the BGE
embedding model download on the user's click straight from Hugging Face at the commits the lock pins, each checked by
size and SHA-256. There is no model host and no signing key. The items below are the release gates. Item 1 is enforced by `scripts/release-validate.mjs` (check 6) and by every package build; the rest are manual.

1. **AI build inputs.** The runtime and model files pinned in `packaging/ai/ai-assets.lock.json` (path, bytes,
   SHA-256, licence) must be present for the build (`.ai-assets/`, or `OPENFRAME_AI_ASSETS_DIR` on a build machine or
   runner). `scripts/prepare-ai-assets.mjs` verifies them and stages them; `npm run build`, `npm run bundle`,
   `build-msix.mjs`, the E2E test build and the release workflow fail closed when a file is missing or altered. The
   CI runners do not have these files yet, so the release jobs stop at this check until they do.
2. **Redistribution check.** Shipping Gemma inside every package is redistribution under the Gemma Terms of Use: the
   notice "Gemma is provided under and subject to the Gemma Terms of Use found at ai.google.dev/gemma/terms." ships in
   the app (Settings → Offline AI → Technical details, and `ai/notices/` in the package) and in
   `THIRD_PARTY_LICENSES.md`; the Terms and the Gemma Prohibited Use Policy are passed on to recipients; the owner
   confirms the check before the model host goes live and before any package leaves the team (ADR-0026 decision 9). BGE small (MIT), llama.cpp (MIT) and
   the LLVM OpenMP runtime (Apache-2.0 WITH LLVM-exception) notices ship as well.
3. **Package size.** Today's build includes the AI (1,149,734,816 bytes uncompressed for Windows x64; about 1.07 GB
   compressed). After the download flow is rebuilt, the package holds the app and the runtime only (about 20 to 30 MB,
   an estimate). Record the real size of each package when it is first built (ADR-0026 Consequences).
4. **Validation per format** (ADR-0026 decision 10): install with no network, first download, offline inference after
   the download, upgrade and
   uninstall on a clean machine for each package format before it is claimed as supported.
5. **Hardware validation** (the Offline AI bundle gate in 22-qa-test-strategy.md): benchmark, `real_runtime` and
   `ai_real_model` on minimum (8 GB RAM) and recommended hardware with the bundled processor build. Not yet done
   (runtime spec §16).
6. **Store listing** (Store builds): describe the generative AI and its one-time user-clicked model download, offline use after it, and provide a
   report-inappropriate-content channel (27-microsoft-store.md §5.4, §10).
