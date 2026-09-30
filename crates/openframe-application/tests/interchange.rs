//! Screenplay import/export through the application layer (FSD §19–20, §60,
//! §117, §122–123; FSD-SCRIPT-038..050; Import/Export spec §4–§5, §23, §25).

use std::path::Path;

use openframe_domain::Role;
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

const SAMPLE_FOUNTAIN: &[u8] =
    include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/sample.fountain");
const SAMPLE_FDX: &[u8] =
    include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/sample.fdx");
const MALFORMED_FDX: &[u8] =
    include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/malformed.fdx");
const PASTED: &str =
    include_str!("../../openframe-import-export/tests/fixtures/screenplay/pasted.txt");
const PROSE: &[u8] =
    include_bytes!("../../openframe-import-export/tests/fixtures/screenplay/not_a_screenplay.txt");

fn count(env: &TestEnv, sql: &str) -> i64 {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

fn s(v: &Value) -> String {
    v.as_str()
        .unwrap_or_else(|| panic!("expected string, got {v}"))
        .to_string()
}

fn project_rows(env: &TestEnv) -> i64 {
    count(
        env,
        "SELECT (SELECT count(*) FROM screenplay) + (SELECT count(*) FROM screenplay_draft) + (SELECT count(*) FROM screenplay_scene) + (SELECT count(*) FROM screenplay_element) + (SELECT count(*) FROM project_file)",
    )
}

fn import_new(env: &TestEnv, name: &str, bytes: &[u8]) -> Value {
    let path = env.write_file(name, bytes);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    )
}

/// Headings of the numbered scenes of a draft.
fn headings(env: &TestEnv, draft_id: &str) -> Vec<String> {
    env.ok("interchange.draft_scenes", json!({ "draftId": draft_id }))
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| !r["number"].is_null())
        .map(|r| s(&r["heading"]))
        .collect()
}

// ------------------------------------------------------------------ import

#[test]
fn fsd_script_040_fountain_preview_shows_what_would_be_created_and_writes_nothing() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let before = project_rows(&env);
    let path = env.write_file("blackrain_v3.fountain", SAMPLE_FOUNTAIN);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    assert_eq!(p["sourceName"], "blackrain_v3.fountain");
    assert_eq!(p["sourceFormat"], "fountain");
    assert_eq!(p["title"], "Black Rain");
    assert_eq!(p["author"], "Nisha Verma");
    assert_eq!(p["sceneCount"], 4);
    assert!(
        p["characters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "MEERA")
    );
    assert!(p["approxPages"].as_u64().unwrap() >= 1);
    assert_eq!(p["confidence"], "high");
    assert_eq!(
        p["defaultMode"], "new_screenplay",
        "no screenplay in the project yet"
    );
    assert_eq!(project_rows(&env), before, "preview never writes");
}

#[test]
fn fsd_script_038_fdx_import_creates_new_screenplay_with_new_identities() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let r = import_new(&env, "sample.fdx", SAMPLE_FDX);
    assert_eq!(r["mode"], "new_screenplay");
    assert_eq!(r["sourceFormat"], "fdx");
    let draft = s(&r["draftId"]);
    let scenes = env.ok("interchange.draft_scenes", json!({ "draftId": draft }));
    let rows = scenes.as_array().unwrap();
    assert_eq!(
        rows.iter().filter(|r| !r["number"].is_null()).count() as u64,
        r["importedScenes"].as_u64().unwrap()
    );
    // Display numbers are derived from order (FSD-SCRIPT-050).
    for (i, row) in rows.iter().filter(|r| !r["number"].is_null()).enumerate() {
        assert_eq!(row["number"], (i + 1) as u64);
    }
    // Every scene gets its own lineage identity; headings live on the scene row
    // only (the Screenplay module never stores `scene_heading` elements).
    assert_eq!(
        count(
            &env,
            "SELECT count(DISTINCT lineage_id) FROM screenplay_scene"
        ),
        rows.len() as i64
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE element_type = 'scene_heading'"
        ),
        0
    );
    // The Screenplay workspace can open the imported draft.
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    assert_eq!(doc["scenes"].as_array().unwrap().len(), rows.len());
    let src = env.ok("interchange.sources", json!({}));
    let sp = &src["screenplays"][0];
    assert_eq!(sp["currentDraftId"], draft.as_str());
    assert_eq!(sp["drafts"].as_array().unwrap().len(), 1);
}

