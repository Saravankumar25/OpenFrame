//! Local intelligence index + hybrid retrieval (AI/RAG spec §8–§22, §31–§33,
//! §42; contract C2). Driven through the real mutation pipeline and the public
//! retrieval API with a deterministic test embedder (no model download).

use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::MutationMeta;
use openframe_application::events::AppEvent;
use openframe_application::modules::ai::intelligence::{self, IndexSettings};
use openframe_application::modules::ai::retrieval::{
    self, ContextPacket, IndexState, RetrievalRoute, RetrieveRequest,
};
use openframe_application::modules::ai::scope::{self, AiScopeArgs};
use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, Capability, Role, new_id, now_ms};
use openframe_search::Embedder;
use openframe_test_support::TestEnv;
use openframe_test_support::embedder::ConceptEmbedder;
use rusqlite::{Connection, params};
use serde_json::json;

// ------------------------------------------------------------------ fixture

#[allow(dead_code)]
struct World {
    draft_old: String,
    draft: String,
    scenes: Vec<String>,
    lineages: Vec<String>,
    ravi: String,
    anjali: String,
    arjun: String,
    station: String,
    mustang: String,
    card: String,
    act: String,
    sequence: String,
    day: String,
    my_note: String,
    other_note: String,
    other_user: String,
}

fn fast_settings(env: &TestEnv) {
    intelligence::service(&env.core).set_settings(IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        embed_retry: Duration::from_millis(50),
        ..IndexSettings::default()
    });
}

fn attach(env: &TestEnv) -> Arc<ConceptEmbedder> {
    let e = Arc::new(ConceptEmbedder::new());
    intelligence::service(&env.core).set_embedder(Some(e.clone() as Arc<dyn Embedder>));
    e
}

fn idle(env: &TestEnv) {
    intelligence::activate(&env.core).expect("project open");
    assert!(
        intelligence::wait_idle(&env.core, Duration::from_secs(60)),
        "index did not settle: {:?}",
        intelligence::service(&env.core)
            .current()
            .map(|i| i.snapshot())
    );
}

