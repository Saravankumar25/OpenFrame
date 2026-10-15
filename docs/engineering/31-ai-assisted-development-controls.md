# OpenFrame Studio: Vibe Coding Gaps and How to Cover Them

> **History (2026-10-15).** A plan of 2026-10-05 whose rules are enforced in `CONTRIBUTING.md` and the architecture lint. Where it speaks of "Done" or "Verified", the five proof levels of [ADR-0030](../adr/ADR-0030-verification-ladder-and-release-flow.md) now apply. This file moves to the archive in Windows phase W1.

> **Repository integration (2026-10-05).** Promoted from the supplied vibe-coding gap analysis. It is the rationale behind
> the controls in CONTRIBUTING.md and the build plan areas "Project controls and baseline", "Data safety and crash recovery" and "Runtime resilience and failure testing": feature contract, feature status file, architecture lint, migration fixtures,
> lifecycle E2E, round-trip corpus, AI evaluation, golden rendering, stale-proposal suite. "Windows and macOS" rows apply to
> macOS as well, because the macOS work is on the release path (ADR-0017, Accepted 2026-10-06).

**Purpose:** identify the risks created when a large desktop application
is developed through fast/AI-assisted/"vibe coding" workflows, and
convert those risks into enforceable engineering controls.

**Baseline:** OpenFrame `main` at
`d30b5889ab5edf9f69d60f8e6d600cb11d4921a0`.

------------------------------------------------------------------------

# 1. Executive assessment

OpenFrame is no longer a small prototype where "make the screen work" is
enough.

It has:

-   a persistent project format
-   migrations
-   transactional mutations
-   undo/redo
-   recovery
-   search
-   file management
-   permissions
-   imports/exports
-   AI tools
-   a derived retrieval index
-   local model processes
-   package workflows
-   production documents
-   multiple workspaces
-   planned Windows + macOS distribution

At this scale, the primary risk of vibe coding is **not bad code
syntax**.

The primary risk is **semantic drift**:

> code appears to work locally while silently violating an existing
> invariant somewhere else.

Examples:

-   a new command writes directly to SQLite
-   a feature works but has no undo
-   a new field is saved but not exported
-   an export is correct but cannot be re-imported
-   a React component stores canonical data in localStorage
-   AI can see a field that the UI permission model hides
-   a new table is not indexed
-   a delete path forgets Recently Deleted
-   a migration works on a fresh database but breaks an existing project
-   a Windows path works but fails on macOS
-   a model update changes retrieval behavior without evaluation
-   a UI button exists but no reachable command is wired
-   a command is implemented but hidden from the AI coverage contract
    unintentionally
-   a package import creates data that cannot later be safely exported

The cure is not "stop using AI".

The cure is to make the **architecture and verification system stronger
than the coding style**.

------------------------------------------------------------------------

# 2. Vibe-coding risk model

Use these levels, from lowest to highest risk.

| Risk level | Example | Required control |
|---|---|---|
| Cosmetic | spacing, icon, label | normal review |
| Local UI logic | filter, sorting, component state | component unit test |
| Operation wiring | new query/command | Registry + generated types |
| Canonical data | schema/mutation | migration + transaction + undo |
| Cross-module | Story → Screenplay → Breakdown | lineage tests + E2E |
| Security | filesystem/import/AI permissions | threat model + security test |
| Release | installer/update/signing | release gate |
| Data format | migration/import/export | fixture corpus + round-trip |
| AI authority | tools/Change Sets/retrieval | AI coverage + security + eval |

Changes at the cross-module, security, release, data format and AI authority levels should never be merged solely because "the feature
works".

------------------------------------------------------------------------

# 3. The central rule

Every AI-assisted change must answer:

``` text
What invariant does this code preserve?
What existing contract does it touch?
What new contract does it create?
How is it tested?
How does it recover?
How does it export?
How does it behave on Windows?
How does it behave on macOS?
How does AI see it?
How does AI NOT see it?
```

If those questions cannot be answered, the implementation is incomplete.

------------------------------------------------------------------------

# 4. Existing OpenFrame architecture is the anti-vibe-coding safety net

The repository already has excellent enforcement points.

## Registry

Prevents arbitrary UI-to-Rust operations.

## Store::mutate

Provides one canonical mutation path:

