# Owner decisions and manual work

> Rewritten 2026-10-15 so that every row is a decision still in force. Decisions here belong to the product owner. Engineers
> and agents must not decide them silently. Mark a decision **Decided** only when its ADR is Accepted or the owner has
> answered in chat and the answer is written here. The earlier table, with its replaced rows and old wording, is kept in
> [the archive](../archive/2026-10-15-owner-decisions-before-restructure.md). The build plan is [masterplan.md](masterplan.md);
> the phases are in [../roadmap/](../roadmap/README.md).

## 1. Decided

### Product, chapters and business

| Decision | Answer | Date | Record |
|---|---|---|---|
| Audience | One application for writers, screenwriters and production teams; no separate editions | 2026-10-09 | [vision-and-scope.md](../product/vision-and-scope.md) |
| Chapters and order | Chapter 1: Windows track (ten phases, internal `.exe`, team testing), then Mac track (ten phases, internal `.dmg`, team testing). Chapter 2: website. Chapter 3: store packaging. Strict order; later chapters' plans are written when the earlier one closes | 2026-10-15 | [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md), [roadmap](../roadmap/README.md) |
| Scope of the first release | The whole product | 2026-10-06 | none |
| Business model | Paid app with a 7-day free trial, no credit card to start | 2026-10-06 | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) |
| Target prices | Target, not final: 3,000 rupees in India (an earlier message said 2,999), 89 US dollars elsewhere, 89 euros in Europe. The stores' rules are checked first | 2026-10-09 | [Chapter 3](../roadmap/chapter-3-commercial-distribution.md) |
| Price in the app | Never shown; Buy opens the store's own purchase window | 2026-10-07 | [ADR-0023](../adr/ADR-0023-store-trial-and-internal-installer.md) |
| Microsoft Store trial and lock | The Store runs the 7-day trial; ended and not bought gives a full block except Buy and Restore | 2026-10-07 | ADR-0023 |
| Mac App Store trial and lock | Zero-price "7-day Trial" and paid "Lifetime Access" non-consumables, server-checked expiry; full block at expiry; confirmed 2026-10-08 | 2026-10-06 | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md), [ADR-0022](../adr/ADR-0022-mac-trial-end-lock.md) |
| Diagnostics, telemetry and analytics | The app sends nothing automatically; the user can export an inspectable, redacted diagnostics file and an AI report | 2026-10-05 | [ADR-0021](../adr/ADR-0021-diagnostics-telemetry-analytics-policy.md) |
| App screen language | English only; the AI answers in the language of the request | 2026-10-15 | [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) |
| Website | Purchased domain with company email; hosting proposed on Cloudflare Pages; built from faithful recreations of the real screens; "built manually" to be clarified when the Chapter 2 plan is written | 2026-10-15 | [Chapter 2](../roadmap/chapter-2-website.md) |

### Platforms and architecture

| Decision | Answer | Date | Record |
|---|---|---|---|
| Release targets | Windows 11 only; Apple silicon Macs only. The Intel MacBook is a development and verification machine, never a release target | 2026-10-06 | [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md), [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) |
| Core and shells | One Rust core. Windows shell: Tauri and React. Mac shell: SwiftUI over the core through UniFFI. No logic rewritten in Swift. Tauri on the Mac is retired | 2026-10-15 | ADR-0027 |
| When the Mac work starts | After the Windows track has closed (team pilot done); then the full SwiftUI shell | 2026-10-15 | ADR-0027, ADR-0030 |
| Release files | Four: Microsoft Store package, Mac App Store package, internal `.exe`, internal `.dmg` | 2026-10-07 | [ADR-0024](../adr/ADR-0024-internal-team-builds.md) |
| Windows installer (`.exe`) | Internal build for the team only, never sold: no trial, licence, payment or lock | 2026-10-07 | ADR-0023 |
| Signing of internal builds | Unsigned; signing and notarization move to Chapter 3 | 2026-10-08 | ADR-0024 |
| Team password gate | On the internal builds. Asked on every launch; wrong tries wait 1 minute, doubling to 30. Minimum length 8 characters (was 12). Only a hash from a build-computer environment variable is embedded; nobody else sees the password and agents never ask for it. A door lock, not encryption | 2026-10-15 | ADR-0024, [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) |
| Proof levels | Five levels of proof; "Verified" means level 3 or higher on the release build once W9 is reached | 2026-10-15 | ADR-0030 |
| Clean-machine testing | A virtual machine on the owner's computer plus borrowed laptops; the owner tests on several laptops with different graphics cards | 2026-10-15 | ADR-0030, [ADR-0029](../adr/ADR-0029-gpu-acceleration.md) |

### The offline AI