/// Seed every domain through the store pipeline (rows as the modules write them).
fn seed(env: &TestEnv) -> World {
    let s = env.core.project().unwrap();
    let me = env.actor();
    let w = World {
        draft_old: new_id(),
        draft: new_id(),
        scenes: (0..4).map(|_| new_id()).collect(),
        lineages: (0..4).map(|_| new_id()).collect(),
        ravi: new_id(),
        anjali: new_id(),
        arjun: new_id(),
        station: new_id(),
        mustang: new_id(),
        card: new_id(),
        act: new_id(),
        sequence: new_id(),
        day: new_id(),
        my_note: new_id(),
        other_note: new_id(),
        other_user: new_id(),
    };
    let script: Vec<(&str, Vec<(&str, &str)>)> = vec![
        (
            "INT. RAILWAY STATION - NIGHT",
            vec![
                (
                    "action",
                    "Rain hammers the platform. A red Mustang idles outside.",
                ),
                ("character", "RAVI"),
                (
                    "dialogue",
                    "You said you were with your sister all evening.",
                ),
                ("character", "ANJALI"),
                ("dialogue", "I was."),
                (
                    "action",
                    "He studies her face. He suspects she lies to him.",
                ),
            ],
        ),
        (
            "INT. CITY HOSPITAL - DAY",
            vec![
                (
                    "action",
                    "Ravi confronts the doctor in the corridor. He shouts, furious.",
                ),
                ("character", "RAVI"),
                ("dialogue", "Tell me the truth!"),
            ],
        ),
        (
            "EXT. RIVERBANK - DAWN",
            vec![
                ("action", "Anjali sits by the water, isolated and lonely."),
                ("character", "ANJALI"),
                ("dialogue", "Nobody comes here anymore."),
            ],
        ),
        (
            "INT. POLICE STATION - NIGHT",
            vec![
                ("action", "Inspector Arjun reviews the file."),
                ("character", "ARJUN"),
                ("dialogue", "Again."),
            ],
        ),
    ];
    let other_user = w.other_user.clone();
    let me_id = me.user_id.clone();
    let wr = &w;
    s.store
        .mutate(&me, MutationMeta::new("test.seed", "Seed project", Capability::Edit), move |tx| {
            let c = tx.conn();
            let now = now_ms();
            let sp = new_id();
            c.execute(
                "UPDATE project SET genre='Drama', logline='A stationmaster''s son doubts the woman he loves.'",
                [],
            )?;
            c.execute(
                "INSERT INTO screenplay(id, title, current_draft_id, created_at, updated_at) VALUES (?1, 'Railway', ?2, ?3, ?3)",
                params![sp, wr.draft, now],
            )?;
            for (d, name) in [(&wr.draft_old, "Draft 1"), (&wr.draft, "Draft 2")] {
                c.execute(
                    "INSERT INTO screenplay_draft(id, screenplay_id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![d, sp, name, now],
                )?;
            }
            // A scene only in the old draft (draft-boundary checks).
            c.execute(
                "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, 'EXT. OLD MILL - DAY', ?4, ?4)",
                params![new_id(), wr.draft_old, new_id(), now],
            )?;
            for (i, (heading, elements)) in script.iter().enumerate() {
                c.execute(
                    "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![wr.scenes[i], wr.draft, wr.lineages[i], i as i64 + 1, heading, now],
                )?;
                for (j, (t, text)) in elements.iter().enumerate() {
                    c.execute(
                        "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![new_id(), wr.scenes[i], j as i64 + 1, t, text, now],
                    )?;
                }
            }
            // Story.
            c.execute(
                "INSERT INTO story_act(id, title, position, created_at, updated_at) VALUES (?1, 'Act Two', 1, ?2, ?2)",
                params![wr.act, now],
            )?;
            c.execute(
                "INSERT INTO story_sequence(id, act_id, title, position, created_at, updated_at) VALUES (?1, ?2, 'Collapse', 1, ?3, ?3)",
                params![wr.sequence, wr.act, now],
            )?;
            c.execute(
                "INSERT INTO story_scene_card(id, parent_type, parent_id, short_description, screenplay_scene_id, position, created_at, updated_at)
                 VALUES (?1, 'sequence', ?2, 'Ravi catches Anjali in a lie at the station', ?3, 1, ?4, ?4)",
                params![wr.card, wr.sequence, wr.scenes[0], now],
            )?;
            c.execute(
                "INSERT INTO story_beat(id, parent_type, parent_id, text, position, created_at, updated_at)
                 VALUES (?1, 'act', ?2, 'The marriage starts to crack', 1, ?3, ?3)",
                params![new_id(), wr.act, now],
            )?;
            for (id, name, role, pos) in [
                (&wr.ravi, "Ravi", "Protagonist", 1),
                (&wr.anjali, "Anjali", "Wife", 2),
                (&wr.arjun, "Arjun", "Inspector", 3),
            ] {
                c.execute(
                    "INSERT INTO story_character(id, name, role_label, description, position, created_at, updated_at)
                     VALUES (?1, ?2, ?3, 'A person in the story.', ?4, ?5, ?5)",
                    params![id, name, role, pos, now],
                )?;
            }
            c.execute(
                "INSERT INTO story_character_relationship(id, from_character_id, to_character_id, relationship_type, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'married', 1, ?4, ?4)",
                params![new_id(), wr.ravi, wr.anjali, now],
            )?;
            for ch in [&wr.ravi, &wr.anjali] {
                c.execute(
                    "INSERT INTO story_character_card_link(id, character_id, scene_card_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![new_id(), ch, wr.card, now],
                )?;
            }
            // Production.
            c.execute(
                "INSERT INTO location(id, name, address, contact, status, notes_json, created_at, updated_at)
                 VALUES (?1, 'Railway Station', 'Platform 4, Central', 'Station master 555-0199', 'Confirmed', '{\"access\":\"Night shoots need a permit\"}', ?2, ?2)",
                params![wr.station, now],
            )?;
            c.execute(
                "INSERT INTO catalog_item(id, category, name, description, contact, created_at, updated_at)
                 VALUES (?1, 'Vehicles', 'Red Mustang', 'Picture car, 1968.', 'Rental desk 555-0142', ?2, ?2)",
                params![wr.mustang, now],
            )?;
            c.execute(
                "INSERT INTO cast_member(id, person_name, character_id, character_name, contact, notes, created_at, updated_at)
                 VALUES (?1, 'Dev Kumar', ?2, 'Ravi', 'dev.kumar@example.com 555-0100', 'Prefers night calls', ?3, ?3)",
                params![new_id(), wr.ravi, now],
            )?;
            c.execute(
                "INSERT INTO crew_member(id, person_name, role, department, contact, created_at, updated_at)
                 VALUES (?1, 'Mira Shah', 'Director of Photography', 'Camera', '555-0111', ?2, ?2)",
                params![new_id(), now],
            )?;
            let source = new_id();
            c.execute(
                "INSERT INTO production_source(id, draft_id, active, selected_at, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3, ?3)",
                params![source, wr.draft, now],
            )?;
            c.execute(
                "INSERT INTO breakdown_element(id, source_id, scene_id, scene_lineage_id, category, catalog_item_id, display_name, confirmation_state, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 'Vehicles', ?5, 'Red Mustang', 'Confirmed', ?6, ?6)",
                params![new_id(), source, wr.scenes[0], wr.lineages[0], wr.mustang, now],
            )?;
            let schedule = new_id();
            c.execute(
                "INSERT INTO shooting_schedule(id, source_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![schedule, source, now],
            )?;
            c.execute(
                "INSERT INTO shooting_day(id, schedule_id, position, shoot_date, notes, created_at, updated_at)
                 VALUES (?1, ?2, 1, '2026-10-05', 'Night exterior unit', ?3, ?3)",
                params![wr.day, schedule, now],
            )?;
            c.execute(
                "INSERT INTO schedule_strip(id, schedule_id, day_id, scene_id, scene_lineage_id, position, source_heading, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1, 'INT. RAILWAY STATION - NIGHT', ?6, ?6)",
                params![new_id(), schedule, wr.day, wr.scenes[0], wr.lineages[0], now],
            )?;
            let doc = json!({
                "title": "Railway", "dayLabel": "SHOOT DAY 1", "date": "2026-10-05", "crewCall": "18:00",
                "dayNotes": "Bring rain covers.",
                "scenes": [{ "key": "1", "number": "1", "heading": "INT. RAILWAY STATION - NIGHT", "description": "Ravi and Anjali", "pages": "1" }],
                "practical": { "emergencyContact": "Dr Rao 555-0177", "productionNotes": "Quiet on the platform" }
            });
            c.execute(
                "INSERT INTO call_sheet(id, schedule_id, shoot_day_id, title, source_snapshot_json, source_fingerprint, source_captured_at, document_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'Call Sheet — Day 1', '{}', 'fp', ?4, ?5, ?4, ?4)",
                params![new_id(), schedule, wr.day, now, doc.to_string()],
            )?;
            // Visual planning.
            c.execute(
                "INSERT INTO shot(id, scene_id, scene_lineage_id, position, description, size, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, 'Slow push in on Ravi as the train passes', 'Close-up', ?4, ?4)",
                params![new_id(), wr.scenes[0], wr.lineages[0], now],
            )?;
            let mb = new_id();
            c.execute(
                "INSERT INTO moodboard(id, name, scene_id, scene_lineage_id, position, created_at, updated_at) VALUES (?1, 'Station mood', ?2, ?3, 1, ?4, ?4)",
                params![mb, wr.scenes[0], wr.lineages[0], now],
            )?;
            c.execute(
                "INSERT INTO moodboard_item(id, moodboard_id, kind, body, created_at, updated_at) VALUES (?1, ?2, 'note', 'Sodium lights, wet concrete', ?3, ?3)",
                params![new_id(), mb, now],
            )?;
            let sb = new_id();
            c.execute(
                "INSERT INTO storyboard(id, name, scene_id, scene_lineage_id, position, created_at, updated_at) VALUES (?1, 'Station boards', ?2, ?3, 1, ?4, ?4)",
                params![sb, wr.scenes[0], wr.lineages[0], now],
            )?;
            c.execute(
                "INSERT INTO storyboard_panel(id, storyboard_id, description, position, created_at, updated_at) VALUES (?1, ?2, 'Wide of the empty platform', 1, ?3, ?3)",
                params![new_id(), sb, now],
            )?;
            // Notes, tasks, comments, vault, private notes.
            c.execute(
                "INSERT INTO project_note(id, title, body, created_at, updated_at) VALUES (?1, 'Night shoots', 'Difficult night shoots need extra rest days.\n\nBook the generator early.', ?2, ?2)",
                params![new_id(), now],
            )?;
            c.execute(
                "INSERT INTO task(id, title, target_type, target_id, position, created_at, updated_at) VALUES (?1, 'Scout the railway station', 'location', ?2, 1, ?3, ?3)",
                params![new_id(), wr.station, now],
            )?;
            c.execute(
                "INSERT INTO comment(id, target_type, target_id, body, author_user_id, author_name, created_at, updated_at)
                 VALUES (?1, 'story_scene_card', ?2, 'Should the lie be clearer here?', ?3, 'Me', ?4, ?4)",
                params![new_id(), wr.card, me_id, now],
            )?;
            c.execute(
                "INSERT INTO vault_item(id, item_type, title, body, created_at, updated_at) VALUES (?1, 'note', 'Isolation idea', 'A lighthouse keeper who never leaves the island.', ?2, ?2)",
                params![new_id(), now],
            )?;
            c.execute(
                "INSERT INTO private_note(id, owner_user_id, target_type, target_id, body, created_at, updated_at)
                 VALUES (?1, ?2, 'screenplay_scene', ?3, 'MY-PRIVATE: the Mustang rental is too expensive', ?4, ?4)",
                params![wr.my_note, me_id, wr.scenes[0], now],
            )?;
            c.execute(
                "INSERT INTO private_note(id, owner_user_id, target_type, target_id, body, created_at, updated_at)
                 VALUES (?1, ?2, 'screenplay_scene', ?3, 'OTHER-PRIVATE: casting doubts about the Mustang scene', ?4, ?4)",
                params![wr.other_note, other_user, wr.scenes[0], now],
            )?;
            Ok(())
        })
        .unwrap();
    w
}