``` text
permission
→ transaction
→ module logic
→ undo
→ search reindex
→ activity
→ commit
→ events
```

## Generated TypeScript bindings

Reduce Rust/TypeScript DTO drift.

## Migration system

Provides controlled schema evolution.

## Search projections

Provide explicit derived indexing.

## AI OperationMetadata

Prevents accidental AI exposure.

## Change Sets

Keep AI mutations human-approved.

## Security review

Provides concrete attack-oriented regression tests.

The goal is to make these mechanisms **hard to bypass**, especially for
AI-generated code.

------------------------------------------------------------------------

# 5. Gaps that still need stronger coverage

## No mandatory feature contract

A developer can still think:

> "I added the button and backend; feature complete."

Add a feature contract template.

``` markdown
# Feature Contract

## User outcome
## Entry points
## Canonical data
## Queries
## Commands
## Permissions
## Undo
## Recovery
## Search
## AI exposure
## Import
## Export
## Cross-platform
## Accessibility
## Performance
## Tests
## Known limitations
```

Every major feature PR includes this.

------------------------------------------------------------------------

# 6. Implementation can outrun specification

AI coding tends to generate plausible behavior that was never explicitly
required.

Keep a feature status file (`docs/implementation/feature-status.md`) with:

``` text
ID
Source requirement
Module
Operation
Schema
UI entry point
Status
Tests
Export
Import
AI
Windows
macOS
Owner
```

Statuses:

``` text
Specified
Designed
Built
Reachable
Verified
Release-ready
```

A button with backend code is only Built.

------------------------------------------------------------------------

# 7. Duplicated business rules

Vibe coding often creates:

``` text
UI validation
Rust validation
AI validation
Import validation
Export validation
```

with subtly different rules.

Correct design:

``` text
input
 ↓
canonical Rust domain validation
 ↓
same operation
```

UI validation is only a usability optimization.

AI validation is only a security layer.

Import validation must ultimately create data through the same canonical
commands or transaction contracts.

------------------------------------------------------------------------

# 8. Direct database shortcuts

Ban these patterns in application modules:

``` rust
Connection::open(...)
conn.execute(...)
```

outside approved persistence infrastructure.

Add CI/search checks that fail when direct SQLite writes appear in:

-   React
-   Tauri adapter
-   AI toolbox
-   UI-specific modules

Allowed:

-   persistence
-   Store
-   migration code
-   approved derived index connections

------------------------------------------------------------------------

# 9. AI-generated schema changes without lifecycle analysis

For every migration:

``` text
new table
→ create
→ read
→ update
→ delete
→ undo
→ restore
→ search
→ AI index
→ package
→ export
→ import
→ migration
```

Require a migration review checklist.

------------------------------------------------------------------------

# 10. "works on my Windows machine"

The current project documentation is Windows-first.

That is no longer enough.

Every OS-sensitive feature needs a matrix:

  Area                      Windows   macOS
  ------------------------- --------- -------
  path resolution           test      test
  project creation          test      test
  project lock              test      test
  atomic file replacement   test      test
  file open/reveal          test      test
  sidecar lifecycle         test      test
  installer                 test      test
  update                    test      test
  crash recovery            test      test
  import/export             test      test
  theme rendering           test      test
  keyboard shortcuts        test      test
  permissions               test      test

------------------------------------------------------------------------

# 11. Feature branches can silently break AI

Every operation must have metadata:

``` rust
OperationMetadata {
    module,
    description,
    ai_exposure,
    operation_class,
    required_capability,
    destructive,
    irreversible,
    confirmation,
    filesystem_effect,
    long_running,
}
```

Make `ai_tool_coverage` blocking.

For every new operation:

``` text
registered
→ metadata present
→ exposure decision
→ tool mapping
→ security test
→ evaluation case
```

If the feature is intentionally hidden, document why.

------------------------------------------------------------------------

# 12. AI "training" mistaken for feature integration

Do not fine-tune the model every time a feature is added.

OpenFrame's first-line adaptation mechanism should be:

1.  operation metadata
2.  deterministic tools
3.  retrieval documents
4.  context graph
5.  product knowledge
6.  examples/evaluation cases
7.  system constraints

Fine-tuning is a later optimization only if evaluation demonstrates a
model-capability gap.

