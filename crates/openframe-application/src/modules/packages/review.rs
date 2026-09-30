//! The reviewer's side of a screenplay review round trip (FSD §47.5–47.6,
//! UX §3.39 "Reviewer view", mockup 160).
//!
//! The reviewer usually does not have the author's project. Opening a review
//! package creates a small review workspace under the app data folder
//! (`reviews/<packageId>/`): the validated manifest and snapshot, plus the
//! reviewer's own comments. The screenplay is read-only; comments stay
//! separate from the original project and travel back as a Response package.
//! The same viewer shows a package read-only for the author ("View the review
//! without importing") and for kept review records.

use std::fs;
use std::path::PathBuf;

use openframe_domain::{Actor, AppError, AppResult, PACKAGE_FORMAT_VERSION, new_id, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::PackageCount;
use super::backup::PackagePathArgs;
use super::exchange::{self, ExComment, ScriptContent};
use super::format::{self, EntryData, PackageManifest, PackageRef, PackageType, PackageUser};
use super::import::{PackageRecordArgs, PackageSessionArgs};
use crate::core::AppCore;

const MAX_COMMENT_BYTES: usize = 20_000;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageReviewElement {
    pub id: String,
    pub element_type: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageReviewScene {
    pub id: String,
    pub number: u32,
    pub heading: String,
    pub synopsis: Option<String>,
    pub elements: Vec<PackageReviewElement>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageReviewComment {
    pub id: String,
    pub scene_id: Option<String>,
    pub quoted_text: Option<String>,
    pub body: String,
    pub author_name: String,
    #[ts(type = "number")]
    pub created_at: i64,
    /// Written by the current reviewer in this workspace (exported in the response).
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageReviewWorkspace {
    /// Review package id (reviewer workspace), session id or record id.
    pub key: String,
    /// "review" | "session" | "record"
    pub source: String,
    pub package_type: PackageType,
    pub type_label: String,
    pub project_title: String,
    pub screenplay_title: String,
    pub draft_name: String,
    pub exported_by: String,
    #[ts(type = "number")]
    pub exported_at: i64,
    pub scenes: Vec<PackageReviewScene>,
    pub comments: Vec<PackageReviewComment>,
    pub can_comment: bool,
    #[ts(type = "number | null")]
    pub response_exported_at: Option<i64>,
    /// Non-screenplay packages: what the package contains.
    pub summary: Vec<PackageCount>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageReviewSummary {
    pub package_id: String,
    pub project_title: String,
    pub draft_name: String,
    pub exported_by: String,
    #[ts(type = "number")]
    pub exported_at: i64,
    pub my_comment_count: u32,
    #[ts(type = "number | null")]
    pub response_exported_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct PackageResponseExported {
    pub path: String,
    pub file_name: String,
    pub package_id: String,
    pub comment_count: u32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageReviewArgs {
    pub package_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageReviewCommentArgs {
    pub package_id: String,
    pub scene_id: String,
    pub body: String,
    /// Optional text from the scene the note is about.
    #[serde(default)]
    pub quoted_text: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageReviewCommentIdArgs {
    pub package_id: String,
    pub comment_id: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackageReviewExportArgs {
    pub package_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ResponseFile {
    comments: Vec<ExComment>,
    opened_at: i64,
    exported_at: Option<i64>,
}

fn reviews_root(core: &AppCore) -> PathBuf {
    core.config.app_data_dir.join("reviews")
}

fn review_dir(core: &AppCore, package_id: &str) -> AppResult<PathBuf> {
    if !openframe_domain::ids::is_valid_id(package_id) {
        return Err(AppError::not_found("review package"));
    }
    Ok(reviews_root(core).join(package_id))
}

fn read_json<T: for<'de> Deserialize<'de>>(p: &std::path::Path) -> AppResult<T> {
    let bytes = fs::read(p).map_err(|_| AppError::not_found("review package"))?;
    serde_json::from_slice(&bytes).map_err(|e| AppError::internal(e.to_string()))
}

fn write_json<T: Serialize>(p: &std::path::Path, v: &T) -> AppResult<()> {
    openframe_project_format::atomic_write(p, &format::to_json_bytes(v)?)
}

struct Review {
    manifest: PackageManifest,
    content: ScriptContent,
    response: ResponseFile,
}

fn load_review(core: &AppCore, package_id: &str) -> AppResult<Review> {
    let dir = review_dir(core, package_id)?;
    let manifest: PackageManifest = read_json(&dir.join("manifest.json"))?;
    let content: Value = read_json(&dir.join("content.json"))?;
    Ok(Review {
        manifest,
        content: exchange::parse_content(&content)?,
        response: read_json(&dir.join("response.json")).unwrap_or_default(),
    })
}

fn scenes_of(c: &ScriptContent) -> Vec<PackageReviewScene> {
    c.scenes
        .iter()
        .map(|s| PackageReviewScene {
            id: s.id.clone(),
            number: s.number,
            heading: s.heading.clone(),
            synopsis: s.synopsis.clone(),
            elements: s
                .elements
                .iter()
                .map(|e| PackageReviewElement {
                    id: e.id.clone(),
                    element_type: e.element_type.clone(),
                    text: e.text.clone(),
                })
                .collect(),
        })
        .collect()
}

fn comment_dto(c: &ExComment, mine: bool) -> PackageReviewComment {
    PackageReviewComment {
        id: c.id.clone(),
        scene_id: c.scene_id.clone(),
        quoted_text: c.quoted_text.clone(),
        body: c.body.clone(),
        author_name: c.author_name.clone(),
        created_at: c.created_at,
        mine,
    }
}

fn workspace(
    key: &str,
    source: &str,
    m: &PackageManifest,
    content: &Value,
    mine: &[ExComment],
    can_comment: bool,
    exported: Option<i64>,
) -> AppResult<PackageReviewWorkspace> {
    let ty = m.package_type;
    let (scenes, comments, screenplay_title, draft_name) =
        if matches!(ty, PackageType::ScriptReview | PackageType::Response) {
            let c: ScriptContent = exchange::parse_content(content)?;
            let mut comments: Vec<PackageReviewComment> =
                c.comments.iter().map(|x| comment_dto(x, false)).collect();
            comments.extend(mine.iter().map(|x| comment_dto(x, true)));
            (
                scenes_of(&c),
                comments,
                c.draft.screenplay_title.clone(),
                c.draft.name.clone(),
            )
        } else {
            (vec![], vec![], String::new(), String::new())
        };
    Ok(PackageReviewWorkspace {
        key: key.to_string(),
        source: source.to_string(),
        package_type: ty,
        type_label: ty.label().to_string(),
        project_title: m.source_project_title.clone(),
        screenplay_title,
        draft_name,
        exported_by: m.originating_user.display_name.clone(),
        exported_at: m.exported_at,
        scenes,
        comments,
        can_comment,
        response_exported_at: exported,
        summary: exchange::content_summary(ty, content),
    })
}

fn review_workspace(core: &AppCore, package_id: &str) -> AppResult<PackageReviewWorkspace> {
    let r = load_review(core, package_id)?;
    let content =
        serde_json::to_value(&r.content).map_err(|e| AppError::internal(e.to_string()))?;
    let can_comment = r.manifest.package_type == PackageType::ScriptReview;
    workspace(
        package_id,
        "review",
        &r.manifest,
        &content,
        &r.response.comments,
        can_comment,
        r.response.exported_at,
    )
}

/// `packages.review_open`: validate a screenplay review (or response) package
/// and open it in the review viewer. Works without any project open.
pub(crate) fn review_open(
    core: &AppCore,
    _actor: &Actor,
    a: PackagePathArgs,
) -> AppResult<PackageReviewWorkspace> {
    let path = PathBuf::from(a.path.trim());
    let opened = format::open_package(
        &path,
        &core.config.app_data_dir.join("tmp"),
        &[PackageType::ScriptReview, PackageType::Response],
        "a screenplay review package",
    )?;
    let content = opened.content()?;
    let _: ScriptContent = exchange::parse_content(&content)?;
    let dir = review_dir(core, &opened.manifest.package_id)?;
    fs::create_dir_all(&dir)?;
    write_json(&dir.join("manifest.json"), &opened.manifest)?;
    write_json(&dir.join("content.json"), &content)?;
    if !dir.join("response.json").is_file() {
        write_json(
            &dir.join("response.json"),
            &ResponseFile {
                opened_at: now_ms(),
                ..Default::default()
            },
        )?;
    }
    review_workspace(core, &opened.manifest.package_id)
}

pub(crate) fn review_list(
    core: &AppCore,
    actor: &Actor,
    _a: super::import::PackageListArgs,
) -> AppResult<Vec<PackageReviewSummary>> {
    let root = reviews_root(core);
    let mut out = vec![];
    let Ok(entries) = fs::read_dir(&root) else {
        return Ok(out);
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(r) = load_review(core, &name) else {
            continue;
        };
        out.push(PackageReviewSummary {
            package_id: name,
            project_title: r.manifest.source_project_title.clone(),
            draft_name: r.content.draft.name.clone(),
            exported_by: r.manifest.originating_user.display_name.clone(),
            exported_at: r.manifest.exported_at,
            my_comment_count: r
                .response
                .comments
                .iter()
                .filter(|c| c.author_user_id == actor.user_id)
                .count() as u32,
            response_exported_at: r.response.exported_at,
        });
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.exported_at));
    Ok(out)
}

pub(crate) fn review_get(
    core: &AppCore,
    _actor: &Actor,
    a: PackageReviewArgs,
) -> AppResult<PackageReviewWorkspace> {
    review_workspace(core, &a.package_id)
}

pub(crate) fn review_comment(
    core: &AppCore,
    actor: &Actor,
    a: PackageReviewCommentArgs,
) -> AppResult<PackageReviewWorkspace> {
    let body = a.body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::required("Comment"));
    }
    if body.len() > MAX_COMMENT_BYTES {
        return Err(AppError::invalid_input("Comment is too long."));
    }
    let mut r = load_review(core, &a.package_id)?;
    if r.manifest.package_type != PackageType::ScriptReview {
        return Err(AppError::invalid_input(
            "This package is shown read-only; comments can't be added to it.",
        ));
    }
    let scene = r
        .content
        .scenes
        .iter()
        .find(|s| s.id == a.scene_id)
        .ok_or_else(|| AppError::not_found("scene"))?;
    let quoted = a
        .quoted_text
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty());
    if let Some(q) = &quoted
        && !scene.full_text().contains(q.as_str())
    {
        return Err(AppError::validation(
            "quotedText",
            "The quoted text isn't part of this scene.",
        ));
    }
    r.response.comments.push(ExComment {
        id: new_id(),
        parent_id: None,
        target_type: "screenplay_scene".into(),
        target_id: scene.id.clone(),
        scene_id: Some(scene.id.clone()),
        scene_lineage_id: Some(scene.lineage_id.clone()),
        quoted_text: quoted,
        body,
        status: "Open".into(),
        author_user_id: actor.user_id.clone(),
        author_name: actor.display_name.clone(),
        created_at: now_ms(),
    });
    write_json(
        &review_dir(core, &a.package_id)?.join("response.json"),
        &r.response,
    )?;
    review_workspace(core, &a.package_id)
}

pub(crate) fn review_delete_comment(
    core: &AppCore,
    actor: &Actor,
    a: PackageReviewCommentIdArgs,
) -> AppResult<PackageReviewWorkspace> {
    let mut r = load_review(core, &a.package_id)?;
    let before = r.response.comments.len();
    r.response
        .comments
        .retain(|c| !(c.id == a.comment_id && c.author_user_id == actor.user_id));
    if r.response.comments.len() == before {
        return Err(AppError::not_found("comment"));
    }
    write_json(
        &review_dir(core, &a.package_id)?.join("response.json"),
        &r.response,
    )?;
    review_workspace(core, &a.package_id)
}

/// `packages.review_export_response`: the reviewer's comments as a Response
/// package (FSD §47.6). Carries the review's source identities and base
/// snapshot so the author's import can detect staleness and map scenes.
pub(crate) fn review_export_response(
    core: &AppCore,
    actor: &Actor,
    a: PackageReviewExportArgs,
) -> AppResult<PackageResponseExported> {
    let mut r = load_review(core, &a.package_id)?;
    if r.manifest.package_type != PackageType::ScriptReview {
        return Err(AppError::invalid_input(
            "Only a review package can be answered with a response package.",
        ));
    }
    let mine: Vec<ExComment> = r
        .response
        .comments
        .iter()
        .filter(|c| c.author_user_id == actor.user_id)
        .cloned()
        .collect();
    if mine.is_empty() {
        return Err(AppError::validation(
            "comments",
            "Add at least one comment before exporting a response.",
        ));
    }
    let dest = format::package_destination(&a.path, PackageType::Response)?;
    let content = ScriptContent {
        draft: r.content.draft.clone(),
        scenes: r.content.scenes.clone(),
        comments: mine.clone(),
    };
    let manifest = PackageManifest {
        format: format::FORMAT_NAME.into(),
        package_type: PackageType::Response,
        format_version: PACKAGE_FORMAT_VERSION,
        package_id: new_id(),
        source_project_id: r.manifest.source_project_id.clone(),
        source_project_title: r.manifest.source_project_title.clone(),
        exported_at: now_ms(),
        app_version: core.config.app_version.clone(),
        schema_version: None,
        source_draft: r.manifest.source_draft.clone(),
        source_versions: r.manifest.source_versions.clone(),
        included_object_ids: r.manifest.included_object_ids.clone(),
        scope: r.manifest.scope.clone(),
        comments_included: true,
        attachments_included: false,
        private_notes_included: false,
        base_snapshot: r.manifest.base_snapshot.clone(),
        originating_user: PackageUser {
            user_id: actor.user_id.clone(),
            display_name: actor.display_name.clone(),
        },
        label: None,
        responds_to: Some(PackageRef {
            package_id: r.manifest.package_id.clone(),
            package_type: r.manifest.package_type,
        }),
    };
    format::write_package(
        &dest,
        &manifest,
        vec![(
            format::CONTENT_ENTRY.to_string(),
            EntryData::Bytes(format::to_json_bytes(&content)?),
        )],
        &|_, _| {},
        &|| false,
    )?;
    r.response.exported_at = Some(now_ms());
    write_json(
        &review_dir(core, &a.package_id)?.join("response.json"),
        &r.response,
    )?;
    Ok(PackageResponseExported {
        path: dest.to_string_lossy().into_owned(),
        file_name: dest
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        package_id: manifest.package_id,
        comment_count: mine.len() as u32,
    })
}

/// "View the review without importing" (stale package dialog).
pub(crate) fn session_view(
    core: &AppCore,
    actor: &Actor,
    a: PackageSessionArgs,
) -> AppResult<PackageReviewWorkspace> {
    let (m, content) = super::import::session_content(core, actor, &a.session_id)?;
    workspace(&a.session_id, "session", &m, &content, &[], false, None)
}

/// A kept review record, shown read-only.
pub(crate) fn record_view(
    core: &AppCore,
    actor: &Actor,
    a: PackageRecordArgs,
) -> AppResult<PackageReviewWorkspace> {
    let (ty, content, title, source_title, exported_by, exported_at) =
        super::import::record_content(core, actor, &a.record_id)?;
    let m = PackageManifest {
        format: format::FORMAT_NAME.into(),
        package_type: ty,
        format_version: PACKAGE_FORMAT_VERSION,
        package_id: a.record_id.clone(),
        source_project_id: String::new(),
        source_project_title: source_title,
        exported_at: exported_at.unwrap_or(0),
        app_version: String::new(),
        schema_version: None,
        source_draft: None,
        source_versions: vec![],
        included_object_ids: vec![],
        scope: format::PackageScope {
            kind: "record".into(),
            label: title,
            ids: vec![],
        },
        comments_included: true,
        attachments_included: false,
        private_notes_included: false,
        base_snapshot: Default::default(),
        originating_user: PackageUser {
            user_id: String::new(),
            display_name: exported_by.unwrap_or_default(),
        },
        label: None,
        responds_to: None,
    };
    workspace(&a.record_id, "record", &m, &content, &[], false, None)
}
