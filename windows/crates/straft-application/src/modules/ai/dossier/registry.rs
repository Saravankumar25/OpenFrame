//! Which answers read the project. Everything else about what is read is the section tree
//! (`sections::LEAVES`): the person picks the leaves, and each leaf lists its own sources.

use crate::modules::ai::turn::Group;

/// Chat and change requests need no facts; the rest read the project.
pub fn wants_facts(group: Group) -> bool {
    !matches!(group, Group::Chat | Group::Change)
}
