# Orders: port the conversational AI harness to the Mac app

> Written 2026-10-27 on Windows from the code on `main` (commit `73640725`) and a read-only look at the Mac tree. Paste
> everything below the line into a Claude Code session opened on the Mac, in a clone of GitHub `Saravankumar25/OpenFrame`.
> It replaces section 1 of [mac-port-list-2026-10-24.md](mac-port-list-2026-10-24.md) for the AI (that section lists the
> five lanes, which were retired on 2026-10-26).

---

## 0. Mission

Bring the Mac app's AI assistant to the same behaviour and the same measured quality as the Windows assistant on GitHub
`main` `windows/`. One conversational path for every question. Code resolves, fetches and checks. The model only writes.
Measure before and after, on the same gold sets as Windows.

Done means all of the following:

- The Mac core runs the Windows AI code.
- The Mac screens use it.
- The Low Tide eval meets the same bars on Apple silicon with Metal.
- Every number is in the Mac evidence folder.
- Nothing is pushed, merged or packaged without the owner's word in that session.

## 1. Situation (facts checked 2026-10-27)

### 1.1 Repository

GitHub `main` holds two folders, `windows/` and `mac/` (ADR-0034).

- Work only inside `mac/`.
- Read `windows/` as the source of truth for the AI. Never edit `windows/` from this session.
- The shared Rust core exists twice, once in each folder. The two copies drift by design until a port like this one.

### 1.2 Where the Mac AI stands today

| Area | State in `mac/` |
|---|---|
| `crates/openframe-application/src/modules/ai/` | Holds only the pre-lane files: `catalog`, `change_set.rs`, `commands.rs`, `intelligence`, `knowledge.*`, `mod.rs`, `offline.rs`, `orchestrator.rs`, `queries.rs`, `records.rs`, `retrieval.rs`, `scope.rs`, `service.rs`, `toolbox*`, `tools.rs`, `trace.rs`, `types.rs` |
| `migrations/app/` | Stops at `0003_ai_trace.sql` |
| `migrations/project/` | Stops at `0016_callsheets_budget.sql` |
| `crates/openframe-ai/` | Every source file differs from Windows. The Mac copy carries the Metal engine placement of ADR-0033 and the macOS runtime lock entry `llama-cpp-b11259-macos-arm64-metal` |
| Mac-only crates | `openframe-ffi` (UniFFI: one generic `invoke(op, args_json)`, an `EventListener` callback that receives every app event as the Windows JSON, `cancel_task`, `power_event`), `openframe-gold-project` |
| Swift AI screens | `apps/macos/Sources/OpenFrameUI/AI/`: `AIAssistantModel`, `AIConversationView`, `AIDownloadCard`, `AIDownloadModel`, `AIHealthModel`, `AILogic`, `AIPanelView`, `AIProposedChangesView` |
| Swift settings | `apps/macos/Sources/OpenFrameUI/Settings/AISettingsView.swift` |
| Mac evidence | `docs/release/evidence/mac/` (the M8 eval, the Metal vs processor timings) |

### 1.3 What Windows has that the Mac lacks

ADR-0037 and slices S0 to S8 plus fixes. Windows commits, oldest first:

- `e2ee5258` expectation flags
- `a976eb7f` fixtures
- `21fc0c3f` gold sets
- `0d13cdb9` shared eval project
- `0690d6d1` S2 coverage
- `25ddc700` S3 index, resolver, focus
- `d5ab5c23` S4 answer type
- `949c5b52` S5 facts, S6 memory
- `d9c187cb` S7 write and check, S8 speed
- `d229418d` eval cleanup
- `a37e62e6` fix round
- `1780bdc5`, `283e9e32`, `e2ea5867`, `b0b57081` assistant fixes

Read these in `windows/` before you start:

- **How a message runs now:** `docs/engineering/ai-answer-paths.md`.
- **Design and every measurement:** `docs/specs/ai-conversational-harness.md`.
  - Section 4: what stays code.
  - Sections 5.1 to 5.7: one turn.
  - Section 7.5: bars.
  - Sections 13 and 14.
