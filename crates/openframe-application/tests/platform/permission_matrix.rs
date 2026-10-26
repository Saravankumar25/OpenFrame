//! Permission matrix generated from the live registry (Windows phase W3, "harden the core").
//!
//! Every registered operation declares the capability its handler needs (`OperationMetadata`), and
//! `Registry::dispatch` checks it for every caller before the handler runs. These tests walk the
//! registry, so a new operation is covered the day it is registered:
//! - for every operation and every role that lacks its declared capability, the call is refused
//!   with `permission.denied` and the project is not changed;
//! - every read of project content declares a capability (View or stronger); the reads that need
//!   none are application-level and listed here by name, so a new one is a reviewed decision.
//!
//! Today every role holds View (ADR-0009), so "a user without View reads nothing" is enforced by the
//! same check: a role added without View is refused on every read by the first test.

use openframe_application::registry::OpKind;
use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::json;

const ROLES: [Role; 5] = [
    Role::Owner,
    Role::Editor,
    Role::Commenter,
    Role::Viewer,
    Role::ExportOnly,
];

/// Reads that need no open project and return no project content (settings, the app itself,
/// recent projects, the AI runtime state, the team gate and the licence screens).
/// Reviewed 2026-10-16: `ai.health` reads the numbers-only AI trace in app data and
/// `ai.download.status` the download state and free space, never the open project; the reviewer workspace (`packages.review_*`) reads packages kept in the app
/// data folder, and `packages.inspect_package` reads a file the user picked, not the open project.
/// Reviewed 2026-10-27: `storage.info` and `project.discover` read the places OpenFrame keeps things and the
/// project folders found there, never project content.
const APPLICATION_READS: &[&str] = &[
    "ai.diagnostics",
    "ai.download.status",
    "ai.health",
    "ai.status",
    "app.info",
    "app.save_state",
    "app.shortcuts",
    "appearance.check_theme",
    "appearance.get",
    "entitlement.notice",
    "entitlement.status",
    "packages.inspect_package",
    "packages.review_get",
    "packages.review_list",
    "project.current",
    "project.discover",
    "project.list_recent",
    "project.location_check",
    "storage.info",
    "team_gate.status",
    "writing.dictionary",
];

#[test]
fn every_operation_refuses_a_role_without_its_declared_capability() {
    let env = TestEnv::with_project("Permissions", "Feature Film");
    let registry = env.core.registry.clone();
    let before = env.ok("project.current", json!({}));
    let mut refused = 0;
    for name in registry.op_names() {
        let Some(cap) = registry.metadata(name).and_then(|m| m.required_capability) else {
            continue;
        };
        for role in ROLES.into_iter().filter(|r| !r.allows(cap)) {
            let actor = env.actor_with_role(role);
            let e = env
                .call_as(&actor, name, json!({}))
                .expect_err(&format!("{name} ran for {role:?} without {cap:?}"));
            assert_eq!(
                e.code.0, "permission.denied",
                "{name} as {role:?}: {}",
                e.message
            );
            refused += 1;
        }
    }
    assert!(
        refused > 100,
        "only {refused} refusals: the registry walk found too little"
    );
    assert_eq!(
        env.ok("project.current", json!({})),
        before,
        "a refused call changed the project"
    );
}

#[test]
fn every_read_of_project_content_declares_a_capability() {
    let env = TestEnv::new();
    let registry = env.core.registry.clone();
    let mut undeclared: Vec<&str> = registry
        .op_names()
        .into_iter()
        .filter(|name| {
            registry.get(name).is_some_and(|e| e.kind == OpKind::Query)
                && registry
                    .metadata(name)
                    .is_some_and(|m| m.required_capability.is_none())
        })
        .collect();
    undeclared.sort_unstable();
    assert_eq!(
        undeclared, APPLICATION_READS,
        "reads without a declared capability changed; review each one and update APPLICATION_READS"
    );
}

#[test]
fn every_role_can_view_today() {
    // ADR-0009: each role may read the project it is in. If a role without View is ever added,
    // the first test refuses it on every read that declares View.
    for role in ROLES {
        assert!(role.allows(openframe_domain::Capability::View), "{role:?}");
    }
}
