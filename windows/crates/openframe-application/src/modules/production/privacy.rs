//! Contact data privacy (Production depth contract section 3; Security spec section 6).
//!
//! Contact fields are personal data: `catalog_item.contact`, `location.contact`,
//! `location.emergency_contact`, `cast_member.contact` and `crew_member.contact`.
//!
//! - Only the roles that run the production (Owner and Editor) see them in the app.
//!   Commenter, Viewer and Export-only collaborators get the field hidden (`contactHidden`).
//! - The assistant's catalog and location reads leave them out, and AI documents never contain
//!   them. The `cast_and_crew` read shows a cast or crew contact only to an actor who may see it
//!   in the app (S2 of the conversational harness); a request with AI origin is never allowed.
//! - They are not put into the project search index, so a search result never shows them.
//! - Production exports (breakdown report, catalog, reports) never include them.

use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, Role};

/// Whether this actor may read contact fields.
pub fn may_see_contacts(actor: &Actor) -> bool {
    matches!(actor.role, Role::Owner | Role::Editor)
        && !matches!(actor.origin, ActorOrigin::Ai { .. })
}

/// Clear a contact value the actor may not see; returns whether something was hidden.
pub(crate) fn redact(field: &mut Option<String>) -> bool {
    field.take().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(role: Role, origin: ActorOrigin) -> Actor {
        Actor {
            user_id: "u".into(),
            display_name: "U".into(),
            role,
            origin,
        }
    }

    #[test]
    fn only_owner_and_editor_see_contacts_and_never_the_assistant() {
        for (role, ok) in [
            (Role::Owner, true),
            (Role::Editor, true),
            (Role::Commenter, false),
            (Role::Viewer, false),
            (Role::ExportOnly, false),
        ] {
            assert_eq!(
                may_see_contacts(&actor(role, ActorOrigin::Local)),
                ok,
                "{role:?}"
            );
        }
        let ai = ActorOrigin::Ai {
            request_id: "r".into(),
        };
        assert!(!may_see_contacts(&actor(Role::Owner, ai)));
    }
}
