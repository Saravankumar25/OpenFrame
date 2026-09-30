//! Import Sessions for exchange, review and response packages (FSD §47.5–47.9,
//! §50, §121; Import/Export §12–§20; FSD-COL-002 / IEX-015..024, IEX-031/032).
//!
//! ```text
//! open package → validate (format.rs) → identify source project/draft/type
//!   → map objects → detect stale/conflicts/ambiguous/unmapped → proposed changes
//!   → PREVIEW (stored in sys_import_session; nothing changed yet)
//!   → user decision (All safe / Comments only / Selected / Copy as New / Review record / Cancel)
//!   → re-plan against the current project → apply through the normal module
//!     operations (`core.dispatch`, Exchange origin) — all-or-nothing: if any step
//!     fails, every step already applied is rolled back through history undo
//!   → report (Applied / Partially Applied / Pending Review / Failed)
//! ```
//!
//! Identity: objects keep their identity only when the package represents the
//! same project (review/changes to existing objects); everything else is a copy
//! with new identities. Ambiguous or unmatched notes are never discarded: they
//! go to the Review Queue. The approved selection is the change — the package
//! itself never writes to the project.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, AppResult, Capability, new_id, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;

use super::exchange::{
    self, BreakdownContent, CallReviewContent, DraftInfo, ExComment, ExScene, ScheduleContent,
    ScriptContent, ShotsContent, StoryContent,
};
use super::format::{self, PackageManifest, PackageType, PackageUser};
use super::log_package;
use crate::core::AppCore;
use crate::registry::{Registry, SearchDoc, TrashHandler};
use crate::store::{DeleteSpec, DeletedItemRow, MutationMeta, Tx, soft_delete};

