//! Draft comparison (FSD §22, UX §3.15) and revision change lists (FSD §24.7).
//!
//! Read-only: never merges or edits (FSD §22.5). Scenes are matched by scene
//! identity (`lineage_id`) first; unmatched scenes fall back to heading
//! heuristics, and any match that is not certain is flagged `ambiguous`
//! instead of being silently assumed (FSD §22.6).

use std::collections::{BTreeMap, HashMap};

use openframe_domain::enums::ElementType;
use openframe_domain::{Actor, AppError, AppResult, Capability};
use serde::{Deserialize, Serialize};
use similar::{Algorithm, ChangeTag, DiffOp, TextDiff, capture_diff_slices};
use ts_rs::TS;

use super::{ScreenplayDraftRef, ScreenplaySceneDto, load_draft, load_scenes};
use crate::core::AppCore;
use crate::registry::Registry;

pub fn register(r: &mut Registry) {
    r.query("screenplay.compare", compare);
    r.query("screenplay.revision_changes", revision_changes);
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayCompareArgs {
    /// "Before" draft.
    pub draft_a: String,
    /// "After" draft.
    pub draft_b: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ScreenplaySceneChangeKind {
    Added,
    Removed,
    Moved,
    Changed,
    Unchanged,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneSide {
    pub scene_id: String,
    pub number: u32,
    pub heading: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ScreenplayDiffSegmentKind {
    Equal,
    Removed,
    Added,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDiffSegment {
    pub kind: ScreenplayDiffSegmentKind,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDiffLine {
    pub element_type: ElementType,
    pub text: String,
    /// Word-level highlighting for changed lines.
    pub segments: Vec<ScreenplayDiffSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum ScreenplayDiffRowKind {
    Equal,
    Removed,
    Added,
    Changed,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDiffRow {
    pub kind: ScreenplayDiffRowKind,
    pub left: Option<ScreenplayDiffLine>,
    pub right: Option<ScreenplayDiffLine>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplaySceneChange {
    pub kind: ScreenplaySceneChangeKind,
    pub a: Option<ScreenplaySceneSide>,
    pub b: Option<ScreenplaySceneSide>,
    /// "identity" (same scene lineage) or "heading" (heuristic); None when unmatched.
    pub matched_by: Option<String>,
    /// The match is a guess that the writer should confirm (FSD §22.6).
    pub ambiguous: bool,
    /// Also moved relative to other scenes (a changed scene can be moved too).
    pub moved: bool,
    pub lines_changed: u32,
    /// Side-by-side text comparison (empty for unchanged scenes).
    pub rows: Vec<ScreenplayDiffRow>,
}

#[derive(Debug, Clone, Default, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCompareSummary {
    pub added: u32,
    pub removed: u32,
    pub moved: u32,
    pub changed: u32,
    pub unchanged: u32,
    pub ambiguous: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayCompareResult {
    pub draft_a: ScreenplayDraftRef,
    pub draft_b: ScreenplayDraftRef,
    pub summary: ScreenplayCompareSummary,
    pub scenes: Vec<ScreenplaySceneChange>,
}

// ------------------------------------------------------------------ matching

fn norm_heading(h: &str) -> String {
    h.to_uppercase()
        .replace(['—', '–'], "-")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" - ", "-")
}

type Line = (ElementType, String);

fn lines(s: &ScreenplaySceneDto) -> Vec<Line> {
    let mut v = vec![(ElementType::SceneHeading, s.heading.clone())];
    v.extend(s.elements.iter().map(|e| (e.element_type, e.text.clone())));
    v
}

fn side(s: &ScreenplaySceneDto) -> ScreenplaySceneSide {
    ScreenplaySceneSide {
        scene_id: s.id.clone(),
        number: s.number,
        heading: s.heading.clone(),
    }
}

struct Pair {
    a: usize,
    b: usize,
    by: &'static str,
    ambiguous: bool,
}

/// Match scenes of A and B: identity first, then heading heuristics.
fn match_scenes(a: &[ScreenplaySceneDto], b: &[ScreenplaySceneDto]) -> Vec<Pair> {
    let mut pairs = Vec::new();
    let mut a_used = vec![false; a.len()];
    let mut b_used = vec![false; b.len()];
    // 1. Identity. Duplicate lineages on either side are matched in order but flagged.
    let mut a_by_lineage: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, s) in a.iter().enumerate() {
        a_by_lineage
            .entry(s.lineage_id.as_str())
            .or_default()
            .push(i);
    }
    let mut b_by_lineage: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (j, s) in b.iter().enumerate() {
        b_by_lineage
            .entry(s.lineage_id.as_str())
            .or_default()
            .push(j);
    }
    for (lineage, ais) in &a_by_lineage {
        if let Some(bjs) = b_by_lineage.get(lineage) {
            let dup = ais.len() > 1 || bjs.len() > 1;
            for (i, j) in ais.iter().zip(bjs.iter()) {
                pairs.push(Pair {
                    a: *i,
                    b: *j,
                    by: "identity",
                    ambiguous: dup,
                });
                a_used[*i] = true;
                b_used[*j] = true;
            }
        }
    }
    // 2. Heading heuristics for what is left (imported/legacy drafts).
    let mut a_by_heading: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, s) in a.iter().enumerate().filter(|(i, _)| !a_used[*i]) {
        let h = norm_heading(&s.heading);
        if !h.is_empty() {
            a_by_heading.entry(h).or_default().push(i);
        }
    }
    let mut b_by_heading: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (j, s) in b.iter().enumerate().filter(|(j, _)| !b_used[*j]) {
        let h = norm_heading(&s.heading);
        if !h.is_empty() {
            b_by_heading.entry(h).or_default().push(j);
        }
    }
    for (h, ais) in &a_by_heading {
        if let Some(bjs) = b_by_heading.get(h) {
            let ambiguous = ais.len() > 1 || bjs.len() > 1;
            for (i, j) in ais.iter().zip(bjs.iter()) {
                pairs.push(Pair {
                    a: *i,
                    b: *j,
                    by: "heading",
                    ambiguous,
                });
            }
        }
    }
    pairs.sort_by_key(|p| p.b);
    pairs
}

/// Indices (into `seq`) of a longest strictly increasing subsequence.
fn lis(seq: &[usize]) -> Vec<usize> {
    let n = seq.len();
    let mut tails: Vec<usize> = Vec::new(); // indices into seq
    let mut prev: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        let pos = tails.partition_point(|&t| seq[t] < seq[i]);
        if pos > 0 {
            prev[i] = Some(tails[pos - 1]);
        }
        if pos == tails.len() {
            tails.push(i);
        } else {
            tails[pos] = i;
        }
    }
    let mut out = Vec::new();
    let mut cur = tails.last().copied();
    while let Some(i) = cur {
        out.push(i);
        cur = prev[i];
    }
    out.reverse();
    out
}

fn word_segments(old: &str, new: &str) -> (Vec<ScreenplayDiffSegment>, Vec<ScreenplayDiffSegment>) {
    let diff = TextDiff::from_words(old, new);
    let mut left: Vec<ScreenplayDiffSegment> = Vec::new();
    let mut right: Vec<ScreenplayDiffSegment> = Vec::new();
    let push = |v: &mut Vec<ScreenplayDiffSegment>, kind: ScreenplayDiffSegmentKind, t: &str| {
        if let Some(last) = v.last_mut()
            && last.kind == kind
        {
            last.text.push_str(t);
            return;
        }
        v.push(ScreenplayDiffSegment {
            kind,
            text: t.to_string(),
        });
    };
    for ch in diff.iter_all_changes() {
        let t = ch.value();
        match ch.tag() {
            ChangeTag::Equal => {
                push(&mut left, ScreenplayDiffSegmentKind::Equal, t);
                push(&mut right, ScreenplayDiffSegmentKind::Equal, t);
            }
            ChangeTag::Delete => push(&mut left, ScreenplayDiffSegmentKind::Removed, t),
            ChangeTag::Insert => push(&mut right, ScreenplayDiffSegmentKind::Added, t),
        }
    }
    (left, right)
}

fn plain(l: &Line, kind: ScreenplayDiffSegmentKind) -> ScreenplayDiffLine {
    ScreenplayDiffLine {
        element_type: l.0,
        text: l.1.clone(),
        segments: vec![ScreenplayDiffSegment {
            kind,
            text: l.1.clone(),
        }],
    }
}

/// Side-by-side rows for two scenes; returns (rows, changed line count).
pub fn diff_rows(a: &[Line], b: &[Line]) -> (Vec<ScreenplayDiffRow>, u32) {
    let keys_a: Vec<String> = a
        .iter()
        .map(|(t, x)| format!("{}\u{1f}{x}", t.as_str()))
        .collect();
    let keys_b: Vec<String> = b
        .iter()
        .map(|(t, x)| format!("{}\u{1f}{x}", t.as_str()))
        .collect();
    let ops = capture_diff_slices(Algorithm::Myers, &keys_a, &keys_b);
    let mut rows = Vec::new();
    let mut changed = 0u32;
    for op in ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for k in 0..len {
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Equal,
                        left: Some(plain(&a[old_index + k], ScreenplayDiffSegmentKind::Equal)),
                        right: Some(plain(&b[new_index + k], ScreenplayDiffSegmentKind::Equal)),
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for k in 0..old_len {
                    changed += 1;
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Removed,
                        left: Some(plain(&a[old_index + k], ScreenplayDiffSegmentKind::Removed)),
                        right: None,
                    });
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for k in 0..new_len {
                    changed += 1;
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Added,
                        left: None,
                        right: Some(plain(&b[new_index + k], ScreenplayDiffSegmentKind::Added)),
                    });
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let paired = old_len.min(new_len);
                for k in 0..paired {
                    changed += 1;
                    let (l, r) = (&a[old_index + k], &b[new_index + k]);
                    let (ls, rs) = word_segments(&l.1, &r.1);
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Changed,
                        left: Some(ScreenplayDiffLine {
                            element_type: l.0,
                            text: l.1.clone(),
                            segments: ls,
                        }),
                        right: Some(ScreenplayDiffLine {
                            element_type: r.0,
                            text: r.1.clone(),
                            segments: rs,
                        }),
                    });
                }
                for k in paired..old_len {
                    changed += 1;
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Removed,
                        left: Some(plain(&a[old_index + k], ScreenplayDiffSegmentKind::Removed)),
                        right: None,
                    });
                }
                for k in paired..new_len {
                    changed += 1;
                    rows.push(ScreenplayDiffRow {
                        kind: ScreenplayDiffRowKind::Added,
                        left: None,
                        right: Some(plain(&b[new_index + k], ScreenplayDiffSegmentKind::Added)),
                    });
                }
            }
        }
    }
    (rows, changed)
}

