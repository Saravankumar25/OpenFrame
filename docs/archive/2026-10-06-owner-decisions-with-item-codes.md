# 02: Owner decisions and manual work

> Status: **open**. Created 2026-10-05. Referenced by [00-masterplan.md](00-masterplan.md).
> Decisions here belong to the product owner. Engineers and agents must not decide them silently
> (Engineering Package Index §7; ADR-0012). Each decision names what it blocks, a recommendation, and the
> ADR that records the outcome. Mark a decision **Decided** only when the ADR status is *Accepted*.

## 1. Decisions

| ID | Decision | Blocks | Recommendation (not a decision) | ADR | State |
|---|---|---|---|---|---|
| D1 | **Business model and license.** Free/open-source (repo today) vs paid (supplied launch register assumes a 7-day no-card trial, pricing, refunds) vs free proprietary. | License file, Store pricing, entitlement code, EULA/Terms, privacy policy, listing text | Decide first. It changes legal text, code (trial/entitlement) and Store setup. | [ADR-0018](../adr/ADR-0018-distribution-licensing-commerce.md) | Open |
| D2 | **Store v1 scope.** Everything in the PRD, or a smaller first release whose listing matches only shipped features. | Milestone order, listing text, screenshots | Plan assumes full scope (your stated goal). Milestones M2 to M9 are independent, so a staged release only drops milestones from the gate, never reorders the rest. | ADR-0018 | Open |
| D3 | **Supported Windows versions.** Windows 11 only vs Windows 10 1809+. Repo says "Windows 11"; Store manifest says `MinVersion 10.0.17763.0`. | Store manifest, test matrix, listing | Windows 11 only (`10.0.22000.0`) unless you will test Windows 10. | [ADR-0017](../adr/ADR-0017-cross-platform-windows-and-macos.md) | Open |
| D4 | **macOS scope and timing.** Supplied OS plan: equal first-class target. Engineering Index: Windows first, macOS-ready boundaries. | Milestone M16, CI matrix, Apple account | Keep macOS out of the Store gate. Enforce platform-neutral rules now, ship macOS as milestone M16. | ADR-0017 | Open |
| D5 | **Telemetry, diagnostics, analytics.** README: none. Engineering Index: optional opt-in diagnostics. Supplied checklist: crash IDs, funnel analytics, health scorecard. | Privacy policy, Store privacy answers, website analytics | No automatic telemetry. User-initiated, inspectable, redacted diagnostic bundle. Website analytics only if privacy-friendly and disclosed. | [ADR-0021](../adr/ADR-0021-diagnostics-telemetry-analytics-policy.md) | **Decided 2026-10-05**: recommendation accepted (owner delegated to the recommendation) |
| D6 | **Appearance/theme system and dark mode.** Contradicts ADR-0012 §7 (light only). | Milestone M3 | Accept as specified (revised spec §47A). | [ADR-0014](../adr/ADR-0014-appearance-theme-system.md) | Open |
| D7 | **Scene production numbers.** Revised spec requires imported, alphanumeric, frozen numbers after lock. ADR-0011 stores no display numbers. | Milestone M2 schema | Store numbers as a label on the locked draft, never as identity. | [ADR-0015](../adr/ADR-0015-screenplay-production-numbering.md) | Open |
| D8 | **PDF fonts for Indic scripts.** Revised spec makes Telugu, Hindi, Tamil, Malayalam, Kannada P0 including PDF. ADR-0008 allows Latin-1 only. | Milestone M2 | Embedded subset fonts (OFL) with shaping, deterministic output kept. | [ADR-0016](../adr/ADR-0016-pdf-embedded-fonts-and-shaping.md) | Open |
| D9 | **AI runtime packaging in the Store build.** Downloading an executable after install is a certification risk. | Milestones M9, M11 | Bundle the tested `llama.cpp` runtime in the MSIX; download model weights only. Choose the model host. | [ADR-0020](../adr/ADR-0020-ai-runtime-packaging-per-channel.md) | Open |
| D10 | **Crash-safe AI Change Set apply.** Multi-operation apply is not crash-atomic today. | Milestone M1 | Durable apply journal with recovery on open. | [ADR-0019](../adr/ADR-0019-crash-safe-change-set-apply.md) | **Decided 2026-10-05**: recommendation accepted (owner delegated to the recommendation) |
| D11 | **Publisher and product identity.** Legal entity/umbrella company, publisher display name, product name rights, brand. | Partner Center, MSIX identity, license, policies | Reserve "OpenFrame Studio" in Partner Center early. | ADR-0018 | Open |
| D12 | **Template inventory.** Repo has 7 built-ins; revised spec recommends 8 (adds Feature, Short, Episodic screenplay templates; drops the daily-report template). | Milestone M7 | Adopt the revised 8 and keep the report layout as a report type. | none (spec) | Open |
| D13 | **Sample scripts.** OpenFrame-authored, public domain or licensed only. Who writes them. | Milestone M7 | Commission or write 4 fictional samples. | none | Open |
| D14 | **Support model.** Channel, owner, response targets, second operator. | Milestone M12 | One company-owned address plus a public KB. | none | Open |
| D15 | **Direct-download channel in v1.** Needs an Authenticode certificate and updater key. | Milestone M16 | Store first. Direct channel after the Store listing is live. | ADR-0018 | Open |
| D16 | **Target dates and capacity.** Launch date, who builds, how many agents/people in parallel. | Schedule only | Provide so milestones can be dated. | none | Open |
| D17 | **Beta program.** Number and identity of real filmmakers, duration. | Milestone M10 | 8 to 15 testers over 3 to 4 weeks on a release candidate. | none | Open |
| D18 | **Localization.** v1 UI English only, localization-ready architecture. | Milestone M10 | Confirm English-only v1. | none | Open |

