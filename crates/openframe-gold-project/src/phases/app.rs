//! Application settings (profile, appearance, shortcuts, proofing, licence state) and the project
//! itself (create, settings, status, recent list, duplicate, archive, locate).

use serde_json::json;

use crate::Ctx;
use crate::gold::{Gold, s};

pub fn settings(g: &Gold, ctx: &mut Ctx) {
    g.phase("Application settings");
    g.ok("app.info", json!({}));
    g.ok(
        "app.set_display_name",
        json!({ "displayName": "Meenakshi Varma" }),
    );
    g.ok("entitlement.status", json!({}));
    g.ok("entitlement.notice", json!({ "today": "2026-11-02" }));
    // The first-run screens follow a purchase; an internal build has none, so each is refused.
    for notice in ["dataNote", "tutorial", "welcome", "reminder"] {
        g.refused(
            "entitlement.record_notice",
            json!({ "notice": notice, "today": "2026-11-02" }),
        );
    }
    let mut store = serde_json::Map::new();
    for op in [
        "entitlement.start_trial",
        "entitlement.buy",
        "entitlement.restore_purchase",
    ] {
        store.insert(op.into(), g.probe(op, json!({})));
    }
    ctx.facts.insert(
        "storeCallsInInternalBuild".into(),
        serde_json::Value::Object(store),
    );
    g.ok("entitlement.notice", json!({ "today": "2026-11-02" }));

    // Appearance: the art department's grading theme, imported from a file and applied.
    let theme = ctx.media.path("low-tide-radio-theme.json");
    let check = g.ok("appearance.check_theme", json!({ "path": theme }));
    g.ok(
        "appearance.import_theme",
        json!({ "path": theme, "apply": true, "expectedDigest": check["digest"] }),
    );
    g.ok(
        "appearance.rename_theme",
        json!({ "id": "low-tide-night", "name": "Low Tide Night (grade)" }),
    );
    g.ok(
        "appearance.export_theme",
        json!({ "id": "low-tide-night", "path": ctx.outp("settings/low-tide-night-theme.json") }),
    );
    // A day variant was tried in the grade test and removed again.
    let day = std::fs::read_to_string(&theme)
        .unwrap_or_default()
        .replace("low-tide-night", "low-tide-day")
        .replace("Low Tide Night", "Low Tide Day")
        .replace("\"mode\"", "\"_mode\"");
    g.ok("appearance.check_theme", json!({ "text": day }));
    g.ok(
        "appearance.import_theme",
        json!({ "text": day, "apply": false }),
    );
    g.ok("appearance.delete_theme", json!({ "id": "low-tide-day" }));
    g.ok("appearance.set_mode", json!({ "mode": "dark" }));
    g.ok(
        "appearance.set_theme",
        json!({ "themeId": "low-tide-night" }),
    );
    g.ok("appearance.get", json!({}));

    // Keyboard shortcuts: the writer's own keys for shots and the find bar.
    g.ok("app.shortcuts", json!({}));
    g.ok(
        "app.set_shortcuts",
        json!({ "changes": [{ "commandId": "screenplay.element_shot", "keys": ["Mod+Shift+7"] }] }),
    );
    g.ok("app.reset_shortcuts", json!({}));
    g.ok(
        "app.set_shortcuts",
        json!({ "changes": [
            { "commandId": "screenplay.element_shot", "keys": ["Mod+Shift+7"] },
            { "commandId": "screenplay.find", "keys": ["shift+mod+f"] }
        ] }),
    );
    ctx.facts
        .insert("shortcuts".into(), g.ok("app.shortcuts", json!({})));

    g.ok("diagnostics.preview", json!({}));
    g.ok("ai.status", json!({}));
    g.ok("ai.download.status", json!({}));
}

pub fn project(g: &Gold, ctx: &mut Ctx) {
    g.phase("Project");
    let p = ctx.story["project"].clone();
    // Created under the working title, renamed once the title was settled.
    let created = g.ok(
        "project.create",
        json!({ "title": "Low Tide", "projectType": p["projectType"], "language": p["language"],
                "genre": p["genre"], "creator": p["creator"] }),
    );
    let project_id = s(&created["project"]["id"]);
    let project_id = if project_id.is_empty() {
        s(&created["summary"]["id"])
    } else {
        project_id
    };
    ctx.set("project", project_id.clone());
    g.ok("project.current", json!({}));
    g.ok(
        "project.rename",
        json!({ "projectId": project_id, "title": p["title"] }),
    );
    g.ok(
        "project.update_settings",
        json!({ "title": null, "language": p["language"], "genre": p["genre"], "creator": p["creator"],
                "logline": p["logline"], "projectType": null, "settings": p["settings"] }),
    );
    g.ok("project.set_status", json!({ "status": "Writing" }));
    g.ok("project.get_settings", json!({}));
    g.ok("project.home", json!({}));
    g.ok("project.members", json!({}));
    g.ok(
        "project.set_pinned",
        json!({ "projectId": project_id, "pinned": true }),
    );
    g.ok("project.list_recent", json!({ "includeArchived": false }));
    g.ok("project.save", json!({}));
    g.ok("project.recovery_state", json!({}));
    g.ok(
        "project.set_last_location",
        json!({ "location": { "workspace": "story" } }),
    );
    let here = g
        .core
        .project()
        .map(|s| s.layout.root().to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.set("projectPath", here.clone());
    g.ok(
        "project.location_check",
        json!({ "path": g.root.join("projects").to_string_lossy() }),
    );
}

/// The safety copy made before the big revision: duplicated, archived, moved and located again,
/// then taken off the recent list (its folder is kept on disk; see COVERAGE.md for project.delete).
pub fn safety_copy(g: &Gold, ctx: &mut Ctx) {
    g.phase("Project safety copy");
    let id = ctx.id("project");
    let dup = g.ok("project.duplicate", json!({ "projectId": id }));
    let dup_id = s(&dup["projectId"]);
    let dup_path = s(&dup["path"]);
    if dup_id.is_empty() {
        return;
    }
    g.ok(
        "project.set_archived",
        json!({ "projectId": dup_id, "archived": true }),
    );
    g.ok("project.list_recent", json!({ "includeArchived": true }));
    // The copy is moved to the archive drive folder and found again with Locate.
    let moved = g
        .root
        .join("archive-drive")
        .join("Low Tide Radio (safety copy before blue pages)");
    std::fs::create_dir_all(moved.parent().unwrap()).ok();
    if std::fs::rename(&dup_path, &moved).is_ok() {
        // Locate opens the project; the gold project is opened again right after.
        g.ok(
            "project.locate",
            json!({ "projectId": dup_id, "path": moved.to_string_lossy() }),
        );
        g.ok("project.close", json!({}));
        g.ok("project.open", json!({ "path": ctx.id("projectPath") }));
    }
    g.ok("project.remove_recent", json!({ "projectId": dup_id }));
    ctx.set("safetyCopyPath", moved.to_string_lossy().into_owned());
}

pub fn finish(g: &Gold, ctx: &mut Ctx) {
    g.phase("Wrap");
    g.ok("project.set_status", json!({ "status": "Shooting" }));
    g.ok(
        "project.set_last_location",
        json!({ "location": { "workspace": "callsheets" } }),
    );
    g.ok("project.save", json!({}));
    g.ok("app.save_state", json!({}));
    ctx.facts
        .insert("home".into(), g.ok("project.home", json!({})));
}
