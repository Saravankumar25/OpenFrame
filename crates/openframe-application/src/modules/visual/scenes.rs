//! Scene resolution for visual planning (FSD §34, §53, §55).
//!
//! Visual planning is attached to SCENE IDENTITY (`screenplay_scene.lineage_id`).
//! The scenes offered for planning come from the active Production Source draft(s)
//! when production has started, otherwise from the screenplay's current draft.
//! Display numbers are derived from order within the draft and are never stored.
//!
//! Script changes never delete or rewrite visual planning: each shot/storyboard
//! keeps a snapshot hash of the scene content it was planned against, and a
//! different current hash flags the scene "Scene changed since planning" until
//! the user marks it reviewed.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use openframe_domain::{Actor, AppError, AppResult, Capability, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::core::AppCore;
use crate::store::{MutationMeta, Tx};

/// Above this many scene fingerprints in one read, compute them all in bulk.
pub(crate) const PRIME_THRESHOLD: usize = 8;

/// One screenplay scene available for planning.
#[derive(Debug, Clone)]
pub struct SceneRow {
    pub id: String,
    pub lineage_id: String,
    pub draft_id: String,
    /// 1-based order within its draft (the displayed scene number).
    pub number: i64,
    pub heading: String,
    pub omitted: bool,
    /// Screenplay title when several screenplays (episodes) are in the source.
    pub group: Option<String>,
}

/// Per-read context: the planning source and its scenes, with lazily computed
/// content hashes.
pub struct SceneCtx {
    pub source_kind: &'static str,
    pub source_label: String,
    pub scenes: Vec<SceneRow>,
    by_lineage: HashMap<String, usize>,
    by_id: HashMap<String, usize>,
    hashes: RefCell<HashMap<String, (String, i64)>>,
}