#[test]
fn fsd_script_044_import_into_existing_screenplay_never_replaces_current_draft() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let first = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let sp = s(&first["screenplayId"]);
    let current = s(&first["draftId"]);
    let original_headings = headings(&env, &current);

    let path = env.write_file("blackrain.fountain", SAMPLE_FOUNTAIN);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    assert_eq!(p["defaultMode"], "new_draft");
    assert_eq!(p["existingScreenplays"][0]["id"], sp.as_str());
    let second = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_draft", "screenplayId": sp }),
    );
    assert_ne!(second["draftId"], first["draftId"]);
    assert_eq!(
        second["draftName"], "blackrain (imported) 2",
        "names never collide"
    );
    assert!(s(&second["draftLabel"]).starts_with("Draft 2"));

    let src = env.ok("interchange.sources", json!({}));
    assert_eq!(src["screenplays"].as_array().unwrap().len(), 1);
    assert_eq!(
        src["screenplays"][0]["currentDraftId"],
        current.as_str(),
        "current draft unchanged"
    );
    assert_eq!(headings(&env, &current), original_headings);
    // A new copy: scene identities differ between the drafts.
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM (SELECT lineage_id FROM screenplay_scene GROUP BY lineage_id HAVING count(*) > 1)"
        ),
        0
    );

    // A project has one screenplay: a second "New Screenplay" is refused, nothing is written.
    let p = env.ok("screenplay.import_preview", json!({ "pastedText": PASTED }));
    assert_eq!(p["canCreateScreenplay"], false);
    let rows = project_rows(&env);
    assert_eq!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_screenplay" })
        ),
        "validation.mode"
    );
    assert_eq!(project_rows(&env), rows);
    // Without an explicit screenplay the single one is used; unknown modes are refused.
    assert_eq!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "replace_current" })
        ),
        "validation.invalid_input"
    );
    // The failed apply kept the preview available.
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_draft" }),
    );
}

#[test]
fn fsd_script_043_pasted_text_uses_txt_heuristics_with_honest_warnings() {
    let env = TestEnv::with_project("Film", "Short Film");
    let p = env.ok("screenplay.import_preview", json!({ "pastedText": PASTED }));
    assert_eq!(p["sourceFormat"], "pasted");
    assert_eq!(p["sceneCount"], 3);
    assert!(
        p["needsReview"].as_bool().unwrap(),
        "STREET — DAY has no INT./EXT."
    );
    let uncertain: Vec<&Value> = p["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["status"] == "uncertain_heading")
        .collect();
    assert_eq!(uncertain.len(), 1);
    assert!(
        p["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "uncertain_heading" && w["level"] == "attention")
    );
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    assert_eq!(r["screenplayTitle"], "Pasted screenplay");
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay WHERE format = 'Short'"
        ),
        1
    );
    assert!(r["warningsNeedingAttention"].as_u64().unwrap() >= 1);
}

#[test]
fn keep_source_file_adds_it_to_project_files_in_the_same_step() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let path = env.write_file("blackrain.fountain", SAMPLE_FOUNTAIN);
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay", "keepSourceFile": true }),
    );
    assert!(r["keptFileId"].is_string());
    let files = env.ok("files.list", json!({}));
    assert!(
        files["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == r["keptFileId"])
    );
    // One undo step removes the screenplay and the kept file together.
    env.undo();
    assert_eq!(project_rows(&env), 0);
    env.redo();
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM project_file WHERE deleted_at IS NULL"
        ),
        1
    );
    assert_eq!(count(&env, "SELECT count(*) FROM screenplay"), 1);
}

#[test]
fn import_is_one_undoable_step_and_survives_restart() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let r = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let elements = count(&env, "SELECT count(*) FROM screenplay_element");
    assert!(elements > 10);
    env.undo();
    assert_eq!(project_rows(&env), 0);
    env.redo();
    assert_eq!(
        count(&env, "SELECT count(*) FROM screenplay_element"),
        elements
    );

    // Imported scenes of the current draft are searchable like any other scene.
    let hits = env.ok("search.query", json!({ "text": "apartment morning" }));
    assert!(
        hits.as_array()
            .unwrap()
            .iter()
            .any(|h| h["entityType"] == "screenplay_scene"),
        "{hits}"
    );

    let path = env.project_path();
    let env = env.restart();
    env.reopen_project(&path);
    assert_eq!(headings(&env, &s(&r["draftId"])).len(), 4);
    assert_eq!(
        count(&env, "SELECT count(*) FROM screenplay_element"),
        elements
    );
}

