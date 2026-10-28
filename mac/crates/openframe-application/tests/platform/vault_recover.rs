//! Getting the Global Idea Vault back: files in the vault folder that no item uses are listed and
//! added back, and a folder picked for "Locate Existing Vault" is read before anything changes.

use std::fs;
use std::path::{Path, PathBuf};

use openframe_application::MutationMeta;
use openframe_domain::Capability;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn png_bytes() -> Vec<u8> {
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&640u32.to_be_bytes());
    png.extend_from_slice(&480u32.to_be_bytes());
    png.extend_from_slice(&[0u8; 32]);
    png
}

fn global_items(env: &TestEnv) -> Vec<Value> {
    env.ok(
        "vault.list",
        json!({ "store": "global", "view": { "kind": "all" } }),
    )
    .as_array()
    .unwrap()
    .clone()
}

fn orphans(env: &TestEnv) -> Value {
    env.ok("vault.orphan_files", json!({ "store": "global" }))
}

/// What an add that failed before the fix left behind: the copied file is in the vault folder,
/// its rows are gone.
fn leave_orphan(env: &TestEnv, name: &str) -> PathBuf {
    let p = env.write_file(name, &png_bytes());
    let item = env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [p] }),
    )["added"][0]
        .clone();
    let path = PathBuf::from(item["asset"]["path"].as_str().unwrap());
    let (item_id, asset_id) = (
        item["id"].as_str().unwrap().to_string(),
        item["asset"]["id"].as_str().unwrap().to_string(),
    );
    env.core
        .global_store()
        .unwrap()
        .mutate(
            &env.actor(),
            MutationMeta::new("test.lose_rows", "Lost the rows", Capability::Edit),
            |tx| {
                tx.conn()
                    .execute("DELETE FROM vault_item WHERE id = ?1", [&item_id])?;
                tx.conn()
                    .execute("DELETE FROM asset WHERE id = ?1", [&asset_id])?;
                Ok(())
            },
        )
        .unwrap();
    assert!(path.is_file());
    path
}

#[test]
fn files_left_in_the_vault_folder_are_listed_and_added_back() {
    let env = TestEnv::new();
    let kept = env.write_file("kept.png", &png_bytes());
    env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [kept] }),
    );
    let lost = leave_orphan(&env, "lost.png");
    // Staging files, hidden files and files a sync client puts there are not ours.
    let shard = lost.parent().unwrap();
    fs::write(shard.join(".abc.0123456789abcdef.tmp"), b"x").unwrap();
    fs::write(shard.join("desktop.ini"), b"x").unwrap();
    assert_eq!(global_items(&env).len(), 1);

    let o = orphans(&env);
    assert_eq!(o["count"], 1);
    assert_eq!(o["bytes"], png_bytes().len() as u64);
    assert_eq!(
        o["samples"][0],
        lost.file_name().unwrap().to_string_lossy().as_ref()
    );

    let r = env.ok("vault.recover_files", json!({ "store": "global" }));
    assert_eq!(r["added"], 1);
    assert_eq!(r["failed"], 0);
    assert_eq!(orphans(&env)["count"], 0);
    let items = global_items(&env);
    assert_eq!(items.len(), 2);
    let back = items
        .iter()
        .find(|i| i["asset"]["originalName"] == "Recovered file 1.png")
        .expect("the recovered file is an item again");
    assert_eq!(back["itemType"], "image");
    assert_eq!(back["asset"]["mediaType"], "image/png");
    assert_eq!(back["asset"]["width"], 640);
    assert_eq!(back["asset"]["available"], true);
    let path = Path::new(back["asset"]["path"].as_str().unwrap());
    assert_eq!(path, lost, "the item uses the file where it already was");
    assert_eq!(fs::read(path).unwrap(), png_bytes());

    // One undo step; the file stays, so it can be added back again.
    env.ok("history.undo", json!({ "store": "global" }));
    assert_eq!(global_items(&env).len(), 1);
    assert!(lost.is_file());
    env.ok("history.redo", json!({ "store": "global" }));
    assert_eq!(global_items(&env).len(), 2);
}