impl SceneCtx {
    pub fn load(c: &Connection) -> AppResult<SceneCtx> {
        // (draft id, draft name, screenplay title)
        let mut drafts: Vec<(String, String, String)> = {
            let mut stmt = c.prepare(
                "SELECT d.id, d.name, s.title FROM production_source ps
                 JOIN screenplay_draft d ON d.id = ps.draft_id
                 JOIN screenplay s ON s.id = d.screenplay_id
                 WHERE ps.active = 1 AND d.deleted_at IS NULL AND s.deleted_at IS NULL
                 ORDER BY s.created_at, s.id, ps.selected_at",
            )?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?
        };
        let mut kind = "productionSource";
        if drafts.is_empty() {
            kind = "currentDraft";
            let mut stmt = c.prepare(
                "SELECT d.id, d.name, s.title FROM screenplay s
                 JOIN screenplay_draft d ON d.id = s.current_draft_id
                 WHERE s.deleted_at IS NULL AND d.deleted_at IS NULL
                 ORDER BY s.created_at, s.id",
            )?;
            drafts = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
        }
        drafts.dedup_by(|a, b| a.0 == b.0);
        if drafts.is_empty() {
            kind = "none";
        }
        let multi = drafts.len() > 1;
        let source_label = match kind {
            "productionSource" => format!(
                "Production Source: {}",
                drafts
                    .iter()
                    .map(|d| d.1.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            "currentDraft" => {
                format!(
                    "Current draft: {}",
                    drafts
                        .iter()
                        .map(|d| d.1.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            _ => String::new(),
        };
        let mut scenes = Vec::new();
        for (draft_id, _, title) in &drafts {
            let mut stmt = c.prepare(
                "SELECT id, lineage_id, heading, omitted FROM screenplay_scene
                 WHERE draft_id = ?1 AND deleted_at IS NULL ORDER BY position, id",
            )?;
            let rows = stmt
                .query_map([draft_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, bool>(3)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for (i, (id, lineage_id, heading, omitted)) in rows.into_iter().enumerate() {
                scenes.push(SceneRow {
                    id,
                    lineage_id,
                    draft_id: draft_id.clone(),
                    number: i as i64 + 1,
                    heading,
                    omitted,
                    group: multi.then(|| title.clone()),
                });
            }
        }
        let mut by_lineage = HashMap::new();
        let mut by_id = HashMap::new();
        for (i, s) in scenes.iter().enumerate() {
            by_lineage.entry(s.lineage_id.clone()).or_insert(i);
            by_id.insert(s.id.clone(), i);
        }
        Ok(SceneCtx {
            source_kind: kind,
            source_label,
            scenes,
            by_lineage,
            by_id,
            hashes: RefCell::new(HashMap::new()),
        })
    }

    pub fn by_lineage(&self, lineage: &str) -> Option<&SceneRow> {
        self.by_lineage.get(lineage).map(|&i| &self.scenes[i])
    }

    pub fn by_id(&self, id: &str) -> Option<&SceneRow> {
        self.by_id.get(id).map(|&i| &self.scenes[i])
    }

    /// Index of a lineage in script order (for sorting); removed scenes sort last.
    pub fn order_of(&self, lineage: &str) -> usize {
        self.by_lineage.get(lineage).copied().unwrap_or(usize::MAX)
    }

    /// The scene a new planning object attaches to. Accepts a scene row id from
    /// any draft and resolves it by identity in the planning source.
    pub fn resolve(&self, c: &Connection, scene_id: &str) -> AppResult<SceneRow> {
        if let Some(s) = self.by_id(scene_id) {
            return Ok(s.clone());
        }
        let lineage: Option<String> = c
            .query_row(
                "SELECT lineage_id FROM screenplay_scene WHERE id=?1",
                [scene_id],
                |r| r.get(0),
            )
            .optional()?;
        match lineage.and_then(|l| self.by_lineage(&l).cloned()) {
            Some(s) => Ok(s),
            None => Err(AppError::new(
                "not_found.scene",
                "That scene isn't in the script version used for production planning. Choose a scene from the list.",
            )),
        }
    }

    /// Display number, e.g. "12" (prefixed with the episode title when several screenplays are planned).
    pub fn number_label(&self, s: &SceneRow) -> String {
        s.number.to_string()
    }

    /// Content fingerprint of a scene (heading, omitted state and every element) and
    /// when it last changed.
    pub fn hash(&self, c: &Connection, scene_id: &str) -> AppResult<(String, i64)> {
        if let Some(h) = self.hashes.borrow().get(scene_id) {
            return Ok(h.clone());
        }
        let h = scene_hash(c, scene_id)?;
        self.hashes
            .borrow_mut()
            .insert(scene_id.to_string(), h.clone());
        Ok(h)
    }

    /// Compute the fingerprints of every planning scene with two queries per draft
    /// (instead of two per scene). List views call this before walking all scenes.
    pub fn prime_hashes(&self, c: &Connection) -> AppResult<()> {
        let mut drafts: Vec<&str> = self.scenes.iter().map(|s| s.draft_id.as_str()).collect();
        drafts.sort_unstable();
        drafts.dedup();
        let mut hashes = self.hashes.borrow_mut();
        for draft in drafts {
            let mut builders: HashMap<String, SceneHasher> = HashMap::new();
            {
                let mut stmt = c.prepare_cached(
                    "SELECT id, heading, omitted, updated_at FROM screenplay_scene
                     WHERE draft_id=?1 AND deleted_at IS NULL",
                )?;
                let mut rows = stmt.query([draft])?;
                while let Some(r) = rows.next()? {
                    let id: String = r.get(0)?;
                    let heading: String = r.get(1)?;
                    builders.insert(id, SceneHasher::new(&heading, r.get(2)?, r.get(3)?));
                }
            }
            let mut stmt = c.prepare_cached(
                "SELECT e.scene_id, e.element_type, e.text, e.dual, e.updated_at FROM screenplay_element e
                 JOIN screenplay_scene s ON s.id = e.scene_id
                 WHERE s.draft_id=?1 AND s.deleted_at IS NULL ORDER BY e.scene_id, e.position, e.id",
            )?;
            let mut rows = stmt.query([draft])?;
            while let Some(r) = rows.next()? {
                let sid: String = r.get(0)?;
                if let Some(h) = builders.get_mut(&sid) {
                    let t: String = r.get(1)?;
                    let text: String = r.get(2)?;
                    h.element(&t, &text, r.get(3)?, r.get(4)?);
                }
            }
            for (id, h) in builders {
                hashes.entry(id).or_insert_with(|| h.finish());
            }
        }
        Ok(())
    }

    /// Review state of a planning object: (scene removed from source, needs review).
    pub fn review_state(
        &self,
        c: &Connection,
        lineage: &str,
        stored: Option<&str>,
        flagged: bool,
    ) -> AppResult<(bool, bool)> {
        match self.by_lineage(lineage) {
            None => Ok((true, flagged)),
            Some(s) => {
                let (h, _) = self.hash(c, &s.id)?;
                Ok((false, flagged || stored.is_some_and(|x| x != h)))
            }
        }
    }
}

/// Fingerprint of a scene's content. Independent of scene position/number, so
/// reordering scenes never flags visual planning (FSD §55 "Scene reordered").
pub fn scene_hash(c: &Connection, scene_id: &str) -> AppResult<(String, i64)> {
    let head: Option<(String, bool, i64)> = c
        .query_row(
            "SELECT heading, omitted, updated_at FROM screenplay_scene WHERE id=?1",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((heading, omitted, changed)) = head else {
        return Err(AppError::not_found("scene"));
    };
    let mut h = SceneHasher::new(&heading, omitted, changed);
    let mut stmt = c.prepare_cached(
        "SELECT element_type, text, dual, updated_at FROM screenplay_element WHERE scene_id=?1 ORDER BY position, id",
    )?;
    let mut rows = stmt.query([scene_id])?;
    while let Some(r) = rows.next()? {
        let t: String = r.get(0)?;
        let text: String = r.get(1)?;
        h.element(&t, &text, r.get(2)?, r.get(3)?);
    }
    Ok(h.finish())
}

/// Incremental builder of a scene fingerprint, shared by the single-scene and
/// the bulk path (`SceneCtx::prime_hashes`) so both produce identical hashes.
struct SceneHasher {
    buf: String,
    changed: i64,
}

impl SceneHasher {
    fn new(heading: &str, omitted: bool, updated_at: i64) -> Self {
        let mut buf = String::with_capacity(1024);
        buf.push_str(heading.trim());
        buf.push('\u{1e}');
        buf.push(if omitted { '1' } else { '0' });
        SceneHasher {
            buf,
            changed: updated_at,
        }
    }

    fn element(&mut self, element_type: &str, text: &str, dual: bool, updated_at: i64) {
        self.changed = self.changed.max(updated_at);
        self.buf.push('\u{1e}');
        self.buf.push_str(element_type);
        self.buf.push('\u{1f}');
        self.buf.push_str(text.trim_end());
        if dual {
            self.buf.push_str("\u{1f}d");
        }
    }

    fn finish(self) -> (String, i64) {
        (
            openframe_security::sha256_bytes(self.buf.as_bytes()),
            self.changed,
        )
    }
}

// ------------------------------------------------------------------ DTOs

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VisualScene {
    pub scene_id: String,
    pub lineage_id: String,
    pub draft_id: String,
    /// Displayed scene number (derived from order; never stored).
    pub number: String,
    pub heading: String,
    pub omitted: bool,
    /// Screenplay/episode title when several screenplays are planned.
    pub group_label: Option<String>,
    #[ts(type = "number")]
    pub shot_count: i64,
    #[ts(type = "number")]
    pub storyboard_count: i64,
    #[ts(type = "number")]
    pub moodboard_count: i64,
    /// "Scene changed since planning" (FSD §55).
    pub needs_review: bool,
    /// When the scene content last changed (only when `needs_review`).
    #[ts(type = "number | null")]
    pub changed_at: Option<i64>,
}

/// Planning kept for a scene that is no longer in the source (FSD §55 "Scene removed").
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct RemovedVisualScene {
    pub lineage_id: String,
    /// Heading as it was when the planning was made.
    pub heading: String,
    #[ts(type = "number")]
    pub shot_count: i64,
    #[ts(type = "number")]
    pub storyboard_count: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct VisualScenes {
    /// "productionSource", "currentDraft" or "none" (no screenplay yet).
    pub source_kind: String,
    pub source_label: String,
    pub scenes: Vec<VisualScene>,
    pub removed: Vec<RemovedVisualScene>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualScenesArgs {}

#[derive(Default)]
struct LineageStats {
    shots: i64,
    boards: i64,
    moods: i64,
    heading: Option<(i64, String)>,
    /// (stored hash, flagged) of every planning object.
    marks: Vec<(Option<String>, bool)>,
}

pub fn list_scenes(core: &AppCore, actor: &Actor, _: VisualScenesArgs) -> AppResult<VisualScenes> {
    actor.require(Capability::View, "view production planning")?;
    let s = core.project()?;
    s.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let mut stats: BTreeMap<String, LineageStats> = BTreeMap::new();
        for (table, is_shot) in [("shot", true), ("storyboard", false)] {
            let mut stmt = c.prepare(&format!(
                "SELECT scene_lineage_id, scene_hash, needs_review, scene_heading, updated_at FROM {table}
                 WHERE deleted_at IS NULL AND scene_lineage_id IS NOT NULL"
            ))?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let lineage: String = r.get(0)?;
                let e = stats.entry(lineage).or_default();
                if is_shot {
                    e.shots += 1;
                } else {
                    e.boards += 1;
                }
                e.marks.push((r.get(1)?, r.get(2)?));
                let heading: Option<String> = r.get(3)?;
                let at: i64 = r.get(4)?;
                if let Some(h) = heading
                    && e.heading.as_ref().is_none_or(|(t, _)| at > *t)
                {
                    e.heading = Some((at, h));
                }
            }
        }
        {
            let mut stmt = c.prepare(
                "SELECT scene_lineage_id, count(*) FROM moodboard WHERE deleted_at IS NULL AND scene_lineage_id IS NOT NULL GROUP BY 1",
            )?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                stats.entry(r.get(0)?).or_default().moods = r.get(1)?;
            }
        }
        // Many planned scenes: fingerprint them in bulk rather than two queries each.
        if stats.values().filter(|st| !st.marks.is_empty()).count() > PRIME_THRESHOLD {
            ctx.prime_hashes(c)?;
        }
        let mut scenes = Vec::with_capacity(ctx.scenes.len());
        for sc in &ctx.scenes {
            let st = stats.get(&sc.lineage_id);
            let mut needs_review = false;
            let mut changed_at = None;
            if let Some(st) = st.filter(|st| !st.marks.is_empty()) {
                let (h, at) = ctx.hash(c, &sc.id)?;
                needs_review = st.marks.iter().any(|(stored, flagged)| *flagged || stored.as_deref().is_some_and(|x| x != h));
                if needs_review {
                    changed_at = Some(at);
                }
            }
            scenes.push(VisualScene {
                scene_id: sc.id.clone(),
                lineage_id: sc.lineage_id.clone(),
                draft_id: sc.draft_id.clone(),
                number: ctx.number_label(sc),
                heading: sc.heading.clone(),
                omitted: sc.omitted,
                group_label: sc.group.clone(),
                shot_count: st.map(|s| s.shots).unwrap_or(0),
                storyboard_count: st.map(|s| s.boards).unwrap_or(0),
                moodboard_count: st.map(|s| s.moods).unwrap_or(0),
                needs_review,
                changed_at,
            });
        }
        let removed = stats
            .iter()
            .filter(|(l, st)| ctx.by_lineage(l).is_none() && (st.shots > 0 || st.boards > 0))
            .map(|(l, st)| RemovedVisualScene {
                lineage_id: l.clone(),
                heading: st.heading.as_ref().map(|(_, h)| h.clone()).unwrap_or_default(),
                shot_count: st.shots,
                storyboard_count: st.boards,
            })
            .collect();
        Ok(VisualScenes { source_kind: ctx.source_kind.to_string(), source_label: ctx.source_label.clone(), scenes, removed })
    })
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarkSceneReviewedArgs {
    pub scene_lineage_id: String,
}

/// "Mark reviewed": the user has checked the shots/storyboards against the changed
/// scene. Only the planning snapshot is updated — planning content is untouched.
pub fn mark_scene_reviewed(
    core: &AppCore,
    actor: &Actor,
    args: MarkSceneReviewedArgs,
) -> AppResult<()> {
    let s = core.project()?;
    let label = s.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let sc = ctx.by_lineage(&args.scene_lineage_id).ok_or_else(|| {
            AppError::new(
                "not_found.scene",
                "This scene is no longer in the script. Its planning is kept for reference.",
            )
        })?;
        Ok(format!("Scene {}", ctx.number_label(sc)))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "visual.mark_scene_reviewed",
            format!("Marked {label} planning as reviewed"),
            Capability::Edit,
        ),
        |tx| {
            let ctx = SceneCtx::load(tx.conn())?;
            let sc = ctx
                .by_lineage(&args.scene_lineage_id)
                .cloned()
                .ok_or_else(|| AppError::not_found("scene"))?;
            refresh_snapshot(tx, &ctx, &sc)
        },
    )
}

/// Point every live shot/storyboard of a scene at its current content.
pub fn refresh_snapshot(tx: &Tx<'_>, ctx: &SceneCtx, sc: &SceneRow) -> AppResult<()> {
    let (h, _) = ctx.hash(tx.conn(), &sc.id)?;
    let now = now_ms();
    for table in ["shot", "storyboard"] {
        tx.conn().execute(
            &format!(
                "UPDATE {table} SET scene_hash=?1, scene_heading=?2, scene_id=?3, needs_review=0, rev=rev+1, updated_at=?4
                 WHERE scene_lineage_id=?5 AND deleted_at IS NULL
                   AND (scene_hash IS NOT ?1 OR needs_review<>0 OR scene_id IS NOT ?3 OR scene_heading IS NOT ?2)"
            ),
            params![h, sc.heading, sc.id, now, sc.lineage_id],
        )?;
    }
    Ok(())
}

/// For other modules (e.g. a Production Source update): explicitly flag a scene's
/// visual planning for review. Never deletes or edits planning content.
pub fn flag_scene_for_review(tx: &Tx<'_>, scene_lineage_id: &str) -> AppResult<()> {
    let now = now_ms();
    for table in ["shot", "storyboard"] {
        tx.conn().execute(
            &format!(
                "UPDATE {table} SET needs_review=1, rev=rev+1, updated_at=?1
                 WHERE scene_lineage_id=?2 AND deleted_at IS NULL AND needs_review=0"
            ),
            params![now, scene_lineage_id],
        )?;
    }
    Ok(())
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneCharactersArgs {
    pub scene_id: String,
}

/// Character names spoken in a scene (from character cues), for the shot
/// "Characters" field. Extensions such as (V.O.) / (CONT'D) are removed.
pub fn scene_characters(
    core: &AppCore,
    actor: &Actor,
    args: SceneCharactersArgs,
) -> AppResult<Vec<String>> {
    actor.require(Capability::View, "view production planning")?;
    let s = core.project()?;
    s.store.read(|c| {
        let ctx = SceneCtx::load(c)?;
        let scene_id = ctx.resolve(c, &args.scene_id).map(|s| s.id).unwrap_or(args.scene_id.clone());
        let mut stmt = c.prepare(
            "SELECT text FROM screenplay_element WHERE scene_id=?1 AND element_type='character' ORDER BY position",
        )?;
        let cues = stmt.query_map([&scene_id], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?;
        let mut out: Vec<String> = Vec::new();
        for cue in cues {
            let name = cue.split('(').next().unwrap_or("").trim().trim_end_matches('^').trim().to_uppercase();
            if !name.is_empty() && !out.contains(&name) {
                out.push(name);
            }
        }
        Ok(out)
    })
}