/// Compare two ordered scene lists (pure; unit-testable).
pub fn compare_scenes(
    a: &[ScreenplaySceneDto],
    b: &[ScreenplaySceneDto],
) -> (ScreenplayCompareSummary, Vec<ScreenplaySceneChange>) {
    let pairs = match_scenes(a, b);
    let a_seq: Vec<usize> = pairs.iter().map(|p| p.a).collect();
    let in_order: std::collections::HashSet<usize> = lis(&a_seq).into_iter().collect();
    let mut summary = ScreenplayCompareSummary::default();
    // Changes in B order; removed A scenes are placed after their nearest matched predecessor.
    let mut by_b: Vec<ScreenplaySceneChange> = Vec::new();
    let mut a_to_row: HashMap<usize, usize> = HashMap::new();
    let pair_by_b: HashMap<usize, (usize, &Pair)> = pairs
        .iter()
        .enumerate()
        .map(|(k, p)| (p.b, (k, p)))
        .collect();
    for (j, sb) in b.iter().enumerate() {
        match pair_by_b.get(&j) {
            Some((k, p)) => {
                let sa = &a[p.a];
                let moved = !in_order.contains(k);
                let (la, lb) = (lines(sa), lines(sb));
                let same = la == lb;
                let (rows, changed) = if same {
                    (vec![], 0)
                } else {
                    diff_rows(&la, &lb)
                };
                let kind = if !same {
                    ScreenplaySceneChangeKind::Changed
                } else if moved {
                    ScreenplaySceneChangeKind::Moved
                } else {
                    ScreenplaySceneChangeKind::Unchanged
                };
                a_to_row.insert(p.a, by_b.len());
                by_b.push(ScreenplaySceneChange {
                    kind,
                    a: Some(side(sa)),
                    b: Some(side(sb)),
                    matched_by: Some(p.by.to_string()),
                    ambiguous: p.ambiguous,
                    moved,
                    lines_changed: changed,
                    rows,
                });
            }
            None => {
                let lb = lines(sb);
                let (rows, changed) = diff_rows(&[], &lb);
                by_b.push(ScreenplaySceneChange {
                    kind: ScreenplaySceneChangeKind::Added,
                    a: None,
                    b: Some(side(sb)),
                    matched_by: None,
                    ambiguous: false,
                    moved: false,
                    lines_changed: changed,
                    rows,
                });
            }
        }
    }
    // Insert removed scenes.
    let matched_a: std::collections::HashSet<usize> = pairs.iter().map(|p| p.a).collect();
    let mut inserts: BTreeMap<usize, Vec<ScreenplaySceneChange>> = BTreeMap::new(); // after row index (usize::MAX = at start)
    for (i, sa) in a.iter().enumerate() {
        if matched_a.contains(&i) {
            continue;
        }
        let anchor = (0..i).rev().find_map(|p| a_to_row.get(&p).copied());
        let la = lines(sa);
        let (rows, changed) = diff_rows(&la, &[]);
        let change = ScreenplaySceneChange {
            kind: ScreenplaySceneChangeKind::Removed,
            a: Some(side(sa)),
            b: None,
            matched_by: None,
            ambiguous: false,
            moved: false,
            lines_changed: changed,
            rows,
        };
        inserts
            .entry(anchor.map(|x| x + 1).unwrap_or(0))
            .or_default()
            .push(change);
    }
    let mut out = Vec::with_capacity(by_b.len() + inserts.values().map(|v| v.len()).sum::<usize>());
    for (idx, change) in by_b.into_iter().enumerate() {
        if let Some(v) = inserts.remove(&idx) {
            out.extend(v);
        }
        out.push(change);
    }
    for (_, v) in inserts {
        out.extend(v);
    }
    for c in &out {
        match c.kind {
            ScreenplaySceneChangeKind::Added => summary.added += 1,
            ScreenplaySceneChangeKind::Removed => summary.removed += 1,
            ScreenplaySceneChangeKind::Moved => summary.moved += 1,
            ScreenplaySceneChangeKind::Changed => summary.changed += 1,
            ScreenplaySceneChangeKind::Unchanged => summary.unchanged += 1,
        }
        if c.ambiguous {
            summary.ambiguous += 1;
        }
    }
    (summary, out)
}

