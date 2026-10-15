//! Recently Deleted keeps items for 30 days, then removes them (owner decision 2026-10-07,
//! ADR-0012 amendment; build plan area 9 box "Recently Deleted 30-day purge").
//!
//! The removal runs when a project opens and when the Global Idea Vault opens at app start. It is
//! checked here at the boundary day, with the clock moved back and forward, with a removal
//! interrupted by a failing drive and by its own per-run limit, and against live items, which it
//! must never touch.

use openframe_application::modules::retention::{self, MAX_PER_RUN, PurgeReport, RETENTION_DAYS};
use openframe_application::store::fault::{CommitFault, fail_commits};
use openframe_application::{AppCore, Store};
use openframe_domain::now_ms;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const DAY: i64 = 24 * 60 * 60 * 1000;

fn note(env: &TestEnv, body: &str) -> String {
    env.ok("notes.create", json!({ "body": body }))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn delete_note(env: &TestEnv, id: &str) {
    env.ok("notes.delete", json!({ "id": id }));
}

fn project_store(core: &AppCore) -> std::sync::Arc<openframe_application::ProjectSession> {
    core.project().expect("project open")
}

/// The Recently Deleted rows of a store, as the screen gets them.
fn deleted(env: &TestEnv, store: &str) -> Vec<Value> {
    env.ok("project.deleted_items", json!({ "store": store }))
        .as_array()
        .unwrap()
        .clone()
}

fn deleted_row<'a>(rows: &'a [Value], object_id: &str) -> Option<&'a Value> {
    rows.iter().find(|r| r["objectId"] == object_id)
}

/// Live notes, as the Notes screen lists them: what the purge must never change.
fn live_notes(env: &TestEnv) -> Value {
    env.ok("notes.list", json!({}))
}

/// The delete time recorded for an object.
fn deleted_at(store: &Store, object_id: &str) -> i64 {
    store
        .read(|c| {
            Ok(c.query_row(
                "SELECT deleted_at FROM deleted_item WHERE object_id=?1",
                [object_id],
                |r| r.get(0),
            )?)
        })
        .unwrap()
}

/// Move a delete back in time by `days` (both the record and the object), as if it had happened
/// that long ago. Test scaffolding through the store's own writer connection.
fn age(store: &Store, table: &str, object_id: &str, days: f64) {
    let shift = (days * DAY as f64) as i64;
    store
        .with_writer(|c| {
            c.execute(
                "UPDATE deleted_item SET deleted_at = deleted_at - ?1 WHERE object_id=?2",
                rusqlite::params![shift, object_id],
            )?;
            c.execute(
                &format!("UPDATE {table} SET deleted_at = deleted_at - ?1 WHERE id=?2"),
                rusqlite::params![shift, object_id],
            )?;
            Ok(())
        })
        .unwrap();
}

fn purge(env: &TestEnv, now: i64) -> PurgeReport {
    retention::purge_expired(
        &project_store(&env.core).store,
        &env.actor(),
        now,
        MAX_PER_RUN,
    )
    .unwrap()
}

fn integrity_ok(store: &Store) {
    let check: String = store
        .read(|c| Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(check, "ok");
}

#[test]
fn an_item_is_removed_exactly_30_days_after_its_delete_and_not_a_moment_before() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let keep = note(&env, "Live note, never touched");
    let gone = note(&env, "Deleted note");
    delete_note(&env, &gone);
    let live = live_notes(&env);
    let at = deleted_at(&project_store(&env.core).store, &gone);

    // The last millisecond of day 30: still there, one day left.
    let r = purge(&env, at + 30 * DAY - 1);
    assert_eq!(r, PurgeReport::default());
    assert!(deleted_row(&deleted(&env, "project"), &gone).is_some());
    assert_eq!(retention::days_left(at, at + 30 * DAY - 1), 1);

    // Exactly 30 days: removed for good, and only that item.
    let r = purge(&env, at + 30 * DAY);
    assert_eq!(r.removed, 1, "{r:?}");
    assert_eq!(r.skipped, 0, "{r:?}");
    assert!(deleted_row(&deleted(&env, "project"), &gone).is_none());
    assert_eq!(
        env.err("notes.get", json!({ "id": gone })),
        "not_found.note"
    );
    assert_eq!(live_notes(&env), live, "live notes unchanged");
    env.ok("notes.get", json!({ "id": keep }));
    integrity_ok(&project_store(&env.core).store);

    // Removal for good is not an undo step and wrote no Activity line per item.
    let activity = env.ok("history.activity", json!({}));
    assert!(
        !activity.to_string().contains("more than 30 days"),
        "{activity}"
    );
}