#[test]
fn failed_imports_change_nothing() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let before = project_rows(&env);

    let bad = env.write_file("broken.fdx", MALFORMED_FDX);
    assert!(
        env.err("screenplay.import_preview", json!({ "path": bad }))
            .starts_with("import.")
    );
    let prose = env.write_file("minutes.txt", PROSE);
    assert_eq!(
        env.err("screenplay.import_preview", json!({ "path": prose })),
        "import.not_a_screenplay"
    );
    let pdf = env.write_file("scan.pdf", b"%PDF-1.7 this is not a real pdf");
    assert_eq!(
        env.err("screenplay.import_preview", json!({ "path": pdf })),
        "import.pdf_unreadable"
    );
    let doc = env.write_file("old.doc", &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    assert_eq!(
        env.err("screenplay.import_preview", json!({ "path": doc })),
        "import.unsupported_format"
    );
    let empty = env.write_file("empty.fountain", b"");
    assert_eq!(
        env.err("screenplay.import_preview", json!({ "path": empty })),
        "import.empty_source"
    );

    // Relative paths, both sources, neither source, bad preview ids.
    assert_eq!(
        env.err(
            "screenplay.import_preview",
            json!({ "path": "relative.fountain" })
        ),
        "validation.invalid_input"
    );
    let good = env.write_file("x.fountain", SAMPLE_FOUNTAIN);
    assert_eq!(
        env.err(
            "screenplay.import_preview",
            json!({ "path": good, "pastedText": PASTED })
        ),
        "validation.invalid_input"
    );
    assert_eq!(
        env.err("screenplay.import_preview", json!({})),
        "validation.required"
    );
    assert_eq!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": "0190a3b4-0000-7000-8000-000000000000", "mode": "new_screenplay" })
        ),
        "import.preview_expired"
    );
    // New Draft with no screenplay in the project is refused before anything is written.
    let p = env.ok("screenplay.import_preview", json!({ "path": good }));
    assert!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_draft" })
        )
        .starts_with("validation.")
    );
    assert_eq!(project_rows(&env), before);
}

#[test]
fn zip_bomb_docx_is_rejected_without_writing() {
    use openframe_security::archive::{EntrySource, write_zip};
    let env = TestEnv::with_project("Film", "Feature Film");
    let path = env.dir.path().join("inputs").join("bomb.docx");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // 64 MB of zeros deflates to a tiny entry: the compression ratio gives it away.
    let zeros = vec![0u8; 64 << 20];
    write_zip(
        &path,
        vec![
            (
                "[Content_Types].xml".to_string(),
                EntrySource::Bytes(b"<Types/>"),
            ),
            ("word/document.xml".to_string(), EntrySource::Bytes(&zeros)),
        ],
    )
    .unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() < 1 << 20);
    let code = env.err(
        "screenplay.import_preview",
        json!({ "path": path.to_string_lossy() }),
    );
    assert!(code.starts_with("import."), "{code}");
    assert_eq!(project_rows(&env), 0);
}

#[test]
fn only_roles_that_may_import_or_export_can() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let r = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let path = env.write_file("again.fountain", SAMPLE_FOUNTAIN);
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("x.pdf").to_string_lossy().into_owned();
    for role in [Role::Viewer, Role::Commenter, Role::ExportOnly] {
        let a = env.actor_with_role(role);
        let e = env
            .call_as(&a, "screenplay.import_preview", json!({ "path": path }))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
    }
    for role in [Role::Viewer, Role::Commenter] {
        let a = env.actor_with_role(role);
        let e = env
            .call_as(
                &a,
                "screenplay.export",
                json!({ "draftId": r["draftId"], "format": "pdf", "path": dest }),
            )
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied", "{role:?}");
    }
    let a = env.actor_with_role(Role::ExportOnly);
    env.call_as(
        &a,
        "screenplay.export",
        json!({ "draftId": r["draftId"], "format": "pdf", "path": dest }),
    )
    .unwrap();
    assert!(Path::new(&dest).is_file());
}

