//! Integrity and recoverability: the full project package is imported and the backup restored
//! into fresh data roots, the project is reopened in a fresh core, and a crash is simulated;
//! each copy is compared with the original.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

#[derive(Default)]
pub struct Copy {
    pub name: String,
    pub how: String,
    pub opened: bool,
    pub error: String,
    pub differences: Vec<String>,
    /// Differences the format documents (for example undo history not travelling).
    pub expected: Vec<String>,
    pub tables_compared: usize,
    pub rows_compared: i64,
    pub assets_compared: usize,
}

#[derive(Default)]
pub struct Report {
    pub source: Fingerprint,
    pub copies: Vec<Copy>,
    pub package_files: Vec<(String, u64, String)>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.copies.iter().all(|c| c.opened && c.differences.is_empty())
    }
}

#[derive(Default, Clone)]
pub struct Fingerprint {
    pub project_id: String,
    pub title: String,
    pub tables: BTreeMap<String, i64>,
    pub drafts: BTreeMap<String, (usize, usize, String)>,
    pub budget: Value,
    pub schedule: Vec<String>,
    pub assets: BTreeMap<String, String>,
    pub relations: BTreeMap<String, i64>,
    pub reads: BTreeMap<String, Value>,
}

fn sha(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, root, out);
        } else if let Ok(bytes) = std::fs::read(&p) {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.insert(rel, sha(&bytes));
        }
    }
}