fn setup() -> (TestEnv, World, Arc<ConceptEmbedder>) {
    let env = TestEnv::with_project("Railway", "Feature Film");
    fast_settings(&env);
    let e = attach(&env);
    let w = seed(&env);
    idle(&env);
    (env, w, e)
}

fn ask(env: &TestEnv, actor: &Actor, q: &str, route: Option<RetrievalRoute>) -> ContextPacket {
    retrieval::retrieve(
        &env.core,
        actor,
        &RetrieveRequest {
            query: q.into(),
            scope: None,
            limit: 12,
            route,
        },
    )
    .unwrap()
}

fn text_of(p: &ContextPacket) -> String {
    p.items
        .iter()
        .map(|i| format!("{}\n{}", i.source, i.text))
        .collect::<Vec<_>>()
        .join("\n---\n")
}

fn has(p: &ContextPacket, entity_type: &str, id: &str) -> bool {
    p.sources
        .iter()
        .any(|s| s.entity_type == entity_type && s.entity_id == id)
}

fn index_conn(env: &TestEnv) -> Connection {
    let ix = intelligence::service(&env.core).current().unwrap();
    let c = Connection::open(ix.index_path()).unwrap();
    openframe_search::vector::register(&c).unwrap();
    c
}

fn scalar(c: &Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |r| r.get(0)).unwrap()
}

