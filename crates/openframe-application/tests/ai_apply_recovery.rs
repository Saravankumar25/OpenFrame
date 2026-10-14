//! Crash-safe application of multi-operation AI Change Sets (ADR-0019; build plan item crash-safe AI changes, forced stop at every apply step;
//! feature AI-005).
//!
//! The process is stopped at every apply boundary (after acceptance, after each operation, after
//! the undo steps are grouped) and the project is reopened as after a crash. On reopen the
//! project must be exactly as it was before the proposal: no AI-created rows, no extra undo steps,
//! nothing to redo, the Change Set `Failed` with a plain explanation, the journal closed and an
//! Activity entry. A completed apply is never touched by recovery.
//!
//! Debug builds only: the forced stops use the debug-only fault-injection point.

#![cfg(debug_assertions)]

use openframe_application::modules::ai::{ChangeSetDraft, OpCall, PreviewRow, change_set};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const OPS: usize = 3;

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

fn text(env: &TestEnv, sql: &str) -> Option<String> {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| {
            use rusqlite::OptionalExtension;
            Ok(c.query_row(sql, [], |r| r.get(0)).optional()?)
        })
        .unwrap()
}

fn cards(env: &TestEnv) -> i64 {
    count(
        env,
        "SELECT count(*) FROM story_scene_card WHERE deleted_at IS NULL",
    )
}

fn done_steps(env: &TestEnv) -> i64 {
    count(env, "SELECT count(*) FROM sys_undo WHERE state='done'")
}

fn redo_steps(env: &TestEnv) -> i64 {
    count(env, "SELECT count(*) FROM sys_undo WHERE state='undone'")
}

fn recoveries(env: &TestEnv) -> i64 {
    count(
        env,
        "SELECT count(*) FROM sys_activity WHERE action='ai.change_set.recover'",
    )
}

fn journal_state(env: &TestEnv, id: &str) -> Option<String> {
    text(
        env,
        &format!("SELECT state FROM sys_ai_apply_journal WHERE change_set_id='{id}'"),
    )
}

fn draft(title: &str, operations: Vec<OpCall>) -> ChangeSetDraft {
    ChangeSetDraft {
        title: title.into(),
        summary: "Crash-safety test".into(),
        operations,
        preview: vec![PreviewRow::normal("Impact", "new objects")],
        exclusions: Vec::new(),
        targets: Vec::new(),
        modules: vec!["Story".into()],
        base_rows: Vec::new(),
        source_tool: "propose_scene_card".into(),
        source_args: json!({}),
        sources: Vec::new(),
    }
}

fn card_op(parent: Value, text: &str) -> OpCall {
    OpCall {
        op: "story.create_card".into(),
        args: json!({ "parent": parent, "shortDescription": text }),
        label: format!("Create {text}"),
    }
}

/// A Change Set that creates `OPS` parking-lot cards.
fn prepare(env: &TestEnv) -> String {
    let ops = (1..=OPS)
        .map(|i| card_op(json!({ "parentType": "parking" }), &format!("AI card {i}")))
        .collect();
    change_set::create(&env.core, &env.actor(), &draft("Three cards", ops))
        .unwrap()
        .id
}

/// Earlier user work that recovery must never touch.
fn user_card(env: &TestEnv) {
    env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "parking" }, "shortDescription": "Written by the user" }),
    );
}

fn reopen_after_crash(env: TestEnv) -> TestEnv {
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    env
}

fn change_set_json(env: &TestEnv, id: &str) -> Value {
    env.ok("ai.change_set.get", json!({ "id": id }))
}

