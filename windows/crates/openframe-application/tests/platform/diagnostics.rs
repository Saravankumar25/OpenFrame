//! Support diagnostics bundle (build plan item export diagnostics, feature CORE-015, ADR-0021) end-to-end
//! through the operation registry: the bundle is user-initiated and written only to the
//! location the user picked, every file is inspectable before saving (preview == bundle),
//! nothing of the project's content, paths, contacts, prompts or keys reaches any file even
//! when the local log itself contains them, destinations follow the export rules, read-only
//! roles are refused, a failed write leaves no partial file, and an export is recorded in
//! Activity without becoming an undo step.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use openframe_application::registry::{self, AiExposure, FsEffect, OpClass};
use openframe_domain::{Capability, Role};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const MARKERS: [&str; 12] = [
    "OFMARKERFOLDER7731",
    "OFMARKERTITLE7731",
    "OFMARKERSCENE7731",
    "OFMARKERSCREEN7731",
    "OFMARKERNOTE7731",
    "OFMARKERPERSON7731",
    "ofmarker.contact7731@example.com",
    "OFMARKERPROMPT7731",
    "OFMARKERKEY7731",
    // Area 13 contact fields: a cast phone number, a location emergency contact and an
    // availability note on a crew member.
    "+91 98450 77310",
    "OFMARKEREMERGENCY7731",
    "OFMARKERAVAIL7731",
];

// ------------------------------------------------------------------ fixtures