fn compare(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayCompareArgs,
) -> AppResult<ScreenplayCompareResult> {
    actor.require(Capability::View, "compare drafts")?;
    if a.draft_a == a.draft_b {
        return Err(AppError::invalid_input(
            "Choose two different drafts to compare.",
        ));
    }
    core.project()?.store.read(|c| {
        let da = load_draft(c, &a.draft_a)?;
        let db = load_draft(c, &a.draft_b)?;
        if da.screenplay_id != db.screenplay_id {
            return Err(AppError::invalid_input(
                "Both drafts must belong to the same screenplay.",
            ));
        }
        let (summary, scenes) = compare_scenes(&load_scenes(c, &da.id)?, &load_scenes(c, &db.id)?);
        Ok(ScreenplayCompareResult {
            draft_a: ScreenplayDraftRef {
                id: da.id,
                name: da.name,
            },
            draft_b: ScreenplayDraftRef {
                id: db.id,
                name: db.name,
            },
            summary,
            scenes,
        })
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenplayRevisionChangesArgs {
    pub draft_id: String,
}

/// Scenes a revision changed relative to the draft it was created from (FSD §24.7).
fn revision_changes(
    core: &AppCore,
    actor: &Actor,
    a: ScreenplayRevisionChangesArgs,
) -> AppResult<ScreenplayCompareResult> {
    actor.require(Capability::View, "view revisions")?;
    core.project()?.store.read(|c| {
        let d = load_draft(c, &a.draft_id)?;
        let src_id = d.created_from.clone().ok_or_else(|| {
            AppError::invalid_input("This draft was not created from another draft.")
        })?;
        let src: (String, String) = c.query_row(
            "SELECT id, name FROM screenplay_draft WHERE id=?1",
            [&src_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let (summary, scenes) = compare_scenes(&load_scenes(c, &src.0)?, &load_scenes(c, &d.id)?);
        Ok(ScreenplayCompareResult {
            draft_a: ScreenplayDraftRef {
                id: src.0,
                name: src.1,
            },
            draft_b: ScreenplayDraftRef {
                id: d.id,
                name: d.name,
            },
            summary,
            scenes: scenes
                .into_iter()
                .filter(|s| s.kind != ScreenplaySceneChangeKind::Unchanged)
                .collect(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::screenplay::ScreenplayElementDto;

    fn scene(id: &str, lineage: &str, heading: &str, body: &[&str]) -> ScreenplaySceneDto {
        ScreenplaySceneDto {
            id: id.into(),
            lineage_id: lineage.into(),
            number: 0,
            heading: heading.into(),
            notes: None,
            synopsis: None,
            story_day: None,
            time_note: None,
            source_scene_card_id: None,
            elements: body
                .iter()
                .enumerate()
                .map(|(i, t)| ScreenplayElementDto {
                    id: format!("{id}-{i}"),
                    element_type: ElementType::Action,
                    text: t.to_string(),
                    dual: false,
                })
                .collect(),
            rev: 1,
        }
    }

    #[test]
    fn categories() {
        let a = vec![
            scene("a1", "L1", "INT. A", &["x"]),
            scene("a2", "L2", "INT. B", &["y"]),
            scene("a3", "L3", "INT. C", &["z"]),
            scene("a4", "L4", "INT. D", &["w"]),
        ];
        // B: L3 moved to front, L2 changed, L4 removed, new scene added.
        let b = vec![
            scene("b3", "L3", "INT. C", &["z"]),
            scene("b1", "L1", "INT. A", &["x"]),
            scene("b2", "L2", "INT. B", &["y changed"]),
            scene("b5", "L5", "EXT. E", &["v"]),
        ];
        let (s, rows) = compare_scenes(&a, &b);
        assert_eq!(
            (s.added, s.removed, s.changed, s.unchanged, s.moved),
            (1, 1, 1, 1, 1),
            "{s:?}"
        );
        let changed = rows
            .iter()
            .find(|r| r.kind == ScreenplaySceneChangeKind::Changed)
            .unwrap();
        assert_eq!(changed.lines_changed, 1);
        assert_eq!(
            changed
                .rows
                .iter()
                .filter(|r| r.kind == ScreenplayDiffRowKind::Changed)
                .count(),
            1
        );
    }

    #[test]
    fn heading_fallback_and_ambiguity() {
        let a = vec![
            scene("a1", "X1", "INT. HOUSE — DAY", &["a"]),
            scene("a2", "X2", "EXT. ROAD", &["b"]),
            scene("a3", "X3", "EXT. ROAD", &["c"]),
        ];
        let b = vec![
            scene("b1", "Y1", "int. house - day", &["a"]),
            scene("b2", "Y2", "EXT. ROAD", &["b"]),
            scene("b3", "Y3", "EXT. ROAD", &["c2"]),
        ];
        let (s, rows) = compare_scenes(&a, &b);
        assert_eq!(s.unchanged + s.changed, 3);
        assert_eq!(rows[0].matched_by.as_deref(), Some("heading"));
        assert!(!rows[0].ambiguous);
        assert!(
            rows[1].ambiguous && rows[2].ambiguous,
            "duplicate headings are flagged, never silently assumed"
        );
        assert_eq!(s.ambiguous, 2);
    }

    #[test]
    fn lis_finds_stable_order() {
        let idx = lis(&[2, 0, 1, 3]);
        let vals: Vec<usize> = idx.iter().map(|&i| [2, 0, 1, 3][i]).collect();
        assert_eq!(vals, vec![0, 1, 3]);
    }
}
