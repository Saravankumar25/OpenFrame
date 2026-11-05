//! Closes the project cleanly, reopens it and imports the package in fresh data roots, compares the
//! two with the original, and writes `COVERAGE.md` from what the finished database holds.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

const ACTIVITY: &str = "Activity entries";

/// (workspace, label, SQL that counts it).
const COUNTS: &[(&str, &str, &str)] = &[
    (
        "Story",
        "Acts",
        "SELECT count(*) FROM story_act WHERE deleted_at IS NULL",
    ),
    (
        "Story",
        "Sequences",
        "SELECT count(*) FROM story_sequence WHERE deleted_at IS NULL",
    ),
    (
        "Story",
        "Scene cards on the board",
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND parent_type='sequence'",
    ),
    (
        "Story",
        "Parking lot cards",
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL AND parent_type='parking'",
    ),
    (
        "Story",
        "Beats",
        "SELECT count(*) FROM story_beat WHERE deleted_at IS NULL",
    ),
    (
        "Story",
        "Characters",
        "SELECT count(*) FROM story_character WHERE deleted_at IS NULL",
    ),
    (
        "Story",
        "Relationships",
        "SELECT count(*) FROM story_character_relationship",
    ),
    (
        "Idea Vault",
        "Items",
        "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL",
    ),
    (
        "Idea Vault",
        "Voice memos",
        "SELECT count(*) FROM vault_item WHERE deleted_at IS NULL AND item_type='voice'",
    ),
    (
        "Idea Vault",
        "Folders",
        "SELECT count(*) FROM vault_folder WHERE deleted_at IS NULL",
    ),
    (
        "Idea Vault",
        "Collections",
        "SELECT count(*) FROM vault_collection WHERE deleted_at IS NULL",
    ),
    (
        "Screenplay",
        "Screenplays",
        "SELECT count(*) FROM screenplay WHERE deleted_at IS NULL",
    ),
    (
        "Screenplay",
        "Drafts",
        "SELECT count(*) FROM screenplay_draft d JOIN screenplay p ON p.id=d.screenplay_id WHERE d.deleted_at IS NULL AND p.deleted_at IS NULL",
    ),
    (
        "Screenplay",
        "Comments",
        "SELECT count(*) FROM comment WHERE deleted_at IS NULL",
    ),
    (
        "Screenplay",
        "Private notes",
        "SELECT count(*) FROM private_note WHERE deleted_at IS NULL",
    ),
    (
        "Breakdown",
        "Breakdown elements",
        "SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL",
    ),
    (
        "Breakdown",
        "Catalog items",
        "SELECT count(*) FROM catalog_item WHERE deleted_at IS NULL",
    ),
    (
        "Production",
        "Locations",
        "SELECT count(*) FROM location WHERE deleted_at IS NULL",
    ),
    (
        "Production",
        "Location photos",
        "SELECT count(*) FROM location_photo WHERE deleted_at IS NULL",
    ),
    (
        "Production",
        "Cast members",
        "SELECT count(*) FROM cast_member WHERE deleted_at IS NULL",
    ),
    (
        "Production",
        "Crew members",
        "SELECT count(*) FROM crew_member WHERE deleted_at IS NULL",
    ),
    (
        "Visual",
        "Moodboards",
        "SELECT count(*) FROM moodboard WHERE deleted_at IS NULL",
    ),
    (
        "Visual",
        "Storyboards",
        "SELECT count(*) FROM storyboard WHERE deleted_at IS NULL",
    ),
    (
        "Visual",
        "Storyboard panels",
        "SELECT count(*) FROM storyboard_panel WHERE deleted_at IS NULL",
    ),
    (
        "Visual",
        "Shots",
        "SELECT count(*) FROM shot WHERE deleted_at IS NULL",
    ),
    (
        "Schedule",
        "Schedules",
        "SELECT count(*) FROM shooting_schedule WHERE deleted_at IS NULL",
    ),
    (
        "Paperwork",
        "Call sheets, every revision",
        "SELECT count(*) FROM call_sheet WHERE deleted_at IS NULL",
    ),
    (
        "Paperwork",
        "Saved sides",
        "SELECT count(*) FROM side WHERE deleted_at IS NULL",
    ),
    (
        "Paperwork",
        "Saved reports",
        "SELECT count(*) FROM production_report WHERE deleted_at IS NULL",
    ),
    (
        "Project pages",
        "Notes",
        "SELECT count(*) FROM project_note WHERE deleted_at IS NULL",
    ),
    (
        "Project pages",
        "Tasks",
        "SELECT count(*) FROM task WHERE deleted_at IS NULL",
    ),
    (
        "Project pages",
        "Files",
        "SELECT count(*) FROM project_file WHERE deleted_at IS NULL",
    ),
    (
        "Project pages",
        ACTIVITY,
        "SELECT count(*) FROM sys_activity",
    ),
];

