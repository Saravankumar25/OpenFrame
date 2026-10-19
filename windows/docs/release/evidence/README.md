# Release evidence

Note on names: the folders `M0` and `M1` (and the other milestone folders) and the file names inside them keep the old item codes on purpose, because they are records of what was observed and must stay findable. Folders created after the restructure are named by date and purpose, for example `2026-10-06-foundation/`, and their rows name the build plan area and the bold item title. To translate an old code into the current plain name, use the "Old code to new name" tables in [../../implementation/reconciliation-log.md](../../implementation/reconciliation-log.md).

> Created 2026-10-05 (masterplan M0.25). Evidence is what was **observed on a date** for a named commit: command
> output, reports, screenshots, artifacts. It never states what *must* be true; specifications and ADRs do that
> ([docs/README.md](../../README.md) §1, level 6). The progress log reports; this folder proves.

## 1. Layout

```text
docs/release/evidence/
  README.md          this file: rules and the index template
  M0/ ... M14/       one folder per masterplan milestone; each has an INDEX.md built from the template below
  release/           the final release evidence package (launch register Appendix C, masterplan M13.09)
```

Inside a milestone folder, name files `<box>-<short-name>.<ext>`, for example `M0.18-verify.log`,
`M1.04-migration-fixtures.txt`, `M0.20-visual-baseline/`. Large binaries (installers, videos, model files) are not
committed; record their path, size and SHA-256 in the index instead.

## 2. Rules

1. **Immutable.** An evidence file is never edited after it is recorded. A re-run adds a new dated file and a new
   index row; the old row stays and is marked `superseded by <row>`.
2. **Traceable.** Every row names the masterplan box, the ledger rows it supports, the commit (`git rev-parse HEAD`,
   plus `+dirty` when the working tree had uncommitted changes), the date, the machine and the exact command.
3. **Honest result.** Use only these labels (launch register §2): `PASS`, `FAIL`, `EVIDENCE PENDING`, `BLOCKED`,
   `N/A`, `RISK ACCEPTED`. A failed run is recorded as `FAIL`, never deleted.
4. **No user content.** Never store real user projects, personal data, keys, signing material or unredacted logs.
   Sample databases are generated from the invented demo project only.
5. **Box ticking.** A masterplan box is ticked only with `(ev: docs/release/evidence/<M>/INDEX.md#<row>)` or a PR or
   test link written after it.

## 3. Index template

Copy into `<milestone>/INDEX.md`:

```markdown
# M#: evidence index

| Row | Box | Ledger | Date | Commit | Machine | Command or source | Result | File |
|---|---|---|---|---|---|---|---|---|
| M#-01 | M#.## | ID, ID | YYYY-MM-DD | abc1234(+dirty) | OS, CPU, RAM | `npm run verify` | PASS | [M#.##-verify.log](M#.##-verify.log) |
```
