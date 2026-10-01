# 23 — Release, Migration & Compatibility

> **Status legend:** ✅ implemented · 🚧 in development · 📋 planned.
> Source: `persistence/src/migrate.rs`, `application/src/{schema.rs,store.rs}`, `project-format`,
> `openframe-domain` version constants. Related: 11-physical-database.md §7, ADR-0002, ADR-0003.

## 1. Version identifiers

| Identifier | Where | Current | Changes when |
|---|---|---|---|
| App version (SemVer) | `Cargo.toml [workspace.package].version`, `package.json`, `apps/desktop/package.json`, `tauri.conf.json` | 0.1.0 | Every release. All four must match; `release-validate` checks this and the git tag `vX.Y.Z`. |
| Project schema version | `PRAGMA user_version` = number of project migrations; mirrored in `openframe.json.schemaVersion` | 9 | A migration is appended |
| Project format version | `PROJECT_FORMAT_VERSION`, `openframe.json.formatVersion` | 1 | Folder layout or manifest changes incompatibly |
| Package format version | `PACKAGE_FORMAT_VERSION` | 1 | Package container changes incompatibly |
| AI compatibility version | `AI_COMPATIBILITY_VERSION`, `openframe.json.aiCompatibilityVersion` | 1 | AI tool or Change Set schemas change incompatibly |
| Global vault / app DB schema | `user_version` of `vault.sqlite` / `app.sqlite` | 2 / 1 | A migration is appended |

## 2. Compatibility policy

- **Forward (old app, new project): refused safely.** A project with a newer `formatVersion` or schema is **never
  modified**. The message is "This project was saved by a newer version of OpenFrame. Update OpenFrame to open it —
  the project has not been changed." (`project_format.too_new`) ✅. Read-only opening of newer projects is not
  supported in v1.
- **Backward (new app, old project): always supported.** Every shipped schema version must open and migrate in every
  later release. There are no "too old" cut-offs without an ADR.
- **Packages:** an app reads packages with `formatVersion ≤` its own. Newer packages are refused with guidance to
  update. Packages written by the app always carry the current version.
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
   recorded, and drift is logged at open. Before the first public release, module migration files may still be edited
   (pre-release rule).
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

## 5. Release process

1. Bump the version in all four files (one commit). Update the changelog. Run `npm run licenses` if dependencies
   changed.
2. `node scripts/release-validate.mjs --report-only` locally.
3. Tag `vX.Y.Z` on `master` → `.github/workflows/release.yml`:
   - **validate** job (fail closed): versions/tag, license selected, signing secrets present, updater configured,
     license inventory clean, `cargo deny check`;
   - **build** job: full verification, Authenticode import, `tauri build` (NSIS + MSI, updater artifacts),
     signature verification, CycloneDX SBOMs, license inventory, SHA256SUMS, **draft** GitHub release.
4. A human reviews the draft (installers on clean VMs, QA gates G6–G12), then publishes it and the update manifest.

**Current blockers** (release-validate report): no license selected (LICENSE-PENDING.md), no signing certificate or
updater key configured, updater plugin not configured in `tauri.conf.json`.

## 6. Channels and updates 📋

- One `stable` channel in v1. Pre-releases use SemVer pre-release tags (`v0.2.0-rc.1`) and are published as GitHub
  pre-releases, never offered to stable users.
- In-app updates: notify, then download and install only after explicit approval (Engineering Index). See
  24-windows-packaging-updater.md.
- Model updates are separate from app updates (Local AI spec §15).

## 7. Offline AI release prerequisites ⛔

Builds include Offline AI. These must be done before a public release; items 1–2 are enforced by
`scripts/release-validate.mjs` (checks 6 and 6b), the rest are manual.

1. **Production manifest signing key.** Set `OPENFRAME_MANIFEST_PUBLIC_KEY` (base64 Ed25519) at build time. The gate
   fails when it is missing, malformed or equal to the development key (`crates/openframe-ai/keys/manifest-dev.pub`).
   A release build never trusts the embedded development-signed manifest.
2. **Model distribution base URL.** Set `MODEL_DISTRIBUTION_BASE_URL` (https) at build time. At install the app
   fetches `<base>/manifest.json` and `<base>/manifest.json.sig` from it.
3. **Production manifest.** Publish a format-2 manifest (profile `openframe-local-ai-v1`, `channel` ≠ `dev`, `sequence`
   higher than any shipped one) signed with the production key (`node crates/openframe-ai/tools/sign-manifest.mjs`),
   listing runtime, Gemma and BGE files with exact bytes and SHA-256. The development manifest points at GitHub and
   Hugging Face; the production one must point at the OpenFrame-controlled host (Engineering Index §7 open decision).
4. **Gemma redistribution terms.** Hosting Gemma on the OpenFrame endpoint is redistribution under the Gemma Terms of
   Use: keep the notice "Gemma is provided under and subject to the Gemma Terms of Use found at
   ai.google.dev/gemma/terms." in the app (Settings → Offline AI → Technical details) and `THIRD_PARTY_LICENSES.md`,
   pass the Terms and the Gemma Prohibited Use Policy on to recipients (e.g. alongside the files on the host and in
   the product's notices), and have the owner confirm the terms were reviewed. BGE small (MIT), llama.cpp (MIT) and
   the LLVM OpenMP runtime (Apache-2.0 WITH LLVM-exception) notices ship as well.
5. **Privacy policy.** Fill in the model download host and its log retention (`docs/store/privacy-policy.md`).
6. **Hardware validation** (QA gate G13): benchmark and `real_runtime` on minimum (8 GB RAM) and recommended hardware,
   and the Vulkan build on a discrete GPU — not yet done (runtime spec §16).
7. **Store listing** (Store builds): disclose the on-demand AI download and generative AI, and provide a
   report-inappropriate-content channel (27-microsoft-store.md §5.4, §10).