------------------------------------------------------------------------

# 13. Retrieval quality is not yet measured

Current implementation has hybrid retrieval, graph expansion, provenance
and canonical revalidation.

The missing piece is a real evaluation set.

Build:

``` text
gold_retrieval_cases.jsonl
```

Each case:

``` json
{
  "query": "...",
  "scope": "...",
  "expected_entities": ["..."],
  "expected_relations": ["..."],
  "route": "HybridGraph"
}
```

Measure:

-   Recall@1
-   Recall@5
-   Recall@10
-   MRR
-   nDCG
-   source correctness
-   permission leakage
-   stale-result rate

Run against:

-   FTS
-   semantic
-   hybrid
-   hybrid + graph

Use real BGE embeddings for release evaluation.

------------------------------------------------------------------------

# 14. No systematic stale-data testing across every domain

AI already records base revisions for proposals.

Expand tests:

``` text
retrieve
→ proposal
→ user changes row
→ apply
→ must refuse/recheck
```

Test this for:

-   screenplay
-   story
-   breakdown
-   schedule
-   call sheets
-   cast/crew
-   budget
-   notes
-   visual planning

------------------------------------------------------------------------

# 15. Insufficient property-based testing

Use property tests for:

### ordering

Moving items repeatedly must preserve a valid ordering.

### undo

``` text
state A
→ command
→ undo
→ state A
```

### redo

``` text
A → B → undo → redo = B
```

### deletion

``` text
delete
→ restore
→ semantic content preserved
```

### import/export

``` text
import
→ export
→ import
```

must preserve required semantics.

### migration

Every supported old fixture must reach the current schema without
violating invariants.

------------------------------------------------------------------------

# 16. No golden rendering discipline

The revised specification requires professional screenplay pagination
and PDF visual regression.

Add golden artifacts for:

-   title page
-   scene heading
-   dialogue
-   continuation
-   page breaks
-   scene numbers
-   revision colors
-   long dialogue
-   Unicode
-   multilingual text
-   empty pages
-   locked pages

Use pixel/structural regression with controlled tolerances.

------------------------------------------------------------------------

# 17. AI-generated UI can create fake completeness

A common failure mode:

``` text
beautiful workspace
↓
mock data
↓
button
↓
toast
```

but no canonical persistence.

Require every workspace acceptance test to prove:

``` text
create
→ close
→ reopen
→ data exists
→ undo
→ redo
→ restart
→ export
```

------------------------------------------------------------------------

# 18. Hidden unreachable features

Audit:

``` text
operation exists
AND
UI entry point exists
AND
user can discover it
AND
route works
```

For each must-have feature record:

``` text
Discoverable: yes/no
Reachable: yes/no
Persisted: yes/no
Recoverable: yes/no
Exported: yes/no
Re-importable: yes/no
```

------------------------------------------------------------------------

# 19. Generated code drift

Never hand-edit generated IPC bindings.

CI:

``` text
cargo test export_bindings
git diff --exit-code apps/desktop/src/ipc/generated
```

This already exists conceptually and should remain blocking.

------------------------------------------------------------------------

# 20. Dependency drift

For every dependency update:

``` text
cargo deny
cargo audit
npm audit
license inventory
build
tests
security review if parser/runtime changed
```

Do not blindly accept automated dependency upgrades.

------------------------------------------------------------------------

# 21. Parser vulnerabilities

Imported:

-   FDX
-   DOCX
-   PDF
-   ZIP
-   images

must be treated as hostile.

Maintain:

``` text
parser corpus
├─ normal
├─ malformed
├─ huge
├─ nested
├─ zip-slip
├─ path traversal
├─ entity expansion
├─ decompression bomb
└─ fuzz-generated
```

Current security review already fixed major parser findings. Preserve
those tests permanently.

------------------------------------------------------------------------

# 22. Local AI process lifecycle

Current security review identifies remaining sidecar hardening work.

Complete:

1.  create process in a suspended/controlled state where platform
    permits
2.  apply process containment
3.  resume
4.  verify executable/hash
5.  enforce localhost + random port
6.  authenticate every request
7.  track PID/process identity
8.  kill on shutdown
9.  sweep orphaned sidecars at startup
10. never execute from arbitrary project paths

Windows should use Job Objects.

