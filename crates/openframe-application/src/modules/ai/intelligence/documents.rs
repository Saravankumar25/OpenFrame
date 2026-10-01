//! Domain documents (§13), domain-aware chunking (§14) and deterministic graph
//! fragments (§16–§17), all built from canonical `project.sqlite` rows.
//!
//! One document per logical entity, keyed `DocKey { kind, id }` where `kind`
//! is usually the canonical table. Each document starts from the entity's
//! registered search projection (so liveness, draft rules and private-note
//! ownership match Global Search exactly) and is enriched with the context a
//! reader needs (characters and location of a scene, act/sequence of a card,
//! relationships of a character, …). The same builder emits the entity's graph
//! fragment: `Canonical` edges from foreign keys / link rows, `Derived` edges
//! from deterministic parsing (cue names, scene-heading locations). Nothing is
//! inferred by a model.
//!
//! Privacy rules applied here (the assembler re-checks at read time):
//! - Private notes carry `owner_user_id` and are only ever returned to that user.
//! - Contact details (phone/e-mail/contact fields, emergency contacts) are
//!   never put into documents: exact contact lookups go through deterministic
//!   tools, not retrieved context.
//! - Comments are visible to every project member with View, as in the UI.
//! - Deleted (Recently Deleted) objects have no document.

use std::collections::{BTreeSet, HashMap, HashSet};

use openframe_domain::AppResult;
use openframe_search::chunk;
use openframe_search::{ChunkInput, DocumentInput, EdgeInput, EdgeProvenance, NodeInput};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value, json};

use crate::registry::{Registry, SearchDoc};

/// Version of these builders. Bump when document text/graph rules change:
/// existing intelligence indexes are then rebuilt on open.
pub const BUILDER_VERSION: u32 = 1;

/// Hard bounds per document (a document is a retrieval unit, not a table dump).
const MAX_CHUNKS_PER_DOC: usize = 24;
const HEADER_MAX: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// Screenplay-style: pack whole lines/elements; split only long scenes.
    Lines(usize),
    /// Notes-style: paragraph/section chunks.
    Paragraphs(usize),
}

/// A document kind.
#[derive(Debug)]
pub struct KindInfo {
    pub kind: &'static str,
    /// Canonical table the entity lives in (and whose search indexer seeds the doc).
    pub table: &'static str,
    /// Provenance label shown to users ("Scene", "Character", …).
    pub label: &'static str,
    /// Module label ("Screenplay", "Story", …).
    pub module: &'static str,
    policy: Policy,
}

const fn k(
    kind: &'static str,
    table: &'static str,
    label: &'static str,
    module: &'static str,
    policy: Policy,
) -> KindInfo {
    KindInfo {
        kind,
        table,
        label,
        module,
        policy,
    }
}

/// Every domain the intelligence index covers (§13).
pub const KINDS: &[KindInfo] = &[
    k(
        "project",
        "project",
        "Project",
        "Project",
        Policy::Paragraphs(1400),
    ),
    k(
        "episode",
        "episode",
        "Episode",
        "Project",
        Policy::Paragraphs(1400),
    ),
    k(
        "vault_item",
        "vault_item",
        "Idea Vault item",
        "Idea Vault",
        Policy::Paragraphs(1200),
    ),
    k(
        "screenplay_scene",
        "screenplay_scene",
        "Scene",
        "Screenplay",
        Policy::Lines(1400),
    ),
    k(
        "story_act",
        "story_act",
        "Act",
        "Story",
        Policy::Paragraphs(1400),
    ),
    k(
        "story_sequence",
        "story_sequence",
        "Sequence",
        "Story",
        Policy::Paragraphs(1400),
    ),
    k(
        "story_beat",
        "story_beat",
        "Beat",
        "Story",
        Policy::Paragraphs(1400),
    ),
    k(
        "story_scene_card",
        "story_scene_card",
        "Story Card",
        "Story",
        Policy::Paragraphs(1400),
    ),
    k(
        "story_character",
        "story_character",
        "Character",
        "Story",
        Policy::Paragraphs(1400),
    ),
    k(
        "project_note",
        "project_note",
        "Project note",
        "Notes",
        Policy::Paragraphs(1200),
    ),
    k(
        "private_note",
        "private_note",
        "Private note",
        "Notes",
        Policy::Paragraphs(1200),
    ),
    k("task", "task", "Task", "Tasks", Policy::Paragraphs(1200)),
    k(
        "comment",
        "comment",
        "Comment",
        "Comments",
        Policy::Paragraphs(1200),
    ),
    k(
        "catalog_item",
        "catalog_item",
        "Production item",
        "Production",
        Policy::Paragraphs(1400),
    ),
    k(
        "location",
        "location",
        "Location",
        "Production",
        Policy::Paragraphs(1400),
    ),
    k(
        "cast_member",
        "cast_member",
        "Cast",
        "Production",
        Policy::Paragraphs(1400),
    ),
    k(
        "crew_member",
        "crew_member",
        "Crew",
        "Production",
        Policy::Paragraphs(1400),
    ),
    k(
        "scene_breakdown",
        "breakdown_element",
        "Scene breakdown",
        "Production",
        Policy::Lines(1400),
    ),
    k(
        "shooting_day",
        "shooting_day",
        "Shooting day",
        "Schedule",
        Policy::Lines(1400),
    ),
    k(
        "call_sheet",
        "call_sheet",
        "Call sheet",
        "Call Sheets",
        Policy::Paragraphs(1000),
    ),
    k(
        "side",
        "side",
        "Sides",
        "Call Sheets",
        Policy::Paragraphs(1000),
    ),
    k(
        "moodboard",
        "moodboard",
        "Moodboard",
        "Visual Planning",
        Policy::Paragraphs(1400),
    ),
    k(
        "moodboard_item",
        "moodboard_item",
        "Moodboard note",
        "Visual Planning",
        Policy::Paragraphs(1200),
    ),
    k(
        "storyboard",
        "storyboard",
        "Storyboard",
        "Visual Planning",
        Policy::Lines(1400),
    ),
    k(
        "storyboard_panel",
        "storyboard_panel",
        "Storyboard panel",
        "Visual Planning",
        Policy::Paragraphs(1200),
    ),
    k(
        "shot",
        "shot",
        "Shot",
        "Visual Planning",
        Policy::Paragraphs(1200),
    ),
    k(
        "project_file",
        "project_file",
        "File",
        "Files",
        Policy::Paragraphs(1200),
    ),
];

pub fn kind_info(kind: &str) -> Option<&'static KindInfo> {
    KINDS.iter().find(|k| k.kind == kind)
}

