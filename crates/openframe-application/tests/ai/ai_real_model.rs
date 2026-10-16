//! The assistant with the REAL model (ignored by default; ADR-0026). Needs the engine staged by
//! `node scripts/prepare-ai-assets.mjs` and the pinned models in `.ai-assets` (or
//! OPENFRAME_AI_ASSETS_DIR), installed in the test's app data as a finished download would:
//!
//! ```text
//! OPENFRAME_AI_BUNDLE_DIR=<repo>/.build-cache/ai-bundle cargo test -p openframe-application --test ai ai_real_model:: -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Deterministic settings (temperature 0, fixed seed). The assertions check structure and
//! effects, never wording: an exact count from project data, a grounded answer with sources, a
//! proposal that changes nothing until it is applied, apply and undo, reject. The same requests
//! are what the desktop end-to-end AI specs send through the real UI (`tests/e2e/specs/7x-ai-*`).

use std::sync::Arc;
use std::time::Instant;

use openframe_application::core::{AppConfig, AppCore};
use openframe_application::events::RecordingSink;
use serde_json::{Value, json};

struct Env {
    _dir: tempfile::TempDir,
    core: Arc<AppCore>,
}

impl Env {
    fn ok(&self, op: &str, args: Value) -> Value {
        self.core
            .dispatch(&self.core.local_actor(), op, args)
            .unwrap_or_else(|e| panic!("{op} failed: {e:?}"))
    }
    fn ask(&self, text: &str, scope: Value) -> Value {
        let t = Instant::now();
        let x = self.ok("ai.ask", json!({ "text": text, "scope": scope }));
        let r = &x["result"];
        println!(
            "\n> {text}\n  {:.1}s kind={} confidence={} steps={:?}\n  {}\n  sources: {}",
            t.elapsed().as_secs_f32(),
            r["kind"],
            r["confidence"],
            r["steps"]
                .as_array()
                .map(|s| s.iter().map(|x| x["label"].clone()).collect::<Vec<_>>()),
            r["content"],
            r["provenance"]
        );
        x
    }
}

fn env() -> Option<Env> {
    let bundle = std::env::var_os("OPENFRAME_AI_BUNDLE_DIR")?;
    let dir = tempfile::tempdir().unwrap();
    let assets = std::env::var_os("OPENFRAME_AI_ASSETS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ai-assets")
        });
    let app_data = AppConfig::isolated(dir.path()).app_data_dir;
    let source = openframe_ai::test_support::install_pinned_models(
        &app_data,
        &assets,
        &openframe_ai::AssetLock::embedded().unwrap(),
    )
    .unwrap();
    let config = AppConfig {
        ai_bundle_dir: Some(bundle.into()),
        ai_model_source: Some(source),
        ai_deterministic: true,
        ..AppConfig::isolated(dir.path())
    };
    let core = AppCore::new(config, Arc::new(RecordingSink::default())).unwrap();
    Some(Env { _dir: dir, core })
}

/// A small film: three Scene Cards built into a screenplay, plus a location.
fn seed(env: &Env) {
    env.ok(
        "project.create",
        json!({ "title": "Night Bus", "projectType": "Feature Film" }),
    );
    let act = env.ok("story.create_act", json!({ "title": "Act One" }))["id"].clone();
    let seq = env.ok(
        "story.create_sequence",
        json!({ "actId": act, "title": "The return" }),
    )["id"]
        .clone();
    let mut cards = Vec::new();
    for (heading, text) in [
        (
            "INT. POLICE STATION - NIGHT",
            "Arjun confesses to Inspector Rao.",
        ),
        (
            "EXT. BUS STOP - NIGHT",
            "Arjun meets Meera again at a rainy bus stop.",
        ),
        (
            "EXT. RAILWAY STATION - DAY",
            "Meera waits for the late train with a red umbrella.",
        ),
    ] {
        let id = env.ok(
            "story.create_card",
            json!({ "parent": { "parentType": "sequence", "parentId": seq }, "shortDescription": text }),
        )["id"]
            .clone();
        env.ok(
            "story.update_card",
            json!({ "id": id, "sceneHeading": heading }),
        );
        cards.push(json!({ "cardId": id }));
    }
    env.ok(
        "story.build_screenplay",
        json!({ "include": cards, "destination": "newScreenplay" }),
    );
}