macOS should use process-group/session controls plus signed executable
placement and hardened distribution.

------------------------------------------------------------------------

# 23. Platform-specific conditional compilation can spread

Bad:

``` rust
#[cfg(windows)]
// inside screenplay module
```

Good:

``` text
application
   ↓
PlatformServices trait
   ↓
windows implementation
macos implementation
```

Feature code stays portable.

------------------------------------------------------------------------

# 24. No architectural "do not do this" enforcement

Create automated architecture checks.

Examples:

``` text
React imports:
  allowed: ipc, UI libraries, local pure utilities
  forbidden: fs, sqlite, shell, HTTP clients

application modules:
  forbidden: direct network
  forbidden: platform paths
  forbidden: Tauri types

AI:
  forbidden: direct SQL writes
  forbidden: filesystem paths
  forbidden: Change Set apply

search:
  forbidden: canonical mutation

platform:
  forbidden: screenplay/business semantics
```

------------------------------------------------------------------------

# 25. AI coding workflow that is safe

For every AI-generated implementation:

## Ask for the contract first

Before code:

``` text
list affected modules
list invariants
list files
list operations
list migrations
list tests
list platform impacts
```

## Implement the smallest vertical slice

``` text
schema
→ operation
→ UI
→ persistence
→ test
```

Do not generate 40 files at once.

## Run mechanical checks

``` bash
npm run format:check
npm run lint
npm run typecheck
npm run test
npm run bindings
```

## Inspect the semantic diff

Ask:

-   Did it bypass Store?
-   Did it add an operation?
-   Did it alter permissions?
-   Did it change file paths?
-   Did it alter migrations?
-   Did it alter AI exposure?
-   Did it alter export?
-   Did it change platform assumptions?

## Run the lifecycle test

``` text
create
save
restart
undo
redo
delete
restore
export
import
```

## Run the security test

Run targeted security suites for any:

-   parser
-   path
-   package
-   AI
-   process
-   external link
-   update

change.

------------------------------------------------------------------------

# 26. PR gates

### Compile gate

``` text
format
clippy
typecheck
eslint
```

### Behavior gate

``` text
unit
integration
frontend
E2E
```

### Architecture gate

``` text
no forbidden imports
no direct DB writes
no unregistered operation
no generated binding drift
```

### Data gate

``` text
migration fixtures
undo/redo
recovery
import/export
```

### Security gate

``` text
cargo deny
audit
parser tests
path tests
AI security
```

### Platform gate

``` text
Windows
macOS
```

### Release gate

``` text
signatures
installer
notarization
SBOM
licenses
checksums
```

------------------------------------------------------------------------

# 27. Definition of "AI-generated code is acceptable"

AI-generated code is acceptable when:

-   it obeys existing architecture
-   contracts are explicit
-   tests prove behavior
-   security boundaries remain intact
-   data lifecycle is complete
-   platform behavior is verified
-   no undocumented shortcut was introduced

The origin of the code is irrelevant after these controls pass.

------------------------------------------------------------------------

# 28. Definition of "vibe coding gap closed"

A gap is closed only when the **system makes the mistake difficult to
introduce**.

Examples:

``` text
"developer remembers not to bypass Store"
```

is weak.

``` text
architecture lint + code review + tests
```

is strong.

Likewise:

``` text
"developer remembers to expose AI metadata"
```

is weak.

``` text
coverage test fails if metadata is missing
```

is strong.

------------------------------------------------------------------------

# 29. Recommended implementation order

1.  feature status file
2.  architecture lint
3.  platform abstraction
4.  migration fixture harness
5.  lifecycle E2E harness
6.  import/export round-trip corpus
7.  AI retrieval evaluation
8.  stale proposal suite
9.  rendering regression suite
10. Windows + macOS CI matrix
11. release gates

These controls should be built **before** the final feature rush, not
after it.

------------------------------------------------------------------------

# 30. Core principle

OpenFrame should be developed like this:

``` text
fast coding
+
strict contracts
+
automatic verification
+
reproducible builds
+
data lifecycle tests
+
security boundaries
+
cross-platform CI
=
fast AND safe
```

The objective is not to eliminate AI-assisted development.

The objective is to ensure that **AI-assisted development cannot
silently redefine OpenFrame's architecture**.
