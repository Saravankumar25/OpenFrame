//! Straft AI with the real models: installed through the app's own "use a model file I already
//! have" path, then real questions and proposals from `ai.json`, so a saved conversation exists.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s, truncate};

fn tasks(g: &Gold) -> usize {
    g.sql_i64("SELECT count(*) FROM task WHERE deleted_at IS NULL")
        .max(0) as usize
}

fn ask(g: &Gold, ctx: &mut Ctx, label: &str, text: &Value, sections: Value) -> Value {
    let started = std::time::Instant::now();
    // Every question joins one conversation, so the panel reopens with all of them.
    let conversation = ctx.ids.get("aiConversation").cloned();
    let x = g.ok(
        "ai.ask",
        json!({ "text": text, "sections": sections, "conversationId": conversation }),
    );
    if conversation.is_none() {
        let hist = g.ok("ai.history", json!({ "conversationId": null, "limit": 1 }));
        if let Some(id) = hist["conversationId"].as_str() {
            ctx.set("aiConversation", id);
        }
    }
    let r = &x["result"];
    let secs = started.elapsed().as_secs_f32();
    println!(
        "  AI {label}: {secs:.1}s kind={} {}",
        r["kind"],
        truncate(&s(&r["content"]), 160)
    );
    let mut log = ctx.facts.get("aiLog").cloned().unwrap_or(json!([]));
    if let Some(list) = log.as_array_mut() {
        list.push(json!({ "label": label, "question": text, "seconds": (secs * 10.0).round() / 10.0, "kind": r["kind"],
            "answer": r["content"], "sources": arr(&r["provenance"]).len(), "operations": r["changeSet"]["operationCount"] }));
    }
    ctx.facts.insert("aiLog".into(), log);
    x
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Straft AI");
    g.ok("ai.status", json!({}));
    g.ok("ai.download.status", json!({}));
    g.ok("ai.sections", json!({}));
    if !ctx.ai_enabled {
        ctx.facts.insert(
            "aiSkipped".into(),
            json!("the AI build inputs are absent or --no-ai was given"),
        );
        return;
    }
    let paused = g.probe("ai.download.pause", json!({}));
    ctx.facts.insert("aiPauseWithoutDownload".into(), paused);
    let models = ctx.repo.join(".ai-assets/models");
    let mut imported = 0;
    for m in [
        "gemma-3-1b-it-q8-0-v1",
        "bge-small-en-v1.5-q8-0-v1",
        "multilingual-e5-small-q8-0-v1",
    ] {
        let p = models.join(m).join("model.gguf");
        let r = g.ok("ai.download.import", json!({ "path": p.to_string_lossy() }));
        if !r.is_null() {
            imported += 1;
        }
    }
    ctx.facts.insert("aiModelsImported".into(), json!(imported));
    g.ok("ai.graphics.set", json!({ "off": false }));
    let status = g.ok("ai.status", json!({}));
    ctx.facts.insert("aiStatus".into(), status.clone());
    g.ok("ai.diagnostics", json!({}));
    g.ok("ai.health", json!({}));
    if status["available"] != json!(true) {
        return;
    }
    let started = g.ok("ai.index_rebuild", json!({}));
    if !started["taskId"].is_null() {
        let _ = g.wait_task(&started);
    }
    g.ok("ai.index_status", json!({}));

    let a = ctx.ai.clone();
    for q in arr(&a["asks"]) {
        let sections = if let Some(leaf) = q["leaf"].as_str() {
            let mut sec = json!({ "leaf": leaf });
            if q["draft"].as_str() == Some("blue") {
                sec["draftId"] = json!(ctx.id("draftBlue"));
            }
            if !q["scene"].is_null() {
                sec["sceneId"] = json!(ctx.scene(&q["scene"]));
            }
            json!([sec])
        } else {
            json!([])
        };
        ask(g, ctx, &s(&q["label"]), &q["text"], sections);
    }

    // A proposal applied and undone, one rejected, one applied and kept.
    let before = tasks(g);
    let x = ask(g, ctx, "propose-apply-undo", &a["apply"], json!([]));
    let cs = s(&x["result"]["changeSet"]["id"]);
    if !cs.is_empty() {
        g.ok("ai.change_set.get", json!({ "id": cs }));
        g.ok("ai.change_set.recheck", json!({ "id": cs }));
        let applied = g.ok("ai.change_set.accept", json!({ "id": cs }));
        let after_apply = tasks(g);
        g.ok("history.undo", json!({ "store": "project" }));
        ctx.facts.insert(
            "aiApplyUndo".into(),
            json!({ "state": applied["state"], "tasksBefore": before, "afterApply": after_apply, "afterUndo": tasks(g) }),
        );
    }
    let x = ask(g, ctx, "propose-reject", &a["reject"], json!([]));
    let cs = s(&x["result"]["changeSet"]["id"]);
    if !cs.is_empty() {
        let r = g.ok("ai.change_set.reject", json!({ "id": cs }));
        ctx.facts
            .insert("aiReject".into(), json!({ "state": r["state"] }));
    }
    let x = ask(g, ctx, "propose-apply-keep", &a["keep"], json!([]));
    let cs = s(&x["result"]["changeSet"]["id"]);
    if !cs.is_empty() {
        let before = tasks(g);
        let r = g.ok("ai.change_set.accept", json!({ "id": cs }));
        ctx.facts.insert(
            "aiApplyKeep".into(),
            json!({ "state": r["state"], "tasksBefore": before, "tasksAfter": tasks(g) }),
        );
    }
    let hist = g.ok("ai.history", json!({ "conversationId": null, "limit": 50 }));
    ctx.facts.insert(
        "aiHistoryTurns".into(),
        json!(arr(&hist["exchanges"]).len()),
    );
    if let Some(conv) = hist["conversationId"].as_str() {
        g.ok(
            "ai.export_conversation",
            json!({ "conversationId": conv, "path": ctx.outp("exports/ai/Straft AI conversation.json") }),
        );
    }
    g.ok(
        "ai.export_report",
        json!({ "path": ctx.outp("exports/ai/Straft AI report.json") }),
    );
    g.ok("ai.stop_runtime", json!({}));
    g.ok("ai.status", json!({}));
    // The models live in the scratch data root; they are removed to free the disk.
    g.ok("ai.download.remove", json!({}));
    ctx.facts
        .insert("aiStatusAfterRemove".into(), g.ok("ai.status", json!({})));
}
