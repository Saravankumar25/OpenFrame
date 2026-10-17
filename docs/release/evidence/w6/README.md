# Windows phase W6 evidence: the AI harness, first round (2026-10-17)

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

## What is left in W6

The remaining W6 items (recipes for the bigger jobs such as summarise an act or a continuity check, memory v1 and its
screen, a language-neutral stop-word list in retrieval, the poisoned-retrieval corpus and a case per feature) are not done.
French, Japanese, Korean and Chinese retrieval stays at 4 or 5 of 8 with either search model.
