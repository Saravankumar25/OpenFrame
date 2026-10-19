//! Security roles and capabilities (Security spec §6–§7, FSD §46).
//!
//! The five roles are the entire security model. Professional labels such as
//! "Writer" or "Director" are *not* roles. Capabilities are the unit checked
//! by every application command before any mutation (ESD §6 step 2).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Role {
    Owner,
    Editor,
    Commenter,
    Viewer,
    ExportOnly,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "Owner",
            Role::Editor => "Editor",
            Role::Commenter => "Commenter",
            Role::Viewer => "Viewer",
            Role::ExportOnly => "ExportOnly",
        }
    }

    pub fn parse(value: &str) -> Option<Role> {
        Some(match value {
            "Owner" => Role::Owner,
            "Editor" => Role::Editor,
            "Commenter" => Role::Commenter,
            "Viewer" => Role::Viewer,
            "ExportOnly" | "Export-only" => Role::ExportOnly,
            _ => return None,
        })
    }

    /// Human label used in the UI ("Export-only" per the terminology lock).
    pub fn label(self) -> &'static str {
        match self {
            Role::ExportOnly => "Export-only",
            other => other.as_str(),
        }
    }
}

/// What an operation needs. Every command/query declares exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Capability {
    /// Read permitted project content (queries, search, read-only AI).
    View,
    /// Create/edit ordinary project content (vault, story, screenplay, production).
    Edit,
    /// Move content to the recoverable deleted state and restore it.
    SoftDelete,
    /// Permanently remove deleted content (deliberate second action).
    PermanentDelete,
    /// Add comments and replies.
    Comment,
    /// Resolve comments / complete review rounds.
    ResolveComments,
    /// Lock a draft as the shooting draft / finalize production documents.
    LockOrFinalize,
    /// Project settings, status, archive, production-source selection.
    ManageProject,
    /// Change collaborator roles/permissions.
    ManagePermissions,
    /// Create document exports (PDF/FDX/…).
    Export,
    /// Create exchange/review/project/backup packages.
    CreatePackage,
    /// Import content that mutates the project (always via preview/Change Set).
    Import,
    /// Accept/apply a Change Set (AI, import, review response).
    ApplyChangeSet,
    /// Ask the AI assistant read/compute/navigate questions.
    UseAi,
}

impl Role {
    /// Permission matrix. Decisions for cells the Security spec leaves open are
    /// documented in docs/adr/ADR-0009-permission-matrix.md.
    pub fn allows(self, cap: Capability) -> bool {
        use Capability as C;
        match self {
            Role::Owner => true,
            Role::Editor => matches!(
                cap,
                C::View
                    | C::Edit
                    | C::SoftDelete
                    | C::Comment
                    | C::ResolveComments
                    | C::LockOrFinalize
                    | C::Export
                    | C::CreatePackage
                    | C::Import
                    | C::ApplyChangeSet
                    | C::UseAi
            ),
            Role::Commenter => matches!(cap, C::View | C::Comment | C::ResolveComments | C::UseAi),
            Role::Viewer => matches!(cap, C::View | C::UseAi),
            Role::ExportOnly => matches!(cap, C::View | C::Export | C::CreatePackage),
        }
    }
}

/// Where a request originates. Collaboration is file-based only (exchange
/// packages); changes from another person's package are applied locally with
/// `Exchange` origin after review. (LAN collaboration was removed from scope.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ActorOrigin {
    Local,
    Exchange { package_id: String },
    Ai { request_id: String },
}

/// The effective user for a command.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub user_id: String,
    pub display_name: String,
    pub role: Role,
    pub origin: ActorOrigin,
}

impl Actor {
    pub fn local_owner(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            display_name: display_name.into(),
            role: Role::Owner,
            origin: ActorOrigin::Local,
        }
    }

    pub fn can(&self, cap: Capability) -> bool {
        self.role.allows(cap)
    }

    pub fn require(&self, cap: Capability, action: &str) -> crate::AppResult<()> {
        if self.can(cap) {
            Ok(())
        } else {
            Err(crate::AppError::permission_denied(action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_cannot_mutate() {
        for cap in [
            Capability::Edit,
            Capability::SoftDelete,
            Capability::Import,
            Capability::ApplyChangeSet,
        ] {
            assert!(!Role::Viewer.allows(cap), "{cap:?}");
        }
        assert!(Role::Viewer.allows(Capability::View));
    }

    #[test]
    fn export_only_can_export_but_not_edit() {
        assert!(Role::ExportOnly.allows(Capability::Export));
        assert!(Role::ExportOnly.allows(Capability::CreatePackage));
        assert!(!Role::ExportOnly.allows(Capability::Edit));
    }

    #[test]
    fn only_owner_manages_permissions_and_hosts() {
        for role in [
            Role::Editor,
            Role::Commenter,
            Role::Viewer,
            Role::ExportOnly,
        ] {
            assert!(!role.allows(Capability::ManagePermissions));
            assert!(!role.allows(Capability::PermanentDelete));
        }
        assert!(Role::Owner.allows(Capability::ManagePermissions));
    }

    #[test]
    fn commenter_cannot_edit_core_content() {
        assert!(Role::Commenter.allows(Capability::Comment));
        assert!(!Role::Commenter.allows(Capability::Edit));
        assert!(!Role::Commenter.allows(Capability::ApplyChangeSet));
    }

    #[test]
    fn role_round_trip() {
        for r in [
            Role::Owner,
            Role::Editor,
            Role::Commenter,
            Role::Viewer,
            Role::ExportOnly,
        ] {
            assert_eq!(Role::parse(r.as_str()), Some(r));
        }
    }
}