#[derive(Clone, Default, PartialEq)]
struct Finger {
    title: String,
    current_scenes: usize,
    drafts: BTreeMap<String, usize>,
    characters: i64,
    breakdown: i64,
    days: usize,
    shooting_days: usize,
    unscheduled: usize,
    planned: i64,
    contingency: i64,
    budget_lines: usize,
    counts: BTreeMap<String, i64>,
    ai_exchanges: usize,
    ai_visible: usize,
}

fn fingerprint(g: &Gold) -> Finger {
    let mut f = Finger::default();
    let cur = g.ok("project.current", json!({}));
    f.title = s(&cur["title"]);
    let ov = g.ok("screenplay.overview", json!({}));
    for d in arr(&ov["drafts"]) {
        let doc = g.ok("screenplay.document", json!({ "draftId": d["id"] }));
        let n = arr(&doc["scenes"]).len();
        f.drafts.insert(s(&d["name"]), n);
        if d["isCurrent"] == json!(true) || d["current"] == json!(true) {
            f.current_scenes = n;
        }
    }
    let sv = g.ok("schedule.get", json!({}));
    let days = arr(&sv["days"]);
    f.days = days.len();
    f.shooting_days = sv["counts"]["days"].as_u64().unwrap_or(0) as usize;
    f.unscheduled = arr(&sv["unscheduled"]).len();
    let b = g.ok("budget.get", json!({}));
    let c = &b["current"];
    f.planned = c["plannedLinesTotal"].as_i64().unwrap_or(-1);
    f.contingency = c["contingencyAmount"].as_i64().unwrap_or(-1);
    f.budget_lines = arr(&c["lines"]).len();
    f.characters = g.sql_i64("SELECT count(*) FROM story_character WHERE deleted_at IS NULL");
    f.breakdown = g.sql_i64("SELECT count(*) FROM breakdown_element WHERE deleted_at IS NULL");
    for (_, label, sql) in COUNTS {
        f.counts.insert((*label).to_string(), g.sql_i64(sql));
    }
    // Rows in the project. The `ai.history` read shows them only to the person who asked.
    f.ai_exchanges = g.sql_i64("SELECT count(*) FROM ai_request WHERE deleted_at IS NULL") as usize;
    let hist = g.ok("ai.history", json!({ "conversationId": null, "limit": 50 }));
    f.ai_visible = arr(&hist["exchanges"]).len();
    f
}

fn differences(a: &Finger, b: &Finger) -> Vec<String> {
    let mut out = Vec::new();
    macro_rules! cmp {
        ($name:expr, $x:expr, $y:expr) => {
            if $x != $y {
                out.push(format!("{}: {:?} against {:?}", $name, $x, $y));
            }
        };
    }
    cmp!("title", a.title, b.title);
    cmp!("current draft scenes", a.current_scenes, b.current_scenes);
    cmp!("drafts", a.drafts, b.drafts);
    cmp!("characters", a.characters, b.characters);
    cmp!("breakdown elements", a.breakdown, b.breakdown);
    cmp!("schedule days", a.days, b.days);
    cmp!("planned lines total", a.planned, b.planned);
    cmp!("contingency", a.contingency, b.contingency);
    // A package is written before the last save and backup, so it lacks their Activity rows.
    let mut ca = a.counts.clone();
    let mut cb = b.counts.clone();
    ca.remove(ACTIVITY);
    cb.remove(ACTIVITY);
    cmp!("counts", ca, cb);
    cmp!("Straft AI exchanges", a.ai_exchanges, b.ai_exchanges);
    out
}

fn fmt_finger(f: &Finger) -> Value {
    json!({ "title": f.title, "currentDraftScenes": f.current_scenes, "draftScenes": f.drafts, "characters": f.characters, "breakdownElements": f.breakdown,
            "scheduleDays": f.days, "shootingDays": f.shooting_days, "unscheduled": f.unscheduled, "plannedLinesPaise": f.planned, "contingencyPaise": f.contingency,
            "budgetLines": f.budget_lines, "aiExchanges": f.ai_exchanges, "aiVisibleToThisUser": f.ai_visible })
}

fn walk_size(p: &Path) -> (u64, u64) {
    let mut bytes = 0;
    let mut files = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            let q = e.path();
            if q.is_dir() {
                let (b, n) = walk_size(&q);
                bytes += b;
                files += n;
            } else if let Ok(m) = q.metadata() {
                bytes += m.len();
                files += 1;
            }
        }
    }
    (bytes, files)
}