// ------------------------------------------------------------------ export

#[test]
fn fsd_script_045_to_048_every_format_exports_and_reimports() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let r = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let draft = s(&r["draftId"]);
    let original = headings(&env, &draft);
    let before = project_rows(&env);
    let out = tempfile::tempdir().unwrap();

    for fmt in ["pdf", "fdx", "fountain", "docx", "txt"] {
        let dest = out.path().join(format!("Black Rain.{fmt}"));
        let res = env.ok(
            "screenplay.export",
            json!({ "draftId": draft, "format": fmt, "path": dest.to_string_lossy() }),
        );
        assert_eq!(res["format"], fmt);
        assert!(dest.is_file(), "{fmt}");
        assert_eq!(
            res["bytesWritten"].as_u64().unwrap(),
            std::fs::metadata(&dest).unwrap().len()
        );
        assert_eq!(res["privateNotesExcluded"], true);
        assert!(s(&res["summary"]).contains("your project does not change"));
        if fmt == "pdf" {
            assert!(res["pages"].as_u64().unwrap() >= 1);
        }
        // Re-import the export into a new draft of the same screenplay.
        let p = env.ok(
            "screenplay.import_preview",
            json!({ "path": dest.to_string_lossy() }),
        );
        assert_eq!(
            p["sceneCount"].as_u64().unwrap(),
            original.len() as u64,
            "{fmt} keeps every scene"
        );
        let back = env.ok(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_draft" }),
        );
        assert_eq!(
            headings(&env, &s(&back["draftId"])),
            original,
            "{fmt} keeps scene headings"
        );
    }
    // Exports are snapshots: exporting writes nothing to the project and adds no undo step.
    let rows = project_rows(&env);
    assert!(rows > before);
    let undo_steps = count(&env, "SELECT count(*) FROM sys_undo");
    let d = out.path().join("snap.fdx");
    env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "fdx", "path": d.to_string_lossy() }),
    );
    assert_eq!(project_rows(&env), rows);
    assert_eq!(count(&env, "SELECT count(*) FROM sys_undo"), undo_steps);
}

#[test]
fn fsd_script_049_050_title_page_and_derived_scene_numbers_in_pdf() {
    let env = TestEnv::with_project("Black Rain", "Feature Film");
    let r = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let draft = s(&r["draftId"]);
    let scenes = env.ok("interchange.draft_scenes", json!({ "draftId": draft }));
    // "FADE IN:" before the first heading opens scene 1, so numbers match the source.
    assert_eq!(scenes[0]["number"], 1);
    assert_eq!(scenes[0]["heading"], "EXT. BUS STOP — NIGHT");
    assert!(
        r["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "opening_material_moved")
    );
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    assert_eq!(doc["scenes"][0]["elements"][0]["text"], "FADE IN:");
    let third = s(&scenes[2]["id"]);
    assert_eq!(scenes[2]["number"], 3);

    // Print preview uses the same layout as the PDF.
    let pv = env.ok(
        "interchange.export_preview",
        json!({ "draftId": draft, "options": { "sceneNumbers": true } }),
    );
    assert_eq!(pv["pages"][0]["titlePage"], true);
    let title_text: Vec<String> = pv["pages"][0]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| s(&l["text"]))
        .collect();
    assert!(
        title_text
            .iter()
            .any(|t| t == "BLACK RAIN" || t == "Black Rain"),
        "{title_text:?}"
    );
    assert!(title_text.iter().any(|t| t.contains("Nisha Verma")));
    assert_eq!(pv["pageWidthIn"], 8.5);
    assert_eq!(pv["pageHeightIn"], 11.0);

    // Selected scenes keep their full-draft display numbers.
    let pv = env.ok(
        "interchange.export_preview",
        json!({ "draftId": draft, "options": { "sceneNumbers": true, "titlePage": false, "sceneIds": [third] } }),
    );
    let numbers: Vec<String> = pv["pages"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| p["lines"].as_array().unwrap().iter())
        .filter(|l| l["style"] == "scene_number")
        .map(|l| s(&l["text"]))
        .collect();
    assert!(
        !numbers.is_empty() && numbers.iter().all(|n| n.trim_end_matches('.') == "3"),
        "{numbers:?}"
    );

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("scene3.pdf");
    let res = env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "pdf", "path": dest.to_string_lossy(), "options": { "sceneIds": [third], "sceneNumbers": true } }),
    );
    assert_eq!(res["sceneCount"], 1);
    assert!(s(&res["summary"]).contains("1 selected scene"));
    // Numbers are display only: nothing about scene identity changed.
    let after = env.ok("interchange.draft_scenes", json!({ "draftId": draft }));
    assert_eq!(
        s(&after
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["number"] == 3)
            .unwrap()["id"]),
        third
    );
}

