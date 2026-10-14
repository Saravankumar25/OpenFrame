//! §34 performance measurements for the intelligence index and hybrid
//! retrieval on a generated, feature-length fixture (deterministic content).
//!
//! Budgets (spec §34): FTS retrieval < 150 ms, vector search < 100 ms, hybrid
//! retrieval + context assembly < 250 ms typical, graph expansion < 50 ms, no
//! embedding work inside a write transaction (saves stay fast while indexing).
//!
//! Debug builds (our crates at opt-level 0) are checked against 3× the budget;
//! `cargo test --release -p openframe-application --test ai_retrieval_perf -- --nocapture`
//! checks the budgets as written and prints the numbers recorded in
//! docs/engineering/18-retrieval-intelligence-index.md.

use std::sync::Arc;
use std::time::{Duration, Instant};

use openframe_application::MutationMeta;
use openframe_application::modules::ai::intelligence::{self, IndexSettings};
use openframe_application::modules::ai::retrieval::{self, RetrievalRoute, RetrieveRequest};
use openframe_domain::{Capability, new_id, now_ms};
use openframe_search::graph::{self, TraversalLimits};
use openframe_search::{Embedder, SemanticIndex, SqliteVecIndex, docs};
use openframe_test_support::TestEnv;
use openframe_test_support::embedder::ConceptEmbedder;
use rusqlite::{Connection, params};

const SCENES: usize = 240;
const ELEMENTS_PER_SCENE: usize = 12;
const CHARACTERS: usize = 40;
const LOCATIONS: usize = 60;
const CATALOG: usize = 300;
const DAYS: usize = 30;

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[(self.next() as usize) % v.len()]
    }
}

const WORDS: &[&str] = &[
    "rain", "platform", "letter", "window", "silence", "promise", "doubt", "lies", "hospital",
    "doctor", "argument", "alone", "train", "mustang", "river", "market", "phone", "money", "debt",
    "brother", "sister", "wedding", "funeral", "guilt", "shame", "night", "dawn", "shadow",
    "stairs", "kitchen", "office", "police", "file", "photo", "secret", "truth", "betrayal",
    "smile", "tears", "door", "street", "crowd", "music", "radio", "storm", "fire",
];
const NAMES: &[&str] = &[
    "RAVI", "ANJALI", "ARJUN", "MEERA", "KABIR", "NISHA", "VIKRAM", "LATA", "SAMEER", "TARA",
    "OMAR", "PRIYA",
];

fn sentence(r: &mut Lcg, n: usize) -> String {
    let mut s: Vec<&str> = (0..n).map(|_| r.pick(WORDS)).collect();
    s[0] = r.pick(&["He", "She", "They", "Nobody", "Someone"]);
    format!("{}.", s.join(" "))
}

