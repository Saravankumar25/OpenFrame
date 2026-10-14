//! Property tests for the core editing invariants (build plan item property tests).
//!
//! Deterministic randomized tests: a small seeded xorshift generator drives many random edit
//! sequences through the public operation registry (exactly what the UI calls), and every step is
//! checked against a model kept in the test. No property-testing crate is used (a new dependency
//! needs owner approval). Each failure message names the seed and step, so a failure replays
//! exactly; `OPENFRAME_PROPERTY_CASES=<n>` runs `n` times as many seeds for a deeper local soak.
//!
//! Properties:
//! 1. Story Board order after many random multi-card moves equals the model list; card
//!    positions are dense (1..n) and unique in every container; the order survives a restart.
//! 2. Undo and redo identity: undoing every random edit returns the exact canonical database
//!    content (every undo-tracked table, every column, timestamps included) of an earlier state,
//!    in reverse order, back to the start; redoing retraces the same states to the exact final
//!    content. Identities therefore never change.
//! 3. Delete and restore (Recently Deleted): restoring returns the same ids, containers,
//!    positions and text; whole sequences come back with their cards in order.
//! 4. Import, export, import: random screenplays built from a small vocabulary keep scene count,
//!    headings, element types and text through Fountain and FDX, both in the interchange library
//!    and end to end through the application (import, export, re-import as a new draft).

use std::collections::BTreeMap;

use openframe_import_export::fdx::{self, FdxOptions};
use openframe_import_export::{ScreenplayDoc, fountain};
use openframe_test_support::TestEnv;
use serde_json::{Value, json};

// ------------------------------------------------------------------------------ generator

/// xorshift64* (Vigna). Deterministic across platforms and runs.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        // Spread small seeds over the state space; the state must never be zero.
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03 | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        assert!(n > 0);
        (self.next() % n as u64) as usize
    }
    fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.below(i + 1);
            xs.swap(i, j);
        }
    }
}

/// Seeds for one property: `base` cases, multiplied by `OPENFRAME_PROPERTY_CASES` when set.
fn seeds(property: u64, base: u64) -> impl Iterator<Item = u64> {
    let factor = std::env::var("OPENFRAME_PROPERTY_CASES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .max(1);
    (0..base * factor).map(move |i| property * 1_000_003 + i)
}

#[test]
fn generator_is_deterministic_and_covers_its_range() {
    let a: Vec<u64> = {
        let mut r = Rng::new(7);
        (0..64).map(|_| r.next()).collect()
    };
    let b: Vec<u64> = {
        let mut r = Rng::new(7);
        (0..64).map(|_| r.next()).collect()
    };
    assert_eq!(a, b, "same seed, same sequence");
    let mut r = Rng::new(8);
    assert_ne!(a[0], r.next(), "different seeds differ");
    let mut seen = [0usize; 5];
    for _ in 0..5000 {
        seen[r.below(5)] += 1;
    }
    assert!(seen.iter().all(|&n| n > 800), "roughly uniform: {seen:?}");
}

// ------------------------------------------------------------------------------ helpers

fn id(v: &Value) -> String {
    v["id"].as_str().expect("id").to_string()
}

fn item(kind: &str, id: &str) -> Value {
    json!({ "kind": kind, "id": id })
}

fn ids(items: &Value) -> Vec<String> {
    items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect()
}

/// Ordered child ids of an act, a sequence, or "parking".
fn kids(b: &Value, container: &str) -> Vec<String> {
    if container == "parking" || container == "unassigned" {
        return ids(&b[container]);
    }
    for a in b["acts"].as_array().unwrap() {
        if a["id"] == container {
            return ids(&a["items"]);
        }
        for it in a["items"].as_array().unwrap() {
            if it["kind"] == "sequence" && it["id"] == container {
                return ids(&it["items"]);
            }
        }
    }
    panic!("container {container} not on the board: {b}");
}

fn container_json(container: &str, seqs: &[String], acts: &[String]) -> Value {
    if container == "parking" {
        json!({ "parentType": "parking" })
    } else if seqs.iter().any(|s| s == container) {
        json!({ "parentType": "sequence", "parentId": container })
    } else {
        assert!(acts.iter().any(|a| a == container), "{container}");
        json!({ "parentType": "act", "parentId": container })
    }
}

fn create_card(env: &TestEnv, parent: Value, text: &str) -> String {
    id(&env.ok(
        "story.create_card",
        json!({ "parent": parent, "shortDescription": text }),
    ))
}

fn read<T>(env: &TestEnv, f: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>) -> T {
    env.core
        .project()
        .unwrap()
        .store
        .read(|c| Ok(f(c)?))
        .unwrap()
}

/// Live card ids per container (parent_type:parent_id), ordered by stored position, with the
/// positions themselves.
fn stored_card_order(env: &TestEnv) -> BTreeMap<String, Vec<(i64, String)>> {
    read(env, |c| {
        let mut stmt = c.prepare(
            "SELECT parent_type, COALESCE(parent_id, ''), position, id FROM story_scene_card
             WHERE deleted_at IS NULL ORDER BY parent_type, parent_id, position, id",
        )?;
        let mut out: BTreeMap<String, Vec<(i64, String)>> = BTreeMap::new();
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (t, p, pos, id) = row?;
            let key = if t == "parking" { t } else { p };
            out.entry(key).or_default().push((pos, id));
        }
        Ok(out)
    })
}

