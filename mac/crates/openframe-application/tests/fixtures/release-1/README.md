# Release 1 fixture

What a person's data looks like at the first shipped release. These bytes never change. Each
release adds one new folder beside this one (`release-2`, `release-3`, ...), made by that release's
build, and keeps the older ones. The test `update_simulation_opens_release_1_project` opens a copy
of this project with the build under test, which is what an update does to a real person's work.

## What is here

- `project/`: a project folder as the app writes it. `openframe.json` and `project.sqlite`. Records:
  a two-scene screenplay (Harbour Light, with action and dialogue), two story characters (Meera,
  Karthik), one budget line (Cast, Lead actor, 5,000,000 planned, 5,500,000 actual), one note.
- `appdata/entitlement/notices.json`: notice flags with every flag set (welcome shown, data note
  acknowledged, tutorial completed, a reminder day).

Left out on purpose: the write-ahead log and shared-memory files (the database was folded into one
file before copying), the lock file, the recovery marker and checkpoint, caches, and safety copies.
The database alone is about 1.1 MB, which is the size of an empty project's schema.

## How it was made

`generate_release_1_fixture` in `crates/openframe-application/tests/platform/update_simulation.rs`
drives the app's own operations (project create, screenplay import, story, budget, notes), closes
the project so the log is folded in, and copies the folder. It is ignored in normal runs. It was run
once, by hand, on 2026-11-03:

    cargo test -p openframe-application --test platform generate_release_1_fixture -- --ignored

Do not run it again for release 1. For a new release, copy the generator to a new function that
writes `release-N`, run it on that release's build, and commit the result.