/// Everything compared between the original and a copy.
pub fn fingerprint(g: &Gold) -> Fingerprint {
    let mut f = Fingerprint::default();
    let cur = g.ok("project.current", json!({}));
    f.project_id = s(&cur["id"]);
    f.title = s(&cur["title"]);
    for row in g.sql_rows("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name") {
        let t = s(&row[0]);
        let n = g.sql_i64(&format!("SELECT count(*) FROM \"{t}\""));
        f.tables.insert(t, n);
    }
    let ov = g.ok("screenplay.overview", json!({}));
    for d in arr(&ov["drafts"]) {
        let id = s(&d["id"]);
        let doc = g.ok("screenplay.document", json!({ "draftId": id }));
        let mut h = Sha256::new();
        let mut els = 0;
        for sc in arr(&doc["scenes"]) {
            h.update(s(&sc["label"]).as_bytes());
            h.update(s(&sc["heading"]).as_bytes());
            for e in arr(&sc["elements"]) {
                h.update(s(&e["elementType"]).as_bytes());
                h.update(s(&e["text"]).as_bytes());
                h.update(if e["dual"] == json!(true) { b"d" } else { b"-" });
                h.update(s(&e["revisionMark"]).as_bytes());
                els += 1;
            }
        }
        f.drafts.insert(s(&d["name"]), (arr(&doc["scenes"]).len(), els, hex::encode(h.finalize())));
    }
    let b = g.ok("budget.get", json!({}));
    let c = &b["current"];
    f.budget = json!({ "currency": c["currency"], "plannedTotal": c["plannedTotal"], "plannedLinesTotal": c["plannedLinesTotal"],
        "actualTotal": c["actualTotal"], "contingencyAmount": c["contingencyAmount"], "lines": arr(&c["lines"]).len(), "snapshots": arr(&b["snapshots"]).len() });
    let sv = g.ok("schedule.get", json!({}));
    for d in arr(&sv["days"]) {
        let scenes: Vec<String> = arr(&d["items"])
            .iter()
            .map(|it| if it["strip"].is_null() { format!("[{}]", s(&it["marker"]["label"])) } else { s(&it["strip"]["number"]).to_string() + &it["strip"]["number"].to_string() })
            .collect();
        f.schedule.push(format!("{} {} {}", s(&d["label"]), s(&d["date"]), scenes.join(" ")));
    }
    if let Ok(p) = g.core.project() {
        let root = p.layout.root().to_path_buf();
        let mut all = BTreeMap::new();
        walk(&root, &root, &mut all);
        // The database, its journal files, the lock and the session marker differ by nature.
        f.assets = all
            .into_iter()
            .filter(|(k, _)| {
                !(k.ends_with(".sqlite") || k.ends_with(".sqlite-wal") || k.ends_with(".sqlite-shm") || k.ends_with(".db")
                    || k.ends_with("-wal") || k.ends_with("-shm") || k.contains("lock") || k.contains("session") || k.starts_with("logs/")
                    || k.contains("backups/") || k.contains("cache"))
            })
            .collect();
    }
    for (name, sql) in [
        ("card character links", "SELECT count(*) FROM story_character_card_link"),
        ("story relationships", "SELECT count(*) FROM story_character_relationship"),
        ("breakdown elements with catalog item", "SELECT count(*) FROM breakdown_element WHERE catalog_item_id IS NOT NULL AND deleted_at IS NULL"),
        ("shots linked to panels", "SELECT count(*) FROM storyboard_panel WHERE shot_id IS NOT NULL AND deleted_at IS NULL"),
        ("cast members with a character", "SELECT count(*) FROM cast_member WHERE deleted_at IS NULL"),
        ("comments", "SELECT count(*) FROM comment WHERE deleted_at IS NULL"),
        ("private notes", "SELECT count(*) FROM private_note WHERE deleted_at IS NULL"),
    ] {
        f.relations.insert(name.into(), g.sql_i64(sql));
    }
    // Reads through the operations the screens use.
    f.reads.insert("board".into(), {
        let b = g.ok("story.board", json!({ "episodeId": null }));
        json!({ "cards": b["cardCount"], "beats": b["beatCount"], "sequences": b["sequenceCount"], "acts": arr(&b["acts"]).len() })
    });
    f.reads.insert("callSheets".into(), json!(arr(&g.ok("callsheets.list", json!({}))).len()));
    f.reads.insert("cast".into(), json!(arr(&g.ok("cast.list", json!({ "includeArchived": true }))["members"]).len()));
    f.reads.insert("crew".into(), json!(arr(&g.ok("crew.list", json!({ "includeArchived": true }))).len()));
    f.reads.insert("locations".into(), json!(arr(&g.ok("locations.list", json!({ "status": null, "search": null, "includeArchived": true }))).len()));
    f.reads.insert("catalog".into(), json!(arr(&g.ok("catalog.list", json!({ "category": null, "search": null, "includeArchived": true }))).len()));
    f.reads.insert("moodboards".into(), json!(arr(&g.ok("moodboard.list", json!({}))).len()));
    f.reads.insert("storyboards".into(), json!(arr(&g.ok("storyboard.list", json!({}))).len()));
    f.reads.insert("shots".into(), json!(arr(&g.ok("shot.list", json!({ "includeRemoved": true }))).len()));
    f.reads.insert("notes".into(), json!(arr(&g.ok("notes.list", json!({ "text": null }))).len()));
    f.reads.insert("tasks".into(), json!(arr(&g.ok("tasks.list", json!({ "status": null, "ownerUserId": null }))).len()));
    f.reads.insert("vault".into(), json!(arr(&g.ok("vault.list", json!({ "store": "project", "view": null, "search": null }))).len()));
    f.reads.insert("files".into(), json!(arr(&g.ok("files.list", json!({}))["files"]).len()));
    f.reads.insert("reports".into(), json!(arr(&g.ok("reports.list", json!({}))).len()));
    f.reads.insert("sides".into(), json!(arr(&g.ok("sides.list", json!({}))).len()));
    f.reads.insert("productionSource".into(), g.ok("production.source", json!({}))["draft"]["name"].clone());
    f
}

/// Tables whose rows are history of this installation rather than project content.
const VOLATILE: &[&str] = &["sys_", "undo", "redo", "activity", "history_step", "search_", "ai_", "intelligence", "package_log", "package_session"];