/// Positions in every container are exactly 1..=n (dense and unique).
#[track_caller]
fn assert_dense(order: &BTreeMap<String, Vec<(i64, String)>>, ctx: &str) {
    for (container, rows) in order {
        let pos: Vec<i64> = rows.iter().map(|(p, _)| *p).collect();
        let want: Vec<i64> = (1..=rows.len() as i64).collect();
        assert_eq!(pos, want, "{ctx}: positions in {container}");
    }
}

/// Positions in every container are unique and strictly increasing in board order.
#[track_caller]
fn assert_unique_increasing(order: &BTreeMap<String, Vec<(i64, String)>>, ctx: &str) {
    for (container, rows) in order {
        assert!(
            rows.windows(2).all(|w| w[0].0 < w[1].0),
            "{ctx}: positions in {container}: {rows:?}"
        );
    }
}

// ------------------------------------------------------------- 1. ordering after moves

struct BoardModel {
    /// Containers in board order: the act's sequences, then the Parking Lot.
    order: Vec<String>,
    cards: BTreeMap<String, Vec<String>>,
}

impl BoardModel {
    fn board_rank(&self, card: &str) -> usize {
        let mut i = 0;
        for c in &self.order {
            for x in &self.cards[c] {
                if x == card {
                    return i;
                }
                i += 1;
            }
        }
        panic!("{card} not in model");
    }
    fn all_cards(&self) -> Vec<String> {
        self.order
            .iter()
            .flat_map(|c| self.cards[c].clone())
            .collect()
    }
}

#[track_caller]
fn assert_board_matches(env: &TestEnv, m: &BoardModel, ctx: &str) {
    let b = env.ok("story.board", json!({}));
    for c in &m.order {
        assert_eq!(&kids(&b, c), &m.cards[c], "{ctx}: board order of {c}");
    }
    let stored = stored_card_order(env);
    assert_dense(&stored, ctx);
    for c in &m.order {
        let got: Vec<String> = stored
            .get(c)
            .map(|v| v.iter().map(|(_, id)| id.clone()).collect())
            .unwrap_or_default();
        assert_eq!(&got, &m.cards[c], "{ctx}: stored order of {c}");
    }
    let total: usize = stored.values().map(Vec::len).sum();
    assert_eq!(total, m.all_cards().len(), "{ctx}: no card lost or added");
}