pub fn finish(g: &Gold, ctx: &Ctx, work: &Path, build_secs: u64) {
    let project_path = ctx.id("projectPath");
    println!("== Verify: fingerprint of the original");
    let source = fingerprint(g);
    g.ok("project.close", json!({}));
    g.shutdown();
    let leftovers: Vec<String> = std::fs::read_dir(&project_path)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.contains("lock") || n.ends_with("-wal") || n.ends_with("-shm"))
                .collect()
        })
        .unwrap_or_default();

    // Reopen the folder itself on a fresh data root.
    let version = env!("CARGO_PKG_VERSION");
    let a = Gold::open_as(&work.join("verify-open"), None, version);
    a.phase("Verify: reopen the project folder");
    a.ok("project.open", json!({ "path": project_path }));
    let reopened = fingerprint(&a);
    let db_tables: Vec<String> = a
        .sql_rows("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .iter()
        .map(|r| s(&r[0]))
        .collect();
    let mut schedule_lines = Vec::new();
    let sv = a.ok("schedule.get", json!({}));
    let mut covered = 0usize;
    for d in arr(&sv["days"]) {
        covered += arr(&d["items"])
            .iter()
            .filter(|it| !it["strip"].is_null())
            .count();
        schedule_lines.push(s(&d["date"]));
    }
    let bd = a.ok("breakdown.scenes", json!({ "filter": "complete" }));
    let complete = arr(&bd["scenes"]).len();
    let by_category = a.sql_rows("SELECT category, count(*) FROM breakdown_element WHERE deleted_at IS NULL GROUP BY category ORDER BY category");
    let dood = a.ok("schedule.dood", json!({}));
    let dood_rows = arr(&dood["rows"]).len() + arr(&dood["cast"]).len();
    let home = a.ok("project.home", json!({}));
    let trash = a.ok("trash.list", json!({ "store": "project" }));
    let recovery = a.ok("project.recovery_state", json!({}));
    let story_days = a.sql_i64("SELECT count(*) FROM screenplay_scene s JOIN screenplay_draft d ON d.id=s.draft_id JOIN screenplay p ON p.id=d.screenplay_id WHERE p.current_draft_id=d.id AND s.deleted_at IS NULL AND s.story_day IS NOT NULL");
    a.ok("project.close", json!({}));
    a.shutdown();
    let after_open: Vec<String> = std::fs::read_dir(&project_path)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.contains("lock") || n.ends_with("-wal") || n.ends_with("-shm"))
                .collect()
        })
        .unwrap_or_default();

    // Import the full project package on another fresh data root.
    let b = Gold::open_as(&work.join("verify-package"), None, version);
    b.phase("Verify: import the full project package");
    let started = b.ok(
        "packages.import_project",
        json!({ "path": ctx.id("projectPackage"), "mode": "open", "parentDir": null }),
    );
    let import = b.wait_task(&started);
    let imported = import.as_ref().ok().map(|_| fingerprint(&b));
    b.shutdown();

    // Restore the backup on a third data root.
    let c = Gold::open_as(&work.join("verify-backup"), None, version);
    c.phase("Verify: restore the backup");
    let started = c.ok(
        "packages.import_project",
        json!({ "path": ctx.id("finalBackup"), "mode": "open", "parentDir": null }),
    );
    let restore = c.wait_task(&started);
    let restored = restore.as_ref().ok().map(|_| fingerprint(&c));
    c.shutdown();

    let reopen_diff = differences(&source, &reopened);
    let import_diff = imported
        .as_ref()
        .map_or(vec!["import failed".to_string()], |f| {
            differences(&source, f)
        });
    let restore_diff = restored
        .as_ref()
        .map_or(vec!["restore failed".to_string()], |f| {
            differences(&source, f)
        });

    let mut failures: Vec<String> = g
        .failures
        .borrow()
        .iter()
        .map(|f| {
            format!(
                "{} / {}: {} {} {}",
                f.phase, f.op, f.code, f.message, f.detail
            )
        })
        .collect();
    for gg in [&a, &b, &c] {
        failures.extend(gg.failures.borrow().iter().map(|f| {
            format!(
                "{} / {}: {} {} {}",
                f.phase, f.op, f.code, f.message, f.detail
            )
        }));
    }
    let ops_used = g
        .counts
        .borrow()
        .iter()
        .filter(|(_, c)| c.ok + c.expected_err > 0)
        .count();
    let calls: u32 = g.counts.borrow().values().map(|c| c.ok).sum();
    let exports = ctx.facts.get("exports").map(|v| arr(v).len()).unwrap_or(0);

    let mut md = String::new();
    let _ = writeln!(md, "# {}: demo project coverage\n", ctx.title);
    let _ = writeln!(
        md,
        "Counted from the finished project database after it was closed and reopened on a fresh data root. Nothing here is typed by hand.\n"
    );
    let _ = writeln!(
        md,
        "- Operations used: {ops_used}. Successful calls: {calls}. Build time: {build_secs} s."
    );
    let _ = writeln!(
        md,
        "- Calls that failed although they were expected to work: {}.",
        failures.len()
    );
    let _ = writeln!(md, "- Exported files recorded: {exports}.\n");
    let _ = writeln!(md, "## Scenes per draft\n");
    for (name, n) in &reopened.drafts {
        let _ = writeln!(md, "- {name}: {n} scenes");
    }
    let mut section = "";
    for (ws, label, _) in COUNTS {
        if *ws != section {
            let _ = writeln!(md, "\n## {ws}\n");
            section = ws;
        }
        let _ = writeln!(
            md,
            "- {label}: {}",
            reopened.counts.get(*label).copied().unwrap_or(-1)
        );
    }
    let _ = writeln!(md, "\n## Breakdown by category\n");
    for r in &by_category {
        let _ = writeln!(md, "- {}: {}", s(&r[0]), r[1]);
    }
    let _ = writeln!(md, "- Scenes marked complete: {complete}");
    let _ = writeln!(md, "\n## Schedule and budget\n");
    let _ = writeln!(
        md,
        "- Scenes with a Story Day in the current draft: {story_days}."
    );
    let _ = writeln!(
        md,
        "- Calendar days: {}. Shooting days: {}. Strips on days: {covered}. Unscheduled strips: {}.",
        reopened.days, reopened.shooting_days, reopened.unscheduled
    );
    let _ = writeln!(md, "- Day Out of Days rows: {dood_rows}.");
    let _ = writeln!(
        md,
        "- Budget lines: {}. Planned total of the lines: {} paise. Contingency: {} paise.",
        reopened.budget_lines, reopened.planned, reopened.contingency
    );
    let _ = writeln!(md, "\n## Straft AI\n");
    let _ = writeln!(
        md,
        "- Exchanges saved in the project: {}. Shown to the user who asked: {}. Shown to a new user on another installation: {}.",
        reopened.ai_exchanges, source.ai_visible, reopened.ai_visible
    );
    if let Some(skip) = ctx.facts.get("aiSkipped") {
        let _ = writeln!(md, "- Skipped: {}", s(skip));
    }
    let _ = writeln!(md, "\n## Verification\n");
    let _ = writeln!(
        md,
        "- Reopened folder on a fresh data root: {}.",
        if reopen_diff.is_empty() {
            "identical".to_string()
        } else {
            reopen_diff.join("; ")
        }
    );
    let _ = writeln!(
        md,
        "- Full project package imported on a fresh data root: {}.",
        if import_diff.is_empty() {
            "identical".to_string()
        } else {
            import_diff.join("; ")
        }
    );
    let _ = writeln!(
        md,
        "- Backup restored on a fresh data root: {}.",
        if restore_diff.is_empty() {
            "identical".to_string()
        } else {
            restore_diff.join("; ")
        }
    );
    let _ = writeln!(
        md,
        "- Files beside the database after the last close: {}. The empty `.lock` file is the single-writer lock; the OS releases it when the project closes.",
        after_open.join(", ")
    );
    let _ = writeln!(
        md,
        "- Activity rows: {} in the folder, {} in the package. The package is written before the last save and backup.",
        reopened.counts.get(ACTIVITY).copied().unwrap_or(-1),
        imported
            .as_ref()
            .and_then(|f| f.counts.get(ACTIVITY).copied())
            .unwrap_or(-1)
    );
    std::fs::write(ctx.out.join("COVERAGE.md"), &md).expect("COVERAGE.md");

    let (bytes, files) = walk_size(Path::new(&project_path));
    let summary = json!({
        "projectPath": project_path, "projectFiles": files, "projectBytes": bytes, "failures": failures,
        "source": fmt_finger(&source), "reopened": fmt_finger(&reopened), "imported": imported.as_ref().map(fmt_finger), "restored": restored.as_ref().map(fmt_finger),
        "reopenDifferences": reopen_diff, "importDifferences": import_diff, "restoreDifferences": restore_diff,
        "recoveryState": recovery, "scenesWithStoryDay": story_days, "leftoversAfterClose": leftovers, "leftoversAfterReopen": after_open, "tables": db_tables, "coveredStrips": covered, "doodRows": dood_rows,
        "completeScenes": complete, "homeKeys": home.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()), "trashRows": arr(&trash).len(),
        "facts": ctx.facts, "packageImportError": import.err().map(|e| e.message), "backupRestoreError": restore.err().map(|e| e.message),
    });
    let dump = work.join("run-summary.json");
    std::fs::write(
        &dump,
        serde_json::to_string_pretty(&summary).unwrap_or_default(),
    )
    .expect("summary");
    println!(
        "done in {build_secs}s. {} unexpected failures. Summary: {}",
        summary["failures"].as_array().map_or(0, Vec::len),
        dump.display()
    );
}
