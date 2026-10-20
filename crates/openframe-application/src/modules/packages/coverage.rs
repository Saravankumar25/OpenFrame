//! Package coverage: what every project table contributes to each package class.
//!
//! The project schema grows with every migration. A package writer that silently skips a new
//! table or column loses the user's data on restore or transfer, so coverage is a checked
//! contract, not a convention:
//!
//! - **Backup Package** (`.ofbackup`): a consistent copy of the whole project database, so every
//!   table and column travels. Nothing here can leave anything out of a backup.
//! - **Full Project Package** (`.ofproject`): the same copy, then [`scrub_for_transfer`] removes
//!   what must not reach another person (other members' private notes, undo history). Each table
//!   says whether it is kept, reduced to the exporting user's rows, emptied, or handled by
//!   dedicated code, always with the reason.
//! - **Exchange packages** (review, Story Board, breakdown, shots, schedule, call sheet and
//!   response): an explicit subset. Each table says which columns the exchange writer
//!   (`exchange.rs`) carries and why every other column is left out, or why the table is not
//!   carried at all.
//!
//! [`coverage_problems`] compares this list with a freshly migrated project database and names
//! every table or column that has no decision, and every decision that names something the
//! schema no longer has. The test `package_coverage_follows_the_project_schema` fails on any
//! problem, so a migration that adds data cannot merge until its package behaviour is decided.

use std::collections::{BTreeMap, BTreeSet};

use openframe_domain::AppResult;
use rusqlite::Connection;

use super::format::PackageType;