#[test]
fn m1_06_story_board_order_equals_model_after_random_moves_and_restart() {
    const MOVES: usize = 30;
    for seed in seeds(1, 8) {
        let mut rng = Rng::new(seed);
        let env = TestEnv::with_project("Order", "Feature Film");
        let act = id(&env.ok("story.create_act", json!({ "title": "Act One" })));
        let acts = vec![act.clone()];
        let seqs: Vec<String> = (0..3)
            .map(|i| {
                id(&env.ok(
                    "story.create_sequence",
                    json!({ "actId": act, "title": format!("Sequence {i}") }),
                ))
            })
            .collect();
        let mut m = BoardModel {
            order: seqs
                .iter()
                .cloned()
                .chain(["parking".to_string()])
                .collect(),
            cards: BTreeMap::new(),
        };
        let mut n = 0;
        for c in m.order.clone() {
            let count = if c == "parking" {
                rng.range(0, 2)
            } else {
                rng.range(0, 4)
            };
            let mut list = Vec::new();
            for _ in 0..count {
                list.push(create_card(
                    &env,
                    container_json(&c, &seqs, &acts),
                    &format!("card {n}"),
                ));
                n += 1;
            }
            m.cards.insert(c, list);
        }
        while m.all_cards().len() < 4 {
            let c = seqs[0].clone();
            let new = create_card(&env, container_json(&c, &seqs, &acts), &format!("card {n}"));
            n += 1;
            m.cards.get_mut(&c).unwrap().push(new);
        }
        assert_board_matches(&env, &m, &format!("seed {seed} setup"));

        for step in 0..MOVES {
            let ctx = format!("seed {seed} move {step}");
            let mut pool = m.all_cards();
            rng.shuffle(&mut pool);
            let k = rng.range(1, 3.min(pool.len()));
            let mut moving: Vec<String> = pool[..k].to_vec();
            let target = rng.pick(&m.order).clone();
            let remaining: Vec<String> = m.cards[&target]
                .iter()
                .filter(|c| !moving.contains(c))
                .cloned()
                .collect();
            let (before, index) = if !remaining.is_empty() && rng.chance(70) {
                let i = rng.below(remaining.len());
                (Some(remaining[i].clone()), i)
            } else {
                (None, remaining.len())
            };
            // The selection is sent in a scrambled order; the product places it in board order.
            let request: Vec<Value> = moving.iter().map(|c| item("card", c)).collect();
            moving.sort_by_key(|c| m.board_rank(c));
            let mut args =
                json!({ "items": request, "target": container_json(&target, &seqs, &acts) });
            if let Some(b) = &before {
                args["before"] = item("card", b);
            }
            env.call("story.move_items", args.clone())
                .unwrap_or_else(|e| panic!("{ctx}: {args} failed: {} {:?}", e.code, e.detail));
            for list in m.cards.values_mut() {
                list.retain(|c| !moving.contains(c));
            }
            let list = m.cards.get_mut(&target).unwrap();
            for (j, c) in moving.into_iter().enumerate() {
                list.insert(index + j, c);
            }
            assert_board_matches(&env, &m, &ctx);
        }

        let path = env.project_path();
        let env = env.restart();
        env.reopen_project(&path);
        assert_board_matches(&env, &m, &format!("seed {seed} after restart"));
    }
}

// ------------------------------------------------------------- 2. undo and redo identity