/// Kinds whose documents follow the screenplay's current/production drafts:
/// re-synchronised as a whole when a draft is switched or a production source changes.
pub const SCREENPLAY_KINDS: &[&str] = &[
    "screenplay_scene",
    "scene_breakdown",
    "shooting_day",
    "moodboard",
    "storyboard",
    "shot",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocKey {
    pub kind: &'static str,
    pub id: String,
}

impl DocKey {
    pub fn new(kind: &'static str, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }
    pub fn doc_id(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
    pub fn parse(doc_id: &str) -> Option<DocKey> {
        let (kind, id) = doc_id.split_once(':')?;
        let info = kind_info(kind)?;
        Some(DocKey::new(info.kind, id))
    }
    pub fn from_table(table: &str, id: &str) -> Option<DocKey> {
        KINDS
            .iter()
            .find(|k| k.kind == table && k.table == table)
            .map(|k| DocKey::new(k.kind, id))
    }
    pub fn info(&self) -> &'static KindInfo {
        kind_info(self.kind).expect("DocKey kinds come from KINDS")
    }
}

/// A built document plus its graph fragment.
#[derive(Debug, Clone)]
pub struct Built {
    pub doc: DocumentInput,
    /// Navigation target (same shape as Global Search results).
    pub nav: Value,
    pub own_nodes: Vec<NodeInput>,
    pub referenced: Vec<NodeInput>,
    pub edges: Vec<EdgeInput>,
}

// ------------------------------------------------------------------ helpers

fn text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn short(s: &str, n: usize) -> String {
    chunk::truncate_chars(&text(s), n)
}

fn opt_line(label: &str, v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!("{label}: {}", short(v, 300)))
}

fn node(entity_type: &str, entity_id: &str, label: &str, module: &str) -> NodeInput {
    NodeInput {
        entity_type: entity_type.to_string(),
        entity_id: entity_id.to_string(),
        label: short(label, 140),
        module: module.to_string(),
        owner_user_id: None,
        source_rev: None,
        metadata: json!({}),
    }
}

fn edge(
    from: (&str, &str),
    relation: &str,
    to: (&str, &str),
    provenance: EdgeProvenance,
) -> EdgeInput {
    EdgeInput {
        from: (from.0.to_string(), from.1.to_string()),
        relation: relation.to_string(),
        to: (to.0.to_string(), to.1.to_string()),
        provenance,
        weight: 1.0,
        source_rev: None,
        metadata: json!({}),
    }
}

fn rev(c: &Connection, table: &str, id: &str) -> AppResult<Option<i64>> {
    Ok(c.query_row(
        &format!("SELECT rev FROM \"{}\" WHERE id=?1", table.replace('"', "")),
        [id],
        |r| r.get(0),
    )
    .optional()?)
}

/// Normalised name for deterministic matching ("Railway Station" == "RAILWAY  STATION").
fn norm(s: &str) -> String {
    s.to_uppercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The current-draft scene with this lineage (scenes keep identity across drafts).
fn current_scene_by_lineage(c: &Connection, lineage: &str) -> AppResult<Option<(String, String)>> {
    Ok(c.query_row(
        "SELECT s.id, s.heading FROM screenplay_scene s JOIN screenplay sp ON sp.current_draft_id = s.draft_id
         WHERE s.lineage_id=?1 AND s.deleted_at IS NULL AND sp.deleted_at IS NULL LIMIT 1",
        [lineage],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?)
}

/// The scene node a production/visual object should attach to: the current-draft
/// scene of the same lineage when there is one, else the referenced scene itself.
fn scene_anchor(
    c: &Connection,
    scene_id: Option<&str>,
    lineage: Option<&str>,
) -> AppResult<Option<NodeInput>> {
    if let Some(l) = lineage
        && let Some((id, heading)) = current_scene_by_lineage(c, l)?
    {
        return Ok(Some(node("screenplay_scene", &id, &heading, "Screenplay")));
    }
    let Some(id) = scene_id else { return Ok(None) };
    let row: Option<(String, Option<String>)> = c
        .query_row(
            "SELECT heading, lineage_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((heading, lin)) = row else {
        return Ok(None);
    };
    if lineage.is_none()
        && let Some(l) = lin
        && let Some((cid, ch)) = current_scene_by_lineage(c, &l)?
    {
        return Ok(Some(node("screenplay_scene", &cid, &ch, "Screenplay")));
    }
    Ok(Some(node("screenplay_scene", id, &heading, "Screenplay")))
}

/// Target node for comments/tasks/private notes (their `target_type` is a table name).
fn target_node(c: &Connection, target_type: &str, target_id: &str) -> AppResult<Option<NodeInput>> {
    let Some(info) = KINDS
        .iter()
        .find(|k| k.table == target_type && k.kind == target_type)
    else {
        return Ok(None);
    };
    let live: bool = c
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE id=?1 AND deleted_at IS NULL)",
                info.table
            ),
            [target_id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    Ok(live.then(|| node(info.kind, target_id, info.label, info.module)))
}

// ------------------------------------------------------------------ builder

/// Accumulates one document while it is enriched.
struct Draft {
    info: &'static KindInfo,
    project_id: String,
    id: String,
    title: String,
    body: String,
    /// Replaces `body` chunking with explicit logical units (sections).
    units: Option<Vec<String>>,
    header: Vec<String>,
    meta: Map<String, Value>,
    nav: Value,
    owner: Option<String>,
    scope_key: Option<String>,
    source_rev: Option<i64>,
    deps: Vec<(String, String)>,
    own_node: bool,
    referenced: Vec<NodeInput>,
    edges: Vec<EdgeInput>,
}

impl Draft {
    fn new(info: &'static KindInfo, project_id: &str, id: &str, base: SearchDoc) -> Self {
        let mut meta = Map::new();
        if !base.context.is_empty() {
            meta.insert("context".into(), json!(base.context));
        }
        Self {
            info,
            project_id: project_id.to_string(),
            id: id.to_string(),
            title: text(&base.title),
            body: base.body,
            units: None,
            header: Vec::new(),
            meta,
            nav: base.nav,
            owner: base.owner_user_id,
            scope_key: None,
            source_rev: None,
            deps: Vec::new(),
            own_node: true,
            referenced: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn line(&mut self, l: Option<String>) {
        if let Some(l) = l {
            self.header.push(l);
        }
    }

    fn me(&self) -> (&str, &str) {
        (self.info.kind, self.id.as_str())
    }

    fn link_to(&mut self, relation: &str, to: NodeInput, prov: EdgeProvenance) {
        let e = edge(self.me(), relation, (&to.entity_type, &to.entity_id), prov);
        self.edges.push(e);
        self.referenced.push(to);
    }

    fn link_from(&mut self, from: NodeInput, relation: &str, prov: EdgeProvenance) {
        let e = edge(
            (&from.entity_type, &from.entity_id),
            relation,
            self.me(),
            prov,
        );
        self.edges.push(e);
        self.referenced.push(from);
    }

    fn finish(self) -> Built {
        let header = {
            let mut h = vec![format!("{}: {}", self.info.label, self.title)];
            h.extend(self.header.iter().cloned());
            chunk::truncate_chars(&h.join("\n"), HEADER_MAX)
        };
        let pieces: Vec<String> = match (&self.units, self.info.policy) {
            (Some(units), Policy::Lines(max) | Policy::Paragraphs(max)) => {
                chunk::pack_units(units.iter().map(|s| s.as_str()), max, "\n\n")
            }
            (None, Policy::Lines(max)) => chunk::lines(&self.body, max),
            (None, Policy::Paragraphs(max)) => chunk::paragraphs(&self.body, max),
        };
        let mut chunks: Vec<ChunkInput> = pieces
            .into_iter()
            .take(MAX_CHUNKS_PER_DOC)
            .enumerate()
            .map(|(i, p)| ChunkInput {
                text: format!("{header}\n{p}"),
                metadata: json!({ "part": i + 1 }),
            })
            .collect();
        if chunks.is_empty() {
            chunks.push(ChunkInput {
                text: header.clone(),
                metadata: json!({ "part": 1 }),
            });
        }
        let body = match &self.units {
            Some(u) => u.join("\n\n"),
            None => self.body.clone(),
        };
        let mut meta = self.meta;
        meta.insert("nav".into(), self.nav.clone());
        meta.insert("header".into(), json!(self.header));
        let own_nodes = if self.own_node {
            vec![NodeInput {
                entity_type: self.info.kind.to_string(),
                entity_id: self.id.clone(),
                label: short(&self.title, 140),
                module: self.info.module.to_string(),
                owner_user_id: self.owner.clone(),
                source_rev: self.source_rev,
                metadata: json!({}),
            }]
        } else {
            Vec::new()
        };
        Built {
            doc: DocumentInput {
                doc_id: format!("{}:{}", self.info.kind, self.id),
                entity_type: self.info.kind.to_string(),
                entity_id: self.id.clone(),
                source_table: self.info.table.to_string(),
                project_id: self.project_id,
                module: self.info.module.to_string(),
                title: self.title,
                body,
                owner_user_id: self.owner,
                source_rev: self.source_rev,
                scope_key: self.scope_key,
                metadata: Value::Object(meta),
                chunks,
                dependencies: self.deps,
            },
            nav: self.nav,
            own_nodes,
            referenced: self.referenced,
            edges: self.edges,
        }
    }
}

/// Build the document + graph fragment for `key`, or None when the entity does
/// not exist, is deleted, is not indexable (e.g. a scene of a non-current
/// draft) or has nothing meaningful to say.
pub fn build(
    c: &Connection,
    registry: &Registry,
    project_id: &str,
    key: &DocKey,
) -> AppResult<Option<Built>> {
    match key.kind {
        "project" => project_doc(c, project_id, &key.id),
        "scene_breakdown" => scene_breakdown(c, project_id, &key.id),
        kind => {
            let Some(info) = kind_info(kind) else {
                return Ok(None);
            };
            let Some(indexer) = registry.indexer_for(info.table) else {
                return Ok(None);
            };
            let Some(base) = indexer(c, &key.id)? else {
                return Ok(None);
            };
            let mut d = Draft::new(info, project_id, &key.id, base);
            d.source_rev = rev(c, info.table, &key.id)?;
            let keep = enrich(c, &mut d)?;
            Ok(keep.then(|| d.finish()))
        }
    }
}

/// Per-kind enrichment. Returns false to drop the document.
fn enrich(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    match d.info.kind {
        "screenplay_scene" => scene(c, d),
        "story_scene_card" => card(c, d),
        "story_sequence" => sequence(c, d),
        "story_beat" => beat(c, d),
        "story_act" => act(c, d),
        "story_character" => character(c, d),
        "cast_member" => cast(c, d),
        "crew_member" => crew(c, d),
        "location" => location(c, d),
        "catalog_item" => catalog(c, d),
        "shooting_day" => shooting_day(c, d),
        "call_sheet" => call_sheet(c, d),
        "side" => side(c, d),
        "moodboard" | "storyboard" => visual_board(c, d),
        "moodboard_item" => parent_link(c, d, "moodboard_id", "moodboard", "Moodboard"),
        "storyboard_panel" => panel(c, d),
        "shot" => shot(c, d),
        "task" => task(c, d),
        "comment" => comment(c, d),
        "private_note" => private_note(c, d),
        "project_note" => project_note(c, d),
        "vault_item" => vault_item(c, d),
        "project_file" => project_file(c, d),
        "episode" => Ok(true),
        _ => Ok(true),
    }
}

fn scene(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, String, Option<String>, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT draft_id, heading, synopsis, story_day, source_scene_card_id FROM screenplay_scene WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((draft_id, heading, synopsis, story_day, source_card)) = row else {
        return Ok(false);
    };
    let (draft_name, screenplay_id): (String, String) = c.query_row(
        "SELECT name, screenplay_id FROM screenplay_draft WHERE id=?1",
        [&draft_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let number = crate::modules::screenplay::scene_number(c, &d.id)?.unwrap_or(0);
    let parsed = crate::modules::schedule::source::parse_heading(&heading);
    // Characters: cue elements, linked to Story characters canonically where the
    // writer confirmed the link, else by exact (normalised) name.
    let mut cues: Vec<String> = Vec::new();
    {
        let mut stmt = c.prepare_cached(
            "SELECT text FROM screenplay_element WHERE scene_id=?1 AND element_type='character' ORDER BY position",
        )?;
        let rows = stmt.query_map([&d.id], |r| r.get::<_, String>(0))?;
        for r in rows {
            let cue = crate::modules::screenplay::characters::normalize_cue(&r?);
            if !cue.is_empty() && !cues.contains(&cue) && cues.len() < 40 {
                cues.push(cue);
            }
        }
    }
    let links: HashMap<String, (Option<String>, bool)> = {
        let mut stmt = c.prepare_cached(
            "SELECT cue_name, character_id, ignored FROM screenplay_character_link WHERE screenplay_id=?1",
        )?;
        stmt.query_map([&screenplay_id], |r| {
            Ok((
                crate::modules::screenplay::characters::normalize_cue(&r.get::<_, String>(0)?),
                (r.get(1)?, r.get::<_, i64>(2)? != 0),
            ))
        })?
        .collect::<Result<_, _>>()?
    };
    let story_chars: Vec<(String, String)> = {
        let mut stmt =
            c.prepare_cached("SELECT id, name FROM story_character WHERE deleted_at IS NULL")?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    let by_name: HashMap<String, (String, String)> = story_chars
        .iter()
        .map(|(id, name)| {
            (
                crate::modules::screenplay::characters::normalize_cue(name),
                (id.clone(), name.clone()),
            )
        })
        .collect();
    let live: HashSet<&str> = story_chars.iter().map(|(id, _)| id.as_str()).collect();
    let mut names = Vec::new();
    for cue in &cues {
        let linked = links.get(cue);
        if linked.is_some_and(|(_, ignored)| *ignored) {
            names.push(cue.clone());
            continue;
        }
        match linked
            .and_then(|(id, _)| id.clone())
            .filter(|id| live.contains(id.as_str()))
        {
            Some(id) => {
                let name = story_chars
                    .iter()
                    .find(|(i, _)| *i == id)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| cue.clone());
                names.push(name.clone());
                d.link_from(
                    node("story_character", &id, &name, "Story"),
                    "appears_in",
                    EdgeProvenance::Canonical,
                );
            }
            None => match by_name.get(cue) {
                Some((id, name)) => {
                    names.push(name.clone());
                    d.link_from(
                        node("story_character", id, name, "Story"),
                        "appears_in",
                        EdgeProvenance::Derived,
                    );
                }
                None => names.push(cue.clone()),
            },
        }
    }
    // Location: deterministic match of the heading's location against Production locations.
    let loc_norm = norm(&parsed.location);
    if !loc_norm.is_empty() {
        let mut stmt = c.prepare_cached(
            "SELECT id, name FROM location WHERE deleted_at IS NULL AND archived=0",
        )?;
        let locs: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, name) in locs {
            let n = norm(&name);
            let hit = !n.is_empty()
                && (n == loc_norm || format!(" {loc_norm} ").contains(&format!(" {n} ")));
            if hit {
                d.link_to(
                    "located_at",
                    node("location", &id, &name, "Production"),
                    EdgeProvenance::Derived,
                );
            }
        }
    }
    // Story cards representing this scene.
    let mut cards: Vec<(String, String, String, Option<String>)> = Vec::new();
    {
        let mut stmt = c.prepare_cached(
            "SELECT id, short_description, parent_type, parent_id FROM story_scene_card
             WHERE deleted_at IS NULL AND (screenplay_scene_id=?1 OR id=?2) ORDER BY position LIMIT 4",
        )?;
        let rows = stmt.query_map(params![d.id, source_card], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;
        for r in rows {
            cards.push(r?);
        }
    }
    let story_place = match cards.first() {
        Some((_, _, pt, pid)) => story_path(c, pt, pid.as_deref())?,
        None => None,
    };
    for (id, desc, _, _) in &cards {
        d.link_to(
            "represented_by",
            node("story_scene_card", id, desc, "Story"),
            EdgeProvenance::Canonical,
        );
    }
    d.link_to(
        "belongs_to",
        node("screenplay_draft", &draft_id, &draft_name, "Screenplay"),
        EdgeProvenance::Canonical,
    );
    d.scope_key = Some(draft_id.clone());
    d.line(Some(format!("Scene {number} · {}", short(&draft_name, 80))));
    d.line((!names.is_empty()).then(|| format!("Characters: {}", names.join(", "))));
    d.line(opt_line("Location", Some(&parsed.location)));
    d.line(opt_line("Time", Some(&parsed.time_label)));
    d.line(opt_line("Story day", story_day.as_deref()));
    d.line(story_place.map(|p| format!("Story: {p}")));
    d.line(opt_line("Synopsis", synopsis.as_deref()));
    d.meta.insert("draftId".into(), json!(draft_id));
    d.meta.insert("number".into(), json!(number));
    d.meta.insert("heading".into(), json!(heading));
    d.meta.insert("characters".into(), json!(names));
    d.meta.insert("location".into(), json!(parsed.location));
    Ok(true)
}

/// "Act 2 › Collapse" for a card/beat parent.
fn story_path(
    c: &Connection,
    parent_type: &str,
    parent_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(pid) = parent_id else {
        return Ok(match parent_type {
            "parking" => Some("Parking Lot".into()),
            "unassigned" => Some("Unassigned".into()),
            _ => None,
        });
    };
    Ok(match parent_type {
        "act" => c
            .query_row("SELECT title FROM story_act WHERE id=?1 AND deleted_at IS NULL", [pid], |r| {
                r.get::<_, String>(0)
            })
            .optional()?,
        "sequence" => c
            .query_row(
                "SELECT a.title, s.title FROM story_sequence s LEFT JOIN story_act a ON a.id = s.act_id
                 WHERE s.id=?1 AND s.deleted_at IS NULL",
                [pid],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
            .map(|(a, s)| match a {
                Some(a) => format!("{a} › {s}"),
                None => s,
            }),
        _ => None,
    })
}

fn parent_node(c: &Connection, parent_type: &str, parent_id: &str) -> AppResult<Option<NodeInput>> {
    let (table, label) = match parent_type {
        "act" => ("story_act", "Act"),
        "sequence" => ("story_sequence", "Sequence"),
        _ => return Ok(None),
    };
    let title: Option<String> = c
        .query_row(
            &format!("SELECT title FROM {table} WHERE id=?1 AND deleted_at IS NULL"),
            [parent_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(title.map(|t| node(table, parent_id, &format!("{label}: {t}"), "Story")))
}

fn card(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, Option<String>, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT parent_type, parent_id, source_beat_id, source_vault_item_id FROM story_scene_card WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((pt, pid, beat, vault)) = row else {
        return Ok(false);
    };
    d.line(story_path(c, &pt, pid.as_deref())?.map(|p| format!("Story: {p}")));
    if let Some(pid) = &pid
        && let Some(p) = parent_node(c, &pt, pid)?
    {
        d.link_to("belongs_to", p, EdgeProvenance::Canonical);
    }
    let chars: Vec<(String, String)> = {
        let mut stmt = c.prepare_cached(
            "SELECT ch.id, ch.name FROM story_character_card_link l JOIN story_character ch ON ch.id = l.character_id
             WHERE l.scene_card_id=?1 AND ch.deleted_at IS NULL ORDER BY ch.position",
        )?;
        stmt.query_map([&d.id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?
    };
    d.line((!chars.is_empty()).then(|| {
        format!(
            "Characters: {}",
            chars
                .iter()
                .map(|(_, n)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }));
    for (id, name) in chars {
        d.link_from(
            node("story_character", &id, &name, "Story"),
            "appears_in",
            EdgeProvenance::Canonical,
        );
    }
    if let Some(v) = vault
        && let Some(n) = target_node(c, "vault_item", &v)?
    {
        d.link_to("derived_from", n, EdgeProvenance::Canonical);
    }
    if let Some(b) = beat
        && let Some(n) = target_node(c, "story_beat", &b)?
    {
        d.link_to("derived_from", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn sequence(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let act: Option<(String, String)> = c
        .query_row(
            "SELECT a.id, a.title FROM story_sequence s JOIN story_act a ON a.id = s.act_id
             WHERE s.id=?1 AND a.deleted_at IS NULL",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((id, title)) = act {
        d.line(Some(format!("Act: {}", short(&title, 120))));
        d.link_to(
            "belongs_to",
            node("story_act", &id, &format!("Act: {title}"), "Story"),
            EdgeProvenance::Canonical,
        );
    }
    Ok(true)
}

fn beat(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, Option<String>, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT parent_type, parent_id, converted_scene_card_id, source_vault_item_id FROM story_beat WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((pt, pid, converted, vault)) = row else {
        return Ok(false);
    };
    d.line(story_path(c, &pt, pid.as_deref())?.map(|p| format!("Story: {p}")));
    if let Some(pid) = &pid
        && let Some(p) = parent_node(c, &pt, pid)?
    {
        d.link_to("belongs_to", p, EdgeProvenance::Canonical);
    }
    if let Some(card) = converted
        && let Some(n) = target_node(c, "story_scene_card", &card)?
    {
        d.link_to("converted_to", n, EdgeProvenance::Canonical);
    }
    if let Some(v) = vault
        && let Some(n) = target_node(c, "vault_item", &v)?
    {
        d.link_to("derived_from", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn act(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let ep: Option<(String, String)> = c
        .query_row(
            "SELECT e.id, e.title FROM story_act a JOIN episode e ON e.id = a.episode_id WHERE a.id=?1 AND e.deleted_at IS NULL",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((id, title)) = ep {
        d.line(Some(format!("Episode: {}", short(&title, 120))));
        d.link_to(
            "belongs_to",
            node("episode", &id, &title, "Project"),
            EdgeProvenance::Canonical,
        );
    }
    Ok(true)
}

fn character(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let role: Option<Option<String>> = c
        .query_row(
            "SELECT role_label FROM story_character WHERE id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(role) = role else { return Ok(false) };
    d.line(opt_line("Role", role.as_deref()));
    let rels: Vec<(String, String, String, bool)> = {
        let mut stmt = c.prepare_cached(
            "SELECT o.id, o.name, r.relationship_type, r.from_character_id = ?1
             FROM story_character_relationship r
             JOIN story_character o ON o.id = CASE WHEN r.from_character_id = ?1 THEN r.to_character_id ELSE r.from_character_id END
             WHERE (r.from_character_id = ?1 OR r.to_character_id = ?1) AND o.deleted_at IS NULL
             ORDER BY r.position LIMIT 24",
        )?;
        stmt.query_map([&d.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?
    };
    if !rels.is_empty() {
        d.line(Some(format!(
            "Relationships: {}",
            rels.iter()
                .map(|(_, n, t, _)| format!("{n} ({})", short(t, 40)))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    for (id, name, rtype, outgoing) in rels {
        // Each relationship edge is owned by its "from" character's fragment.
        if outgoing {
            let mut e = edge(
                ("story_character", &d.id),
                "related_to",
                ("story_character", &id),
                EdgeProvenance::Canonical,
            );
            e.metadata = json!({ "type": rtype });
            d.edges.push(e);
            d.referenced
                .push(node("story_character", &id, &name, "Story"));
        }
    }
    let cast: Vec<String> = {
        let mut stmt = c.prepare_cached(
            "SELECT person_name FROM cast_member WHERE character_id=?1 AND deleted_at IS NULL AND archived=0 ORDER BY is_primary DESC LIMIT 6",
        )?;
        stmt.query_map([&d.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    d.line((!cast.is_empty()).then(|| format!("Played by: {}", cast.join(", "))));
    let cards: Vec<String> = {
        let mut stmt = c.prepare_cached(
            "SELECT sc.short_description FROM story_character_card_link l JOIN story_scene_card sc ON sc.id = l.scene_card_id
             WHERE l.character_id=?1 AND sc.deleted_at IS NULL ORDER BY sc.position LIMIT 6",
        )?;
        stmt.query_map([&d.id], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .map(|s| short(&s, 90))
            .filter(|s| !s.is_empty())
            .collect()
    };
    if !cards.is_empty() {
        d.units = None;
        let related = format!("On story cards: {}", cards.join(" | "));
        d.body = if d.body.trim().is_empty() {
            related
        } else {
            format!("{}\n\n{related}", d.body)
        };
    }
    Ok(true)
}

fn cast(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT person_name, character_id, character_name, availability_notes, notes FROM cast_member WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((person, char_id, char_name, availability, notes)) = row else {
        return Ok(false);
    };
    let mut character_label = char_name.clone();
    if let Some(id) = char_id {
        let name: Option<String> = c
            .query_row(
                "SELECT name FROM story_character WHERE id=?1 AND deleted_at IS NULL",
                [&id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(n) = name {
            character_label = Some(n.clone());
            d.link_to(
                "plays",
                node("story_character", &id, &n, "Story"),
                EdgeProvenance::Canonical,
            );
        }
    }
    // Contact details are deliberately left out (see module docs).
    d.body = [
        Some(match &character_label {
            Some(ch) => format!("{person} plays {ch}."),
            None => format!("{person} (cast)."),
        }),
        opt_line("Availability", availability.as_deref()),
        opt_line("Notes", notes.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n");
    Ok(true)
}

fn crew(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, String, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT person_name, role, department, notes FROM crew_member WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((person, role, dept, notes)) = row else {
        return Ok(false);
    };
    d.body = [
        Some(match dept.as_deref().filter(|x| !x.trim().is_empty()) {
            Some(dp) => format!("{person} — {role} ({dp})."),
            None => format!("{person} — {role}."),
        }),
        opt_line("Notes", notes.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n");
    Ok(true)
}

/// Flatten a notes JSON object ({"access": "...", "power": "..."}) into lines.
fn json_notes(v: &str) -> Vec<String> {
    let Ok(Value::Object(m)) = serde_json::from_str::<Value>(v) else {
        return Vec::new();
    };
    m.iter()
        .filter_map(|(k, v)| {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Null => return None,
                other => other.to_string(),
            };
            let s = s.trim();
            if s.is_empty()
                || k.to_lowercase().contains("contact")
                || k.to_lowercase().contains("phone")
            {
                None
            } else {
                Some(format!("{}: {}", k.replace('_', " "), short(s, 600)))
            }
        })
        .collect()
}

fn location(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, Option<String>, String, String, Option<String>)> = c
        .query_row(
            "SELECT name, address, status, notes_json, replacement_location_id FROM location WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((name, address, status, notes, replacement)) = row else {
        return Ok(false);
    };
    d.line(Some(format!("Status: {status}")));
    let mut body = vec![format!("{name}.")];
    if let Some(a) = opt_line("Address", address.as_deref()) {
        body.push(a);
    }
    body.extend(json_notes(&notes));
    d.body = body.join("\n\n");
    if let Some(r) = replacement
        && let Some(n) = target_node(c, "location", &r)?
    {
        d.link_to("replaced_by", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn catalog(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, String, Option<String>, Option<String>, String, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT category, name, description, notes, status, character_id, location_id FROM catalog_item WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?;
    let Some((category, name, description, notes, status, char_id, loc_id)) = row else {
        return Ok(false);
    };
    let aliases: Vec<String> = {
        let mut stmt = c.prepare_cached(
            "SELECT alias FROM catalog_alias WHERE catalog_item_id=?1 ORDER BY alias LIMIT 12",
        )?;
        stmt.query_map([&d.id], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    d.line(Some(format!("Category: {category} · Status: {status}")));
    d.line((!aliases.is_empty()).then(|| format!("Also called: {}", aliases.join(", "))));
    d.body = [
        Some(format!("{name} ({category}).")),
        description.filter(|s| !s.trim().is_empty()),
        opt_line("Notes", notes.as_deref()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n");
    if let Some(ch) = char_id
        && let Some(n) = target_node(c, "story_character", &ch)?
    {
        d.link_to("associated_with", n, EdgeProvenance::Canonical);
    }
    if let Some(l) = loc_id
        && let Some(n) = target_node(c, "location", &l)?
    {
        d.link_to("associated_with", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn shooting_day(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let mut stmt = c.prepare_cached(
        "SELECT id, scene_id, scene_lineage_id, source_heading FROM schedule_strip
         WHERE day_id=?1 AND deleted_at IS NULL AND archived=0 ORDER BY position LIMIT 60",
    )?;
    let strips: Vec<(String, String, String, String)> = stmt
        .query_map([&d.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<Result<_, _>>()?;
    let mut lines = Vec::new();
    for (strip_id, scene_id, lineage, heading) in &strips {
        d.deps.push(("schedule_strip".into(), strip_id.clone()));
        if let Some(anchor) = scene_anchor(c, Some(scene_id), Some(lineage))? {
            lines.push(format!("Scene: {}", short(&anchor.label, 140)));
            d.link_from(anchor, "scheduled_on", EdgeProvenance::Canonical);
        } else if !heading.trim().is_empty() {
            lines.push(format!("Scene: {}", short(heading, 140)));
        }
    }
    let mut stmt = c.prepare_cached(
        "SELECT marker_type, label, at_time FROM schedule_marker WHERE day_id=?1 AND deleted_at IS NULL ORDER BY position LIMIT 20",
    )?;
    let markers: Vec<(String, String, Option<String>)> = stmt
        .query_map([&d.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (t, l, at) in markers {
        lines.push(match at {
            Some(at) => format!("{t}: {} at {at}", short(&l, 80)),
            None => format!("{t}: {}", short(&l, 80)),
        });
    }
    if !lines.is_empty() {
        d.body = if d.body.trim().is_empty() {
            lines.join("\n")
        } else {
            format!("{}\n{}", d.body, lines.join("\n"))
        };
    }
    Ok(true)
}

fn call_sheet(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    use crate::modules::schedule::callsheet::CallSheetDocument;
    let row: Option<(String, String, String)> = c
        .query_row(
            "SELECT shoot_day_id, status, document_json FROM call_sheet WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((day_id, status, json_doc)) = row else {
        return Ok(false);
    };
    d.line(Some(format!("Status: {status}")));
    if let Some(n) = target_node(c, "shooting_day", &day_id)? {
        d.link_to("for_day", n, EdgeProvenance::Canonical);
    }
    // Logical sections (§14). Emergency contacts and other contact fields are left out.
    if let Ok(doc) = serde_json::from_str::<CallSheetDocument>(&json_doc) {
        let mut sections = Vec::new();
        let mut head = vec![format!(
            "{} {}",
            doc.day_label,
            doc.date.clone().unwrap_or_default()
        )];
        if !doc.crew_call.trim().is_empty() {
            head.push(format!("Crew call: {}", doc.crew_call));
        }
        sections.push(head.join("\n"));
        if !doc.scenes.is_empty() {
            sections.push(format!(
                "Scenes:\n{}",
                doc.scenes
                    .iter()
                    .map(|s| format!(
                        "{} {} — {}",
                        s.number,
                        s.heading,
                        short(&s.description, 200)
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !doc.cast.is_empty() {
            sections.push(format!(
                "Cast:\n{}",
                doc.cast
                    .iter()
                    .map(|x| format!("{} as {} — call {}", x.actor, x.character, x.call_time))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !doc.locations.is_empty() {
            sections.push(format!(
                "Locations:\n{}",
                doc.locations
                    .iter()
                    .map(|l| {
                        let mut s = l.name.clone();
                        for (label, v) in [
                            ("address", &l.address),
                            ("meet", &l.meeting_point),
                            ("parking", &l.parking),
                        ] {
                            if !v.trim().is_empty() {
                                s.push_str(&format!("; {label}: {}", short(v, 160)));
                            }
                        }
                        s
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let p = &doc.practical;
        let notes: Vec<String> = [
            opt_line("Day notes", Some(&doc.day_notes)),
            opt_line("Production notes", Some(&p.production_notes)),
            opt_line("Travel", Some(&p.travel_notes)),
            opt_line("Meal break", Some(&p.meal_break)),
            opt_line("Weather", doc.optional.weather.as_deref()),
            opt_line("Special notes", doc.optional.special_notes.as_deref()),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !notes.is_empty() {
            sections.push(notes.join("\n"));
        }
        d.units = Some(sections);
    }
    Ok(true)
}

fn side(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let day: Option<Option<String>> = c
        .query_row("SELECT shoot_day_id FROM side WHERE id=?1", [&d.id], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(Some(day)) = day
        && let Some(n) = target_node(c, "shooting_day", &day)?
    {
        d.link_to("for_day", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn visual_board(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let table = d.info.table;
    let row: Option<(Option<String>, Option<String>)> = c
        .query_row(
            &format!("SELECT scene_id, scene_lineage_id FROM {table} WHERE id=?1"),
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((scene, lineage)) = row
        && let Some(anchor) = scene_anchor(c, scene.as_deref(), lineage.as_deref())?
    {
        d.line(Some(format!("For scene: {}", short(&anchor.label, 140))));
        d.link_to("for_scene", anchor, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn parent_link(
    c: &Connection,
    d: &mut Draft,
    col: &str,
    parent_kind: &str,
    label: &str,
) -> AppResult<bool> {
    let table = d.info.table;
    let parent: Option<String> = c
        .query_row(
            &format!("SELECT {col} FROM {table} WHERE id=?1"),
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(p) = parent
        && let Some(n) = target_node(c, parent_kind, &p)?
    {
        let mut n = n;
        let name: Option<String> = c
            .query_row(
                &format!("SELECT name FROM {parent_kind} WHERE id=?1"),
                [&p],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(name) = name {
            d.line(Some(format!("{label}: {}", short(&name, 120))));
            n.label = short(&name, 140);
        }
        d.link_to("part_of", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn panel(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    parent_link(c, d, "storyboard_id", "storyboard", "Storyboard")?;
    let shot: Option<Option<String>> = c
        .query_row(
            "SELECT shot_id FROM storyboard_panel WHERE id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(Some(s)) = shot
        && let Some(n) = target_node(c, "shot", &s)?
    {
        d.link_to("depicts", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn shot(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = c
        .query_row(
            "SELECT scene_id, scene_lineage_id, size, movement, angle FROM shot WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let Some((scene, lineage, size, movement, angle)) = row else {
        return Ok(false);
    };
    let spec: Vec<String> = [size, movement, angle]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    d.line((!spec.is_empty()).then(|| format!("Camera: {}", spec.join(", "))));
    if let Some(anchor) = scene_anchor(c, Some(&scene), Some(&lineage))? {
        d.line(Some(format!("For scene: {}", short(&anchor.label, 140))));
        d.link_to("for_scene", anchor, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn task(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT status, target_type, target_id FROM task WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((status, tt, tid)) = row else {
        return Ok(false);
    };
    d.line(Some(format!("Status: {status}")));
    if let (Some(tt), Some(tid)) = (tt, tid)
        && let Some(n) = target_node(c, &tt, &tid)?
    {
        d.link_to("about", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn comment(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let row: Option<(String, String, String)> = c
        .query_row(
            "SELECT status, target_type, target_id FROM comment WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((status, tt, tid)) = row else {
        return Ok(false);
    };
    d.line(Some(format!("Status: {status}")));
    // Replies are part of the root comment's document.
    let mut stmt = c.prepare_cached("SELECT id FROM comment WHERE parent_id=?1")?;
    let replies: Vec<String> = stmt
        .query_map([&d.id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for r in replies {
        d.deps.push(("comment".into(), r));
    }
    if let Some(n) = target_node(c, &tt, &tid)? {
        d.link_to("on", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn private_note(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    if d.owner.is_none() {
        // A private note without an owner must never become shared context.
        return Ok(false);
    }
    let row: Option<(Option<String>, Option<String>)> = c
        .query_row(
            "SELECT target_type, target_id FROM private_note WHERE id=?1",
            [&d.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((Some(tt), Some(tid))) = row
        && let Some(n) = target_node(c, &tt, &tid)?
    {
        d.link_to("about", n, EdgeProvenance::Canonical);
    }
    Ok(true)
}

fn project_note(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let pinned: Option<bool> = c
        .query_row(
            "SELECT pinned FROM project_note WHERE id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    d.line(pinned.filter(|p| *p).map(|_| "Pinned".to_string()));
    Ok(true)
}

fn vault_item(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let kind: Option<String> = c
        .query_row(
            "SELECT item_type FROM vault_item WHERE id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    d.line(kind.map(|k| format!("Type: {k}")));
    let mut stmt =
        c.prepare_cached("SELECT tag FROM vault_item_tag WHERE item_id=?1 ORDER BY tag LIMIT 20")?;
    let tags: Vec<String> = stmt
        .query_map([&d.id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    d.line((!tags.is_empty()).then(|| format!("Tags: {}", tags.join(", "))));
    Ok(true)
}

fn project_file(c: &Connection, d: &mut Draft) -> AppResult<bool> {
    let folder: Option<String> = c
        .query_row(
            "SELECT f.name FROM project_file p JOIN project_file_folder f ON f.id = p.folder_id WHERE p.id=?1",
            [&d.id],
            |r| r.get(0),
        )
        .optional()?;
    d.line(folder.map(|f| format!("Folder: {}", short(&f, 120))));
    Ok(true)
}

/// Project overview document (title, type, genre, logline…).
fn project_doc(c: &Connection, project_id: &str, id: &str) -> AppResult<Option<Built>> {
    let row: Option<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>)> = c
        .query_row(
            "SELECT title, project_type, status, genre, language, creator, logline FROM project WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?;
    let Some((title, ptype, status, genre, language, creator, logline)) = row else {
        return Ok(None);
    };
    let info = kind_info("project").expect("project kind");
    let base = SearchDoc {
        entity_type: "project".into(),
        title,
        body: logline.clone().unwrap_or_default(),
        context: "Project".into(),
        nav: json!({ "workspace": "home" }),
        owner_user_id: None,
    };
    let mut d = Draft::new(info, project_id, id, base);
    d.source_rev = rev(c, "project", id)?;
    d.line(Some(format!("{ptype} · Status: {status}")));
    d.line(opt_line("Genre", genre.as_deref()));
    d.line(opt_line("Language", language.as_deref()));
    d.line(opt_line("Created by", creator.as_deref()));
    if let Some(l) = logline.filter(|l| !l.trim().is_empty()) {
        d.body = format!("Logline: {l}");
    }
    Ok(Some(d.finish()))
}

/// The breakdown of one production-source scene as one document (§13 "breakdown elements").
fn scene_breakdown(c: &Connection, project_id: &str, scene_id: &str) -> AppResult<Option<Built>> {
    let source: Option<(String, String)> = c
        .query_row(
            "SELECT id, draft_id FROM production_source WHERE active=1 ORDER BY selected_at DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((source_id, source_draft)) = source else {
        return Ok(None);
    };
    let scene: Option<(String, String, String)> = c
        .query_row(
            "SELECT draft_id, heading, lineage_id FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
            [scene_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((draft_id, heading, lineage)) = scene else {
        return Ok(None);
    };
    if draft_id != source_draft {
        return Ok(None);
    }
    let mut stmt = c.prepare_cached(
        "SELECT e.id, e.category, e.display_name, e.notes, e.confirmation_state, e.catalog_item_id, ci.name
         FROM breakdown_element e LEFT JOIN catalog_item ci ON ci.id = e.catalog_item_id AND ci.deleted_at IS NULL
         WHERE e.scene_id=?1 AND e.source_id=?2 AND e.deleted_at IS NULL AND e.archived=0
           AND e.confirmation_state <> 'Rejected'
         ORDER BY e.category, e.display_name LIMIT 200",
    )?;
    #[allow(clippy::type_complexity)]
    let elements: Vec<(
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    )> = stmt
        .query_map(params![scene_id, source_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if elements.is_empty() {
        return Ok(None);
    }
    let number = crate::modules::screenplay::scene_number(c, scene_id)?.unwrap_or(0);
    let info = kind_info("scene_breakdown").expect("breakdown kind");
    let base = SearchDoc {
        entity_type: "scene_breakdown".into(),
        title: format!("Scene {number} — {}", text(&heading)),
        body: String::new(),
        context: "Breakdown".into(),
        nav: json!({ "workspace": "breakdown", "sceneId": scene_id, "sceneLineageId": lineage }),
        owner_user_id: None,
    };
    let mut d = Draft::new(info, project_id, scene_id, base);
    d.own_node = false;
    d.scope_key = Some(draft_id);
    let anchor = scene_anchor(c, Some(scene_id), Some(&lineage))?
        .unwrap_or_else(|| node("screenplay_scene", scene_id, &heading, "Screenplay"));
    let mut lines = Vec::new();
    let mut seen_items = BTreeSet::new();
    for (id, category, name, notes, state, item, item_name) in elements {
        d.deps.push(("breakdown_element".into(), id));
        let mut l = format!("{category}: {}", short(&name, 120));
        if state == "Suggested" {
            l.push_str(" (suggested)");
        }
        if let Some(n) = notes.filter(|n| !n.trim().is_empty()) {
            l.push_str(&format!(" — {}", short(&n, 200)));
        }
        lines.push(l);
        if let (Some(item), Some(item_name)) = (item, item_name)
            && seen_items.insert(item.clone())
        {
            let mut e = edge(
                (&anchor.entity_type, &anchor.entity_id),
                "requires",
                ("catalog_item", &item),
                EdgeProvenance::Canonical,
            );
            e.metadata = json!({ "category": category });
            d.edges.push(e);
            d.referenced
                .push(node("catalog_item", &item, &item_name, "Production"));
        }
    }
    d.referenced.push(anchor);
    d.body = lines.join("\n");
    Ok(Some(d.finish()))
}

// ------------------------------------------------------------ enumeration

/// SQL listing the live entity ids of a kind.
fn list_sql(kind: &str) -> Option<String> {
    Some(match kind {
        "project" => "SELECT id FROM project".into(),
        "screenplay_scene" => "SELECT s.id FROM screenplay_scene s JOIN screenplay sp ON sp.current_draft_id = s.draft_id
                               WHERE s.deleted_at IS NULL AND sp.deleted_at IS NULL"
            .into(),
        "scene_breakdown" => "SELECT DISTINCT e.scene_id FROM breakdown_element e
                              JOIN production_source ps ON ps.id = e.source_id AND ps.active = 1
                              WHERE e.deleted_at IS NULL"
            .into(),
        "comment" => "SELECT id FROM comment WHERE deleted_at IS NULL AND parent_id IS NULL".into(),
        other => {
            let info = kind_info(other)?;
            format!("SELECT id FROM \"{}\" WHERE deleted_at IS NULL", info.table)
        }
    })
}

/// Live entity keys of one kind (tables that don't exist yield nothing).
pub fn keys_of_kind(c: &Connection, kind: &'static str) -> AppResult<Vec<DocKey>> {
    let Some(sql) = list_sql(kind) else {
        return Ok(Vec::new());
    };
    let mut stmt = match c.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return Ok(Vec::new()),
    };
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(ids.into_iter().map(|id| DocKey::new(kind, id)).collect())
}

pub fn all_keys(c: &Connection) -> AppResult<Vec<DocKey>> {
    let mut out = Vec::new();
    for k in KINDS {
        out.extend(keys_of_kind(c, k.kind)?);
    }
    Ok(out)
}

// ------------------------------------------------------ change → documents

/// What a committed change invalidates.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Invalidation {
    pub keys: BTreeSet<DocKey>,
    /// Whole kinds to re-synchronise (e.g. after a draft switch).
    pub resync: BTreeSet<&'static str>,
    /// Canonical rows changed (for dependency lookups in the index).
    pub rows: BTreeSet<(String, String)>,
}

impl Invalidation {
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.resync.is_empty() && self.rows.is_empty()
    }
    pub fn merge(&mut self, other: Invalidation) {
        self.keys.extend(other.keys);
        self.resync.extend(other.resync);
        self.rows.extend(other.rows);
    }
}

fn field<'a>(old: Option<&'a Value>, new: Option<&'a Value>, col: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for img in [old, new].into_iter().flatten() {
        if let Some(v) = img.get(col).and_then(|v| v.as_str())
            && !out.contains(&v)
        {
            out.push(v);
        }
    }
    out
}

/// Map one changed canonical row (with its before/after images) to the
/// documents it affects. Pure: runs on the committing thread after commit.
pub fn invalidate(
    inv: &mut Invalidation,
    table: &str,
    id: &str,
    old: Option<&Value>,
    new: Option<&Value>,
) {
    if table.starts_with("sys_") || table.starts_with("search_") || table.starts_with("ai_") {
        return;
    }
    inv.rows.insert((table.to_string(), id.to_string()));
    if let Some(key) = DocKey::from_table(table, id) {
        inv.keys.insert(key);
    }
    let mut add = |kind: &'static str, col: &str| {
        for v in field(old, new, col) {
            inv.keys.insert(DocKey::new(kind, v));
        }
    };
    match table {
        "screenplay_element" => add("screenplay_scene", "scene_id"),
        "screenplay_scene" => add("scene_breakdown", "id"),
        "breakdown_element" => add("scene_breakdown", "scene_id"),
        "story_character_card_link" => {
            add("story_scene_card", "scene_card_id");
            add("story_character", "character_id");
        }
        "story_character_relationship" => {
            add("story_character", "from_character_id");
            add("story_character", "to_character_id");
        }
        "cast_member" => add("story_character", "character_id"),
        "catalog_alias" => add("catalog_item", "catalog_item_id"),
        "location_photo" => add("location", "location_id"),
        "schedule_strip" | "schedule_marker" => add("shooting_day", "day_id"),
        "storyboard_panel" => add("storyboard", "storyboard_id"),
        "moodboard_item" => add("moodboard", "moodboard_id"),
        "comment" => add("comment", "parent_id"),
        "vault_item_tag" | "vault_item_collection" => add("vault_item", "item_id"),
        _ => {}
    }
    match table {
        "screenplay"
        | "screenplay_draft"
        | "production_source"
        | "screenplay_character_link"
        | "shooting_schedule" => {
            inv.resync.extend(SCREENPLAY_KINDS.iter().copied());
            if table == "screenplay_character_link" {
                inv.resync.insert("screenplay_scene");
            }
        }
        // A new/renamed/deleted Story character or Production location changes
        // the derived scene edges (cue and heading matching).
        "story_character" | "location" => {
            let renamed_or_gone = match (old, new) {
                (Some(o), Some(n)) => {
                    o.get("name") != n.get("name") || o.get("deleted_at") != n.get("deleted_at")
                }
                _ => true,
            };
            if renamed_or_gone {
                inv.resync.insert("screenplay_scene");
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_unknown_kinds_are_refused() {
        let k = DocKey::new("screenplay_scene", "abc");
        assert_eq!(DocKey::parse(&k.doc_id()), Some(k));
        assert_eq!(DocKey::parse("sys_undo:1"), None);
        assert_eq!(DocKey::parse("template:1"), None);
        assert!(DocKey::from_table("breakdown_element", "x").is_none());
        assert!(DocKey::from_table("story_character", "x").is_some());
    }

    #[test]
    fn element_changes_invalidate_their_scene_via_row_images() {
        let mut inv = Invalidation::default();
        // Undo of an insert: only the old image exists.
        invalidate(
            &mut inv,
            "screenplay_element",
            "e1",
            Some(&json!({ "id": "e1", "scene_id": "s1" })),
            None,
        );
        assert!(inv.keys.contains(&DocKey::new("screenplay_scene", "s1")));
        invalidate(
            &mut inv,
            "breakdown_element",
            "b1",
            Some(&json!({ "scene_id": "s1" })),
            Some(&json!({ "scene_id": "s2" })),
        );
        assert!(inv.keys.contains(&DocKey::new("scene_breakdown", "s1")));
        assert!(inv.keys.contains(&DocKey::new("scene_breakdown", "s2")));
        invalidate(&mut inv, "screenplay", "sp", None, Some(&json!({})));
        assert!(inv.resync.contains("screenplay_scene"));
        let mut ignored = Invalidation::default();
        invalidate(&mut ignored, "sys_activity", "a", None, None);
        assert!(ignored.is_empty());
    }

    #[test]
    fn every_kind_has_a_listing() {
        for k in KINDS {
            assert!(list_sql(k.kind).is_some(), "{}", k.kind);
        }
        assert!(norm("Railway  Station!") == "RAILWAY STATION");
    }
}
