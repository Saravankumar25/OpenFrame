# Mac port list: the 2026-10-24 upgrade

> Written 2026-10-24. Owner decision: Windows first; the Mac work for this upgrade is a list, not a parallel Mac build
> ([owner-decisions.md](../implementation/owner-decisions.md)). This is the list the Mac track
> ([Chapter 1b](chapter-1b-mac-phases.md)) works from. The Windows side is the source of truth. Nothing in the Mac tree
> was changed to make this list.

## How to read this list

- **The Mac tree has its own copy of every crate.** `OpenFrameMac/crates/` holds `openframe-application`,
  `openframe-import-export`, `openframe-ai`, `openframe-ffi` and the rest, as separate files. The Windows changes do not
  reach the Mac by themselves. Each Rust item below is a copy of the Windows file (or a merge when the Mac copy has its own
  edits, such as the Metal engine placement of ADR-0033), then the Mac tests.
- **State of the Mac copy on 2026-10-24** (listed, not changed): `migrations/app` stops at `0003_ai_trace.sql`;
  `crates/openframe-application/src/modules/ai/` has none of the files added below; `crates/openframe-import-export/src/`
  has `report.rs` but no `report/` folder (the Windows tree has `report/design.rs`), so its production PDF renderer is
  older than the Windows one even before this upgrade.
- **Windows range:** commits after `17114505` (merge of WindowsHTML) on `upgrade/harness-exports-screenplay`. Work still
  landing on that branch (the production PDF renderer, section 6) is added here when it lands.
- **Order:** section 1 first (the other sections need its migrations and operations), then the sections in any order.
- The generated Swift types in `OpenFrameCore/Generated` are regenerated from the Rust DTOs after section 1, not edited.

## 1. The Rust core (copy, then test on the Mac)

> **Superseded for the AI on 2026-10-27** by [mac-ai-harness-port-orders.md](mac-ai-harness-port-orders.md): the AI lanes,
> chat, recipe, verifier and intent rows below were retired on Windows (slice S7) and must not be ported. The non-AI rows
> stay.

| Item | Windows files | Mac action |
|---|---|---|
| AI lanes and routing | `crates/openframe-application/src/modules/ai/lanes.rs`, `typo.rs`, `intent.rs`, `intents.json`, `understand.rs` | Add the files; wire in `mod.rs` and `orchestrator.rs` as on Windows |
| Chat lane, briefings, recipe | `ai/chat.rs`, `briefing.rs`, `recipe.rs`, `test_script.rs` | Add the files |
| Code verifiers | `ai/verify.rs`; shared checks in `crates/openframe-ai-checks/src/lib.rs` and its `Cargo.toml` | Add; keep one set of checks for the app and the eval harness |
| Orchestrator, tools, toolbox, commands, records, types, knowledge | `ai/orchestrator.rs`, `tools.rs`, `toolbox.rs`, `commands.rs`, `records.rs`, `types.rs`, `knowledge.md` | Merge. Keep the Mac Metal engine placement (ADR-0033). The Windows default of 4 agent steps and the compact tool signatures come with `orchestrator.rs` |
| Progress and Cancel | `ai/cancel.rs` (`ai.cancel`), the `progress` module event in `orchestrator.rs` | Add the operation; the event reaches Swift through the FFI event channel (section 2) |
| Save conversation for review | `ai/conversation_export.rs` (`ai.export_conversation`) | Add the operation; the Save panel is Swift (section 2) |
| Trace columns | `ai/trace.rs`; migrations `migrations/app/0004_ai_trace_cache.sql`, `0005_ai_trace_lane.sql`, `0006_ai_trace_intent.sql`; entries in `migrations/frozen.json` | Copy the three migrations unchanged and record them (`node scripts/check-migrations.mjs --record`); never edit a frozen file |
| Engine client sampling and cached tokens | `crates/openframe-ai/src/client.rs` | Merge (top_p, top_k, `cached_tokens`) |
| Schema and core wiring | `crates/openframe-application/src/schema.rs`, `core.rs`, `modules/mod.rs`, `Cargo.toml` | Merge |
| Project cover photo | `modules/project_cover.rs`, `modules/project.rs`, migration `migrations/app/0007_project_cover.sql` | Copy; the covers folder is `<app data>/covers/assets` |
| Screenplay layout, standard v5 | `crates/openframe-import-export/src/layout.rs`, `rules.rs` (36 pt before scene headings, transition and shot keep-with-next, `(MORE)` and `CUE (CONT'D)` at every split of one speech, title page block gutter and a second title page for a surplus) | Copy both files; layout tests and goldens change by the new gaps only |
| `number_first_page` page setting | `modules/screenplay/setup.rs` (default on) | Copy; saved scripts print as before |
| Sides: selected cast | `modules/exports/sides.rs`, `modules/schedule/docs.rs`, `schedule/source.rs` (cover wraps and continues, dual dialogue kept, only the scenes that remain) | Copy |
| AI evaluation | `crates/openframe-ai-eval/` (`src/lib.rs`, `src/main.rs`, `gold/conversation.en.jsonl`, `gold/paraphrase.jsonl`, `gold/README.md`), `crates/openframe-test-support/src/embedder.rs` | Copy; run the Mac baseline before any AI change (ADR-0028) |

## 2. The FFI and the Swift core (`OpenFrameMac/apps/macos`, `crates/openframe-ffi`)