- **Decisions:** `docs/adr/ADR-0037-conversational-assistant-focus-and-dossiers.md`.
- **Owner decisions of 2026-10-26:** `docs/implementation/owner-decisions.md`.
  - Long-term memory is on, visible and deletable.
  - Progress lines only, no token streaming.
  - DeepSeek Harness is not used.
  - Laya is not used.
  - The model stays Gemma 3 1B.

### 1.4 Windows measurements to match

Low Tide gold sets, 274 cases, after S8, in `windows/docs/release/evidence/ai-conversational-after-s8-2026-10-26/`:

| Measure | Windows result |
|---|---|
| Answer facts | 64.0% |
| Required facts | 68.3% |
| Length class | 93.2% |
| No re-ask | 97.2% |
| Repetition | 0.4% |
| Answer group | 90.4% |
| Median time | 4.4 s |
| p95 time | 22.9 s |
| Prompt cache reuse | 64.9% |

Answer type on 40 unseen messages: 87.5% exact, 92.5% by group.

Correct abstention was 15% before the fix round. The fix round changed the wording. No full re-run after the fix round exists on Windows yet.

## 2. Rules of engagement (non-negotiable)

1. **Start of session.** Read, in order and nothing more:
   - `mac/CLAUDE.md`
   - `mac/docs/implementation/handoff.md`
   - `mac/docs/roadmap/chapter-1b-mac-phases.md` section 10 (phase M8)
   - this file

   State the work item, the components it touches and how it will be verified.
2. **Git.**
   - Branch from the Mac line: `git switch -c mac/ai-conversational-port`.
   - Commit locally after each phase, with plain prose messages.
   - Never `git push`, never open or merge a pull request, never `git pull` over local work, never force anything, unless the owner says so in this session.
   - Never build the `.dmg` or any installer unless the owner says so in this session.
3. **Measure first (ADR-0028).** Phase 1 saves a Mac baseline before any AI code changes. Every later phase ends with a measured result written to evidence. Never claim a number you did not measure. Say what ran, on what data, how many times.
4. **Deterministic judging only.** At run time only code judges answers, never a model. At most three passes. A pass-1 answer that passes every check is shown at once.
5. **One copy of each idea.** Port the Windows code; do not re-invent it in Swift. Swift is a screen over the core. No AI logic, check, prompt or rule may live in Swift.
6. **Do not port the retired five-lane code.** That code is `lanes.rs`, `chat.rs`, `recipe.rs`, `briefing.rs`, `verify.rs`, `understand.rs`, `intent.rs`, `intents.json`, the regex answer routes, `follow_up`, `just_the_number`, and the `clarify`, `summarize_scope`, `character_profile` and `answer_product_question` tools. It no longer exists on Windows; the 2026-10-24 port list is superseded for the AI.
7. **Frozen migrations.**
   - Copy migration files byte for byte (`git show` or `cp` from `windows/`, never retyped, never re-saved by an editor).
   - Keep the Windows numbers.
   - Record each one with `node scripts/check-migrations.mjs --record` and verify with `node scripts/check-migrations.mjs`.
   - Never edit a frozen file. If a hash differs only by line endings, stop and find the normalisation rule in the script; do not hand-edit hashes.
8. **Security standard.**
   - Least privilege; no silent network use; the engine stays a helper process with the per-launch hash check (ADR-0033).
   - The privacy guards of the facts stage must hold on the Mac exactly as on Windows: other users' private notes never; contact data only for Owner and Editor; planted instructions marked as data; no file paths.
   - Run `security-guidance` and `ecc:security-review` on the facts guards, the memory saving rules and every new FFI or Swift entry point.
9. **Writing standard.** No em dash anywhere; plain words (`docs/engineering/35-writing-standard.md`). `npm run check:writing` must not go up.
10. **Build hygiene.**
    - On a 16 GB Mac use `CARGO_BUILD_JOBS=4` and one heavy cargo build at a time; use `RUSTC_WRAPPER=` if `sccache` is not installed.
    - Never create a `target/` inside a worktree. Investigate when temporary data passes about 10 GB.
    - Never hold 50 GB without the owner's approval.