#[test]
fn the_screens_say_how_many_days_are_left() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = note(&env, "Counting down");
    delete_note(&env, &id);
    let s = project_store(&env.core);
    let row = |env: &TestEnv| deleted_row(&deleted(env, "project"), &id).cloned().unwrap();
    assert_eq!(row(&env)["daysLeft"], RETENTION_DAYS);
    age(&s.store, "project_note", &id, 3.0);
    assert_eq!(row(&env)["daysLeft"], 27);
    age(&s.store, "project_note", &id, 26.5);
    assert_eq!(
        row(&env)["daysLeft"],
        1,
        "half a day left still reads 1 day"
    );

    // The Idea Vault screens (project and Global) carry the same count.
    for store in ["project", "global"] {
        let item = env.ok(
            "vault.create",
            json!({ "store": store, "itemType": "note", "body": "Harbour at dawn. హైదరాబాద్ में बारिश" }),
        );
        env.ok(
            "vault.delete",
            json!({ "store": store, "ids": [item["id"]] }),
        );
        let trash = env.ok("vault.trash_list", json!({ "store": store }));
        let row = trash
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["deleted"]["objectId"] == item["id"])
            .unwrap();
        assert_eq!(row["daysLeft"], RETENTION_DAYS, "{store}");
        let listed = deleted(&env, store);
        assert_eq!(
            deleted_row(&listed, item["id"].as_str().unwrap()).unwrap()["daysLeft"],
            RETENTION_DAYS
        );
    }
}

#[test]
fn a_clock_moved_back_never_removes_early_and_never_shows_more_than_30_days() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = note(&env, "Deleted, then the clock went back");
    delete_note(&env, &id);
    let s = project_store(&env.core);
    let at = deleted_at(&s.store, &id);

    // The computer's clock now reads a time before the delete.
    for back in [1, 10, 400] {
        let r = purge(&env, at - back * DAY);
        assert_eq!(r, PurgeReport::default(), "{back} days back");
        assert_eq!(retention::days_left(at, at - back * DAY), RETENTION_DAYS);
    }
    // A delete time in the future (as seen from a clock turned back) shows 30 days.
    age(&s.store, "project_note", &id, -5.0);
    assert_eq!(
        deleted_row(&deleted(&env, "project"), &id).unwrap()["daysLeft"],
        RETENTION_DAYS
    );
    // Reopening the project with that record removes nothing.
    drop(s);
    let path = env.project_path();
    env.ok("project.close", json!({}));
    env.reopen_project(&path);
    assert!(deleted_row(&deleted(&env, "project"), &id).is_some());
}

#[test]
fn a_clock_moved_forward_removes_only_due_items_and_never_live_ones() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let live_ids: Vec<String> = ["One", "Two", "తెలుగు నోట్", "हिंदी नोट"]
        .iter()
        .map(|b| note(&env, b))
        .collect();
    let deleted_ids: Vec<String> = (0..3).map(|i| note(&env, &format!("Old {i}"))).collect();
    for id in &deleted_ids {
        delete_note(&env, id);
    }
    // A vault folder deleted with a live item that had been inside it.
    let folder = env.ok(
        "vault.create_folder",
        json!({ "store": "project", "name": "Stations" }),
    );
    let item = env.ok(
        "vault.create",
        json!({ "store": "project", "itemType": "note", "body": "Platform 4" }),
    );
    env.ok(
        "vault.move_to_folder",
        json!({ "store": "project", "ids": [item["id"]], "folderId": folder["id"] }),
    );
    env.ok(
        "vault.delete_folder",
        json!({ "store": "project", "id": folder["id"] }),
    );
    let live = live_notes(&env);
    let item_before = env.ok("vault.get", json!({ "store": "project", "id": item["id"] }));

    // The clock jumps 31 days ahead: every deleted item is due, nothing live is.
    let r = purge(&env, now_ms() + 31 * DAY);
    assert_eq!(r.removed, 4, "{r:?}");
    assert_eq!(r.skipped, 0, "{r:?}");
    assert!(deleted(&env, "project").is_empty());
    assert_eq!(live_notes(&env), live);
    for id in &live_ids {
        env.ok("notes.get", json!({ "id": id }));
    }
    assert_eq!(
        env.ok("vault.get", json!({ "store": "project", "id": item["id"] })),
        item_before,
        "a live item that was in the removed folder is unchanged"
    );
    integrity_ok(&project_store(&env.core).store);

    // The clock comes back: nothing more happens and nothing comes back.
    assert_eq!(purge(&env, now_ms()), PurgeReport::default());
    assert_eq!(live_notes(&env), live);
}