| Item | Windows reference | Mac action |
|---|---|---|
| Module events to Swift | Windows sends `ai.progress` as a module event (`apps/desktop/src-tauri/src/lib.rs` carries the Tauri side) | Forward the same event through `crates/openframe-ffi/src/shell.rs` to `OpenFrameCore/Core.swift`; no new callback type if the existing event stream can carry it |
| Regenerated types | `apps/desktop/src/ipc/generated/` (cover, page setup, Sides, story page DTOs and others) | Regenerate the Swift types in `OpenFrameCore/Generated` |
| Picture access for covers | Tauri asset scope allows `<app data>/covers/assets` | On the Mac the app reads its own container; copy the chosen file in (App Sandbox build: a user-selected read grant, then copy) |
| Packaged rules file | `apps/desktop/src/workspaces/screenplay/editor/elementRules.json` (generated) | Replace `elementRules.json` in the `OpenFrameUI` resources; `SharedScreenplayRules.swift` reads it |

## 3. SwiftUI screens (`OpenFrameMac/apps/macos/Sources/OpenFrameUI`)

| Item | Windows reference | Mac file to change |
|---|---|---|
| Progress line and Cancel in the AI panel | `apps/desktop/src/ai/Conversation.tsx` (`useProgress`, Cancel button, polite live region) | `AI/AIConversationView.swift`, `AIAssistantModel.swift` (post an accessibility announcement for each new line) |
| Answer text style | `apps/desktop/src/ai/answerText.tsx`: paragraphs on blank lines, "- " bullets, "1. " numbers, bold "Idea:" label, "Used N of N scenes" under a whole-screenplay answer, no HTML | `AI/AILogic.swift` (parse), `AIConversationView.swift` (draw) |
| Save conversation for review | AI panel Save button, `ai.export_conversation` | `AIPanelView.swift` with `NSSavePanel`; nothing saved on its own |
| `number_first_page` in page setup | `workspaces/screenplay/pagesetup.tsx` (checkbox) | `Screenplay/Panels/PageSetupSheet.swift`, `PanelState.swift` |
| Dark page / Light page choice | `ScreenplayView.tsx`, `util.ts`, `screenplay.css` (page colour tokens; screen only, never the PDF or print) | `Screenplay/ScreenplayWorkspace.swift` View menu, `Screenplay/Editor/ScreenplayTextView.swift` colours; the choice is saved with the writer's layout for the project and user |
| Relationship maps | `workspaces/story/Characters.tsx`, `relationshipLayout.ts` (one character's map up to eight in a ring then two columns; a separate whole-film map in layers; boxed names, arrows, labels clear of boxes; names open a character's map) | `Story/StoryCharacters.swift`, `StoryLogic.swift` (port the layout function with its tests) |
| Side panel stays in view | `workspaces/production/index.tsx` (a selected low row keeps its side panel in view) | `Schedule/ScheduleDrawers.swift`, `CallSheets/CallSheetsWorkspace.swift` and the other drawers; check each |
| Sides: selected cast | `workspaces/production/tabs/sides.tsx` (scene checklist applies the same rule as the backend; scenes without the cast are unticked, disabled, say why; each name shows its scene count; a line says how many scenes are in; own ticks kept) | `CallSheets/SidesView.swift`, `SidesModel.swift` |
| Project cover photo on Home | `app/home/AppHome.tsx` (choose a photo, generated cover as the fallback, clear) | `Home/HomeView.swift` with `NSOpenPanel` |

## 4. Tests to carry over

- The Rust tests that sit next to each new module (`ai/*/tests.rs`, `tests/screenplay_v5.rs` in `openframe-import-export`,
  the Sides and page setup tests in `openframe-application`) move with the files.
- Web tests have a Swift counterpart to write: the answer text parser, the relationship layout, the Sides scene rule and the
  dark page contrast pairs.
- end-to-end: the Mac scripts under `apps/macos/E2E` for the AI panel (progress, Cancel, Save), page setup and Home cover.
  The Windows suite runs once at the end of the whole upgrade (owner decision); the Mac suite follows its own phase.

## 5. Documents to keep in step

[ADR-0035](../adr/ADR-0035-ai-lanes-briefings-and-code-verifiers.md), [ADR-0036](../adr/ADR-0036-production-pdf-design-system-v2.md),
[ai-answer-paths.md](../engineering/ai-answer-paths.md) (the Mac path needs its own note where Metal changes timings),
[32-screenplay-engine-contract.md](../engineering/32-screenplay-engine-contract.md),
[37-platform-boundary.md](../engineering/37-platform-boundary.md) (the Mac copy of the crates diverges here),
and the feature status rows of the Mac track. A feature row moves only with Mac evidence.

## 6. Production PDF renderer v2

The design is [ADR-0036](../adr/ADR-0036-production-pdf-design-system-v2.md) and
[pdf-design-package-v2](../specs/pdf-design-package-v2/README.md). On Windows the renderer slice
(`crates/openframe-import-export/src/report.rs`, `report/design.rs`, `exports/*` builders, the footer logo asset, the
golden PDFs) had not landed when this list was written. The Mac tree has `report.rs` only, with no `report/` folder, so the
port is the whole Windows renderer, not a patch. Port it after the Windows slice passes its golden files, and add the
exact files here at that time. The package notes that the `mac/` renderer files were not compared when the package was
made; compare them first.

## 7. Not part of the port

Windows-only: the Tauri shell code (`apps/desktop/src-tauri`), the WebView2 and Vulkan notes, the `.exe` packaging.
Nothing here asks the Mac app to copy the Windows look; the owner decision of 2026-10-19 keeps the two designs apart.
