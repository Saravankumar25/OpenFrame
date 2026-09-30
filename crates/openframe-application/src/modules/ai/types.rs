//! AI DTOs shared by the orchestrator, tools, change sets and the UI.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

/// Operation classes (AI spec §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiOperationClass")]
pub enum OperationClass {
    Read,
    Compute,
    Navigate,
    Suggest,
    Mutate,
}

impl OperationClass {
    pub fn as_str(self) -> &'static str {
        match self {
            OperationClass::Read => "Read",
            OperationClass::Compute => "Compute",
            OperationClass::Navigate => "Navigate",
            OperationClass::Suggest => "Suggest",
            OperationClass::Mutate => "Mutate",
        }
    }
}

/// What kind of reply this is — the UI renders each distinctly (AI spec §29, §41.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AiResultKind {
    Answer,
    Navigate,
    Suggestion,
    Proposal,
    Denied,
    Private,
    Clarify,
    Unavailable,
    Failed,
}

impl AiResultKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AiResultKind::Answer => "Answer",
            AiResultKind::Navigate => "Navigate",
            AiResultKind::Suggestion => "Suggestion",
            AiResultKind::Proposal => "Proposal",
            AiResultKind::Denied => "Denied",
            AiResultKind::Private => "Private",
            AiResultKind::Clarify => "Clarify",
            AiResultKind::Unavailable => "Unavailable",
            AiResultKind::Failed => "Failed",
        }
    }
    pub fn parse(s: &str) -> AiResultKind {
        match s {
            "Answer" => AiResultKind::Answer,
            "Navigate" => AiResultKind::Navigate,
            "Suggestion" => AiResultKind::Suggestion,
            "Proposal" => AiResultKind::Proposal,
            "Denied" => AiResultKind::Denied,
            "Private" => AiResultKind::Private,
            "Clarify" => AiResultKind::Clarify,
            "Unavailable" => AiResultKind::Unavailable,
            _ => AiResultKind::Failed,
        }
    }
}

/// AI Result status (FSD §42.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum AiResultStatus {
    Informational,
    #[serde(rename = "Pending Approval")]
    PendingApproval,
    Accepted,
    Rejected,
    Applied,
    Stale,
    Conflict,
    Failed,
}

impl AiResultStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            AiResultStatus::Informational => "Informational",
            AiResultStatus::PendingApproval => "Pending Approval",
            AiResultStatus::Accepted => "Accepted",
            AiResultStatus::Rejected => "Rejected",
            AiResultStatus::Applied => "Applied",
            AiResultStatus::Stale => "Stale",
            AiResultStatus::Conflict => "Conflict",
            AiResultStatus::Failed => "Failed",
        }
    }
    pub fn parse(s: &str) -> AiResultStatus {
        match s {
            "Pending Approval" => AiResultStatus::PendingApproval,
            "Accepted" => AiResultStatus::Accepted,
            "Rejected" => AiResultStatus::Rejected,
            "Applied" => AiResultStatus::Applied,
            "Stale" => AiResultStatus::Stale,
            "Conflict" => AiResultStatus::Conflict,
            "Failed" => AiResultStatus::Failed,
            _ => AiResultStatus::Informational,
        }
    }
}

/// Exact facts come from deterministic queries; never a numeric pseudo-certainty (AI spec §50.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiConfidence")]
pub enum Confidence {
    Exact,
    Inferred,
    Unavailable,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Exact => "Exact",
            Confidence::Inferred => "Inferred",
            Confidence::Unavailable => "Unavailable",
        }
    }
    pub fn parse(s: &str) -> Option<Confidence> {
        match s {
            "Exact" => Some(Confidence::Exact),
            "Inferred" => Some(Confidence::Inferred),
            "Unavailable" => Some(Confidence::Unavailable),
            _ => None,
        }
    }
}

/// Where an answer came from (AI spec §30).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiProvenance")]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    /// e.g. "Draft", "Basis", "Product guide".
    pub kind: String,
    pub label: String,
}