fn compare(name: &str, how: &str, a: &Fingerprint, b: &Fingerprint, same_identity: bool) -> Copy {
    let mut c = Copy { name: name.into(), how: how.into(), opened: true, ..Default::default() };
    if same_identity && a.project_id != b.project_id {
        c.differences.push(format!("project id {} became {}", a.project_id, b.project_id));
    }
    for (t, n) in &a.tables {
        let m = b.tables.get(t).copied().unwrap_or(-1);
        c.tables_compared += 1;
        c.rows_compared += *n;
        if *n != m {
            let line = format!("table {t}: {n} rows in the original, {m} in the copy");
            if t == "project_member" && m == n + 1 {
                c.expected.push(format!("{line}: the person who opens the copy on another installation is added as its Owner (project.rs, \"Ensure the local user is a member\")"));
            } else if VOLATILE.iter().any(|v| t.contains(v)) {
                c.expected.push(format!("{line} (installation history or derived search index, not project content)"));
            } else {
                c.differences.push(line)
            }
        }
    }
    for t in b.tables.keys().filter(|t| !a.tables.contains_key(*t)) {
        c.differences.push(format!("table {t} exists only in the copy"));
    }
    for (d, x) in &a.drafts {
        match b.drafts.get(d) {
            Some(y) if y == x => {}
            Some(y) => c.differences.push(format!("draft {d}: {} scenes, {} elements, text {} became {} scenes, {} elements, text {}", x.0, x.1, &x.2[..12], y.0, y.1, &y.2[..12])),
            None => c.differences.push(format!("draft {d} is missing")),
        }
    }
    if a.budget != b.budget {
        c.differences.push(format!("budget {} became {}", a.budget, b.budget));
    }
    if a.schedule != b.schedule {
        c.differences.push(format!("schedule differs: {} days became {}", a.schedule.len(), b.schedule.len()));
    }
    for (k, h) in &a.assets {
        c.assets_compared += 1;
        match b.assets.get(k) {
            Some(x) if x == h => {}
            Some(_) => c.differences.push(format!("file {k} has different content")),
            None => c.differences.push(format!("file {k} is missing")),
        }
    }
    for k in b.assets.keys().filter(|k| !a.assets.contains_key(*k)) {
        c.expected.push(format!("file {k} exists only in the copy: a linked file outside the original project folder, copied into the portable package"));
    }
    for (k, v) in &a.relations {
        if b.relations.get(k) != Some(v) {
            c.differences.push(format!("{k}: {v} became {:?}", b.relations.get(k)));
        }
    }
    for (k, v) in &a.reads {
        if b.reads.get(k) != Some(v) {
            c.differences.push(format!("read {k}: {v} became {}", b.reads.get(k).cloned().unwrap_or(Value::Null)));
        }
    }
    c
}

fn import_into(root: &Path, file: &str, mode: &str) -> (Gold, Result<Value, String>) {
    let g = Gold::open(root, None);
    g.phase(&format!("Verify: import {mode}"));
    g.ok("packages.inspect_package", json!({ "path": file }));
    let started = g.ok("packages.import_project", json!({ "path": file, "mode": mode, "parentDir": null }));
    let r = g.wait_task(&started).map_err(|e| format!("{}: {}", e.code.0, e.message));
    (g, r)
}