fn seed(env: &TestEnv) {
    let s = env.core.project().unwrap();
    s.store
        .mutate(&env.actor(), MutationMeta::new("test.seed", "Seed", Capability::Edit), |tx| {
            let c = tx.conn();
            let now = now_ms();
            let mut r = Lcg(7);
            let sp = new_id();
            let draft = new_id();
            c.execute(
                "INSERT INTO screenplay(id, title, current_draft_id, created_at, updated_at) VALUES (?1, 'Perf', ?2, ?3, ?3)",
                params![sp, draft, now],
            )?;
            c.execute(
                "INSERT INTO screenplay_draft(id, screenplay_id, name, created_at, updated_at) VALUES (?1, ?2, 'Draft 1', ?3, ?3)",
                params![draft, sp, now],
            )?;
            let mut chars = Vec::new();
            for i in 0..CHARACTERS {
                let id = new_id();
                let name = format!("{} {i}", NAMES[i % NAMES.len()]);
                c.execute(
                    "INSERT INTO story_character(id, name, role_label, description, position, created_at, updated_at) VALUES (?1, ?2, 'Supporting', ?3, ?4, ?5, ?5)",
                    params![id, name, sentence(&mut r, 14), i as i64, now],
                )?;
                chars.push((id, name));
            }
            let mut locs = Vec::new();
            for i in 0..LOCATIONS {
                let id = new_id();
                let name = format!("{} {} {i}", r.pick(&["Old", "New", "North", "East"]), r.pick(&["Station", "Market", "Hospital", "Temple", "Harbour"]));
                c.execute(
                    "INSERT INTO location(id, name, address, status, created_at, updated_at) VALUES (?1, ?2, 'Somewhere', 'Confirmed', ?3, ?3)",
                    params![id, name, now],
                )?;
                locs.push((id, name));
            }
            let mut items = Vec::new();
            for i in 0..CATALOG {
                let id = new_id();
                c.execute(
                    "INSERT INTO catalog_item(id, category, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![id, r.pick(&["Props", "Costumes", "Vehicles", "Set Dressing"]), format!("Item {i} {}", r.pick(WORDS)), sentence(&mut r, 8), now],
                )?;
                items.push(id);
            }
            let source = new_id();
            c.execute(
                "INSERT INTO production_source(id, draft_id, active, selected_at, created_at, updated_at) VALUES (?1, ?2, 1, ?3, ?3, ?3)",
                params![source, draft, now],
            )?;
            let schedule = new_id();
            c.execute(
                "INSERT INTO shooting_schedule(id, source_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params![schedule, source, now],
            )?;
            let days: Vec<String> = (0..DAYS).map(|_| new_id()).collect();
            for (i, d) in days.iter().enumerate() {
                c.execute(
                    "INSERT INTO shooting_day(id, schedule_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![d, schedule, i as i64 + 1, now],
                )?;
            }
            let act = new_id();
            c.execute(
                "INSERT INTO story_act(id, title, position, created_at, updated_at) VALUES (?1, 'Act One', 1, ?2, ?2)",
                params![act, now],
            )?;
            for i in 0..SCENES {
                let scene = new_id();
                let lineage = new_id();
                let (_, loc) = &locs[(r.next() as usize) % locs.len()];
                let heading = format!("{}. {} - {}", r.pick(&["INT", "EXT"]), loc.to_uppercase(), r.pick(&["DAY", "NIGHT"]));
                c.execute(
                    "INSERT INTO screenplay_scene(id, draft_id, lineage_id, position, heading, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![scene, draft, lineage, i as i64 + 1, heading, now],
                )?;
                for j in 0..ELEMENTS_PER_SCENE {
                    let (t, text) = match j % 3 {
                        0 => ("action", sentence(&mut r, 18)),
                        1 => ("character", chars[(r.next() as usize) % chars.len()].1.to_uppercase()),
                        _ => ("dialogue", sentence(&mut r, 10)),
                    };
                    c.execute(
                        "INSERT INTO screenplay_element(id, scene_id, position, element_type, text, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                        params![new_id(), scene, j as i64 + 1, t, text, now],
                    )?;
                }
                for _ in 0..3 {
                    let item = &items[(r.next() as usize) % items.len()];
                    c.execute(
                        "INSERT INTO breakdown_element(id, source_id, scene_id, scene_lineage_id, category, catalog_item_id, display_name, confirmation_state, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, 'Props', ?5, 'Item', 'Confirmed', ?6, ?6)",
                        params![new_id(), source, scene, lineage, item, now],
                    )?;
                }
                c.execute(
                    "INSERT INTO schedule_strip(id, schedule_id, day_id, scene_id, scene_lineage_id, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![new_id(), schedule, days[i % DAYS], scene, lineage, i as i64, now],
                )?;
                c.execute(
                    "INSERT INTO story_scene_card(id, parent_type, parent_id, short_description, screenplay_scene_id, position, created_at, updated_at)
                     VALUES (?1, 'act', ?2, ?3, ?4, ?5, ?6, ?6)",
                    params![new_id(), act, sentence(&mut r, 9), scene, i as i64, now],
                )?;
            }
            for i in 0..50 {
                c.execute(
                    "INSERT INTO project_note(id, title, body, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![new_id(), format!("Note {i}"), format!("{}\n\n{}", sentence(&mut r, 30), sentence(&mut r, 30)), now],
                )?;
                c.execute(
                    "INSERT INTO task(id, title, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![new_id(), sentence(&mut r, 6), i as i64, now],
                )?;
                c.execute(
                    "INSERT INTO vault_item(id, item_type, title, body, created_at, updated_at) VALUES (?1, 'note', ?2, ?3, ?4, ?4)",
                    params![new_id(), format!("Idea {i}"), sentence(&mut r, 25), now],
                )?;
            }
            Ok(())
        })
        .unwrap();
}

fn pct(mut v: Vec<Duration>, p: f64) -> Duration {
    v.sort();
    let i = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[i]
}

fn factor() -> u32 {
    if cfg!(debug_assertions) { 3 } else { 1 }
}

fn check(label: &str, samples: Vec<Duration>, budget_ms: u64) {
    let p50 = pct(samples.clone(), 0.5);
    let p95 = pct(samples, 0.95);
    let limit = Duration::from_millis(budget_ms * factor() as u64);
    println!(
        "§34 {label}: p50 {:.1} ms, p95 {:.1} ms (budget {budget_ms} ms, checked against {} ms)",
        p50.as_secs_f64() * 1e3,
        p95.as_secs_f64() * 1e3,
        limit.as_millis()
    );
    assert!(p50 <= limit, "{label} p50 {p50:?} over {limit:?}");
}

