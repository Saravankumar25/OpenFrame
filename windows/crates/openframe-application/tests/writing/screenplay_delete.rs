//! Deleting a whole screenplay (owner decision 2026-10-08, ADR-0012 amendment; masterplan area 11
//! box "Several screenplays per project"). A screenplay goes to Recently Deleted for 30 days and
//! comes back with every draft, locked draft, production number, page setup and link; the
//! Production Source screenplay cannot be deleted; Owner and Editor only; one undo step; the
//! assistant never deletes one; the 30-day purge removes it. Through the public operation
//! registry, the same path as the interface.

use openframe_application::MutationMeta;
use openframe_application::modules::ai::toolbox;
use openframe_application::modules::retention::{self, MAX_PER_RUN};
use openframe_application::registry::{self, AiExposure};
use openframe_application::store::fault::{CommitFault, fail_commits};
use openframe_domain::{Capability, Role, new_id, now_ms};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const DAY: i64 = 24 * 60 * 60 * 1000;

/// A feature project with its first screenplay "Black Rain": (env, screenplay, current draft).
fn setup() -> (TestEnv, String, String) {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let created = env.ok("screenplay.create", json!({}));
    let sp = created["screenplay"]["id"].as_str().unwrap().to_string();
    let draft = created["draft"]["id"].as_str().unwrap().to_string();
    (env, sp, draft)
}

fn doc(env: &TestEnv, draft: &str) -> Value {
    env.ok("screenplay.document", json!({ "draftId": draft }))
}

/// Replace the first element of the draft's first scene.
fn write(env: &TestEnv, draft: &str, text: &str) {
    let el = doc(env, draft)["scenes"][0]["elements"][0]["id"].clone();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [{ "op": "updateElement", "id": el, "text": text }] }),
    );
}

/// The Recently Deleted row of an object, as the screen lists it (Null when not there).
fn deleted_row(env: &TestEnv, object_id: &str) -> Value {
    env.ok("project.deleted_items", json!({}))
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["objectId"] == object_id)
        .cloned()
        .unwrap_or(Value::Null)
}