pub fn run(g: &Gold, ctx: &Ctx, work: &Path) -> Report {
    let mut report = Report::default();
    g.phase("Verify: fingerprint of the original");
    let source = fingerprint(g);
    report.source = source.clone();
    for p in [ctx.id("projectPackage"), ctx.id("finalBackupPath"), ctx.id("backupPath")] {
        if let Ok(m) = std::fs::metadata(&p) {
            let bytes = std::fs::read(&p).unwrap_or_default();
            report.package_files.push((p.clone(), m.len(), sha(&bytes)));
        }
    }

    // 1. The full project package on a new computer.
    let (a, r) = import_into(&work.join("verify-package"), &ctx.id("projectPackage"), "open");
    let mut c = match r {
        Ok(_) => compare("Full project package (.ofproject), imported on a fresh data root", "packages.import_project mode open", &source, &fingerprint(&a), true),
        Err(e) => Copy { name: "Full project package".into(), error: e, ..Default::default() },
    };
    if c.opened {
        // Read back after a restart of that installation.
        let path = a.core.project().map(|p| p.layout.root().to_string_lossy().into_owned()).unwrap_or_default();
        a.shutdown();
        drop(a);
        let a2 = Gold::open(&work.join("verify-package"), None);
        a2.ok("project.open", json!({ "path": path }));
        let again = compare("", "", &source, &fingerprint(&a2), true);
        for d in again.differences {
            c.differences.push(format!("after restart: {d}"));
        }
        merge(g, &a2);
        a2.shutdown();
    }
    report.copies.push(c);

    // 2. The final backup restored on a fresh data root, then a simulated crash.
    let (b, r) = import_into(&work.join("verify-backup"), &ctx.id("finalBackupPath"), "open");
    let c = match r {
        Ok(_) => compare("Final backup (.ofbackup), restored on a fresh data root", "packages.import_project mode open", &source, &fingerprint(&b), true),
        Err(e) => Copy { name: "Final backup".into(), error: e, ..Default::default() },
    };
    report.copies.push(c);
    if b.core.project().is_ok() {
        // Crash: the process ends without closing the project.
        let path = b.core.project().map(|p| p.layout.root().to_string_lossy().into_owned()).unwrap_or_default();
        b.ok("notes.create", json!({ "title": "Written just before the crash", "body": "Autosaved line.", "pinned": false }));
        merge(g, &b);
        let session = b.core.set_project(None);
        drop(session);
        let Gold { core, .. } = b;
        drop(core);
        let b2 = Gold::open(&work.join("verify-backup"), None);
        b2.phase("Verify: crash recovery");
        b2.ok("project.open", json!({ "path": path }));
        let state = b2.ok("project.recovery_state", json!({}));
        b2.ok("project.resolve_recovery", json!({ "choice": "latest" }));
        let fp = fingerprint(&b2);
        let mut c = compare("Restored backup after a simulated crash (latest autosave kept)", "project.recovery_state, project.resolve_recovery latest", &source, &fp, true);
        let notes_before = source.reads.get("notes").and_then(Value::as_i64).unwrap_or(0);
        let notes_after = fp.reads.get("notes").and_then(Value::as_i64).unwrap_or(0);
        // The note written before the crash must survive: exactly one more note.
        c.differences.retain(|d| !d.starts_with("read notes") && !d.starts_with("table project_note"));
        if notes_after != notes_before + 1 {
            c.differences.push(format!("the note written before the crash was lost: {notes_before} notes before, {notes_after} after"));
        }
        c.expected.push(format!("recovery offer: {}", state));
        c.expected.push("one extra note (written before the crash, on purpose)".into());
        report.copies.push(c);
        merge(g, &b2);
        b2.shutdown();
    } else {
        merge(g, &b);
    }

    // 3. The pre-shoot backup opened as a copy next to the package import.
    let (d, r) = import_into(&work.join("verify-package"), &ctx.id("backupPath"), "copy");
    let mut c = match r {
        Ok(_) => {
            let fp = fingerprint(&d);
            let mut c = Copy { name: "Pre-shoot backup opened as a copy beside the imported project".into(), how: "packages.import_project mode copy".into(), opened: true, ..Default::default() };
            if fp.project_id == source.project_id {
                c.differences.push("the copy kept the original project id".into());
            }
            if fp.drafts.is_empty() {
                c.differences.push("the copy has no screenplay drafts".into());
            }
            c.expected.push(format!("title of the copy: {}", fp.title));
            c.expected.push("an earlier state (before the AI phase and the final packages), so counts are not compared".into());
            c
        }
        Err(e) => Copy { name: "Pre-shoot backup as a copy".into(), error: e, ..Default::default() },
    };
    c.tables_compared = 0;
    report.copies.push(c);
    merge(g, &d);
    d.shutdown();

    report
}

fn merge(into: &Gold, from: &Gold) {
    crate::phases::packages::merge_counts(into, from);
    from.counts.borrow_mut().clear();
    from.failures.borrow_mut().clear();
}

pub fn reopen_original(work: &Path, ctx: &Ctx, source: &Fingerprint) -> Copy {
    let g = Gold::open(&work.join("studio"), None);
    g.phase("Verify: reopen the original in a fresh core");
    g.ok("project.open", json!({ "path": ctx.id("projectPath") }));
    let c = compare("Original reopened in a fresh core after a clean quit", "AppCore::new on the same data root, project.open", source, &fingerprint(&g), true);
    g.shutdown();
    c
}

