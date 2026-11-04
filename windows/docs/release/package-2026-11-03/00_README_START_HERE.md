# Straft Studio: Release Package (trial, licensing, store readiness, AI re-harness)

Prepared 2026-11-03 for the owner, Saravan. Written against the real code (GitHub `main`, tree `10e5d7b`), not against ADRs or docs. Where this package and the code disagree, the code is the ground truth for what exists today and this package is the instruction for what must change. Report every disagreement you find.

## What this package is

One package, given unchanged to the Windows session and to the Mac session. It is both the context and the prompt. It tells each coding agent exactly what to build so the app can be packaged as MSIX (Microsoft Store) and as a sandboxed `.app` (Mac App Store).

The owner has not yet finished the Microsoft Partner Center or Apple Developer enrolment (both are under verification). So every value that only those programs can give is a placeholder (file 06). Build everything else now.

## Read in this order

| File | Windows session | Mac session | What it holds |
|---|---|---|---|
| `01_MASTER_PROMPT.md` | read | read | The task, the rules and the definition of done. Start here. |
| `02_TRIAL_AND_LICENSING_SPEC.md` | read | read | One behaviour for both platforms: chip, reminders, buyers, lock, wording, tests. |
| `03_WINDOWS_TASKS.md` | read | skip | Microsoft Store trial (the Store owns the clock), refresh, refund, MSIX, ARM64. |
| `04_MAC_TASKS.md` | skip | read | Fully offline 7-day trial with StoreKit 2. No licence server. Clock guard. App Store pipeline. |
| `05_UPDATE_SAFETY_AND_STANDARDS.md` | read | read | Will a Store update keep user data intact? Audit of the real code, gaps, and a standards checklist. |
| `06_PLACEHOLDERS_AND_COMPANY_DETAILS.md` | read | read | Every placeholder, where it lives, who supplies it. Company details to use (and not to use). |
| `07_CONTACT_FEEDBACK_SOCIALS.md` | read | read | In-app "Contact us" goes to the website contact page. Social handles stay placeholders. |
| `08_CHANGES_FROM_PREVIOUS_ZIP.md` | read | read | What changed against the earlier mocks package and why. |
| `ai-reharness.md` | read | read | Track B: section-scoped AI assistant. Its own gates and order inside the file. |
| `Trial_Licensing_Mocks/` | read | read | HTML mocks 182 to 203 and screenshots, plus the UI copy and logo note. Open `index.html`. |

## Two tracks

- **Track A (do first): trial, licensing, update safety, publish readiness.** Files 01 to 08 and the mocks.
- **Track B (after Track A is green): AI re-harness.** `ai-reharness.md`. The owner includes it in this package, which counts as the owner's go-ahead on 2026-11-03. Record it as an ADR (next free number, check `docs/adr/README.md`) before building, then follow its own slices R0 to R7 with its measure-first rule. The Windows app goes first; the Mac needs its own SwiftUI picker over the same Rust payload.

## Ground rules for both sessions

1. Follow the repository's own `CLAUDE.md` session protocol (read order, one work item per session, handoff file, progress log). That file says Chapter 3 (commercial distribution) was not started yet. The owner has now started it: record this as an owner decision in `docs/implementation/owner-decisions.md` and in the handoff file before you write code.
2. The source code is the ground truth. Read the code you change before changing it. Do not judge the state of the repo from documents.
3. The Rust crates (`straft-entitlement`, `straft-application`) exist in both trees (`windows/crates`, `mac/crates`). Apply the same Rust change in both trees, then run each tree's own gates. The Mac tree also has `scripts/gen-mac-parity.mjs`: run it. Do not let the two copies drift.
4. Never invent a value that only the stores can give. Use a placeholder named `REPLACE_WITH_...` (the existing Store build guard already rejects anything matching `/REPLACE_/`). A release build must fail while one remains (file 06).
5. No licence server, no device binding, no analytics, no accounts. The app stays offline except the model download the user starts and the stores' own calls.
6. Prices never appear as typed text in the app. Windows shows none. The Mac shows the price StoreKit supplies at run time (file 04, section M5), because Apple's guideline requires the charge to be disclosed before a trial starts.
7. Do not put personal details, bank, tax or company registration numbers in the repo, the app, screenshots or CI logs. Use only the company details listed in file 06.
8. Run every gate the repo defines before you call a work item done: build, tests, `check-writing-standard`, `check-migrations`, `architecture-lint`, `check-secrets`, `app-addresses`, `check-links`, `release-validate`. Fix what you broke. Report what you could not run.
9. Keep wording plain and in the repo's writing standard (no em dashes, no jargon in screens).
10. If something in this package is wrong, ambiguous or impossible, stop that item, say why in the handoff file, and carry on with the next one. Do not guess silently.

## What the owner will do himself (not your job)

Finish Microsoft Partner Center and Apple Developer enrolment; create the app records, the Mac in-app purchases, store listings, screenshots, age ratings and tax/banking forms; supply the real identity values and social handles. Everything you need from those is listed as a placeholder in file 06.