| Decision | Answer | Date | Record |
|---|---|---|---|
| How the AI ships | The runtime ships in every package. The models download after install on the user's click from a signed manifest with SHA-256 checks; nothing downloads silently | 2026-10-15 | [ADR-0026](../adr/ADR-0026-offline-ai-download-after-install.md) |
| Visual C++ runtime for the engine | The Microsoft Visual C++ runtime DLLs ship app-local next to the engine | 2026-10-08 | ADR-0026 |
| Release bar | English: 90 percent correct tool and arguments, 85 percent correct end state on the gold set, on the release build, on both platforms. Not lowered silently | 2026-10-15 | [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) |
| Languages | English first. Also Hindi, Telugu, Tamil, Kannada, Malayalam, Spanish, French, German, Chinese, Japanese, Korean, held to the same retrieval and tool-choice bar, writing slightly plainer | 2026-10-15 | ADR-0028 |
| Models | Two tiers (a small default, a larger tier for 16 GB or a suitable card). Any open model with a clean licence may win a bake-off; Gemma 3 is the incumbent. One multilingual embedding model preferred, chosen by the bake-off | 2026-10-15 | ADR-0028 |
| The loop | Verifier-guided retries, three passes at most, 60 seconds in total on a processor; after the last pass, the best answer marked "not sure" plus one clarifying question | 2026-10-15 | ADR-0028 |
| Memory | Per project; sharing across projects only when the person turns it on; learning on by default with a screen to view and delete it; written only from the person's own actions | 2026-10-15 | ADR-0028 |
| AI log | Numbers and ids only, plus a manual Export AI report button; nothing leaves the computer automatically | 2026-10-15 | ADR-0028 |
| Test judging | Claude at development time with a human spot check for writing quality. The non-English test sets are reviewed by Claude only (owner's choice); their reports say "automated review" until a native speaker checks a sample | 2026-10-15 | ADR-0028 |
| Runtime judging | While people use the app, only permanent deterministic code evaluates AI tasks and adapts the harness; no model, Gemma or any other, judges answers. Claude evaluates only at development and test time and its findings become code | 2026-10-15 | ADR-0028 |
| Apple's on-device model | An opt-in setting on the Mac; the default stays the open model | 2026-10-15 | ADR-0027, ADR-0028 |
| GPU | Vulkan first; a pure decision function; a first-run probe that falls back to the processor; integrated graphics allowed when the probe passes; embeddings stay on the processor; speed claims only for measured cards | 2026-10-15 | [ADR-0029](../adr/ADR-0029-gpu-acceleration.md) |
| AI files for CI | CI tests the download against a test manifest and host; local builds use `.ai-assets` | 2026-10-15 | ADR-0026 |
| Crash-safe AI changes | An AI change that a crash interrupts is undone on the next open | 2026-10-05 | [ADR-0019](../adr/ADR-0019-crash-safe-change-set-apply.md) |
| AI Assistant panel placement | The panel docks beside the page and narrows it; never covers a control; open or closed is remembered | 2026-10-08 | UX specification section 71 |

### Product behaviour

| Decision | Answer | Date | Record |
|---|---|---|---|
| Themes and dark mode | 15 built-in themes with Light, Dark and System; today's look is the default; six Light-mode contrast pairs raised to 4.5 to 1 | 2026-10-07 | [ADR-0014](../adr/ADR-0014-appearance-theme-system.md) |
| Scene production numbers | A frozen number label on the locked draft, never the identity | 2026-10-07 | [ADR-0015](../adr/ADR-0015-screenplay-production-numbering.md) |
| PDF fonts for Indian scripts | Embedded subset fonts under the Open Font Licence with shaping | 2026-10-07 | [ADR-0016](../adr/ADR-0016-pdf-embedded-fonts-and-shaping.md) |
| Built-in templates and samples | Eight templates; four fictional samples owned by OpenFrame | 2026-10-07 | masterplan area 15 |
| Build Screenplay with a card without a heading | Ask: add a heading or leave the card out; never build silently | 2026-10-07 | ADR-0012 amendment |
| Call sheet status words | Draft, Needs refresh, Final | 2026-10-07 | ADR-0012 amendment |
| Recently Deleted | Items are kept 30 days, then purged automatically | 2026-10-07 | ADR-0012 amendment |
| Several screenplays per project | Yes; each has its own drafts; the Production Source screenplay cannot be deleted until another is chosen | 2026-10-08 | ADR-0012 amendment |
| Private notes in a restored backup | The person restoring becomes the owner of its private notes | 2026-10-08 | ADR-0012 amendment |
| Enter after a character name | One Enter creates the character and moves to Dialogue | 2026-10-08 | Revision 1 specification section 7 |
| Dashes in on-screen text | No em dash, and no en dash used as a dash, in any text OpenFrame shows or writes itself | 2026-10-08 | [Writing standard](../engineering/35-writing-standard.md) |

### How the work is done

| Decision | Answer | Date | Record |
|---|---|---|---|
| How work reaches `main` | Local merge only, after unit tests and end-to-end specs pass. A work-in-progress commit on its own branch is a checkpoint. No push, pull request or repository change unless the owner asks in that conversation | 2026-10-08 | CLAUDE.md |
| No build without the word | An installer, a release executable or a disk image is built only when the owner says so in that conversation | 2026-10-08 | CLAUDE.md, ADR-0030 |
| Repository visibility | Private. Classic branch protection needs a paid plan; a free ruleset is an owner choice | 2026-10-06 | masterplan area 1 |
| Verification before ticking | A box is ticked only with `(ev: ...)` evidence, at the proof level it names | 2026-10-08 | CLAUDE.md, ADR-0030 |
| Documentation | Organised by chapter; history moves to the archive and is not rewritten; no two current documents give different rules; one pass, not several | 2026-10-15 | [docs/README.md](../README.md) |
| Build folders and storage | One build folder per checkout under `.build-cache`; budgets in the storage policy | 2026-10-08 | [development-storage-policy.md](../development-storage-policy.md) |
| Unmaintained font crates | Dated exception for `rustybuzz` and `ttf-parser` until 2027-01-06 | 2026-10-08 | `deny.toml`, masterplan area 6 |
| Release-build tests and real data | The release build honours the test data folder for file locations only; every other test switch stays debug-only | 2026-10-16 | [ADR-0031](../adr/ADR-0031-release-build-test-data-folder.md) |
| WebView2's own diagnostic data | Allow and disclose: the runtime's Microsoft diagnostic-data upload follows the Windows setting and cannot be switched off by the app; spec 05 tolerates only that shape | 2026-10-16 | Threat model section 4 item 8, privacy policy |
| Tests from a release-ready point of view | Every test that proves a feature runs against the release build and its budgets are gates there: the end-to-end suite on the release executable (open and gated), the performance budgets as hard checks, the AI harness on the release profile, the Rust suite once with `--release`; debug runs are only for working on a change | 2026-10-16 | this file; spec 97; DEVELOPMENT |
| W1 to W3 on `main` | Merged locally (no push) before W4 started | 2026-10-16 | progress log |

## 2. Still open

The "Recommendation" column is advice, not a decision. "Blocks" names the phase that waits.

| Decision | Blocks | Recommendation | Record |
|---|---|---|---|
| Price, currency, what the trial unlocks | Chapter 3, website | Owner, 2026-10-08: decided later. Nothing touches pricing until then | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) |
| Final prices (3,000 or 2,999 rupees) | Chapter 3 | Confirm; check which prices each store allows | [Chapter 3](../roadmap/chapter-3-commercial-distribution.md) |
| Refund rules | Chapter 3 | Follow each store's own rules | ADR-0018 |
| Publisher and product identity | Chapter 3 | Reserve "OpenFrame Studio" in Partner Center early | ADR-0018 |
| End-user licence terms | Chapter 3 | Replace the pending licence with a written agreement reviewed by counsel; it passes on every model's use restrictions | ADR-0018 |
| Support model | Chapter 2 | One company address plus a public help centre | none |
| Launch date and capacity | Schedule only | Say how many people build in parallel | none |
| Beta programme | Chapter 3 | 8 to 15 real users for 3 to 4 weeks on a release candidate | none |
| Store payment route on Windows | Chapter 3 | Store trial and purchase, or an own system; read the current policy first | Chapter 3 plan |
| Stronger team-gate protection and encryption at rest | Chapter 3 | Decide with the store builds; options in [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) | ADR-0030 |
| Product wording "filmmakers" | W1 | **Decided 2026-10-16 (owner, in chat):** the long description uses the README sentence "a local-first desktop studio for writers, screenwriters and production teams, covering ..."; done in `tauri.conf.json`, package descriptions, CONTRIBUTING, privacy policy draft and code comments. The AI instructions keep their wording until the W5 measurement exists (ADR-0028) | [vision](../product/vision-and-scope.md) |
| Documents folder unknown | W2 | Refuse to start with a plain message (the Windows plan assumes this; confirm) | [37-platform-boundary.md](../engineering/37-platform-boundary.md) |
| Where new projects are saved by default | W2 | Warn when Documents is synced and offer a local folder; never move existing projects | W2 |
| Worktree review | W1 | Approve the table of 39 dead worktree folders and 31 unmerged agent branches before anything is removed | [development-storage-policy.md](../development-storage-policy.md) |
| Which tooling stays | W1 | Keep or remove `claude-mem`, the GSD framework and the stale `AGENTS.md` (see CLAUDE.md "Installed tooling") | CLAUDE.md |
| macOS version of the Intel MacBook | M1 | Owner reports it; it sets the Xcode version and what Intel can prove | [ADR-0027](../adr/ADR-0027-rust-core-two-ui-shells.md) |
| Screenplay editor on the Mac | M4 | Decide after the spike with its numbers | ADR-0027 open item 1 |
| How the Mac runs the AI engine | M8 | Helper process or linked library, after reading Apple's sandbox rules | ADR-0027 open item 2 |
| Meaning of "build the website manually" | Chapter 2 plan | Clarify before the plan is written | [Chapter 2](../roadmap/chapter-2-website.md) |
| Human review of the non-English gold sets | W5, pilot | Owner chose Claude only; a native-speaker sample from the team during the pilot is recommended | [ADR-0028](../adr/ADR-0028-ai-quality-program-and-harness.md) |