#[test]
fn adr_0019_stop_after_every_apply_boundary_reopens_consistent() {
    // Boundaries: 0 = accepted, k = after operation k, OPS + 1 = after grouping.
    for boundary in 0..=OPS + 1 {
        let env = TestEnv::with_project("Night Bus", "Feature Film");
        user_card(&env);
        let id = prepare(&env);
        let cards_before = cards(&env);
        let steps_before = done_steps(&env);

        change_set::fault::crash_apply_at(Some(boundary));
        let err = env
            .call("ai.change_set.accept", json!({ "id": id }))
            .unwrap_err();
        change_set::fault::crash_apply_at(None);
        assert!(
            err.detail
                .as_deref()
                .unwrap_or_default()
                .contains("fault injection"),
            "boundary {boundary}: {err:?}"
        );
        assert_eq!(
            journal_state(&env, &id).as_deref(),
            Some("applying"),
            "boundary {boundary}"
        );

        let env = reopen_after_crash(env);
        assert_eq!(
            cards(&env),
            cards_before,
            "boundary {boundary}: AI cards remain after recovery"
        );
        assert_eq!(
            done_steps(&env),
            steps_before,
            "boundary {boundary}: undo history changed"
        );
        assert_eq!(
            redo_steps(&env),
            0,
            "boundary {boundary}: the partial apply can be redone"
        );
        assert_eq!(
            journal_state(&env, &id).as_deref(),
            Some("compensated"),
            "boundary {boundary}"
        );
        let cs = change_set_json(&env, &id);
        assert_eq!(cs["state"], "Failed", "boundary {boundary}: {cs}");
        assert_eq!(
            cs["errorMessage"],
            change_set::RECOVERED_MESSAGE,
            "boundary {boundary}"
        );
        assert_eq!(
            recoveries(&env),
            1,
            "boundary {boundary}: recovery not recorded in Activity"
        );
        // The user's own earlier work is intact and still the next step to undo.
        assert_eq!(
            text(
                &env,
                "SELECT short_description FROM story_scene_card WHERE deleted_at IS NULL"
            ),
            Some("Written by the user".into()),
            "boundary {boundary}"
        );
        env.undo();
        assert_eq!(
            cards(&env),
            0,
            "boundary {boundary}: the user's step is no longer undoable"
        );
    }
}

#[test]
fn recovery_runs_once_and_a_second_open_changes_nothing() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = prepare(&env);
    change_set::fault::crash_apply_at(Some(2));
    assert!(
        env.call("ai.change_set.accept", json!({ "id": id }))
            .is_err()
    );
    change_set::fault::crash_apply_at(None);
    let env = reopen_after_crash(env);
    let snapshot = (cards(&env), done_steps(&env), redo_steps(&env));
    let env = reopen_after_crash(env);
    assert_eq!((cards(&env), done_steps(&env), redo_steps(&env)), snapshot);
    assert_eq!(recoveries(&env), 1, "recovery ran twice");
}

#[test]
fn a_completed_apply_closes_the_journal_and_survives_a_crash() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = prepare(&env);
    let before = cards(&env);
    let r = env.ok("ai.change_set.accept", json!({ "id": id }));
    assert_eq!(r["state"], "Applied", "{r}");
    assert_eq!(journal_state(&env, &id).as_deref(), Some("applied"));
    let env = reopen_after_crash(env);
    assert_eq!(
        cards(&env),
        before + OPS as i64,
        "a completed apply was compensated"
    );
    assert_eq!(change_set_json(&env, &id)["state"], "Applied");
    assert_eq!(journal_state(&env, &id).as_deref(), Some("applied"));
    assert_eq!(recoveries(&env), 0);
    // Still one undo step for the whole proposal.
    env.undo();
    assert_eq!(cards(&env), before);
}

#[test]
fn a_live_failure_closes_the_journal_as_compensated() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let ops = vec![
        card_op(json!({ "parentType": "parking" }), "First"),
        card_op(
            json!({ "parentType": "act", "parentId": "missing-act" }),
            "Second",
        ),
    ];
    let id = change_set::create(&env.core, &env.actor(), &draft("One good, one bad", ops))
        .unwrap()
        .id;
    let r = env.ok("ai.change_set.accept", json!({ "id": id }));
    assert_eq!(r["state"], "Failed", "{r}");
    assert_eq!(cards(&env), 0);
    assert_eq!(journal_state(&env, &id).as_deref(), Some("compensated"));
    // Nothing is left for recovery to do on the next open.
    let env = reopen_after_crash(env);
    assert_eq!(cards(&env), 0);
    assert_eq!(recoveries(&env), 0);
}

#[test]
fn a_rejected_or_pending_change_set_never_opens_a_journal() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let pending = prepare(&env);
    let rejected = prepare(&env);
    env.ok("ai.change_set.reject", json!({ "id": rejected }));
    let env = reopen_after_crash(env);
    assert_eq!(journal_state(&env, &pending), None);
    assert_eq!(journal_state(&env, &rejected), None);
    assert_eq!(change_set_json(&env, &pending)["state"], "Pending");
    assert_eq!(recoveries(&env), 0);
}

