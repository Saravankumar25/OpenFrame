# SQL build-site audit (release gate S2 and S3)

> 2026-10-17, branch `windows/w9-endgame` at the commit that follows this page. Method: a script listed every `format!`
> call whose text holds an SQL keyword in non-test Rust code under `crates/` and `apps/desktop/src-tauri/src` (154 sites);
> a read-only security review then traced every interpolated value to its origin and searched for SQL built by
> concatenation, `push_str`, `.join(` and `.replace(`, and for non-literal `prepare`, `query_row` and `execute_batch`
> arguments (5 more sites). Fixes for the findings are listed at the end with their tests. Results after the fixes: `--test ai` 102 passed, `--test writing` 218, `--lib` 962, `--test production` 145, `openframe-persistence` 25; clippy clean.

## Result

No user-controlled value is placed into SQL text. Every FTS5 `MATCH` input is reduced to quoted terms. Three findings,
none an injection, all fixed in this branch.

| Class | Values |
|---|---|
| CONST (compile-time constant or literal fragment) | 130 |
| PLACEHOLDERS (generated `?` lists or counts) | 9 |
| IDENT_FIXED (name from an enum match or static list) | 29 |
| IDENT_SCHEMA (name read from `sqlite_master` or `pragma_table_info`, quoted) | 25 |
| IDENT_VALIDATED (name from stored rows or IPC, checked before use) | 12 |
| USER_VALUE | 0 |
| UNCLEAR | 0 |

Defence in depth already in place: both connections set `PRAGMA trusted_schema=OFF` (`openframe-persistence/src/lib.rs`,
`openframe-search/src/db.rs`); a received project's schema names are screened by `safe_schema_name` and
`untrusted_schema_problems` before use. There is no SQL in `apps/desktop/src-tauri/src`.

## Values that are not constants (paths under `crates/openframe-application/src/modules/` unless named)

| Site | Value | Class | Origin and check |
|---|---|---|---|
| ai/queries.rs:22, 597, 599 | table | IDENT_VALIDATED | `valid_ident` and `table_exists` in `row_rev`; origin a stored change set or a literal |
| comments/mod.rs:375, 409, 416 | table, target_type | IDENT_VALIDATED | `is_identifier` and `table_exists`; origin IPC or stored comment rows |
| comments/mod.rs:680 | where_sql, status_sql | CONST | literal clauses joined, values bound; status from match arms |
| files.rs:714, 726 | t, col | IDENT_SCHEMA | `sqlite_master` and `pragma_table_info` names, quotes doubled |
| idea_vault/model.rs:475 | where_sql, order_sql, limit_sql | CONST | `view_sql` literals; folder, collection and tag values bound |
| packages/backup.rs:303 | t | IDENT_VALIDATED | package schema screened by `untrusted_schema_problems` before this point |
| packages/import.rs:440, 442 | t | IDENT_FIXED | `normalize_target` returns a `TRACKED_TARGETS` element |
| production/breakdown.rs:1763, 1775, 1781 | raw, t | IDENT_SCHEMA | `sqlite_master` names, quotes doubled |
| project.rs:1979, 2000 | table, t | IDENT_VALIDATED | `table_exists`, quotes doubled or stripped |
| retention.rs:177 | table | IDENT_FIXED | handler's static table matched to the stored row; quotes doubled |
| store.rs:913, 1063 | spec.table, handler.table | IDENT_FIXED | literals or `kind.table()`; quotes stripped or doubled |
| store.rs:975, 1005, 1029 | table | IDENT_VALIDATED | only after `restore_deleted` matched a registered handler |
| story/index.rs:630, 639 | t | IDENT_SCHEMA | `sqlite_master` names |
| ai/toolbox/resolve.rs:241, 363 | entity fields, col, table | CONST, IDENT_FIXED | `&'static str` from the `entity!` macro |
| visual/common.rs:213, 252 | title, alive | CONST | `&'static` literals in `VaultQuery` |
| openframe-persistence/src/rows.rs, undo.rs | t, sets, names, lit, new, old | IDENT_SCHEMA, PLACEHOLDERS | `quote_ident` doubles quotes; column names from the schema |
| openframe-search/src/vector.rs:143 | dim | PLACEHOLDERS | a `usize` |

The remaining 130 values are constants (`*_COLS`, `PRODUCTION_STATES`, `DRAFT_SQL`, `LIVE_SCENE`, `VEC_TABLE` and
similar) or literal if/else fragments.

## FTS5 MATCH (all safe)

| Site | Helper | How user text is neutralised |
|---|---|---|
| modules/search.rs:95 | `fts_query` | split on whitespace, ends trimmed, `"` doubled, each term quoted with `*`, AND, at most 12 terms, 500-character cap |
| idea_vault/search.rs:78 | `search::fts_query` | same helper |
| ai/tools.rs:1431 | local `fts_query` | split on every non-alphanumeric; letters and digits only; at most 8 terms |
| ai/toolbox/reads.rs:564 | `lexical_hits` | split on non-alphanumeric, terms of 3 or more characters, at most 10 |
| ai/intelligence/retriever.rs:152 | `fts_or_query` | `"` doubled, every term quoted; unit test covers `NEAR("x" OR y)` |
| openframe-search/src/vector.rs:127 | n/a | sqlite-vec `MATCH` with a bound vector, not FTS5 |

There is no `GLOB` in non-test code.

## Findings and fixes

| Id | Finding | Severity | Fix | Test |
|---|---|---|---|---|
| F1 | Four AI read tools built LIKE patterns that stripped `%` but left `_` as a wildcard and had no `ESCAPE` clause (`ai/toolbox/reads.rs` notes and vault search, `reads_production.rs` catalog and locations). Values were bound, so no injection; a search for `a_c` also matched "abc" | Low | Use the shared `like_pattern` helper (escapes `\`, `%`, `_`) with `ESCAPE '\'` | `like_searches_treat_underscore_and_percent_as_plain_text` (tests/ai/ai_toolbox.rs): failed first (the abc note matched), then passed |
| F2 | Three readers of stored table names (`comments::target_state`, `project::describe_row`, `queries::row_rev`) checked only that the name is an identifier and the table exists. A received project naming a table without an `id` column (for example `sqlite_master`) made the read fail and broke the whole comment listing | Low | Treat a table without an `id` column as "target unavailable" | `a_stored_target_table_without_an_id_column_does_not_break_listing` (tests/writing/comments.rs): failed first with "no such column: id", then passed; unit tests `row_rev_ignores_a_table_without_an_id_column`, `a_table_without_an_id_column_is_not_available` |
| H1 | Ten sites put already-validated names into SQL without doubling quotes, safe only because of upstream checks | Hardening | Quote with the shared `quote_ident` | Covered by the F2 tests and the existing suites |

Not covered by this audit: test code, and the bundled SQLite version itself (tracked by `cargo audit`).