fn tasks(env: &Env) -> usize {
    env.ok("tasks.list", json!({}))
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0)
}

#[test]
#[ignore]
fn real_model_answers_retrieves_proposes_applies_and_undoes() {
    let Some(env) = env() else {
        eprintln!("OPENFRAME_AI_BUNDLE_DIR not set; skipping");
        return;
    };
    let status = env.ok("ai.status", json!({}));
    assert_eq!(status["available"], true, "{status}");
    seed(&env);

    // Exact count from project data.
    let x = env.ask(
        "How many scenes are in the script?",
        json!({ "kind": "CurrentScreenplay" }),
    );
    assert_eq!(x["result"]["kind"], "Answer", "{x}");
    assert!(
        x["result"]["content"].as_str().unwrap().contains('3'),
        "{x}"
    );

    // Grounded in the project: the answer names its sources.
    let x = env.ask(
        "Which scene happens at the bus stop?",
        json!({ "kind": "WholeProject" }),
    );
    assert!(
        matches!(x["result"]["kind"].as_str(), Some("Answer" | "Navigate")),
        "{x}"
    );
    assert!(
        !x["result"]["provenance"].as_array().unwrap().is_empty(),
        "{x}"
    );

    // A proposal changes nothing until it is applied; apply, then undo.
    let before = tasks(&env);
    let x = env.ask(
        "Create a task to scout the railway station.",
        json!({ "kind": "WholeProject" }),
    );
    assert_eq!(x["result"]["kind"], "Proposal", "{x}");
    let cs = x["result"]["changeSet"]["id"].as_str().unwrap().to_string();
    assert_eq!(tasks(&env), before, "nothing changes before Apply");
    let proposed = x["result"]["changeSet"]["operationCount"].as_u64().unwrap() as usize;
    assert!(proposed >= 1, "{x}");
    let applied = env.ok("ai.change_set.accept", json!({ "id": cs }));
    assert_eq!(applied["state"], "Applied", "{applied}");
    // The small model may propose the task more than once in one Change Set; what is applied is
    // exactly what was shown for review.
    assert_eq!(tasks(&env), before + proposed);
    env.ok("history.undo", json!({}));
    assert_eq!(
        tasks(&env),
        before,
        "one Undo reverses the whole Change Set"
    );

    // Reject leaves the project unchanged.
    let x = env.ask(
        "Create a task to book the bus for the night shoot.",
        json!({ "kind": "WholeProject" }),
    );
    assert_eq!(x["result"]["kind"], "Proposal", "{x}");
    let cs = x["result"]["changeSet"]["id"].as_str().unwrap().to_string();
    let rejected = env.ok("ai.change_set.reject", json!({ "id": cs }));
    assert_eq!(rejected["state"], "Rejected");
    assert_eq!(tasks(&env), before);
}

#[test]
#[ignore]
fn real_model_proposal_whose_target_changed_is_stale_and_never_applied() {
    let Some(env) = env() else {
        return;
    };
    seed(&env);
    let task = env.ok("tasks.create", json!({ "title": "Confirm pier permit" }))["id"].clone();
    let x = env.ask(
        "Mark the task Confirm pier permit as done.",
        json!({ "kind": "WholeProject" }),
    );
    assert_eq!(x["result"]["kind"], "Proposal", "{x}");
    let cs = x["result"]["changeSet"]["id"].as_str().unwrap().to_string();
    // The user completes the task by hand before applying the proposal.
    env.ok("tasks.set_done", json!({ "id": task, "done": true }));
    let after = env.ok("ai.change_set.accept", json!({ "id": cs }));
    assert!(
        matches!(after["state"].as_str(), Some("Stale" | "Conflict")),
        "{after}"
    );
    assert_eq!(
        after["appliedOperations"], 0,
        "a stale proposal is never applied"
    );
}
