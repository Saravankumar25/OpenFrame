//! Test-only helpers: an in-memory project with the real migrations and a screenplay inserted as
//! rows, for the query understanding and briefing tests. No model, no files.

use openframe_domain::{new_id, now_ms};
use rusqlite::{Connection, params};

use super::queries::{self, DraftRef};

/// One scene to insert: heading, synopsis, writer's note, omitted flag and its elements.
pub(super) struct TestScene {
    pub heading: String,
    pub synopsis: Option<String>,
    pub notes: Option<String>,
    pub omitted: bool,
    pub elements: Vec<(String, String)>,
}

/// A scene from a heading and (element type, text) pairs.
pub(super) fn scene(heading: &str, elements: &[(&str, &str)]) -> TestScene {
    TestScene {
        heading: heading.to_string(),
        synopsis: None,
        notes: None,
        omitted: false,
        elements: elements
            .iter()
            .map(|(t, x)| (t.to_string(), x.to_string()))
            .collect(),
    }
}

/// A scene with a dialogue exchange written as ("WHO", "line") pairs.
pub(super) fn talk(heading: &str, lines: &[(&str, &str)]) -> TestScene {
    let mut elements = Vec::new();
    for (who, text) in lines {
        elements.push(("character".to_string(), who.to_string()));
        elements.push(("dialogue".to_string(), text.to_string()));
    }
    TestScene {
        elements,
        ..scene(heading, &[])
    }
}

/// A project database with every real migration applied.
pub(super) fn project() -> Connection {
    let mut c = openframe_persistence::open_in_memory().unwrap();
    openframe_persistence::migrate::apply(&mut c, crate::schema::PROJECT_MIGRATIONS).unwrap();
    c
}

/// Insert a screenplay with one current draft and return the draft reference.
pub(super) fn add(c: &Connection, title: &str, scenes: &[TestScene]) -> DraftRef {
    let (sp, draft, now) = (new_id(), new_id(), now_ms());
    c.execute(
        "INSERT INTO screenplay(id, title, current_draft_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)",
        params![sp, title, draft, now],
    )
    .unwrap();
    c.execute(
        "INSERT INTO screenplay_draft(id, screenplay_id, name, created_at, updated_at)
         VALUES (?1, ?2, 'Draft 1', ?3, ?3)",
        params![draft, sp, now],
    )
    .unwrap();
    c.execute_batch("BEGIN").unwrap();
    for (i, s) in scenes.iter().enumerate() {
        let sid = new_id();
        c.execute(
            "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, synopsis, notes,
                                          omitted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                sid,
                draft,
                new_id(),
                i as i64 + 1,
                s.heading,
                s.synopsis,
                s.notes,
                s.omitted,
                now
            ],
        )
        .unwrap();
        for (j, (t, text)) in s.elements.iter().enumerate() {
            c.execute(
                "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![new_id(), sid, j as i64 + 1, t, text, now],
            )
            .unwrap();
        }
    }
    c.execute_batch("COMMIT").unwrap();
    queries::draft_by_id(c, &draft).unwrap().unwrap()
}

/// One of the sample scripts that ship with the app, imported with the importer's own
/// conversion into stored scenes and elements.
pub(super) fn sample(c: &Connection, id: &str) -> DraftRef {
    let s = openframe_import_export::samples::find(id).expect("sample exists");
    let mut outcome = openframe_import_export::import_bytes(s.bytes, s.format).unwrap();
    // The importer moves "FADE IN:" into scene 1 before the screenplay is stored.
    crate::modules::interchange::draft::normalize_for_project(&mut outcome.doc);
    let content =
        crate::modules::interchange::draft::to_new_scenes(&outcome.doc, &mut |_| Ok(None)).unwrap();
    let scenes: Vec<TestScene> = content
        .scenes
        .iter()
        .map(|n| TestScene {
            heading: n.heading.clone(),
            synopsis: n.synopsis.clone(),
            notes: n.notes.clone(),
            omitted: false,
            elements: n
                .elements
                .iter()
                .map(|e| (e.element_type.as_str().to_string(), e.text.clone()))
                .collect(),
        })
        .collect();
    add(c, s.title, &scenes)
}

/// The id of the n-th scene (1-based) of a draft.
pub(super) fn scene_id(c: &Connection, draft: &DraftRef, n: usize) -> String {
    queries::scenes(c, &draft.id).unwrap()[n - 1].id.clone()
}
