# Roadmap: three chapters

> Rewritten 2026-10-15 (owner decisions in [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) to
> [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The roadmap has three chapters. Chapter 1 has two
> tracks, one for Windows and one for the Mac, of ten phases each. The 26-area [build plan](../implementation/masterplan.md)
> keeps every checklist box and its evidence; this page maps its areas onto the chapters. The earlier roadmap map is in
> [the archive](../archive/2026-10-15-roadmap-map-before-restructure.md).

## 1. The chapters

| Chapter | Plan | State on 2026-10-15 |
|---|---|---|
| 1. Complete, test and validate | [overview and exit criteria](chapter-1-complete-test-validate.md) | In progress |
| 1, Windows track (W1 to W10) | [chapter-1a-windows-phases.md](chapter-1a-windows-phases.md) | Phase W1 is next. No installer has been built |
| 1, Mac track (M1 to M10) | [chapter-1b-mac-phases.md](chapter-1b-mac-phases.md) | Starts when W10 has closed. A native SwiftUI shell over the same Rust core |
| 2. Product website | [chapter-2-website.md](chapter-2-website.md) | Not started. Its implementation plan is written when Chapter 1 closes |
| 3. Commercial distribution | [chapter-3-commercial-distribution.md](chapter-3-commercial-distribution.md) | Not started. Its implementation plan is written when Chapter 2 closes |

The order is strict: Windows application, internal `.exe`, team testing; then the Mac application, internal `.dmg`, team
testing; then the website; then the store packages. No later chapter's code is written early.

Related: [testing strategy](../testing/README.md), [internal test plan](../testing/internal-test-plan.md),
[development storage policy](../development-storage-policy.md), [decision register](../adr/README.md).

## 2. Which build plan area belongs to which phase

The progress numbers come from `npm run plan:status`; this table never repeats them.

| Area of the build plan | Chapter and phase | Note |
|---|---|---|
| 1. Decisions and owner tasks | 1 (W1), some rows in 3 | Store accounts, counsel and signing rows are Chapter 3 |
| 2. Project controls and baseline | W1 | |
| 3. Data safety and crash recovery | W2 | Close flush, cloud-synced folders, delete and locate tests |
| 4. Runtime resilience and failure testing | W2, W3 | |
| 5. Test infrastructure and quality gates | W1, W4 | Release-build suite, shared scenario list |
| 6. Security hardening | W3 | Fuzzing, property tests, supply chain |
| 7. Platform adapter layer | W1 for Windows; M1 to M3 for the Mac | The Mac adapter first runs in M3 |
| 8. AI assistant quality and hardening | W5, W6, W7, W8; M8 on the Mac | Evaluation, harness, GPU, download flow |
| 9. Beat Board and Idea Vault | W4; M5 | |
| 10. Writing comfort | W4; M4 | |
| 11. Screenplay engine and file formats | W4; M4, M7 | |
| 12. Appearance and themes | W4; M3 | |
| 13. Breakdown, catalog, locations, cast and crew | W4; M5 | |
| 14. Shooting schedule and Day Out of Days | W4; M6 | |
| 15. Call sheets, budget, templates, samples, exports | W4; M6, M7 | |
| 16. Packages and backup | W2, W4; M7 | |
| 17. Product-wide quality, accessibility and pilot | W4, W10; M9, M10 | The long beta (8 to 15 people, 3 to 4 weeks) is Chapter 3. The 2 to 3 day team pilot is Chapter 1 |
| 18. Payment and free trial in all channels | 3 | The internal builds have no trial |
| 19. Microsoft Store package | 3 | |
| 20. Installers | W9 for the `.exe`; M9 for the `.dmg`; signing and the updater in 3 | |
| 21. Mac App Store package | 3 | |
| 22. Legal, privacy, website and support | 2 for the website and support pages; 3 for the licence agreement and counsel review | |
| 23. Final review and submission | 3 | |
| 24. Certification, publication and launch | 3 | |
| 25. First 90 days after launch | 3 | |
| 26. Conditions that stop a release | 3 | |

## 3. Rules that hold for all chapters

1. One Rust core, two shells (Tauri on Windows, SwiftUI on the Mac). Platform differences live behind the platform boundary
   and the entitlement adapters.
2. Nothing is advertised that is not Verified on the exact build people download ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) levels).
3. A decision that is the owner's is recorded in [owner-decisions.md](../implementation/owner-decisions.md) before it is built.
4. Small, independently provable work items. Each states its objective, where it lives, the behaviour, how it is verified
   and when it is accepted.
5. Work is committed locally on a branch and merged to `main` only after its checks pass. Nothing is pushed, and no
   installer or release executable is built, without the owner's word in that conversation.
