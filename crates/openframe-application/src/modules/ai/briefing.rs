//! Briefing builders (AI harness rebuild plan, sections 5.5 and 5.6).
//!
//! "Code researches, the model writes": a briefing is plain labelled text that code builds from
//! the draft, so the model never has to count, rank or look things up. Every function is
//! deterministic and bounded by a budget in characters (about four characters to a token).
//!
//! Rules every briefing keeps:
//! - A line that states something about the script carries a scene number ("Scene 4: ..."), so
//!   an answer can cite it. A line from a Story record starts "Record:", a line about the
//!   briefing itself starts "Note:", a heading starts "## ".
//! - A long line is cut at a word with an ellipsis, and a closing note says how many were cut and
//!   how many lines did not fit.
//! - A tie such as "sister" appears only when it is a Character Relationship record. Pronouns and
//!   gender are never inferred.
//! - An exchange is counted from adjacent dialogue blocks inside a scene: a block by A followed
//!   by the next block by B (A and B different) is one A to B exchange. Sharing a scene alone is
//!   not interaction.
//! - The script is read once per call (scenes, elements, Beat Board links in a handful of
//!   queries) and grouped in memory.
//!
//! Scene numbers are those of `queries::scenes`. Omitted scenes keep their number and are left
//! out of every briefing, and so is a scene with no heading and no text.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::RangeInclusive;

use openframe_domain::{AppError, AppResult};
use rusqlite::Connection;

use super::queries::{self, DraftRef};
use super::tools::dialogue_blocks;
use super::understand::{self, Alias, CharName};

pub(crate) const SCENE_BUDGET: usize = 2_600;
pub(crate) const CHARACTER_BUDGET: usize = 3_000;
pub(crate) const RELATIONSHIP_BUDGET: usize = 3_400;
pub(crate) const ACT_BUDGET: usize = 3_000;
pub(crate) const MAP_BUDGET: usize = 2_000;

/// Longest single line of a briefing.
const LINE_MAX: usize = 240;
/// A quoted line of dialogue.
const QUOTE_MAX: usize = 180;
const ONE_LINER_MAX: usize = 300;

// ------------------------------------------------------------------ text helpers