#[test]
fn spec_34_retrieval_budgets_on_a_feature_length_fixture() {
    let env = TestEnv::with_project("Perf", "Feature Film");
    intelligence::service(&env.core).set_settings(IndexSettings {
        debounce: Duration::from_millis(0),
        max_latency: Duration::from_millis(0),
        ..IndexSettings::default()
    });
    let embedder = Arc::new(ConceptEmbedder::new());
    intelligence::service(&env.core).set_embedder(Some(embedder.clone() as Arc<dyn Embedder>));
    seed(&env);

    // Initial build (documents + graph + embeddings), in the background.
    let t = Instant::now();
    let ix = intelligence::activate(&env.core).unwrap();
    assert!(ix.wait_idle(Duration::from_secs(600)));
    let build = t.elapsed();
    let snap = ix.snapshot();
    println!(
        "§34 initial index: {} documents, {} chunks, {} nodes, {} edges in {:.2} s ({} embedding calls)",
        snap.counts.documents,
        snap.counts.chunks,
        snap.counts.nodes,
        snap.counts.edges,
        build.as_secs_f64(),
        embedder.calls()
    );
    assert!(snap.semantic_ready);

    let actor = env.actor();
    let queries = [
        "Where does she begin to doubt his promise?",
        "scenes with the mustang at night",
        "guilt and shame after the funeral",
        "RAVI 0 argument in the kitchen",
        "hospital doctor truth",
        "Old Station 3",
        "betrayal between brother and sister",
        "what props does the harbour scene need on the shooting day",
        "letters and photos hidden in the office",
        "storm on the platform",
    ];
    let run = |route: RetrievalRoute| -> Vec<Duration> {
        let mut out = Vec::new();
        for _ in 0..3 {
            for q in queries {
                let t = Instant::now();
                let p = retrieval::retrieve(
                    &env.core,
                    &actor,
                    &RetrieveRequest {
                        query: q.into(),
                        scope: None,
                        limit: 12,
                        route: Some(route),
                    },
                )
                .unwrap();
                out.push(t.elapsed());
                assert!(p.items.len() <= 12);
            }
        }
        out
    };
    // Warm-up (query-embedding cache, statement caches).
    run(RetrievalRoute::Hybrid);
    check(
        "FTS retrieval (lexical route, end-to-end incl. assembly)",
        run(RetrievalRoute::Lexical),
        150,
    );
    check(
        "hybrid retrieval + context assembly",
        run(RetrievalRoute::Hybrid),
        250,
    );
    check(
        "hybrid + graph retrieval + context assembly",
        run(RetrievalRoute::HybridGraph),
        250,
    );

    // Vector KNN alone (after the query embedding is available).
    let conn = Connection::open(ix.index_path()).unwrap();
    openframe_search::vector::register(&conn).unwrap();
    let qv = embedder.vector("Where does she begin to doubt his promise?");
    let mut samples = Vec::new();
    for _ in 0..30 {
        let t = Instant::now();
        let hits = SqliteVecIndex.search(&conn, &qv, 48).unwrap();
        let rows = docs::chunks_by_vec_rowids(
            &conn,
            &hits.iter().map(|h| h.vec_rowid).collect::<Vec<_>>(),
        )
        .unwrap();
        samples.push(t.elapsed());
        assert_eq!(rows.len(), 48);
    }
    check("vector search (k=48, with chunk lookup)", samples, 100);

    // Graph expansion alone: 5 scene seeds, 2 hops, capped.
    let seeds: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT node_id FROM context_node WHERE entity_type='screenplay_scene' LIMIT 5",
            )
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let limits = TraversalLimits {
        max_hops: 2,
        max_nodes: 16,
        max_edges_per_node: 12,
        ..TraversalLimits::default()
    };
    let mut samples = Vec::new();
    for _ in 0..30 {
        let t = Instant::now();
        let hits = graph::expand(&conn, &seeds, &limits, &|_| true).unwrap();
        samples.push(t.elapsed());
        assert!(!hits.is_empty() && hits.len() <= 16);
    }
    check("graph expansion (5 seeds, 2 hops)", samples, 50);

    // Saves while the index is busy: no embedding work in the writer transaction.
    embedder.set_delay(Some(Duration::from_millis(40)));
    let scene_el: String = env
        .core
        .project()
        .unwrap()
        .store
        .read(|c| {
            Ok(c.query_row(
                "SELECT id FROM screenplay_element WHERE element_type='action' LIMIT 1",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    let mut samples = Vec::new();
    for i in 0..30 {
        let t = Instant::now();
        env.core
            .project()
            .unwrap()
            .store
            .mutate(
                &actor,
                MutationMeta::new("test.type", "Typing", Capability::Edit).coalesce("typing"),
                |tx| {
                    tx.conn().execute(
                        "UPDATE screenplay_element SET text=?1 WHERE id=?2",
                        params![format!("She waits by the stairs, beat {i}."), scene_el],
                    )?;
                    Ok(())
                },
            )
            .unwrap();
        samples.push(t.elapsed());
    }
    check("save latency while indexing/embedding runs", samples, 50);
    let t = Instant::now();
    assert!(ix.wait_idle(Duration::from_secs(120)));
    println!(
        "§34 edit → indexed + embedded after the last keystroke: {:.0} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
}