/// Canonical content: every undo-tracked table, every column, ordered by id. Excluded, by the
/// same rule the undo capture uses (`openframe_persistence::undo::UNTRACKED_PREFIXES`):
/// `_*` (schema bookkeeping), `sqlite_*`, `sys_*` (undo stack, activity, session state) and
/// `search_*` (derived search index), plus virtual tables. Timestamps are compared too: undo
/// restores exact row images, so nothing is excluded for being "legitimately different".
fn canonical_dump(env: &TestEnv) -> BTreeMap<String, Vec<String>> {
    read(env, |c| {
        let tables: Vec<String> = {
            let mut stmt = c.prepare(
                "SELECT name FROM main.sqlite_master WHERE type='table'
                 AND sql NOT LIKE 'CREATE VIRTUAL TABLE%' ORDER BY name",
            )?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        let mut out = BTreeMap::new();
        for t in tables.into_iter().filter(|t| {
            !openframe_persistence::undo::UNTRACKED_PREFIXES
                .iter()
                .any(|p| t.starts_with(p))
        }) {
            let mut stmt = c.prepare(&format!("SELECT * FROM \"{t}\" ORDER BY id"))?;
            let cols = stmt.column_count();
            let rows: Vec<String> = stmt
                .query_map([], |r| {
                    let vals: Vec<rusqlite::types::Value> =
                        (0..cols).map(|i| r.get(i)).collect::<Result<_, _>>()?;
                    Ok(format!("{vals:?}"))
                })?
                .collect::<Result<_, _>>()?;
            out.insert(t, rows);
        }
        Ok(out)
    })
}

fn undo_info(env: &TestEnv) -> (bool, bool) {
    let i = env.ok("history.info", json!({}));
    (
        i["canUndo"].as_bool().unwrap(),
        i["canRedo"].as_bool().unwrap(),
    )
}

/// One random, valid, user-visible edit. Returns a description for failure messages.
fn random_edit(
    env: &TestEnv,
    rng: &mut Rng,
    step: usize,
    acts: &mut Vec<String>,
    seqs: &mut Vec<String>,
    cards: &mut Vec<String>,
) -> String {
    loop {
        let containers: Vec<String> = acts
            .iter()
            .chain(seqs.iter())
            .cloned()
            .chain(["parking".to_string()])
            .collect();
        match rng.below(8) {
            0 => {
                let t = format!("Act {step}");
                acts.push(id(&env.ok("story.create_act", json!({ "title": t }))));
                return format!("create act {t}");
            }
            1 if !acts.is_empty() => {
                let a = rng.pick(acts).clone();
                let s = id(&env.ok(
                    "story.create_sequence",
                    json!({ "actId": a, "title": format!("Sequence {step}") }),
                ));
                seqs.push(s);
                return "create sequence".into();
            }
            2 | 3 => {
                let c = rng.pick(&containers).clone();
                let parent = container_json(&c, seqs, acts);
                cards.push(create_card(env, parent, &format!("Card {step}")));
                return format!("create card in {c}");
            }
            4 if !cards.is_empty() => {
                let c = rng.pick(cards).clone();
                env.ok(
                    "story.update_card",
                    json!({ "id": c, "shortDescription": format!("Edited at step {step}") }),
                );
                return format!("update card {c}");
            }
            5 if !acts.is_empty() => {
                let a = rng.pick(acts).clone();
                env.ok(
                    "story.update_act",
                    json!({ "id": a, "title": format!("Renamed at step {step}") }),
                );
                return format!("rename act {a}");
            }
            6 if !cards.is_empty() => {
                let c = rng.pick(cards).clone();
                // Always a different container, so the move is a real change.
                let (kind, parent): (String, Option<String>) = read(env, |db| {
                    db.query_row(
                        "SELECT parent_type, parent_id FROM story_scene_card WHERE id=?1",
                        [&c],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                });
                let current = if kind == "parking" {
                    kind
                } else {
                    parent.unwrap()
                };
                let others: Vec<String> =
                    containers.into_iter().filter(|x| *x != current).collect();
                if others.is_empty() {
                    continue;
                }
                let target = rng.pick(&others).clone();
                env.ok(
                    "story.move_items",
                    json!({ "items": [item("card", &c)], "target": container_json(&target, seqs, acts) }),
                );
                return format!("move card {c} to {target}");
            }
            7 if !cards.is_empty() => {
                let i = rng.below(cards.len());
                let c = cards.remove(i);
                env.ok("story.delete_items", json!({ "items": [item("card", &c)] }));
                return format!("delete card {c}");
            }
            _ => continue,
        }
    }
}

#[test]
fn m1_06_undo_all_then_redo_all_restores_exact_canonical_content() {
    for seed in seeds(2, 12) {
        let mut rng = Rng::new(seed);
        let env = TestEnv::with_project("Undo", "Feature Film");
        assert_eq!(
            undo_info(&env),
            (false, false),
            "a new project has no history"
        );
        let edits = rng.range(4, 12);
        let mut snaps = vec![canonical_dump(&env)];
        let mut log = Vec::new();
        let (mut acts, mut seqs, mut cards) = (Vec::new(), Vec::new(), Vec::new());
        for step in 0..edits {
            log.push(random_edit(
                &env, &mut rng, step, &mut acts, &mut seqs, &mut cards,
            ));
            snaps.push(canonical_dump(&env));
            assert_ne!(
                snaps[step],
                snaps[step + 1],
                "seed {seed}: edit {step} ({}) changed nothing",
                log[step]
            );
        }
        // Undo walks back through earlier states, newest first, until the history is empty.
        // Consecutive edits of the same object may share one step (typing coalescing), so each
        // undo must land on SOME earlier snapshot, strictly older than the previous one.
        let mut visited = vec![edits];
        while undo_info(&env).0 {
            assert!(
                visited.len() <= edits,
                "seed {seed}: more undo steps than edits"
            );
            env.undo();
            let now = canonical_dump(&env);
            let last = *visited.last().unwrap();
            let at = (0..last)
                .rev()
                .find(|&i| snaps[i] == now)
                .unwrap_or_else(|| {
                    panic!(
                        "seed {seed}: undo {} reached a state that never existed (edits: {log:?})",
                        visited.len()
                    )
                });
            visited.push(at);
        }
        assert_eq!(
            *visited.last().unwrap(),
            0,
            "seed {seed}: undo stopped before the start (visited {visited:?}, edits {log:?})"
        );
        // Redo retraces exactly the same states in the opposite order.
        for &want in visited.iter().rev().skip(1) {
            assert!(
                undo_info(&env).1,
                "seed {seed}: redo ran out before state {want}"
            );
            env.redo();
            assert!(
                canonical_dump(&env) == snaps[want],
                "seed {seed}: redo did not return to state {want} (edits {log:?})"
            );
        }
        assert!(!undo_info(&env).1, "seed {seed}: redo left over");
        assert!(
            canonical_dump(&env) == snaps[edits],
            "seed {seed}: final content differs"
        );
    }
}

// ------------------------------------------------------------- 3. delete and restore

/// Card rows minus the bookkeeping a restore legitimately updates (`updated_at`, `rev`,
/// `deleted_at` while in Recently Deleted), keyed by id.
fn card_rows(env: &TestEnv) -> BTreeMap<String, String> {
    read(env, |c| {
        let mut stmt = c.prepare(
            "SELECT id, episode_id, parent_type, parent_id, position, short_description,
                    scene_heading, notes, color, created_at
             FROM story_scene_card WHERE deleted_at IS NULL ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            let vals: Vec<rusqlite::types::Value> =
                (0..10).map(|i| r.get(i)).collect::<Result<_, _>>()?;
            Ok((r.get::<_, String>(0)?, format!("{vals:?}")))
        })?;
        rows.collect()
    })
}

fn sequence_rows(env: &TestEnv) -> BTreeMap<String, String> {
    read(env, |c| {
        let mut stmt = c.prepare(
            "SELECT id, act_id, position, title FROM story_sequence WHERE deleted_at IS NULL ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            let vals: Vec<rusqlite::types::Value> =
                (0..4).map(|i| r.get(i)).collect::<Result<_, _>>()?;
            Ok((r.get::<_, String>(0)?, format!("{vals:?}")))
        })?;
        rows.collect()
    })
}