Product questions left open by ADR-0012 and still unresolved: whether Build Screenplay blocks on missing scene
headings (PRD C-1); multiple screenplays per project (PRD C-28); call sheet status vocabulary and the "needs
refresh" trigger; Recently Deleted retention window and auto-purge. Record each answer as an ADR amendment.

## 2. Manual work (cannot be done by code changes)

Lead times are typical, not guaranteed. Start the long ones in milestone M0.

| ID | Work | Needed by | Lead time | Notes |
|---|---|---|---|---|
| X1 | Register the legal entity/umbrella company, or confirm which entity publishes. | M0 | days to weeks | Blocks Partner Center, license, policies. |
| X2 | Create Microsoft Partner Center developer account for that entity, verify it, add a **second administrator**. | M0 | days to weeks | Store account loss is a continuity risk. |
| X3 | Reserve the product name "OpenFrame Studio" and copy Product Identity (Name, Publisher, PublisherDisplayName) into `packaging/msix/store.config.json`. | M0 | hours after X2 | Placeholders currently block Store builds. |
| X4 | Choose and add the license (D1). Delete `LICENSE-PENDING.md`. | M0 | decision | Release gate fails closed until done. |
| X5 | Trademark and name clearance, domain, social handles, logo ownership, font licenses. | M0 | days to weeks | Do before spending on brand. |
| X6 | Buy the domain; set up company-owned email, SPF, DKIM, DMARC; HTTPS hosting. | M12 | days | Needed for privacy and support URLs. |
| X7 | Choose and provision the production model-distribution host (HTTPS). Generate the production Ed25519 manifest key, store the private key offline, set `OPENFRAME_MANIFEST_PUBLIC_KEY` and `MODEL_DISTRIBUTION_BASE_URL`. | M9 | days | Release gate fails if the dev key is used. |
| X8 | Gemma Terms of Use review and recorded owner confirmation (redistribution on your host). | M9 | days | `docs/engineering/23-release-migration.md` §7. |
| X9 | Qualified counsel review of Privacy Policy, Terms/EULA, refund text, AI disclosures. | M12 | 1 to 3 weeks | Required for the policy pages the Store links to. |
| X10 | Test hardware: minimum-spec Windows 11 PC (8 GB RAM), a recommended-spec PC, a discrete-GPU PC, clean Windows 11 VMs, high-DPI and multi-monitor setups. | M9, M10 | procurement | Needed for AI benchmark gate and clean-machine tests. |
| X11 | Run Windows App Certification Kit (WACK) on the exact Store MSIX and archive the report. | M11 | hours | Must be a Windows machine with the SDK. |
| X12 | Recruit and run the real-filmmaker beta (D17). | M10 | weeks | Needs a release candidate. |
| X13 | Write or commission the sample scripts (D13). | M7 | days to weeks | Copyright-safe only. |
| X14 | Store listing: description, screenshots from the exact release candidate with an invented demo project, age rating (IARC questionnaire, answer accurately for screenplays with mature themes), category, pricing/availability, **Notes for Certification**. | M13 | days | Claims must match shipped behavior (Store policy). |
| X15 | Support setup: inbox, KB, second operator, escalation levels, security disclosure address. | M12 | days | Also the AI "report output" destination. |
| X16 | Continuity: password manager, 2FA recovery codes, GitHub second admin, backups of keys and runbooks, a recovery drill. | M13 | days | Launch register BCP-001..005. |
| X17 | Marketing assets: trailer, founder story, content bank, creator/community outreach. | M14 | weeks | Not a Store gate. Required by the launch plan. |
| X18 | Final GO/NO-GO signature by the product owner. | M13 | hours | |
| X19 | Click "Submit" in Partner Center, monitor certification, respond to reviewer feedback. | M14 | days | Do not automate. |
| X20 | If macOS (D4): Apple Developer Program enrollment, Developer ID certificate, notarization credentials, second operator. | M16 | days to weeks | Not on the Store critical path. |
| X21 | If direct channel (D15): Authenticode certificate and updater signing key. | M16 | days to weeks | |