#[test]
fn export_destination_is_validated_and_failures_leave_no_file() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let r = import_new(&env, "blackrain.fountain", SAMPLE_FOUNTAIN);
    let draft = s(&r["draftId"]);
    let out = tempfile::tempdir().unwrap();

    // Extension is added when missing.
    let res = env.ok("screenplay.export", json!({ "draftId": draft, "format": "fdx", "path": out.path().join("noext").to_string_lossy() }));
    assert!(s(&res["path"]).ends_with("noext.fdx"));
    // Never inside the project's own folder.
    let inside = Path::new(&env.project_path()).join("inside.pdf");
    assert_eq!(
        env.err(
            "screenplay.export",
            json!({ "draftId": draft, "format": "pdf", "path": inside.to_string_lossy() })
        ),
        "export.inside_project"
    );
    // Missing folder, relative path, unknown format, unknown draft.
    let missing = out.path().join("nope").join("x.pdf");
    assert_eq!(
        env.err(
            "screenplay.export",
            json!({ "draftId": draft, "format": "pdf", "path": missing.to_string_lossy() })
        ),
        "export.folder_missing"
    );
    assert_eq!(
        env.err(
            "screenplay.export",
            json!({ "draftId": draft, "format": "pdf", "path": "x.pdf" })
        ),
        "validation.invalid_input"
    );
    assert_eq!(env.err("screenplay.export", json!({ "draftId": draft, "format": "rtf", "path": out.path().join("x.rtf").to_string_lossy() })), "validation.invalid_input");
    assert_eq!(
        env.err("screenplay.export", json!({ "draftId": "0190a3b4-0000-7000-8000-000000000000", "format": "pdf", "path": out.path().join("x.pdf").to_string_lossy() })),
        "not_found.draft"
    );
    assert_eq!(
        env.err("screenplay.export", json!({ "draftId": draft, "format": "pdf", "path": out.path().join("x.pdf").to_string_lossy(), "options": { "sceneIds": [] } })),
        "validation.scene_ids"
    );
}

#[test]
fn pdf_export_refuses_characters_courier_cannot_print() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let p = env.ok(
        "screenplay.import_preview",
        json!({ "pastedText": "INT. घर - DAY\n\nमीरा enters.\n\nMEERA\nNamaste.\n" }),
    );
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay" }),
    );
    let draft = s(&r["draftId"]);
    let pv = env.ok("interchange.export_preview", json!({ "draftId": draft }));
    assert!(
        pv["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "pdf_unsupported_characters")
    );

    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("hindi.pdf");
    assert_eq!(
        env.err(
            "screenplay.export",
            json!({ "draftId": draft, "format": "pdf", "path": dest.to_string_lossy() })
        ),
        "export.pdf_unsupported_characters"
    );
    assert_eq!(
        std::fs::read_dir(out.path()).unwrap().count(),
        0,
        "nothing written"
    );
    // Unicode-capable formats keep the text.
    let fdx = out.path().join("hindi.fdx");
    env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "fdx", "path": fdx.to_string_lossy() }),
    );
    assert!(std::fs::read_to_string(&fdx).unwrap().contains("मीरा"));
}

#[test]
fn drafts_built_from_the_story_board_export_cleanly() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let act = s(&env.ok("story.create_act", json!({ "title": "Act One" }))["id"]);
    let seq = s(&env.ok(
        "story.create_sequence",
        json!({ "actId": act, "title": "Opening" }),
    )["id"]);
    let card = s(&env.ok(
        "story.create_card",
        json!({ "parent": { "parentType": "sequence", "parentId": seq }, "shortDescription": "Meera waits for the bus." }),
    )["id"]);
    env.ok(
        "story.update_card",
        json!({ "id": card, "sceneHeading": "EXT. BUS STOP — NIGHT" }),
    );
    let b = env.ok("story.build_screenplay", json!({ "include": [{ "cardId": card }], "destination": "newScreenplay", "descriptionMode": "actionText" }));
    let draft = s(&b["draftId"]);
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("story.fountain");
    env.ok(
        "screenplay.export",
        json!({ "draftId": draft, "format": "fountain", "path": dest.to_string_lossy() }),
    );
    let text = std::fs::read_to_string(&dest).unwrap();
    assert_eq!(
        text.matches("EXT. BUS STOP — NIGHT").count(),
        1,
        "heading printed once:\n{text}"
    );
    assert!(text.contains("Meera waits for the bus."));
}

