//! Embedded migrations. Each module owns exactly one project migration file;
//! the numbering is fixed here so modules can be developed independently.
//! Rule (Release/Migration spec): once a version ships, its migration file is
//! frozen — schema changes are made by appending a new migration.

use openframe_persistence::migrate::Migration;

macro_rules! m {
    ($v:expr, $name:expr, $path:expr) => {
        Migration {
            version: $v,
            name: $name,
            sql: include_str!($path),
        }
    };
}

pub const PROJECT_MIGRATIONS: &[Migration] = &[
    m!(1, "core", "../../../migrations/project/0001_core.sql"),
    m!(
        2,
        "idea_vault",
        "../../../migrations/project/0002_idea_vault.sql"
    ),
    m!(3, "story", "../../../migrations/project/0003_story.sql"),
    m!(
        4,
        "screenplay",
        "../../../migrations/project/0004_screenplay.sql"
    ),
    m!(
        5,
        "production",
        "../../../migrations/project/0005_production.sql"
    ),
    m!(6, "visual", "../../../migrations/project/0006_visual.sql"),
    m!(
        7,
        "schedule",
        "../../../migrations/project/0007_schedule.sql"
    ),
    m!(8, "ai", "../../../migrations/project/0008_ai.sql"),
    m!(
        9,
        "collaboration",
        "../../../migrations/project/0009_collaboration.sql"
    ),
    m!(
        10,
        "performance",
        "../../../migrations/project/0010_performance.sql"
    ),
];

pub const GLOBAL_MIGRATIONS: &[Migration] = &[
    m!(1, "core", "../../../migrations/global/0001_core.sql"),
    m!(
        2,
        "idea_vault",
        "../../../migrations/global/0002_idea_vault.sql"
    ),
    m!(
        3,
        "performance",
        "../../../migrations/global/0003_performance.sql"
    ),
];

pub const APP_MIGRATIONS: &[Migration] = &[m!(1, "app", "../../../migrations/app/0001_app.sql")];

pub fn project_schema_version() -> u32 {
    PROJECT_MIGRATIONS.len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_migration_sets_apply_cleanly_and_are_undo_trackable() {
        for (set, tracked) in [
            (PROJECT_MIGRATIONS, true),
            (GLOBAL_MIGRATIONS, true),
            (APP_MIGRATIONS, false),
        ] {
            let mut c = openframe_persistence::open_in_memory().unwrap();
            openframe_persistence::migrate::apply(&mut c, set).unwrap();
            if tracked {
                openframe_persistence::undo::install_capture(&c)
                    .expect("every canonical table is undo-trackable");
            }
        }
    }
}
