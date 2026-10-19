//! Deterministic, read-only project queries used by AI tools (AI spec §8).
//! Only hub-table contract columns are read. Exact facts come from here,
//! never from the model.

use std::collections::{BTreeMap, HashMap};

use openframe_domain::{AppError, AppResult};
use openframe_persistence::undo::quote_ident;
use rusqlite::{Connection, OptionalExtension, params};

pub fn table_exists(c: &Connection, name: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [name],
        |r| r.get(0),
    )?)
}

pub fn column_exists(c: &Connection, table: &str, column: &str) -> AppResult<bool> {
    if !valid_ident(table) {
        return Ok(false);
    }
    let mut stmt = c.prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    Ok(cols.iter().any(|c| c == column))
}

/// Identifiers interpolated into SQL must be plain lowercase snake_case.
pub fn valid_ident(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[derive(Debug, Clone, PartialEq)]
pub struct DraftRef {
    pub id: String,
    pub name: String,
    pub status: String,
    /// 1-based creation order within its screenplay.
    pub ordinal: usize,
    pub is_current: bool,
    /// The episode whose screenplay this draft belongs to (series).
    pub episode_id: Option<String>,
    pub screenplay_id: String,
    pub screenplay_title: String,
    /// The project holds more than one screenplay (ADR-0012, 2026-10-08), so
    /// labels name the screenplay too.
    pub several_screenplays: bool,
}

impl DraftRef {
    /// "Draft 2", or "Draft 2 of “The Short”" when the project holds several screenplays.
    pub fn label(&self) -> String {
        if self.several_screenplays {
            format!("{} of “{}”", self.name, self.screenplay_title)
        } else {
            self.name.clone()
        }
    }
    pub fn locked(&self) -> bool {
        self.status == "Locked"
    }
}

/// Every live draft, grouped by screenplay (oldest screenplay first), each
/// group in creation order.
pub fn list_drafts(c: &Connection) -> AppResult<Vec<DraftRef>> {
    let mut stmt = c.prepare(
        "SELECT d.id, d.name, d.status, (s.current_draft_id = d.id) AS cur, s.episode_id, s.id, s.title
         FROM screenplay_draft d JOIN screenplay s ON s.id = d.screenplay_id
         WHERE d.deleted_at IS NULL AND s.deleted_at IS NULL
         ORDER BY s.created_at, s.id, d.created_at, d.id",
    )?;
    type Row = (String, String, String, bool, Option<String>, String, String);
    let rows: Vec<Row> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get::<_, Option<bool>>(3)?.unwrap_or(false),
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    let several = rows
        .iter()
        .map(|r| r.5.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len()
        > 1;
    let mut ordinals: HashMap<String, usize> = HashMap::new();
    Ok(rows
        .into_iter()
        .map(
            |(id, name, status, cur, episode_id, screenplay_id, screenplay_title)| {
                let n = ordinals.entry(screenplay_id.clone()).or_insert(0);
                *n += 1;
                DraftRef {
                    id,
                    name,
                    status,
                    ordinal: *n,
                    is_current: cur,
                    episode_id,
                    screenplay_id,
                    screenplay_title,
                    several_screenplays: several,
                }
            },
        )
        .collect())
}

/// Current drafts of every screenplay, oldest screenplay first.
pub fn current_drafts(c: &Connection) -> AppResult<Vec<DraftRef>> {
    Ok(list_drafts(c)?
        .into_iter()
        .filter(|d| d.is_current)
        .collect())
}

/// The project's current draft when that is unambiguous: the current draft of
/// its only screenplay. With several screenplays this is None, so no caller
/// picks one silently; tools then ask which screenplay is meant.
pub fn current_draft(c: &Connection) -> AppResult<Option<DraftRef>> {
    let drafts = list_drafts(c)?;
    if drafts.first().is_some_and(|d| d.several_screenplays) {
        return Ok(None);
    }
    Ok(drafts
        .iter()
        .find(|d| d.is_current)
        .cloned()
        .or_else(|| drafts.last().cloned()))
}

/// Titles of the screenplays, oldest first (for "which screenplay?" questions).
fn screenplay_titles(drafts: &[DraftRef]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for d in drafts {
        let t = format!("“{}”", d.screenplay_title);
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

pub fn draft_by_id(c: &Connection, id: &str) -> AppResult<Option<DraftRef>> {
    Ok(list_drafts(c)?.into_iter().find(|d| d.id == id))
}

/// The screenplay a draft reference names, when the project holds several:
/// "Draft 2 of The Short", "The Short: Draft 2" or just "The Short". Returns
/// the screenplay id and the reference with the title taken out. The longest
/// matching title wins, so "Rain" never shadows "Black Rain".
fn screenplay_in_reference(drafts: &[DraftRef], lower: &str) -> Option<(String, String)> {
    let mut best: Option<(&DraftRef, usize)> = None;
    for d in drafts {
        let t = d.screenplay_title.trim().to_lowercase();
        if t.is_empty() {
            continue;
        }
        if let Some(at) = lower.find(&t)
            && best.is_none_or(|(b, _)| b.screenplay_title.trim().len() < t.len())
        {
            best = Some((d, at));
        }
    }
    let (d, at) = best?;
    let t = d.screenplay_title.trim().to_lowercase();
    let rest = format!("{}{}", &lower[..at], &lower[at + t.len()..]);
    // Only the joining words around the title go ("… of the", "… in"); words
    // inside a draft name such as "The End" stay.
    let mut words: Vec<&str> = Vec::new();
    let cleaned = rest.replace(['“', '”', '"', ':', ','], " ");
    words.extend(cleaned.split_whitespace());
    while words
        .last()
        .is_some_and(|w| matches!(*w, "of" | "in" | "from" | "the" | "screenplay" | "-"))
    {
        words.pop();
    }
    Some((d.screenplay_id.clone(), words.join(" ")))
}

/// Resolve "current", a draft name, "Draft 7" or "7", optionally with the
/// screenplay ("Draft 2 of The Short"). Ambiguity is an error the assistant
/// turns into one focused clarification (AI spec §6.3); with several
/// screenplays it asks which one rather than picking one.
pub fn resolve_draft(
    c: &Connection,
    reference: Option<&str>,
    default: Option<&DraftRef>,
) -> AppResult<DraftRef> {
    let mut drafts = list_drafts(c)?;
    if drafts.is_empty() {
        return Err(AppError::ai(
            "no_screenplay",
            "This project doesn't have a screenplay draft yet.",
        ));
    }
    let r = reference.map(|s| s.trim()).unwrap_or("");
    let mut lower = r.to_lowercase();
    let several = drafts[0].several_screenplays;
    let mut narrowed = false;
    if several && let Some((screenplay, rest)) = screenplay_in_reference(&drafts, &lower) {
        drafts.retain(|d| d.screenplay_id == screenplay);
        lower = rest;
        narrowed = true;
    }
    let one_screenplay = !several || narrowed;
    if lower.is_empty()
        || matches!(
            lower.as_str(),
            "current" | "current draft" | "this draft" | "the current draft" | "latest"
        )
    {
        if let Some(d) = default
            && drafts.iter().any(|x| x.id == d.id)
        {
            return Ok(d.clone());
        }
        if !one_screenplay {
            return Err(ambiguous(format!(
                "This project has several screenplays. Which one do you mean: {}?",
                join_or(&screenplay_titles(&drafts))
            )));
        }
        return drafts
            .iter()
            .find(|d| d.is_current)
            .or_else(|| drafts.last())
            .cloned()
            .ok_or_else(|| {
                AppError::ai(
                    "no_screenplay",
                    "This project doesn't have a screenplay draft yet.",
                )
            });
    }
    let exact: Vec<&DraftRef> = drafts
        .iter()
        .filter(|d| d.name.to_lowercase() == lower)
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0].clone());
    }
    if exact.len() > 1 {
        return Err(ambiguous(format!(
            "Which draft do you mean: {}?",
            join_or(&exact.iter().map(|d| d.label()).collect::<Vec<_>>())
        )));
    }
    let digits: String = lower
        .trim_start_matches("draft")
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if let Ok(n) = digits.parse::<usize>() {
        let token = format!("draft {n}");
        let named: Vec<&DraftRef> = drafts
            .iter()
            .filter(|d| {
                let dl = d.name.to_lowercase();
                dl == token
                    || dl.starts_with(&format!("{token} "))
                    || dl.starts_with(&format!("{token}:"))
                    || dl.starts_with(&format!("{token}\u{2014}"))
                    || dl.starts_with(&format!("{token} \u{2014}"))
            })
            .collect();
        if named.len() == 1 {
            return Ok(named[0].clone());
        }
        if named.len() > 1 {
            return Err(ambiguous(format!(
                "Which draft do you mean: {}?",
                join_or(&named.iter().map(|d| d.label()).collect::<Vec<_>>())
            )));
        }
        // "7" means the 7th draft; with several screenplays only once the
        // screenplay is known, because each counts its own drafts.
        if one_screenplay && let Some(d) = drafts.get(n.wrapping_sub(1)) {
            return Ok(d.clone());
        }
        if !one_screenplay {
            let by_ordinal: Vec<&DraftRef> = drafts.iter().filter(|d| d.ordinal == n).collect();
            if !by_ordinal.is_empty() {
                return Err(ambiguous(format!(
                    "Which draft do you mean: {}?",
                    join_or(&by_ordinal.iter().map(|d| d.label()).collect::<Vec<_>>())
                )));
            }
        }
    }
    let partial: Vec<&DraftRef> = drafts
        .iter()
        .filter(|d| d.name.to_lowercase().contains(&lower))
        .collect();
    match partial.len() {
        1 => Ok(partial[0].clone()),
        0 => Err(AppError::ai(
            "not_found",
            format!("I couldn't find a draft called “{r}”."),
        )),
        _ => Err(ambiguous(format!(
            "Which draft do you mean: {}?",
            join_or(&partial.iter().map(|d| d.label()).collect::<Vec<_>>())
        ))),
    }
}

pub fn ambiguous(question: impl Into<String>) -> AppError {
    AppError::ai("ambiguous", question)
}

pub fn join_or(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} or {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

pub fn join_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneRow {
    pub id: String,
    pub lineage_id: String,
    /// Display number derived from position (never stored, Domain §2.2).
    pub number: usize,
    pub heading: String,
    pub omitted: bool,
    pub rev: i64,
}

impl SceneRow {
    pub fn label(&self) -> String {
        if self.heading.trim().is_empty() {
            format!("Scene {}", self.number)
        } else {
            format!("Scene {}: {}", self.number, self.heading.trim())
        }
    }
}

pub fn scenes(c: &Connection, draft_id: &str) -> AppResult<Vec<SceneRow>> {
    let mut stmt = c.prepare(
        "SELECT id, lineage_id, heading, omitted, rev FROM screenplay_scene
         WHERE draft_id=?1 AND deleted_at IS NULL ORDER BY position, id",
    )?;
    let rows: Vec<(String, String, String, bool, i64)> = stmt
        .query_map([draft_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .enumerate()
        .map(|(i, (id, lineage_id, heading, omitted, rev))| SceneRow {
            id,
            lineage_id,
            number: i + 1,
            heading,
            omitted,
            rev,
        })
        .collect())
}

pub fn scene_by_id(c: &Connection, scene_id: &str) -> AppResult<Option<(DraftRef, SceneRow)>> {
    let draft_id: Option<String> = c
        .query_row(
            "SELECT draft_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
            [scene_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(draft_id) = draft_id else {
        return Ok(None);
    };
    let Some(draft) = draft_by_id(c, &draft_id)? else {
        return Ok(None);
    };
    let scene = scenes(c, &draft_id)?.into_iter().find(|s| s.id == scene_id);
    Ok(scene.map(|s| (draft, s)))
}

/// Normalize a Character cue: uppercase, drop extensions like (V.O.) / (CONT'D) and dual-dialogue carets.
pub fn normalize_cue(text: &str) -> String {
    let mut s = text.trim().trim_end_matches('^').trim().to_string();
    if let Some(open) = s.find('(') {
        s.truncate(open);
    }
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// Distinct character cues in a draft → scene ids where each speaks (ordered by scene).
pub fn character_cues(c: &Connection, draft_id: &str) -> AppResult<BTreeMap<String, Vec<String>>> {
    let mut stmt = c.prepare(
        "SELECT e.text, s.id FROM screenplay_element e
         JOIN screenplay_scene s ON s.id = e.scene_id
         WHERE s.draft_id=?1 AND s.deleted_at IS NULL AND e.element_type='character'
         ORDER BY s.position, e.position",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([draft_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (text, scene) in rows {
        let name = normalize_cue(&text);
        if name.is_empty() {
            continue;
        }
        let v = out.entry(name).or_default();
        if !v.contains(&scene) {
            v.push(scene);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heading {
    pub int_ext: Option<String>,
    pub location: String,
    pub time: Option<String>,
}

/// Parse a scene heading ("INT. POLICE STATION - NIGHT", any dash).
pub fn parse_heading(heading: &str) -> Heading {
    let h = heading.trim().to_uppercase();
    if h.is_empty() {
        return Heading::default();
    }
    let prefixes = [
        ("INT./EXT.", "INT./EXT."),
        ("INT/EXT.", "INT./EXT."),
        ("INT/EXT", "INT./EXT."),
        ("EXT./INT.", "INT./EXT."),
        ("I/E.", "INT./EXT."),
        ("I/E", "INT./EXT."),
        ("INT.", "INT."),
        ("EXT.", "EXT."),
        ("INT ", "INT."),
        ("EXT ", "EXT."),
    ];
    let mut rest = h.as_str();
    let mut int_ext = None;
    for (p, norm) in prefixes {
        if let Some(r) = rest.strip_prefix(p) {
            int_ext = Some(norm.to_string());
            rest = r;
            break;
        }
    }
    let rest = rest.trim();
    let mut location = rest.to_string();
    let mut time = None;
    for sep in [" \u{2014} ", " \u{2013} ", " - ", "\u{2014}", "\u{2013}"] {
        if let Some(idx) = rest.rfind(sep) {
            let t = rest[idx + sep.len()..].trim();
            if !t.is_empty() && t.len() <= 24 {
                time = Some(t.to_string());
                location = rest[..idx].trim().to_string();
            }
            break;
        }
    }
    Heading {
        int_ext,
        location: location
            .trim_matches(|c: char| c == '.' || c.is_whitespace())
            .to_string(),
        time,
    }
}

pub fn character_records(c: &Connection) -> AppResult<Vec<(String, String, i64)>> {
    let mut stmt =
        c.prepare("SELECT id, name, rev FROM story_character WHERE deleted_at IS NULL AND archived=0 ORDER BY position, name")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

pub fn count(c: &Connection, sql: &str, p: impl rusqlite::Params) -> AppResult<i64> {
    Ok(c.query_row(sql, p, |r| r.get(0))?)
}

/// `count(*)` with an `IN (…)` list of ids appended to `sql_prefix`.
pub fn count_in(c: &Connection, sql_prefix: &str, ids: &[String]) -> AppResult<i64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let marks = vec!["?"; ids.len()].join(",");
    let sql = format!("{sql_prefix} ({marks})");
    Ok(c.query_row(&sql, rusqlite::params_from_iter(ids.iter()), |r| r.get(0))?)
}

/// Readable text of one scene (element order preserved).
pub fn scene_text(c: &Connection, scene_id: &str, max_chars: usize) -> AppResult<String> {
    let mut stmt = c.prepare(
        "SELECT element_type, text FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([scene_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut out = String::new();
    for (t, text) in rows {
        let line = match t.as_str() {
            "character" => format!("\n{}", text.trim().to_uppercase()),
            "parenthetical" => format!("({})", text.trim().trim_matches(['(', ')'])),
            "scene_heading" => format!("{}\n", text.trim().to_uppercase()),
            "note" => continue,
            _ => text.trim().to_string(),
        };
        out.push_str(&line);
        out.push('\n');
        if out.len() > max_chars {
            break;
        }
    }
    Ok(truncate_chars(&out, max_chars))
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push('…');
    t
}

/// Content hash of a scene (heading + element types/text) for draft comparison.
pub fn scene_fingerprint(c: &Connection, scene_id: &str) -> AppResult<String> {
    let heading: String = c.query_row(
        "SELECT heading FROM screenplay_scene WHERE id=?1",
        [scene_id],
        |r| r.get(0),
    )?;
    let mut stmt = c.prepare(
        "SELECT element_type, text FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
    )?;
    let rows: Vec<(String, String)> = stmt
        .query_map([scene_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut s = heading.trim().to_uppercase();
    for (t, text) in rows {
        s.push('\u{1f}');
        s.push_str(&t);
        s.push('\u{1e}');
        s.push_str(text.trim());
    }
    Ok(openframe_security::sha256_bytes(s.as_bytes()))
}

/// Active Production Source draft, if one is selected.
pub fn production_source_draft(c: &Connection) -> AppResult<Option<(String, String)>> {
    Ok(c
        .query_row(
            "SELECT p.id, p.draft_id FROM production_source p JOIN screenplay_draft d ON d.id = p.draft_id
             WHERE p.active=1 AND d.deleted_at IS NULL ORDER BY p.selected_at DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

/// A row's current revision (None when missing or deleted). `table` must be a valid identifier.
pub fn row_rev(c: &Connection, table: &str, id: &str) -> AppResult<Option<i64>> {
    if !valid_ident(table) || !table_exists(c, table)? || !column_exists(c, table, "id")? {
        return Ok(None);
    }
    let has_deleted = column_exists(c, table, "deleted_at")?;
    let sql = if has_deleted {
        format!(
            "SELECT rev FROM {} WHERE id=?1 AND deleted_at IS NULL",
            quote_ident(table)
        )
    } else {
        format!("SELECT rev FROM {} WHERE id=?1", quote_ident(table))
    };
    Ok(c.query_row(&sql, params![id], |r| r.get(0)).optional()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_rev_ignores_a_table_without_an_id_column() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE odd(x TEXT)").unwrap();
        assert_eq!(row_rev(&c, "odd", "1").unwrap(), None);
    }

    #[test]
    fn cues_normalize() {
        assert_eq!(normalize_cue("Ravi (V.O.)"), "RAVI");
        assert_eq!(normalize_cue("  arjun  (CONT'D) "), "ARJUN");
        assert_eq!(normalize_cue("MEERA ^"), "MEERA");
        assert_eq!(normalize_cue("Dr.  Rao"), "DR. RAO");
    }

    #[test]
    fn scene_labels_use_a_colon_never_a_dash() {
        // Owner decision 2026-10-08: no em or en dash in text OpenFrame writes itself. The
        // heading after the colon is the writer's own text and stays as typed.
        let row = |heading: &str| SceneRow {
            id: "s".into(),
            lineage_id: "l".into(),
            number: 12,
            heading: heading.into(),
            omitted: false,
            rev: 0,
        };
        assert_eq!(
            row("INT. HOSPITAL - DAY").label(),
            "Scene 12: INT. HOSPITAL - DAY"
        );
        assert_eq!(row("  ").label(), "Scene 12");
        assert_eq!(
            row("INT. HOSPITAL \u{2014} DAY").label(),
            "Scene 12: INT. HOSPITAL \u{2014} DAY"
        );
    }

    #[test]
    fn headings_parse() {
        let h = parse_heading("INT. POLICE STATION \u{2014} NIGHT");
        assert_eq!(h.int_ext.as_deref(), Some("INT."));
        assert_eq!(h.location, "POLICE STATION");
        assert_eq!(h.time.as_deref(), Some("NIGHT"));
        let h = parse_heading("ext. old railway station - dawn");
        assert_eq!(h.location, "OLD RAILWAY STATION");
        assert_eq!(h.time.as_deref(), Some("DAWN"));
        let h = parse_heading("INT./EXT. CAR");
        assert_eq!(h.int_ext.as_deref(), Some("INT./EXT."));
        assert_eq!(h.location, "CAR");
        assert_eq!(h.time, None);
    }

    #[test]
    fn lists_join_naturally() {
        assert_eq!(
            join_and(&["2".into(), "15".into(), "26".into()]),
            "2, 15 and 26"
        );
        assert_eq!(join_or(&["A".into(), "B".into()]), "A or B");
    }
}