// ================================================================= DTOs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageChangeState {
    /// Maps unambiguously onto the current project; applied by "Accept All Safe".
    Safe,
    /// Needs an explicit decision (e.g. additive schedule moves, auto catalog matching).
    Review,
    /// The same object changed in both places, or no longer exists here.
    Conflict,
    /// Several places match; goes to the Review Queue.
    Ambiguous,
    /// Its context no longer exists; goes to the Review Queue.
    Unmapped,
    /// Informational (already present, text difference shown for review only).
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageChangeKind {
    Comment,
    Create,
    Update,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageChoice {
    pub scene_id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSpec {
    pub kind: String,
    pub reason: String,
    pub source_label: Option<String>,
    pub body: String,
    pub quoted_text: Option<String>,
}

/// One proposed change in the comparison preview (FSD §121).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageChange {
    pub id: String,
    pub group: String,
    pub group_label: String,
    pub label: String,
    pub detail: Option<String>,
    pub kind: PackageChangeKind,
    pub state: PackageChangeState,
    /// Can be applied by this import (informational items and deleted-object conflicts can't).
    pub applicable: bool,
    pub selected_by_default: bool,
    /// Part of "Copy as New" (a new outline copy with new identities).
    pub copy_only: bool,
    pub candidates: Vec<PackageChoice>,
    #[ts(skip)]
    #[serde(default)]
    pub op: Option<String>,
    #[ts(skip)]
    #[serde(default)]
    pub args: Option<Value>,
    #[ts(skip)]
    #[serde(default)]
    pub author: Option<PackageUser>,
    #[ts(skip)]
    #[serde(default)]
    pub queue: Option<QueueSpec>,
    #[ts(skip)]
    #[serde(default)]
    pub source_comment_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageChangeGroup {
    pub key: String,
    pub label: String,
    pub count: u32,
    pub state: PackageChangeState,
    pub applicable: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UndoStep {
    pub actor_id: String,
    pub actor_name: String,
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageImportResult {
    /// Applied | PartiallyApplied | PendingReview | Failed | Undone
    pub status: String,
    pub message: String,
    pub applied: u32,
    pub queued: u32,
    pub pending: u32,
    pub record_id: Option<String>,
    pub safety_backup: Option<String>,
    pub can_undo: bool,
    #[ts(skip)]
    #[serde(default)]
    pub steps: Vec<UndoStep>,
    #[ts(skip)]
    #[serde(default)]
    pub imported_comment_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Plan {
    compatibility: String,
    stale: bool,
    stale_title: Option<String>,
    rejection: Option<String>,
    source_version: String,
    host_version: String,
    changes: Vec<PackageChange>,
    warnings: Vec<String>,
    can_copy_as_new: bool,
    can_create_record: bool,
}

/// The Import Session as the preview dialog sees it (mockups 161/162).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageImportSession {
    pub id: String,
    pub package_id: String,
    pub package_type: PackageType,
    pub type_label: String,
    pub file_name: String,
    pub source_project_id: String,
    pub source_project_title: String,
    pub same_project: bool,
    pub source_version: String,
    pub host_version: String,
    /// Valid | Stale | Rejected
    pub compatibility: String,
    pub stale: bool,
    pub stale_title: Option<String>,
    pub rejection: Option<String>,
    #[ts(type = "number")]
    pub exported_at: i64,
    pub exported_by: String,
    pub comments_included: bool,
    pub attachments_included: bool,
    pub groups: Vec<PackageChangeGroup>,
    pub changes: Vec<PackageChange>,
    pub warnings: Vec<String>,
    pub can_copy_as_new: bool,
    pub can_create_record: bool,
    /// A screenplay snapshot that can be viewed without importing.
    pub can_view: bool,
    /// Previewed | Applied | PartiallyApplied | PendingReview | Rejected | Failed | Cancelled | Undone
    pub status: String,
    pub result: Option<PackageImportResult>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageSessionSummary {
    pub id: String,
    pub file_name: String,
    pub package_type: PackageType,
    pub type_label: String,
    pub source_project_title: String,
    pub compatibility: String,
    pub status: String,
    pub message: Option<String>,
    pub can_undo: bool,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageLogEntry {
    pub id: String,
    pub direction: String,
    pub package_type: String,
    pub file_name: String,
    pub summary: String,
    pub actor_name: Option<String>,
    #[ts(type = "number")]
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageQueueItem {
    pub id: String,
    /// "ambiguous" | "unmapped"
    pub kind: String,
    pub author_name: String,
    pub body: String,
    pub quoted_text: Option<String>,
    pub source_label: Option<String>,
    pub reason: String,
    pub candidates: Vec<PackageChoice>,
    pub status: String,
    pub attached_label: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub rev: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecordSummary {
    pub id: String,
    pub title: String,
    pub package_type: PackageType,
    pub type_label: String,
    pub source_project_title: String,
    pub source_label: Option<String>,
    pub comment_count: u32,
    pub exported_by: Option<String>,
    #[ts(type = "number | null")]
    pub exported_at: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub enum PackageApplyMode {
    /// "Accept All Safe" / "Import All Safe".
    AllSafe,
    /// "Import comments only" (default for stale review packages).
    CommentsOnly,
    /// "Import Selected".
    Selected,
    /// "Copy as New": a new outline copy with new identities.
    CopyAsNew,
    /// "Create a separate review record".
    Record,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageApplyArgs {
    pub session_id: String,
    pub mode: PackageApplyMode,
    #[serde(default)]
    pub selected: Vec<String>,
    /// "Create Backup Before Import" (FSD §118).
    #[serde(default)]
    pub create_backup: bool,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageSessionArgs {
    pub session_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageQueueAttachArgs {
    pub item_id: String,
    pub scene_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageRecordArgs {
    pub record_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageListArgs {
    #[serde(default)]
    pub limit: Option<u32>,
}

// =============================================================== helpers

fn change(
    id: String,
    group: &str,
    group_label: &str,
    label: String,
    kind: PackageChangeKind,
    state: PackageChangeState,
) -> PackageChange {
    let applicable = kind != PackageChangeKind::Info;
    PackageChange {
        id,
        group: group.to_string(),
        group_label: group_label.to_string(),
        label,
        detail: None,
        kind,
        state,
        applicable,
        selected_by_default: applicable && state == PackageChangeState::Safe,
        copy_only: false,
        candidates: vec![],
        op: None,
        args: None,
        author: None,
        queue: None,
        source_comment_id: None,
    }
}

fn with_op(mut c: PackageChange, op: &str, args: Value) -> PackageChange {
    c.op = Some(op.to_string());
    c.args = Some(args);
    c
}

fn excerpt(s: &str, n: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > n {
        format!("{}…", one.chars().take(n).collect::<String>())
    } else {
        one
    }
}

fn utf16_offset(s: &str, byte: usize) -> u32 {
    s[..byte].encode_utf16().count() as u32
}

fn scene_label(s: &ExScene) -> String {
    let h = if s.heading.trim().is_empty() {
        "(untitled scene)"
    } else {
        s.heading.trim()
    };
    format!("Scene {} · {h}", s.number)
}

const TRACKED_TARGETS: &[&str] = &[
    "screenplay_draft",
    "screenplay_scene",
    "screenplay_element",
    "story_act",
    "story_sequence",
    "story_beat",
    "story_scene_card",
    "breakdown_element",
    "catalog_item",
    "shot",
    "shooting_day",
    "call_sheet",
];

/// Canonical comment target table for a package target type.
fn normalize_target(t: &str) -> Option<&'static str> {
    let t = match t {
        "scene_card" => "story_scene_card",
        "beat" => "story_beat",
        "sequence" => "story_sequence",
        "act" => "story_act",
        other => other,
    };
    TRACKED_TARGETS.iter().copied().find(|x| *x == t)
}

/// (rev, deleted) of a row in one of the known tables.
fn host_row(c: &Connection, table: &str, id: &str) -> AppResult<Option<(i64, bool)>> {
    let Some(t) = normalize_target(table) else {
        return Ok(None);
    };
    let has_deleted = !matches!(t, "screenplay_element");
    let sql = if has_deleted {
        format!("SELECT rev, deleted_at IS NOT NULL FROM \"{t}\" WHERE id=?1")
    } else {
        format!("SELECT rev, 0 FROM \"{t}\" WHERE id=?1")
    };
    Ok(c.query_row(&sql, [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?)
}

fn comment_exists(c: &Connection, id: &str) -> AppResult<bool> {
    Ok(c.query_row(
        "SELECT EXISTS(SELECT 1 FROM comment WHERE id=?1)",
        [id],
        |r| r.get(0),
    )?)
}

/// Stale check for generic packages: any included row changed here since export.
fn changed_since_export(c: &Connection, m: &PackageManifest) -> AppResult<usize> {
    let mut n = 0;
    for (k, rev) in &m.base_snapshot {
        let Some((table, id)) = k.split_once(':') else {
            continue;
        };
        if let Some((host_rev, _)) = host_row(c, table, id)?
            && host_rev != *rev
        {
            n += 1;
        }
    }
    Ok(n)
}

fn base_rev(m: &PackageManifest, table: &str, id: &str) -> Option<i64> {
    m.base_snapshot.get(&format!("{table}:{id}")).copied()
}

fn is_uuid(s: &str) -> bool {
    openframe_domain::ids::is_valid_id(s)
}

fn author_of(cm: &ExComment, m: &PackageManifest) -> PackageUser {
    if is_uuid(&cm.author_user_id) && !cm.author_name.trim().is_empty() {
        PackageUser {
            user_id: cm.author_user_id.clone(),
            display_name: excerpt(&cm.author_name, 80),
        }
    } else {
        m.originating_user.clone()
    }
}

fn queue_change(
    id: String,
    cm: &ExComment,
    m: &PackageManifest,
    kind: &str,
    reason: String,
    source_label: Option<String>,
    candidates: Vec<PackageChoice>,
) -> PackageChange {
    let state = if kind == "ambiguous" {
        PackageChangeState::Ambiguous
    } else {
        PackageChangeState::Unmapped
    };
    let (group, label) = if kind == "ambiguous" {
        ("comments.ambiguous", "Comments — ambiguous mapping")
    } else {
        ("comments.unmapped", "Comments — unmatched (Review Queue)")
    };
    let mut ch = change(
        id,
        group,
        label,
        format!(
            "“{}” — {}",
            excerpt(&cm.body, 60),
            author_of(cm, m).display_name
        ),
        PackageChangeKind::Comment,
        state,
    );
    ch.detail = Some(reason.clone());
    ch.candidates = candidates;
    ch.selected_by_default = true;
    ch.author = Some(author_of(cm, m));
    ch.source_comment_id = Some(cm.id.clone());
    ch.queue = Some(QueueSpec {
        kind: kind.to_string(),
        reason,
        source_label,
        body: cm.body.clone(),
        quoted_text: cm.quoted_text.clone(),
    });
    ch
}

fn present_change(cm: &ExComment) -> PackageChange {
    let mut ch = change(
        format!("comment:{}", cm.id),
        "comments.present",
        "Comments already in your project",
        format!("“{}”", excerpt(&cm.body, 60)),
        PackageChangeKind::Info,
        PackageChangeState::None,
    );
    ch.source_comment_id = Some(cm.id.clone());
    ch
}

fn comment_change(
    id: String,
    cm: &ExComment,
    m: &PackageManifest,
    op: &str,
    args: Value,
    where_label: String,
) -> PackageChange {
    let author = author_of(cm, m);
    let mut ch = change(
        id,
        "comments",
        "Comments",
        format!("“{}” — {}", excerpt(&cm.body, 60), author.display_name),
        PackageChangeKind::Comment,
        PackageChangeState::Safe,
    );
    ch.detail = Some(where_label);
    ch.author = Some(author);
    ch.source_comment_id = Some(cm.id.clone());
    with_op(ch, op, args)
}

fn body_with_quote(cm: &ExComment) -> String {
    match &cm.quoted_text {
        Some(q) if !q.trim().is_empty() => format!("“{}” — {}", q.trim(), cm.body),
        _ => cm.body.clone(),
    }
}

/// Split comments into roots then replies (replies need their parent first).
fn ordered(comments: &[ExComment]) -> Vec<&ExComment> {
    let mut v: Vec<&ExComment> = comments.iter().filter(|c| c.parent_id.is_none()).collect();
    v.extend(comments.iter().filter(|c| c.parent_id.is_some()));
    v
}

/// Replies: attach under the existing or newly imported parent; otherwise queue.
fn plan_reply(
    c: &Connection,
    cm: &ExComment,
    m: &PackageManifest,
    planned: &HashMap<String, PackageChangeState>,
    out: &mut Vec<PackageChange>,
) -> AppResult<()> {
    let parent = cm.parent_id.clone().unwrap_or_default();
    let id = format!("comment:{}", cm.id);
    if comment_exists(c, &parent)? {
        out.push(comment_change(
            id,
            cm,
            m,
            "comment.reply",
            json!({ "parentId": parent, "body": cm.body }),
            "Reply in an existing thread".into(),
        ));
    } else if planned.get(&parent) == Some(&PackageChangeState::Safe) {
        out.push(comment_change(
            id,
            cm,
            m,
            "comment.reply",
            json!({ "parentId": format!("$ref:comment:{parent}"), "body": cm.body }),
            "Reply to an imported comment".into(),
        ));
    } else {
        out.push(queue_change(
            id,
            cm,
            m,
            "unmapped",
            "Reply to a note that needs a decision.".into(),
            None,
            vec![],
        ));
    }
    Ok(())
}

// =============================================================== planning

struct Ctx<'a> {
    c: &'a Connection,
    m: &'a PackageManifest,
    same_project: bool,
    prior: &'a HashSet<String>,
}

impl Ctx<'_> {
    fn already(&self, cm: &ExComment) -> AppResult<bool> {
        Ok(self.prior.contains(&cm.id) || comment_exists(self.c, &cm.id)?)
    }
}

fn plan_script(x: &Ctx<'_>, content: &Value) -> AppResult<Plan> {
    let content: ScriptContent = exchange::parse_content(content)?;
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: content.draft.name.clone(),
        can_create_record: true,
        ..Default::default()
    };
    if !x.same_project {
        plan.compatibility = "Rejected".into();
        plan.rejection = Some(format!(
            "This review belongs to another project (“{}”). You can view it, but its notes can't be attached to this project.",
            x.m.source_project_title
        ));
        plan.can_create_record = false;
        plan.host_version = "—".into();
        return Ok(plan);
    }
    let pkg_draft = exchange::load_draft_info(x.c, &content.draft.id)?;
    let host: DraftInfo = match exchange::current_draft(x.c, Some(&content.draft.screenplay_id))? {
        Some(d) => d,
        None => match exchange::current_draft(x.c, None)? {
            Some(d) => d,
            None => {
                plan.compatibility = "Rejected".into();
                plan.rejection =
                    Some("This project has no screenplay to attach the review to.".into());
                plan.host_version = "—".into();
                return Ok(plan);
            }
        },
    };
    plan.host_version = host.name.clone();
    if host.id != content.draft.id {
        plan.stale = true;
        plan.stale_title = Some(format!(
            "This review was created from {}. Your current project is {}.",
            content.draft.name, host.name
        ));
    } else if pkg_draft.is_none() || changed_since_export(x.c, x.m)? > 0 {
        plan.stale = true;
        plan.stale_title = Some(format!(
            "This review was created from an earlier state of {}. The draft changed after the package was exported.",
            content.draft.name
        ));
    }
    if plan.stale {
        plan.compatibility = "Stale".into();
    }
    let host_scenes = exchange::load_scenes(x.c, &host.id)?;
    // Text differences are surfaced for explicit review, never applied (FSD §50).
    for ps in &content.scenes {
        let matches: Vec<&ExScene> = host_scenes
            .iter()
            .filter(|h| h.lineage_id == ps.lineage_id)
            .collect();
        let label = format!("Scene {} · {}", ps.number, excerpt(&ps.heading, 48));
        match matches.as_slice() {
            [] => {
                let mut ch = change(
                    format!("text:{}", ps.id),
                    "text",
                    "Screenplay text changes (review only)",
                    label,
                    PackageChangeKind::Info,
                    PackageChangeState::Review,
                );
                ch.detail = Some(format!("This scene no longer exists in {}.", host.name));
                plan.changes.push(ch);
            }
            [h] if h.full_text() != ps.full_text() => {
                let mut ch = change(
                    format!("text:{}", ps.id),
                    "text",
                    "Screenplay text changes (review only)",
                    label,
                    PackageChangeKind::Info,
                    PackageChangeState::Review,
                );
                ch.detail = Some(format!(
                    "The text differs from {} (now Scene {}). Import never changes screenplay text — compare and edit it in the Screenplay.",
                    host.name, h.number
                ));
                plan.changes.push(ch);
            }
            _ => {}
        }
    }
    let pkg_scene: HashMap<&str, &ExScene> =
        content.scenes.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut planned: HashMap<String, PackageChangeState> = HashMap::new();
    for cm in ordered(&content.comments) {
        if x.already(cm)? {
            plan.changes.push(present_change(cm));
            continue;
        }
        if cm.parent_id.is_some() {
            plan_reply(x.c, cm, x.m, &planned, &mut plan.changes)?;
            continue;
        }
        let id = format!("comment:{}", cm.id);
        if cm.target_type == "screenplay_draft" {
            plan.changes.push(comment_change(
                id,
                cm,
                x.m,
                "comment.create",
                json!({ "targetType": "screenplay_draft", "targetId": host.id, "body": body_with_quote(cm) }),
                format!("On {}", host.name),
            ));
            planned.insert(cm.id.clone(), PackageChangeState::Safe);
            continue;
        }
        let lineage = cm.scene_lineage_id.clone().or_else(|| {
            cm.scene_id
                .as_deref()
                .and_then(|s| pkg_scene.get(s))
                .map(|s| s.lineage_id.clone())
        });
        let source_label = cm
            .scene_id
            .as_deref()
            .and_then(|s| pkg_scene.get(s))
            .map(|s| format!("Scene {} in {}", s.number, content.draft.name));
        let mut candidates: Vec<&ExScene> = match (&cm.scene_id, &lineage) {
            (Some(sid), _) if host_scenes.iter().any(|h| &h.id == sid) => {
                host_scenes.iter().filter(|h| &h.id == sid).collect()
            }
            (_, Some(l)) => host_scenes.iter().filter(|h| &h.lineage_id == l).collect(),
            _ => vec![],
        };
        let quote = cm.quoted_text.clone().filter(|q| !q.trim().is_empty());
        let mut matched_by_text = false;
        if candidates.is_empty()
            && let Some(q) = &quote
        {
            let ql = q.to_lowercase();
            candidates = host_scenes
                .iter()
                .filter(|h| h.full_text().to_lowercase().contains(&ql))
                .collect();
            matched_by_text = true;
        }
        match candidates.as_slice() {
            [] => {
                let reason = if quote.is_some() {
                    format!("The original text no longer exists in {}.", host.name)
                } else {
                    format!("The scene no longer exists in {}.", host.name)
                };
                plan.changes.push(queue_change(
                    id,
                    cm,
                    x.m,
                    "unmapped",
                    reason,
                    source_label,
                    vec![],
                ));
                planned.insert(cm.id.clone(), PackageChangeState::Unmapped);
            }
            [scene] => {
                let (args, where_label) = script_comment_args(cm, scene, quote.as_deref());
                let mut ch = comment_change(
                    id,
                    cm,
                    x.m,
                    "comment.create",
                    args,
                    if matched_by_text {
                        format!("{where_label} (matched by its text)")
                    } else {
                        where_label
                    },
                );
                if plan.stale && scene.id != cm.scene_id.clone().unwrap_or_default() {
                    ch.detail = Some(format!(
                        "{} — attached to the matching scene in {}",
                        ch.detail.clone().unwrap_or_default(),
                        host.name
                    ));
                }
                plan.changes.push(ch);
                planned.insert(cm.id.clone(), PackageChangeState::Safe);
            }
            many => {
                let choices: Vec<PackageChoice> = many
                    .iter()
                    .map(|s| PackageChoice {
                        scene_id: s.id.clone(),
                        label: scene_label(s),
                    })
                    .collect();
                let reason = format!(
                    "Matches {} places: {}.",
                    choices.len(),
                    choices
                        .iter()
                        .map(|c| c.label.clone())
                        .collect::<Vec<_>>()
                        .join(" and ")
                );
                plan.changes.push(queue_change(
                    id,
                    cm,
                    x.m,
                    "ambiguous",
                    reason,
                    source_label,
                    choices,
                ));
                planned.insert(cm.id.clone(), PackageChangeState::Ambiguous);
            }
        }
    }
    Ok(plan)
}

/// Comment args for a mapped screenplay scene: anchored to the quoted text when
/// it occurs exactly once in one element, otherwise on the scene with the quote kept.
fn script_comment_args(cm: &ExComment, scene: &ExScene, quote: Option<&str>) -> (Value, String) {
    let label = scene_label(scene);
    if let Some(q) = quote {
        let hits: Vec<(&str, usize)> = scene
            .elements
            .iter()
            .flat_map(|e| {
                e.text
                    .match_indices(q)
                    .map(move |(b, _)| (e.id.as_str(), b))
            })
            .collect();
        if let [(el, byte)] = hits.as_slice() {
            let text = &scene
                .elements
                .iter()
                .find(|e| e.id == *el)
                .map(|e| e.text.clone())
                .unwrap_or_default();
            let start = utf16_offset(text, *byte);
            let end = start + q.encode_utf16().count() as u32;
            return (
                json!({ "targetType": "screenplay_element", "targetId": el, "body": cm.body, "anchor": { "start": start, "end": end } }),
                format!("{label} — on “{}”", excerpt(q, 40)),
            );
        }
    }
    (
        json!({ "targetType": "screenplay_scene", "targetId": scene.id, "body": body_with_quote(cm) }),
        label,
    )
}

/// Generic comment mapping for non-screenplay packages.
fn plan_generic_comments(
    x: &Ctx<'_>,
    comments: &[ExComment],
    created: &HashMap<String, String>,
    plan: &mut Plan,
) -> AppResult<()> {
    let mut planned: HashMap<String, PackageChangeState> = HashMap::new();
    for cm in ordered(comments) {
        if x.already(cm)? {
            plan.changes.push(present_change(cm));
            continue;
        }
        if cm.parent_id.is_some() {
            plan_reply(x.c, cm, x.m, &planned, &mut plan.changes)?;
            continue;
        }
        let id = format!("comment:{}", cm.id);
        let Some(target) = normalize_target(&cm.target_type) else {
            plan.changes.push(queue_change(
                id,
                cm,
                x.m,
                "unmapped",
                "OpenFrame can't attach a note to this kind of item.".into(),
                None,
                vec![],
            ));
            planned.insert(cm.id.clone(), PackageChangeState::Unmapped);
            continue;
        };
        let live =
            x.same_project && matches!(host_row(x.c, target, &cm.target_id)?, Some((_, false)));
        if live {
            plan.changes.push(comment_change(
                id,
                cm,
                x.m,
                "comment.create",
                json!({ "targetType": target, "targetId": cm.target_id, "body": body_with_quote(cm) }),
                "On the same item in your project".into(),
            ));
            planned.insert(cm.id.clone(), PackageChangeState::Safe);
        } else if let Some(change_id) = created.get(&cm.target_id) {
            let mut ch = comment_change(
                id,
                cm,
                x.m,
                "comment.create",
                json!({ "targetType": target, "targetId": format!("$ref:{change_id}"), "body": body_with_quote(cm) }),
                "On an item added by this import".into(),
            );
            ch.copy_only = change_id.starts_with("copy:");
            if ch.copy_only {
                ch.selected_by_default = false;
            }
            plan.changes.push(ch);
            planned.insert(cm.id.clone(), PackageChangeState::Safe);
        } else {
            plan.changes.push(queue_change(
                id,
                cm,
                x.m,
                "unmapped",
                "The item this note was on isn't in your project.".into(),
                None,
                vec![],
            ));
            planned.insert(cm.id.clone(), PackageChangeState::Unmapped);
        }
    }
    Ok(())
}

fn diff_text(
    label: &str,
    host: &Option<String>,
    pkg: &Option<String>,
    args: &mut serde_json::Map<String, Value>,
    key: &str,
    changed: &mut Vec<String>,
) {
    let h = host.clone().unwrap_or_default();
    let p = pkg.clone().unwrap_or_default();
    if h != p {
        args.insert(key.to_string(), Value::String(p));
        changed.push(label.to_string());
    }
}

fn update_state(x: &Ctx<'_>, table: &str, id: &str, host_rev: i64) -> PackageChangeState {
    match base_rev(x.m, table, id) {
        Some(b) if b == host_rev => PackageChangeState::Safe,
        _ => PackageChangeState::Conflict,
    }
}

fn gone_change(id: String, group: &str, group_label: &str, label: String) -> PackageChange {
    let mut ch = change(
        id,
        group,
        group_label,
        label,
        PackageChangeKind::Info,
        PackageChangeState::Conflict,
    );
    ch.detail = Some(
        "Incoming package refers to an object that no longer exists in the host project.".into(),
    );
    ch
}

fn plan_story(x: &Ctx<'_>, content: &Value, session_files: bool) -> AppResult<Plan> {
    let s: StoryContent = exchange::parse_content(content)?;
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: format!("Story Board exported {}", x.m.source_project_title),
        host_version: "Your Story Board".into(),
        can_copy_as_new: true,
        can_create_record: true,
        ..Default::default()
    };
    let mut copied: HashMap<String, String> = HashMap::new();
    // ---- Copy as New: the whole outline with new identities.
    {
        let mut kids: HashMap<(String, String), Vec<(i64, u8, String)>> = HashMap::new();
        for q in &s.sequences {
            if let Some(a) = &q.act_id {
                kids.entry(("act".into(), a.clone())).or_default().push((
                    q.position,
                    0,
                    q.id.clone(),
                ));
            }
        }
        for b in &s.beats {
            let key = (
                b.parent_type.clone(),
                b.parent_id.clone().unwrap_or_default(),
            );
            kids.entry(key)
                .or_default()
                .push((b.position, 1, b.id.clone()));
        }
        for cd in &s.cards {
            let key = (
                cd.parent_type.clone(),
                cd.parent_id.clone().unwrap_or_default(),
            );
            kids.entry(key)
                .or_default()
                .push((cd.position, 2, cd.id.clone()));
        }
        for v in kids.values_mut() {
            v.sort();
        }
        let seqs: HashMap<&str, &exchange::ExSequence> =
            s.sequences.iter().map(|q| (q.id.as_str(), q)).collect();
        let beats: HashMap<&str, &exchange::ExBeat> =
            s.beats.iter().map(|q| (q.id.as_str(), q)).collect();
        let cards: HashMap<&str, &exchange::ExCard> =
            s.cards.iter().map(|q| (q.id.as_str(), q)).collect();
        let mut copies: Vec<PackageChange> = vec![];
        let copy = |id: String, label: String, op: &str, args: Value| {
            let mut ch = with_op(
                change(
                    id,
                    "copy",
                    "Copy as new outline",
                    label,
                    PackageChangeKind::Create,
                    PackageChangeState::Safe,
                ),
                op,
                args,
            );
            ch.copy_only = true;
            ch.selected_by_default = false;
            ch
        };
        fn children(
            parent: Value,
            key: (String, String),
            kids: &HashMap<(String, String), Vec<(i64, u8, String)>>,
            seqs: &HashMap<&str, &exchange::ExSequence>,
            beats: &HashMap<&str, &exchange::ExBeat>,
            cards: &HashMap<&str, &exchange::ExCard>,
            copies: &mut Vec<PackageChange>,
            copied: &mut HashMap<String, String>,
            copy: &dyn Fn(String, String, &str, Value) -> PackageChange,
        ) {
            for (_, kind, id) in kids.get(&key).cloned().unwrap_or_default() {
                match kind {
                    0 => {
                        let q = seqs[id.as_str()];
                        let cid = format!("copy:seq:{id}");
                        copies.push(copy(
                            cid.clone(),
                            format!("Sequence “{}”", excerpt(&q.title, 40)),
                            "story.create_sequence",
                            json!({ "actId": parent["parentId"], "title": q.title }),
                        ));
                        if let Some(n) = &q.note {
                            copies.push(copy(
                                format!("copy:seqnote:{id}"),
                                format!("Sequence note “{}”", excerpt(&q.title, 30)),
                                "story.update_sequence",
                                json!({ "id": format!("$ref:{cid}"), "note": n }),
                            ));
                        }
                        copied.insert(id.clone(), cid.clone());
                        children(
                            json!({ "parentType": "sequence", "parentId": format!("$ref:{cid}") }),
                            ("sequence".into(), id.clone()),
                            kids,
                            seqs,
                            beats,
                            cards,
                            copies,
                            copied,
                            copy,
                        );
                    }
                    1 => {
                        let b = beats[id.as_str()];
                        let cid = format!("copy:beat:{id}");
                        let mut args = json!({ "parent": parent, "text": b.text });
                        if let Some(col) = &b.color {
                            args["color"] = json!(col);
                        }
                        copies.push(copy(
                            cid.clone(),
                            format!("Beat “{}”", excerpt(&b.text, 40)),
                            "story.create_beat",
                            args,
                        ));
                        if let Some(n) = &b.note {
                            copies.push(copy(
                                format!("copy:beatnote:{id}"),
                                "Beat note".into(),
                                "story.update_beat",
                                json!({ "id": format!("$ref:{cid}"), "note": n }),
                            ));
                        }
                        copied.insert(id.clone(), cid);
                    }
                    _ => {
                        let cd = cards[id.as_str()];
                        let cid = format!("copy:card:{id}");
                        let mut args =
                            json!({ "parent": parent, "shortDescription": cd.short_description });
                        if let Some(h) = &cd.scene_heading {
                            args["sceneHeading"] = json!(h);
                        }
                        copies.push(copy(
                            cid.clone(),
                            format!("Scene Card “{}”", excerpt(&cd.short_description, 40)),
                            "story.create_card",
                            args,
                        ));
                        if cd.notes.is_some() || cd.color.is_some() {
                            let mut up = json!({ "id": format!("$ref:{cid}") });
                            if let Some(n) = &cd.notes {
                                up["notes"] = json!(n);
                            }
                            if let Some(col) = &cd.color {
                                up["color"] = json!(col);
                            }
                            copies.push(copy(
                                format!("copy:cardextra:{id}"),
                                "Scene Card notes".into(),
                                "story.update_card",
                                up,
                            ));
                        }
                        copied.insert(id.clone(), cid);
                    }
                }
            }
        }
        let mut acts = s.acts.clone();
        acts.sort_by(|a, b| a.position.cmp(&b.position).then(a.id.cmp(&b.id)));
        for a in &acts {
            let cid = format!("copy:act:{}", a.id);
            let mut args = json!({ "title": a.title });
            if let Some(n) = &a.note {
                args["note"] = json!(n);
            }
            copies.push(copy(
                cid.clone(),
                format!("Act “{}”", excerpt(&a.title, 40)),
                "story.create_act",
                args,
            ));
            copied.insert(a.id.clone(), cid.clone());
            children(
                json!({ "parentType": "act", "parentId": format!("$ref:{cid}") }),
                ("act".into(), a.id.clone()),
                &kids,
                &seqs,
                &beats,
                &cards,
                &mut copies,
                &mut copied,
                &copy,
            );
        }
        for pt in ["parking", "unassigned"] {
            children(
                json!({ "parentType": "parking" }),
                (pt.into(), String::new()),
                &kids,
                &seqs,
                &beats,
                &cards,
                &mut copies,
                &mut copied,
                &copy,
            );
        }
        let skipped = s.sequences.iter().filter(|q| q.act_id.is_none()).count();
        if skipped > 0 {
            plan.warnings.push(format!(
                "{skipped} sequence(s) without an Act are not included in a copy."
            ));
        }
        if session_files {
            for at in &s.attachments {
                let owner_change = copied.get(&at.owner_id).cloned();
                if let Some(oc) = owner_change {
                    let ch = copy(
                        format!("copy:att:{}", at.id),
                        format!("Attachment “{}”", excerpt(&at.file_name, 40)),
                        "story.add_attachment",
                        json!({ "ownerType": at.owner_type, "ownerId": format!("$ref:{oc}"), "path": format!("$file:{}", at.entry) }),
                    );
                    copies.push(ch);
                }
            }
        }
        plan.changes.extend(copies);
    }
    // Same-project additions get their own map: they never hang off a copy.
    let mut created: HashMap<String, String> = HashMap::new();
    if x.same_project {
        // ---- Changes to the same objects (identity preserved).
        let mut ups: Vec<PackageChange> = vec![];
        for a in &s.acts {
            let host: Option<(String, Option<String>, i64, bool)> =
                x.c.query_row(
                    "SELECT title, note, rev, deleted_at IS NOT NULL FROM story_act WHERE id=?1",
                    [&a.id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()?;
            let label = format!("Act “{}”", excerpt(&a.title, 40));
            match host {
                Some((_, _, _, true)) => ups.push(gone_change(
                    format!("act:{}", a.id),
                    "story.acts",
                    "Acts",
                    label,
                )),
                Some((title, note, rev, false)) => {
                    let mut args = serde_json::Map::new();
                    let mut changed = vec![];
                    diff_text(
                        "title",
                        &Some(title),
                        &Some(a.title.clone()),
                        &mut args,
                        "title",
                        &mut changed,
                    );
                    diff_text("note", &note, &a.note, &mut args, "note", &mut changed);
                    if !changed.is_empty() {
                        args.insert("id".into(), json!(a.id));
                        args.insert("expectedRev".into(), json!(rev));
                        let mut ch = with_op(
                            change(
                                format!("act:{}", a.id),
                                "story.acts",
                                "Acts",
                                label,
                                PackageChangeKind::Update,
                                update_state(x, "story_act", &a.id, rev),
                            ),
                            "story.update_act",
                            Value::Object(args),
                        );
                        ch.detail = Some(format!("Changed: {}", changed.join(", ")));
                        ups.push(ch);
                    }
                }
                None => {
                    let mut args = json!({ "title": a.title });
                    if let Some(n) = &a.note {
                        args["note"] = json!(n);
                    }
                    ups.push(with_op(
                        change(
                            format!("new:act:{}", a.id),
                            "story.new",
                            "New Story Board items",
                            label,
                            PackageChangeKind::Create,
                            PackageChangeState::Safe,
                        ),
                        "story.create_act",
                        args,
                    ));
                    created.insert(a.id.clone(), format!("new:act:{}", a.id));
                }
            }
        }
        for q in &s.sequences {
            let host: Option<(String, Option<String>, i64, bool)> = x
                .c
                .query_row("SELECT title, note, rev, deleted_at IS NOT NULL FROM story_sequence WHERE id=?1", [&q.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
                .optional()?;
            let label = format!("Sequence “{}”", excerpt(&q.title, 40));
            match host {
                Some((_, _, _, true)) => ups.push(gone_change(
                    format!("seq:{}", q.id),
                    "story.sequences",
                    "Sequences",
                    label,
                )),
                Some((title, note, rev, false)) => {
                    let mut args = serde_json::Map::new();
                    let mut changed = vec![];
                    diff_text(
                        "title",
                        &Some(title),
                        &Some(q.title.clone()),
                        &mut args,
                        "title",
                        &mut changed,
                    );
                    diff_text("note", &note, &q.note, &mut args, "note", &mut changed);
                    if !changed.is_empty() {
                        args.insert("id".into(), json!(q.id));
                        args.insert("expectedRev".into(), json!(rev));
                        let mut ch = with_op(
                            change(
                                format!("seq:{}", q.id),
                                "story.sequences",
                                "Sequences",
                                label,
                                PackageChangeKind::Update,
                                update_state(x, "story_sequence", &q.id, rev),
                            ),
                            "story.update_sequence",
                            Value::Object(args),
                        );
                        ch.detail = Some(format!("Changed: {}", changed.join(", ")));
                        ups.push(ch);
                    }
                }
                None => {
                    let parent = q.act_id.as_ref().and_then(|a| {
                        if matches!(
                            host_row(x.c, "story_act", a).ok().flatten(),
                            Some((_, false))
                        ) {
                            Some(json!(a))
                        } else {
                            created.get(a).map(|c| json!(format!("$ref:{c}")))
                        }
                    });
                    if let Some(act) = parent {
                        ups.push(with_op(
                            change(
                                format!("new:seq:{}", q.id),
                                "story.new",
                                "New Story Board items",
                                label,
                                PackageChangeKind::Create,
                                PackageChangeState::Safe,
                            ),
                            "story.create_sequence",
                            json!({ "actId": act, "title": q.title }),
                        ));
                        created.insert(q.id.clone(), format!("new:seq:{}", q.id));
                    } else {
                        let mut ch = change(
                            format!("new:seq:{}", q.id),
                            "story.new",
                            "New Story Board items",
                            label,
                            PackageChangeKind::Info,
                            PackageChangeState::Unmapped,
                        );
                        ch.detail = Some("Its Act isn't in your project. Use Copy as New to bring the whole outline in.".into());
                        ups.push(ch);
                    }
                }
            }
        }
        let parent_ref = |pt: &str,
                          pid: &Option<String>,
                          created: &HashMap<String, String>|
         -> (Value, PackageChangeState) {
            match (pt, pid) {
                ("act" | "sequence", Some(id)) => {
                    let table = if pt == "act" {
                        "story_act"
                    } else {
                        "story_sequence"
                    };
                    if matches!(host_row(x.c, table, id).ok().flatten(), Some((_, false))) {
                        (
                            json!({ "parentType": pt, "parentId": id }),
                            PackageChangeState::Safe,
                        )
                    } else if let Some(c) = created.get(id) {
                        (
                            json!({ "parentType": pt, "parentId": format!("$ref:{c}") }),
                            PackageChangeState::Safe,
                        )
                    } else {
                        (
                            json!({ "parentType": "parking" }),
                            PackageChangeState::Review,
                        )
                    }
                }
                _ => (json!({ "parentType": "parking" }), PackageChangeState::Safe),
            }
        };
        for b in &s.beats {
            let host: Option<(String, Option<String>, Option<String>, i64, bool)> = x
                .c
                .query_row("SELECT text, note, color, rev, deleted_at IS NOT NULL FROM story_beat WHERE id=?1", [&b.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
                .optional()?;
            let label = format!("Beat “{}”", excerpt(&b.text, 40));
            match host {
                Some((_, _, _, _, true)) => ups.push(gone_change(
                    format!("beat:{}", b.id),
                    "story.beats",
                    "Beats",
                    label,
                )),
                Some((text, note, color, rev, false)) => {
                    let mut args = serde_json::Map::new();
                    let mut changed = vec![];
                    diff_text(
                        "text",
                        &Some(text),
                        &Some(b.text.clone()),
                        &mut args,
                        "text",
                        &mut changed,
                    );
                    diff_text("note", &note, &b.note, &mut args, "note", &mut changed);
                    diff_text("colour", &color, &b.color, &mut args, "color", &mut changed);
                    if !changed.is_empty() {
                        args.insert("id".into(), json!(b.id));
                        args.insert("expectedRev".into(), json!(rev));
                        let mut ch = with_op(
                            change(
                                format!("beat:{}", b.id),
                                "story.beats",
                                "Beats",
                                label,
                                PackageChangeKind::Update,
                                update_state(x, "story_beat", &b.id, rev),
                            ),
                            "story.update_beat",
                            Value::Object(args),
                        );
                        ch.detail = Some(format!("Changed: {}", changed.join(", ")));
                        ups.push(ch);
                    }
                }
                None => {
                    let (parent, state) = parent_ref(&b.parent_type, &b.parent_id, &created);
                    let mut ch = with_op(
                        change(
                            format!("new:beat:{}", b.id),
                            "story.new",
                            "New Story Board items",
                            label,
                            PackageChangeKind::Create,
                            state,
                        ),
                        "story.create_beat",
                        json!({ "parent": parent, "text": b.text }),
                    );
                    if state == PackageChangeState::Review {
                        ch.detail = Some(
                            "Its place isn't in your project; it will go to the Parking Lot."
                                .into(),
                        );
                    }
                    ups.push(ch);
                    created.insert(b.id.clone(), format!("new:beat:{}", b.id));
                }
            }
        }
        for cd in &s.cards {
            let host: Option<(String, Option<String>, Option<String>, Option<String>, i64, bool)> = x
                .c
                .query_row(
                    "SELECT short_description, scene_heading, notes, color, rev, deleted_at IS NOT NULL FROM story_scene_card WHERE id=?1",
                    [&cd.id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
                )
                .optional()?;
            let label = format!("Scene Card “{}”", excerpt(&cd.short_description, 40));
            match host {
                Some((.., true)) => ups.push(gone_change(
                    format!("card:{}", cd.id),
                    "story.cards",
                    "Scene Cards",
                    label,
                )),
                Some((desc, heading, notes, color, rev, false)) => {
                    let mut args = serde_json::Map::new();
                    let mut changed = vec![];
                    diff_text(
                        "description",
                        &Some(desc),
                        &Some(cd.short_description.clone()),
                        &mut args,
                        "shortDescription",
                        &mut changed,
                    );
                    diff_text(
                        "scene heading",
                        &heading,
                        &cd.scene_heading,
                        &mut args,
                        "sceneHeading",
                        &mut changed,
                    );
                    diff_text("notes", &notes, &cd.notes, &mut args, "notes", &mut changed);
                    diff_text(
                        "colour",
                        &color,
                        &cd.color,
                        &mut args,
                        "color",
                        &mut changed,
                    );
                    if !changed.is_empty() {
                        args.insert("id".into(), json!(cd.id));
                        args.insert("expectedRev".into(), json!(rev));
                        let mut ch = with_op(
                            change(
                                format!("card:{}", cd.id),
                                "story.cards",
                                "Changed Scene Cards",
                                label,
                                PackageChangeKind::Update,
                                update_state(x, "story_scene_card", &cd.id, rev),
                            ),
                            "story.update_card",
                            Value::Object(args),
                        );
                        ch.detail = Some(format!("Changed: {}", changed.join(", ")));
                        ups.push(ch);
                    }
                }
                None => {
                    let (parent, state) = parent_ref(&cd.parent_type, &cd.parent_id, &created);
                    let mut args =
                        json!({ "parent": parent, "shortDescription": cd.short_description });
                    if let Some(h) = &cd.scene_heading {
                        args["sceneHeading"] = json!(h);
                    }
                    let mut ch = with_op(
                        change(
                            format!("new:card:{}", cd.id),
                            "story.new",
                            "New Story Board items",
                            label,
                            PackageChangeKind::Create,
                            state,
                        ),
                        "story.create_card",
                        args,
                    );
                    if state == PackageChangeState::Review {
                        ch.detail = Some(
                            "Its place isn't in your project; it will go to the Parking Lot."
                                .into(),
                        );
                    }
                    ups.push(ch);
                    created.insert(cd.id.clone(), format!("new:card:{}", cd.id));
                }
            }
        }
        for ch in &mut ups {
            if ch.state == PackageChangeState::Conflict && ch.applicable {
                ch.selected_by_default = false;
                ch.detail = Some(format!(
                    "{} — this item also changed in your project since the package was exported. Selecting it replaces your version.",
                    ch.detail.clone().unwrap_or_default()
                ));
            }
        }
        plan.changes.extend(ups);
        let stale = changed_since_export(x.c, x.m)?;
        if stale > 0 {
            plan.stale = true;
            plan.compatibility = "Stale".into();
            plan.stale_title = Some("This package was created from an older project state.".into());
        }
    } else {
        plan.warnings.push(format!(
            "This Story Board comes from another project (“{}”). It can be imported as a new copy with new identities.",
            x.m.source_project_title
        ));
    }
    // Comments follow same-project additions first, otherwise the copies.
    let mut targets = copied;
    targets.extend(created);
    plan_generic_comments(x, &s.comments, &targets, &mut plan)?;
    Ok(plan)
}

fn reject_other_project(x: &Ctx<'_>, what: &str) -> Option<Plan> {
    if x.same_project {
        return None;
    }
    Some(Plan {
        compatibility: "Rejected".into(),
        rejection: Some(format!(
            "This {what} belongs to another project (“{}”). It refers to scenes and schedule items that don't exist here, so it can't be imported.",
            x.m.source_project_title
        )),
        source_version: x.m.source_project_title.clone(),
        host_version: "—".into(),
        ..Default::default()
    })
}

fn mark_stale(x: &Ctx<'_>, plan: &mut Plan) -> AppResult<()> {
    if changed_since_export(x.c, x.m)? > 0 {
        plan.stale = true;
        plan.compatibility = "Stale".into();
        plan.stale_title = Some("This package was created from an older project state.".into());
    }
    Ok(())
}

fn plan_breakdown(x: &Ctx<'_>, content: &Value) -> AppResult<Plan> {
    if let Some(p) = reject_other_project(x, "Breakdown package") {
        return Ok(p);
    }
    let b: BreakdownContent = exchange::parse_content(content)?;
    let Some((source_id, draft_id, draft_name)) = exchange::active_source(x.c)? else {
        return Ok(Plan {
            compatibility: "Rejected".into(),
            rejection: Some(
                "Choose a Production Source in this project before importing breakdown changes."
                    .into(),
            ),
            source_version: b.source_draft_name,
            host_version: "—".into(),
            ..Default::default()
        });
    };
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: format!("Production Source: {}", b.source_draft_name),
        host_version: format!("Production Source: {draft_name}"),
        can_create_record: true,
        ..Default::default()
    };
    if b.source_draft_id != draft_id {
        plan.stale = true;
        plan.compatibility = "Stale".into();
        plan.stale_title = Some(format!(
            "This package was created from {}. Your Production Source is now {draft_name}.",
            b.source_draft_name
        ));
    }
    let host_scenes = exchange::load_scenes(x.c, &draft_id)?;
    for e in &b.elements {
        let host: Option<(Option<String>, i64, bool)> =
            x.c.query_row(
                "SELECT notes, rev, deleted_at IS NOT NULL FROM breakdown_element WHERE id=?1",
                [&e.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let label = format!("{} · {}", e.category, excerpt(&e.display_name, 40));
        match host {
            Some((_, _, true)) => plan.changes.push(gone_change(
                format!("bd:{}", e.id),
                "breakdown.changed",
                "Changed breakdown elements",
                label,
            )),
            Some((notes, rev, false)) => {
                if notes.clone().unwrap_or_default() != e.notes.clone().unwrap_or_default() {
                    let mut ch = with_op(
                        change(
                            format!("bd:{}", e.id),
                            "breakdown.changed",
                            "Changed breakdown elements",
                            label,
                            PackageChangeKind::Update,
                            update_state(x, "breakdown_element", &e.id, rev),
                        ),
                        "breakdown.update_element",
                        json!({ "id": e.id, "notes": e.notes.clone().unwrap_or_default() }),
                    );
                    ch.detail = Some("Notes changed".into());
                    if ch.state == PackageChangeState::Conflict {
                        ch.selected_by_default = false;
                    }
                    plan.changes.push(ch);
                }
            }
            None => {
                let scene = host_scenes.iter().find(|s| s.id == e.scene_id).or_else(|| {
                    host_scenes
                        .iter()
                        .find(|s| s.lineage_id == e.scene_lineage_id)
                });
                let Some(scene) = scene else {
                    let mut ch = change(
                        format!("bd:new:{}", e.id),
                        "breakdown.new",
                        "New breakdown elements",
                        label,
                        PackageChangeKind::Info,
                        PackageChangeState::Unmapped,
                    );
                    ch.detail = Some("Its scene isn't part of your Production Source.".into());
                    plan.changes.push(ch);
                    continue;
                };
                let dup: bool = x.c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM breakdown_element WHERE source_id=?1 AND scene_id=?2 AND deleted_at IS NULL
                       AND confirmation_state IN ('Confirmed','Manual')
                       AND (catalog_item_id IS ?3 AND ?3 IS NOT NULL OR (category=?4 AND lower(display_name)=lower(?5))))",
                    params![source_id, scene.id, e.catalog_item_id, e.category, e.display_name],
                    |r| r.get(0),
                )?;
                if dup {
                    let mut ch = change(
                        format!("bd:new:{}", e.id),
                        "breakdown.present",
                        "Already in your breakdown",
                        label,
                        PackageChangeKind::Info,
                        PackageChangeState::None,
                    );
                    ch.detail = Some(scene_label(scene));
                    plan.changes.push(ch);
                    continue;
                }
                let existing_item: Option<String> = match &e.catalog_item_id {
                    Some(id) => x
                        .c
                        .query_row(
                            "SELECT id FROM catalog_item WHERE id=?1 AND category=?2 AND deleted_at IS NULL AND archived=0",
                            params![id, e.category],
                            |r| r.get(0),
                        )
                        .optional()?,
                    None => None,
                };
                let (catalog, state, note) = match existing_item {
                    Some(id) => (
                        json!({ "mode": "existing", "catalogItemId": id }),
                        PackageChangeState::Safe,
                        "Uses the same Catalog item",
                    ),
                    None => (
                        json!({ "mode": "auto" }),
                        PackageChangeState::Review,
                        "Matched against your Catalog when applied",
                    ),
                };
                let mut ch = with_op(
                    change(
                        format!("bd:new:{}", e.id),
                        "breakdown.new",
                        "New breakdown elements",
                        label,
                        PackageChangeKind::Create,
                        state,
                    ),
                    "breakdown.add_element",
                    json!({ "sceneId": scene.id, "category": e.category, "name": e.display_name, "notes": e.notes, "catalog": catalog }),
                );
                ch.detail = Some(format!("{} — {note}", scene_label(scene)));
                plan.changes.push(ch);
            }
        }
    }
    mark_stale(x, &mut plan)?;
    plan_generic_comments(x, &b.comments, &HashMap::new(), &mut plan)?;
    Ok(plan)
}

fn plan_shots(x: &Ctx<'_>, content: &Value) -> AppResult<Plan> {
    if let Some(p) = reject_other_project(x, "Shot List package") {
        return Ok(p);
    }
    let s: ShotsContent = exchange::parse_content(content)?;
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: "Shot List".into(),
        host_version: "Your Shot List".into(),
        can_create_record: true,
        ..Default::default()
    };
    let draft = match exchange::active_source(x.c)? {
        Some((_, d, _)) => Some(d),
        None => exchange::current_draft(x.c, None)?.map(|d| d.id),
    };
    let host_scenes = match &draft {
        Some(d) => exchange::load_scenes(x.c, d)?,
        None => vec![],
    };
    let mut created = HashMap::new();
    for sh in &s.shots {
        let host: Option<(String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>, i64, bool)> = x
            .c
            .query_row(
                "SELECT description, size, movement, angle, lens, camera_notes, characters_json, sound_note, rev, deleted_at IS NOT NULL FROM shot WHERE id=?1",
                [&sh.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?)),
            )
            .optional()?;
        let label = format!("Shot “{}”", excerpt(&sh.description, 40));
        match host {
            Some((.., true)) => plan.changes.push(gone_change(
                format!("shot:{}", sh.id),
                "shots.changed",
                "Changed shots",
                label,
            )),
            Some((desc, size, movement, angle, lens, notes, chars, sound, rev, false)) => {
                let mut args = serde_json::Map::new();
                let mut changed = vec![];
                diff_text(
                    "description",
                    &Some(desc),
                    &Some(sh.description.clone()),
                    &mut args,
                    "description",
                    &mut changed,
                );
                diff_text("size", &size, &sh.size, &mut args, "size", &mut changed);
                diff_text(
                    "movement",
                    &movement,
                    &sh.movement,
                    &mut args,
                    "movement",
                    &mut changed,
                );
                diff_text("angle", &angle, &sh.angle, &mut args, "angle", &mut changed);
                diff_text("lens", &lens, &sh.lens, &mut args, "lens", &mut changed);
                diff_text(
                    "camera notes",
                    &notes,
                    &sh.camera_notes,
                    &mut args,
                    "cameraNotes",
                    &mut changed,
                );
                diff_text(
                    "sound",
                    &sound,
                    &sh.sound_note,
                    &mut args,
                    "soundNote",
                    &mut changed,
                );
                let host_chars: Vec<String> = serde_json::from_str(&chars).unwrap_or_default();
                if host_chars != sh.characters {
                    args.insert("characters".into(), json!(sh.characters));
                    changed.push("characters".into());
                }
                if !changed.is_empty() {
                    args.insert("id".into(), json!(sh.id));
                    args.insert("expectedRev".into(), json!(rev));
                    let mut ch = with_op(
                        change(
                            format!("shot:{}", sh.id),
                            "shots.changed",
                            "Changed shots",
                            label,
                            PackageChangeKind::Update,
                            update_state(x, "shot", &sh.id, rev),
                        ),
                        "shot.update",
                        Value::Object(args),
                    );
                    ch.detail = Some(format!("Changed: {}", changed.join(", ")));
                    if ch.state == PackageChangeState::Conflict {
                        ch.selected_by_default = false;
                    }
                    plan.changes.push(ch);
                }
            }
            None => {
                let scene_id = host_scenes
                    .iter()
                    .find(|h| h.id == sh.scene_id)
                    .or_else(|| {
                        host_scenes
                            .iter()
                            .find(|h| h.lineage_id == sh.scene_lineage_id)
                    })
                    .map(|h| h.id.clone());
                let Some(scene_id) = scene_id else {
                    let mut ch = change(
                        format!("shot:new:{}", sh.id),
                        "shots.new",
                        "New shots",
                        label,
                        PackageChangeKind::Info,
                        PackageChangeState::Unmapped,
                    );
                    ch.detail = Some("Its scene isn't in your screenplay any more.".into());
                    plan.changes.push(ch);
                    continue;
                };
                let mut args = json!({ "sceneId": scene_id, "description": sh.description, "characters": sh.characters });
                for (k, v) in [
                    ("size", &sh.size),
                    ("movement", &sh.movement),
                    ("angle", &sh.angle),
                    ("lens", &sh.lens),
                    ("cameraNotes", &sh.camera_notes),
                    ("soundNote", &sh.sound_note),
                ] {
                    if let Some(v) = v {
                        args[k] = json!(v);
                    }
                }
                plan.changes.push(with_op(
                    change(
                        format!("shot:new:{}", sh.id),
                        "shots.new",
                        "New shots",
                        label,
                        PackageChangeKind::Create,
                        PackageChangeState::Safe,
                    ),
                    "shot.create",
                    args,
                ));
                created.insert(sh.id.clone(), format!("shot:new:{}", sh.id));
            }
        }
    }
    mark_stale(x, &mut plan)?;
    plan_generic_comments(x, &s.comments, &created, &mut plan)?;
    Ok(plan)
}

fn plan_schedule(x: &Ctx<'_>, content: &Value) -> AppResult<Plan> {
    if let Some(p) = reject_other_project(x, "Schedule package") {
        return Ok(p);
    }
    let s: ScheduleContent = exchange::parse_content(content)?;
    let exists: bool = x.c.query_row(
        "SELECT EXISTS(SELECT 1 FROM shooting_schedule WHERE id=?1 AND deleted_at IS NULL)",
        [&s.schedule_id],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(Plan {
            compatibility: "Rejected".into(),
            rejection: Some(
                "This schedule doesn't exist in your project any more, so it can't be imported."
                    .into(),
            ),
            source_version: s.schedule_name,
            host_version: "—".into(),
            ..Default::default()
        });
    }
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: s.schedule_name.clone(),
        host_version: "Your shooting schedule".into(),
        can_create_record: true,
        ..Default::default()
    };
    let mut created: HashMap<String, String> = HashMap::new();
    let pkg_days: HashSet<&str> = s.days.iter().map(|d| d.id.as_str()).collect();
    let mut sorted = s.days.clone();
    sorted.sort_by(|a, b| a.position.cmp(&b.position).then(a.id.cmp(&b.id)));
    for (i, d) in sorted.iter().enumerate() {
        let label = match &d.date {
            Some(date) => format!("Day {} · {date}", i + 1),
            None => format!("Day {}", i + 1),
        };
        let host: Option<(Option<String>, Option<String>, i64, bool)> = x
            .c
            .query_row("SELECT shoot_date, notes, rev, deleted_at IS NOT NULL FROM shooting_day WHERE id=?1", [&d.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .optional()?;
        match host {
            Some((.., true)) => plan.changes.push(gone_change(
                format!("day:{}", d.id),
                "schedule.days",
                "Shooting days",
                label.clone(),
            )),
            Some((date, notes, rev, false)) => {
                let state = update_state(x, "shooting_day", &d.id, rev);
                if notes.clone().unwrap_or_default() != d.notes.clone().unwrap_or_default() {
                    let mut ch = with_op(
                        change(
                            format!("daynotes:{}", d.id),
                            "schedule.notes",
                            "Day notes",
                            label.clone(),
                            PackageChangeKind::Update,
                            state,
                        ),
                        "schedule.set_day_notes",
                        json!({ "dayId": d.id, "notes": d.notes }),
                    );
                    ch.detail = Some(format!(
                        "“{}”",
                        excerpt(&d.notes.clone().unwrap_or_default(), 60)
                    ));
                    ch.selected_by_default = state == PackageChangeState::Safe;
                    plan.changes.push(ch);
                }
                if date != d.date {
                    let mut ch = with_op(
                        change(
                            format!("daydate:{}", d.id),
                            "schedule.dates",
                            "Shooting dates",
                            label.clone(),
                            PackageChangeKind::Update,
                            PackageChangeState::Review,
                        ),
                        "schedule.set_day_date",
                        json!({ "dayId": d.id, "date": d.date }),
                    );
                    ch.detail = Some(format!(
                        "{} → {}",
                        date.unwrap_or_else(|| "no date".into()),
                        d.date.clone().unwrap_or_else(|| "no date".into())
                    ));
                    ch.selected_by_default = false;
                    plan.changes.push(ch);
                }
            }
            None => {
                let mut args = json!({ "scheduleId": s.schedule_id, "offDay": d.is_off_day });
                if let Some(date) = &d.date {
                    args["date"] = json!(date);
                }
                if let Some(n) = &d.notes {
                    args["notes"] = json!(n);
                }
                let mut ch = with_op(
                    change(
                        format!("day:new:{}", d.id),
                        "schedule.newdays",
                        "New shooting days",
                        label.clone(),
                        PackageChangeKind::Create,
                        PackageChangeState::Safe,
                    ),
                    "schedule.create_day",
                    args,
                );
                ch.detail = Some("Added at the end of your schedule".into());
                plan.changes.push(ch);
                created.insert(d.id.clone(), format!("day:new:{}", d.id));
            }
        }
        for st in &d.strips {
            let host: Option<(Option<String>, bool, i64)> = x
                .c
                .query_row("SELECT day_id, deleted_at IS NOT NULL OR archived = 1, rev FROM schedule_strip WHERE id=?1", [&st.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .optional()?;
            let target_day = if created.contains_key(&d.id) {
                json!(format!("$ref:{}", created[&d.id]))
            } else {
                json!(d.id)
            };
            match host {
                Some((host_day, false, _)) if host_day.as_deref() != Some(d.id.as_str()) => {
                    let mut ch = with_op(
                        change(
                            format!("strip:{}", st.id),
                            "schedule.moves",
                            "Scene assignments",
                            format!("{} → {label}", excerpt(&st.heading, 40)),
                            PackageChangeKind::Update,
                            PackageChangeState::Review,
                        ),
                        "schedule.move_strip",
                        json!({ "stripId": st.id, "dayId": target_day }),
                    );
                    ch.detail = Some(
                        "Moves a scene in your schedule — applied only when you select it.".into(),
                    );
                    ch.selected_by_default = false;
                    plan.changes.push(ch);
                }
                Some(_) => {}
                None => {
                    let mut ch = change(
                        format!("strip:{}", st.id),
                        "schedule.moves",
                        "Scene assignments",
                        excerpt(&st.heading, 40),
                        PackageChangeKind::Info,
                        PackageChangeState::Unmapped,
                    );
                    ch.detail = Some("This scene isn't in your schedule.".into());
                    plan.changes.push(ch);
                }
            }
        }
    }
    let local_only: i64 = {
        let mut stmt =
            x.c.prepare("SELECT id FROM shooting_day WHERE schedule_id=?1 AND deleted_at IS NULL")?;
        let ids: Vec<String> = stmt
            .query_map([&s.schedule_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        ids.iter()
            .filter(|id| !pkg_days.contains(id.as_str()))
            .count() as i64
    };
    if local_only > 0 {
        let mut ch = change(
            "schedule:kept".into(),
            "schedule.kept",
            "Your days not in the package",
            format!("{local_only} day(s) kept"),
            PackageChangeKind::Info,
            PackageChangeState::None,
        );
        ch.detail = Some("A Schedule package never deletes your shooting days.".into());
        plan.changes.push(ch);
    }
    if s.days.iter().any(|d| !d.markers.is_empty()) {
        plan.warnings.push("Breaks and company moves in the package are shown for reference; add them in the schedule if you want them.".into());
    }
    mark_stale(x, &mut plan)?;
    plan_generic_comments(x, &s.comments, &created, &mut plan)?;
    Ok(plan)
}

fn plan_call_review(x: &Ctx<'_>, content: &Value) -> AppResult<Plan> {
    if let Some(p) = reject_other_project(x, "Call Sheet review package") {
        return Ok(p);
    }
    let s: CallReviewContent = exchange::parse_content(content)?;
    let mut plan = Plan {
        compatibility: "Valid".into(),
        source_version: format!("{} (revision {})", s.title, s.revision),
        host_version: "Your call sheet".into(),
        can_create_record: true,
        ..Default::default()
    };
    if let Some((rev, false)) = host_row(x.c, "call_sheet", &s.call_sheet_id)?
        && rev != s.rev
    {
        plan.stale = true;
        plan.compatibility = "Stale".into();
        plan.stale_title =
            Some("This call sheet changed after the review package was exported.".into());
    }
    plan.warnings.push("Call sheet feedback is added as comments. The shooting schedule is never changed by this import.".into());
    plan_generic_comments(x, &s.comments, &HashMap::new(), &mut plan)?;
    Ok(plan)
}

fn build_plan(
    c: &Connection,
    host_project_id: &str,
    m: &PackageManifest,
    content: &Value,
    prior: &HashSet<String>,
    session_files: bool,
) -> AppResult<Plan> {
    let x = Ctx {
        c,
        m,
        same_project: m.source_project_id == host_project_id,
        prior,
    };
    let mut plan = match m.package_type {
        PackageType::ScriptReview | PackageType::Response => plan_script(&x, content)?,
        PackageType::Story => plan_story(&x, content, session_files)?,
        PackageType::Breakdown => plan_breakdown(&x, content)?,
        PackageType::Shots => plan_shots(&x, content)?,
        PackageType::Schedule => plan_schedule(&x, content)?,
        PackageType::CallReview => plan_call_review(&x, content)?,
        PackageType::Backup | PackageType::Project => Plan {
            compatibility: "Rejected".into(),
            rejection: Some(
                "This is a whole-project package. Use Open Project Package… instead.".into(),
            ),
            ..Default::default()
        },
    };
    if plan.compatibility == "Rejected" {
        plan.changes.clear();
        plan.can_copy_as_new = false;
    }
    Ok(plan)
}

fn groups_of(changes: &[PackageChange]) -> Vec<PackageChangeGroup> {
    let mut out: Vec<PackageChangeGroup> = vec![];
    for ch in changes.iter().filter(|c| !c.copy_only) {
        let rank = |s: PackageChangeState| match s {
            PackageChangeState::Conflict => 5,
            PackageChangeState::Ambiguous => 4,
            PackageChangeState::Unmapped => 3,
            PackageChangeState::Review => 2,
            PackageChangeState::Safe => 1,
            PackageChangeState::None => 0,
        };
        match out.iter_mut().find(|g| g.key == ch.group) {
            Some(g) => {
                g.count += 1;
                if ch.applicable {
                    g.applicable += 1;
                }
                if rank(ch.state) > rank(g.state) {
                    g.state = ch.state;
                }
            }
            None => out.push(PackageChangeGroup {
                key: ch.group.clone(),
                label: ch.group_label.clone(),
                count: 1,
                state: ch.state,
                applicable: ch.applicable as u32,
            }),
        }
    }
    let copies = changes.iter().filter(|c| c.copy_only).count();
    if copies > 0 {
        out.push(PackageChangeGroup {
            key: "copy".into(),
            label: "Copy as new outline (new identities)".into(),
            count: copies as u32,
            state: PackageChangeState::Review,
            applicable: copies as u32,
        });
    }
    out
}

// ============================================================ session store

struct SessionRow {
    id: String,
    package_id: String,
    package_type: String,
    file_name: String,
    source_project_id: String,
    source_project_title: String,
    compatibility: String,
    manifest_json: String,
    content_json: String,
    preview_json: String,
    status: String,
    result_json: Option<String>,
    created_at: i64,
}

fn load_session(c: &Connection, id: &str) -> AppResult<SessionRow> {
    c.query_row(
        "SELECT id, package_id, package_type, file_name, source_project_id, source_project_title, compatibility,
                manifest_json, content_json, preview_json, status, result_json, created_at
         FROM sys_import_session WHERE id=?1",
        [id],
        |r| {
            Ok(SessionRow {
                id: r.get(0)?,
                package_id: r.get(1)?,
                package_type: r.get(2)?,
                file_name: r.get(3)?,
                source_project_id: r.get(4)?,
                source_project_title: r.get(5)?,
                compatibility: r.get(6)?,
                manifest_json: r.get(7)?,
                content_json: r.get(8)?,
                preview_json: r.get(9)?,
                status: r.get(10)?,
                result_json: r.get(11)?,
                created_at: r.get(12)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| AppError::not_found("import"))
}

fn parse<T: for<'de> Deserialize<'de>>(s: &str) -> AppResult<T> {
    serde_json::from_str(s).map_err(|e| AppError::internal(e.to_string()))
}

fn to_json<T: Serialize>(v: &T) -> AppResult<String> {
    serde_json::to_string(v).map_err(|e| AppError::internal(e.to_string()))
}

fn session_dto(row: &SessionRow) -> AppResult<PackageImportSession> {
    let m: PackageManifest = parse(&row.manifest_json)?;
    let plan: Plan = parse(&row.preview_json)?;
    let result: Option<PackageImportResult> = row.result_json.as_deref().map(parse).transpose()?;
    let ty = PackageType::parse(&row.package_type).unwrap_or(m.package_type);
    Ok(PackageImportSession {
        id: row.id.clone(),
        package_id: row.package_id.clone(),
        package_type: ty,
        type_label: ty.label().to_string(),
        file_name: row.file_name.clone(),
        source_project_id: row.source_project_id.clone(),
        source_project_title: row.source_project_title.clone(),
        same_project: false,
        source_version: plan.source_version.clone(),
        host_version: plan.host_version.clone(),
        compatibility: row.compatibility.clone(),
        stale: plan.stale,
        stale_title: plan.stale_title.clone(),
        rejection: plan.rejection.clone(),
        exported_at: m.exported_at,
        exported_by: m.originating_user.display_name.clone(),
        comments_included: m.comments_included,
        attachments_included: m.attachments_included,
        groups: groups_of(&plan.changes),
        changes: plan.changes,
        warnings: plan.warnings,
        can_copy_as_new: plan.can_copy_as_new,
        can_create_record: plan.can_create_record,
        can_view: matches!(ty, PackageType::ScriptReview | PackageType::Response),
        status: row.status.clone(),
        result,
        created_at: row.created_at,
    })
}

/// Source comment ids already imported from this package by earlier sessions.
fn prior_imported(c: &Connection, package_id: &str) -> AppResult<HashSet<String>> {
    let mut stmt = c.prepare(
        "SELECT result_json FROM sys_import_session WHERE package_id=?1 AND status IN ('Applied','PartiallyApplied','PendingReview') AND result_json IS NOT NULL",
    )?;
    let rows: Vec<String> = stmt
        .query_map([package_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = HashSet::new();
    for r in rows {
        if let Ok(res) = serde_json::from_str::<PackageImportResult>(&r) {
            out.extend(res.imported_comment_ids);
        }
    }
    Ok(out)
}

fn session_files_dir(core: &AppCore, session_id: &str) -> AppResult<PathBuf> {
    let s = core.project()?;
    Ok(s.layout.cache().join("imports").join(session_id))
}

// ================================================================ ops

/// `packages.open_exchange`: validate the package and build the Import Session
/// preview. Validation failure = zero project mutation (IEX-032).
pub(crate) fn open_exchange(
    core: &AppCore,
    actor: &Actor,
    args: super::backup::PackagePathArgs,
) -> AppResult<PackageImportSession> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let path = PathBuf::from(args.path.trim());
    let exchange_types: Vec<PackageType> = PackageType::ALL
        .into_iter()
        .filter(|t| t.is_exchange())
        .collect();
    let opened = format::open_package(
        &path,
        &core.config.app_data_dir.join("tmp"),
        &exchange_types,
        "an exchange or review package",
    )?;
    let m = opened.manifest.clone();
    let content = opened.content()?;
    let session_id = new_id();
    // Keep attachment files for a later "Copy as New" (project cache; rebuildable, never canonical).
    let mut session_files = false;
    if m.package_type == PackageType::Story && m.attachments_included {
        let story: StoryContent = exchange::parse_content(&content)?;
        if !story.attachments.is_empty() {
            let dir = session_files_dir(core, &session_id)?;
            for at in &story.attachments {
                let src = opened.entry_path(&at.entry)?;
                let dest = dir.join(openframe_security::validate_relative(&at.entry)?);
                if let Some(p) = dest.parent() {
                    std::fs::create_dir_all(p)?;
                }
                if src.is_file() {
                    std::fs::copy(&src, &dest)?;
                }
            }
            session_files = true;
        }
    }
    let project_id = s.project_id();
    let plan = s.store.read(|c| {
        let prior = prior_imported(c, &m.package_id)?;
        build_plan(c, &project_id, &m, &content, &prior, session_files)
    })?;
    let now = now_ms();
    let row = SessionRow {
        id: session_id.clone(),
        package_id: m.package_id.clone(),
        package_type: m.package_type.as_str().to_string(),
        file_name: opened.file_name.clone(),
        source_project_id: m.source_project_id.clone(),
        source_project_title: m.source_project_title.clone(),
        compatibility: plan.compatibility.clone(),
        manifest_json: to_json(&m)?,
        content_json: to_json(&content)?,
        preview_json: to_json(&plan)?,
        status: if plan.compatibility == "Rejected" {
            "Rejected".into()
        } else {
            "Previewed".into()
        },
        result_json: None,
        created_at: now,
    };
    s.store.mutate(
        actor,
        MutationMeta::new("packages.open_exchange", format!("Previewed package “{}”", row.file_name), Capability::View).not_undoable().quiet(),
        |tx| {
            tx.conn().execute(
                "INSERT INTO sys_import_session(id, package_id, package_type, file_name, package_sha256, source_project_id,
                    source_project_title, compatibility, manifest_json, content_json, preview_json, status, created_by, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
                params![
                    row.id,
                    row.package_id,
                    row.package_type,
                    row.file_name,
                    opened.sha256,
                    row.source_project_id,
                    row.source_project_title,
                    row.compatibility,
                    row.manifest_json,
                    row.content_json,
                    row.preview_json,
                    row.status,
                    actor.user_id,
                    now
                ],
            )?;
            Ok(())
        },
    )?;
    let mut dto = session_dto(&row)?;
    dto.same_project = m.source_project_id == project_id;
    Ok(dto)
}

pub(crate) fn get_session(
    core: &AppCore,
    actor: &Actor,
    a: PackageSessionArgs,
) -> AppResult<PackageImportSession> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let pid = s.project_id();
    let row = s.store.read(|c| load_session(c, &a.session_id))?;
    let mut dto = session_dto(&row)?;
    dto.same_project = row.source_project_id == pid;
    Ok(dto)
}

pub(crate) fn list_sessions(
    core: &AppCore,
    actor: &Actor,
    a: PackageListArgs,
) -> AppResult<Vec<PackageSessionSummary>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id, file_name, package_type, source_project_title, compatibility, status, result_json, created_at
             FROM sys_import_session ORDER BY created_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([a.limit.unwrap_or(100).min(500)], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, i64>(7)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows
            .into_iter()
            .map(|(id, file_name, ty, src, compat, status, result, created_at)| {
                let res: Option<PackageImportResult> = result.and_then(|r| serde_json::from_str(&r).ok());
                let ty = PackageType::parse(&ty).unwrap_or(PackageType::Story);
                PackageSessionSummary {
                    id,
                    file_name,
                    package_type: ty,
                    type_label: ty.label().to_string(),
                    source_project_title: src,
                    compatibility: compat,
                    can_undo: res.as_ref().map(|r| r.can_undo && !r.steps.is_empty()).unwrap_or(false) && status != "Undone",
                    message: res.map(|r| r.message),
                    status,
                    created_at,
                }
            })
            .collect())
    })
}

pub(crate) fn package_log(
    core: &AppCore,
    actor: &Actor,
    a: PackageListArgs,
) -> AppResult<Vec<PackageLogEntry>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id, direction, package_type, file_name, summary, actor_name, at FROM sys_package_log ORDER BY at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([a.limit.unwrap_or(100).min(500)], |r| {
                Ok(PackageLogEntry {
                    id: r.get(0)?,
                    direction: r.get(1)?,
                    package_type: r.get(2)?,
                    file_name: r.get(3)?,
                    summary: r.get(4)?,
                    actor_name: r.get(5)?,
                    at: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn set_status(
    core: &AppCore,
    actor: &Actor,
    id: &str,
    status: &str,
    extra: impl FnOnce(&Connection) -> AppResult<()>,
) -> AppResult<()> {
    let s = core.project()?;
    s.store.mutate(
        actor,
        MutationMeta::new("packages.session", "Updated import", Capability::View)
            .not_undoable()
            .quiet(),
        |tx| {
            tx.conn().execute(
                "UPDATE sys_import_session SET status=?1, updated_at=?2 WHERE id=?3",
                params![status, now_ms(), id],
            )?;
            extra(tx.conn())
        },
    )
}

/// `packages.cancel_import`: "Cancel leaves the project unchanged" (IEX-022).
pub(crate) fn cancel_import(
    core: &AppCore,
    actor: &Actor,
    a: PackageSessionArgs,
) -> AppResult<PackageImportSession> {
    let s = core.project()?;
    let row = s.store.read(|c| load_session(c, &a.session_id))?;
    if row.status == "Previewed" {
        set_status(core, actor, &a.session_id, "Cancelled", |_| Ok(()))?;
        let _ = std::fs::remove_dir_all(session_files_dir(core, &a.session_id)?);
    }
    get_session(core, actor, a)
}

fn exchange_actor(user: &PackageUser, host: &Actor, package_id: &str) -> Actor {
    Actor {
        user_id: if is_uuid(&user.user_id) {
            user.user_id.clone()
        } else {
            host.user_id.clone()
        },
        display_name: if user.display_name.trim().is_empty() {
            host.display_name.clone()
        } else {
            excerpt(&user.display_name, 80)
        },
        role: host.role,
        origin: ActorOrigin::Exchange {
            package_id: package_id.to_string(),
        },
    }
}

fn top_done_seq(c: &Connection, actor_id: &str) -> AppResult<i64> {
    Ok(c.query_row(
        "SELECT COALESCE(MAX(seq), 0) FROM sys_undo WHERE actor_id IS ?1 AND state='done'",
        [actor_id],
        |r| r.get(0),
    )?)
}

/// Undo `steps` newest-first through the normal history (never overwrites later work).
fn undo_steps(core: &AppCore, host: &Actor, package_id: &str, steps: &[UndoStep]) -> AppResult<()> {
    let s = core.project()?;
    let mut sorted = steps.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.seq));
    // Verify first: each step must still be done and be its author's latest step
    // once the newer steps of this import are undone.
    s.store.read(|c| {
        for st in &sorted {
            let done: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM sys_undo WHERE seq=?1 AND state='done')",
                [st.seq],
                |r| r.get(0),
            )?;
            let later: bool = c.query_row(
                "SELECT EXISTS(SELECT 1 FROM sys_undo WHERE actor_id IS ?1 AND state='done' AND seq > ?2)",
                params![st.actor_id, st.seq],
                |r| r.get(0),
            )?;
            let later_ours = sorted.iter().any(|o| o.actor_id == st.actor_id && o.seq > st.seq);
            if !done || (later && !later_ours) {
                return Err(AppError::new(
                    "conflict.undo_import",
                    "This import can't be undone because the imported content was changed afterwards. Your latest work was kept.",
                ));
            }
        }
        Ok(())
    })?;
    for st in &sorted {
        let a = exchange_actor(
            &PackageUser {
                user_id: st.actor_id.clone(),
                display_name: st.actor_name.clone(),
            },
            host,
            package_id,
        );
        s.store.undo(&a)?;
    }
    Ok(())
}

fn resolve_refs(
    v: &Value,
    results: &HashMap<String, String>,
    files_dir: &Path,
) -> AppResult<Value> {
    Ok(match v {
        Value::String(s) if s.starts_with("$ref:") => {
            let key = &s[5..];
            Value::String(
                results
                    .get(key)
                    .cloned()
                    .ok_or_else(|| AppError::internal(format!("unresolved reference {key}")))?,
            )
        }
        Value::String(s) if s.starts_with("$file:") => {
            let rel = openframe_security::validate_relative(&s[6..])?;
            Value::String(files_dir.join(rel).to_string_lossy().into_owned())
        }
        Value::Array(a) => Value::Array(
            a.iter()
                .map(|x| resolve_refs(x, results, files_dir))
                .collect::<AppResult<_>>()?,
        ),
        Value::Object(o) => {
            let mut out = serde_json::Map::new();
            for (k, x) in o {
                out.insert(k.clone(), resolve_refs(x, results, files_dir)?);
            }
            Value::Object(out)
        }
        other => other.clone(),
    })
}

fn refs_in(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) if s.starts_with("$ref:") => out.push(s[5..].to_string()),
        Value::Array(a) => a.iter().for_each(|x| refs_in(x, out)),
        Value::Object(o) => o.values().for_each(|x| refs_in(x, out)),
        _ => {}
    }
}

/// The id of whatever an operation created: a string, `{id}` or `{comment: {id}}`.
fn created_id(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                o.get("comment")
                    .and_then(|c| c.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            }),
        _ => None,
    }
}

fn group_noun(ch: &PackageChange) -> String {
    ch.group_label.to_lowercase()
}

fn were_applied(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one} was applied")
    } else {
        format!("{n} {many} were applied")
    }
}

/// `packages.apply_import`: re-plan against the current project, apply the
/// chosen changes through the normal module operations, all-or-nothing.
pub(crate) fn apply_import(
    core: &AppCore,
    actor: &Actor,
    a: PackageApplyArgs,
) -> AppResult<PackageImportSession> {
    let s = core.project()?;
    let project_id = s.project_id();
    let row = s.store.read(|c| load_session(c, &a.session_id))?;
    if !matches!(row.status.as_str(), "Previewed" | "Failed" | "Rejected") {
        return Err(AppError::conflict(
            "This import was already finished. Open the package again to import it again.",
        ));
    }
    let m: PackageManifest = parse(&row.manifest_json)?;
    let content: Value = parse(&row.content_json)?;
    let files_dir = session_files_dir(core, &row.id)?;
    let session_files = files_dir.is_dir();
    let plan = s.store.read(|c| {
        let prior = prior_imported(c, &m.package_id)?;
        build_plan(c, &project_id, &m, &content, &prior, session_files)
    })?;
    if plan.compatibility == "Rejected" {
        return Err(AppError::import(
            "package_rejected",
            plan.rejection
                .clone()
                .unwrap_or_else(|| "This package can't be imported into this project.".into()),
        ));
    }
    let selected: HashSet<&str> = a.selected.iter().map(String::as_str).collect();
    let is_queue = |c: &PackageChange| c.queue.is_some();
    let wanted = |c: &PackageChange| -> bool {
        if !c.applicable {
            return false;
        }
        match a.mode {
            PackageApplyMode::AllSafe => {
                !c.copy_only && (c.state == PackageChangeState::Safe || is_queue(c))
            }
            PackageApplyMode::CommentsOnly => {
                !c.copy_only
                    && c.kind == PackageChangeKind::Comment
                    && (c.state == PackageChangeState::Safe || is_queue(c))
            }
            PackageApplyMode::Selected => selected.contains(c.id.as_str()),
            PackageApplyMode::CopyAsNew => c.copy_only,
            PackageApplyMode::Record => false,
        }
    };
    if a.mode == PackageApplyMode::CopyAsNew && !plan.can_copy_as_new {
        return Err(AppError::invalid_input(
            "This package can't be copied as new content.",
        ));
    }
    if a.mode == PackageApplyMode::Record && !plan.can_create_record {
        return Err(AppError::invalid_input(
            "A separate review record can't be created for this package.",
        ));
    }
    // Chosen changes, dropping any whose referenced change isn't chosen too.
    let mut chosen: Vec<&PackageChange> = vec![];
    let mut chosen_ids: HashSet<&str> = HashSet::new();
    for ch in plan.changes.iter().filter(|c| wanted(c)) {
        let mut deps = vec![];
        if let Some(args) = &ch.args {
            refs_in(args, &mut deps);
        }
        if deps.iter().all(|d| chosen_ids.contains(d.as_str())) {
            chosen_ids.insert(ch.id.as_str());
            chosen.push(ch);
        }
    }
    let ops: Vec<&PackageChange> = chosen
        .iter()
        .copied()
        .filter(|c| c.op.is_some() && c.queue.is_none())
        .collect();
    let queued: Vec<&PackageChange> = chosen
        .iter()
        .copied()
        .filter(|c| c.queue.is_some())
        .collect();
    if ops.iter().any(|c| c.kind != PackageChangeKind::Comment)
        || a.mode == PackageApplyMode::Record
    {
        actor.require(Capability::Import, "import changes into this project")?;
    }
    if !ops.is_empty() || !queued.is_empty() {
        actor.require(Capability::Comment, "add imported comments")?;
    }
    // "Create Backup Before Import": a verified database copy in the project's backups folder.
    let safety_backup = if a.create_backup {
        let dest = s
            .layout
            .safety_backups()
            .join(format!("pre-import-{}.sqlite", now_ms()));
        std::fs::create_dir_all(s.layout.safety_backups())?;
        s.store
            .with_writer(|c| openframe_persistence::backup_to(c, &dest))?;
        Some(dest.to_string_lossy().into_owned())
    } else {
        None
    };
    let local = Actor {
        origin: ActorOrigin::Exchange {
            package_id: m.package_id.clone(),
        },
        ..actor.clone()
    };
    let mut steps: Vec<UndoStep> = vec![];
    let mut results: HashMap<String, String> = HashMap::new();
    let mut imported_comment_ids: Vec<String> = vec![];
    let mut record_id: Option<String> = None;
    let record_step = |steps: &mut Vec<UndoStep>, who: &Actor, before: i64| -> AppResult<()> {
        let after = s.store.read(|c| top_done_seq(c, &who.user_id))?;
        if after > before {
            steps.push(UndoStep {
                actor_id: who.user_id.clone(),
                actor_name: who.display_name.clone(),
                seq: after,
            });
        }
        Ok(())
    };
    let outcome = (|| -> AppResult<()> {
        for ch in &ops {
            let who = match (&ch.kind, &ch.author) {
                (PackageChangeKind::Comment, Some(u)) => exchange_actor(u, actor, &m.package_id),
                _ => exchange_actor(&m.originating_user, actor, &m.package_id),
            };
            let args = resolve_refs(
                ch.args.as_ref().unwrap_or(&Value::Null),
                &results,
                &files_dir,
            )?;
            let before = s.store.read(|c| top_done_seq(c, &who.user_id))?;
            let out = core
                .dispatch(&who, ch.op.as_deref().unwrap_or_default(), args)
                .map_err(|e| {
                    let msg = format!("{} could not be applied: {}", ch.label, e.message);
                    AppError::import("apply_failed", msg).with_detail(e.code_str().to_string())
                })?;
            record_step(&mut steps, &who, before)?;
            if let Some(id) = created_id(&out) {
                results.insert(ch.id.clone(), id);
            }
            if let Some(src) = &ch.source_comment_id {
                imported_comment_ids.push(src.clone());
            }
        }
        if !queued.is_empty() {
            let before = s.store.read(|c| top_done_seq(c, &local.user_id))?;
            s.store.mutate(
                &local,
                MutationMeta::new(
                    "packages.queue",
                    format!("Kept {} imported note(s) in the Review Queue", queued.len()),
                    Capability::Comment,
                ),
                |tx| {
                    for ch in &queued {
                        let q = ch.queue.as_ref().expect("queue spec");
                        let author = ch.author.clone().unwrap_or_else(|| m.originating_user.clone());
                        tx.conn().execute(
                            "INSERT INTO review_queue_item(id, session_id, package_id, source_comment_id, kind, author_user_id, author_name,
                                body, quoted_text, source_label, reason, candidates_json, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)",
                            params![
                                new_id(),
                                row.id,
                                m.package_id,
                                ch.source_comment_id,
                                q.kind,
                                author.user_id,
                                author.display_name,
                                q.body,
                                q.quoted_text,
                                q.source_label,
                                q.reason,
                                to_json(&ch.candidates)?,
                                now_ms()
                            ],
                        )?;
                    }
                    Ok(())
                },
            )?;
            record_step(&mut steps, &local, before)?;
            imported_comment_ids.extend(queued.iter().filter_map(|c| c.source_comment_id.clone()));
        }
        if a.mode == PackageApplyMode::Record {
            let before = s.store.read(|c| top_done_seq(c, &local.user_id))?;
            let id = new_id();
            let comment_count = content
                .get("comments")
                .and_then(Value::as_array)
                .map(|v| v.len())
                .unwrap_or(0) as i64;
            let title = format!("{} — {}", m.package_type.label(), plan.source_version);
            s.store.mutate(
                &local,
                MutationMeta::new("packages.create_record", format!("Kept “{}” as a separate review record", row.file_name), Capability::Import)
                    .target("exchange_review_record", &id),
                |tx| {
                    tx.conn().execute(
                        "INSERT INTO exchange_review_record(id, session_id, package_id, package_type, source_project_id, source_project_title,
                            source_label, title, content_json, comment_count, exported_by, exported_at, created_by, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)",
                        params![
                            id,
                            row.id,
                            m.package_id,
                            m.package_type.as_str(),
                            m.source_project_id,
                            m.source_project_title,
                            plan.source_version,
                            title,
                            row.content_json,
                            comment_count,
                            m.originating_user.display_name,
                            m.exported_at,
                            actor.user_id,
                            now_ms()
                        ],
                    )?;
                    Ok(())
                },
            )?;
            record_step(&mut steps, &local, before)?;
            record_id = Some(id);
        }
        Ok(())
    })();
    let applied = ops.len();
    let result = match outcome {
        Err(e) => {
            // All-or-nothing: roll back everything this import applied.
            let rolled_back = undo_steps(core, actor, &m.package_id, &steps);
            let message = match &rolled_back {
                Ok(()) => format!(
                    "Import failed: {} Nothing was imported — your project was not changed.",
                    e.message
                ),
                Err(_) => format!(
                    "Import failed: {} Some of the {} changes applied before the failure could not be rolled back; check Activity.",
                    e.message,
                    steps.len()
                ),
            };
            PackageImportResult {
                status: "Failed".into(),
                message,
                applied: 0,
                queued: 0,
                pending: 0,
                record_id: None,
                safety_backup: safety_backup.clone(),
                can_undo: false,
                steps: vec![],
                imported_comment_ids: vec![],
            }
        }
        Ok(()) => {
            // Changes that were not applied and still need a decision. "Copy as New"
            // items are an alternative to the others, never "pending".
            let pending: Vec<&PackageChange> = plan
                .changes
                .iter()
                .filter(|c| c.applicable && !c.copy_only && !chosen_ids.contains(c.id.as_str()))
                .collect();
            let text_review = plan.changes.iter().filter(|c| c.group == "text").count();
            let comments_applied = ops
                .iter()
                .filter(|c| c.kind == PackageChangeKind::Comment)
                .count();
            let other_applied = applied - comments_applied;
            let mut done_parts = vec![];
            if comments_applied > 0 {
                done_parts.push(were_applied(comments_applied, "comment", "comments"));
            }
            if other_applied > 0 {
                done_parts.push(were_applied(other_applied, "change", "changes"));
            }
            let mut by_group: BTreeMap<String, usize> = BTreeMap::new();
            for c in &pending {
                *by_group.entry(group_noun(c)).or_default() += 1;
            }
            if text_review > 0 {
                *by_group.entry("scene text changes".into()).or_default() += text_review;
            }
            let remain_parts: Vec<String> =
                by_group.iter().map(|(g, n)| format!("{n} {g}")).collect();
            let remaining = pending.len() + text_review;
            let status = if applied == 0 && queued.is_empty() && record_id.is_none() {
                if remaining > 0 {
                    "PendingReview"
                } else {
                    "Applied"
                }
            } else if applied > 0 && remaining == 0 && queued.is_empty() {
                "Applied"
            } else if applied > 0 {
                "PartiallyApplied"
            } else {
                "PendingReview"
            };
            let mut message = match status {
                "Applied" if applied == 0 => {
                    "Nothing in this package needed to change your project.".to_string()
                }
                "Applied" => format!("Imported: {}.", done_parts.join("; ")),
                "PartiallyApplied" => {
                    format!("Import finished partially. {}", done_parts.join("; "))
                }
                _ => "Nothing was applied to your project content.".to_string(),
            };
            if status == "PartiallyApplied" && !remain_parts.is_empty() {
                message.push_str(&format!("; {} remain in Review.", remain_parts.join(", ")));
            } else if status == "PartiallyApplied" {
                message.push('.');
            } else if status == "PendingReview" && !remain_parts.is_empty() {
                message.push_str(&format!(" {} remain in Review.", remain_parts.join(", ")));
            }
            if !queued.is_empty() {
                message.push_str(&format!(
                    " {} waiting in the Review Queue — nothing was discarded.",
                    if queued.len() == 1 {
                        "1 note is".to_string()
                    } else {
                        format!("{} notes are", queued.len())
                    }
                ));
            }
            if record_id.is_some() {
                message.push_str(" The package was kept as a separate review record; your working content was not changed.");
            }
            PackageImportResult {
                status: status.into(),
                message,
                applied: applied as u32,
                queued: queued.len() as u32,
                pending: remaining as u32,
                record_id: record_id.clone(),
                safety_backup: safety_backup.clone(),
                can_undo: !steps.is_empty(),
                steps: steps.clone(),
                imported_comment_ids,
            }
        }
    };
    let status = result.status.clone();
    let result_json = to_json(&result)?;
    let from_seq = steps.iter().map(|s| s.seq).min();
    let to_seq = steps.iter().map(|s| s.seq).max();
    let selection_json = to_json(&a.selected)?;
    let mode_str = format!("{:?}", a.mode);
    set_status(core, actor, &row.id, &status, |c| {
        c.execute(
            "UPDATE sys_import_session SET apply_mode=?1, selection_json=?2, result_json=?3, apply_actor_id=?4, apply_actor_name=?5,
                undo_from_seq=?6, undo_to_seq=?7, safety_backup=?8, preview_json=?9, compatibility=?10 WHERE id=?11",
            params![
                mode_str,
                selection_json,
                result_json,
                m.originating_user.user_id,
                m.originating_user.display_name,
                from_seq,
                to_seq,
                safety_backup,
                to_json(&plan)?,
                plan.compatibility,
                row.id
            ],
        )?;
        Ok(())
    })?;
    log_package(
        &s.store,
        actor,
        "import",
        m.package_type,
        &m.package_id,
        &row.file_name,
        &result.message,
        Some(&row.id),
    )?;
    get_session(core, actor, PackageSessionArgs { session_id: row.id })
}

/// `packages.undo_import`: undo every step one applied import made (FSD §51.2:
/// a major imported package action is one undoable action where practical).
pub(crate) fn undo_import(
    core: &AppCore,
    actor: &Actor,
    a: PackageSessionArgs,
) -> AppResult<PackageImportSession> {
    let s = core.project()?;
    let row = s.store.read(|c| load_session(c, &a.session_id))?;
    let mut result: PackageImportResult = row
        .result_json
        .as_deref()
        .map(parse)
        .transpose()?
        .ok_or_else(|| AppError::conflict("This import hasn't been applied."))?;
    if row.status == "Undone" || result.steps.is_empty() {
        return Err(AppError::conflict(
            "There is nothing to undo for this import.",
        ));
    }
    let package_id = row.package_id.clone();
    undo_steps(core, actor, &package_id, &result.steps)?;
    result.status = "Undone".into();
    result.can_undo = false;
    result.message = format!(
        "Undid the import of “{}”. {}",
        row.file_name, result.message
    );
    result.imported_comment_ids.clear();
    let json = to_json(&result)?;
    set_status(core, actor, &row.id, "Undone", |c| {
        c.execute(
            "UPDATE sys_import_session SET result_json=?1 WHERE id=?2",
            params![json, row.id],
        )?;
        Ok(())
    })?;
    let ty = PackageType::parse(&row.package_type).unwrap_or(PackageType::Story);
    log_package(
        &s.store,
        actor,
        "import",
        ty,
        &package_id,
        &row.file_name,
        &format!("Undid import of “{}”", row.file_name),
        Some(&row.id),
    )?;
    get_session(core, actor, a)
}

// ============================================================ review queue

fn queue_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<PackageQueueItem> {
    let cands: String = r.get(7)?;
    Ok(PackageQueueItem {
        id: r.get(0)?,
        kind: r.get(1)?,
        author_name: r.get(2)?,
        body: r.get(3)?,
        quoted_text: r.get(4)?,
        source_label: r.get(5)?,
        reason: r.get(6)?,
        candidates: serde_json::from_str(&cands).unwrap_or_default(),
        status: r.get(8)?,
        attached_label: r.get(9)?,
        created_at: r.get(10)?,
        rev: r.get(11)?,
    })
}

const QUEUE_COLS: &str = "id, kind, author_name, body, quoted_text, source_label, reason, candidates_json, status, attached_label, created_at, rev";

pub(crate) fn review_queue(
    core: &AppCore,
    actor: &Actor,
    _a: PackageListArgs,
) -> AppResult<Vec<PackageQueueItem>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(&format!(
            "SELECT {QUEUE_COLS} FROM review_queue_item WHERE deleted_at IS NULL
             ORDER BY status = 'Attached', kind, created_at, id"
        ))?;
        let rows = stmt
            .query_map([], queue_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

/// `packages.queue_attach`: attach a queued note to a scene the user chose
/// ("Attach to Scene 9" / "Attach manually…"). The comment keeps its author.
pub(crate) fn queue_attach(
    core: &AppCore,
    actor: &Actor,
    a: PackageQueueAttachArgs,
) -> AppResult<PackageQueueItem> {
    actor.require(Capability::Comment, "add comments in this project")?;
    let s = core.project()?;
    let (item, author_id, package_id): (PackageQueueItem, Option<String>, String) = s.store.read(|c| {
        c.query_row(
            &format!("SELECT {QUEUE_COLS}, author_user_id, package_id FROM review_queue_item WHERE id=?1 AND deleted_at IS NULL"),
            [&a.item_id],
            |r| Ok((queue_row(r)?, r.get(12)?, r.get(13)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("review note"))
    })?;
    if item.status != "Pending" {
        return Err(AppError::conflict("This note was already attached."));
    }
    let label = s.store.read(|c| {
        let row: Option<(String, String)> = c
            .query_row(
                "SELECT draft_id, heading FROM screenplay_scene WHERE id=?1 AND deleted_at IS NULL",
                [&a.scene_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (draft, heading) = row.ok_or_else(|| AppError::not_found("scene"))?;
        let n = exchange::load_scenes(c, &draft)?
            .iter()
            .position(|x| x.id == a.scene_id)
            .map(|i| i + 1)
            .unwrap_or(0);
        Ok(format!("Scene {n} · {}", heading.trim()))
    })?;
    let author = exchange_actor(
        &PackageUser {
            user_id: author_id.unwrap_or_default(),
            display_name: item.author_name.clone(),
        },
        actor,
        &package_id,
    );
    let body = match &item.quoted_text {
        Some(q) if !q.trim().is_empty() => format!("“{}” — {}", q.trim(), item.body),
        _ => item.body.clone(),
    };
    let out = core.dispatch(
        &author,
        "comment.create",
        json!({ "targetType": "screenplay_scene", "targetId": a.scene_id, "body": body }),
    )?;
    let comment_id = created_id(&out);
    s.store.mutate(
        actor,
        MutationMeta::new("packages.queue_attach", format!("Attached a review note to {label}"), Capability::Comment)
            .target("review_queue_item", &a.item_id),
        |tx| {
            tx.conn().execute(
                "UPDATE review_queue_item SET status='Attached', attached_comment_id=?1, attached_label=?2, updated_at=?3, rev=rev+1 WHERE id=?4",
                params![comment_id, label, now_ms(), a.item_id],
            )?;
            Ok(())
        },
    )?;
    s.store.read(|c| {
        Ok(c.query_row(
            &format!("SELECT {QUEUE_COLS} FROM review_queue_item WHERE id=?1"),
            [&a.item_id],
            queue_row,
        )?)
    })
}

// =========================================================== review records

pub(crate) fn records(
    core: &AppCore,
    actor: &Actor,
    _a: PackageListArgs,
) -> AppResult<Vec<PackageRecordSummary>> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        let mut stmt = c.prepare(
            "SELECT id, title, package_type, source_project_title, source_label, comment_count, exported_by, exported_at, created_at
             FROM exchange_review_record WHERE deleted_at IS NULL ORDER BY created_at DESC, id",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let ty: String = r.get(2)?;
                let ty = PackageType::parse(&ty).unwrap_or(PackageType::ScriptReview);
                Ok(PackageRecordSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    package_type: ty,
                    type_label: ty.label().to_string(),
                    source_project_title: r.get(3)?,
                    source_label: r.get(4)?,
                    comment_count: r.get::<_, i64>(5)? as u32,
                    exported_by: r.get(6)?,
                    exported_at: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

pub(crate) fn record_content(
    core: &AppCore,
    actor: &Actor,
    id: &str,
) -> AppResult<(
    PackageType,
    Value,
    String,
    String,
    Option<String>,
    Option<i64>,
)> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    s.store.read(|c| {
        c.query_row(
            "SELECT package_type, content_json, title, source_project_title, exported_by, exported_at FROM exchange_review_record WHERE id=?1 AND deleted_at IS NULL",
            [id],
            |r| {
                let ty: String = r.get(0)?;
                let content: String = r.get(1)?;
                Ok((
                    PackageType::parse(&ty).unwrap_or(PackageType::ScriptReview),
                    serde_json::from_str(&content).unwrap_or(Value::Null),
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("review record"))
    })
}

pub(crate) fn session_content(
    core: &AppCore,
    actor: &Actor,
    id: &str,
) -> AppResult<(PackageManifest, Value)> {
    actor.require(Capability::View, "view this project")?;
    let s = core.project()?;
    let row = s.store.read(|c| load_session(c, id))?;
    Ok((parse(&row.manifest_json)?, parse(&row.content_json)?))
}

pub(crate) fn record_delete(core: &AppCore, actor: &Actor, a: PackageRecordArgs) -> AppResult<()> {
    let s = core.project()?;
    let title: String = s.store.read(|c| {
        c.query_row(
            "SELECT title FROM exchange_review_record WHERE id=?1 AND deleted_at IS NULL",
            [&a.record_id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::not_found("review record"))
    })?;
    s.store.mutate(
        actor,
        MutationMeta::new(
            "packages.record_delete",
            format!("Deleted review record “{title}”"),
            Capability::SoftDelete,
        )
        .target("exchange_review_record", &a.record_id),
        |tx| {
            soft_delete(
                tx,
                DeleteSpec {
                    object_type: "exchange_review_record",
                    table: "exchange_review_record",
                    id: &a.record_id,
                    title: Some(title.clone()),
                    parent_type: None,
                    parent_id: None,
                    position: None,
                },
            )
        },
    )
}

fn purge_record(tx: &Tx<'_>, row: &DeletedItemRow) -> AppResult<()> {
    tx.conn().execute(
        "DELETE FROM exchange_review_record WHERE id=?1",
        [&row.object_id],
    )?;
    Ok(())
}

fn index_record(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, Option<String>, bool)> = c
        .query_row(
            "SELECT title, source_project_title, source_label, deleted_at IS NOT NULL FROM exchange_review_record WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(match row {
        Some((title, src, label, false)) => Some(SearchDoc {
            entity_type: "exchange_review_record".into(),
            title,
            body: format!("{src} {}", label.unwrap_or_default()),
            context: "Review".into(),
            nav: json!({ "workspace": "home", "packageRecordId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

fn index_queue_item(c: &Connection, id: &str) -> AppResult<Option<SearchDoc>> {
    let row: Option<(String, String, Option<String>, bool)> = c
        .query_row(
            "SELECT author_name, body, quoted_text, deleted_at IS NOT NULL FROM review_queue_item WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    Ok(match row {
        Some((author, body, quote, false)) => Some(SearchDoc {
            entity_type: "review_queue_item".into(),
            title: format!("Review note from {author}"),
            body: format!("{body} {}", quote.unwrap_or_default()),
            context: "Review Queue".into(),
            nav: json!({ "workspace": "home", "reviewQueueItemId": id }),
            owner_user_id: None,
        }),
        _ => None,
    })
}

pub(crate) fn register_data(r: &mut Registry) {
    r.indexer("exchange_review_record", index_record);
    r.indexer("review_queue_item", index_queue_item);
    r.trash_handler(TrashHandler {
        object_type: "exchange_review_record",
        table: "exchange_review_record",
        label: "Review record",
        restore: None,
        purge: purge_record,
    });
}
