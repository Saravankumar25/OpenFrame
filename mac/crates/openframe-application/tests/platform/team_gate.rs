//! Team password gate of the internal builds through the application core (ADR-0024; build plan
//! area 20 box "Team password gate"). The hash is made here from a test-only password with small
//! Argon2id costs; no real team password appears anywhere.

use std::sync::Arc;

use openframe_application::events::RecordingSink;
use openframe_application::modules::entitlement::{TEAM_GATE_ALLOWED, TeamGateUnlockArgs};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::auth::ActorOrigin;
use openframe_entitlement::team_gate::hash_with_costs;
use serde_json::{Value, json};

const TEST_PASSWORD: &str = "test-only team password 42";

fn test_hash() -> String {
    hash_with_costs(TEST_PASSWORD, b"test-only-salt!!", 256, 1, 1).unwrap()
}

fn start(root: &std::path::Path, hash: Option<String>) -> Arc<AppCore> {
    AppCore::new(
        AppConfig {
            team_password_hash: hash,
            ..AppConfig::isolated(root)
        },
        Arc::new(RecordingSink::default()),
    )
    .expect("startup never fails")
}

fn call(core: &AppCore, op: &str, args: Value) -> Result<Value, openframe_domain::AppError> {
    core.dispatch(&core.local_actor(), op, args)
}

fn unlock(core: &AppCore, password: &str) -> Value {
    call(core, "team_gate.unlock", json!({ "password": password })).unwrap()
}

/// Operations a user reaches first: none of them may run before the password.
const GATED: &[(&str, &str)] = &[
    ("project.current", "{}"),
    ("project.list_recent", "{}"),
    ("app.info", "{}"),
    ("entitlement.status", "{}"),
    ("vault.list", r#"{"store":"global"}"#),
    (
        "project.create",
        r#"{"title":"Night Bus","projectType":"Feature Film"}"#,
    ),
];

#[test]
fn nothing_opens_before_the_right_password_and_everything_opens_after() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(dir.path(), Some(test_hash()));
    let status = call(&core, "team_gate.status", json!({})).unwrap();
    assert_eq!(status["required"], true);
    assert_eq!(status["unlocked"], false);
    for (op, args) in GATED {
        let e = call(&core, op, serde_json::from_str(args).unwrap()).unwrap_err();
        assert_eq!(e.code.0, "permission.team_password_required", "{op}");
        assert_eq!(
            e.message,
            "Enter the team password to open OpenFrame Studio."
        );
    }
    // The theme still loads so the password screen looks like the app.
    assert!(TEAM_GATE_ALLOWED.contains(&"appearance.get"));
    call(&core, "appearance.get", json!({})).unwrap();
    // Nothing was created on disk for the Global Idea Vault before the password.
    core.startup_maintenance();
    assert!(
        !dir.path()
            .join("global-vault")
            .join("vault.sqlite")
            .exists()
    );

    let wrong = unlock(&core, "not the team password");
    assert_eq!(wrong["status"]["unlocked"], false);
    assert_eq!(
        wrong["message"],
        "That password is not right. 4 tries left before a wait."
    );
    let right = unlock(&core, TEST_PASSWORD);
    assert_eq!(right["status"]["unlocked"], true);
    assert!(right["message"].is_null());
    // The app start work ran once the gate opened.
    assert!(
        dir.path()
            .join("global-vault")
            .join("vault.sqlite")
            .exists()
    );
    for (op, args) in GATED {
        call(&core, op, serde_json::from_str(args).unwrap())
            .unwrap_or_else(|e| panic!("{op} after unlock: {}", e.code));
    }

    // The next launch asks again.
    drop(core);
    let core = start(dir.path(), Some(test_hash()));
    assert_eq!(
        call(&core, "team_gate.status", json!({})).unwrap()["unlocked"],
        false
    );
    assert!(call(&core, "project.current", json!({})).is_err());
}

#[test]
fn five_wrong_tries_wait_and_a_restart_keeps_the_count_and_the_wait() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(dir.path(), Some(test_hash()));
    for _ in 0..3 {
        unlock(&core, "wrong team password");
    }
    drop(core);
    // A restart remembers the 3 wrong tries.
    let core = start(dir.path(), Some(test_hash()));
    assert_eq!(
        call(&core, "team_gate.status", json!({})).unwrap()["triesBeforeWait"],
        2
    );
    unlock(&core, "wrong team password");
    let fifth = unlock(&core, "wrong team password");
    assert_eq!(
        fifth["message"],
        "That password is not right. Too many wrong tries: wait 1 minute before you try again."
    );
    let wait = fifth["status"]["waitSeconds"].as_u64().unwrap();
    assert!((59..=60).contains(&wait), "{wait}");
    // During the wait even the right password is not checked.
    let waiting = unlock(&core, TEST_PASSWORD);
    assert_eq!(waiting["status"]["unlocked"], false);
    assert_eq!(
        waiting["message"],
        "Too many wrong tries. Try again in 1 minute."
    );
    drop(core);
    // A restart does not end the wait.
    let core = start(dir.path(), Some(test_hash()));
    let status = call(&core, "team_gate.status", json!({})).unwrap();
    assert!(status["waitSeconds"].as_u64().unwrap() > 0, "{status}");
    assert_eq!(unlock(&core, TEST_PASSWORD)["status"]["unlocked"], false);
    // The count file holds numbers only, never what was typed.
    let saved =
        std::fs::read_to_string(dir.path().join("appdata/entitlement/team-gate.json")).unwrap();
    assert!(!saved.contains("wrong team password"), "{saved}");
    assert!(!saved.contains(TEST_PASSWORD), "{saved}");
}

#[test]
fn a_store_build_has_no_gate() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(dir.path(), None);
    let status = call(&core, "team_gate.status", json!({})).unwrap();
    assert_eq!(status["required"], false);
    assert_eq!(status["unlocked"], true);
    for (op, args) in GATED {
        call(&core, op, serde_json::from_str(args).unwrap())
            .unwrap_or_else(|e| panic!("{op}: {}", e.code));
    }
    // Typing a password changes nothing and writes nothing.
    let answer = unlock(&core, TEST_PASSWORD);
    assert_eq!(answer["status"]["required"], false);
    assert!(
        !dir.path()
            .join("appdata/entitlement/team-gate.json")
            .exists()
    );
}

#[test]
fn only_the_person_at_the_computer_can_enter_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(dir.path(), Some(test_hash()));
    let mut assistant = core.local_actor();
    assistant.origin = ActorOrigin::Ai {
        request_id: "r1".into(),
    };
    let e = core
        .dispatch(
            &assistant,
            "team_gate.unlock",
            json!({ "password": TEST_PASSWORD }),
        )
        .unwrap_err();
    assert_eq!(e.code.0, "permission.denied");
    assert_eq!(
        call(&core, "team_gate.status", json!({})).unwrap()["unlocked"],
        false
    );
    // A password never shows in a debug print of the arguments.
    let args: TeamGateUnlockArgs =
        serde_json::from_value(json!({ "password": TEST_PASSWORD })).unwrap();
    assert!(!format!("{args:?}").contains(TEST_PASSWORD));
}