#[test]
fn an_interrupted_purge_leaves_every_item_whole_and_finishes_later() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let live = note(&env, "Live");
    let ids: Vec<String> = (0..5).map(|i| note(&env, &format!("Due {i}"))).collect();
    for id in &ids {
        delete_note(&env, id);
    }
    let later = now_ms() + 31 * DAY;
    let snapshot = live_notes(&env);

    // The drive fails at every commit: nothing is removed, nothing is half-removed.
    fail_commits(Some(CommitFault::DiskFull));
    let r = purge(&env, later);
    fail_commits(None);
    assert_eq!(r.removed, 0, "{r:?}");
    assert_eq!(r.skipped, 5, "{r:?}");
    assert_eq!(deleted(&env, "project").len(), 5);
    integrity_ok(&project_store(&env.core).store);
    assert_eq!(live_notes(&env), snapshot);

    // Each item is still whole: it can be restored, and a restored item is live again.
    let rows = deleted(&env, "project");
    let restore = deleted_row(&rows, &ids[0]).unwrap()["id"].clone();
    env.ok("trash.restore", json!({ "id": restore }));
    env.ok("notes.get", json!({ "id": ids[0] }));

    // A run stopped by its limit removes whole items only; the next run finishes the rest.
    let r =
        retention::purge_expired(&project_store(&env.core).store, &env.actor(), later, 2).unwrap();
    assert_eq!(
        r,
        PurgeReport {
            removed: 2,
            skipped: 0,
            deferred: 2
        }
    );
    assert_eq!(deleted(&env, "project").len(), 2);
    let r = purge(&env, later);
    assert_eq!(r.removed, 2, "{r:?}");
    assert!(deleted(&env, "project").is_empty());
    env.ok("notes.get", json!({ "id": ids[0] }));
    env.ok("notes.get", json!({ "id": live }));
    integrity_ok(&project_store(&env.core).store);
}

#[test]
fn a_delete_record_that_points_at_a_live_object_is_never_followed() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let id = note(&env, "Live behind a stale record");
    delete_note(&env, &id);
    let other = note(&env, "Record names the wrong table");
    delete_note(&env, &other);
    let s = project_store(&env.core);
    // A received project can carry any record. One says its object is deleted while the object
    // is live; the other names a table that is not the one registered for its type.
    s.store
        .with_writer(|c| {
            c.execute("UPDATE project_note SET deleted_at=NULL WHERE id=?1", [&id])?;
            c.execute(
                "UPDATE deleted_item SET table_name='story_scene_card' WHERE object_id=?1",
                [&other],
            )?;
            Ok(())
        })
        .unwrap();
    let r = purge(&env, now_ms() + 31 * DAY);
    assert_eq!(r.removed, 0, "{r:?}");
    assert_eq!(r.skipped, 2, "{r:?}");
    assert_eq!(
        env.ok("notes.get", json!({ "id": id }))["body"],
        "Live behind a stale record"
    );
}

#[test]
fn the_purge_runs_when_a_project_opens_and_when_the_app_starts() {
    let env = TestEnv::with_project("Night Bus", "Feature Film");
    let due = note(&env, "Deleted 31 days ago");
    let young = note(&env, "Deleted 29 days ago");
    delete_note(&env, &due);
    delete_note(&env, &young);
    {
        let s = project_store(&env.core);
        age(&s.store, "project_note", &due, 31.0);
        age(&s.store, "project_note", &young, 29.0);
    }
    // The Global Idea Vault: one item due, one not.
    let g_due = env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "Old idea" }),
    );
    let g_young = env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "Recent idea" }),
    );
    env.ok(
        "vault.delete",
        json!({ "store": "global", "ids": [g_due["id"], g_young["id"]] }),
    );
    {
        let g = env.core.global_store().unwrap();
        age(&g, "vault_item", g_due["id"].as_str().unwrap(), 45.0);
    }
    let path = env.project_path();

    // Quit and start again: the app start removes the vault item, the project open the note.
    let env = env.restart();
    env.core.startup_maintenance();
    let trash = env.ok("vault.trash_list", json!({ "store": "global" }));
    let ids: Vec<&Value> = trash
        .as_array()
        .unwrap()
        .iter()
        .map(|r| &r["deleted"]["objectId"])
        .collect();
    assert_eq!(ids, vec![&g_young["id"]], "{trash}");

    env.reopen_project(&path);
    let rows = deleted(&env, "project");
    assert!(deleted_row(&rows, &due).is_none(), "{rows:?}");
    let young_row = deleted_row(&rows, &young).unwrap();
    assert_eq!(young_row["daysLeft"], 1);
    assert_eq!(env.err("notes.get", json!({ "id": due })), "not_found.note");

    // The Owner can still remove an item early.
    env.ok("trash.purge", json!({ "id": young_row["id"] }));
    assert!(deleted(&env, "project").is_empty());
}