impl Provenance {
    pub fn new(kind: &str, label: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            label: label.into(),
        }
    }
}

/// A UI navigation target (mirrors the shell's Route).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiNavTarget")]
#[serde(rename_all = "camelCase")]
pub struct NavTarget {
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
}

impl NavTarget {
    pub fn to(workspace: &str) -> Self {
        Self {
            workspace: workspace.to_string(),
            sub: None,
            params: BTreeMap::new(),
        }
    }
    pub fn with(mut self, k: &str, v: &str) -> Self {
        self.params.insert(k.to_string(), v.to_string());
        self
    }
    pub fn with_sub(mut self, s: &str) -> Self {
        self.sub = Some(s.to_string());
        self
    }
}

/// A listed object in an answer (e.g. a scene), optionally navigable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiResultItem")]
#[serde(rename_all = "camelCase")]
pub struct ResultItem {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nav: Option<NavTarget>,
}

/// A referenced project object (table + id + human label).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiObjRef")]
#[serde(rename_all = "camelCase")]
pub struct ObjRef {
    pub table: String,
    pub id: String,
    pub label: String,
}

/// One row of a Change Set preview card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[ts(rename = "AiPreviewRow")]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    pub label: String,
    pub value: String,
    /// "normal" | "excluded" | "locked"
    pub tone: String,
}

impl PreviewRow {
    pub fn normal(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone: "normal".into(),
        }
    }
    pub fn excluded(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone: "excluded".into(),
        }
    }
    pub fn locked(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone: "locked".into(),
        }
    }
}

/// A registry operation call inside a Change Set. Operations are always built
/// by OpenFrame code from validated tool arguments — never taken verbatim from
/// model output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpCall {
    pub op: String,
    pub args: Value,
    pub label: String,
}

/// A prepared, not-yet-persisted Change Set.
#[derive(Debug, Clone)]
pub struct ChangeSetDraft {
    pub title: String,
    pub summary: String,
    pub operations: Vec<OpCall>,
    pub preview: Vec<PreviewRow>,
    pub exclusions: Vec<PreviewRow>,
    pub targets: Vec<ObjRef>,
    pub modules: Vec<String>,
    /// Rows whose revision forms the base version (table, id).
    pub base_rows: Vec<(String, String)>,
    /// Tool + validated arguments, so "Re-check & Review" can rebuild the proposal.
    pub source_tool: String,
    pub source_args: Value,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[ts(rename = "AiChangeSetDto")]
#[serde(rename_all = "camelCase")]
pub struct ChangeSetDto {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Pending | Accepted | Rejected | Applied | Stale | Conflict | Failed
    pub state: String,
    /// Not Checked | Valid | Invalid | Needs Review
    pub validation_state: String,
    pub rows: Vec<PreviewRow>,
    pub exclusions: Vec<PreviewRow>,
    pub affected_modules: Vec<String>,
    pub targets: Vec<ObjRef>,
    pub operation_count: u32,
    pub stale_reason: Option<String>,
    pub error_message: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub approved_at: Option<i64>,
    #[ts(type = "number | null")]
    pub applied_at: Option<i64>,
    pub applied_operations: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiResultDto {
    pub id: String,
    pub kind: AiResultKind,
    pub status: AiResultStatus,
    pub content: String,
    pub details: Vec<String>,
    pub confidence: Option<Confidence>,
    pub provenance: Vec<Provenance>,
    pub items: Vec<ResultItem>,
    pub nav: Option<NavTarget>,
    pub change_set: Option<ChangeSetDto>,
    pub error_code: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// One user request and the assistant's reply.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct AiExchange {
    pub request_id: String,
    pub conversation_id: String,
    pub request_text: String,
    pub scope_label: String,
    pub operation_class: Option<OperationClass>,
    pub model_reference: Option<String>,
    pub result: AiResultDto,
    #[ts(type = "number")]
    pub created_at: i64,
}