fn out_dir(env: &TestEnv) -> PathBuf {
    let d = env.dir.path().join("support-out");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn dest(env: &TestEnv, name: &str) -> String {
    out_dir(env).join(name).to_string_lossy().into_owned()
}

fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Every entry of the written ZIP as (name, text).
fn zip_entries(path: &str) -> Vec<(String, String)> {
    let f = std::fs::File::open(path).unwrap();
    let mut z = zip::ZipArchive::new(f).unwrap();
    let mut out = Vec::new();
    for i in 0..z.len() {
        let mut e = z.by_index(i).unwrap();
        let mut s = String::new();
        e.read_to_string(&mut s).unwrap();
        out.push((e.name().unwrap().to_string(), s));
    }
    out
}

/// A project in a folder named after a marker, with marker content in the screenplay,
/// a note, a contact, an assistant request and a key-like value.
fn seeded_env() -> TestEnv {
    let env = TestEnv::new();
    let parent = env.dir.path().join("Films").join(MARKERS[0]);
    std::fs::create_dir_all(&parent).unwrap();
    env.ok(
        "project.create",
        json!({ "title": MARKERS[1], "projectType": "Feature Film",
                "parentDir": parent.to_string_lossy() }),
    );
    let draft = env.ok("screenplay.create", json!({}))["draft"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    let scene = doc["scenes"][0]["id"].as_str().unwrap().to_string();
    let element = doc["scenes"][0]["elements"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    env.ok(
        "screenplay.apply_edits",
        json!({ "draftId": draft, "ops": [
            { "op": "updateScene", "id": scene, "heading": format!("INT. {} - NIGHT", MARKERS[2]) },
            { "op": "updateElement", "id": element, "text": format!("{} api_key={}", MARKERS[3], MARKERS[8]) },
        ] }),
    );
    env.ok(
        "notes.create",
        json!({ "title": MARKERS[4], "body": format!("{} token={}", MARKERS[4], MARKERS[8]) }),
    );
    let crew = env.ok(
        "crew.create",
        json!({ "personName": MARKERS[5], "role": "Sound Recordist", "department": "Sound" }),
    );
    env.ok(
        "crew.update",
        json!({ "id": crew["id"], "contact": MARKERS[6] }),
    );
    env.ok(
        "cast.create",
        json!({ "personName": MARKERS[5], "characterName": "ARJUN", "contact": MARKERS[9] }),
    );
    let location = env.ok(
        "locations.create",
        json!({ "name": "Old Railway Station", "nearestHospital": "District Hospital",
                "emergencyContact": MARKERS[10] }),
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "crew_member", "ownerId": crew["id"], "windows": [
            { "startDate": "2027-06-09", "endDate": "2027-06-20", "kind": "Unavailable", "note": MARKERS[11] },
        ] }),
    );
    env.ok(
        "availability.set",
        json!({ "ownerType": "location", "ownerId": location["id"], "windows": [
            { "startDate": "2027-06-01", "endDate": "2027-06-01", "kind": "Available" },
        ] }),
    );
    // No model is installed in tests: the request fails, but it carries the prompt marker.
    let _ = env.call(
        "ai.ask",
        json!({ "text": format!("{} {}", MARKERS[7], MARKERS[3]), "scope": { "kind": "WholeProject" } }),
    );
    write_logs(&env);
    env
}

/// A local log as the desktop app writes it (JSON lines), with realistic entries and,
/// to prove the bundle never trusts the log, entries that leak every marker and path.
fn write_logs(env: &TestEnv) {
    let dir = env.core.config.app_data_dir.join("logs");
    std::fs::create_dir_all(&dir).unwrap();
    let root = env.project_path();
    let leak = format!(
        "{} {} {} {} {} {} {} {} {root}",
        MARKERS[3],
        MARKERS[4],
        MARKERS[6],
        MARKERS[7],
        MARKERS[8],
        MARKERS[9],
        MARKERS[10],
        MARKERS[11]
    );
    let line = |level: &str, target: &str, fields: Value| {
        json!({ "timestamp": "2026-10-05T09:15:02.123456Z", "level": level, "fields": fields, "target": target })
            .to_string()
    };
    let lines = [
        line(
            "INFO",
            "openframe_desktop_lib",
            json!({ "message": "OpenFrame Studio starting", "version": "0.1.0", "build": "openframe-distribution-channel:direct", "channel": "direct", "packaged": false, "in_app_updates": true }),
        ),
        line(
            "INFO",
            "openframe_desktop_lib::distribution",
            json!({ "message": "WebView2 runtime available", "webview2": "154.0.4258.37" }),
        ),
        line(
            "INFO",
            "openframe_desktop_lib",
            json!({ "message": "operation failed", "op": "notes.create", "code": "validation.invalid_input", "ms": 3 }),
        ),
        line(
            "INFO",
            "openframe_desktop_lib",
            json!({ "message": "slow operation", "op": "screenplay.apply_edits", "ms": 512 }),
        ),
        // An operation name the webview made up (it can send any string).
        line(
            "INFO",
            "openframe_desktop_lib",
            json!({ "message": "operation failed", "op": MARKERS[3], "code": "security.unknown_operation", "ms": 1 }),
        ),
        // Content in free-text fields, a non-identifier key, nested values and odd codes.
        line(
            "WARN",
            "openframe_application::store",
            json!({ "message": "could not remove purged file; it was left in place",
                    "detail": leak, "path": root, "error": format!("token={}", MARKERS[8]),
                    "OFMARKERKEY7731": "x", "nested": { "a": MARKERS[2] },
                    "kind": MARKERS[1], "code": MARKERS[6], "action": MARKERS[5] }),
        ),
        // A message OpenFrame does not know (would be a bug in a log call site).
        line(
            "ERROR",
            "openframe_application",
            json!({ "message": format!("{} at {root}", MARKERS[3]), "code": "storage.disk_full" }),
        ),
        // A third-party crate quoting a request.
        line(
            "INFO",
            "reqwest::connect",
            json!({ "message": format!("POST http://127.0.0.1:9/v1 prompt={}", MARKERS[7]) }),
        ),
        // The panic hook: location plus a message that quotes content.
        line(
            "ERROR",
            "openframe_desktop_lib",
            json!({ "message": format!("panic at src/modules/story.rs:12: {} {}", MARKERS[3], root) }),
        ),
        // Not JSON, too long, and a bad level: skipped, never parsed into the bundle.
        format!("not json {leak}"),
        format!(
            "{{\"level\":\"INFO\",\"fields\":{{\"message\":\"{}\"}}}}",
            "x".repeat(20_000) + MARKERS[4]
        ),
        line(
            "NOTICE",
            "openframe_application",
            json!({ "message": leak }),
        ),
    ];
    std::fs::write(
        dir.join("openframe.2026-10-05.log"),
        lines.join("\n") + "\n",
    )
    .unwrap();
    // An older day and an unrelated file in the logs folder.
    std::fs::write(
        dir.join("openframe.2026-10-04.log"),
        line(
            "INFO",
            "openframe_desktop_lib",
            json!({ "message": "slow operation", "op": "schedule.get", "ms": 300 }),
        ) + "\n",
    )
    .unwrap();
    std::fs::write(dir.join("notes.txt"), leak).unwrap();
}

fn assert_clean(env: &TestEnv, what: &str, text: &str) {
    let lower = text.to_lowercase();
    for m in MARKERS {
        assert!(
            !lower.contains(&m.to_lowercase()),
            "{what} contains {m}:\n{text}"
        );
    }
    let mut paths = vec![
        env.dir.path().to_string_lossy().into_owned(),
        env.project_path(),
    ];
    if let Ok(home) = std::env::var("USERPROFILE") {
        paths.push(home);
    }
    for p in paths {
        for form in [p.clone(), p.replace('\\', "\\\\"), p.replace('\\', "/")] {
            assert!(
                !lower.contains(&form.to_lowercase()),
                "{what} contains the path {form}"
            );
        }
    }
}

// ------------------------------------------------------------------ privacy

#[test]
fn core_015_bundle_contains_no_content_paths_contacts_prompts_or_keys() {
    let env = seeded_env();
    let preview = env.ok("diagnostics.preview", json!({}));
    let files = preview["files"].as_array().unwrap();
    for f in files {
        assert_clean(
            &env,
            f["name"].as_str().unwrap(),
            f["content"].as_str().unwrap(),
        );
    }
    let r = env.ok(
        "diagnostics.export_bundle",
        json!({ "path": dest(&env, "support.zip") }),
    );
    let path = r["path"].as_str().unwrap().to_string();
    let entries = zip_entries(&path);
    assert_eq!(entries.len(), 6);
    let all: String = entries
        .iter()
        .map(|(n, s)| format!("== {n}\n{s}"))
        .collect();
    for (name, text) in &entries {
        assert_clean(&env, name, text);
    }
    // The reply to the user's own window names where the bundle was saved (the path the user
    // chose, used above to open it). That is not part of the bundle; everything else must be
    // clean.
    let mut reply = r.clone();
    reply.as_object_mut().unwrap().remove("path");
    assert_clean(&env, "result", &reply.to_string());

    // Positive controls: the support facts are there.
    let get = |n: &str| {
        entries
            .iter()
            .find(|(name, _)| name == n)
            .unwrap()
            .1
            .clone()
    };
    let summary: Value = serde_json::from_str(&get("summary.json")).unwrap();
    assert_eq!(summary["app"]["version"], "0.1.0-test");
    assert_eq!(summary["distribution"]["channel"], "direct");
    assert_eq!(summary["distribution"]["packaged"], false);
    assert_eq!(summary["webView2Version"], "154.0.4258.37");
    assert_eq!(summary["system"]["architecture"], std::env::consts::ARCH);
    assert!(summary["system"]["osVersion"].as_str().unwrap().len() > 1);
    let schema = &summary["schemaVersions"];
    assert_eq!(schema["project"]["open"], true);
    assert!(schema["project"]["current"].as_u64().unwrap() >= 1);
    assert_eq!(
        schema["project"]["current"], schema["project"]["supported"],
        "a new project is at the latest schema"
    );
    assert!(schema["appDatabase"]["current"].as_u64().unwrap() >= 1);
    assert_eq!(
        summary["logs"]["filesRead"], 2,
        "the unrelated file is not read"
    );

    let errors: Value = serde_json::from_str(&get("errors.json")).unwrap();
    let codes: Vec<&str> = errors["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"validation.invalid_input"), "{codes:?}");
    assert!(codes.contains(&"storage.disk_full"), "{codes:?}");
    let timings = get("timings.json");
    assert!(timings.contains("screenplay.apply_edits") && timings.contains("512"));
    assert!(timings.contains("schedule.get"), "older log day is read");
    let tail = get("log-tail.txt");
    assert!(
        tail.contains("op=notes.create code=validation.invalid_input ms=3"),
        "{tail}"
    );
    assert!(tail.contains("panic at src/modules/story.rs:12 (message omitted)"));
    assert!(tail.contains("[message omitted]"));
    assert!(tail.contains("op=[unknown operation]") || tail.contains("op=[omitted]"));
    assert!(tail.contains("detail=[omitted]"));
    assert!(get("ai-status.json").contains("\"status\""));
    assert!(!all.contains("store_dir") && !all.contains("storeDir") && !all.contains("logFile"));
}

#[test]
fn core_015_preview_is_exactly_what_is_saved_and_the_manifest_lists_every_file() {
    let env = seeded_env();
    let preview = env.ok("diagnostics.preview", json!({}));
    let names: Vec<&str> = preview["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "manifest.json",
            "summary.json",
            "errors.json",
            "timings.json",
            "log-tail.txt",
            "ai-status.json"
        ]
    );
    for f in preview["files"].as_array().unwrap() {
        assert!(f["description"].as_str().unwrap().len() > 20);
        assert_eq!(
            f["bytes"].as_u64().unwrap(),
            f["content"].as_str().unwrap().len() as u64
        );
    }
    let suggested = preview["suggestedFileName"].as_str().unwrap();
    assert!(suggested.starts_with("OpenFrame Diagnostics ") && suggested.ends_with(".zip"));
    assert!(preview["neverIncluded"].as_array().unwrap().len() >= 5);

    let r = env.ok(
        "diagnostics.export_bundle",
        json!({ "path": dest(&env, "bundle") }),
    );
    assert!(
        r["fileName"].as_str().unwrap().ends_with(".zip"),
        ".zip is added"
    );
    let entries = zip_entries(r["path"].as_str().unwrap());
    let zip_names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(zip_names, names);
    assert_eq!(r["files"].as_array().unwrap().len(), 6);
    let manifest: Value = serde_json::from_str(&entries[0].1).unwrap();
    assert_eq!(manifest["sentAutomatically"], false);
    let listed: Vec<&str> = manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(listed, names, "the manifest lists every file");
    for (name, text) in entries.iter().skip(1) {
        let m = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name.as_str())
            .unwrap();
        assert_eq!(
            m["sha256"].as_str().unwrap(),
            openframe_security::sha256_bytes(text.as_bytes()),
            "{name} checksum"
        );
        assert_eq!(m["bytes"].as_u64().unwrap(), text.len() as u64);
    }
}