/// Operations a proposal may use whose registry metadata declares a filesystem effect, each with
/// the reviewed reason the effect can't happen inside a Change Set (ADR-0019 decision 4).
const REVIEWED_FS_OPERATIONS: &[(&str, &str)] = &[
    (
        "storyboard.add_panel",
        "writes an image only for `visual` image (`path`) or sketch (`dataBase64`); propose_storyboard_panel always builds `placeholder` and apply refuses both arguments",
    ),
    (
        "story.create_character",
        "reads an image only through `imagePath`, which the propose_character schema does not have; apply refuses file arguments",
    ),
];

/// ADR-0019 decision 4: compensation can only undo what the undo log holds, so a Change Set
/// must be database-only. Every proposal operation with a filesystem effect is reviewed above,
/// and no proposal schema can produce a file argument.
#[test]
fn adr_0019_every_change_set_operation_is_database_only() {
    use openframe_application::modules::ai::catalog;
    use openframe_application::registry::{self, FsEffect};
    let mut checked = 0;
    let mut unreviewed: Vec<String> = Vec::new();
    for spec in catalog::PROPOSALS.iter() {
        let schema = (spec.schema)();
        for key in schema["properties"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
        {
            // References to project records (`file`, `character`) are ids, not paths.
            let lower = key.to_lowercase();
            assert!(
                !change_set::FILE_ARGUMENTS.contains(&key.as_str()) && !lower.ends_with("path"),
                "proposal {} accepts `{key}`, which looks like a file argument",
                spec.tool
            );
        }
        for op in spec.required_ops.iter().chain(spec.optional_ops) {
            if !catalog::allowed_op(op) {
                continue;
            }
            let meta = registry::catalog()
                .metadata(op)
                .unwrap_or_else(|| panic!("{op} has no metadata"));
            if !matches!(meta.filesystem_effect, FsEffect::None)
                && !REVIEWED_FS_OPERATIONS.iter().any(|(o, _)| o == op)
            {
                unreviewed.push(format!("{op} (proposal {})", spec.tool));
            }
            checked += 1;
        }
    }
    unreviewed.sort();
    unreviewed.dedup();
    assert!(
        unreviewed.is_empty(),
        "operations with a filesystem effect that undo cannot compensate (ADR-0019 decision 4): {unreviewed:#?}"
    );
    assert!(checked > 10, "only {checked} proposal operations checked");
}

#[test]
fn a_change_set_carrying_a_file_argument_is_refused_and_nothing_changes() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let picture = env.write_file("face.png", b"\x89PNG\r\n\x1a\n");
    let op = OpCall {
        op: "story.create_character".into(),
        args: json!({ "name": "Arjun", "imagePath": picture }),
        label: "Create character".into(),
    };
    let id = change_set::create(&env.core, &env.actor(), &draft("Tampered", vec![op]))
        .unwrap()
        .id;
    let r = env.ok("ai.change_set.accept", json!({ "id": id }));
    assert_eq!(r["state"], "Failed", "{r}");
    assert_eq!(count(&env, "SELECT count(*) FROM story_character"), 0);
    assert_eq!(
        journal_state(&env, &id),
        None,
        "refused before acceptance, so no journal"
    );
}

#[test]
fn duplicating_a_project_with_an_unfinished_apply_never_claims_it_was_undone() {
    // The copy has no undo history, so the unfinished apply in it can't be compensated; it must
    // say so instead of reporting that nothing remains (review finding H1).
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = prepare(&env);
    change_set::fault::crash_apply_at(Some(2));
    assert!(
        env.call("ai.change_set.accept", json!({ "id": id }))
            .is_err()
    );
    change_set::fault::crash_apply_at(None);
    let project_id = env.ok("project.current", json!({}))["id"]
        .as_str()
        .unwrap()
        .to_string();
    let env = env.crash_and_restart();
    let copy = env.ok("project.duplicate", json!({ "projectId": project_id }));
    env.reopen_project(copy["path"].as_str().expect("copy path"));
    assert_eq!(
        journal_state(&env, &id).as_deref(),
        Some("compensation_failed")
    );
    let cs = change_set_json(&env, &id);
    assert_eq!(cs["state"], "Failed", "{cs}");
    assert_eq!(cs["errorMessage"], change_set::RECOVERY_INCOMPLETE_MESSAGE);
    assert_eq!(
        recoveries(&env),
        0,
        "recovery must not report a compensation it could not do"
    );
}
