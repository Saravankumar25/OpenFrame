//! Packages: the review exchanged with a second installation, the backup, and the full project
//! package last, so it carries everything.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn exchange(g: &Gold, ctx: &mut Ctx, ty: &str, file: &str, extra: Value) -> String {
    let path = ctx.outp(&format!("packages/{file}"));
    let mut args = json!({ "packageType": ty, "draftId": null, "scope": "full", "sceneIds": [], "actId": null,
                           "includeComments": true, "includeAttachments": true, "includePrivateNotes": false, "path": null });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        args[k] = v;
    }
    g.ok("packages.exchange_preview", args.clone());
    args["path"] = json!(path);
    let r = g.ok("packages.export_exchange", args);
    let written = s(&r["path"]);
    if written.is_empty() { path } else { written }
}

/// A reader on a second installation comments on the shooting draft and sends a response package.
pub fn producer_review(g: &Gold, ctx: &mut Ctx) {
    g.phase("Packages: a review on another computer");
    let t = ctx.title.clone();
    let pr = ctx.script["producer"].clone();
    let review = exchange(
        g,
        ctx,
        "scriptReview",
        &format!("{t} - script review.ofscriptreview"),
        json!({ "draftId": ctx.id("draftShooting"), "scope": "full" }),
    );
    let reviewer = Gold::open(
        &g.root.parent().unwrap_or(&g.root).join("reviewer-laptop"),
        None,
    );
    reviewer.phase("Reviewer");
    reviewer.ok("app.set_display_name", json!({ "displayName": pr["name"] }));
    let ws = reviewer.ok("packages.review_open", json!({ "path": review }));
    let pid = s(&ws["key"]);
    let scenes = arr(&ws["scenes"]);
    let scene_id = |list: &[Value], n: &Value| {
        list.get((n.as_u64().unwrap_or(1) as usize).saturating_sub(1))
            .map(|x| s(&x["id"]) + &s(&x["sceneId"]))
            .unwrap_or_default()
    };
    for c in arr(&pr["comments"]) {
        reviewer.ok(
            "packages.review_comment",
            json!({ "packageId": pid, "sceneId": scene_id(&scenes, &c["scene"]), "body": c["body"], "quotedText": c["quotedText"] }),
        );
    }
    let typo = &pr["typo"];
    let oops = reviewer.ok(
        "packages.review_comment",
        json!({ "packageId": pid, "sceneId": scene_id(&scenes, &typo["scene"]), "body": typo["body"], "quotedText": null }),
    );
    if let Some(id) = arr(&oops["comments"])
        .iter()
        .find(|c| s(&c["body"]) == s(&typo["body"]))
        .map(|c| s(&c["id"]))
    {
        reviewer.ok(
            "packages.review_delete_comment",
            json!({ "packageId": pid, "commentId": id }),
        );
    }
    // A second copy, made from the imported draft, comes back matched by its text.
    let emailed = exchange(
        g,
        ctx,
        "scriptReview",
        &format!("{t} - script review (imported draft).ofscriptreview"),
        json!({ "draftId": ctx.id("draftImported"), "scope": "full" }),
    );
    let ws2 = reviewer.ok("packages.review_open", json!({ "path": emailed }));
    let pid2 = s(&ws2["key"]);
    let scenes2 = arr(&ws2["scenes"]);
    for c in arr(&pr["emailed"]) {
        reviewer.ok(
            "packages.review_comment",
            json!({ "packageId": pid2, "sceneId": scene_id(&scenes2, &c["scene"]), "body": c["body"], "quotedText": c["quotedText"] }),
        );
    }
    let response2 = ctx.outp(&format!(
        "packages/{t} - response (imported draft).ofresponse"
    ));
    let r2 = reviewer.ok(
        "packages.review_export_response",
        json!({ "packageId": pid2, "path": response2 }),
    );
    ctx.set(
        "response2",
        r2["path"].as_str().map(str::to_string).unwrap_or(response2),
    );
    reviewer.ok("packages.review_get", json!({ "packageId": pid }));
    reviewer.ok("packages.review_list", json!({ "limit": null }));
    let response = ctx.outp(&format!("packages/{t} - response.ofresponse"));
    let r = reviewer.ok(
        "packages.review_export_response",
        json!({ "packageId": pid, "path": response }),
    );
    ctx.set(
        "response",
        r["path"].as_str().map(str::to_string).unwrap_or(response),
    );
    reviewer.shutdown();
    let mut c = g.counts.borrow_mut();
    for (op, n) in reviewer.counts.borrow().iter() {
        let e = c.entry(op.clone()).or_default();
        e.ok += n.ok;
        e.expected_err += n.expected_err;
        e.unexpected_err += n.unexpected_err;
    }
    g.failures
        .borrow_mut()
        .extend(reviewer.failures.borrow().iter().cloned());
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Packages: backup");
    let t = ctx.title.clone();
    g.ok("packages.backup_preview", json!({}));
    let backup = ctx.outp(&format!("packages/{t}.ofbackup"));
    let r = g.task(
        "packages.create_backup",
        json!({ "path": backup, "label": "before the responses", "includeExternal": true }),
    );
    let backup = r["path"].as_str().map(str::to_string).unwrap_or(backup);
    ctx.set("backupEarly", backup.clone());
    g.ok("packages.inspect_package", json!({ "path": backup }));
    let dup = ctx.outp("packages/cancelled.ofbackup");
    let started = g.ok(
        "packages.create_backup",
        json!({ "path": dup, "label": "double click", "includeExternal": false }),
    );
    if let Some(task) = started["taskId"].as_str() {
        g.ok("app.cancel_task", json!({ "taskId": task }));
        let _ = g.wait_task(&started);
        std::fs::remove_file(&dup).ok();
    }

    g.phase("Packages: exchange");
    g.ok("packages.exchange_sources", json!({}));
    g.ok(
        "packages.exchange_scenes",
        json!({ "draftId": ctx.id("draftBlue") }),
    );
    g.refused(
        "packages.exchange_preview",
        json!({ "packageType": "scriptReview", "draftId": null, "scope": "full", "sceneIds": [], "actId": null,
                "includeComments": true, "includeAttachments": false, "includePrivateNotes": true, "path": null }),
    );
    exchange(
        g,
        ctx,
        "scriptReview",
        &format!("{t} - script review (revision pages).ofscriptreview"),
        json!({ "draftId": ctx.id("draftBlue"), "scope": "comments" }),
    );
    let story = exchange(g, ctx, "story", &format!("{t} - story.ofstory"), json!({}));

    g.phase("Packages: the responses come back");
    let response = ctx.id("response");
    let session = g.ok("packages.open_exchange", json!({ "path": response }));
    let sid = s(&session["id"]);
    g.ok("packages.session", json!({ "sessionId": sid }));
    g.ok("packages.session_view", json!({ "sessionId": sid }));
    g.ok("packages.sessions", json!({ "limit": null }));
    let applied = g.ok(
        "packages.apply_import",
        json!({ "sessionId": sid, "mode": "allSafe", "selected": [], "createBackup": true }),
    );
    ctx.facts.insert(
        "responseImport".into(),
        json!({ "status": applied["status"], "message": applied["result"]["message"] }),
    );
    g.ok("packages.undo_import", json!({ "sessionId": sid }));
    let again = g.ok("packages.open_exchange", json!({ "path": response }));
    g.ok(
        "packages.apply_import",
        json!({ "sessionId": again["id"], "mode": "allSafe", "selected": [], "createBackup": false }),
    );
    let s2 = g.ok(
        "packages.open_exchange",
        json!({ "path": ctx.id("response2") }),
    );
    let a2 = g.ok(
        "packages.apply_import",
        json!({ "sessionId": s2["id"], "mode": "allSafe", "selected": [], "createBackup": false }),
    );
    ctx.facts.insert(
        "responseImport2".into(),
        json!({ "status": a2["status"], "message": a2["result"]["message"] }),
    );
    let queue = g.ok("packages.review_queue", json!({ "limit": null }));
    for item in arr(&queue) {
        if s(&item["status"]) != "Attached" {
            let cands = arr(&item["candidates"]);
            let target = cands
                .first()
                .map(|c| s(&c["sceneId"]))
                .unwrap_or_else(|| ctx.scene(&ctx.script["producer"]["queueScene"]));
            g.ok(
                "packages.queue_attach",
                json!({ "itemId": item["id"], "sceneId": target }),
            );
        }
    }
    let rec = g.ok("packages.open_exchange", json!({ "path": response }));
    g.ok(
        "packages.apply_import",
        json!({ "sessionId": rec["id"], "mode": "record", "selected": [], "createBackup": false }),
    );
    let records = g.ok("packages.records", json!({ "limit": null }));
    if let Some(r) = arr(&records).first() {
        g.ok("packages.record_view", json!({ "recordId": r["id"] }));
        g.ok("packages.record_delete", json!({ "recordId": r["id"] }));
    }
    let wrong = g.ok("packages.open_exchange", json!({ "path": story }));
    if !wrong.is_null() {
        g.ok(
            "packages.cancel_import",
            json!({ "sessionId": wrong["id"] }),
        );
    }
    g.ok("packages.log", json!({ "limit": 100 }));

    g.phase("Packages: full project package");
    g.ok("project.save", json!({}));
    let pkg = ctx.outp(&format!("packages/{t}.ofproject"));
    let r = g.task(
        "packages.export_project",
        json!({ "path": pkg, "label": null, "includeExternal": true }),
    );
    ctx.set(
        "projectPackage",
        r["path"]
            .as_str()
            .map(str::to_string)
            .unwrap_or(pkg.clone()),
    );
    g.ok(
        "packages.inspect_package",
        json!({ "path": ctx.id("projectPackage") }),
    );
    let final_backup = ctx.outp(&format!("packages/{t} (final).ofbackup"));
    let r = g.task(
        "packages.create_backup",
        json!({ "path": final_backup, "label": "final", "includeExternal": true }),
    );
    ctx.set(
        "finalBackup",
        r["path"]
            .as_str()
            .map(str::to_string)
            .unwrap_or(final_backup),
    );
}
