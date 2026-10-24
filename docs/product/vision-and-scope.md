# Product vision and scope

> Rewritten 2026-10-15 from the owner's statements in chat. This is the one place that says who OpenFrame is for, what the
> chapters of the work are, and what the product will and will not claim. Detailed behaviour stays in the
> specifications ([specs/](../specs)); decisions stay in the [decision register](../adr/README.md); open owner questions
> stay in [owner-decisions.md](../implementation/owner-decisions.md). Where this page and an older document disagree on
> audience, chapters or order of work, this page wins and the older text is history. The earlier version is in
> [history](../history/2026-10-15-vision-and-scope-before-restructure.md).

## 1. What OpenFrame is

OpenFrame Studio is a desktop application for writers, screenwriters and production teams. Everyone uses the same
application. There are no separate editions for different kinds of user.

A person can go from a first idea to a shoot day inside one project folder on their own computer:

- Idea Vault: notes, images, links, files and voice notes.
- Beat Board: acts, sequences, beats and scene cards.
- Screenplay: a professional screenplay editor with drafts, comparison, review rounds, locking and revisions.
- Breakdown: tagging the elements of every scene, plus the catalog, locations, cast and crew.
- Visual planning: moodboards, storyboards and shot lists.
- Schedule: stripboard, shooting days, Day Out of Days.
- Call sheets, sides, reports and a light budget.
- Import and export: Final Draft (FDX), Fountain, Word (DOCX), PDF, plain text, backups and review packages.
- An offline AI assistant that reads the project and proposes changes that the person approves.

The work stays on the person's computer. There is no account, no cloud service and no telemetry. The one network use is
the AI download the person starts with a click.

## 2. The technology

One Rust core holds all rules, storage, search, parsers, exports and the AI safety path, over SQLite
([ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md)). Two shells use it:

- **Windows:** Tauri v2 with React and TypeScript screens.
- **macOS:** a native SwiftUI application that calls the same core through a generated interface (UniFFI).

The offline AI is run by `llama.cpp` behind a provider interface: a small default model and a larger tier for strong
computers, an embedding model chosen by measurement, graphics-card acceleration where it passes a first-run check, and on
the Mac an optional Apple on-device model ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md),
[ADR-0029](../adr/ADR-0029-gpu-acceleration.md)). English comes first and is held to a measured release bar; Hindi, Telugu,
Tamil, Kannada, Malayalam, Spanish, French, German, Chinese, Japanese and Korean are also supported and measured. The
application screens are English only.

How these fit together is explained in [architecture/README.md](../architecture/README.md) and, in full detail, in the
[retrospection and audit report](../reports/2026-10-09-project-retrospection-and-audit.md) (a snapshot of 2026-10-09).

## 3. The chapters

Each chapter has its own plan under [../roadmap/](../roadmap/README.md).

| Chapter | Goal | Plan |
|---|---|---|
| 1. Complete, test and validate | Windows track first: finish the Windows application, prove it end to end, build the internal `.exe`, team testing by hand. Then the Mac track: the SwiftUI shell over the same core, proved end to end, the internal `.dmg`, team testing by hand | [overview](../roadmap/chapter-1-complete-test-validate.md), [Windows phases](../roadmap/chapter-1a-windows-phases.md), [Mac phases](../roadmap/chapter-1b-mac-phases.md) |
| 2. Product website | A small static website on a purchased domain with company email: product information, tutorials, onboarding, support, contact, Privacy Policy and Terms, faithful recreations of the real screens | [chapter-2](../roadmap/chapter-2-website.md) |
| 3. Commercial distribution | The same application packaged for the Microsoft Store and the Mac App Store, with a 7-day free trial and paid activation, platform signing, notarization, licensing and updates | [chapter-3](../roadmap/chapter-3-commercial-distribution.md) |

The order is strict ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). Chapter 1 is the only chapter in
progress. The implementation plan for Chapter 2 is written when Chapter 1 closes, and for Chapter 3 when Chapter 2 closes.
A decision that a later chapter would otherwise make expensive to change may be taken earlier, but no later chapter's code
is written early.

## 4. What the internal builds are

The internal `.exe` and `.dmg` are for evaluation by the owner and the invited team.

- They are not signed and not notarized. Windows SmartScreen and macOS Gatekeeper warn on first run. That is an
  installation restriction, not an application defect, and the test plan says how to get past it.
- They contain the app and the AI runtime. The AI models download once, after installation, when the user clicks the
  download button ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)).
- They carry no trial, no purchase, no price and no store lock ([ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md)).
- They ask the team password on every launch ([ADR-0024](../adr/ADR-0024-internal-team-builds.md), minimum length 8 by
  [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)). The gate is a door lock for the pilot, not
  encryption.

## 5. Business model (target, not final)

The proposed model is a 7-day free trial followed by paid activation, one price per region:

| Region | Target price |
|---|---|
| India | 3,000 rupees |
| USA and the rest of the world | 89 US dollars |
| Europe | 89 euros |

Recorded from the owner's messages of 2026-10-09. An earlier message said 2,999 rupees and the later one says 3,000, so the
owner confirms the final figure. These are targets. [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) still says
price and refund rules are open, and [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md) says the app never
shows a price: the store's own purchase window shows it. Neither store's rules are assumed to allow this model. Chapter 3
checks them before anything is built.

## 6. What OpenFrame does not do, and must not claim

- No live network collaboration. Live LAN collaboration was removed ([ADR-0007](../adr/ADR-0007-lan-collaboration-removed.md)).
  The mock-ups 164 to 168 still show it. They must not be used for the website.
- No user accounts and no roles in the interface. The Rust core has a permission model (Owner, Editor, Commenter,
  Viewer, Export-only) used by packages and exports, but a person using the app is always the owner of their projects.
  Mock-up 155 (permissions) must not be advertised.
- No cloud AI, no AI that changes a project on its own. Every AI change is a proposal that a person reads and applies.
  The person does not pick a model by name; the app proposes a tier for the computer.
- No encryption of projects at rest in the internal builds (a Chapter 3 decision).
- No claim of graphics-card speed-ups except for a named card that was measured ([ADR-0029](../adr/ADR-0029-gpu-acceleration.md)).
- No claim that a non-English language is "strong" beyond what its report says. The reports for the non-English sets are
  marked "automated review" until a native speaker has checked a sample ([ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md)).
- Windows 11 and Apple silicon Macs only.

## 7. The claims rule

Nothing is advertised, on the website or in a store listing, unless the feature is Verified on the exact build that people
download, at proof level 3 or higher ([ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md)).
[feature-status.md](../implementation/feature-status.md) is the source. The same rule applies to screenshots and to the
HTML recreations of screens made for the website.

## 8. Open product questions

Listed with their owners in [owner-decisions.md](../implementation/owner-decisions.md) section 2:

1. Final prices, currencies and refund rules.
2. Decided 2026-10-16: the product description says "writers, screenwriters and production teams" (window and installer
   text changed in phase W1). The AI instructions change after the W5 measurement exists (ADR-0028).
3. The end-user licence agreement, which must pass on every shipped model's use restrictions.
4. What "build the website manually" means (Chapter 2).