/// How a Full Project Package treats a table (a Backup Package always keeps every table whole).
#[derive(Debug, Clone, Copy)]
pub enum Transfer {
    /// The whole table travels.
    Kept,
    /// Only rows whose `owner_column` is NULL or the exporting user travel.
    OwnRowsOnly {
        owner_column: &'static str,
        why: &'static str,
    },
    /// No rows travel.
    Emptied { why: &'static str },
    /// Dedicated code in [`scrub_for_transfer`] adjusts the table.
    Special { why: &'static str },
}

/// Whether exchange packages carry data from a table.
#[derive(Debug, Clone, Copy)]
pub enum Exchange {
    Carried {
        /// The exchange package types whose content holds this table's rows.
        packages: &'static [PackageType],
        /// Columns the exchange writer reads to build the package: values in `content.json`,
        /// row order, row selection or the base snapshot used to detect stale content.
        columns: &'static [&'static str],
        /// Every other column, with the reason it stays out. Row bookkeeping
        /// ([`BOOKKEEPING`]) is covered for every carried table.
        left_out: &'static [(&'static str, &'static str)],
    },
    NotCarried {
        why: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct TableCoverage {
    pub table: &'static str,
    pub transfer: Transfer,
    pub exchange: Exchange,
}

/// Columns every carried table may leave out without its own entry.
pub const BOOKKEEPING: &[(&str, &str)] = &[
    (
        "created_at",
        "row bookkeeping; the receiving project stamps its own times",
    ),
    (
        "updated_at",
        "row bookkeeping; packages carry `rev` to detect stale content",
    ),
    (
        "deleted_at",
        "deleted rows are never sent; the column only filters them out",
    ),
];

const ALL_EXCHANGE: &[PackageType] = &[
    PackageType::ScriptReview,
    PackageType::Story,
    PackageType::Response,
];
const SCRIPT: &[PackageType] = &[PackageType::ScriptReview, PackageType::Response];
const STORY: &[PackageType] = &[PackageType::Story];
/// Owner change list, 2026-10-19: these package types are no longer exported. Packages made earlier still import.
const REMOVED_PACKAGES: &str =
    "the Breakdown, Shot List, Schedule and Call Sheet Review packages are no longer exported";

// Reasons shared by several tables.
const LOCAL_HISTORY: &str = "local history of this project; a receiving project keeps its own";
const SYSTEM_STATE: &str = "system state of this installation, not project content";
const DERIVED_INDEX: &str = "derived search index; rebuilt from project content";
const NOT_REVIEWED: &str =
    "not part of any exchange package in this format; travels in Backup and Full Project Packages";
const AI_HISTORY: &str = "Offline AI history stays on the computer that made it";
const PACKAGE_STATE: &str = "records of packages this project received; meaningless elsewhere";

const fn kept() -> Transfer {
    Transfer::Kept
}
const fn not_carried(why: &'static str) -> Exchange {
    Exchange::NotCarried { why }
}
const fn t(table: &'static str, transfer: Transfer, exchange: Exchange) -> TableCoverage {
    TableCoverage {
        table,
        transfer,
        exchange,
    }
}

/// One entry per project table. Keep it in schema order of the migration that adds the table.
pub const COVERAGE: &[TableCoverage] = &[
    // 0001 core
    t(
        "project",
        kept(),
        not_carried(
            "the project row identifies the source project; exchange manifests carry its id and title",
        ),
    ),
    t(
        "project_member",
        kept(),
        not_carried("members and roles belong to each project"),
    ),
    t(
        "asset",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &["id", "original_name"],
            left_out: &[
                ("storage_mode", "the file itself travels as a package entry"),
                ("rel_path", "a location inside the source project folder"),
                ("external_path", "a location on the sender's computer"),
                (
                    "media_type",
                    "recomputed when the file is added to the receiving project",
                ),
                (
                    "byte_size",
                    "recomputed when the file is added to the receiving project",
                ),
                ("sha256", "the package checksum list covers the file"),
                (
                    "width",
                    "recomputed when the file is added to the receiving project",
                ),
                (
                    "height",
                    "recomputed when the file is added to the receiving project",
                ),
                (
                    "duration_ms",
                    "recomputed when the file is added to the receiving project",
                ),
                ("last_seen_at", "local file health check time"),
                ("rev", "files are copied as new, never updated in place"),
            ],
        },
    ),
    t(
        "comment",
        kept(),
        Exchange::Carried {
            packages: ALL_EXCHANGE,
            columns: &[
                "id",
                "parent_id",
                "target_type",
                "target_id",
                "scene_id",
                "quoted_text",
                "body",
                "status",
                "author_user_id",
                "author_name",
            ],
            left_out: &[
                ("review_round_id", "review rounds are local planning"),
                (
                    "anchor_json",
                    "text anchors are rebuilt from the quoted text on import",
                ),
                ("context_moved", "recomputed against the receiving project"),
                (
                    "resolved_by",
                    "resolution is local; the status column travels",
                ),
                (
                    "resolved_at",
                    "resolution is local; the status column travels",
                ),
                ("rev", "comments are added, never updated in place"),
            ],
        },
    ),
    t(
        "private_note",
        Transfer::OwnRowsOnly {
            owner_column: "owner_user_id",
            why: "another member's private notes never leave their project (Security spec 8.7, review finding PN-02)",
        },
        not_carried("private notes are never in exchange packages (Security spec 8.6)"),
    ),
    t(
        "deleted_item",
        Transfer::Special {
            why: "tombstones of removed private notes would name them; dropped with the notes",
        },
        not_carried("Recently Deleted is local; deleted rows are never sent"),
    ),
    t("snapshot", kept(), not_carried(NOT_REVIEWED)),
    t("template", kept(), not_carried(NOT_REVIEWED)),
    t("project_file_folder", kept(), not_carried(NOT_REVIEWED)),
    t("project_file", kept(), not_carried(NOT_REVIEWED)),
    t(
        "project_note",
        kept(),
        not_carried("project notes are internal to the production team"),
    ),
    t(
        "task",
        kept(),
        not_carried("tasks are internal to the production team"),
    ),
    t(
        "search_doc",
        Transfer::OwnRowsOnly {
            owner_column: "owner_user_id",
            why: "index rows of other members' private notes would copy their text",
        },
        not_carried(DERIVED_INDEX),
    ),
    t(
        "search_fts",
        Transfer::Special {
            why: "rebuilt from the scrubbed search_doc so no removed text stays in the index",
        },
        not_carried(DERIVED_INDEX),
    ),
    t(
        "search_fts_data",
        Transfer::Special {
            why: "shadow table of search_fts, rebuilt with it",
        },
        not_carried(DERIVED_INDEX),
    ),
    t(
        "search_fts_idx",
        Transfer::Special {
            why: "shadow table of search_fts, rebuilt with it",
        },
        not_carried(DERIVED_INDEX),
    ),
    t(
        "search_fts_docsize",
        Transfer::Special {
            why: "shadow table of search_fts, rebuilt with it",
        },
        not_carried(DERIVED_INDEX),
    ),
    t(
        "search_fts_config",
        Transfer::Special {
            why: "shadow table of search_fts, rebuilt with it",
        },
        not_carried(DERIVED_INDEX),
    ),
    t("sys_activity", kept(), not_carried(LOCAL_HISTORY)),
    t(
        "sys_undo",
        Transfer::Emptied {
            why: "undo history holds row images of old text and other members' changes",
        },
        not_carried(LOCAL_HISTORY),
    ),
    t(
        "sys_view_state",
        kept(),
        not_carried("screen layout preferences are local"),
    ),
    t("sqlite_sequence", kept(), not_carried(SYSTEM_STATE)),
    t(
        "_schema_migrations",
        kept(),
        not_carried(
            "the migration record of this project database; a restore checks it and migrates an older database forward",
        ),
    ),
    // 0002 idea vault
    t("vault_folder", kept(), not_carried(NOT_REVIEWED)),
    t("vault_collection", kept(), not_carried(NOT_REVIEWED)),
    t("vault_item", kept(), not_carried(NOT_REVIEWED)),
    t("vault_item_collection", kept(), not_carried(NOT_REVIEWED)),
    t("vault_item_tag", kept(), not_carried(NOT_REVIEWED)),
    // 0003 story
    t("season", kept(), not_carried(NOT_REVIEWED)),
    t("episode", kept(), not_carried(NOT_REVIEWED)),
    t(
        "story_act",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &["id", "title", "note", "position", "rev"],
            left_out: &[
                (
                    "episode_id",
                    "episodes are not part of a Story Board package in this format",
                ),
                ("collapsed", "a display preference of the sender"),
            ],
        },
    ),
    t(
        "story_sequence",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &["id", "act_id", "title", "note", "position", "rev"],
            left_out: &[
                (
                    "episode_id",
                    "episodes are not part of a Story Board package in this format",
                ),
                ("collapsed", "a display preference of the sender"),
            ],
        },
    ),
    t(
        "story_beat",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &[
                "id",
                "parent_type",
                "parent_id",
                "text",
                "note",
                "color",
                "position",
                "rev",
            ],
            left_out: &[
                (
                    "episode_id",
                    "episodes are not part of a Story Board package in this format",
                ),
                (
                    "state",
                    "beat working state is the sender's; the beat's place on the board travels",
                ),
                (
                    "converted_scene_card_id",
                    "a link inside the sender's board",
                ),
                (
                    "source_vault_item_id",
                    "a link into the sender's Idea Vault",
                ),
                ("parked_from_type", "parking history of the sender"),
                ("parked_from_id", "parking history of the sender"),
                ("parked_from_index", "parking history of the sender"),
                ("parked_at", "parking history of the sender"),
            ],
        },
    ),
    t(
        "story_scene_card",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &[
                "id",
                "parent_type",
                "parent_id",
                "short_description",
                "scene_heading",
                "notes",
                "color",
                "long_description",
                "position",
                "rev",
            ],
            left_out: &[
                (
                    "episode_id",
                    "episodes are not part of a Story Board package in this format",
                ),
                ("screenplay_scene_id", "a link into the sender's screenplay"),
                ("source_beat_id", "a link inside the sender's board"),
                (
                    "source_vault_item_id",
                    "a link into the sender's Idea Vault",
                ),
                ("parked_from_type", "parking history of the sender"),
                ("parked_from_id", "parking history of the sender"),
                ("parked_from_index", "parking history of the sender"),
                ("parked_at", "parking history of the sender"),
            ],
        },
    ),
    t(
        "story_attachment",
        kept(),
        Exchange::Carried {
            packages: STORY,
            columns: &["id", "owner_type", "owner_id", "asset_id", "position"],
            left_out: &[(
                "rev",
                "attachments are copied as new, never updated in place",
            )],
        },
    ),
    t("story_character", kept(), not_carried(NOT_REVIEWED)),
    t(
        "story_character_card_link",
        kept(),
        not_carried(NOT_REVIEWED),
    ),
    t(
        "story_character_relationship",
        kept(),
        not_carried(NOT_REVIEWED),
    ),
    // 0004 screenplay
    t(
        "screenplay",
        kept(),
        Exchange::Carried {
            packages: SCRIPT,
            columns: &["id", "title", "current_draft_id"],
            left_out: &[
                (
                    "episode_id",
                    "episodes are not part of a review package in this format",
                ),
                (
                    "format",
                    "page format is a property of the sender's project",
                ),
                ("title_page_json", "the title page is not reviewed"),
                (
                    "page_setup_json",
                    "page layout is a property of the sender's project; the reviewer reads the text, not pages",
                ),
                ("rev", "the draft revision identifies the reviewed version"),
            ],
        },
    ),
    t(
        "screenplay_draft",
        kept(),
        Exchange::Carried {
            packages: SCRIPT,
            columns: &["id", "screenplay_id", "name", "status", "rev"],
            left_out: &[
                ("note", "draft notes are the author's working notes"),
                ("created_from_draft_id", "draft lineage is local history"),
                ("locked_at", "lock state belongs to the author's project"),
                ("locked_by", "lock state belongs to the author's project"),
                (
                    "locked_by_name",
                    "lock state belongs to the author's project",
                ),
                (
                    "revision_label",
                    "revision sets are not part of a review package in this format",
                ),
                (
                    "revision_color",
                    "revision sets are not part of a review package in this format",
                ),
                (
                    "revision_reason",
                    "revision sets are not part of a review package in this format",
                ),
            ],
        },
    ),
    t(
        "screenplay_scene",
        kept(),
        Exchange::Carried {
            packages: SCRIPT,
            columns: &[
                "id",
                "draft_id",
                "lineage_id",
                "position",
                "heading",
                "synopsis",
                "production_number",
                "rev",
            ],
            left_out: &[
                ("notes", "scene working notes stay with the author"),
                (
                    "story_day",
                    "story timing is not part of a review package in this format",
                ),
                (
                    "time_note",
                    "story timing is not part of a review package in this format",
                ),
                (
                    "source_scene_card_id",
                    "a link into the sender's Story Board",
                ),
                (
                    "omitted",
                    "omitted scenes are deleted from the reviewed text; only live scenes are sent",
                ),
                (
                    "production_number_source",
                    "how the number was set is the sender's history; the number itself travels",
                ),
            ],
        },
    ),
    t(
        "screenplay_element",
        kept(),
        Exchange::Carried {
            packages: SCRIPT,
            columns: &[
                "id",
                "scene_id",
                "position",
                "element_type",
                "text",
                "custom_element_id",
                "rev",
            ],
            left_out: &[
                (
                    "dual",
                    "dual dialogue layout does not change the reviewed text",
                ),
                (
                    "revision_mark",
                    "revision marks are not part of a review package in this format",
                ),
                (
                    "import_source",
                    "which file the element was imported from is the sender's history",
                ),
            ],
        },
    ),
    t(
        "screenplay_character_link",
        kept(),
        not_carried(NOT_REVIEWED),
    ),
    t(
        "screenplay_history_point",
        kept(),
        not_carried(LOCAL_HISTORY),
    ),
    t(
        "review_round",
        kept(),
        not_carried("review rounds are local planning"),
    ),
    t("sys_screenplay_sync", kept(), not_carried(SYSTEM_STATE)),
    // 0005 production
    t("production_source", kept(), not_carried(REMOVED_PACKAGES)),
    t(
        "production_scene_state",
        kept(),
        not_carried("production state is recomputed in the receiving project"),
    ),
    t("location", kept(), not_carried(NOT_REVIEWED)),
    t("location_photo", kept(), not_carried(NOT_REVIEWED)),
    t("catalog_item", kept(), not_carried(REMOVED_PACKAGES)),
    t("catalog_alias", kept(), not_carried(NOT_REVIEWED)),
    t("breakdown_element", kept(), not_carried(REMOVED_PACKAGES)),
    t(
        "breakdown_dismissed",
        kept(),
        not_carried("dismissed suggestions are a local choice"),
    ),
    t(
        "cast_member",
        kept(),
        not_carried("cast contact details never leave the project in an exchange package"),
    ),
    t(
        "crew_member",
        kept(),
        not_carried("crew contact details never leave the project in an exchange package"),
    ),
    t("budget_snapshot", kept(), not_carried(NOT_REVIEWED)),
    t("budget_line", kept(), not_carried(NOT_REVIEWED)),
    // 0006 visual
    t("moodboard", kept(), not_carried(NOT_REVIEWED)),
    t(
        "moodboard_item",
        kept(),
        not_carried(
            "not part of any exchange package in this format; items marked private stay private in exports",
        ),
    ),
    t("storyboard", kept(), not_carried(NOT_REVIEWED)),
    t("storyboard_panel", kept(), not_carried(NOT_REVIEWED)),
    t("shot", kept(), not_carried(REMOVED_PACKAGES)),
    // 0007 schedule
    t("shooting_schedule", kept(), not_carried(REMOVED_PACKAGES)),
    t("shooting_day", kept(), not_carried(REMOVED_PACKAGES)),
    t("schedule_strip", kept(), not_carried(REMOVED_PACKAGES)),
    t("schedule_marker", kept(), not_carried(REMOVED_PACKAGES)),
    t(
        "schedule_warning_ack",
        kept(),
        not_carried("warning decisions are local"),
    ),
    t("call_sheet", kept(), not_carried(REMOVED_PACKAGES)),
    t("side", kept(), not_carried(NOT_REVIEWED)),
    t("production_report", kept(), not_carried(NOT_REVIEWED)),
    // 0008 AI
    t("ai_request", kept(), not_carried(AI_HISTORY)),
    t(
        "ai_result",
        Transfer::Special {
            why: "an unfinished AI apply can't be compensated without undo history and is marked failed (ADR-0019)",
        },
        not_carried(AI_HISTORY),
    ),
    t("ai_tool_invocation", kept(), not_carried(AI_HISTORY)),
    t(
        "change_set",
        Transfer::Special {
            why: "an unfinished AI apply can't be compensated without undo history and is marked failed (ADR-0019)",
        },
        not_carried(AI_HISTORY),
    ),
    // 0009 collaboration
    t("sys_package_log", kept(), not_carried(LOCAL_HISTORY)),
    t("sys_import_session", kept(), not_carried(PACKAGE_STATE)),
    t("review_queue_item", kept(), not_carried(PACKAGE_STATE)),
    t("exchange_review_record", kept(), not_carried(PACKAGE_STATE)),
    // 0011 AI apply journal
    t(
        "sys_ai_apply_journal",
        Transfer::Special {
            why: "open journals are closed as failed because undo history is not sent (ADR-0019)",
        },
        not_carried(AI_HISTORY),
    ),
    // 0013 screenplay engine
    t(
        "screenplay_custom_element",
        kept(),
        Exchange::Carried {
            packages: SCRIPT,
            columns: &["id", "name", "look", "uppercase", "italic"],
            left_out: &[
                (
                    "screenplay_id",
                    "the package carries one screenplay; the element is matched by id",
                ),
                ("shortcut", "a keyboard shortcut of the sender"),
                (
                    "space_before",
                    "page layout; the reviewer reads the text, not pages",
                ),
                ("next_element", "an editing behaviour of the sender"),
                (
                    "export_name",
                    "file export naming; review packages are not exports",
                ),
                ("rev", "element types are read only in a review"),
            ],
        },
    ),
    t(
        "screenplay_page_preset",
        kept(),
        not_carried("page presets are a writing setting of the sender"),
    ),
    t(
        "screenplay_revision",
        kept(),
        not_carried("revision sets are not part of a review package in this format"),
    ),
    t(
        "screenplay_lock_baseline",
        kept(),
        not_carried("the locked page baseline belongs to the author's project"),
    ),
    // 0014 production depth
    t(
        "breakdown_category_style",
        kept(),
        not_carried("category colours and icons are a display choice of each project"),
    ),
    t(
        "availability_window",
        kept(),
        not_carried(
            "cast, crew and location availability is not part of any exchange package in this format; travels in Backup and Full Project Packages",
        ),
    ),
    // 0015 schedule timeline and Day Out of Days corrections
    t(
        "dood_override",
        kept(),
        not_carried(
            "Day Out of Days corrections are the sender's own edits to a derived report; they travel in Backup and Full Project Packages",
        ),
    ),
];