#[test]
fn core_015_works_without_an_open_project_and_without_logs() {
    let env = TestEnv::new();
    let r = env.ok(
        "diagnostics.export_bundle",
        json!({ "path": dest(&env, "no-project.zip") }),
    );
    let entries = zip_entries(r["path"].as_str().unwrap());
    let summary: Value = serde_json::from_str(&entries[1].1).unwrap();
    assert_eq!(summary["schemaVersions"]["project"]["open"], false);
    assert_eq!(summary["distribution"]["channel"], "not recorded");
    assert_eq!(summary["logs"]["filesRead"], 0);
}

// ------------------------------------------------------------------ destination

#[test]
fn core_015_destination_must_be_a_real_location_outside_openframe_folders() {
    let env = seeded_env();
    let code = |path: String| env.err("diagnostics.export_bundle", json!({ "path": path }));
    assert_eq!(code("relative.zip".into()), "validation.invalid_input");
    assert_eq!(code(String::new()), "validation.invalid_input");
    let missing = env.dir.path().join("no-such-folder").join("d.zip");
    assert_eq!(
        code(missing.to_string_lossy().into_owned()),
        "export.folder_missing"
    );
    let inside = PathBuf::from(env.project_path()).join("d.zip");
    assert_eq!(
        code(inside.to_string_lossy().into_owned()),
        "export.inside_project"
    );
    let logs = env.core.config.app_data_dir.join("logs").join("d.zip");
    assert_eq!(
        code(logs.to_string_lossy().into_owned()),
        "export.inside_app_folder"
    );
    std::fs::create_dir_all(&env.core.config.global_vault_dir).unwrap();
    let vault = env.core.config.global_vault_dir.join("d.zip");
    assert_eq!(
        code(vault.to_string_lossy().into_owned()),
        "validation.invalid_input"
    );
    // A folder, not a file.
    std::fs::create_dir_all(out_dir(&env).join("folder.zip")).unwrap();
    assert_eq!(code(dest(&env, "folder.zip")), "validation.invalid_input");
    #[cfg(windows)]
    {
        assert_eq!(
            code(format!("{}:stream.zip", dest(&env, "ads"))),
            "security.path_rejected"
        );
        assert_eq!(code(dest(&env, "NUL.zip")), "security.path_rejected");
        assert_eq!(
            code("\\\\.\\PhysicalDrive0\\d.zip".into()),
            "security.path_rejected"
        );
    }
    assert_eq!(
        files_in(&out_dir(&env)),
        vec!["folder.zip".to_string()],
        "nothing was written for refused destinations"
    );
    assert!(!PathBuf::from(env.project_path()).join("d.zip").exists());
}

