//! The Mac interface end to end, from the Rust side (Mac phase M2): start the core on a test
//! folder, create a project, create a screenplay and a scene, undo, receive the events, and get
//! coded errors back for bad requests. The Swift package runs the same journey from Swift
//! (apps/macos/Tests/OpenFrameCoreTests).

use std::sync::{Arc, Mutex};

use openframe_ffi::{CoreConfig, CoreError, CoreHandle, EventListener};
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<Value>>>);

impl EventListener for Recorder {
    fn on_event(&self, event_json: String) {
        let v: Value = serde_json::from_str(&event_json).expect("events are JSON");
        self.0.lock().unwrap().push(v);
    }
}

impl Recorder {
    fn types(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .map(|e| e["type"].as_str().unwrap_or("").to_string())
            .collect()
    }
    fn data_changed_tables(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e["type"] == "dataChanged")
            .flat_map(|e| e["tables"].as_array().cloned().unwrap_or_default())
            .filter_map(|t| t.as_str().map(str::to_string))
            .collect()
    }
}

fn start(root: &std::path::Path, rec: &Recorder) -> Arc<CoreHandle> {
    CoreHandle::start(
        CoreConfig {
            app_version: "0.1.0-test".into(),
            isolated_data_root: Some(root.to_string_lossy().into_owned()),
            ai_bundle_dir: None,
            write_logs: false,
        },
        Box::new(rec.clone()),
    )
    .expect("core starts on a test folder")
}

fn call(core: &CoreHandle, op: &str, args: Value) -> Value {
    let out = core
        .invoke(op.into(), args.to_string())
        .unwrap_or_else(|e| panic!("{op} failed: {e:?}"));
    serde_json::from_str(&out).expect("results are JSON")
}

fn code_of(e: CoreError) -> String {
    let CoreError::Failed { code, .. } = e;
    code
}

#[test]
fn a_project_a_scene_an_undo_and_the_events_cross_the_interface() {
    let dir = tempfile::tempdir().unwrap();
    let rec = Recorder::default();
    let core = start(dir.path(), &rec);

    let project = call(
        &core,
        "project.create",
        json!({ "title": "Mac Journey", "projectType": "Feature Film" }),
    );
    assert!(project.is_object(), "{project}");
    assert!(rec.types().iter().any(|t| t == "projectOpened"), "{:?}", rec.types());

    let sp = call(&core, "screenplay.create", json!({}));
    let draft = sp["draft"]["id"].as_str().expect("draft id").to_string();
    let scene = call(
        &core,
        "screenplay.create_scene",
        json!({ "draftId": draft, "heading": "INT. RAILWAY STATION - NIGHT" }),
    );
    assert!(scene["createdIds"][0].is_string(), "{scene}");
    assert!(
        rec.data_changed_tables().iter().any(|t| t.starts_with("screenplay")),
        "{:?}",
        rec.data_changed_tables()
    );

    let doc = call(&core, "screenplay.document", json!({ "draftId": draft }));
    let before = doc.to_string();
    assert!(before.contains("RAILWAY STATION"), "{before}");

    let label = call(&core, "history.undo", json!({}));
    assert!(!label.is_null(), "undo had nothing to undo");
    let after = call(&core, "screenplay.document", json!({ "draftId": draft })).to_string();
    assert!(!after.contains("RAILWAY STATION"), "undo removed the scene: {after}");

    call(&core, "history.redo", json!({}));
    let redone = call(&core, "screenplay.document", json!({ "draftId": draft })).to_string();
    assert!(redone.contains("RAILWAY STATION"));

    core.stop();
    assert!(rec.types().iter().any(|t| t == "projectClosed"), "{:?}", rec.types());
}

#[test]
fn bad_requests_come_back_as_coded_errors_never_panics() {
    let dir = tempfile::tempdir().unwrap();
    let rec = Recorder::default();
    let core = start(dir.path(), &rec);

    let unknown = core.invoke("shell.run_anything".into(), "{}".into()).unwrap_err();
    assert_eq!(code_of(unknown), "security.unknown_operation");

    let not_json = core.invoke("project.list_recent".into(), "{not json".into()).unwrap_err();
    assert_eq!(code_of(not_json), "validation.invalid_input");

    // An empty argument string means `{}`.
    let recent = core.invoke("project.list_recent".into(), String::new()).unwrap();
    assert!(serde_json::from_str::<Value>(&recent).is_ok());

    // No project is open: project operations say so with a code.
    let none = core
        .invoke("screenplay.create".into(), "{}".into())
        .unwrap_err();
    assert!(!code_of(none).is_empty());

    // Paths never come from the shell: unknown locations and non-web links are refused.
    assert!(core.reveal_location("anywhere".into(), None, None).is_err());
    assert!(core.open_url("file:///etc/hosts".into()).is_err());
    core.stop();
}

#[test]
fn the_allow_list_and_the_inventory_agree() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(dir.path(), &Recorder::default());
    let names = core.operation_names();
    assert!(names.len() > 400, "{}", names.len());
    assert!(names.windows(2).all(|w| w[0] < w[1]), "sorted and unique");
    let inventory: Vec<Value> =
        serde_json::from_str(&openframe_ffi::operation_inventory_json()).unwrap();
    let inv: std::collections::BTreeSet<&str> =
        inventory.iter().map(|o| o["name"].as_str().unwrap()).collect();
    let live: std::collections::BTreeSet<&str> = names.iter().map(String::as_str).collect();
    let only_inventory: Vec<_> = inv.difference(&live).collect();
    let only_live: Vec<_> = live.difference(&inv).collect();
    // The live core of a debug build on a test folder adds the hand-set licence operations
    // (`entitlement.debug_*`); every other operation is in both lists.
    assert!(only_inventory.is_empty(), "{only_inventory:?}");
    assert!(
        only_live.iter().all(|n| n.starts_with("entitlement.debug")),
        "{only_live:?}"
    );
    for op in &inventory {
        assert!(matches!(op["kind"].as_str(), Some("query" | "command")), "{op}");
        assert!(!op["argType"].as_str().unwrap().is_empty(), "{op}");
    }
    core.stop();
}

#[test]
fn a_project_reopens_after_the_core_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let rec = Recorder::default();
    let core = start(dir.path(), &rec);
    call(&core, "project.create", json!({ "title": "Restart", "projectType": "Feature Film" }));
    let sp = call(&core, "screenplay.create", json!({}));
    let draft = sp["draft"]["id"].as_str().unwrap().to_string();
    call(
        &core,
        "screenplay.create_scene",
        json!({ "draftId": draft, "heading": "EXT. HARBOUR - DAY" }),
    );
    core.stop();
    drop(core);

    let core = start(dir.path(), &Recorder::default());
    let recent = call(&core, "project.list_recent", json!({}));
    let first = recent
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .expect("the project is in the recent list");
    let path = first["path"].as_str().expect("recent path").to_string();
    call(&core, "project.open", json!({ "path": path }));
    let doc = call(&core, "screenplay.document", json!({ "draftId": draft })).to_string();
    assert!(doc.contains("HARBOUR"), "{doc}");
    core.stop();
}