/// The coverage entry of a table.
pub fn for_table(table: &str) -> Option<&'static TableCoverage> {
    COVERAGE.iter().find(|c| c.table == table)
}

/// `table.column` entries an exchange package of this type carries (format 2 manifests list
/// them so a reader can tell a value that was empty from one the writer did not carry).
pub fn carried_by(ty: PackageType) -> Vec<String> {
    let mut out = BTreeSet::new();
    for c in COVERAGE {
        if let Exchange::Carried {
            packages, columns, ..
        } = c.exchange
            && packages.contains(&ty)
        {
            for col in columns {
                out.insert(format!("{}.{col}", c.table));
            }
        }
    }
    out.into_iter().collect()
}

/// Tables and their columns as SQLite reports them.
pub fn schema_columns(c: &Connection) -> AppResult<BTreeMap<String, Vec<String>>> {
    let mut stmt = c.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = BTreeMap::new();
    for t in tables {
        let mut stmt = c.prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")?;
        let cols: Vec<String> = stmt
            .query_map([&t], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        out.insert(t, cols);
    }
    Ok(out)
}

/// Every gap between the schema of `c` and [`COVERAGE`], as sentences naming the table or
/// column and what to do. Empty when the package writers cover the whole schema.
pub fn coverage_problems(c: &Connection) -> AppResult<Vec<String>> {
    let schema = schema_columns(c)?;
    let mut problems = vec![];
    let mut seen = BTreeSet::new();
    for cov in COVERAGE {
        if !seen.insert(cov.table) {
            problems.push(format!(
                "table `{}` is listed twice in the package coverage",
                cov.table
            ));
        }
    }
    for (table, cols) in &schema {
        let Some(cov) = for_table(table) else {
            problems.push(format!(
                "table `{table}` is not covered by the package writer: add it to COVERAGE in \
                 crates/openframe-application/src/modules/packages/coverage.rs with its Full \
                 Project Package rule and whether exchange packages carry it (or why not)"
            ));
            continue;
        };
        if let Transfer::OwnRowsOnly { owner_column, .. } = cov.transfer
            && !cols.iter().any(|c| c == owner_column)
        {
            problems.push(format!(
                "table `{table}` keeps own rows by column `{owner_column}`, which the schema does not have"
            ));
        }
        let Exchange::Carried {
            packages,
            columns,
            left_out,
        } = cov.exchange
        else {
            continue;
        };
        if packages.is_empty() {
            problems.push(format!(
                "table `{table}` is carried by no exchange package type"
            ));
        }
        let mut decided: BTreeMap<&str, &str> = BTreeMap::new();
        for col in columns {
            if decided.insert(col, "carried").is_some() {
                problems.push(format!("column `{table}.{col}` is listed twice"));
            }
        }
        for (col, why) in left_out.iter().chain(BOOKKEEPING.iter()) {
            if why.trim().is_empty() {
                problems.push(format!(
                    "column `{table}.{col}` is left out without a reason"
                ));
            }
            if decided.insert(col, "left out").is_some()
                && !BOOKKEEPING.iter().any(|(b, _)| b == col)
            {
                problems.push(format!("column `{table}.{col}` is listed twice"));
            }
        }
        for col in cols {
            if !decided.contains_key(col.as_str()) {
                problems.push(format!(
                    "column `{table}.{col}` is not covered by the exchange package writer: list \
                     it as carried (and write it in exchange.rs) or as left out with a reason"
                ));
            }
        }
        for col in columns.iter().chain(left_out.iter().map(|(c, _)| c)) {
            if !cols.iter().any(|c| c == col) {
                problems.push(format!(
                    "column `{table}.{col}` is in the package coverage but not in the schema"
                ));
            }
        }
    }
    for cov in COVERAGE {
        if !schema.contains_key(cov.table) {
            problems.push(format!(
                "table `{}` is in the package coverage but not in the schema",
                cov.table
            ));
        }
    }
    Ok(problems)
}

/// Turn a copy of the project database into a Full Project Package database for
/// `exporter_user_id`, following [`COVERAGE`]. Runs inside one transaction on the copy, then
/// rebuilds the search index and compacts the file so removed text is not left in free pages.
pub(crate) fn scrub_for_transfer(c: &Connection, exporter_user_id: &str) -> AppResult<()> {
    c.execute_batch("BEGIN IMMEDIATE;")?;
    let scrub = (|| -> AppResult<()> {
        let present: BTreeSet<String> = schema_columns(c)?.into_keys().collect();
        for cov in COVERAGE {
            if !present.contains(cov.table) {
                // A copy made by an older schema simply lacks the table.
                continue;
            }
            match cov.transfer {
                Transfer::OwnRowsOnly { owner_column, .. } => {
                    c.execute(
                        &format!(
                            "DELETE FROM \"{}\" WHERE \"{owner_column}\" IS NOT NULL AND \"{owner_column}\" <> ?1",
                            cov.table
                        ),
                        [exporter_user_id],
                    )?;
                }
                Transfer::Emptied { .. } => {
                    c.execute(&format!("DELETE FROM \"{}\"", cov.table), [])?;
                }
                Transfer::Kept | Transfer::Special { .. } => {}
            }
        }
        // Special: tombstones of private notes that no longer exist in the copy.
        c.execute(
            "DELETE FROM deleted_item WHERE table_name='private_note' AND object_id NOT IN (SELECT id FROM private_note)",
            [],
        )?;
        // Special: without history an unfinished AI apply can no longer be compensated (ADR-0019).
        crate::modules::ai::change_set::abandon_open_journals(c)?;
        // Special: the external-content full-text index keeps tokens of deleted rows in its
        // segments until it is rebuilt from the scrubbed search_doc.
        if present.contains("search_fts") {
            c.execute("INSERT INTO search_fts(search_fts) VALUES('rebuild')", [])?;
        }
        Ok(())
    })();
    match scrub {
        Ok(()) => c.execute_batch("COMMIT;")?,
        Err(e) => {
            let _ = c.execute_batch("ROLLBACK;");
            return Err(e);
        }
    }
    // Deleted rows stay readable in free pages until the file is rewritten. SQLite runs VACUUM
    // through an internal ATTACH, which the hardened connection forbids (limit 0, PKG-03). The
    // limit is lifted for this one statement only; VACUUM runs no trigger or view SQL, so no
    // SQL from the project can use it, and it is restored even when VACUUM fails.
    use rusqlite::limits::Limit;
    c.execute_batch("PRAGMA secure_delete=ON;")?;
    c.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 1)?;
    let vacuum = c.execute_batch("VACUUM;");
    c.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
    vacuum?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_coverage_follows_the_project_schema() {
        let mut c = openframe_persistence::open_in_memory().unwrap();
        openframe_persistence::migrate::apply(&mut c, crate::schema::PROJECT_MIGRATIONS).unwrap();
        let problems = coverage_problems(&c).unwrap();
        assert!(
            problems.is_empty(),
            "the package writers do not cover the project schema ({} problems):\n  - {}",
            problems.len(),
            problems.join("\n  - ")
        );
    }

    #[test]
    fn coverage_check_names_a_new_table_and_a_new_column() {
        let mut c = openframe_persistence::open_in_memory().unwrap();
        openframe_persistence::migrate::apply(&mut c, crate::schema::PROJECT_MIGRATIONS).unwrap();
        c.execute_batch(
            "CREATE TABLE example_new_table (id TEXT PRIMARY KEY, amount INTEGER);
             ALTER TABLE story_scene_card ADD COLUMN mood TEXT;",
        )
        .unwrap();
        let problems = coverage_problems(&c).unwrap();
        assert!(
            problems
                .iter()
                .any(|p| p.starts_with("table `example_new_table` is not covered")),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.starts_with("column `story_scene_card.mood` is not covered")),
            "{problems:?}"
        );
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn every_carried_column_is_written_by_the_exchange_writer() {
        // A cheap guard against a coverage entry that claims a column the writer never reads.
        let writer = include_str!("exchange.rs");
        for cov in COVERAGE {
            if let Exchange::Carried { columns, .. } = cov.exchange {
                for col in columns {
                    assert!(
                        writer.contains(col),
                        "coverage says exchange packages carry `{}.{col}` but exchange.rs never names it",
                        cov.table
                    );
                }
            }
        }
    }

    #[test]
    fn manifests_list_what_each_exchange_type_carries() {
        let script = carried_by(PackageType::ScriptReview);
        assert!(script.contains(&"screenplay_element.text".to_string()));
        assert!(script.contains(&"comment.body".to_string()));
        assert!(!script.iter().any(|c| c.starts_with("private_note.")));
        assert!(carried_by(PackageType::Backup).is_empty());
    }
}