// ------------------------------------------------------------------ permissions

#[test]
fn core_015_read_only_roles_cannot_preview_or_export() {
    let env = seeded_env();
    for role in [Role::Viewer, Role::Commenter] {
        let who = env.actor_with_role(role);
        let e = env
            .call_as(&who, "diagnostics.preview", json!({}))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?} preview");
        let e = env
            .call_as(
                &who,
                "diagnostics.export_bundle",
                json!({ "path": dest(&env, &format!("{role:?}.zip")) }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?} export");
    }
    assert!(files_in(&out_dir(&env)).is_empty(), "nothing written");
    for role in [Role::ExportOnly, Role::Editor, Role::Owner] {
        let who = env.actor_with_role(role);
        env.call_as(
            &who,
            "diagnostics.export_bundle",
            json!({ "path": dest(&env, &format!("{role:?}.zip")) }),
        )
        .unwrap_or_else(|e| panic!("{role:?}: {}", e.message));
    }
}

#[test]
fn core_015_operations_are_hidden_from_the_assistant_and_declare_their_effects() {
    let reg = registry::catalog();
    let export = reg.metadata("diagnostics.export_bundle").unwrap();
    assert!(matches!(export.ai_exposure, AiExposure::Hidden(r) if r.contains("ADR-0021")));
    assert_eq!(export.filesystem_effect, FsEffect::WritesUserFile);
    assert_eq!(export.required_capability, Some(Capability::Export));
    assert_eq!(export.operation_class, OpClass::Mutate);
    assert!(!export.long_running);
    let preview = reg.metadata("diagnostics.preview").unwrap();
    assert!(preview.ai_exposure.is_hidden());
    assert_eq!(preview.filesystem_effect, FsEffect::None);
}