11. **Fact gates.** If a tool gate asks for facts before an edit, give them and retry.
12. **Delegation.** Sonnet subagents write code one at a time. Opus plans, reviews and verifies. Parallel agents only with a written file-ownership map, never two writers on one file.

## 3. Phase 1: Mac baseline (no AI code change)

1. **Build and copy the eval.** Build the Mac eval exactly as it is today (`cargo build --release -p openframe-ai-eval --bin ai-eval`) and copy the binary aside.
2. **Bring over the test data, unchanged, from `windows/`:**
   - the Low Tide fixtures: `crates/openframe-ai-eval/gold/fixtures/`;
   - the gold sets `conversation.lowtide.en.jsonl` and `tools.lowtide.en.jsonl`;
   - `turn-sets/`;
   - the fixture loader `src/fixture.rs` and its tests.

   Do this only if the current Mac eval can load them without the rest of the Windows code. If it cannot, record "Mac baseline on the old 4 sample sets only" and run the existing Mac gold sets instead.
3. **Run the baseline.** Point `OPENFRAME_AI_BUNDLE_DIR` at the Mac bundle and `OPENFRAME_AI_ASSETS_DIR` at the pinned models (`packaging/ai/fetch-macos-runtime.sh` lays out the runtime). The eval falls back to the processor when Metal is off. Run on Metal:

   ```
   ai-eval --gold crates/openframe-ai-eval/gold --out <folder> --only lt-
   ```
4. **Save the evidence.** Save `report.md`, `summary.json` and `results.jsonl` to `docs/release/evidence/mac/ai-conversational-baseline-<date>/` with a README: build, commit, engine place, wall time.
5. **Exit:** baseline saved and committed.

## 4. Phase 2: bring the shared Rust core level with Windows

The AI code depends on non-AI core changes, for example:

- `store.rs`: the content-commit counter.
- `core.rs`: `ProjectSession.ai_index`, warm-up, and the library setup of ADR-0038.
- `modules/exports::money`: Indian grouping.
- `production/privacy.rs`.
- `writing/proofing.rs` and `writing/shortcuts.rs`.
- `appearance`.

So do not copy AI files alone. Bring the whole shared core level, then put the Mac-only parts back.

1. **Find the merge base.**
   ```
   git merge-base origin/mac/m1-toolchain origin/backup/main-before-two-folders
   ```
   This is the W9 commit both lines grew from. If those branches are not present, use the oldest commit of `mac/` history and say which one you used.
2. **Build the Mac-only change list.** For every file under `crates/` and `migrations/`, diff the Mac copy against the base. These edits must survive:
   - the Metal placement in `crates/openframe-ai/` (ADR-0033);
   - the macOS runtime lock entry in `packaging/ai/ai-assets.lock.json`;
   - macOS code in `crates/openframe-platform/`;
   - the two Mac-only crates `openframe-ffi` and `openframe-gold-project`;
   - StoreKit and entitlement changes;
   - anything else the diff shows.

   Write the list to `docs/release/evidence/mac/ai-port-mac-only-changes.md` before merging.
3. **Merge three ways, file by file.** For each shared crate (`openframe-application`, `openframe-ai`, `openframe-ai-checks`, `openframe-ai-eval`, `openframe-domain`, `openframe-entitlement`, `openframe-import-export`, `openframe-persistence`, `openframe-platform`, `openframe-project-format`, `openframe-search`, `openframe-security`, `openframe-test-support`), merge base, Mac and Windows with `git merge-file` or an equivalent.
   - Windows wins for the AI.
   - Mac wins for its listed Mac-only edits.
   - A real conflict is resolved by hand, and both versions are recorded in the change list file.
4. **Engine settings on Metal.** In `crates/openframe-ai`, keep the Metal flags of ADR-0033 (`-ngl 999 --device MTL0`, the fall back to processor, the remembered failure) and add the Windows S8 settings:
   - `cache_prompt: true` on every request (`build_body`);
   - `--parallel 1`;
   - `--cache-ram 512` (`PROMPT_STORE_MB`);
   - the answer slot API (`.slot(SLOT_ANSWER)`);
   - reading `timings.cache_n` into the trace's `cached_tokens`.

   Check every flag with the Mac `llama-server --help` of release `b11259`. Keep `START_ENGINE = false` for the early engine start unless Phase 6 measures it shorter on the Mac. Keep the embedding engine on the processor (ADR-0029 decision 7).