fn other_user_actor(w: &World) -> Actor {
    Actor {
        user_id: w.other_user.clone(),
        display_name: "Collaborator".into(),
        role: Role::Editor,
        origin: ActorOrigin::Local,
    }
}

fn mutate(env: &TestEnv, f: impl FnOnce(&Connection) -> rusqlite::Result<()>) {
    env.core
        .project()
        .unwrap()
        .store
        .mutate(
            &env.actor(),
            MutationMeta::new("test.edit", "Edit", Capability::Edit),
            |tx| {
                f(tx.conn())?;
                Ok(())
            },
        )
        .unwrap();
}

// ------------------------------------------------------------------ tests

#[test]
fn spec_13_every_domain_gets_documents_chunks_vectors_and_graph() {
    let (env, _w, _e) = setup();
    let c = index_conn(&env);
    for kind in [
        "project",
        "screenplay_scene",
        "story_act",
        "story_sequence",
        "story_beat",
        "story_scene_card",
        "story_character",
        "project_note",
        "private_note",
        "task",
        "comment",
        "vault_item",
        "catalog_item",
        "location",
        "cast_member",
        "crew_member",
        "scene_breakdown",
        "shooting_day",
        "call_sheet",
        "moodboard",
        "moodboard_item",
        "storyboard",
        "storyboard_panel",
        "shot",
    ] {
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM semantic_document WHERE entity_type=?1",
                [kind],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n >= 1, "no document for {kind}");
    }
    // Only the current draft's scenes are indexed (4 scenes, not the old draft's mill scene).
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_document WHERE entity_type='screenplay_scene'"
        ),
        4
    );
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_document WHERE body LIKE '%OLD MILL%'"
        ),
        0
    );
    // Every chunk is embedded with the test model and carries the §14 fields.
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_chunk WHERE embedding_model_id IS NULL"
        ),
        0
    );
    assert!(
        scalar(&c, "SELECT count(*) FROM vec_chunk")
            >= scalar(&c, "SELECT count(*) FROM semantic_document")
    );
    // Canonical + derived graph edges; nothing inferred.
    for rel in [
        "appears_in",
        "located_at",
        "represented_by",
        "belongs_to",
        "related_to",
        "requires",
        "scheduled_on",
        "plays",
        "for_day",
        "for_scene",
        "part_of",
        "about",
        "on",
    ] {
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM context_edge WHERE relation=?1",
                [rel],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n >= 1, "no {rel} edge");
    }
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM context_edge WHERE provenance='Inferred'"
        ),
        0
    );
    // Heading → location is a deterministic parse (Derived); FK links are Canonical.
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM context_edge WHERE relation='located_at' AND provenance='Derived'"
        ),
        1
    );
    assert!(
        scalar(
            &c,
            "SELECT count(*) FROM context_edge WHERE relation='requires' AND provenance='Canonical'"
        ) >= 1
    );
    let meta: String = c
        .query_row(
            "SELECT value FROM index_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(meta, openframe_search::SCHEMA_VERSION.to_string());
    let snap = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot();
    assert_eq!(snap.state, IndexState::Current);
    assert!(snap.semantic_ready);
}

#[test]
fn scene_documents_carry_characters_location_and_story_context() {
    let (env, w, _e) = setup();
    let c = index_conn(&env);
    let chunk: String = c
        .query_row(
            "SELECT text FROM semantic_chunk WHERE doc_id=?1 AND ordinal=0",
            [format!("screenplay_scene:{}", w.scenes[0])],
            |r| r.get(0),
        )
        .unwrap();
    assert!(chunk.contains("Characters: Ravi, Anjali"), "{chunk}");
    assert!(chunk.contains("Location: RAILWAY STATION"), "{chunk}");
    assert!(chunk.contains("Story: Act Two › Collapse"), "{chunk}");
    assert!(chunk.contains("He suspects she lies to him."), "{chunk}");
}