// ------------------------------------------------------------------ atomic write

#[cfg(windows)]
#[test]
fn core_015_a_failed_write_keeps_the_previous_file_and_leaves_no_partial_file() {
    let env = seeded_env();
    let path = dest(&env, "locked.zip");
    std::fs::write(&path, b"old").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms.clone()).unwrap();
    let e = env
        .call("diagnostics.export_bundle", json!({ "path": path }))
        .unwrap_err();
    assert!(
        e.code.0.starts_with("storage.") || e.code.0.starts_with("export."),
        "{}",
        e.code.0
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    assert_eq!(
        files_in(&out_dir(&env)),
        vec!["locked.zip".to_string()],
        "no partial file left behind"
    );
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&path, perms).unwrap();
}

#[test]
fn core_015_a_users_file_with_the_staging_name_is_never_touched() {
    // The zip writer stages through the shared atomic replace (a unique hidden name), so a
    // user's file called `support.partial` next to the destination is neither read nor removed.
    let env = seeded_env();
    let staging = out_dir(&env).join("support.partial");
    std::fs::write(&staging, b"mine").unwrap();
    env.ok(
        "diagnostics.export_bundle",
        json!({ "path": dest(&env, "support.zip") }),
    );
    assert_eq!(std::fs::read(&staging).unwrap(), b"mine");
    let mut names = files_in(&out_dir(&env));
    names.sort();
    assert_eq!(
        names,
        vec!["support.partial".to_string(), "support.zip".to_string()]
    );
}

// ------------------------------------------------------------------ activity, undo, restart

#[test]
fn core_015_export_is_recorded_in_activity_and_is_not_an_undo_step() {
    let env = seeded_env();
    let before = env.ok("history.info", json!({}));
    env.ok(
        "diagnostics.export_bundle",
        json!({ "path": dest(&env, "a.zip") }),
    );
    let after = env.ok("history.info", json!({}));
    assert_eq!(before, after, "the undo stack is unchanged");
    let has_entry = |env: &TestEnv| {
        env.ok("history.activity", json!({}))
            .as_array()
            .unwrap()
            .iter()
            .any(|a| {
                a["action"] == "diagnostics.export_bundle"
                    && a["summary"] == "Exported a diagnostics bundle for support"
            })
    };
    assert!(has_entry(&env));
    // Preview records nothing.
    let count = |env: &TestEnv| {
        env.ok("history.activity", json!({}))
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["action"] == "diagnostics.export_bundle")
            .count()
    };
    env.ok("diagnostics.preview", json!({}));
    assert_eq!(count(&env), 1);
    // The entry survives a restart and a crash.
    let root = env.project_path();
    let env = env.restart();
    env.reopen_project(&root);
    assert!(has_entry(&env));
    let env = env.crash_and_restart();
    env.reopen_project(&root);
    assert!(has_entry(&env));
    let activity = env.ok("history.activity", json!({})).to_string();
    assert!(!activity.contains("a.zip"), "Activity holds no file path");
}