fn titles(env: &TestEnv, args: Value) -> Vec<String> {
    env.ok("screenplay.overview", args)["screenplays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["title"].as_str().unwrap().to_string())
        .collect()
}

fn hits(env: &TestEnv, text: &str) -> usize {
    env.ok("search.query", json!({ "text": text }))
        .as_array()
        .unwrap()
        .len()
}

fn draft_names(env: &TestEnv, screenplay: &str) -> Vec<String> {
    env.ok("screenplay.overview", json!({ "screenplayId": screenplay }))["drafts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_string())
        .collect()
}

struct Second {
    id: String,
    draft: String,
    scene: String,
    deleted_draft: String,
}

/// A second screenplay with text in its current draft, a production number, a second draft, a
/// deleted third draft, A4 page setup and a locked current draft.
fn rich_second(env: &TestEnv, title: &str, text: &str) -> Second {
    let created = env.ok("screenplay.create", json!({ "title": title }));
    let id = created["screenplay"]["id"].as_str().unwrap().to_string();
    let draft = created["draft"]["id"].as_str().unwrap().to_string();
    write(env, &draft, text);
    let scene = doc(env, &draft)["scenes"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": draft, "name": "Draft 2", "makeCurrent": false }),
    );
    let deleted_draft = env.ok(
        "screenplay.new_draft",
        json!({ "sourceDraftId": draft, "name": "Draft 3", "makeCurrent": false }),
    )["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "screenplay.delete_draft",
        json!({ "draftId": deleted_draft }),
    );
    env.ok(
        "screenplay.set_scene_number",
        json!({ "sceneId": scene, "number": "7" }),
    );
    let mut setup = env.ok("screenplay.page_setup", json!({ "screenplayId": id }))["setup"].clone();
    setup["paper"] = json!("A4");
    env.ok(
        "screenplay.update_page_setup",
        json!({ "screenplayId": id, "setup": setup }),
    );
    env.ok("screenplay.lock_draft", json!({ "draftId": draft }));
    Second {
        id,
        draft,
        scene,
        deleted_draft,
    }
}

#[test]
fn a_deleted_screenplay_waits_in_recently_deleted_and_comes_back_with_everything() {
    let (env, first, _) = setup();
    let s = rich_second(&env, "The Short", "A violet umbrella.");
    let doc_before = doc(&env, &s.draft);
    let overview_before = env.ok("screenplay.overview", json!({ "screenplayId": s.id }));
    assert_eq!(hits(&env, "violet umbrella"), 1);

    env.ok("screenplay.delete", json!({ "screenplayId": s.id }));
    // Gone from the picker, search, and every read or edit of its drafts.
    assert_eq!(titles(&env, json!({})), vec!["Black Rain"]);
    assert_eq!(hits(&env, "violet umbrella"), 0, "search drops it");
    let fallback = env.ok("screenplay.overview", json!({ "screenplayId": s.id }));
    assert_eq!(fallback["screenplay"]["id"], first.as_str());
    assert_eq!(
        env.err("screenplay.document", json!({ "draftId": s.draft })),
        "not_found.draft"
    );
    let el = doc_before["scenes"][0]["elements"][0]["id"].clone();
    assert_eq!(
        env.err(
            "screenplay.apply_edits",
            json!({ "draftId": s.draft, "ops": [{ "op": "updateElement", "id": el, "text": "Lost?" }] })
        ),
        "not_found.draft"
    );
    assert_eq!(
        env.err(
            "screenplay.rename",
            json!({ "screenplayId": s.id, "title": "Other" })
        ),
        "not_found.screenplay"
    );
    assert_eq!(
        env.err("screenplay.delete", json!({ "screenplayId": s.id })),
        "not_found.screenplay",
        "deleting twice"
    );
    // Recently Deleted lists it for 30 days.
    let row = deleted_row(&env, &s.id);
    assert_eq!(row["objectType"], "screenplay");
    assert_eq!(row["typeLabel"], "Screenplay");
    assert_eq!(row["title"], "The Short");
    assert_eq!(row["wasIn"], "Screenplay");
    assert_eq!(row["daysLeft"], 30);
    // Its deleted draft waits for the screenplay.
    let draft_row = deleted_row(&env, &s.deleted_draft);
    assert_eq!(draft_row["wasIn"], "Screenplay “The Short”");
    assert_eq!(draft_row["parentMissing"], true);
    let e = env
        .call("trash.restore", json!({ "id": draft_row["id"] }))
        .unwrap_err();
    assert_eq!(e.code.0, "conflict.state");
    assert!(
        e.message.contains("Restore the screenplay first"),
        "{}",
        e.message
    );

    // Restore brings back every draft, the lock, the number, the page setup and search.
    let restored = env.ok("trash.restore", json!({ "id": row["id"] }));
    assert!(restored.get("restoredNote").is_none(), "{restored}");
    assert_eq!(titles(&env, json!({})), vec!["Black Rain", "The Short"]);
    let after = env.ok("screenplay.overview", json!({ "screenplayId": s.id }));
    assert_eq!(after["drafts"], overview_before["drafts"]);
    assert_eq!(
        after["screenplay"]["pageSetup"],
        overview_before["screenplay"]["pageSetup"]
    );
    assert_eq!(after["screenplay"]["title"], "The Short");
    let doc_after = doc(&env, &s.draft);
    assert_eq!(doc_after["scenes"], doc_before["scenes"]);
    assert_eq!(doc_after["canEdit"], false, "still locked");
    assert_eq!(doc_after["scenes"][0]["label"], "7");
    assert_eq!(
        env.ok("screenplay.pagination", json!({ "draftId": s.draft }))["paper"],
        "A4"
    );
    assert_eq!(
        env.ok("screenplay.locate", json!({ "sceneId": s.scene }))["screenplayId"],
        s.id.as_str()
    );
    assert_eq!(hits(&env, "violet umbrella"), 1, "search has it again");
    assert!(deleted_row(&env, &s.id).is_null());
    // Now its deleted draft can come back too.
    env.ok("trash.restore", json!({ "id": draft_row["id"] }));
    assert_eq!(
        draft_names(&env, &s.id),
        vec!["Draft 1", "Draft 2", "Draft 3"]
    );
}

#[test]
fn deleting_a_screenplay_is_one_undo_step_for_owners_and_editors_only() {
    let (env, _, _) = setup();
    let second = env.ok("screenplay.create", json!({ "title": "The Short" }));
    let sid = second["screenplay"]["id"].as_str().unwrap().to_string();
    let draft = second["draft"]["id"].as_str().unwrap().to_string();
    write(&env, &draft, "A violet umbrella.");
    for role in [Role::Viewer, Role::Commenter, Role::ExportOnly] {
        let a = env.actor_with_role(role);
        let e = env
            .call_as(&a, "screenplay.delete", json!({ "screenplayId": sid }))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
    }
    assert_eq!(titles(&env, json!({})).len(), 2);
    let editor = env.actor_with_role(Role::Editor);
    env.call_as(&editor, "screenplay.delete", json!({ "screenplayId": sid }))
        .unwrap();
    assert_eq!(titles(&env, json!({})), vec!["Black Rain"]);
    // Undo brings it back, search included; redo deletes it again.
    env.undo();
    assert_eq!(titles(&env, json!({})), vec!["Black Rain", "The Short"]);
    assert!(deleted_row(&env, &sid).is_null());
    assert_eq!(hits(&env, "violet umbrella"), 1);
    env.redo();
    assert_eq!(titles(&env, json!({})), vec!["Black Rain"]);
    assert_eq!(hits(&env, "violet umbrella"), 0);
    assert!(!deleted_row(&env, &sid).is_null());

    // The assistant never deletes a screenplay; it can propose restoring one.
    let reg = registry::catalog();
    let meta = reg.metadata("screenplay.delete").unwrap();
    assert!(matches!(meta.ai_exposure, AiExposure::Hidden(_)));
    assert!(meta.destructive && meta.confirmation);
    assert_eq!(meta.required_capability, Some(Capability::SoftDelete));
    assert!(!toolbox::op_exposed(reg, "screenplay.delete"));
    assert!(toolbox::op_exposed(reg, "trash.restore"));
}

#[test]
fn deleting_the_production_source_screenplay_waits_for_another_source() {
    let (env, first, first_draft) = setup();
    let second = env.ok("screenplay.create", json!({ "title": "The Short" }));
    let sid = second["screenplay"]["id"].as_str().unwrap().to_string();
    let sdraft = second["draft"]["id"].as_str().unwrap().to_string();
    env.ok("production.set_source", json!({ "draftId": first_draft }));
    let e = env
        .call("screenplay.delete", json!({ "screenplayId": first }))
        .unwrap_err();
    assert_eq!(e.code.0, "conflict.state");
    assert!(
        e.message.contains("“Black Rain” is the Production Source")
            && e.message
                .contains("Choose a draft of another screenplay as the Production Source first"),
        "{}",
        e.message
    );
    assert_eq!(titles(&env, json!({})).len(), 2, "nothing changed");
    assert!(deleted_row(&env, &first).is_null());
    // Another screenplay can still go, and come back.
    env.ok("screenplay.delete", json!({ "screenplayId": sid }));
    let row = deleted_row(&env, &sid);
    env.ok("trash.restore", json!({ "id": row["id"] }));
    // Once production follows the other screenplay, the first one may be deleted.
    env.ok("production.apply_update", json!({ "draftId": sdraft }));
    env.ok("screenplay.delete", json!({ "screenplayId": first }));
    assert_eq!(titles(&env, json!({})), vec!["The Short"]);
    // It was a Production Source once, so production history keeps it: it is never removed for
    // good, by hand or by the 30-day purge, and stays restorable.
    let row = deleted_row(&env, &first);
    let e = env
        .call("trash.purge", json!({ "id": row["id"] }))
        .unwrap_err();
    assert_eq!(e.code.0, "conflict.state");
    assert!(e.message.contains("Restore it instead"), "{}", e.message);
    let report = retention::purge_expired(
        &env.core.project().unwrap().store,
        &env.actor(),
        now_ms() + 31 * DAY,
        MAX_PER_RUN,
    )
    .unwrap();
    assert_eq!(report.removed, 0, "{report:?}");
    assert!(!deleted_row(&env, &first).is_null(), "still restorable");
    env.ok("trash.restore", json!({ "id": row["id"] }));
    assert_eq!(titles(&env, json!({})), vec!["Black Rain", "The Short"]);
}

#[test]
fn the_last_screenplay_can_be_deleted_and_a_new_one_started() {
    let (env, first, draft) = setup();
    write(&env, &draft, "The only line.");
    env.ok("screenplay.delete", json!({ "screenplayId": first }));
    let ov = env.ok("screenplay.overview", json!({}));
    assert!(ov["screenplay"].is_null(), "like a new project: {ov}");
    assert_eq!(ov["screenplays"].as_array().unwrap().len(), 0);
    assert_eq!(ov["drafts"].as_array().unwrap().len(), 0);
    let fresh = env.ok("screenplay.create", json!({}));
    assert_eq!(
        fresh["screenplay"]["title"], "Black Rain",
        "the title is free again"
    );
    // Restoring the old one while the new one has its title gives it a number, and says so.
    let row = deleted_row(&env, &first);
    let restored = env.ok("trash.restore", json!({ "id": row["id"] }));
    assert_eq!(
        restored["restoredNote"],
        "Another screenplay is now called “Black Rain”, so this one was restored as “Black Rain (2)”."
    );
    let back = env.ok("screenplay.overview", json!({ "screenplayId": first }));
    assert_eq!(back["screenplay"]["title"], "Black Rain (2)");
    assert_eq!(back["screenplay"]["titlePage"]["title"], "Black Rain (2)");
    assert!(doc(&env, &draft).to_string().contains("The only line."));
}

#[test]
fn a_failed_delete_changes_nothing_and_a_finished_one_survives_a_crash() {
    let (env, _, _) = setup();
    let second = env.ok("screenplay.create", json!({ "title": "The Short" }));
    let sid = second["screenplay"]["id"].as_str().unwrap().to_string();
    for f in [CommitFault::DiskFull, CommitFault::WriteFailed] {
        fail_commits(Some(f));
        let r = env.call("screenplay.delete", json!({ "screenplayId": sid }));
        fail_commits(None);
        assert!(r.is_err(), "{f:?}");
        assert_eq!(titles(&env, json!({})).len(), 2, "{f:?}");
        assert!(deleted_row(&env, &sid).is_null(), "{f:?}: no half delete");
    }
    env.ok("screenplay.delete", json!({ "screenplayId": sid }));
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    assert_eq!(titles(&env, json!({})), vec!["Black Rain"]);
    assert!(!deleted_row(&env, &sid).is_null());
}

#[test]
fn a_screenplay_is_removed_for_good_30_days_after_its_delete() {
    let (env, _, first_draft) = setup();
    let s = rich_second(&env, "The Short", "A violet umbrella.");
    env.ok("screenplay.delete", json!({ "screenplayId": s.id }));
    let session = env.core.project().unwrap();
    let store = &session.store;
    let count = |sql: &str, id: &str| -> i64 {
        store
            .read(|c| Ok(c.query_row(sql, [id], |r| r.get(0))?))
            .unwrap()
    };
    let deleted_at: i64 = store
        .read(|c| {
            Ok(c.query_row(
                "SELECT deleted_at FROM deleted_item WHERE object_id=?1",
                [&s.id],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    // One millisecond before the 30 days are up, the screenplay stays. (Draft 3 was deleted a
    // moment earlier, on its own, so its own 30 days may already be over.)
    retention::purge_expired(store, &env.actor(), deleted_at + 30 * DAY - 1, MAX_PER_RUN).unwrap();
    assert!(!deleted_row(&env, &s.id).is_null());
    let r =
        retention::purge_expired(store, &env.actor(), deleted_at + 30 * DAY, MAX_PER_RUN).unwrap();
    assert_eq!(r.skipped, 0, "{r:?}");
    assert!(deleted_row(&env, &s.id).is_null());
    assert!(
        deleted_row(&env, &s.deleted_draft).is_null(),
        "its deleted draft went with it"
    );
    assert_eq!(
        count("SELECT count(*) FROM screenplay WHERE id=?1", &s.id),
        0
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM screenplay_draft WHERE screenplay_id=?1",
            &s.id
        ),
        0
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM screenplay_scene WHERE id=?1",
            &s.scene
        ),
        0
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM screenplay_lock_baseline WHERE draft_id=?1",
            &s.draft
        ),
        0
    );
    assert_eq!(
        count(
            "SELECT count(*) FROM pragma_foreign_key_check WHERE ?1 IS NOT NULL",
            "x"
        ),
        0,
        "no dangling references"
    );
    // The other screenplay is untouched.
    assert_eq!(titles(&env, json!({})), vec!["Black Rain"]);
    assert_eq!(
        doc(&env, &first_draft)["scenes"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn an_episode_may_hold_several_screenplays_and_lose_one_to_recently_deleted() {
    let env = TestEnv::with_project("Series", "Episodic");
    let ep = new_id();
    let s = env.core.project().unwrap();
    s.store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.seed", "Seed", Capability::Edit),
            |tx| {
                tx.conn().execute(
                    "INSERT INTO episode(id, title, position, created_at, updated_at) VALUES (?1, 'Pilot', 1, 0, 0)",
                    [&ep],
                )?;
                Ok(())
            },
        )
        .unwrap();
    env.ok("screenplay.create", json!({ "episodeId": ep }));
    let second = env.ok(
        "screenplay.create",
        json!({ "episodeId": ep, "title": "Pilot (short)" }),
    );
    assert_eq!(
        titles(&env, json!({ "episodeId": ep })),
        vec!["Pilot", "Pilot (short)"]
    );
    let sid = second["screenplay"]["id"].as_str().unwrap().to_string();
    env.ok("screenplay.delete", json!({ "screenplayId": sid }));
    assert_eq!(titles(&env, json!({ "episodeId": ep })), vec!["Pilot"]);
    let row = deleted_row(&env, &sid);
    assert_eq!(row["wasIn"], "Screenplay · Episode “Pilot”");
    // While the episode itself is deleted, its screenplay waits for it.
    env.ok("story.delete_episode", json!({ "id": ep }));
    let e = env
        .call("trash.restore", json!({ "id": row["id"] }))
        .unwrap_err();
    assert_eq!(e.code.0, "conflict.state");
    assert!(
        e.message.contains("Restore the episode first"),
        "{}",
        e.message
    );
    let ep_row = deleted_row(&env, &ep);
    env.ok("trash.restore", json!({ "id": ep_row["id"] }));
    env.ok("trash.restore", json!({ "id": row["id"] }));
    assert_eq!(
        titles(&env, json!({ "episodeId": ep })),
        vec!["Pilot", "Pilot (short)"]
    );
}
