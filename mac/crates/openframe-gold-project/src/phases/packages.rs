//! Packages: the backup and the full project package (the gold reference files), and the
//! exchange packages for collaborators, with a reviewer on a second installation who comments
//! and sends a response package back.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn exchange(g: &Gold, ctx: &mut Ctx, ty: &str, file: &str, extra: Value) -> String {
    let path = ctx.outp(&format!("packages/{file}"));
    let mut args = json!({ "packageType": ty, "draftId": null, "scope": "full", "sceneIds": [], "actId": null, "callSheetId": null,
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

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Packages: backup and full project package");
    g.ok("packages.backup_preview", json!({}));
    let backup = ctx.outp("packages/Low Tide Radio.ofbackup");
    let r = g.task("packages.create_backup", json!({ "path": backup, "label": "before the shoot", "includeExternal": true }));
    let backup = r["path"].as_str().map(str::to_string).unwrap_or(backup);
    ctx.set("backupPath", backup.clone());
    g.ok("packages.inspect_package", json!({ "path": backup }));
    // A second backup started by a double click is cancelled at once.
    let dup = ctx.outp("packages/cancelled.ofbackup");
    let started = g.ok("packages.create_backup", json!({ "path": dup, "label": "double click", "includeExternal": false }));
    if let Some(t) = started["taskId"].as_str() {
        g.ok("app.cancel_task", json!({ "taskId": t }));
        let end = g.wait_task(&started);
        let exists = std::path::Path::new(&dup).exists();
        ctx.facts.insert("cancelledBackup".into(), json!({ "result": end.as_ref().map(|_| "completed before the cancel arrived").unwrap_or("cancelled"), "fileLeftBehind": exists }));
        if exists {
            std::fs::remove_file(&dup).ok();
        }
    }

    g.phase("Packages: exchange with collaborators");
    g.ok("packages.exchange_sources", json!({}));
    g.ok("packages.exchange_scenes", json!({ "draftId": ctx.id("draftBlue") }));
    // Private notes never travel in an exchange package.
    g.refused(
        "packages.exchange_preview",
        json!({ "packageType": "scriptReview", "draftId": null, "scope": "full", "sceneIds": [], "actId": null, "callSheetId": null,
                "includeComments": true, "includeAttachments": false, "includePrivateNotes": true, "path": null }),
    );
    exchange(g, ctx, "scriptReview", "Low Tide Radio - script review (blue pages).ofscriptreview", json!({ "draftId": ctx.id("draftBlue"), "scope": "comments" }));
    exchange(g, ctx, "story", "Low Tide Radio - story.ofstory", json!({}));
    exchange(g, ctx, "breakdown", "Low Tide Radio - breakdown.ofbreakdown", json!({}));
    let s24 = ctx.scenes.get(&24).cloned().unwrap_or_default();
    exchange(g, ctx, "shots", "Low Tide Radio - shots scene 24.ofshots", json!({ "scope": "scenes", "sceneIds": [s24] }));
    let schedule = exchange(g, ctx, "schedule", "Low Tide Radio - schedule.ofschedule", json!({}));
    let cs = ctx.call_sheets.get(&9).cloned().unwrap_or_default();
    exchange(g, ctx, "callReview", "Low Tide Radio - call sheet day 9.ofcallreview", json!({ "callSheetId": cs }));

    let response = ctx.id("producerResponse");
    g.phase("Packages: the response comes back");
    let session = g.ok("packages.open_exchange", json!({ "path": response }));
    let sid = s(&session["id"]);
    g.ok("packages.session", json!({ "sessionId": sid }));
    g.ok("packages.session_view", json!({ "sessionId": sid }));
    g.ok("packages.sessions", json!({ "limit": null }));
    let applied = g.ok("packages.apply_import", json!({ "sessionId": sid, "mode": "allSafe", "selected": [], "createBackup": true }));
    ctx.facts.insert("responseImport".into(), json!({ "status": applied["status"], "message": applied["result"]["message"] }));
    // Imported into the wrong scene order by mistake: undone and applied again.
    g.ok("packages.undo_import", json!({ "sessionId": sid }));
    let again = g.ok("packages.open_exchange", json!({ "path": response }));
    g.ok("packages.apply_import", json!({ "sessionId": again["id"], "mode": "allSafe", "selected": [], "createBackup": false }));
    // The notes on the emailed Fountain file: matched by their quoted text; the one that fits
    // several scenes waits in the Review Queue and is attached by hand.
    let s2 = g.ok("packages.open_exchange", json!({ "path": ctx.id("producerResponse2") }));
    let a2 = g.ok("packages.apply_import", json!({ "sessionId": s2["id"], "mode": "allSafe", "selected": [], "createBackup": false }));
    ctx.facts.insert("responseImportFountainDelivery".into(), json!({ "status": a2["status"], "message": a2["result"]["message"] }));
    let queue = g.ok("packages.review_queue", json!({ "limit": null }));
    ctx.facts.insert("reviewQueue".into(), json!(arr(&queue).iter().map(|i| format!("{} {}", s(&i["kind"]), s(&i["body"]))).collect::<Vec<_>>()));
    for item in arr(&queue) {
        if s(&item["status"]) != "Attached" {
            // The producer meant scene 24 (the dog) and scene 28 (the logbook line).
            let want = if s(&item["body"]).contains("dog") { "Scene 24" } else { "Scene 28" };
            let cands = arr(&item["candidates"]);
            let target = cands
                .iter()
                .find(|c| s(&c["label"]).starts_with(want))
                .or(cands.first())
                .map(|c| s(&c["sceneId"]))
                .unwrap_or_else(|| s24.clone());
            g.ok("packages.queue_attach", json!({ "itemId": item["id"], "sceneId": target }));
        }
    }
    // The same response kept as a separate review record, viewed and then removed.
    let rec = g.ok("packages.open_exchange", json!({ "path": response }));
    g.ok("packages.apply_import", json!({ "sessionId": rec["id"], "mode": "record", "selected": [], "createBackup": false }));
    let records = g.ok("packages.records", json!({ "limit": null }));
    if let Some(r) = arr(&records).first() {
        g.ok("packages.record_view", json!({ "recordId": r["id"] }));
        g.ok("packages.record_delete", json!({ "recordId": r["id"] }));
    }
    // The schedule package opened by mistake is cancelled without importing.
    let wrong = g.ok("packages.open_exchange", json!({ "path": schedule }));
    if !wrong.is_null() {
        g.ok("packages.cancel_import", json!({ "sessionId": wrong["id"] }));
    }
    g.ok("packages.log", json!({ "limit": 100 }));

    // The full project package last, so it carries everything above.
    g.phase("Packages: final full project package");
    g.ok("project.save", json!({}));
    let pkg = ctx.outp("packages/Low Tide Radio.ofproject");
    let r = g.task("packages.export_project", json!({ "path": pkg, "label": null, "includeExternal": true }));
    let pkg = r["path"].as_str().map(str::to_string).unwrap_or(pkg);
    ctx.set("projectPackage", pkg.clone());
    g.ok("packages.inspect_package", json!({ "path": pkg }));
    // A final backup of the finished state as well.
    let final_backup = ctx.outp("packages/Low Tide Radio (final).ofbackup");
    let r = g.task("packages.create_backup", json!({ "path": final_backup, "label": "final", "includeExternal": true }));
    ctx.set("finalBackupPath", r["path"].as_str().map(str::to_string).unwrap_or(final_backup));
}

/// The producer reads the shooting draft on his own laptop (a second installation), comments,
/// and sends a response package. It arrives after the blue pages, which omit scene 22.
pub fn producer_review(g: &Gold, ctx: &mut Ctx) {
    g.phase("Packages: the producer's review on another computer");
    let review = exchange(g, ctx, "scriptReview", "Low Tide Radio - script review (shooting draft).ofscriptreview", json!({ "draftId": ctx.id("draftShooting"), "scope": "full" }));
    let reviewer = Gold::open(&g.root.parent().unwrap().join("producer-laptop"), None);
    reviewer.phase("Producer's review");
    reviewer.ok("app.set_display_name", json!({ "displayName": "Arun Joseph" }));
    let ws = reviewer.ok("packages.review_open", json!({ "path": review }));
    let pid = s(&ws["key"]);
    let scenes = arr(&ws["scenes"]);
    let scene_id = |n: usize| scenes.get(n - 1).map(|sc| s(&sc["id"]) + &s(&sc["sceneId"])).unwrap_or_default();
    reviewer.ok("packages.review_comment", json!({ "packageId": pid, "sceneId": scene_id(24), "body": "Is Kannan in every shot here? The wrangler needs to know.", "quotedText": null }));
    reviewer.ok("packages.review_comment", json!({ "packageId": pid, "sceneId": scene_id(39), "body": "This is the film. Give the names all the time they need.", "quotedText": null }));
    reviewer.ok("packages.review_comment", json!({ "packageId": pid, "sceneId": scene_id(22), "body": "Lovely shot, but day 4 is too long. Can the empty room live at the top of 23?", "quotedText": "THE COIR STRING OF KEYS IS GONE" }));
    let oops = reviewer.ok("packages.review_comment", json!({ "packageId": pid, "sceneId": scene_id(2), "body": "typo", "quotedText": null }));
    let oops_id = arr(&oops["comments"]).iter().find(|c| s(&c["body"]) == "typo").map(|c| s(&c["id"]));
    if let Some(id) = oops_id {
        reviewer.ok("packages.review_delete_comment", json!({ "packageId": pid, "commentId": id }));
    }
    // He also read the Fountain file the writer emailed him; that draft's scenes are matched to
    // the project by their text when the notes come back.
    let emailed = exchange(g, ctx, "scriptReview", "Low Tide Radio - script review (writer's Fountain delivery).ofscriptreview", json!({ "draftId": ctx.id("draftImported"), "scope": "full" }));
    let ws2 = reviewer.ok("packages.review_open", json!({ "path": emailed }));
    let pid2 = s(&ws2["key"]);
    let scenes2 = arr(&ws2["scenes"]);
    let scene2 = |n: usize| scenes2.get(n - 1).map(|sc| s(&sc["id"]) + &s(&sc["sceneId"])).unwrap_or_default();
    reviewer.ok("packages.review_comment", json!({ "packageId": pid2, "sceneId": scene2(28), "body": "Read this line slower. It is the whole case.", "quotedText": "MAIN POWER CUT BY ORDER" }));
    reviewer.ok("packages.review_comment", json!({ "packageId": pid2, "sceneId": scene2(24), "body": "Make sure we always hear the dog when we cannot see him.", "quotedText": "Kannan" }));
    let response2 = ctx.outp("packages/Low Tide Radio - producer response (Fountain delivery).ofresponse");
    let r2 = reviewer.ok("packages.review_export_response", json!({ "packageId": pid2, "path": response2 }));
    ctx.set("producerResponse2", r2["path"].as_str().map(str::to_string).unwrap_or(response2));
    reviewer.ok("packages.review_get", json!({ "packageId": pid }));
    reviewer.ok("packages.review_list", json!({ "limit": null }));
    let response = ctx.outp("packages/Low Tide Radio - producer response.ofresponse");
    let r = reviewer.ok("packages.review_export_response", json!({ "packageId": pid, "path": response }));
    ctx.set("producerResponse", r["path"].as_str().map(str::to_string).unwrap_or(response));
    reviewer.shutdown();
    merge_counts(g, &reviewer);
}

pub fn merge_counts(into: &Gold, from: &Gold) {
    let mut c = into.counts.borrow_mut();
    for (op, n) in from.counts.borrow().iter() {
        let e = c.entry(op.clone()).or_default();
        e.ok += n.ok;
        e.expected_err += n.expected_err;
        e.unexpected_err += n.unexpected_err;
    }
    into.failures.borrow_mut().extend(from.failures.borrow().iter().cloned());
}
