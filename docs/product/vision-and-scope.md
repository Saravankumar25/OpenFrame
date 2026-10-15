# Product vision and scope

> Written 2026-10-09 from the owner's statements in chat. This is the one place that says who OpenFrame is for, what the
> three chapters of the work are, and what the product will and will not claim. Detailed behaviour stays in the
> specifications ([specs/](../specs)); decisions stay in the [decision register](../adr/README.md); open owner questions
> stay in [owner-decisions.md](../implementation/owner-decisions.md). Where this page and an older document disagree on
> audience, chapters or order of work, this page wins and the older text is history.

## 1. What OpenFrame is

OpenFrame Studio is a desktop application for writers, screenwriters and production teams. Everyone uses the same
application. There are no separate editions for different kinds of user.

A person can go from a first idea to a shoot day inside one project folder on their own computer:

- Idea Vault: notes, images, links, files and voice notes.
- Story Board: acts, sequences, beats and scene cards.
- Screenplay: a professional screenplay editor with drafts, comparison, review rounds, locking and revisions.
- Breakdown: tagging the elements of every scene, plus the catalog, locations, cast and crew.
- Visual planning: moodboards, storyboards and shot lists.
- Schedule: stripboard, shooting days, Day Out of Days.
- Call sheets, sides, reports and a light budget.
- Import and export: Final Draft (FDX), Fountain, Word (DOCX), PDF, plain text, backups and review packages.
- An offline AI assistant that reads the project and proposes changes that the person approves.

The work stays on the person's computer. There is no account, no cloud service and no telemetry.

## 2. The technology

React and TypeScript for the screens, Tauri v2 as the desktop shell, Rust for all logic and file handling, SQLite for the
data, and one bundled local AI model (Google Gemma 3 1B Instruct) with a small embedding model, run by llama.cpp.
How these fit together is explained in [architecture/README.md](../architecture/README.md) and, in full detail, in the
[retrospection and audit report](../reports/2026-10-09-project-retrospection-and-audit.md).

## 3. The three chapters

The work is organised in three chapters. Each has its own plan under [../roadmap/](../roadmap/README.md).

| Chapter | Goal | Plan |
|---|---|---|
| 1. Complete, test and validate OpenFrame | One fully working application, a Windows `.exe` and a macOS `.dmg` for internal use, installed on the owner's own Windows and Mac laptops, then used by real writers and production teams for 2 to 3 days, with every important problem fixed and re-checked | [chapter-1](../roadmap/chapter-1-complete-test-validate.md) |
| 2. Product website | A small static website on Cloudflare Pages with product information, tutorials, onboarding, support, contact, Privacy Policy and Terms, built from faithful recreations of the real screens | [chapter-2](../roadmap/chapter-2-website.md) |
| 3. Commercial distribution | The same application packaged for the Microsoft Store and the Mac App Store, with a 7-day free trial and paid activation, platform signing, notarization, licensing and updates | [chapter-3](../roadmap/chapter-3-commercial-distribution.md) |

Chapter 1 is the only chapter in progress. Chapter 2 starts when the application screens are stable. Chapter 3 starts
when Chapter 1 is finished. A decision that Chapter 3 would otherwise make expensive to change later may be taken in
Chapter 1, but no Chapter 3 code is written in Chapter 1.

## 4. What the internal builds are

The internal `.exe` and `.dmg` are for evaluation by the owner and invited testers.

- They are not signed and not notarized yet. Windows SmartScreen and macOS Gatekeeper will warn on first run. That is an
  installation restriction, not an application defect, and the test plan says how to get past it.
- They contain the app and the offline AI runtime. The AI models (Gemma 3 1B weights, embedding weights) download once,
  after installation, when the user clicks the download button ([ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md)).
- They carry no trial, no purchase, no price and no store lock ([ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md)).
- The team password gate of [ADR-0024](../adr/ADR-0024-internal-team-builds.md) is currently on. The owner decides whether
  the writers who test the build keep it (open question in owner-decisions.md).

## 5. Business model (target, not final)

The proposed model is a 7-day free trial followed by paid activation, one price per region:

| Region | Target price |
|---|---|
| India | 3,000 rupees |
| USA and the rest of the world | 89 US dollars |
| Europe | 89 euros |

Recorded from the owner's messages of 2026-10-09. An earlier message said 2,999 rupees and the later one says 3,000, so
the owner confirms the final figure. These are targets. [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md)
still says price and refund rules are open, and [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md) says the
app never shows a price: the store's own purchase window shows it. Neither store's rules are assumed to allow this
model. Chapter 3 checks them before anything is built.

## 6. What OpenFrame does not do, and must not claim

- No live network collaboration. Live LAN collaboration was removed ([ADR-0007](../adr/ADR-0007-lan-collaboration-removed.md)).
  The mock-ups 164 to 168 still show it. They must not be used for the website.
- No user accounts and no roles in the interface. The Rust core has a permission model (Owner, Editor, Commenter,
  Viewer, Export-only) used by packages and exports, but a person using the app is always the owner of their projects.
  Mock-up 155 (permissions) must not be advertised.
- No cloud AI, no model choice, no AI that changes a project on its own. Every AI change is a proposal that a person
  reads and applies.
- No encryption of projects at rest.
- Windows 11 and Apple silicon Macs only.

## 7. The claims rule

Nothing is advertised, on the website or in a store listing, unless the feature is Verified on the exact build that
people download. [feature-status.md](../implementation/feature-status.md) is the source for that. The same rule applies to
screenshots and to the HTML recreations of screens made for the website.

## 8. Open product questions

These are listed with their owners in [owner-decisions.md](../implementation/owner-decisions.md):

1. Final prices, currencies and refund rules.
2. Whether internal testers keep the team password gate.
3. The wording of the product description. The application text, the AI policy text and the installer description still
   say "filmmakers"; they are to be changed to "writers, screenwriters and production teams" when the owner approves the
   copy, because user-visible text is also product behaviour.
4. The end-user licence agreement, which must pass on the Gemma use restrictions to every user.
