# Chapter 1: complete, test and validate OpenFrame on Windows and on the Mac

> Rewritten 2026-10-15 after the owner's 25 decisions ([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) to
> [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The earlier Chapter 1 plan (seven streams) is kept in
> [history](../history/2026-10-15-chapter-1-plan-before-restructure.md) and is history. This page holds the exit
> criteria and the rules that hold across both tracks. The work itself is in the two phase plans.

## 1. The two tracks

| Track | Plan | Starts when | Result |
|---|---|---|---|
| Windows, ten phases W1 to W10 | [chapter-1a-windows-phases.md](chapter-1a-windows-phases.md) | Now | The Windows application, proved end to end, built as the internal `.exe`, and tested by the team |
| Mac, ten phases M1 to M10 | [chapter-1b-mac-phases.md](chapter-1b-mac-phases.md) | W10 has closed | The SwiftUI Mac application over the same Rust core, proved end to end, built as the internal `.dmg`, and tested by the team |

The tracks are sequential. The Windows core is frozen at W10 and the Mac shell is built on it. Chapter 1 closes when both
tracks have closed. No Chapter 2 or Chapter 3 code is written in Chapter 1. The implementation plans for Chapters 2 and 3
are written after Chapter 1 closes ([Chapter 2](chapter-2-website.md), [Chapter 3](chapter-3-commercial-distribution.md)).

## 2. Exit criteria

Chapter 1 is finished when all of these are true, each with evidence:

1. Every Must-ship feature is Reachable in the real application and proved at level 3 or higher on the release build of each
   platform ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) five levels).
2. No open defect that loses data, corrupts a project, fails a migration or recovery, lets the AI change a project on its
   own, or exposes private notes.
3. The internal Windows `.exe` installs, runs, upgrades and uninstalls on a clean Windows 11 computer and on a borrowed
   laptop, and the AI works with the network switched off after its one download.
4. The internal macOS `.dmg` does the same on a clean Apple silicon Mac.
5. The AI meets the English release bar (90 percent correct tool and arguments, 85 percent correct end state) on both
   platforms, and every other listed language has a measured report ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)).
6. The team has used each build as its main writing tool for 2 to 3 days, every problem has a written bug report, and every
   important one has a fix that was checked again.
7. The documents say only what is true, and no two current documents give different rules ([documentation map](../README.md)).

## 3. Rules that hold in both tracks

- One Rust core. The Windows shell is Tauri and React; the Mac shell is SwiftUI. Neither shell holds rules.
- A feature row claims only the proof level it has evidence for. Nothing is advertised that is not Verified on the exact
  build people download.
- The evaluation harness comes before any AI change; a change stays only if a number improves.
- Installers and release executables are built only when the owner says so in that conversation. Nothing is pushed without
  the owner's word. Work is committed locally on a branch and merged to `main` only after its checks pass.
- Internal builds are unsigned, carry no trial or licence, ask the team password, and need the AI download after install.
- Owner decisions are recorded in [owner-decisions.md](../implementation/owner-decisions.md) before they are built.

## 4. Hardware the owner provides

| Need | Used for |
|---|---|
| This Windows 11 Home laptop (Intel Core Ultra 5, integrated graphics, 15 GB) | Development, the processor baseline, the integrated-graphics probe test |
| A virtual machine on this laptop | Clean-machine install ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) decision 3) |
| Borrowed laptops with different graphics cards | Clean install and the graphics-card benchmarks ([ADR-0029](../adr/ADR-0029-gpu-acceleration.md)) |
| An 8 GB Windows laptop | The slow-hardware baseline and the performance budgets |
| The Intel MacBook | Mac development and everything that is not specific to Apple silicon |
| An Apple silicon Mac (soon) | Metal, Apple's model, the `.dmg`, and the second proof level for every Mac row |

## 5. Owner tasks across the chapter

Listed with their lead times in [owner-decisions.md](../implementation/owner-decisions.md) section 3. The ones that block
phases: the macOS version of the Intel MacBook (M1); approval of the wording change and the worktree review (W1); approval of
the gold sets and the model choice (W5); the licence check for the chosen models (W8); the
word to build (W4, W9, W10, M9, M10); the borrowed laptops (W7, W9); the team for both pilots (W10, M10).

## 6. What moves to later chapters

- **Chapter 2:** the website, its domain and mailboxes, support pages, the Privacy Policy and Terms pages.
- **Chapter 3:** store packaging, accounts, payment and trial, the licence server, signing and notarization, the updater,
  the end-user licence agreement, counsel review, "Report AI output", stronger protection for the team gate and encryption of
  projects at rest, and the first 90 days after launch. The 83 build plan boxes tagged "Later: store publishing" belong here.