pub fn write(report: &Report, ctx: &Ctx, base: &Path) {
    let mut md = String::new();
    md.push_str("# Low Tide Radio: verification of the exported project\n\n");
    md.push_str("Generated by `cargo run -p openframe-gold-project --release`. Every number below comes from that run.\n\n");
    md.push_str(&format!("Result: **{}**.\n\n", if report.ok() { "every copy matches the original" } else { "differences found (listed below)" }));
    md.push_str("## What was compared\n\n");
    md.push_str("- Row counts of every table in the project database (soft-deleted rows included).\n");
    md.push_str("- Every screenplay draft: scene count, element count and a SHA-256 over scene labels, headings, element types, text, dual flags and revision marks.\n");
    md.push_str("- Budget totals and line count; the schedule day by day (dates, scene order, breaks).\n");
    md.push_str("- Every file in the project folder except the database, its journal, the lock and the session marker, by SHA-256.\n");
    md.push_str("- Relationship counts (card and character links, catalog links, panel and shot links, comments, private notes) and the counts the screens read.\n\n");
    md.push_str("## Original\n\n");
    let s0 = &report.source;
    md.push_str(&format!("- Project id `{}`, title {}\n", s0.project_id, s0.title));
    md.push_str(&format!("- {} tables, {} rows; {} project files hashed\n", s0.tables.len(), s0.tables.values().sum::<i64>(), s0.assets.len()));
    for (d, (sc, el, h)) in &s0.drafts {
        md.push_str(&format!("- Draft {d}: {sc} scenes, {el} elements, text `{}`\n", &h[..16]));
    }
    md.push_str(&format!("- Budget: {}\n", s0.budget));
    md.push_str(&format!("- Schedule: {} days\n", s0.schedule.len()));
    for (k, v) in &s0.reads {
        md.push_str(&format!("- {k}: {v}\n"));
    }
    for (k, v) in &s0.relations {
        md.push_str(&format!("- {k}: {v}\n"));
    }
    md.push_str("\n## Package files\n\n| File | Bytes | SHA-256 |\n|---|---|---|\n");
    for (p, n, h) in &report.package_files {
        let rel = Path::new(p).strip_prefix(base).map(|r| r.to_string_lossy().into_owned()).unwrap_or(p.clone());
        md.push_str(&format!("| {rel} | {n} | `{}` |\n", &h[..16]));
    }
    md.push_str("\n## Copies\n\n");
    for c in &report.copies {
        md.push_str(&format!("### {}\n\n", c.name));
        md.push_str(&format!("- How: {}\n", c.how));
        if !c.error.is_empty() {
            md.push_str(&format!("- **Could not open:** {}\n\n", c.error));
            continue;
        }
        if c.tables_compared > 0 {
            md.push_str(&format!("- Compared {} tables ({} rows) and {} files.\n", c.tables_compared, c.rows_compared, c.assets_compared));
        }
        if c.differences.is_empty() {
            md.push_str("- Differences: none.\n");
        } else {
            md.push_str(&format!("- **Differences ({}):**\n", c.differences.len()));
            for d in &c.differences {
                md.push_str(&format!("  - {d}\n"));
            }
        }
        if !c.expected.is_empty() {
            md.push_str("- Expected and explained:\n");
            for d in &c.expected {
                md.push_str(&format!("  - {d}\n"));
            }
        }
        md.push('\n');
    }
    md.push_str("## Facts recorded during the build\n\n```json\n");
    let facts: serde_json::Map<String, Value> = ctx.facts.iter().filter(|(k, _)| k.as_str() != "exports").map(|(k, v)| (k.clone(), v.clone())).collect();
    md.push_str(&serde_json::to_string_pretty(&Value::Object(facts)).unwrap_or_default());
    md.push_str("\n```\n");
    std::fs::write(base.join("VERIFY.md"), md).expect("VERIFY.md");
}
