//! Project Notes, Tasks (FSD §106, §158, §159) and Templates (FSD §59, §113; Domain §20).
//!
//! * Project notes: small free-form notes that don't belong in the Idea Vault or
//!   another workspace. Pinned notes list first. Recoverable delete.
//! * Tasks: title, optional due date / owner / related object, Open ⇄ Done.
//!   Completing a task never changes the related object. Not a PM system.
//! * Templates: built-in (read-only), global (application database, shared by
//!   every project) and project templates. Using a template copies it — later
//!   template edits never change what was created from it.

use crate::registry::Registry;

pub mod project_notes;
pub mod tasks;
pub mod templates;

pub fn register(r: &mut Registry) {
    project_notes::register(r);
    tasks::register(r);
    templates::register(r);
}