5. **Migrations.** Copy every Windows migration the Mac lacks, byte for byte, in number order.
   - App: `0004_ai_trace_cache`, `0005_ai_trace_lane`, `0006_ai_trace_intent` (unused columns, kept because shipped migrations are frozen), `0007_project_cover`, `0008_ai_trace_turn`, `0009_ai_trace_dossier`.
   - Project: `0017_story_page`, `0018_ai_memory`, `0019_story_page_format`.

   The non-AI ones (`0007`, `0017`, `0019`) come too: projects and packages move between Windows and Mac, so the numbers and schemas must be identical on both. Record them in `migrations/frozen.json` with the script.
6. **Library setup (ADR-0038).** The Windows core can ask the shell for a first-start library choice. The Mac has no setup screen yet (Mac track 1b blocker). Make the Mac FFI `CoreConfig` pass the value that keeps the old behaviour (`library_setup: None`, which sets the library up in the folder that holds `projects_dir`). Confirm with a test that the Mac app still starts and opens a project with no setup screen. Do not build the setup screen in this work.
7. **Other behaviour changes.** For every other non-AI behaviour change that comes with the core: if an operation the Mac shell already calls now behaves differently (arguments, defaults, errors), list it. Then adapt the Swift call or keep the old default, and test it. The call sheet order, story page, Beat Board layouts and vault fixes are backend-only here; their Mac screens are not part of this work.
8. **Regenerate the Swift interface.** Run `npm run bindings` (ts-rs export), then `npm run mac:types`, then `apps/macos/scripts/build-core.sh` (XCFramework, Swift interface, generated types). Never edit `OpenFrameCore/Generated`.
9. **Verify:**
   - `RUSTC_WRAPPER= cargo test --workspace --exclude openframe-desktop` (expect the Windows counts plus the Mac-only tests);
   - `cargo clippy --workspace --all-targets --exclude openframe-desktop -- -D warnings`;
   - `cargo fmt --all -- --check`;
   - `node scripts/check-migrations.mjs`;
   - `npm run check:writing`;
   - `node scripts/check-links.mjs`;
   - `apps/macos/scripts/test.sh`.
10. **Exit:** the core is level, all green, committed.

## 5. Phase 3: confirm the AI path in the Mac core

There is no new code in this phase, only checks. The ported core must already contain:

| Stage | Files in `crates/openframe-application/src/modules/ai/` |
|---|---|
| Memory | `memory/` (`mod`, `rules`, `long`, `short`, `tests`) |
| Index, resolver, focus | `index/` (`mod`, `build`, `resolve`, `focus`, `tests`) |
| Answer type | `turn.rs`, `turn/` (`rules`, `neighbour`, `model`, `domains`, `warm`, `tests`), `turn_examples.jsonl` |
| Facts and guards | `dossier.rs`, `dossier/` (`registry`, `ctx`, `guard`, `names`, `words`, `fetch/` with 11 fetchers, `tests`) |
| Write, check, fallback | `answer.rs`, `answer/` (`prompt`, `check`, `fallback`, `tests`), `text.rs` |
| Navigate and change | `action.rs`, `action/tests.rs`, `catalog/`, `change_set.rs` |
| Warm-up | `intelligence/warm.rs`, `turn/warm.rs` |
| Store and trace | `trace.rs`, `records.rs`, `conversation_export.rs`, `cancel.rs` |
| Operations | `ai.ask`, `ai.cancel`, `ai.history`, `ai.export_conversation`, `ai.memory_list`, `ai.memory_delete`, `ai.change_set.*`, `ai.status`, `ai.health`, `ai.download.*`, `ai.graphics.set`, `ai.index_status`, `ai.index_rebuild`, `ai.stop_runtime`, `ai.diagnostics`, `ai.export_report` |

Run `cargo test -p openframe-application --test ai` and `--lib modules::ai`. The integration tests cover:

