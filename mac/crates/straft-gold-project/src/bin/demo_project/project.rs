//! Application settings, the project itself, the safety copy and the final save.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, s};

pub fn settings(g: &Gold, ctx: &mut Ctx) {
    g.phase("Application settings");
    let p = ctx.project.clone();
    g.ok("app.info", json!({}));
    g.ok(
        "app.set_display_name",
        json!({ "displayName": p["displayName"] }),
    );
    g.ok("entitlement.status", json!({}));
    let today = p["titlePage"]["date"].clone();
    g.ok("entitlement.notice", json!({ "today": today }));
    // The first-run screens follow a purchase; an internal build has none, so each is refused.
    for notice in ["dataNote", "tutorial", "welcome", "reminder"] {
        g.refused(
            "entitlement.record_notice",
            json!({ "notice": notice, "today": today }),
        );
    }
    for op in [
        "entitlement.start_trial",
        "entitlement.buy",
        "entitlement.restore_purchase",
    ] {
        g.probe(op, json!({}));
    }

    let t = &p["theme"];
    let theme = ctx.media_path(&s(&t["file"]));
    let check = g.ok("appearance.check_theme", json!({ "path": theme }));
    g.ok(
        "appearance.import_theme",
        json!({ "path": theme, "apply": true, "expectedDigest": check["digest"] }),
    );
    g.ok(
        "appearance.rename_theme",
        json!({ "id": t["id"], "name": t["name"] }),
    );
    g.ok(
        "appearance.export_theme",
        json!({ "id": t["id"], "path": ctx.outp(&format!("exports/settings/{}.json", s(&t["id"]))) }),
    );
    // A variant tried once and removed again.
    let swap = t["dayText"].as_array().cloned().unwrap_or_default();
    let day = std::fs::read_to_string(&theme)
        .unwrap_or_default()
        .replace(&s(&swap[0]), &s(&swap[1]))
        .replace(&s(&swap[2]), &s(&swap[3]));
    g.ok("appearance.check_theme", json!({ "text": day }));
    g.ok(
        "appearance.import_theme",
        json!({ "text": day, "apply": false }),
    );
    g.ok("appearance.delete_theme", json!({ "id": swap[1] }));
    g.ok("appearance.set_mode", json!({ "mode": t["mode"] }));
    g.ok("appearance.set_theme", json!({ "themeId": t["id"] }));
    g.ok("appearance.get", json!({}));

    g.ok("app.shortcuts", json!({}));
    g.ok(
        "app.set_shortcuts",
        json!({ "changes": [p["shortcuts"][0]] }),
    );
    g.ok("app.reset_shortcuts", json!({}));
    g.ok("app.set_shortcuts", json!({ "changes": p["shortcuts"] }));
    g.ok("diagnostics.preview", json!({}));
    g.ok("ai.status", json!({}));
    g.ok("ai.download.status", json!({}));
}

pub fn create(g: &Gold, ctx: &mut Ctx) {
    g.phase("Project");
    let p = ctx.project.clone();
    let created = g.ok(
        "project.create",
        json!({ "title": p["title"], "projectType": p["projectType"], "language": p["language"],
                "genre": p["genre"], "creator": p["creator"], "parentDir": ctx.out.to_string_lossy() }),
    );
    let mut project_id = s(&created["project"]["id"]);
    if project_id.is_empty() {
        project_id = s(&created["summary"]["id"]);
    }
    ctx.set("project", project_id.clone());
    g.ok("project.current", json!({}));
    g.ok(
        "project.rename",
        json!({ "projectId": project_id, "title": p["workingTitle"] }),
    );
    g.ok(
        "project.rename",
        json!({ "projectId": project_id, "title": p["title"] }),
    );
    g.ok(
        "project.update_settings",
        json!({ "title": null, "language": p["language"], "genre": p["genre"], "creator": p["creator"],
                "logline": p["logline"], "projectType": null, "settings": p["settings"] }),
    );
    g.ok(
        "project.set_status",
        json!({ "status": p["statusWriting"] }),
    );
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
        .map(|sess| sess.layout.root().to_string_lossy().into_owned())
        .unwrap_or_default();
    ctx.set("projectPath", here);
    g.ok(
        "project.location_check",
        json!({ "path": ctx.out.to_string_lossy() }),
    );
}

/// The safety copy made before the big revision: duplicated, archived, moved to the scratch folder,
/// located again and taken off the recent list.
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
    let moved = g
        .root
        .join("archive-drive")
        .join(s(&ctx.project["safetyCopyName"]));
    std::fs::create_dir_all(moved.parent().unwrap()).ok();
    if std::fs::rename(&dup_path, &moved).is_ok() {
        g.ok(
            "project.locate",
            json!({ "projectId": dup_id, "path": moved.to_string_lossy() }),
        );
        g.ok("project.close", json!({}));
        g.ok("project.open", json!({ "path": ctx.id("projectPath") }));
    }
    g.ok("project.remove_recent", json!({ "projectId": dup_id }));
}

pub fn finish(g: &Gold, ctx: &mut Ctx) {
    g.phase("Wrap");
    g.ok(
        "project.set_status",
        json!({ "status": ctx.project["statusEnd"] }),
    );
    g.ok(
        "project.set_last_location",
        json!({ "location": { "workspace": "home" } }),
    );
    g.ok("project.save", json!({}));
    g.ok("app.save_state", json!({}));
    let home: Value = g.ok("project.home", json!({}));
    ctx.facts.insert("home".into(), home);
}