fn trash_entry(env: &TestEnv, object_id: &str) -> String {
    let t = env.ok("trash.list", json!({}));
    t.as_array()
        .unwrap()
        .iter()
        .find(|r| r["objectId"] == object_id)
        .unwrap_or_else(|| panic!("{object_id} not in Recently Deleted: {t}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn m1_06_delete_then_restore_returns_same_ids_positions_and_text() {
    for seed in seeds(3, 12) {
        let mut rng = Rng::new(seed);
        let env = TestEnv::with_project("Restore", "Feature Film");
        let act = id(&env.ok("story.create_act", json!({ "title": "Act One" })));
        let acts = vec![act.clone()];
        let seqs: Vec<String> = (0..2)
            .map(|i| {
                id(&env.ok(
                    "story.create_sequence",
                    json!({ "actId": act, "title": format!("Sequence {i}") }),
                ))
            })
            .collect();
        let mut all = Vec::new();
        for (ci, c) in seqs
            .iter()
            .cloned()
            .chain(["parking".to_string()])
            .enumerate()
        {
            let count = if c == "parking" {
                rng.range(0, 2)
            } else {
                rng.range(2, 6)
            };
            for k in 0..count {
                all.push(create_card(
                    &env,
                    container_json(&c, &seqs, &acts),
                    &format!("Card {ci}.{k}"),
                ));
            }
        }
        let original = card_rows(&env);
        let original_seqs = sequence_rows(&env);
        let original_order = stored_card_order(&env);

        // (a) Cards deleted one at a time and restored newest first come back exactly.
        let mut pool = all.clone();
        rng.shuffle(&mut pool);
        let victims: Vec<String> = pool[..rng.range(1, 4.min(pool.len()))].to_vec();
        // In between, every container shows the original order minus the cards still deleted.
        // (A soft-deleted card keeps its row and position, so live positions may have gaps
        // until the container is next reordered; they stay unique and increasing.)
        let check_between = |deleted: &[String], ctx: &str| {
            let now = stored_card_order(&env);
            assert_unique_increasing(&now, ctx);
            for (container, rows) in &original_order {
                let want: Vec<&String> = rows
                    .iter()
                    .map(|(_, id)| id)
                    .filter(|id| !deleted.contains(id))
                    .collect();
                let got: Vec<&String> = now
                    .get(container)
                    .map(|v| v.iter().map(|(_, id)| id).collect())
                    .unwrap_or_default();
                assert_eq!(got, want, "{ctx}: order in {container}");
            }
        };
        let mut entries = Vec::new();
        for (i, v) in victims.iter().enumerate() {
            env.ok("story.delete_items", json!({ "items": [item("card", v)] }));
            check_between(&victims[..=i], &format!("seed {seed} delete {i}"));
            entries.push(trash_entry(&env, v));
        }
        for (i, e) in entries.iter().enumerate().rev() {
            env.ok("trash.restore", json!({ "id": e }));
            check_between(&victims[..i], &format!("seed {seed} restore {i}"));
        }
        assert_eq!(
            card_rows(&env),
            original,
            "seed {seed}: newest-first restore of {victims:?}"
        );

        // (b) A multi-card delete restored in any order brings back the same ids, each in its
        // own container, with the untouched cards still in their order and dense positions.
        rng.shuffle(&mut pool);
        let victims: Vec<String> = pool[..rng.range(2, 4.min(pool.len()))].to_vec();
        let request: Vec<Value> = victims.iter().map(|v| item("card", v)).collect();
        env.ok("story.delete_items", json!({ "items": request }));
        let mut entries: Vec<String> = victims.iter().map(|v| trash_entry(&env, v)).collect();
        rng.shuffle(&mut entries);
        for e in &entries {
            env.ok("trash.restore", json!({ "id": e }));
        }
        let after = stored_card_order(&env);
        assert_dense(&after, &format!("seed {seed} any-order restore"));
        assert_eq!(
            after.keys().collect::<Vec<_>>(),
            original_order.keys().collect::<Vec<_>>(),
            "seed {seed}: containers"
        );
        for (container, rows) in &original_order {
            let mut want: Vec<&String> = rows.iter().map(|(_, id)| id).collect();
            let mut got: Vec<&String> = after[container].iter().map(|(_, id)| id).collect();
            let untouched = |v: &Vec<&String>| -> Vec<String> {
                v.iter()
                    .filter(|id| !victims.contains(id))
                    .map(|s| s.to_string())
                    .collect()
            };
            assert_eq!(
                untouched(&got),
                untouched(&want),
                "seed {seed}: untouched order in {container}"
            );
            want.sort();
            got.sort();
            assert_eq!(got, want, "seed {seed}: same cards back in {container}");
        }
        assert!(
            env.ok("trash.list", json!({}))
                .as_array()
                .unwrap()
                .is_empty(),
            "seed {seed}: Recently Deleted is empty again"
        );
        // Part (c) starts from the board as it is now.
        let original = card_rows(&env);

        // (c) A whole sequence (with its cards) deleted and restored comes back in place.
        let s = rng.pick(&seqs).clone();
        env.ok(
            "story.delete_sequence",
            json!({ "id": s, "mode": "deleteAll" }),
        );
        assert!(!kids(&env.ok("story.board", json!({})), &act).contains(&s));
        env.ok("trash.restore", json!({ "id": trash_entry(&env, &s) }));
        assert_eq!(
            sequence_rows(&env),
            original_seqs,
            "seed {seed}: sequence back in place"
        );
        assert_eq!(
            card_rows(&env),
            original,
            "seed {seed}: its cards back with the same ids, order and text"
        );
    }
}

// ------------------------------------------------------------- 4. import, export, import

const PLACES: &[&str] = &[
    "BUS DEPOT",
    "RAILWAY PLATFORM",
    "MEERA'S KITCHEN",
    "POLICE STATION",
    "ROOFTOP",
    "OLD MILL ROAD",
];
const TIMES: &[&str] = &["NIGHT", "DAY", "DAWN", "CONTINUOUS", "LATER"];
const PREFIXES: &[&str] = &["INT.", "EXT.", "INT./EXT."];
const ACTIONS: &[&str] = &[
    "Rain hammers the roof of the depot.",
    "Meera counts the coins twice, then a third time.",
    "A bus pulls away & leaves her behind.",
    "The lights flicker, then die.",
    "Arjun's phone buzzes on the table. Nobody answers.",
    "Somewhere a dog barks — once.",
];
const NAMES: &[&str] = &["MEERA", "ARJUN", "RAVI", "ANJALI", "DR. RAO"];
const LINES: &[&str] = &[
    "You said you would never come back.",
    "Not tonight.",
    "Where is the last bus?",
    "I kept your letters — all of them.",
    "Fine & dandy, thanks.",
    "Then why are you still here?",
];
const PARENS: &[&str] = &["(quietly)", "(beat)", "(into phone)"];
const TRANSITIONS: &[&str] = &["CUT TO:", "DISSOLVE TO:", "SMASH CUT TO:"];

/// Scene heading plus (element type, text) pairs: the semantics an import must keep.
type Semantics = Vec<(String, Vec<(String, String)>)>;

/// A random screenplay as Fountain source and the semantics it encodes.
fn random_screenplay(rng: &mut Rng, title: &str) -> (String, Semantics) {
    let mut src = format!("Title: {title}\nAuthor: Property Test\n\n");
    let mut sem = Vec::new();
    for _ in 0..rng.range(1, 6) {
        let heading = format!(
            "{} {} — {}",
            rng.pick(PREFIXES),
            rng.pick(PLACES),
            rng.pick(TIMES)
        );
        src.push_str(&heading);
        src.push_str("\n\n");
        let mut els: Vec<(String, String)> = Vec::new();
        let blocks = rng.range(1, 6);
        for b in 0..blocks {
            match rng.below(5) {
                0 | 1 => {
                    let a = rng.pick(ACTIONS).to_string();
                    src.push_str(&format!("{a}\n\n"));
                    els.push(("action".into(), a));
                }
                2 | 3 => {
                    let who = rng.pick(NAMES).to_string();
                    src.push_str(&format!("{who}\n"));
                    els.push(("character".into(), who));
                    if rng.chance(40) {
                        let p = rng.pick(PARENS).to_string();
                        src.push_str(&format!("{p}\n"));
                        els.push(("parenthetical".into(), p));
                    }
                    let line = rng.pick(LINES).to_string();
                    src.push_str(&format!("{line}\n\n"));
                    els.push(("dialogue".into(), line));
                }
                _ if b + 1 == blocks => {
                    let t = rng.pick(TRANSITIONS).to_string();
                    src.push_str(&format!("{t}\n\n"));
                    els.push(("transition".into(), t));
                }
                _ => {
                    let a = rng.pick(ACTIONS).to_string();
                    src.push_str(&format!("{a}\n\n"));
                    els.push(("action".into(), a));
                }
            }
        }
        sem.push((heading, els));
    }
    (src, sem)
}

fn doc_semantics(doc: &ScreenplayDoc) -> Semantics {
    doc.scenes
        .iter()
        .map(|s| {
            (
                s.heading.clone(),
                s.elements
                    .iter()
                    .map(|e| (e.kind.as_str().to_string(), e.text.clone()))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn m1_06_fountain_and_fdx_round_trips_keep_scenes_headings_elements_and_text() {
    let fdx_opts = FdxOptions {
        scene_numbers: false,
        include_notes: false,
        revision_marks: false,
        title_page: true,
    };
    for seed in seeds(4, 300) {
        let mut rng = Rng::new(seed);
        let title = format!("Seed {seed}");
        let (src, want) = random_screenplay(&mut rng, &title);
        let first = fountain::parse(&src)
            .unwrap_or_else(|e| panic!("seed {seed}: {} {:?}\n{src}", e.code, e.detail))
            .doc;
        assert_eq!(doc_semantics(&first), want, "seed {seed}: import\n{src}");
        assert_eq!(first.headed_scene_count(), want.len(), "seed {seed}");
        assert_eq!(
            first.title_page.title(),
            Some(title.as_str()),
            "seed {seed}"
        );

        // Fountain: import, export, import.
        let text = fountain::write(&first, false, false);
        let again = fountain::parse(&text).unwrap().doc;
        assert_eq!(doc_semantics(&again), want, "seed {seed}: fountain\n{text}");
        assert_eq!(
            again.title_page.title(),
            Some(title.as_str()),
            "seed {seed}"
        );
        assert_eq!(
            fountain::write(&again, false, false),
            text,
            "seed {seed}: second Fountain export is byte-stable"
        );

        // FDX: import, export, import.
        let xml = fdx::write(&first, &fdx_opts);
        let from_fdx = fdx::parse(xml.as_bytes())
            .unwrap_or_else(|e| panic!("seed {seed}: {} {:?}", e.code, e.detail))
            .doc;
        assert_eq!(doc_semantics(&from_fdx), want, "seed {seed}: fdx\n{xml}");
        assert_eq!(
            // FDX title pages print the title in capitals (Final Draft convention, `fdx::write`).
            from_fdx.title_page.title().map(str::to_uppercase),
            Some(title.to_uppercase()),
            "seed {seed}"
        );
        assert_eq!(
            fdx::write(&from_fdx, &fdx_opts),
            xml,
            "seed {seed}: second FDX export is byte-stable"
        );

        // Across formats: FDX back to Fountain keeps the same screenplay.
        let cross = fountain::parse(&fountain::write(&from_fdx, false, false))
            .unwrap()
            .doc;
        assert_eq!(doc_semantics(&cross), want, "seed {seed}: fdx → fountain");
    }
}

/// Scenes of a stored draft as the Screenplay workspace shows them.
fn draft_semantics(env: &TestEnv, draft: &str) -> Semantics {
    let doc = env.ok("screenplay.document", json!({ "draftId": draft }));
    doc["scenes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["heading"].as_str().unwrap().to_string(),
                s["elements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| {
                        (
                            e["elementType"].as_str().unwrap().to_string(),
                            e["text"].as_str().unwrap().to_string(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn import(env: &TestEnv, path: &str, mode: &str) -> Value {
    let p = env.ok("screenplay.import_preview", json!({ "path": path }));
    env.ok(
        "screenplay.import_apply",
        json!({ "previewId": p["previewId"], "mode": mode }),
    )
}

#[test]
fn m1_06_app_import_export_reimport_keeps_scenes_headings_elements_and_text() {
    for seed in seeds(5, 10) {
        let mut rng = Rng::new(seed);
        let (src, want) = random_screenplay(&mut rng, &format!("Seed {seed}"));
        let env = TestEnv::with_project("Interchange", "Feature Film");
        let path = env.write_file("source.fountain", src.as_bytes());
        let first = import(&env, &path, "new_screenplay");
        let draft = first["draftId"].as_str().unwrap().to_string();
        assert_eq!(
            draft_semantics(&env, &draft),
            want,
            "seed {seed}: imported draft\n{src}"
        );
        let out = env.dir.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        for fmt in ["fountain", "fdx"] {
            let dest = out.join(format!("export.{fmt}"));
            env.ok(
                "screenplay.export",
                json!({ "draftId": draft, "format": fmt, "path": dest.to_string_lossy() }),
            );
            let back = import(&env, &dest.to_string_lossy(), "new_draft");
            let back_draft = back["draftId"].as_str().unwrap();
            assert_ne!(back_draft, draft, "seed {seed}: re-import is a new draft");
            assert_eq!(
                draft_semantics(&env, back_draft),
                want,
                "seed {seed}: {fmt} export re-imported\n{}",
                std::fs::read_to_string(&dest).unwrap()
            );
        }
        // Exporting never changed the original draft.
        assert_eq!(draft_semantics(&env, &draft), want, "seed {seed}: original");
    }
}