- several screenplays answered without asking;
- an angry message getting a brief calm reply with no data;
- a missing wardrobe reported as missing with the fixed sentence ("I could not find X. I looked in A and B.");
- navigate opening the scene;
- a change making a proposal;
- another user's private note never reaching the prompt;
- a Viewer never getting a contact.

All must pass on the Mac. **Exit:** green, and the counts written in the commit message.

## 6. Phase 4: the Swift screens

Only screens. Every behaviour comes from the core through `invoke` and the event stream. Match the Windows behaviour; keep the Mac look (owner decision 2026-10-19: the two designs stay apart). Windows references are under `windows/apps/desktop/src/`.

1. **Assistant panel.** Files: `AIPanelView`, `AIConversationView`, `AIAssistantModel`, `AILogic`. Windows reference: `ai/Conversation.tsx`, `ai/answerText.tsx`.
   - Call `ai.ask` with `text`, `scope`, `conversationId`. Keep one conversation id per conversation and reuse the id the core returns. This is what makes focus, memory and "no re-ask" work: a new id per message breaks all three.
   - Render `content` with the Windows answer-text rules (paragraphs on blank lines, "- " bullets, "1. " numbers, no HTML).
   - Results whose `itemsAreAnswer` is true show their items under the answer. Otherwise the items and `provenance` sit behind a "See sources" disclosure.
   - Show `details` only for kinds `Failed`, `Unavailable`, `Clarify`, `Private`, as Windows does.
   - `nav` opens the target screen through the Mac router (scene, workspace, object). Map every `AiNavTarget` the core can emit, including app screens (project files, budget, call sheets, schedule, tasks) added in the fix round.
   - `changeSet` uses the existing `AIProposedChangesView` (accept, reject, recheck). Nothing applies without the person.
   - The one question the core may ask ("Which screenplay do you mean: A or B?") is shown as a normal answer; the reply goes back in the same conversation.
   - `ai.history` with the conversation id loads the earlier turns on open.
2. **Progress and Cancel.** Subscribe through the existing `EventListener` to the `ai.progress` event. Payload fields: `conversationId`, `label`, `step`, `total`. Show the label lines for the running conversation only. Post an accessibility announcement for each new line. A Cancel button calls `ai.cancel` with the conversation id. No token streaming.
3. **Save conversation for review.** A button calls `ai.export_conversation` with the conversation id and a path chosen in `NSSavePanel`. Nothing is saved on its own.
4. **Assistant memory in Settings.** File: `Settings/AISettingsView.swift`. Windows reference: `workspaces/settings/AssistantMemory.tsx` (Settings > Offline AI > Assistant memory).
   - List `ai.memory_list` entries with text and date.
   - Each entry has a Delete button with the accessible label "Delete: <text>", calling `ai.memory_delete` with the id, then refreshing the list.
   - Empty state.
   - One line that says entries stay on this Mac and are saved only when the person states a fact or confirms an answer.
5. **Accessibility.** VoiceOver labels on every control, keyboard reachable, Dynamic Type, contrast pairs in Light and Dark. Use `docs/engineering/34-ux-laws-checklist.md`.
6. **Tests.**
   - Swift tests for the answer-text parser, the nav mapping, progress filtering by conversation, and the memory list delete flow.
   - Mac end-to-end scripts under `apps/macos/E2E` for: ask, progress, Cancel, Save, sources, a navigate answer, a change proposal accepted and rejected, a memory entry deleted.
   - Run `apps/macos/scripts/test.sh` and the E2E runner.
7. **Parity and HTML replica.** Run `npm run mac:parity`. Update `MacHTML/` screens for the panel and the memory card in Light and Dark if that folder is kept current.
8. **Exit:** screens built, tests green, committed.

## 7. Phase 5: measure on the Mac

1. **Full run.** Release build, Metal on, deterministic sampling, quiet machine (no other build running):
   ```
   ai-eval --gold crates/openframe-ai-eval/gold --out <folder> --only lt-
   ```
   Then `--turn-only` and `--dossier-only`. The latency section is written on every run (`--warmup on|off`, `--latency-from <folder>`).
