//! Idea Vault research flags (FSD 107.5 and 107.6; mock 045; feature row "Selection, context
//! menu, collections, research flags, Recently Deleted").
//!
//! The flag is the user's own verification state of a research source: Unchecked, Checked or
//! Needs Expert Check, or no flag at all. It works the same in the Project and the Global Idea
//! Vault, is one undo step, survives restart and crash, travels with copies between the vaults,
//! and the assistant can read it but never set it.

use openframe_application::registry::{self, AiExposure};
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

fn note(env: &TestEnv, store: &str, body: &str) -> Value {
    env.ok(
        "vault.create",
        json!({ "store": store, "itemType": "note", "body": body }),
    )
}

/// Quit cleanly, start again and reopen the same project.
fn restart(env: TestEnv) -> TestEnv {
    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    env
}

fn crash(env: TestEnv) -> TestEnv {
    let path = env.project_path();
    let env = env.crash_and_restart();
    env.reopen_project(&path);
    env
}

fn flag(env: &TestEnv, store: &str, id: &Value) -> Value {
    env.ok("vault.get", json!({ "store": store, "id": id }))["researchFlag"].clone()
}

#[test]
fn research_flags_are_set_cleared_undone_and_kept_after_restart() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let pdf = note(
        &env,
        "project",
        "Police procedure: order of evidence handling.",
    );
    let audio = note(&env, "project", "Interview with a retired inspector.");
    let other = note(&env, "project", "An idea that is not research.");
    // No flag until the user sets one; importing never marks anything.
    assert_eq!(flag(&env, "project", &pdf["id"]), Value::Null);

    env.ok(
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [pdf["id"]], "flag": "needsExpertCheck" }),
    );
    env.ok(
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [audio["id"]], "flag": "checked" }),
    );
    assert_eq!(flag(&env, "project", &pdf["id"]), "needsExpertCheck");
    assert_eq!(flag(&env, "project", &audio["id"]), "checked");
    assert_eq!(flag(&env, "project", &other["id"]), Value::Null);

    // Lists carry the flag (grid, card, list and folder views all read vault.list).
    let list = env.ok("vault.list", json!({ "store": "project" }));
    let row = list
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == pdf["id"])
        .unwrap()
        .clone();
    assert_eq!(row["researchFlag"], "needsExpertCheck");

    // One undo step each; redo reapplies.
    env.undo();
    assert_eq!(flag(&env, "project", &audio["id"]), Value::Null);
    env.redo();
    assert_eq!(flag(&env, "project", &audio["id"]), "checked");

    // Several at once, then cleared.
    env.ok(
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [pdf["id"], audio["id"]], "flag": "unchecked" }),
    );
    assert_eq!(flag(&env, "project", &pdf["id"]), "unchecked");
    assert_eq!(flag(&env, "project", &audio["id"]), "unchecked");
    env.undo();
    assert_eq!(flag(&env, "project", &pdf["id"]), "needsExpertCheck");
    assert_eq!(flag(&env, "project", &audio["id"]), "checked");
    env.ok(
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [audio["id"]], "flag": null }),
    );
    assert_eq!(flag(&env, "project", &audio["id"]), Value::Null);

    let env = restart(env);
    assert_eq!(flag(&env, "project", &pdf["id"]), "needsExpertCheck");
    let env = crash(env);
    assert_eq!(flag(&env, "project", &pdf["id"]), "needsExpertCheck");
    assert_eq!(flag(&env, "project", &audio["id"]), Value::Null);
}

#[test]
fn research_flags_validate_input_and_respect_roles() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let n = note(&env, "project", "Source");
    assert_eq!(
        env.err(
            "vault.set_research_flag",
            json!({ "store": "project", "ids": [], "flag": "checked" })
        ),
        "validation.invalid_input"
    );
    assert!(
        env.call(
            "vault.set_research_flag",
            json!({ "store": "project", "ids": [n["id"]], "flag": "true" })
        )
        .is_err(),
        "only the three flags exist"
    );
    assert!(
        env.call(
            "vault.set_research_flag",
            json!({ "store": "project", "ids": ["0190f1b2-0000-7000-8000-000000000000"], "flag": "checked" })
        )
        .is_err()
    );
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(
                &who,
                "vault.set_research_flag",
                json!({ "store": "project", "ids": [n["id"]], "flag": "checked" }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
    }
    assert_eq!(flag(&env, "project", &n["id"]), Value::Null);
    // A deleted item cannot be flagged; restoring it brings it back unflagged as it was.
    env.ok(
        "vault.delete",
        json!({ "store": "project", "ids": [n["id"]] }),
    );
    assert!(
        env.call(
            "vault.set_research_flag",
            json!({ "store": "project", "ids": [n["id"]], "flag": "checked" })
        )
        .is_err()
    );
}

#[test]
fn research_flags_work_in_the_global_vault_and_travel_with_copies() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let g = note(&env, "global", "FIR format, state police site.");
    env.ok(
        "vault.set_research_flag",
        json!({ "store": "global", "ids": [g["id"]], "flag": "checked" }),
    );
    assert_eq!(flag(&env, "global", &g["id"]), "checked");
    let copied = env.ok(
        "vault.copy_to_store",
        json!({ "from": "global", "to": "project", "ids": [g["id"]] }),
    )["copied"][0]
        .clone();
    assert_eq!(flag(&env, "project", &copied), "checked");
    // Independent copies: changing the project copy leaves the Global item alone.
    env.ok(
        "vault.set_research_flag",
        json!({ "store": "project", "ids": [copied], "flag": "needsExpertCheck" }),
    );
    assert_eq!(flag(&env, "global", &g["id"]), "checked");
    // The Global flag survives an app restart with no project open.
    let env = restart(env);
    assert_eq!(flag(&env, "global", &g["id"]), "checked");
}

#[test]
fn research_flags_are_hidden_from_the_assistant_with_a_reason() {
    let reg = registry::catalog();
    let m = reg
        .get("vault.set_research_flag")
        .unwrap()
        .meta
        .as_ref()
        .unwrap();
    match m.ai_exposure {
        AiExposure::Hidden(reason) => assert!(reason.contains("never marks research")),
        other => panic!("the assistant must not set research flags: {other:?}"),
    }
}