## 3. Manual work (cannot be done by code changes)

Lead times are typical, not guaranteed. Start the long ones early.

| Work | Needed before | Lead time | Notes |
|---|---|---|---|
| Install VirtualBox (or another virtual machine tool) and make a clean Windows 11 machine | W9 | hours | The owner's computer runs Windows 11 Home, which has no Sandbox |
| Borrow laptops with different graphics cards and one 8 GB laptop | W5, W7, W9 | procurement | Benchmarks and clean install; store each result with the card name and driver |
| Report the macOS version of the Intel MacBook | M1 | minutes | Sets the Xcode version |
| Obtain an Apple silicon Mac | M8, M9 | procurement | Needed for Metal, Apple's model and the `.dmg` |
| Choose the model host (HTTPS), generate the production Ed25519 manifest key, keep the private key offline, set `OPENFRAME_MANIFEST_PUBLIC_KEY` and `MODEL_DISTRIBUTION_BASE_URL` | W8 | days | The release build fails without both (checked in `build.rs`, 2026-10-16). Steps: on an offline computer run `node scripts/model-manifest.mjs new-key --out <file outside the repository>` and keep that file offline; set the printed `OPENFRAME_MANIFEST_PUBLIC_KEY` and the host address on the build computer; run `node scripts/model-manifest.mjs publish --key <that file> --out <folder>` and upload the folder as it is. The private key never enters the repository |
| Redistribution check of every shipped model: confirm OpenFrame may host the weights and tokenizer for download; record the confirmation. Same for the embedding model and the runtime | W5 (candidates), W8 (host) | days | ADR-0026 decision 9; [23-release-migration.md](../engineering/23-release-migration.md) |
| Set `OPENFRAME_TEAM_PASSWORD` on the build computer before building an internal `.exe` or `.dmg` | W9, M9 | minutes | Only the owner and the team know it; never written to a file, chat, log or the repository |
| Approve the gold sets and the bake-off result | W5 | days | |
| Recruit the team for both pilots | W10, M10 | days | |
| Buy the domain; company email with SPF, DKIM, DMARC; HTTPS hosting | Chapter 2 | days | Needed for the privacy and support links |
| Register the legal entity or confirm the publisher | Chapter 3 | days to weeks | Blocks Partner Center, licence and policies |
| Create the Microsoft Partner Center account, add a second administrator; reserve the name; copy identity values into `packaging/msix/store.config.json` | Chapter 3 | days to weeks | Placeholders block store builds |
| Create the Apple Developer Program account, App Store Connect record, certificates, second operator | Chapter 3 | days to weeks | Needs a Mac |
| Buy a code-signing certificate and generate the updater signing key | Chapter 3 | days to weeks | Keep private keys offline |
| Write or approve the end-user licence agreement and delete `LICENSE-PENDING.md` | Chapter 3 | decision | The release check fails closed until done |
| Trademark and name clearance, social handles, logo ownership, font licences | Chapter 2 and 3 | days to weeks | Before spending on brand |
| Counsel review of the privacy policy, terms, refund text and AI disclosures | Chapter 2 and 3 | 1 to 3 weeks | Required for the policy pages the stores link to |
| Support setup: inbox, help centre, second operator, escalation, security disclosure address | Chapter 2 | days | Also the destination for "Report AI output" |
| Run the Windows App Certification Kit on the exact Store package | Chapter 3 | hours | Needs the SDK |
| Create the two in-app purchases in App Store Connect, turn Family Sharing off, add sandbox testers; stand up the licence server and its signing key; confirm Apple's Root CA fingerprint | Chapter 3 | days | Private keys never go in the app |
| Run the Apple validation and TestFlight on the exact Mac package | Chapter 3 | hours to days | |
| Store listings, marketing assets, continuity (password manager, recovery codes, key backups) | Chapter 3 | days to weeks | Claims must match shipped behaviour |
| Final go or no-go signature; press Submit in each store console | Chapter 3 | hours to days | Do not automate |
| **Done 2026-10-08 by the owner:** Windows Defender exclusions for `C:\OpenFrame`, `.cargo`, `.rustup` and the sccache folder | n/a | n/a | Security settings stay with the owner; agents never change them |
