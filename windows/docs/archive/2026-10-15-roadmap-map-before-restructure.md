# Roadmap: three chapters

> Written 2026-10-09. The roadmap has three chapters (owner decision of 2026-10-09, see
> [vision-and-scope.md](../product/vision-and-scope.md)). This page maps the existing 26-area
> [build plan](../implementation/masterplan.md) onto them, so the old checklist and the new plan agree. The build plan keeps
> every box and its evidence; the chapter pages say what to do next.

## 1. The three chapters

| Chapter | Plan | State on 2026-10-09 |
|---|---|---|
| 1. Complete, test and validate | [chapter-1-complete-test-validate.md](chapter-1-complete-test-validate.md) | In progress. Features are largely built and proved on the development machine. No installer has been built. The Mac build does not exist yet. |
| 2. Product website | [chapter-2-website.md](chapter-2-website.md) | Not started. Waits for stable screens. A plan and rules exist. |
| 3. Commercial distribution | [chapter-3-commercial-distribution.md](chapter-3-commercial-distribution.md) | Not started. Some code exists (entitlement crate, Mac licence server, store lock) and is unverified against real stores. |

Related: [testing strategy](../testing/README.md), [internal test plan](../testing/internal-test-plan.md),
[development storage policy](../development-storage-policy.md).

## 2. Which build plan area belongs to which chapter

The progress numbers come from `npm run plan:status`; this table never repeats them.

| Area of the build plan | Chapter | Note |
|---|---|---|
| 1. Decisions and owner tasks | 1, with some rows in 3 | Store accounts, counsel and signing rows are Chapter 3 |
| 2. Project controls and baseline | 1 | |
| 3. Data safety and crash recovery | 1 | Gains three boxes on 2026-10-09 (typed text on close, cloud-synced folders, delete and locate tests) |
| 4. Runtime resilience and failure testing | 1 | |
| 5. Test infrastructure and quality gates | 1 | Includes "one test binary per crate" |
| 6. Security hardening from the baseline audit | 1 | |
| 7. Platform adapter layer for Windows and Mac | 1 | The Mac half waits for the Mac bring-up |
| 8. AI assistant quality and hardening | 1 | Store-specific validation boxes are Chapter 3 |
| 9. Story Board and Idea Vault | 1 | |
| 10. Writing comfort | 1 | |
| 11. Screenplay engine and file formats | 1 | |
| 12. Appearance and themes | 1 | |
| 13. Breakdown, catalog, locations, cast and crew | 1 | |
| 14. Shooting schedule and Day Out of Days | 1 | |
| 15. Call sheets, budget, templates, samples and exports | 1 | |
| 16. Packages and backup | 1 | |
| 17. Product-wide quality, accessibility and beta | 1 | The long beta (8 to 15 people, 3 to 4 weeks) is Chapter 3. The 2 to 3 day tester pilot is Chapter 1 |
| 18. Payment and free trial in all three channels | 3 | The internal build has no trial; the shared interface and store lock are built and unverified |
| 19. Microsoft Store package | 3 | |
| 20. Windows installer | 1 for the internal `.exe` and the internal `.dmg`; 3 for signing and the updater | |
| 21. Mac App Store package | 3 | |
| 22. Legal, privacy, website and support | 2 for the website and support pages; 3 for the licence agreement and counsel review | |
| 23. Final review and submission | 3 | |
| 24. Certification, publication and launch | 3 | |
| 25. First 90 days after launch | 3 | |
| 26. Conditions that stop a release | 3 | |

## 3. Rules that hold for all chapters

1. One shared application. Windows and Mac differences live behind the platform crate and the entitlement adapters. No
   second code base.
2. Nothing is advertised that is not Verified.
3. A decision that is the owner's is recorded in [owner-decisions.md](../implementation/owner-decisions.md) before it is
   built.
4. Small, independently provable work items. Each states its objective, where it lives, the behaviour, how it is verified
   and when it is accepted.
5. Work is committed locally on a branch and merged to `main` only after its tests pass. Nothing is pushed, and no
   installer is built, without the owner's word in that conversation.
