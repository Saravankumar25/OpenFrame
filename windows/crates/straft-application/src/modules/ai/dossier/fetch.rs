//! The fetchers: the functions the leaves of `sections::LEAVES` run. Each file reads one area of the
//! project; a leaf entry point (`*_leaf`) treats its leaf as the topic of the question.

pub mod breakdown;
pub mod budget;
pub mod dayviews;
pub mod files;
pub mod notes;
pub mod overview;
pub mod people;
pub mod product;
pub mod production;
pub mod reports;
pub mod schedule;
pub mod screenplay;
pub mod settings;
pub mod story;
pub mod timeline;
pub mod vault;
pub mod visual;