#[test]
fn nothing_to_recover_changes_nothing() {
    let env = TestEnv::new();
    assert_eq!(orphans(&env)["count"], 0);
    let r = env.ok("vault.recover_files", json!({ "store": "global" }));
    assert_eq!(r["added"], 0);
    assert!(global_items(&env).is_empty());
}

/// A rollback that cannot remove a copy (another program holds it) still fails with the real
/// error, and leaves the file for recovery instead of hiding the problem.
#[cfg(windows)]
#[test]
fn a_held_copy_does_not_change_the_error_of_a_failed_change() {
    use std::os::windows::fs::OpenOptionsExt;
    let env = TestEnv::new();
    let held = env.dir.path().join("held.png");
    fs::write(&held, b"x").unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&held)
        .unwrap();
    let e = env
        .core
        .global_store()
        .unwrap()
        .mutate(
            &env.actor(),
            MutationMeta::new("test.fail", "Fails", Capability::Edit),
            |tx| -> openframe_domain::AppResult<()> {
                tx.created_file(held.clone());
                Err(openframe_domain::AppError::invalid_input("the real reason"))
            },
        )
        .unwrap_err();
    assert_eq!(e.message, "the real reason");
    drop(lock);
    assert!(held.is_file(), "left in place, not lost silently");
}

// ----------------------------------------------------------- Locate Existing Vault

fn inspect(env: &TestEnv, dir: &Path) -> Value {
    env.ok("vault.inspect_folder", json!({ "dir": dir }))
}

#[test]
fn a_picked_folder_is_classified_with_real_counts() {
    let env = TestEnv::new();
    let a = env.write_file("a.png", &png_bytes());
    let b = env.write_file("b.png", &png_bytes());
    let r = env.ok(
        "vault.add_files",
        json!({ "store": "global", "paths": [a, b] }),
    );
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "A note." }),
    );
    // One file goes missing from the folder.
    fs::remove_file(r["added"][1]["asset"]["path"].as_str().unwrap()).unwrap();
    let vault = env.core.vault_dir();

    let v = inspect(&env, &vault);
    assert_eq!(v["kind"], "vault");
    assert_eq!(v["ideas"], 3);
    assert_eq!(v["filesPresent"], 1);
    assert_eq!(v["filesMissing"], 1);
    assert_eq!(v["active"], true);
    assert_eq!(
        PathBuf::from(v["vaultDir"].as_str().unwrap()),
        vault,
        "the vault folder itself"
    );

    let parent = vault.parent().unwrap();
    let p = inspect(&env, parent);
    assert_eq!(p["kind"], "parentOfVault");
    assert_eq!(PathBuf::from(p["vaultDir"].as_str().unwrap()), vault);
    assert_eq!(p["ideas"], 3);

    let empty = env.dir.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    let n = inspect(&env, &empty);
    assert_eq!(n["kind"], "nothing");
    assert!(n["vaultDir"].is_null());
    assert!(
        fs::read_dir(&empty).unwrap().next().is_none(),
        "nothing is created"
    );

    assert_eq!(
        env.err(
            "vault.inspect_folder",
            json!({ "dir": env.dir.path().join("gone") })
        ),
        "not_found.folder"
    );
}

#[test]
fn a_project_folder_and_another_vault_are_told_apart() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let project = PathBuf::from(env.project_path());
    assert_eq!(inspect(&env, &project)["kind"], "project");

    // A copy of the vault elsewhere is a vault, but not the one in use. It has no ideas yet.
    env.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "body": "Opens the vault." }),
    );
    let other = env.dir.path().join("Elsewhere").join("Global Idea Vault");
    fs::create_dir_all(&other).unwrap();
    env.core
        .global_store()
        .unwrap()
        .with_writer(|c| {
            openframe_persistence::backup_to(c, &other.join("vault.sqlite"))?;
            Ok(())
        })
        .unwrap();
    let o = inspect(&env, &other);
    assert_eq!(o["kind"], "vault");
    assert_eq!(o["active"], false);
    assert_eq!(o["ideas"], 1);
    let damaged = env.dir.path().join("Damaged");
    fs::create_dir_all(&damaged).unwrap();
    fs::write(damaged.join("vault.sqlite"), b"not a database").unwrap();
    assert_eq!(
        env.err("vault.inspect_folder", json!({ "dir": damaged })),
        "storage.not_a_vault"
    );
}
