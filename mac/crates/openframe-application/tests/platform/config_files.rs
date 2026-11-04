//! Small settings files that are cut short or filled with garbage (a crash during a write, a
//! full disk, a sync tool, a person editing by hand). The app must still start and work, and
//! the file that holds the person's identity must be kept, never thrown away.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use openframe_application::events::RecordingSink;
use openframe_application::{AppConfig, AppCore};
use serde_json::json;

/// Where a file lives and which configuration puts it there.
struct Case {
    path: &'static str,
    /// The first start of a new computer (the pointer file lives next to the roaming folder).
    needs_setup: bool,
}

const CASES: [Case; 7] = [
    Case {
        path: "appdata/entitlement/notices.json",
        needs_setup: false,
    },
    Case {
        path: "library.json",
        needs_setup: false,
    },
    Case {
        path: "roaming/OpenFrame/library-pointer.json",
        needs_setup: true,
    },
    Case {
        path: "appdata/ai/bundle-check.json",
        needs_setup: false,
    },
    Case {
        path: "appdata/ai/graphics.json",
        needs_setup: false,
    },
    Case {
        path: "appdata/engines.json",
        needs_setup: false,
    },
    Case {
        path: "appdata/models/active.json",
        needs_setup: false,
    },
];

/// A believable file of each kind, then cut in half.
const GOOD: &str = r#"{"version":1,"format":1,"userId":"u-1","displayName":"Asha","welcomeShown":true,"previous":null,"files":[{"rel":"a","bytes":10}],"updatedAt":1700000000000}"#;

fn corruptions() -> [(&'static str, Vec<u8>); 3] {
    [
        ("truncated", GOOD.as_bytes()[..GOOD.len() / 2].to_vec()),
        ("garbage", b"\x00\xff\x01 not json at all {{{[".to_vec()),
        ("empty", Vec::new()),
    ]
}

fn start(root: &Path, needs_setup: bool) -> Arc<AppCore> {
    let config = if needs_setup {
        AppConfig::isolated_needing_setup(root)
    } else {
        AppConfig::isolated(root)
    };
    AppCore::new(config, Arc::new(RecordingSink::default())).expect("the app starts")
}

/// The app answers. Before the library is chosen most screens wait for the choice; that is the
/// normal first start, not a failure.
fn works(core: &AppCore, needs_setup: bool) {
    let actor = core.local_actor();
    core.dispatch(&actor, "app.info", json!({}))
        .expect("app.info");
    let op = if needs_setup {
        "library.status"
    } else {
        "ai.status"
    };
    core.dispatch(&actor, op, json!({})).expect(op);
}

fn quit(core: Arc<AppCore>) {
    core.shutdown();
    drop(core);
}

fn unreadable_copies(dir: &Path, name: &str) -> Vec<PathBuf> {
    let prefix = format!("{name}.unreadable-");
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&prefix))
        })
        .collect()
}

#[test]
fn a_cut_short_or_garbage_settings_file_never_stops_the_app() {
    for case in &CASES {
        for (kind, bytes) in corruptions() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join(case.path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, &bytes).unwrap();

            let core = start(dir.path(), case.needs_setup);
            works(&core, case.needs_setup);
            quit(core);

            // A second start over whatever the first one left behind also works.
            let core = start(dir.path(), case.needs_setup);
            works(&core, case.needs_setup);
            quit(core);

            if case.path == "library.json" && !bytes.is_empty() {
                // The identity file is never overwritten or lost: the bytes stay beside it.
                let copies = unreadable_copies(file.parent().unwrap(), "library.json");
                assert_eq!(copies.len(), 1, "{} {kind}: {copies:?}", case.path);
                assert_eq!(fs::read(&copies[0]).unwrap(), bytes, "{kind}");
            }
        }
    }
}

#[test]
fn a_bad_notice_record_is_kept_for_support_and_the_screens_start_over() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("appdata/entitlement/notices.json");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"{\"welcomeShown\": tr").unwrap();
    let core = start(dir.path(), false);
    works(&core, false);
    quit(core);
    let bad = file.with_file_name("notices.json.bad");
    assert_eq!(fs::read(&bad).unwrap(), b"{\"welcomeShown\": tr");
}

#[test]
fn a_bad_recovery_marker_does_not_stop_a_project_from_opening() {
    for (kind, bytes) in corruptions() {
        let dir = tempfile::tempdir().unwrap();
        let core = start(dir.path(), false);
        let actor = core.local_actor();
        core.dispatch(
            &actor,
            "project.create",
            json!({ "title": "Film", "projectType": "Feature Film" }),
        )
        .unwrap();
        let root = core.project().unwrap().layout.root().to_path_buf();
        core.dispatch(&actor, "project.close", json!({})).unwrap();
        quit(core);

        let marker = root.join("recovery").join("session.json");
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        fs::write(&marker, &bytes).unwrap();

        let core = start(dir.path(), false);
        let actor = core.local_actor();
        core.dispatch(&actor, "project.open", json!({ "path": root }))
            .unwrap_or_else(|e| panic!("{kind}: the project did not open: {}", e.code_str()));
        works(&core, false);
        core.dispatch(&actor, "project.close", json!({})).unwrap();
        quit(core);
    }
}
