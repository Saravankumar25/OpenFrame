# Windows phase W6 evidence: the AI harness (2026-10-17)

Proof level 2 (development computer, ADR-0030). Same laptop, gold sets and release build settings as the
[W5 baseline](../w5/README.md). Every change below was measured with `ai-eval` on all 679 approved gold cases
(deterministic sampling) and kept because its number improved (ADR-0028).

## What changed

1. **Recipes** (`modules/ai/commands.rs`): plain English requests whose tool and arguments are clear become the tool call
   without the model. Open a scene or a workspace; scene and location counts; who speaks; scenes two characters share;
   page count; add to the catalog; create a character; replace text; notes; task; moodboard; schedule; storyboard;
   location. They run through the same toolbox, schema check, permissions and Proposed Changes card. Unit tests include
   near misses that must still reach the model.
2. **Verifier** (`orchestrator.rs` `verify_language`): an answer not in the request's language gets at most two rewrites
   within 60 seconds; deterministic code (`openframe-ai-checks`) accepts a rewrite only in the right language and with every
   number kept, otherwise the first answer stays. Passes and failed checks are written to the AI trace.
3. **Tool picker** (`toolbox.rs` `semantic_bonus`): tools closest in meaning to the request in the multilingual search
   space get a rank bonus on top of the English word score; the picker waits at most 1.5 seconds for it.

## Results ([report.md](report.md), [summary.json](summary.json), [results.jsonl](results.jsonl))

| Measure | W5 baseline | W6 round 1 | Bar |
|---|---|---|---|
| English correct tool and arguments | 40.7 % | **72.0 %** | 90 % |
| English correct end state after apply | 25.3 % | **50.6 %** | 85 % |
| Answer in the request's language, other languages | 0 % | **25 to 69 %** | |
| Right tool offered, other languages | 61.5 % | **77 to 85 %** | |
| Retrieval recall at 5, English / all | 99.4 % / 92.1 % | 99.4 % / 92.1 % | |
| Model calls per English tool case (median) | 2 | 1 | |
| Prompt tokens per English tool case (median) | 6,428 | 2,035 | |

**The bar is not met** (ADR-0028 decision 2: the owner is shown the figures and decides).

## Timing note

The two runs' times do not compare: the baseline ran on the Intel Arc graphics, round 1 on the processor, because the
integrated-graphics memory rule (model plus a 2 GB reserve of free RAM) found about 3.2 GB free that night. Per call round
1 made half the model calls and read a third of the prompt tokens. `ai-eval` now records where the engine ran.

## What is left in W6 (unchanged by rounds 2 and 3)

The remaining W6 items (recipes for the bigger jobs such as summarise an act or a continuity check, memory v1 and its
screen, a language-neutral stop-word list in retrieval, the poisoned-retrieval corpus and a case per feature) are not done.
French, Japanese, Korean and Chinese retrieval stays at 4 or 5 of 8 with either search model.

## Rounds 2 and 3 (2026-10-17, later the same day)

Same laptop, gold sets and release build settings; the engine ran on the processor both times.

1. **Round 2 recipes** (`commands.rs` `recipe_more`): mentions, plain reads, product questions, scene cards, rename,
   scene comments, vault quotes and ideas, project status, time notes, props, new drafts, new scenes and cast. Requests
   that join two jobs, cards placed in an act and long rename targets still go to the model.
2. **Round 3**: a rename of a character written only in the script (no Character record, the usual state of an imported
   script) now renames the screenplay cues instead of stopping; a name found nowhere is still refused (test
   `rename_reaches_screenplay_cues_when_the_character_has_no_record`). New recipes for adding an act, renaming the
   current draft, starting a new draft, a private note and two more scene card phrasings.
3. **Eight gold checks that could never pass** were fixed (requests and expected results unchanged): three comment
   checks and four rename checks called `comment.list` and `screenplay.characters` without the draft id they need, and
   the vault folder check listed items instead of folders. Every other no-argument check passes in at least one case.

| Measure | Round 1 | Round 2 | Round 3 | Bar |
|---|---|---|---|---|
| English correct tool and arguments | 72.0 % | 96.7 % | **96.7 %** (145 of 150) | 90 % |
| English correct end state after apply | 50.6 % | 70.9 % | 83.5 % in the full run; **71 of 80 (88.8 %)** with the rename checks fixed | 85 % |

Round 3 files: [round3/report.md](round3/report.md), [round3/summary.json](round3/summary.json). The full run used the
rename checks before their fix; the ten cases `action-en-050` to `action-en-059` were then rerun with the fixed checks
and all ten pass ([round3/rename-recheck.md](round3/rename-recheck.md)). No code changed between the two runs.

**The English bar is met** (96.7 % tool, 88.8 % end state). The nine end-state misses left: a scene card text with an
apostrophe inside quotes, a sequence with no act named, "three desks as set dressing" read as a note, a comment on the
whole draft, a replace of a word the sample script does not contain (a gold set flaw), a relationship, a schedule that
needs a Production Source draft, "pin a note saying", and "create characters for this scene". Other languages are
measured in the same report.
