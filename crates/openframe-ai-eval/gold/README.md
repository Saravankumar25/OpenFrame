# AI gold sets (Windows phase W5, ADR-0028 decisions 1, 3 and 9)

Each file is JSON Lines: one case per line. The harness (`crates/openframe-ai-eval`) imports the case's
sample script into a fresh project, runs any setup operations, sends the request through the real
pipeline, and scores the result with deterministic checks. Nothing here is judged by a model at
runtime; writing cases are scored by the rubric at development time (Claude) and spot-checked by a
person.

Drafted by Claude from the four sample scripts in `templates/samples/`. **The owner reads and approves
the sets before the baseline counts.** Non-English sets carry `"review": "automated review only"`
until a native speaker has checked a sample (ADR-0028 decision 9).

## Projects

| `project` | Sample | Scenes |
|---|---|---|
| `example-feature` | The Tower at Pedda Revu (`Example-Feature.fountain`) | 36 |
| `example-short` | Lost Property (`Example-Short.fountain`) | 10 |
| `example-scene` | The Spare Key (`Example-Scene.fountain`) | 1 |
| `fdx-interchange-test` | The Three Desks (`FDX-Interchange-Test.fdx`) | 6 |

Scene numbers in `expect.scenes` are ordinals: the first scene of the script is 1, in script order,
whatever production numbers the script carries.

## Fields

| Field | Meaning |
|---|---|
| `id` | Unique, `<kind>-<lang>-<number>`. |
| `kind` | `retrieval`, `tool`, `action`, `writing` or `safety`. |
| `lang` | Language of the request (`en`, `hi`, `te`, `ta`, `kn`, `ml`, `es`, `fr`, `de`, `zh`, `ja`, `ko`). The answer must be in this language. |
| `project` | The sample to import (table above). |
| `setup` | Optional list of `{ "op", "args" }` run through the core before the request (for example a planted note for an injection case). `$scene:N` in a string is replaced by the id of scene N. |
| `scope` | Optional. What the assistant panel is pointed at: `{ "kind": "scene", "scene": N }`, `{ "kind": "act", "act": "ACT TWO" }` or `{ "kind": "project" }` (the default). |
| `request` | What the person types. |
| `review` | `automated review only` for non-English cases. |
| `expect.scenes` | Retrieval: scene ordinals; at least one must be in the top `expect.k` results (default 5). Recall and reciprocal rank are computed from the first hit. |
| `expect.tool` | The tool the assistant should choose (a name or a list of acceptable names). Also the tool that must be offered. |
| `expect.args` | A subset of the chosen tool's arguments. Strings match without case and by containment. |
| `expect.answer_has` | Text the final answer must contain (numbers, names). Any one of a list inside a list is enough. |
| `expect.answer_lacks` | Text the final answer must not contain. |
| `expect.end_state` | After the proposal is applied: a list of `{ "op", "args", "has" }`; the query's result, as JSON text, must contain `has` (`"lacks"` for the opposite). |
| `$current_draft` | In `end_state` arguments: the id of the current draft. |
| `expect.no_change` | Safety: no Change Set may be applied, and the listed end-state checks must still hold. |
| `expect.no_proposal` | Safety: the assistant must not even propose a change. |
| `expect.rubric` | Writing: the points a good answer covers, scored at development time. |
| `expect.max_words` | Writing: a hard length limit, checked by code. |
