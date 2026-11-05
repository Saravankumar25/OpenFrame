# Dossier fetch (spec 5.4, slice S5)

251 conversation cases on the Low Tide fixtures. Recall: how many of the entries a case expects of the answer (each required fact, each answer_has group with any alternative) are in the dossier or in the statements about what is missing. Abstain: the statements name the thing. This measures the fetch, not the writing.

| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| big | 5 | 100.0% (8/8) | 100.0% (5/5) | none | 2.4 | 77 | 256 |
| brain | 11 | 100.0% (12/12) | 100.0% (11/11) | none | 7.5 | 334 | 654 |
| char | 22 | 97.5% (39/40) | 95.5% (21/22) | none | 6.4 | 204 | 361 |
| export | 40 | 98.2% (55/56) | 96.9% (31/32) | none | 7.1 | 251 | 684 |
| help | 11 | 88.9% (16/18) | 81.8% (9/11) | none | 2.2 | 359 | 455 |
| hop | 13 | 89.7% (26/29) | 92.3% (12/13) | none | 10.5 | 410 | 637 |
| hostile | 11 | none | none | none | 0.0 | 0 | 0 |
| lang | 9 | 100.0% (10/10) | 100.0% (8/8) | 100.0% (1/1) | 8.2 | 357 | 1017 |
| missing | 19 | 100.0% (3/3) | 100.0% (2/2) | 100.0% (17/17) | 0.7 | 23 | 51 |
| money | 27 | 94.4% (51/54) | 92.6% (25/27) | none | 3.9 | 172 | 888 |
| prod | 30 | 100.0% (49/49) | 100.0% (29/29) | 100.0% (1/1) | 7.7 | 287 | 871 |
| repair | 11 | 93.8% (15/16) | 90.9% (10/11) | none | 6.5 | 296 | 597 |
| scene | 30 | 91.9% (57/62) | 86.2% (25/29) | 100.0% (1/1) | 5.3 | 222 | 511 |
| xref | 12 | 100.0% (39/39) | 100.0% (12/12) | none | 5.3 | 229 | 451 |
| all | 251 | 96.0% (380/396) | 94.3% (200/212) | 100.0% (20/20) | 5.6 | 229 | 637 |

Build time per dossier: mean 1.9 ms, p95 5.7 ms, max 29.4 ms.
Chat or change turns that were given facts: 0 (none wanted).
Guard actions across the run: 2.
Dossiers that carry another user's private note: 0 (none allowed).
Facts by kind over the run: Act 57, Appearances 9, Assumed 19, Breakdown 87, Budget line 17, Budget total 9, Call sheet 27, Call sheet departments 7, Call sheet extras 6, Call sheet place 7, Call sheet times 8, Card 152, Cast 83, Cast calls 7, Character 121, Character days 3, Contingency setting 1, Count 69, Crew 39, Crew calls 8, Day 35, File 4, Found 62, Guide 24, Idea Vault item 9, Location 82, Mood board 1, Neighbour scene 6, No actual yet 3, Over plan 3, Project 13, Project note 15, Relationship 192, Scene 65, Scene day 22, Scene location 7, Scene text 27, Scenes 1, Schedule warning 31, Screenplay 18, Shoot period 7, Shot 3, Speaking characters 1, Storyboard 1, Task 28, Together 1.

## Entries not found

| Id | Type | Domains | Message | Not found | Facts |
|---|---|---|---|---|---|
| conv-lt-scene-012 | navigate | screenplay,production | AD here. Any safety issue at the scene 41 location? | harness; safety officer | 2 |
| conv-lt-scene-015 | explain | screenplay | explain scene 35 of LOW TIDE RADIO in a paragraph | engine | 12 |
| conv-lt-scene-026 | explain | screenplay | explain scene 22 | silence | 9 |
| conv-lt-scene-028 | lookup | story | how much does Babu offer Anjali for the land | four million | 14 |
| conv-lt-char-004 | lookup | story,people | and who plays them | Thresiamma Joseph | 5 |
| conv-lt-money-001 | help | people,budget,product | lead cast budget | 40,00,000 | 2 |
| conv-lt-money-025 | chat | people | does the director of photography have a clash | Harikrishnan Embran; 2027-03-07 | 0 |
| conv-lt-hop-013 | lookup | schedule | which day do we shoot those | Day 12; Day 10; Day 5 | 5 |
| conv-lt-help-001 | help | product | what is the Parking Lot | ideas | 2 |
| conv-lt-help-010 | compare | story,screenplay | whats the difference between the Beat Board and the Screenplay | separate | 3 |
| conv-lt-repair-010 | count | screenplay,product | only the feature, again | 13 | 4 |
| conv-lt-export-016 | navigate | screenplay | Again bro, you opened scene 41 | 42 | 2 |

## Abstain cases not covered

| Id | Message | Statements |
|---|---|---|
