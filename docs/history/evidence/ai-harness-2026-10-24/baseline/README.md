# AI harness baseline, 2026-10-24

Measured before any assistant behaviour change (ADR-0028 decision 1, harness plan section 9), with the
evaluation binary built from commit 9002719f. Local model Gemma 3 1B Q8_0, search model Multilingual E5
small, graphics placement automatic, development laptop (Intel Core Ultra 5 125H). The CPU was shared
with builds of other work, so times are upper bounds.

| Run | Set | Direct routes | Key result |
|---|---|---|---|
| tool-on | tools.en (160) | on | correct tool and arguments 100.0% |
| tool-off | tools.en (160) | off | correct tool and arguments 27.2% |
| para-on | paraphrase (90, held out) | on | correct tool 20.0%, answer facts 37.8% |
| para-off | paraphrase (90) | off | correct tool 13.3%, answer facts 26.7% |
| conv-on | conversation.en (60) | on | answer facts 42.9%, invented names 13.1%, within length 55.0%, correct abstention 0 of 4, median 39 s |
| conv-off | conversation.en (60) | off | answer facts 42.9%, invented names 13.4%, within length 66.7%, correct abstention 0 of 4 |

Reading: the 100 percent tool score comes from regex routes written against the gold wording. Reworded
requests reach the model, which chooses `retrieve_context` for almost everything; typos also miss the
routes. Conversation answers invent names (an unknown "Marcus" was described as the radio shop owner),
idea requests trigger change proposals, and whole-script questions get one generic paragraph.

Each `*.report.md` is the harness report; `*.summary.json` the numbers. The gold sets are drafted by
Claude and wait for the owner's approval before the numbers count as the official baseline.