2. **Processor run.** Repeat the full run with Graphics card set to Off (processor only), so the Mac fallback path is measured too.
3. **Compare against the Mac baseline and the Windows numbers** in one table: answer facts, required facts, length class, no re-ask, no navigate on explain, no data dump, repetition, abstention, answer type exact and group, tool and arguments, end state, median, p95, first question, cache reuse.
4. **Bars (spec 7.5).** Say plainly which are met and which are not. Do not tune the answer-type rules on `turn-sets/fresh-40.jsonl`: it is the held-out set. Write a new set if you need one.
5. **A failed bar.** Find the cause in the trace (answer type, dossier size, passes, offenders, fallback, cached tokens, stage times). If the cause is shared code, fix it in `mac/` and write the same fix down for `windows/` (ADR-0034: a core change made for one platform is made in the other in the same piece of work; record it in both progress logs). Re-run.
6. **Save the evidence.** Save to `docs/release/evidence/mac/ai-conversational-after-port-<date>/` with a README holding the table and the limits (one machine, my labels, synthetic fixture).
7. **Exit:** bars met on Apple silicon with Metal and the default provider, or the open ones listed with their causes.

## 8. Phase 6: speed on the Mac

1. **Early start.** Measure first-question time with the early engine start on and off (`AiManager::prime`, `START_ENGINE`), five pairs on a quiet machine. Turn it on only if it is shorter.
2. **Memory.** Check engine memory on Metal with the 512 MB prompt store against the RAM checks of the placement code (unified memory: the model plus margin plus reserve).
3. **Power.** Check that warm-up skips when the AI is off and that `power_event(sleeping)` stops background work.
4. **Exit:** numbers in the evidence README; the setting chosen and why.

## 9. Phase 7: close-out (30 minutes, no more)

1. **Docs in `mac/`.**
   - Update `docs/engineering/ai-answer-paths.md` (copy the Windows page, add one Mac section: Metal, helper process, timings).
   - Update `docs/specs/ai-conversational-harness.md` (copy from Windows, add the Mac results paragraph).
   - Copy ADR-0037 and add it to `docs/adr/README.md`.
   - Copy the owner-decision rows of 2026-10-26.
   - Mark the AI section of the Mac copy of `mac-port-list-2026-10-24.md` as superseded by this file.
   - Move the Mac copies of any five-lane documents to history, with a "Superseded by" line, and record the move in the reconciliation log.
   - Run `node scripts/check-links.mjs`.
2. **Status.**
   - Move the Mac feature status rows only with Mac evidence (`node scripts/check-feature-status.mjs`).
   - Tick Mac plan boxes only with evidence written after them `(ev: ...)`.
   - Progress numbers come from `npm run plan:status`; never type a count.
3. **Handoff and log.**
   - Rewrite `docs/implementation/handoff.md`: state, next three items, blockers, do-nots.
   - Add a progress-log entry at the top with the dashboard from `npm run plan:status`.
4. **Graphify.** Run `graphify update .` inside `mac/`.
5. **Dead code.** Delete any Mac code the port made dead (old AI helpers, old Swift parsing of lane-specific text), with its tests. Never keep an old way and a new way side by side.
6. **Commit locally.** Do not push.
7. **Report to the owner** in plain, short words:
   - progress by phase as a table;
   - the Mac numbers against Windows;
   - what is blocked on the owner (push, `.dmg` build, bars approval, gold sets approval);
   - the next two or three items;
   - plainly what was not run or not verified.

## 10. Out of scope (do not start)

- The Apple on-device model provider (phase M8 item); it is a separate work item after this port.
- The library setup screen and security-scoped bookmarks (Mac track 1b blocker).
- Mac screens for the story page, Beat Board layouts, call sheet order and vault fixes.
- App Sandbox signing, notarization and the store build (Chapter 3).
- Any Windows change.
- Any push, merge or installer.

## 11. If something blocks you

Finish everything that does not depend on the blocked part. Write what is blocked, what you ruled out and what would
settle it in the handoff, and stop. Ask the owner only for decisions that are theirs, in very simple language. Never
decide an open item of `owner-decisions.md`.