#[test]
fn spec_20_private_notes_are_only_ever_returned_to_their_owner() {
    let (env, w, _e) = setup();
    let me = env.actor();
    for route in [
        RetrievalRoute::Lexical,
        RetrievalRoute::Hybrid,
        RetrievalRoute::HybridGraph,
    ] {
        let p = ask(
            &env,
            &me,
            "private notes about the Mustang scene casting doubts",
            Some(route),
        );
        let t = text_of(&p);
        assert!(
            !t.contains("OTHER-PRIVATE"),
            "{route:?} leaked another user's note:\n{t}"
        );
        assert!(!has(&p, "private_note", &w.other_note));
    }
    let p = ask(
        &env,
        &me,
        "Mustang rental expensive",
        Some(RetrievalRoute::Hybrid),
    );
    assert!(
        has(&p, "private_note", &w.my_note),
        "owner sees their own note"
    );
    let other = other_user_actor(&w);
    let p = ask(
        &env,
        &other,
        "Mustang rental casting doubts",
        Some(RetrievalRoute::HybridGraph),
    );
    let t = text_of(&p);
    assert!(t.contains("OTHER-PRIVATE"));
    assert!(!t.contains("MY-PRIVATE"), "{t}");
    // Graph expansion never walks into someone else's private node either.
    let c = index_conn(&env);
    let owner: String = c
        .query_row(
            "SELECT owner_user_id FROM context_node WHERE node_id=?1",
            [format!("private_note:{}", w.other_note)],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owner, w.other_user);
}

#[test]
fn contact_details_never_enter_documents_or_context() {
    let (env, _w, _e) = setup();
    let c = index_conn(&env);
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_document WHERE body LIKE '%555-01%' OR title LIKE '%555-01%'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_chunk WHERE text LIKE '%555-01%' OR text LIKE '%@example.com%'"
        ),
        0
    );
    let p = ask(
        &env,
        &env.actor(),
        "Dev Kumar Mira Shah station master rental desk contact phone",
        Some(RetrievalRoute::Hybrid),
    );
    let t = text_of(&p);
    assert!(!t.contains("555-01") && !t.contains("@example.com"), "{t}");
    assert!(
        t.contains("Dev Kumar plays Ravi"),
        "cast still retrievable without contacts: {t}"
    );
}

#[test]
fn spec_19_hybrid_finds_meaning_that_keyword_search_misses() {
    let (env, w, _e) = setup();
    let q = "Which moment shows suspicion and betrayal between the couple?";
    let lexical = ask(&env, &env.actor(), q, Some(RetrievalRoute::Lexical));
    assert!(
        !has(&lexical, "screenplay_scene", &w.scenes[0]),
        "FTS has no shared terms"
    );
    let hybrid = ask(&env, &env.actor(), q, Some(RetrievalRoute::Hybrid));
    assert_eq!(hybrid.sources[0].entity_type, "screenplay_scene");
    assert_eq!(hybrid.sources[0].entity_id, w.scenes[0]);
    assert_eq!(hybrid.sources[0].via, "semantic");
    assert!(!hybrid.degraded);
    assert_eq!(hybrid.provenance[0].kind, "Scene");
    assert!(
        hybrid.provenance[0]
            .label
            .starts_with("Scene 1 — INT. RAILWAY STATION")
    );
    // Exact names still win lexically.
    let p = ask(&env, &env.actor(), "Railway Station", None);
    assert_eq!(p.route, RetrievalRoute::Lexical);
    assert!(has(&p, "location", &w.station));
}

#[test]
fn spec_18_graph_expansion_adds_related_cross_module_context() {
    let (env, w, _e) = setup();
    let q = "What props and locations does the railway station scene need on the shooting day?";
    assert_eq!(retrieval::route_for(q), RetrievalRoute::HybridGraph);
    let without = ask(&env, &env.actor(), q, Some(RetrievalRoute::Hybrid));
    assert!(
        !has(&without, "catalog_item", &w.mustang),
        "no shared terms without the graph"
    );
    let with = ask(&env, &env.actor(), q, None);
    assert_eq!(with.route, RetrievalRoute::HybridGraph);
    assert!(
        has(&with, "catalog_item", &w.mustang),
        "{:#?}",
        with.sources
    );
    let m = with
        .sources
        .iter()
        .find(|s| s.entity_type == "catalog_item")
        .unwrap();
    assert_eq!(m.via, "graph");
    assert_eq!(m.relation.as_deref(), Some("requires"));
    assert_eq!(m.nav["workspace"], "production");
    assert!(with.items.len() <= 12);
    let total: usize = with.items.iter().map(|i| i.text.chars().count()).sum();
    assert!(total <= intelligence::assembler::CONTEXT_CHARS);
}