fn flat(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// At most `max` characters, cut at a word with an ellipsis. The flag says whether it was cut.
fn cut(s: &str, max: usize) -> (String, bool) {
    if s.chars().count() <= max {
        return (s.to_string(), false);
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    let head = match head.rfind(' ') {
        Some(i) if head[..i].chars().count() * 2 >= max => head[..i].to_string(),
        _ => head,
    };
    let head = head.trim_end_matches(|c: char| c.is_whitespace() || ",;:-".contains(c));
    (format!("{head}\u{2026}"), true)
}

fn quote(text: &str) -> String {
    format!("\"{}\"", cut(&flat(text), QUOTE_MAX).0)
}

/// "Scene 4", "Scenes 1-3, 5". Runs of three or more are shown as a range.
fn scenes_text(ns: &[usize]) -> String {
    let ns: BTreeSet<usize> = ns.iter().copied().collect();
    let ns: Vec<usize> = ns.into_iter().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < ns.len() {
        let mut j = i;
        while j + 1 < ns.len() && ns[j + 1] == ns[j] + 1 {
            j += 1;
        }
        if j - i >= 2 {
            parts.push(format!("{}-{}", ns[i], ns[j]));
        } else {
            parts.extend(ns[i..=j].iter().map(usize::to_string));
        }
        i = j + 1;
    }
    format!(
        "Scene{} {}",
        if ns.len() == 1 { "" } else { "s" },
        parts.join(", ")
    )
}

/// First sentence of a paragraph, last sentence of a paragraph. A full stop followed by a space
/// ends a sentence, so "Dr. Rao" splits early; the cut line still reads correctly.
fn sentences(text: &str) -> Vec<String> {
    let t = flat(text);
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = t.char_indices().collect();
    for (k, (i, ch)) in chars.iter().enumerate() {
        if matches!(ch, '.' | '!' | '?') && chars.get(k + 1).is_none_or(|(_, n)| *n == ' ') {
            let end = i + ch.len_utf8();
            let s = t[start..end].trim();
            if !s.is_empty() {
                out.push(s.to_string());
            }
            start = end;
        }
    }
    let rest = t[start..].trim();
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}

/// "Scene 3: INT. OFFICE - NIGHT. The knock." for a scene list, at most 300 characters.
fn liner(n: usize, heading: &str, synopsis: &str, first_action: &str) -> String {
    let heading = flat(heading);
    let heading = heading.trim_end_matches('.');
    let gist = if synopsis.trim().is_empty() {
        flat(first_action)
    } else {
        flat(synopsis)
    };
    let mut s = format!("Scene {n}");
    if !heading.is_empty() {
        s.push_str(&format!(": {heading}"));
    }
    if !gist.is_empty() {
        s.push_str(if heading.is_empty() { ": " } else { ". " });
        s.push_str(&gist);
    }
    cut(&s, ONE_LINER_MAX).0
}

/// Collects lines up to a budget. Lines that do not fit are counted, never half written.
struct Out {
    budget: usize,
    reserve: usize,
    used: usize,
    lines: Vec<String>,
    cut: usize,
    dropped: usize,
}

impl Out {
    fn new(budget: usize) -> Out {
        Out {
            budget,
            // Room for the closing notes.
            reserve: (budget / 4).min(130),
            used: 0,
            lines: Vec::new(),
            cut: 0,
            dropped: 0,
        }
    }

    fn line(&mut self, s: impl AsRef<str>) {
        let (t, was_cut) = cut(&flat(s.as_ref()), LINE_MAX);
        let need = t.chars().count() + 1;
        if self.dropped > 0 || self.used + need > self.budget.saturating_sub(self.reserve) {
            self.dropped += 1;
            return;
        }
        self.used += need;
        self.cut += usize::from(was_cut);
        self.lines.push(t);
    }

    fn finish(mut self) -> String {
        if self.cut > 0 {
            self.lines.push(format!(
                "Note: {} long {} cut at a word and end with an ellipsis.",
                self.cut,
                if self.cut == 1 {
                    "line was"
                } else {
                    "lines were"
                }
            ));
        }
        if self.dropped > 0 {
            self.lines
                .push(format!("Note: {} more lines not shown.", self.dropped));
        }
        let text = self.lines.join("\n");
        if text.chars().count() > self.budget {
            return cut(&text, self.budget).0;
        }
        text
    }
}

// ------------------------------------------------------------------ the script

struct SceneData {
    n: usize,
    id: String,
    heading: String,
    synopsis: String,
    notes: String,
    omitted: bool,
    /// Story elements in order: notes, page breaks and the heading element left out.
    elements: Vec<(String, String)>,
    /// Dialogue blocks: (speaker key, text).
    blocks: Vec<(String, String)>,
    /// Speaker keys with their number of blocks, in order of first speaking.
    lines: Vec<(String, usize)>,
    /// Name indexes found in action text, in order, speakers excluded.
    mentioned: Vec<usize>,
}

impl SceneData {
    fn skipped(&self) -> bool {
        self.omitted || (self.heading.trim().is_empty() && self.elements.is_empty())
    }
}

struct Place {
    act: Option<usize>,
    sequence: Option<usize>,
    card: String,
    beat: Option<String>,
}

struct Beat {
    act: Option<usize>,
    text: String,
    scene: Option<usize>,
}

/// One exchange pair of two speakers inside one scene, `a` before `b` alphabetically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Interaction {
    pub a: String,
    pub b: String,
    pub scene: usize,
    pub exchanges_ab: usize,
    pub exchanges_ba: usize,
    /// Dialogue block index (in the scene) where the first and the last exchange begin.
    pub first_pos: usize,
    pub last_pos: usize,
}

struct Script {
    draft_label: String,
    title: String,
    scenes: Vec<SceneData>,
    names: Vec<CharName>,
    name_idx: HashMap<String, usize>,
    acts: Vec<(String, String)>,
    sequences: Vec<(String, Option<usize>)>,
    places: HashMap<String, Place>,
    beats: Vec<Beat>,
}

fn not_found(draft: &DraftRef, what: &str) -> AppError {
    AppError::ai("not_found", format!("{} has no {what}.", draft.label()))
}

impl Script {
    fn load(c: &Connection, draft: &DraftRef) -> AppResult<Script> {
        let mut stmt = c.prepare(
            "SELECT id, heading, coalesce(synopsis,''), coalesce(notes,''), omitted
             FROM screenplay_scene WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
        )?;
        let rows: Vec<(String, String, String, String, bool)> = stmt
            .query_map([&draft.id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;

        let mut by_scene: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut stmt = c.prepare(
            "SELECT e.scene_id, e.element_type, e.text FROM screenplay_element e
             JOIN screenplay_scene s ON s.id = e.scene_id
             WHERE s.draft_id=?1 AND s.deleted_at IS NULL
               AND e.element_type NOT IN ('note','page_break','scene_heading')
             ORDER BY s.position, s.id, e.position, e.id",
        )?;
        let mut q = stmt.query([&draft.id])?;
        while let Some(r) = q.next()? {
            let text: String = r.get(2)?;
            if !text.trim().is_empty() {
                by_scene
                    .entry(r.get(0)?)
                    .or_default()
                    .push((r.get(1)?, text));
            }
        }

        let names = understand::character_names(c, &draft.id)?;
        let name_idx: HashMap<String, usize> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.key.clone(), i))
            .collect();
        let aliases = understand::aliases(&names);
        let mut by_first: HashMap<&str, Vec<&Alias>> = HashMap::new();
        for a in &aliases {
            by_first.entry(a.words[0].as_str()).or_default().push(a);
        }

        let mut scenes = Vec::with_capacity(rows.len());
        for (i, (id, heading, synopsis, notes, omitted)) in rows.into_iter().enumerate() {
            let elements = by_scene.remove(&id).unwrap_or_default();
            let blocks = dialogue_blocks(&elements);
            let mut lines: Vec<(String, usize)> = Vec::new();
            for (who, _) in &blocks {
                match lines.iter_mut().find(|(k, _)| k == who) {
                    Some(l) => l.1 += 1,
                    None => lines.push((who.clone(), 1)),
                }
            }
            let mut mentioned: Vec<usize> = Vec::new();
            for (t, text) in &elements {
                if t != "action" && t != "custom" {
                    continue;
                }
                let toks = understand::words(text);
                for (k, tok) in toks.iter().enumerate() {
                    for a in by_first.get(tok.as_str()).into_iter().flatten() {
                        if toks[k..].starts_with(&a.words)
                            && !mentioned.contains(&a.idx)
                            && !lines.iter().any(|(who, _)| *who == names[a.idx].key)
                        {
                            mentioned.push(a.idx);
                        }
                    }
                }
            }
            scenes.push(SceneData {
                n: i + 1,
                id,
                heading,
                synopsis: flat(&synopsis),
                notes: flat(&notes),
                omitted,
                elements,
                blocks,
                lines,
                mentioned,
            });
        }

        let acts = understand::acts_of(c, draft)?;
        let act_at = |id: &str| acts.iter().position(|(a, _)| a == id);
        let mut stmt = c.prepare(
            "SELECT id, act_id FROM story_sequence WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let seq_rows: Vec<(String, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut stmt = c.prepare(
            "SELECT id, title FROM story_sequence WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let titles: HashMap<String, String> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let sequences: Vec<(String, Option<usize>)> = seq_rows
            .iter()
            .map(|(id, act)| {
                (
                    titles.get(id).cloned().unwrap_or_default(),
                    act.as_deref().and_then(act_at),
                )
            })
            .collect();
        let seq_at = |id: &str| seq_rows.iter().position(|(s, _)| s == id);

        let mut stmt = c.prepare(
            "SELECT id, parent_type, parent_id, text, converted_scene_card_id FROM story_beat
             WHERE deleted_at IS NULL ORDER BY position, id",
        )?;
        let beat_rows: Vec<(String, String, Option<String>, String, Option<String>)> = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .collect::<Result<_, _>>()?;
        let beat_text: HashMap<&str, &str> = beat_rows
            .iter()
            .map(|b| (b.0.as_str(), b.3.as_str()))
            .collect();

        let mut stmt = c.prepare(
            "SELECT id, screenplay_scene_id, parent_type, parent_id, short_description, source_beat_id
             FROM story_scene_card WHERE deleted_at IS NULL AND screenplay_scene_id IS NOT NULL
             ORDER BY position, id",
        )?;
        let card_rows: Vec<(
            String,
            String,
            String,
            Option<String>,
            String,
            Option<String>,
        )> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<Result<_, _>>()?;
        let container = |kind: &str, id: Option<&str>| -> (Option<usize>, Option<usize>) {
            match (kind, id) {
                ("act", Some(id)) => (act_at(id), None),
                ("sequence", Some(id)) => match seq_at(id) {
                    Some(s) => (sequences[s].1, Some(s)),
                    None => (None, None),
                },
                _ => (None, None),
            }
        };
        let mut places: HashMap<String, Place> = HashMap::new();
        let mut card_scene: HashMap<String, String> = HashMap::new();
        for (card_id, scene_id, kind, parent, desc, beat) in &card_rows {
            card_scene.insert(card_id.clone(), scene_id.clone());
            let (act, sequence) = container(kind, parent.as_deref());
            places.entry(scene_id.clone()).or_insert_with(|| Place {
                act,
                sequence,
                card: flat(desc),
                beat: beat
                    .as_deref()
                    .and_then(|b| beat_text.get(b))
                    .map(|t| flat(t)),
            });
        }
        let number_of: HashMap<&str, usize> = scenes.iter().map(|s| (s.id.as_str(), s.n)).collect();
        let beats: Vec<Beat> = beat_rows
            .iter()
            .map(|(_, kind, parent, text, converted)| {
                let (act, _) = container(kind, parent.as_deref());
                Beat {
                    act,
                    text: flat(text),
                    scene: converted
                        .as_deref()
                        .and_then(|c| card_scene.get(c))
                        .and_then(|s| number_of.get(s.as_str()).copied()),
                }
            })
            .collect();

        Ok(Script {
            draft_label: draft.label(),
            title: draft.screenplay_title.clone(),
            scenes,
            names,
            name_idx,
            acts,
            sequences,
            places,
            beats,
        })
    }

    fn live(&self) -> impl DoubleEndedIterator<Item = &SceneData> {
        self.scenes.iter().filter(|s| !s.skipped())
    }

    fn scene(&self, draft: &DraftRef, n: usize) -> AppResult<&SceneData> {
        n.checked_sub(1)
            .and_then(|i| self.scenes.get(i))
            .filter(|s| !s.skipped())
            .ok_or_else(|| not_found(draft, &format!("Scene {n}")))
    }

    /// Short line for a neighbouring scene: heading plus synopsis or first action sentence.
    fn gist(&self, s: &SceneData) -> String {
        let first = first_action(s);
        liner(s.n, &s.heading, &s.synopsis, &first)
    }

    fn place_text(&self, scene_id: &str) -> String {
        let Some(p) = self.places.get(scene_id) else {
            return String::new();
        };
        let mut parts = Vec::new();
        if let Some(a) = p.act {
            parts.push(format!("Act: {}", self.acts[a].1));
        }
        if let Some(s) = p.sequence {
            parts.push(format!("Sequence: {}", self.sequences[s].0));
        }
        if !p.card.is_empty() {
            parts.push(format!("Scene Card: {}", quote(&p.card)));
        }
        if let Some(b) = &p.beat {
            parts.push(format!("Beat: {}", quote(b)));
        }
        parts.join(" > ")
    }

    fn act_of(&self, s: &SceneData) -> Option<usize> {
        self.places.get(&s.id).and_then(|p| p.act)
    }

    /// Exchanges inside one scene, one entry per pair of speakers.
    fn scene_interactions(&self, s: &SceneData) -> Vec<Interaction> {
        let mut map: BTreeMap<(&str, &str), Interaction> = BTreeMap::new();
        for (i, pair) in s.blocks.windows(2).enumerate() {
            let (from, to) = (pair[0].0.as_str(), pair[1].0.as_str());
            if from == to {
                continue;
            }
            let (a, b, forward) = if from < to {
                (from, to, true)
            } else {
                (to, from, false)
            };
            let e = map.entry((a, b)).or_insert_with(|| Interaction {
                a: a.to_string(),
                b: b.to_string(),
                scene: s.n,
                exchanges_ab: 0,
                exchanges_ba: 0,
                first_pos: i,
                last_pos: i,
            });
            if forward {
                e.exchanges_ab += 1;
            } else {
                e.exchanges_ba += 1;
            }
            e.last_pos = i;
        }
        map.into_values().collect()
    }

    fn interactions(&self, range: &RangeInclusive<usize>) -> Vec<Interaction> {
        self.live()
            .filter(|s| range.contains(&s.n))
            .flat_map(|s| self.scene_interactions(s))
            .collect()
    }

    fn present_keys(&self, s: &SceneData) -> Vec<String> {
        let mut out: Vec<String> = s.lines.iter().map(|(k, _)| k.clone()).collect();
        out.extend(s.mentioned.iter().map(|i| self.names[*i].key.clone()));
        out
    }

    fn speaks_in(&self, key: &str) -> Vec<usize> {
        self.live()
            .filter(|s| s.lines.iter().any(|(k, _)| k == key))
            .map(|s| s.n)
            .collect()
    }

    fn present_in(&self, key: &str) -> Vec<usize> {
        self.live()
            .filter(|s| self.present_keys(s).iter().any(|k| k == key))
            .map(|s| s.n)
            .collect()
    }

    fn line_count(&self, key: &str) -> usize {
        self.live()
            .flat_map(|s| s.lines.iter())
            .filter(|(k, _)| k == key)
            .map(|(_, n)| n)
            .sum()
    }

    /// The character a name or first name stands for. Never a guess between several.
    fn find_character(&self, draft: &DraftRef, name: &str) -> AppResult<usize> {
        let key = queries::normalize_cue(name);
        if let Some(i) = self.name_idx.get(&key) {
            return Ok(*i);
        }
        let want = understand::words(name);
        let hits: Vec<usize> = understand::aliases(&self.names)
            .into_iter()
            .filter(|a| a.words == want)
            .map(|a| a.idx)
            .collect();
        match hits[..] {
            [one] => Ok(one),
            _ => Err(not_found(
                draft,
                &format!("character called {}", name.trim()),
            )),
        }
    }
}

/// "FADE IN:" and "CUT TO BLACK." are not what happens in a scene: a sentence in capitals only
/// (a script in a caseless alphabet has none) is passed over.
fn shouting(sentence: &str) -> bool {
    sentence.chars().any(char::is_uppercase) && !sentence.chars().any(char::is_lowercase)
}

fn action_sentences(s: &SceneData) -> impl DoubleEndedIterator<Item = String> + '_ {
    s.elements
        .iter()
        .filter(|(t, _)| t == "action" || t == "custom")
        .flat_map(|(_, text)| sentences(text))
        .filter(|x| !shouting(x))
}

fn first_action(s: &SceneData) -> String {
    action_sentences(s).next().unwrap_or_default()
}

fn last_action(s: &SceneData) -> String {
    action_sentences(s).next_back().unwrap_or_default()
}

/// "1 Scene", "10 Scenes".
fn scene_count(n: usize) -> String {
    format!("{n} Scene{}", if n == 1 { "" } else { "s" })
}

/// "1 scene", "36 scenes": the count as a plain sentence has it.
fn scene_word(n: usize) -> String {
    format!("{n} scene{}", if n == 1 { "" } else { "s" })
}

fn dialogue(who: &str, text: &str) -> String {
    format!("{who}: {}", quote(text))
}

// ------------------------------------------------------------------ scene digest

#[derive(Debug, Clone)]
pub(crate) struct SceneDigest {
    pub ordinal: usize,
    pub heading: String,
    pub synopsis: String,
    /// The writer's own scene note.
    pub notes: String,
    /// Speakers first, then names found in the action text.
    pub present: Vec<String>,
    /// Speaker and number of dialogue blocks, most first.
    pub speaking: Vec<(String, usize)>,
    pub first_line: Option<(String, String)>,
    pub last_line: Option<(String, String)>,
    /// The longest line of each of the top three speakers.
    pub longest: Vec<(String, String)>,
    pub first_action: String,
    pub last_action: String,
    /// "Act: ... > Sequence: ... > Scene Card: ..." when a Scene Card links to this scene.
    pub placement: String,
    pub prev: String,
    pub next: String,
}

impl SceneDigest {
    /// "Scene 3: heading. synopsis or first action sentence", at most 300 characters.
    pub(crate) fn one_liner(&self) -> String {
        liner(
            self.ordinal,
            &self.heading,
            &self.synopsis,
            &self.first_action,
        )
    }
}

fn digest_of(s: &Script, draft: &DraftRef, n: usize) -> AppResult<SceneDigest> {
    let sc = s.scene(draft, n)?;
    let mut speaking = sc.lines.clone();
    speaking.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let longest = speaking
        .iter()
        .take(3)
        .filter_map(|(who, _)| {
            sc.blocks
                .iter()
                .filter(|(w, _)| w == who)
                .fold(None::<&(String, String)>, |best, b| match best {
                    Some(x) if x.1.chars().count() >= b.1.chars().count() => Some(x),
                    _ => Some(b),
                })
                .cloned()
        })
        .collect();
    let neighbour = |idx: Option<&SceneData>| idx.map(|x| s.gist(x)).unwrap_or_default();
    let prev = neighbour(s.live().rfind(|x| x.n < n));
    let next = neighbour(s.live().find(|x| x.n > n));
    Ok(SceneDigest {
        ordinal: n,
        heading: flat(&sc.heading),
        synopsis: sc.synopsis.clone(),
        notes: sc.notes.clone(),
        present: s.present_keys(sc),
        speaking,
        first_line: sc.blocks.first().cloned(),
        last_line: sc.blocks.last().cloned(),
        longest,
        first_action: first_action(sc),
        last_action: last_action(sc),
        placement: s.place_text(&sc.id),
        prev,
        next,
    })
}

#[cfg(test)]
pub(crate) fn scene_digest(
    c: &Connection,
    draft: &DraftRef,
    ordinal: usize,
) -> AppResult<SceneDigest> {
    digest_of(&Script::load(c, draft)?, draft, ordinal)
}

/// The digest of every scene that counts, in order. The script is read once.
pub(crate) fn digests(c: &Connection, draft: &DraftRef) -> AppResult<Vec<SceneDigest>> {
    let s = Script::load(c, draft)?;
    s.live().map(|x| digest_of(&s, draft, x.n)).collect()
}

/// The scenes of each act in order, as ("Act 2: Title", scene numbers). Empty when the draft has no
/// acts. Scenes no Scene Card places in an act come last.
pub(crate) fn act_groups(c: &Connection, draft: &DraftRef) -> AppResult<Vec<(String, Vec<usize>)>> {
    let s = Script::load(c, draft)?;
    let mut groups: Vec<(String, Vec<usize>)> = s
        .acts
        .iter()
        .enumerate()
        .map(|(i, (_, title))| (format!("Act {}: {title}", i + 1), Vec::new()))
        .collect();
    if groups.is_empty() {
        return Ok(groups);
    }
    let mut unplaced = Vec::new();
    for x in s.live() {
        match s.act_of(x) {
            Some(a) => groups[a].1.push(x.n),
            None => unplaced.push(x.n),
        }
    }
    groups.retain(|g| !g.1.is_empty());
    if !unplaced.is_empty() {
        groups.push(("Scenes not placed in an act".to_string(), unplaced));
    }
    Ok(groups)
}

/// The digest lines of a scene briefing: heading, placement, synopsis, notes, who is present and
/// speaks, the first and last lines, the first and last action, the scenes around.
fn digest_lines(o: &mut Out, d: &SceneDigest) {
    let n = d.ordinal;
    if !d.heading.is_empty() {
        o.line(format!("Scene {n}: heading: {}", d.heading));
    }
    if !d.placement.is_empty() {
        o.line(format!(
            "Scene {n}: placement on the Beat Board: {}",
            d.placement
        ));
    }
    if !d.synopsis.is_empty() {
        o.line(format!("Scene {n}: synopsis: {}", d.synopsis));
    }
    if !d.notes.is_empty() {
        o.line(format!("Scene {n}: writer's scene note: {}", d.notes));
    }
    if !d.present.is_empty() {
        o.line(format!(
            "Scene {n}: characters present: {}",
            d.present.join(", ")
        ));
    }
    if !d.speaking.is_empty() {
        let counts: Vec<String> = d.speaking.iter().map(|(k, c)| format!("{k} {c}")).collect();
        o.line(format!(
            "Scene {n}: dialogue blocks per speaker: {}",
            counts.join(", ")
        ));
    }
    if let Some((who, text)) = &d.first_line {
        o.line(format!("Scene {n}: first line: {}", dialogue(who, text)));
    }
    if let Some((who, text)) = &d.last_line {
        o.line(format!("Scene {n}: last line: {}", dialogue(who, text)));
    }
    for (who, text) in &d.longest {
        o.line(format!("Scene {n}: longest line of {who}: {}", quote(text)));
    }
    if !d.first_action.is_empty() {
        o.line(format!("Scene {n}: first action: {}", d.first_action));
    }
    if !d.last_action.is_empty() && d.last_action != d.first_action {
        o.line(format!("Scene {n}: last action: {}", d.last_action));
    }
    if !d.prev.is_empty() {
        o.line(format!("Scene {n}: the scene before is {}", d.prev));
    }
    if !d.next.is_empty() {
        o.line(format!("Scene {n}: the scene after is {}", d.next));
    }
}

/// The scene's own action and dialogue, in the order the script has them.
fn own_lines(o: &mut Out, sc: &SceneData, n: usize) {
    let mut speaker: Option<String> = None;
    let mut paren = String::new();
    for (t, text) in &sc.elements {
        match t.as_str() {
            "character" => {
                speaker = Some(queries::normalize_cue(text));
                paren.clear();
            }
            "parenthetical" => paren = format!(" ({})", text.trim().trim_matches(['(', ')'])),
            "dialogue" => {
                let who = speaker.clone().unwrap_or_else(|| "UNKNOWN".into());
                o.line(format!("Scene {n}: {who}{paren}: {}", quote(text)));
                paren.clear();
            }
            "action" | "custom" => {
                speaker = None;
                o.line(format!("Scene {n}: action: {}", flat(text)));
            }
            other => {
                speaker = None;
                o.line(format!("Scene {n}: {other}: {}", flat(text)));
            }
        }
    }
}

/// The digest, then the scene's own lines in order until the budget is full.
pub(crate) fn scene_briefing(
    c: &Connection,
    draft: &DraftRef,
    ordinal: usize,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let d = digest_of(&s, draft, ordinal)?;
    let sc = s.scene(draft, ordinal)?;
    let mut o = Out::new(budget);
    o.line(format!("## Scene {} ({})", d.ordinal, s.draft_label));
    digest_lines(&mut o, &d);
    own_lines(&mut o, sc, d.ordinal);
    Ok(o.finish())
}

/// For a question that explains one scene: the scene's own lines FIRST, with about two thirds of
/// the budget, then the digest with the rest. What the model reads first is what the script shows.
pub(crate) fn scene_story(
    c: &Connection,
    draft: &DraftRef,
    ordinal: usize,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let d = digest_of(&s, draft, ordinal)?;
    let sc = s.scene(draft, ordinal)?;
    let lines_budget = budget * 2 / 3;
    let mut lines = Out::new(lines_budget);
    lines.line(format!("## Scene {} ({})", d.ordinal, s.draft_label));
    own_lines(&mut lines, sc, d.ordinal);
    let mut digest = Out::new(budget - lines_budget);
    digest_lines(&mut digest, &d);
    Ok(format!("{}\n{}", lines.finish(), digest.finish()))
}

// ------------------------------------------------------------------ interactions

#[cfg(test)]
pub(crate) fn interactions(
    c: &Connection,
    draft: &DraftRef,
    range: RangeInclusive<usize>,
) -> AppResult<Vec<Interaction>> {
    Ok(Script::load(c, draft)?.interactions(&range))
}

// ------------------------------------------------------------------ Story records

struct StoryCharacter {
    id: String,
    name: String,
    role: String,
    description: String,
}

fn story_characters(c: &Connection) -> AppResult<Vec<StoryCharacter>> {
    let mut stmt = c.prepare(
        "SELECT id, name, coalesce(role_label,''), coalesce(description,'') FROM story_character
         WHERE deleted_at IS NULL AND archived=0 ORDER BY position, name",
    )?;
    Ok(stmt
        .query_map([], |r| {
            Ok(StoryCharacter {
                id: r.get(0)?,
                name: r.get(1)?,
                role: r.get(2)?,
                description: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?)
}

/// Character Relationship records between live Story characters: (from id, to id, line) with the
/// line written as "Record: relationship FROM -> TO: "type"".
fn typed_ties(
    c: &Connection,
    people: &[StoryCharacter],
) -> AppResult<Vec<(String, String, String)>> {
    let mut stmt = c.prepare(
        "SELECT from_character_id, to_character_id, relationship_type, coalesce(note,'')
         FROM story_character_relationship ORDER BY position, id",
    )?;
    let rows: Vec<(String, String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let name = |id: &str| people.iter().find(|p| p.id == id).map(|p| p.name.clone());
    let mut out = Vec::new();
    for (from, to, kind, note) in rows {
        let (Some(f), Some(t)) = (name(&from), name(&to)) else {
            continue;
        };
        let mut line = format!("Record: relationship {f} -> {t}: {}", quote(&kind));
        if !note.trim().is_empty() {
            line.push_str(&format!(" (note: {})", flat(&note)));
        }
        out.push((from, to, line));
    }
    Ok(out)
}

fn cast_for(c: &Connection, story_id: Option<&str>, key: &str) -> AppResult<Vec<String>> {
    if !queries::table_exists(c, "cast_member")? {
        return Ok(Vec::new());
    }
    let mut stmt = c.prepare(
        "SELECT person_name, character_id, coalesce(character_name,'') FROM cast_member
         WHERE deleted_at IS NULL AND archived=0 ORDER BY person_name",
    )?;
    let rows: Vec<(String, Option<String>, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, id, cname)| match (id, story_id) {
            (Some(a), Some(b)) => a == b,
            (None, _) => queries::normalize_cue(cname) == key,
            _ => false,
        })
        .map(|(p, _, _)| p)
        .collect())
}

// ------------------------------------------------------------------ character briefing

/// The longest line of each of up to six equal stretches of the story.
fn key_lines(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    let n = lines.len();
    if n <= 6 {
        return lines.to_vec();
    }
    (0..6)
        .filter_map(|b| {
            lines[b * n / 6..(b + 1) * n / 6]
                .iter()
                .fold(None::<&(usize, String)>, |best, l| match best {
                    Some(x) if x.1.chars().count() >= l.1.chars().count() => Some(x),
                    _ => Some(l),
                })
                .cloned()
        })
        .collect()
}

pub(crate) fn character_briefing(
    c: &Connection,
    draft: &DraftRef,
    name: &str,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let idx = s.find_character(draft, name)?;
    let key = s.names[idx].key.clone();
    let label = s.names[idx].label.clone();
    let present = s.present_in(&key);
    let speaks = s.speaks_in(&key);
    let total = s.live().count();
    let mut o = Out::new(budget);
    o.line(format!("## Character: {label} ({})", s.draft_label));
    if let (Some(first), Some(last)) = (present.first(), present.last()) {
        o.line(format!(
            "Scene {first} is the first Scene with {label} and Scene {last} the last. {label} is in {present_n} of {total} Scene{many}.",
            present_n = present.len(),
            many = if total == 1 { "" } else { "s" }
        ));
    } else {
        o.line(format!(
            "{label} is not named in any Scene of {}.",
            s.draft_label
        ));
    }
    if !speaks.is_empty() {
        o.line(format!(
            "{}: {label} speaks, {} dialogue blocks in all.",
            scenes_text(&speaks),
            s.line_count(&key)
        ));
    }
    if !s.acts.is_empty() {
        for (a, (_, title)) in s.acts.iter().enumerate() {
            let here: Vec<usize> = present
                .iter()
                .copied()
                .filter(|n| s.act_of(&s.scenes[n - 1]) == Some(a))
                .collect();
            if !here.is_empty() {
                o.line(format!(
                    "Act \"{title}\": {label} is in {}.",
                    scenes_text(&here)
                ));
            }
        }
        let loose: Vec<usize> = present
            .iter()
            .copied()
            .filter(|n| s.act_of(&s.scenes[n - 1]).is_none())
            .collect();
        if !loose.is_empty() {
            o.line(format!("No act: {label} is in {}.", scenes_text(&loose)));
        }
    }

    // Scene partners: exchanges from adjacent dialogue blocks, presence from the scenes.
    let mut partners: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for i in s.interactions(&(1..=usize::MAX)) {
        let (other, mine, theirs) = if i.a == key {
            (&i.b, i.exchanges_ab, i.exchanges_ba)
        } else if i.b == key {
            (&i.a, i.exchanges_ba, i.exchanges_ab)
        } else {
            continue;
        };
        let e = partners.entry(other.clone()).or_default();
        e.0 += 1;
        e.1 += mine;
        e.2 += theirs;
    }
    let mut ranked: Vec<(String, usize, usize, Vec<usize>, Vec<usize>)> = Vec::new();
    for other in s.names.iter().filter(|n| n.key != key) {
        let theirs = s.present_in(&other.key);
        let shared: Vec<usize> = present
            .iter()
            .copied()
            .filter(|n| theirs.contains(n))
            .collect();
        if shared.is_empty() {
            continue;
        }
        let their_speech = s.speaks_in(&other.key);
        let spoke: Vec<usize> = speaks
            .iter()
            .copied()
            .filter(|n| their_speech.contains(n))
            .collect();
        let (out_n, in_n) = partners.get(&other.key).map_or((0, 0), |p| (p.1, p.2));
        ranked.push((other.key.clone(), out_n, in_n, spoke, shared));
    }
    ranked.sort_by(|a, b| (b.1 + b.2, b.4.len(), &a.0).cmp(&(a.1 + a.2, a.4.len(), &b.0)));
    for (other, out_n, in_n, spoke, shared) in ranked.iter().take(6) {
        o.line(format!(
            "Partner {other}: {} exchanges ({label} to {other} {out_n}, {other} to {label} {in_n}). Both speak in {}; both present in {}.",
            out_n + in_n,
            if spoke.is_empty() { "no Scene".to_string() } else { scenes_text(spoke) },
            scenes_text(shared)
        ));
    }

    // Key lines spread across the story.
    let mut lines: Vec<(usize, String)> = Vec::new();
    for sc in s.live() {
        for (who, text) in &sc.blocks {
            if *who == key {
                lines.push((sc.n, text.clone()));
            }
        }
    }
    for (n, text) in key_lines(&lines) {
        o.line(format!("Scene {n}: key line, {label}: {}", quote(&text)));
    }

    // Story records.
    let people = story_characters(c)?;
    let story = people
        .iter()
        .find(|p| queries::normalize_cue(&p.name) == key);
    if let Some(p) = story {
        if !p.role.trim().is_empty() {
            o.line(format!(
                "Record: role in the Story section: {}",
                flat(&p.role)
            ));
        }
        if !p.description.trim().is_empty() {
            o.line(format!(
                "Record: description in the Story section: {}",
                flat(&p.description)
            ));
        }
        for (from, to, line) in typed_ties(c, &people)? {
            if from == p.id || to == p.id {
                o.line(line);
            }
        }
    }
    for person in cast_for(c, story.map(|p| p.id.as_str()), &key)? {
        o.line(format!("Record: cast: {person} plays {label}."));
    }
    Ok(o.finish())
}

// ------------------------------------------------------------------ relationship briefing

pub(crate) fn relationship_briefing(
    c: &Connection,
    draft: &DraftRef,
    a: &str,
    b: &str,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let (ia, ib) = (s.find_character(draft, a)?, s.find_character(draft, b)?);
    if ia == ib {
        return Err(AppError::ai(
            "ambiguous",
            "A relationship needs two different characters.",
        ));
    }
    let (ka, kb) = (s.names[ia].key.clone(), s.names[ib].key.clone());
    let (la, lb) = (s.names[ia].label.clone(), s.names[ib].label.clone());
    let mut o = Out::new(budget);
    o.line(format!(
        "## Relationship: {la} and {lb} ({})",
        s.draft_label
    ));

    let people = story_characters(c)?;
    let id_of = |k: &str| {
        people
            .iter()
            .find(|p| queries::normalize_cue(&p.name) == k)
            .map(|p| p.id.clone())
    };
    let ties: Vec<String> = match (id_of(&ka), id_of(&kb)) {
        (Some(x), Some(y)) => typed_ties(c, &people)?
            .into_iter()
            .filter(|(f, t, _)| (*f == x && *t == y) || (*f == y && *t == x))
            .map(|(_, _, line)| line)
            .collect(),
        _ => Vec::new(),
    };
    if ties.is_empty() {
        o.line("Record: No relationship recorded.");
    } else {
        for t in ties {
            o.line(t);
        }
    }

    let (pa, pb) = (s.present_in(&ka), s.present_in(&kb));
    let (sa, sb) = (s.speaks_in(&ka), s.speaks_in(&kb));
    let both_speak: Vec<usize> = sa.iter().copied().filter(|n| sb.contains(n)).collect();
    let both_present: Vec<usize> = pa.iter().copied().filter(|n| pb.contains(n)).collect();
    let named_only: Vec<usize> = both_present
        .iter()
        .copied()
        .filter(|n| !both_speak.contains(n))
        .collect();
    if both_present.is_empty() {
        o.line(format!("No Scene has both {la} and {lb}."));
    }
    if !both_speak.is_empty() {
        o.line(format!(
            "{}: {la} and {lb} both speak.",
            scenes_text(&both_speak)
        ));
    }
    if !named_only.is_empty() {
        o.line(format!(
            "{}: {la} and {lb} are both in the Scene but do not both speak.",
            scenes_text(&named_only)
        ));
    }

    let pair: Vec<Interaction> = s
        .interactions(&(1..=usize::MAX))
        .into_iter()
        .filter(|i| (i.a == ka && i.b == kb) || (i.a == kb && i.b == ka))
        .collect();
    // exchanges from the first named character to the second, and back
    let toward = |i: &Interaction| -> (usize, usize) {
        if i.a == ka {
            (i.exchanges_ab, i.exchanges_ba)
        } else {
            (i.exchanges_ba, i.exchanges_ab)
        }
    };
    let (total_ab, total_ba) = pair.iter().fold((0, 0), |t, i| {
        let (x, y) = toward(i);
        (t.0 + x, t.1 + y)
    });
    if pair.is_empty() {
        o.line("Note: They never speak to each other in the script.");
    } else {
        let where_ = scenes_text(&pair.iter().map(|i| i.scene).collect::<Vec<_>>());
        o.line(format!(
            "{where_}: {la} to {lb} {total_ab} exchanges, {lb} to {la} {total_ba} exchanges in all."
        ));
        for i in &pair {
            let (x, y) = toward(i);
            o.line(format!(
                "Scene {}: {la} to {lb} {x}, {lb} to {la} {y}.",
                i.scene
            ));
        }
    }
    if let Some(first_shared) = both_speak.first() {
        let sc = &s.scenes[first_shared - 1];
        if let Some((who, _)) = sc.blocks.iter().find(|(w, _)| *w == ka || *w == kb) {
            o.line(format!(
                "Scene {first_shared}: {who} speaks first in their first shared Scene."
            ));
        }
    }
    if let (Some(first), Some(last)) = (pair.first(), pair.last()) {
        let window = |i: &Interaction, o: &mut Out, label: &str, start: usize| {
            let sc = &s.scenes[i.scene - 1];
            o.line(format!("Scene {}: {label}", i.scene));
            let from = start.saturating_sub(1);
            let to = (start + 2).min(sc.blocks.len() - 1);
            for (who, text) in &sc.blocks[from..=to] {
                o.line(format!("Scene {}: {}", i.scene, dialogue(who, text)));
            }
        };
        if first.scene == last.scene && first.first_pos == last.last_pos {
            window(first, &mut o, "their only interaction", first.first_pos);
        } else {
            window(first, &mut o, "their first interaction", first.first_pos);
            window(last, &mut o, "their last interaction", last.last_pos);
        }
    }
    Ok(o.finish())
}

// ------------------------------------------------------------------ act and group briefings

/// Scene list, main characters and beats for a set of scenes.
fn group(s: &Script, o: &mut Out, scenes: &[usize], beats: &[&Beat]) {
    if scenes.is_empty() {
        o.line("Record: no screenplay Scene is linked here.");
    } else {
        o.line(format!("{} are in this group.", scenes_text(scenes)));
    }
    for n in scenes {
        o.line(s.gist(&s.scenes[n - 1]));
    }
    let mut lines: BTreeMap<&str, usize> = BTreeMap::new();
    for n in scenes {
        for (k, c) in &s.scenes[n - 1].lines {
            *lines.entry(k.as_str()).or_default() += c;
        }
    }
    let mut main: Vec<(&str, usize)> = lines.into_iter().collect();
    main.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    if !main.is_empty() {
        let list: Vec<String> = main
            .iter()
            .take(5)
            .map(|(k, c)| format!("{k} {c}"))
            .collect();
        o.line(format!(
            "{}: dialogue blocks per main character: {}.",
            scenes_text(scenes),
            list.join(", ")
        ));
    }
    for b in beats {
        match b.scene {
            Some(n) => o.line(format!("Scene {n}: beat: {}", quote(&b.text))),
            None => o.line(format!("Record: beat: {}", quote(&b.text))),
        }
    }
}

pub(crate) fn act_briefing(
    c: &Connection,
    draft: &DraftRef,
    act_id: &str,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let a = s
        .acts
        .iter()
        .position(|(id, _)| id == act_id)
        .ok_or_else(|| not_found(draft, "such act"))?;
    let scenes: Vec<usize> = s
        .live()
        .filter(|x| s.act_of(x) == Some(a))
        .map(|x| x.n)
        .collect();
    let beats: Vec<&Beat> = s.beats.iter().filter(|b| b.act == Some(a)).collect();
    let mut o = Out::new(budget);
    o.line(format!(
        "## Act {}: {} ({})",
        a + 1,
        s.acts[a].1,
        s.draft_label
    ));
    for (q, (title, act)) in s.sequences.iter().enumerate() {
        if *act != Some(a) {
            continue;
        }
        let in_seq: Vec<usize> = s
            .live()
            .filter(|x| s.places.get(&x.id).is_some_and(|p| p.sequence == Some(q)))
            .map(|x| x.n)
            .collect();
        if !in_seq.is_empty() {
            o.line(format!("Sequence \"{title}\": {}.", scenes_text(&in_seq)));
        }
    }
    group(&s, &mut o, &scenes, &beats);
    Ok(o.finish())
}

/// The same for a stretch of scenes, for a project with no acts.
pub(crate) fn group_briefing(
    c: &Connection,
    draft: &DraftRef,
    first: usize,
    last: usize,
    budget: usize,
) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let (first, last) = (first.min(last), first.max(last));
    let scenes: Vec<usize> = s
        .live()
        .filter(|x| (first..=last).contains(&x.n))
        .map(|x| x.n)
        .collect();
    let beats: Vec<&Beat> = s
        .beats
        .iter()
        .filter(|b| b.scene.is_some_and(|n| scenes.contains(&n)))
        .collect();
    let mut o = Out::new(budget);
    o.line(format!("## Scenes {first} to {last} ({})", s.draft_label));
    group(&s, &mut o, &scenes, &beats);
    Ok(o.finish())
}

// ------------------------------------------------------------------ project map

fn count_if_table(c: &Connection, table: &str, sql: &str) -> AppResult<Option<i64>> {
    if !queries::table_exists(c, table)? {
        return Ok(None);
    }
    Ok(Some(c.query_row(sql, [], |r| r.get(0))?))
}

pub(crate) fn project_map(c: &Connection, draft: &DraftRef, budget: usize) -> AppResult<String> {
    let s = Script::load(c, draft)?;
    let live: Vec<&SceneData> = s.live().collect();
    let mut o = Out::new(budget);
    o.line(format!("## Project map ({})", s.draft_label));
    o.line(format!("Record: title: {}", flat(&s.title)));
    o.line(format!("Record: draft: {}", s.draft_label));
    o.line(format!(
        "Record: The draft has {} (omitted Scenes are not counted).",
        scene_word(live.len())
    ));
    if s.acts.is_empty() {
        o.line("Record: No acts set.");
    }
    for (a, (_, title)) in s.acts.iter().enumerate() {
        let ns: Vec<usize> = live
            .iter()
            .filter(|x| s.act_of(x) == Some(a))
            .map(|x| x.n)
            .collect();
        if ns.is_empty() {
            o.line(format!(
                "Record: Act {} \"{title}\" has no linked Scene yet.",
                a + 1
            ));
        } else {
            o.line(format!(
                "Act {} \"{title}\": {} ({}).",
                a + 1,
                scenes_text(&ns),
                ns.len()
            ));
        }
        for (q, (seq, act)) in s.sequences.iter().enumerate() {
            let in_seq: Vec<usize> = live
                .iter()
                .filter(|x| s.places.get(&x.id).is_some_and(|p| p.sequence == Some(q)))
                .map(|x| x.n)
                .collect();
            if *act == Some(a) && !in_seq.is_empty() {
                o.line(format!(
                    "Sequence \"{seq}\" in Act {}: {}.",
                    a + 1,
                    scenes_text(&in_seq)
                ));
            }
        }
    }

    let mut chars: Vec<(&CharName, Vec<usize>, usize)> = s
        .names
        .iter()
        .map(|n| (n, s.present_in(&n.key), s.line_count(&n.key)))
        .filter(|(_, p, _)| !p.is_empty())
        .collect();
    chars.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then(b.1.len().cmp(&a.1.len()))
            .then(a.0.key.cmp(&b.0.key))
    });
    for (n, present, lines) in chars.iter().take(8) {
        o.line(format!(
            "Character {}: in {} (first Scene {}, last Scene {}), {lines} dialogue blocks.",
            n.label,
            scene_count(present.len()),
            present[0],
            present[present.len() - 1]
        ));
    }

    let mut places: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for x in &live {
        let loc = queries::parse_heading(&x.heading).location;
        if !loc.is_empty() {
            places.entry(loc).or_default().push(x.n);
        }
    }
    let mut places: Vec<(String, Vec<usize>)> = places.into_iter().collect();
    places.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    for (loc, ns) in places.iter().take(8) {
        o.line(format!(
            "Location {loc}: {} ({}).",
            scene_count(ns.len()),
            scenes_text(ns)
        ));
    }

    if let Some(n) = count_if_table(
        c,
        "shooting_day",
        "SELECT count(*) FROM shooting_day WHERE deleted_at IS NULL AND is_off_day=0",
    )? {
        o.line(format!("Record: shooting days: {n}."));
    }
    if let Some(n) = count_if_table(
        c,
        "task",
        "SELECT count(*) FROM task WHERE deleted_at IS NULL AND status='Open'",
    )? {
        o.line(format!("Record: open tasks: {n}."));
    }
    Ok(o.finish())
}

#[cfg(test)]
mod tests;
