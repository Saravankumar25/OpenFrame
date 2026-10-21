//! The Offline AI with the real models: installed through the app's own "use a model file I
//! already have" path, then real questions, proposals applied, rejected and undone.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s, truncate};

fn tasks(g: &Gold) -> usize {
    g.sql_i64("SELECT count(*) FROM task WHERE deleted_at IS NULL")
        .max(0) as usize
}

fn ask(g: &Gold, ctx: &mut Ctx, label: &str, text: &str, scope: Value) -> Value {
    let started = std::time::Instant::now();
    let x = g.ok("ai.ask", json!({ "text": text, "scope": scope }));
    let r = &x["result"];
    let secs = started.elapsed().as_secs_f32();
    println!(
        "  AI {label}: {:.1}s kind={} {}",
        secs,
        r["kind"],
        truncate(&s(&r["content"]), 160)
    );
    let mut log = ctx.facts.get("aiLog").cloned().unwrap_or(json!([]));
    log.as_array_mut().unwrap().push(json!({
        "label": label, "question": text, "scope": scope["kind"], "seconds": (secs * 10.0).round() / 10.0,
        "kind": r["kind"], "answer": r["content"], "confidence": r["confidence"],
        "sources": arr(&r["provenance"]).len(), "changeSet": r["changeSet"]["id"], "operations": r["changeSet"]["operationCount"]
    }));
    ctx.facts.insert("aiLog".into(), log);
    x
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Offline AI");
    g.ok("ai.status", json!({}));
    g.ok("ai.download.status", json!({}));
    if !ctx.ai_enabled {
        ctx.facts.insert(
            "aiSkipped".into(),
            json!("run with the AI build inputs present to include the Offline AI"),
        );
        return;
    }
    // Nothing is downloading, so Pause has nothing to stop; the files are imported instead.
    let paused = g.probe("ai.download.pause", json!({}));
    ctx.facts.insert("aiPauseWithoutDownload".into(), paused);
    // Install the three pinned model files the way a person who already has them would.
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

    ask(
        g,
        ctx,
        "count",
        "How many scenes are in the script?",
        json!({ "kind": "SpecificDraft", "draftId": ctx.id("draftBlue") }),
    );
    ask(
        g,
        ctx,
        "count-current",
        "How many scenes does this screenplay have?",
        json!({ "kind": "CurrentScreenplay" }),
    );
    ask(
        g,
        ctx,
        "grounded",
        "Which scenes take place at the breakwater?",
        json!({ "kind": "WholeProject" }),
    );
    ask(
        g,
        ctx,
        "production",
        "Which shooting day has the most scenes?",
        json!({ "kind": "Production" }),
    );
    ask(
        g,
        ctx,
        "scene",
        "Who is in this scene?",
        json!({ "kind": "CurrentScene", "sceneId": ctx.scenes[&24] }),
    );
    ask(
        g,
        ctx,
        "story",
        "Summarise Act One in two sentences.",
        json!({ "kind": "StoryBoard" }),
    );
    ask(
        g,
        ctx,
        "product",
        "How do I export the call sheet as a PDF?",
        json!({ "kind": "WholeProject" }),
    );

    // Proposal 1: applied, then undone with one Undo.
    let before = tasks(g);
    let x = ask(
        g,
        ctx,
        "propose-apply-undo",
        "Create a task to confirm the rain tower water tanker for shooting day 9.",
        json!({ "kind": "WholeProject" }),
    );
    let cs = s(&x["result"]["changeSet"]["id"]);
    if !cs.is_empty() {
        g.ok("ai.change_set.get", json!({ "id": cs }));
        g.ok("ai.change_set.recheck", json!({ "id": cs }));
        let applied = g.ok("ai.change_set.accept", json!({ "id": cs }));
        let after_apply = tasks(g);
        g.ok("history.undo", json!({ "store": "project" }));
        let after_undo = tasks(g);
        ctx.facts.insert("aiApplyUndo".into(), json!({ "state": applied["state"], "tasksBefore": before, "afterApply": after_apply, "afterUndo": after_undo }));
    }
    // Proposal 2: rejected.
    let x = ask(
        g,
        ctx,
        "propose-reject",
        "Create a task to book a second Nagra recorder as a backup.",
        json!({ "kind": "WholeProject" }),
    );
    let cs = s(&x["result"]["changeSet"]["id"]);
    if !cs.is_empty() {
        let before = tasks(g);
        let r = g.ok("ai.change_set.reject", json!({ "id": cs }));
        ctx.facts.insert(
            "aiReject".into(),
            json!({ "state": r["state"], "tasksBefore": before, "tasksAfter": tasks(g) }),
        );
    }
    // Proposal 3: applied and kept.
    let x = ask(
        g,
        ctx,
        "propose-apply-keep",
        "Create a task to print the breakwater safety plan for every department head.",
        json!({ "kind": "WholeProject" }),
    );
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
    g.ok(
        "ai.export_report",
        json!({ "path": ctx.outp("ai/ai-report.json") }),
    );
    g.ok("ai.stop_runtime", json!({}));
    g.ok("ai.status", json!({}));
    // Disk space for the shoot laptop: the models are removed after the session (AI is off
    // until the next import or download).
    g.ok("ai.download.remove", json!({}));
    ctx.facts
        .insert("aiStatusAfterRemove".into(), g.ok("ai.status", json!({})));
}