#[test]
fn spec_15_edits_are_indexed_after_commit_and_undo_redo_follow() {
    let (env, w, _e) = setup();
    env.ok(
        "story.update_character",
        json!({ "id": w.ravi, "name": "Raghav" }),
    );
    idle(&env);
    let c = index_conn(&env);
    let title = |c: &Connection| -> String {
        c.query_row(
            "SELECT title FROM semantic_document WHERE doc_id=?1",
            [format!("story_character:{}", w.ravi)],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(title(&c), "Raghav");
    env.undo();
    idle(&env);
    assert_eq!(title(&c), "Ravi");
    env.redo();
    idle(&env);
    assert_eq!(title(&c), "Raghav");
    // Element edits reach their scene (the undo images carry the scene id).
    let el: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM screenplay_element WHERE scene_id=?1 AND element_type='action' ORDER BY position LIMIT 1",
                [&w.scenes[2]],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    mutate(&env, |c| {
        c.execute("UPDATE screenplay_element SET text='Anjali burns the letters at the water''s edge.' WHERE id=?1", [&el])?;
        Ok(())
    });
    idle(&env);
    // (Raw SQL edits bypass the screenplay ops' FTS re-index; the intelligence index
    // still follows them through the row images.)
    let p = ask(
        &env,
        &env.actor(),
        "burns the letters",
        Some(RetrievalRoute::Semantic),
    );
    assert!(has(&p, "screenplay_scene", &w.scenes[2]));
    assert!(text_of(&p).contains("burns the letters"));
    env.undo();
    idle(&env);
    let body: String = c
        .query_row(
            "SELECT body FROM semantic_document WHERE doc_id=?1",
            [format!("screenplay_scene:{}", w.scenes[2])],
            |r| r.get(0),
        )
        .unwrap();
    assert!(body.contains("isolated and lonely") && !body.contains("burns the letters"));
}

#[test]
fn spec_32_delete_removes_derived_entries_and_restore_rebuilds_them() {
    let (env, w, _e) = setup();
    env.ok("story.delete_character", json!({ "id": w.arjun }));
    idle(&env);
    let c = index_conn(&env);
    let doc = format!("story_character:{}", w.arjun);
    let node = doc.clone();
    let count = |sql: &str, id: &str| -> i64 { c.query_row(sql, [id], |r| r.get(0)).unwrap() };
    assert_eq!(
        count(
            "SELECT count(*) FROM semantic_document WHERE doc_id=?1",
            &doc
        ),
        0
    );
    assert_eq!(
        count("SELECT count(*) FROM semantic_chunk WHERE doc_id=?1", &doc),
        0
    );
    assert_eq!(
        count("SELECT count(*) FROM context_node WHERE node_id=?1", &node),
        0
    );
    let p = ask(
        &env,
        &env.actor(),
        "Arjun inspector",
        Some(RetrievalRoute::HybridGraph),
    );
    assert!(
        !has(&p, "story_character", &w.arjun),
        "deleted content never returned"
    );
    let deleted: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM deleted_item WHERE object_id=?1",
                [&w.arjun],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    env.ok("trash.restore", json!({ "id": deleted }));
    idle(&env);
    assert_eq!(
        count(
            "SELECT count(*) FROM semantic_document WHERE doc_id=?1",
            &doc
        ),
        1
    );
    assert_eq!(
        count("SELECT count(*) FROM context_node WHERE node_id=?1", &node),
        1
    );
    // The scene's derived "appears_in" edge towards Arjun is live again.
    assert!(
        count(
            "SELECT count(*) FROM context_edge WHERE from_node_id=?1 AND relation='appears_in'",
            &node
        ) >= 1
    );
    let p = ask(
        &env,
        &env.actor(),
        "Arjun inspector",
        Some(RetrievalRoute::Lexical),
    );
    assert!(has(&p, "story_character", &w.arjun));
}

#[test]
fn spec_15_saves_never_wait_for_embedding_and_typing_stays_responsive() {
    let env = TestEnv::with_project("Typing", "Feature Film");
    fast_settings(&env);
    let e = attach(&env);
    let w = seed(&env);
    e.set_delay(Some(Duration::from_millis(150)));
    intelligence::activate(&env.core).unwrap();
    let el: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM screenplay_element WHERE scene_id=?1 ORDER BY position LIMIT 1",
                [&w.scenes[1]],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    let mut worst = Duration::ZERO;
    for i in 0..20 {
        let text = format!("Ravi paces the corridor, take {i}.");
        let t = Instant::now();
        mutate(&env, |c| {
            c.execute(
                "UPDATE screenplay_element SET text=?1 WHERE id=?2",
                params![text, el],
            )?;
            Ok(())
        });
        worst = worst.max(t.elapsed());
    }
    // Each save completes while the (slow) embedder is busy in the background.
    assert!(worst < Duration::from_millis(120), "slowest save {worst:?}");
    idle(&env);
    let c = index_conn(&env);
    let body: String = c
        .query_row(
            "SELECT body FROM semantic_document WHERE doc_id=?1",
            [format!("screenplay_scene:{}", w.scenes[1])],
            |r| r.get(0),
        )
        .unwrap();
    assert!(body.contains("take 19"), "last edit indexed");
    assert_eq!(
        scalar(
            &c,
            "SELECT count(*) FROM semantic_chunk WHERE embedding_model_id IS NULL"
        ),
        0
    );
}

#[test]
fn spec_33_embedding_failure_never_blocks_saves_and_retrieval_falls_back() {
    let env = TestEnv::with_project("Failing", "Feature Film");
    fast_settings(&env);
    let e = attach(&env);
    e.set_failing(true);
    let w = seed(&env);
    intelligence::activate(&env.core).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    env.ok(
        "story.update_character",
        json!({ "id": w.anjali, "roleLabel": "Lead" }),
    );
    let snap = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot();
    assert!(!snap.semantic_ready);
    let p = ask(
        &env,
        &env.actor(),
        "Railway Station night",
        Some(RetrievalRoute::Hybrid),
    );
    assert!(p.degraded, "semantic path unavailable");
    assert_eq!(p.notice.as_deref(), Some(retrieval::DEGRADED_NOTICE));
    assert!(
        has(&p, "location", &w.station),
        "FTS fallback still answers"
    );
    // The runtime recovers: vectors are filled in without any user action.
    e.set_failing(false);
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let snap = intelligence::service(&env.core)
            .current()
            .unwrap()
            .snapshot();
        if snap.semantic_ready {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "embeddings never recovered: {snap:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let p = ask(
        &env,
        &env.actor(),
        "Railway Station night",
        Some(RetrievalRoute::Hybrid),
    );
    assert!(!p.degraded);
}

#[test]
fn spec_33_without_an_embedding_runtime_retrieval_is_fts_plus_sql() {
    let env = TestEnv::with_project("No embedder", "Feature Film");
    fast_settings(&env);
    let w = seed(&env);
    idle(&env);
    let snap = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot();
    assert_eq!(snap.state, IndexState::Current);
    assert!(!snap.embedder_present && !snap.semantic_ready);
    assert!(
        snap.counts.documents > 10,
        "documents and graph still built"
    );
    let p = ask(
        &env,
        &env.actor(),
        "What happens at the railway station?",
        Some(RetrievalRoute::HybridGraph),
    );
    assert!(p.degraded);
    assert!(has(&p, "screenplay_scene", &w.scenes[0]));
}

#[test]
fn spec_20_draft_boundaries_and_explicit_scope_come_first() {
    let (env, w, _e) = setup();
    let actor = env.actor();
    let s = env.core.project().unwrap();
    let old_scope = s
        .store
        .read(|c| {
            let args: AiScopeArgs =
                serde_json::from_value(json!({ "kind": "SpecificDraft", "draftId": w.draft_old }))
                    .unwrap();
            scope::resolve(c, &actor, &args)
        })
        .unwrap();
    let p = retrieval::retrieve(
        &env.core,
        &actor,
        &RetrieveRequest {
            query: "railway station night".into(),
            scope: Some(old_scope),
            limit: 10,
            route: Some(RetrievalRoute::Hybrid),
        },
    )
    .unwrap();
    assert!(
        !p.sources
            .iter()
            .any(|x| x.entity_type == "screenplay_scene"),
        "another draft's scenes are excluded: {:#?}",
        p.sources
    );
    assert_eq!(p.sources[0].via, "scope");
    assert!(p.items[0].text.contains("OLD MILL"));
    // Current-scene scope: the scene is in the scope item and not duplicated.
    let scene_scope = s
        .store
        .read(|c| {
            let args: AiScopeArgs =
                serde_json::from_value(json!({ "kind": "CurrentScene", "sceneId": w.scenes[0] }))
                    .unwrap();
            scope::resolve(c, &actor, &args)
        })
        .unwrap();
    let p = retrieval::retrieve(
        &env.core,
        &actor,
        &RetrieveRequest {
            query: "Ravi suspects Anjali".into(),
            scope: Some(scene_scope),
            limit: 10,
            route: Some(RetrievalRoute::Hybrid),
        },
    )
    .unwrap();
    assert_eq!(p.sources[0].via, "scope");
    assert!(
        !has(&p, "screenplay_scene", &w.scenes[0]),
        "no duplicate of the scope scene"
    );
}

#[test]
fn spec_21_routes_structured_and_product_help_without_project_rag() {
    let (env, _w, _e) = setup();
    let p = ask(&env, &env.actor(), "How many scenes are there?", None);
    assert_eq!(p.route, RetrievalRoute::Structured);
    assert!(
        p.items.is_empty(),
        "exact facts come from SQL tools, not retrieved chunks"
    );
    let p = ask(&env, &env.actor(), "How do I lock a draft?", None);
    assert_eq!(p.route, RetrievalRoute::ProductHelp);
    assert!(!p.items.is_empty());
    assert!(p.provenance.iter().all(|x| x.kind == "Product guide"));
}

#[test]
fn retrieval_inherits_permissions_and_bounds_its_input() {
    let (env, _w, _e) = setup();
    let export_only = env.actor_with_role(Role::ExportOnly);
    let err = retrieval::retrieve(
        &env.core,
        &export_only,
        &RetrieveRequest {
            query: "railway".into(),
            scope: None,
            limit: 5,
            route: None,
        },
    )
    .unwrap_err();
    assert!(err.is("permission"), "{err:?}");
    let viewer = env.actor_with_role(Role::Viewer);
    let p = ask(&env, &viewer, "railway station", None);
    assert!(!p.items.is_empty(), "viewers may read what they can see");
    let long = "x".repeat(retrieval::MAX_QUERY_CHARS + 1);
    let err = retrieval::retrieve(
        &env.core,
        &env.actor(),
        &RetrieveRequest {
            query: long,
            scope: None,
            limit: 5,
            route: None,
        },
    )
    .unwrap_err();
    assert!(err.is("validation"));
    let p = retrieval::retrieve(
        &env.core,
        &env.actor(),
        &RetrieveRequest {
            query: "station night ravi anjali mustang".into(),
            scope: None,
            limit: 500,
            route: Some(RetrievalRoute::HybridGraph),
        },
    )
    .unwrap();
    assert!(p.items.len() <= retrieval::MAX_ITEMS);
    assert_eq!(p.items.len(), p.provenance.len());
    assert_eq!(p.items.len(), p.sources.len());
}

#[test]
fn spec_32_index_survives_restart_and_is_rebuilt_when_invalid() {
    let (env, _w, e) = setup();
    let path = env.project_path();
    let docs_before = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot()
        .counts
        .documents;
    assert!(e.texts() > 0);
    // Restart: the index is reused, nothing is re-embedded.
    let env = env.restart();
    fast_settings(&env);
    env.reopen_project(&path);
    let e2 = attach(&env);
    idle(&env);
    let snap = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot();
    assert_eq!(snap.counts.documents, docs_before);
    assert_eq!(e2.texts(), 0, "unchanged documents are not re-embedded");
    // A different embedding model replaces every vector.
    let e3 = Arc::new(ConceptEmbedder::new().with_model_id("test-concept-embedder@2"));
    intelligence::service(&env.core).set_embedder(Some(e3.clone() as Arc<dyn Embedder>));
    idle(&env);
    let chunks = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot()
        .counts
        .chunks;
    assert_eq!(e3.texts() as u64, chunks);
    // A damaged or hostile index file is discarded and rebuilt; the project is untouched.
    let index_file = std::path::PathBuf::from(&path)
        .join("cache")
        .join("intelligence.sqlite");
    let env = env.restart();
    fast_settings(&env);
    for suffix in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{suffix}", index_file.display()));
    }
    std::fs::write(&index_file, b"not a database at all").unwrap();
    env.reopen_project(&path);
    attach(&env);
    idle(&env);
    let snap = intelligence::service(&env.core)
        .current()
        .unwrap()
        .snapshot();
    assert_eq!(snap.counts.documents, docs_before);
    assert_eq!(snap.state, IndexState::Current);
    // A canonical schema change (migration) marks it stale → rebuilt.
    let env = env.restart();
    fast_settings(&env);
    {
        let c = Connection::open(&index_file).unwrap();
        c.execute(
            "UPDATE index_meta SET value='1' WHERE key='canonical_schema_version'",
            [],
        )
        .unwrap();
    }
    env.reopen_project(&path);
    let e4 = attach(&env);
    idle(&env);
    assert!(e4.texts() > 0, "rebuilt from scratch after a schema change");
    assert_eq!(
        intelligence::service(&env.core)
            .current()
            .unwrap()
            .snapshot()
            .counts
            .documents,
        docs_before
    );
}

#[test]
fn ops_report_index_status_and_rebuild_in_the_background() {
    let (env, _w, _e) = setup();
    let st = env.ok("ai.index_status", json!({}));
    assert_eq!(st["state"], "Current");
    assert_eq!(st["semanticReady"], true);
    assert!(st["documents"].as_u64().unwrap() > 10);
    assert!(st["message"].is_null());
    let viewer = env.actor_with_role(Role::Viewer);
    assert!(env.call_as(&viewer, "ai.index_status", json!({})).is_ok());
    let export_only = env.actor_with_role(Role::ExportOnly);
    let err = env
        .call_as(&export_only, "ai.index_rebuild", json!({}))
        .unwrap_err();
    assert!(err.is("permission"));
    let started = env.ok("ai.index_rebuild", json!({}));
    let task_id = started["taskId"].as_str().unwrap().to_string();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let done = env.sink.events.lock().iter().any(
            |e| matches!(e, AppEvent::Task(t) if t.task_id == task_id && t.state == "completed"),
        );
        if done {
            break;
        }
        assert!(Instant::now() < deadline, "rebuild task did not complete");
        std::thread::sleep(Duration::from_millis(20));
    }
    let st = env.ok("ai.index_status", json!({}));
    assert_eq!(st["state"], "Current");
    assert!(st["embeddedChunks"].as_u64().unwrap() > 0);
    // Rebuilding touches nothing canonical: no undo step, no activity.
    let undo = env.ok("history.info", json!({}));
    assert_ne!(undo["undoLabel"], "ai.index_rebuild");
    assert_eq!(
        env.err("ai.index_status", json!({ "x": 1 })),
        "validation.invalid_input"
    );
}

#[test]
fn project_close_stops_the_indexer_and_releases_the_index_file() {
    let (env, _w, _e) = setup();
    let path = env.project_path();
    env.ok("project.close", json!({}));
    let index_file = std::path::PathBuf::from(&path)
        .join("cache")
        .join("intelligence.sqlite");
    // Deleting the derived index right after close works (no handle left open) and loses nothing.
    std::fs::remove_file(&index_file).expect("index file released");
    env.reopen_project(&path);
    assert_eq!(retrieval::index_status(&env.core), IndexState::Stale);
    idle(&env);
    assert_eq!(retrieval::index_status(&env.core), IndexState::Current);
}