#[test]
fn episodic_projects_import_one_screenplay_per_episode() {
    let env = TestEnv::with_project("Monsoon Diaries", "Series");
    let season = s(&env.ok("story.create_season", json!({}))["id"]);
    let ep1 = s(&env.ok(
        "story.create_episode",
        json!({ "seasonId": season, "title": "Pilot" }),
    )["id"]);
    let ep2 = s(&env.ok(
        "story.create_episode",
        json!({ "seasonId": season, "title": "The Flood" }),
    )["id"]);

    let p = env.ok("screenplay.import_preview", json!({ "pastedText": PASTED }));
    assert_eq!(p["episodic"], true);
    assert_eq!(p["canCreateScreenplay"], true);
    assert_eq!(p["episodesWithoutScreenplay"].as_array().unwrap().len(), 2);
    // The episode is required.
    assert_eq!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_screenplay" })
        ),
        "validation.episode_id"
    );
    let r = env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": "new_screenplay", "episodeId": ep1 }),
    );
    assert_eq!(
        count(
            &env,
            &format!(
                "SELECT count(*) FROM screenplay WHERE episode_id = '{ep1}' AND format = 'Episodic'"
            )
        ),
        1
    );

    let p = env.ok("screenplay.import_preview", json!({ "pastedText": PASTED }));
    let free: Vec<String> = p["episodesWithoutScreenplay"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| s(&e["id"]))
        .collect();
    assert_eq!(free, vec![ep2.clone()]);
    assert_eq!(p["existingScreenplays"][0]["episodeTitle"], "Pilot");
    assert_eq!(
        env.err(
            "screenplay.import_apply",
            json!({ "previewId": p["previewId"], "mode": "new_screenplay", "episodeId": ep1 })
        ),
        "validation.episode_id"
    );
    let d = env.ok("screenplay.import_apply", json!({ "previewId": p["previewId"], "mode": "new_draft", "screenplayId": r["screenplayId"] }));
    assert_eq!(d["screenplayId"], r["screenplayId"]);
}

#[test]
fn fdx_revision_marks_are_kept_on_import_and_exported() {
    let env = TestEnv::with_project("Film", "Feature Film");
    let fdx = r##"<?xml version="1.0" encoding="UTF-8"?>
<FinalDraft DocumentType="Script" Template="No" Version="5">
  <Content>
    <Paragraph Type="Scene Heading"><Text>INT. KITCHEN - NIGHT</Text></Paragraph>
    <Paragraph Type="Action"><Text RevisionID="1">Meera burns the toast.</Text></Paragraph>
    <Paragraph Type="Action"><Text>Smoke fills the room.</Text></Paragraph>
  </Content>
  <Revisions ActiveSet="1"><Revision ID="1" Mark="*" Name="Blue" Color="#0000FF"/></Revisions>
</FinalDraft>"##;
    let r = import_new(&env, "kitchen.fdx", fdx.as_bytes());
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE revision_mark = 'Blue' AND text = 'Meera burns the toast.'"
        ),
        1
    );
    assert_eq!(
        count(
            &env,
            "SELECT count(*) FROM screenplay_element WHERE revision_mark IS NOT NULL"
        ),
        1
    );
    let out = tempfile::tempdir().unwrap();
    let dest = out.path().join("kitchen.fdx");
    env.ok(
        "screenplay.export",
        json!({ "draftId": r["draftId"], "format": "fdx", "path": dest.to_string_lossy(), "options": { "revisionMarks": true } }),
    );
    let text = std::fs::read_to_string(&dest).unwrap();
    assert!(
        text.contains(r#"RevisionID="1">Meera burns the toast."#),
        "{text}"
    );
    assert!(!text.contains(r#"RevisionID="1">Smoke"#), "{text}");
}
