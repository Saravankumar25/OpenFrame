# Dossier fetch (spec 5.4, slice S5)

706 conversation cases on the Low Tide fixtures. Recall: how many of the entries a case expects of the answer (each required fact, each answer_has group with any alternative) are in the dossier or in the statements about what is missing. Abstain: the statements name the thing. This measures the fetch, not the writing.

| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown-breakdown | 31 | 100.0% (59/59) | 100.0% (31/31) | none | 4.8 | 217 | 881 |
| call-sheets-call-sheets | 19 | 93.9% (31/33) | 94.7% (18/19) | none | 6.8 | 407 | 496 |
| conv | 24 | 95.9% (71/74) | 87.5% (21/24) | none | 9.7 | 702 | 973 |
| files-files | 10 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 116 |
| help-product-guide | 20 | 95.1% (39/41) | 90.0% (18/20) | none | 3.0 | 647 | 943 |
| help-settings | 13 | 100.0% (26/26) | 100.0% (13/13) | none | 4.2 | 622 | 847 |
| idea-vault-global | 9 | 100.0% (15/15) | 100.0% (9/9) | none | 5.0 | 183 | 183 |
| idea-vault-this-project | 12 | 100.0% (16/16) | 100.0% (12/12) | none | 5.0 | 197 | 197 |
| neg | 113 | 100.0% (9/9) | 100.0% (8/8) | 28.6% (2/7) | 9.7 | 444 | 1465 |
| notes-tasks-notes | 13 | 100.0% (18/18) | 100.0% (13/13) | none | 4.7 | 217 | 303 |
| notes-tasks-tasks | 14 | 100.0% (23/23) | 100.0% (14/14) | none | 10.0 | 286 | 286 |
| pair | 45 | 93.4% (141/151) | 84.4% (38/45) | none | 14.4 | 619 | 1463 |
| production-budget | 26 | 90.0% (27/30) | 88.5% (23/26) | none | 1.3 | 49 | 98 |
| production-cast-crew | 33 | 97.4% (38/39) | 96.9% (31/32) | 100.0% (1/1) | 5.2 | 162 | 505 |
| production-catalog | 10 | 95.5% (21/22) | 90.0% (9/10) | none | 42.4 | 1246 | 1474 |
| production-daily-view | 14 | 100.0% (33/33) | 100.0% (14/14) | none | 1.0 | 225 | 283 |
| production-day-out-of-days | 15 | 100.0% (31/31) | 100.0% (15/15) | none | 14.0 | 711 | 730 |
| production-locations | 29 | 93.8% (45/48) | 89.7% (26/29) | none | 10.1 | 399 | 504 |
| production-moodboards | 10 | 100.0% (18/18) | 100.0% (10/10) | none | 5.0 | 431 | 431 |
| production-overview | 7 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production-reports | 12 | 100.0% (29/29) | 100.0% (12/12) | none | 6.0 | 826 | 1486 |
| production-schedule | 29 | 94.2% (49/52) | 89.7% (26/29) | none | 5.8 | 423 | 727 |
| production-shot-lists | 19 | 90.0% (27/30) | 84.2% (16/19) | none | 5.1 | 378 | 670 |
| production-sides | 10 | 95.7% (22/23) | 90.0% (9/10) | none | 3.3 | 298 | 602 |
| production-storyboards | 10 | 100.0% (16/16) | 100.0% (10/10) | none | 2.7 | 207 | 391 |
| screenplay-scenes | 31 | 92.5% (62/67) | 83.3% (25/30) | 100.0% (1/1) | 4.8 | 491 | 892 |
| story-beat-board | 18 | 100.0% (35/35) | 100.0% (18/18) | none | 12.7 | 613 | 670 |
| story-characters | 30 | 90.2% (37/41) | 90.0% (27/30) | none | 7.8 | 380 | 670 |
| story-outline | 10 | 100.0% (19/19) | 100.0% (10/10) | none | 12.9 | 641 | 846 |
| story-timeline | 14 | 100.0% (28/28) | 100.0% (14/14) | none | 7.1 | 260 | 256 |
| triple | 5 | 100.0% (17/17) | 100.0% (5/5) | none | 17.8 | 732 | 923 |
| turn | 51 | 91.8% (78/85) | 89.6% (43/48) | none | 6.8 | 333 | 819 |
| all | 706 | 94.9% (1092/1151) | 91.9% (548/596) | 44.4% (4/9) | 7.9 | 417 | 961 |

## Required-fact recall per leaf (section-scoped cases)

| Leaf or family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown/breakdown | 37 | 100.0% (64/64) | 100.0% (34/34) | none | 5.9 | 258 | 881 |
| call-sheets/call-sheets | 26 | 92.7% (38/41) | 91.3% (21/23) | none | 6.6 | 365 | 496 |
| conv | 7 | 95.7% (22/23) | 85.7% (6/7) | none | 11.3 | 880 | 1157 |
| files/files | 13 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 104 |
| help/product-guide | 26 | 93.2% (41/44) | 87.0% (20/23) | none | 2.8 | 636 | 943 |
| help/settings | 16 | 100.0% (26/26) | 100.0% (13/13) | none | 4.1 | 638 | 847 |
| idea-vault/global | 12 | 100.0% (15/15) | 100.0% (9/9) | none | 5.0 | 183 | 183 |
| idea-vault/this-project | 17 | 100.0% (18/18) | 100.0% (14/14) | none | 5.0 | 197 | 197 |
| neg | 14 | none | none | none | 21.4 | 868 | 1494 |
| notes-tasks/notes | 16 | 100.0% (18/18) | 100.0% (13/13) | none | 4.3 | 194 | 303 |
| notes-tasks/tasks | 21 | 100.0% (27/27) | 100.0% (17/17) | 0.0% (0/1) | 10.0 | 286 | 286 |
| pair | 45 | 93.4% (141/151) | 84.4% (38/45) | none | 14.4 | 619 | 1463 |
| production/budget | 36 | 91.4% (32/35) | 90.3% (28/31) | 0.0% (0/1) | 1.2 | 47 | 77 |
| production/cast-crew | 41 | 93.2% (41/44) | 94.4% (34/36) | 100.0% (2/2) | 5.1 | 160 | 536 |
| production/catalog | 13 | 95.5% (21/22) | 90.0% (9/10) | none | 43.7 | 1297 | 1468 |
| production/daily-view | 20 | 100.0% (39/39) | 100.0% (17/17) | none | 2.0 | 460 | 1481 |
| production/day-out-of-days | 20 | 100.0% (43/43) | 100.0% (17/17) | none | 14.0 | 706 | 730 |
| production/locations | 33 | 93.8% (45/48) | 89.7% (26/29) | 0.0% (0/1) | 11.4 | 408 | 503 |
| production/moodboards | 13 | 100.0% (18/18) | 100.0% (10/10) | none | 4.2 | 363 | 431 |
| production/overview | 10 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production/reports | 15 | 100.0% (29/29) | 100.0% (12/12) | none | 6.0 | 697 | 1486 |
| production/schedule | 38 | 93.2% (55/59) | 87.9% (29/33) | 100.0% (1/1) | 5.6 | 391 | 727 |
| production/shot-lists | 26 | 91.4% (32/35) | 87.0% (20/23) | none | 5.0 | 357 | 677 |
| production/sides | 13 | 95.7% (22/23) | 90.0% (9/10) | none | 5.9 | 349 | 602 |
| production/storyboards | 13 | 100.0% (16/16) | 100.0% (10/10) | none | 2.6 | 190 | 391 |
| screenplay/scenes | 44 | 92.6% (87/94) | 81.6% (31/38) | 50.0% (1/2) | 5.0 | 529 | 880 |
| story/beat-board | 25 | 100.0% (43/43) | 100.0% (22/22) | none | 12.6 | 606 | 670 |
| story/characters | 47 | 94.2% (65/69) | 93.0% (40/43) | 0.0% (0/1) | 8.7 | 410 | 670 |
| story/outline | 13 | 100.0% (19/19) | 100.0% (10/10) | none | 12.7 | 634 | 659 |
| story/story-page | 3 | none | none | none | 0.3 | 17 | 51 |
| story/timeline | 19 | 100.0% (33/33) | 100.0% (16/16) | none | 7.1 | 259 | 256 |
| triple | 5 | 100.0% (17/17) | 100.0% (5/5) | none | 17.8 | 732 | 923 |
| turn | 9 | 86.7% (13/15) | 88.9% (8/9) | none | 10.4 | 311 | 992 |
| all sec- cases | 706 | 94.9% (1092/1151) | 91.9% (548/596) | 44.4% (4/9) | 7.9 | 417 | 961 |

## Pointer (section-scoped cases)

Precision: pointers that went to the expected leaf. Recall: expected pointers that fired right. False pointers: cases that expect none and got one. Bars: 90 percent, 90 percent, at most 5 percent.

| Family | Expect a pointer | Pointed | Precision | Recall | False pointers |
|---|---|---|---|---|---|
| breakdown-breakdown | 0 | 0 | none | none | 0.0% (0/31) |
| call-sheets-call-sheets | 0 | 0 | none | none | 0.0% (0/19) |
| conv | 0 | 0 | none | none | 0.0% (0/24) |
| files-files | 0 | 1 | 0.0% (0/1) | none | 10.0% (1/10) |
| help-product-guide | 0 | 0 | none | none | 0.0% (0/20) |
| help-settings | 0 | 0 | none | none | 0.0% (0/13) |
| idea-vault-global | 0 | 0 | none | none | 0.0% (0/9) |
| idea-vault-this-project | 0 | 0 | none | none | 0.0% (0/12) |
| neg | 106 | 73 | 100.0% (73/73) | 68.9% (73/106) | 0.0% (0/7) |
| notes-tasks-notes | 0 | 0 | none | none | 0.0% (0/13) |
| notes-tasks-tasks | 0 | 0 | none | none | 0.0% (0/14) |
| pair | 0 | 0 | none | none | 0.0% (0/45) |
| production-budget | 0 | 0 | none | none | 0.0% (0/26) |
| production-cast-crew | 0 | 0 | none | none | 0.0% (0/33) |
| production-catalog | 0 | 0 | none | none | 0.0% (0/10) |
| production-daily-view | 0 | 0 | none | none | 0.0% (0/14) |
| production-day-out-of-days | 0 | 0 | none | none | 0.0% (0/15) |
| production-locations | 0 | 1 | 0.0% (0/1) | none | 3.4% (1/29) |
| production-moodboards | 0 | 0 | none | none | 0.0% (0/10) |
| production-overview | 0 | 0 | none | none | 0.0% (0/7) |
| production-reports | 0 | 0 | none | none | 0.0% (0/12) |
| production-schedule | 0 | 1 | 0.0% (0/1) | none | 3.4% (1/29) |
| production-shot-lists | 0 | 2 | 0.0% (0/2) | none | 10.5% (2/19) |
| production-sides | 0 | 0 | none | none | 0.0% (0/10) |
| production-storyboards | 0 | 0 | none | none | 0.0% (0/10) |
| screenplay-scenes | 0 | 0 | none | none | 0.0% (0/31) |
| story-beat-board | 0 | 0 | none | none | 0.0% (0/18) |
| story-characters | 0 | 1 | 0.0% (0/1) | none | 3.3% (1/30) |
| story-outline | 0 | 0 | none | none | 0.0% (0/10) |
| story-timeline | 0 | 0 | none | none | 0.0% (0/14) |
| triple | 0 | 0 | none | none | 0.0% (0/5) |
| turn | 3 | 2 | 100.0% (2/2) | 66.7% (2/3) | 0.0% (0/48) |
| all sec- cases | 109 | 81 | 92.6% (75/81) | 68.8% (75/109) | 1.0% (6/597) |

## Narrative checks that are off (false positives)

The expected answer of each section-scoped case (its required facts and first alternatives) run through the checks. A flag means the check would refuse a correct answer. Turn a check on only when its rate is under 5 percent.

| Check | Cases tested | Flagged | False positives |
|---|---|---|---|
| Unsupported relationship word | 596 | 0 | 0.0% (0/596) |
| Unstated mood | 596 | 0 | 0.0% (0/596) |

### Pointer misses and false pointers

| Id | Leaves | Message | Expected | Chose |
|---|---|---|---|---|
| sec-x-neg-003 | idea-vault/global | Which tasks are still open? | notes-tasks/tasks | none |
| sec-x-neg-013 | screenplay/scenes | What is in the Cold open idea? | idea-vault/global | none |
| sec-x-neg-026 | production/shot-lists | What is in the Cold open idea? | idea-vault/global | none |
| sec-x-story-characters-020 | story/characters | who is varkey | none | production/cast-crew |
| sec-x-production-locations-016 | production/locations | Which locations list Fr. Thomas as the contact? | none | production/cast-crew |
| sec-s-production-shot-lists-004 | production/shot-lists | What size is the shot of Jose bailing? | none | story/characters |
| sec-x-production-shot-lists-013 | production/shot-lists | What is the shot for scene 6? | none | production/schedule |
| sec-x-production-schedule-011 | production/schedule | How many scenes are not scheduled? | none | production/cast-crew |
| sec-s-files-files-003 | files/files | How tall is the coast station mast? | none | screenplay/scenes |
| sec-x-turn-011 | production/schedule | What does the Sea safety plan say? | notes-tasks/notes | none |
| sec-x-neg-005 | story/story-page | What is Act Two called? | story/beat-board | none |
| sec-s-neg-004 | story/story-page | What is the quote about the sea in my vault? | idea-vault/this-project | none |
| sec-s-neg-005 | story/beat-board | What lens is used for the insert of the valves? | production/shot-lists | none |
| sec-x-neg-007 | story/outline | Which tasks are still open? | notes-tasks/tasks | none |
| sec-s-neg-007 | story/outline | On which days does Salim work? | production/day-out-of-days | none |
| sec-x-neg-015 | screenplay/scenes | What lens is used for the insert of the valves? | production/shot-lists | none |
| sec-x-neg-016 | breakdown/breakdown | Which locations are shortlisted? | production/locations | none |
| sec-s-neg-013 | production/overview | What is Act Two called? | story/beat-board | none |
| sec-x-neg-017 | production/overview | What is the quote about the sea in my vault? | idea-vault/this-project | none |
| sec-s-neg-014 | production/catalog | What time is the makeup artist called on Shoot Day 4? | call-sheets/call-sheets | none |
| sec-x-neg-023 | production/moodboards | At what wind speed do we call off the sea day? | notes-tasks/notes | none |
| sec-x-neg-029 | production/schedule | What is Act Two called? | story/beat-board | none |
| sec-s-neg-022 | production/schedule | What is the quote about the sea in my vault? | idea-vault/this-project | none |
| sec-s-neg-023 | production/day-out-of-days | What lens is used for the insert of the valves? | production/shot-lists | none |
| sec-s-neg-024 | production/daily-view | At what wind speed do we call off the sea day? | notes-tasks/notes | none |
| sec-x-neg-035 | production/reports | What lens is used for the insert of the valves? | production/shot-lists | none |
| sec-x-neg-041 | notes-tasks/tasks | What does the beach permit letter say? | files/files | none |
| sec-s-neg-038 | help/product-guide | What lens is used for the insert of the valves? | production/shot-lists | none |
| sec-s-neg-039 | help/product-guide | What does the beach permit letter say? | files/files | none |
| sec-x-neg-043 | help/settings | What does the second panel of the Roof transmission storyboard show? | production/storyboards | none |
| sec-x-neg-048 | production/daily-view,production/sides | How is Varghese related to Mariamma? | story/characters | none |
| sec-s-neg-043 | production/catalog,production/reports | Who plays Mariamma? | production/cast-crew | none |
| sec-x-neg-051 | production/catalog,production/reports,production/overview | Who plays Mariamma? | production/cast-crew | none |
| sec-s-neg-046 | production/schedule,production/daily-view,production/sides | Who plays Mariamma? | production/cast-crew | none |
| sec-x-neg-053 | production/schedule | Which day is scene 3 shot, and what does Anjali wear in it? | breakdown/breakdown | none |
| sec-s-neg-048 | production/cast-crew | Who is the director, and what does Act One cover? | story/beat-board | none |
| sec-x-neg-055 | story/characters | What is Babu's role, and who plays him? | production/cast-crew | none |
| sec-x-neg-056 | production/shot-lists | How many shots does scene 41 have, and what are its storyboard panels? | production/storyboards | none |
| sec-x-neg-057 | production/budget | What is planned for Camera, and how many shoot days are there? | production/schedule | none |
| sec-s-neg-049 | call-sheets/call-sheets | What time is the makeup artist called on Shoot Day 4, and what is planned for catering? | production/budget | none |

Build time per dossier: mean 45.3 ms, p95 103.4 ms, max 2091.4 ms.
Chat or change turns that were given facts: 0 (none wanted).
Guard actions across the run: 44.
Dossiers that carry another user's private note: 0 (none allowed).
Facts by kind over the run: Act 100, Appearance 5, Appearances 24, Assumed 37, Breakdown 180, Budget line 25, Budget total 18, Call sheet 111, Call sheet departments 15, Call sheet extras 11, Call sheet place 15, Call sheet times 15, Card 315, Cast 209, Cast calls 15, Catalog item 579, Character 197, Character days 8, Contingency setting 1, Count 685, Crew 57, Crew calls 15, Daily view 54, Day 31, Day out of days 299, File 56, Found 748, Global vault item 48, Guide 74, Idea Vault item 68, Location 389, Mood board 13, No actual yet 3, Outline act 42, Outline card 132, Over plan 3, Production overview 11, Project 16, Project note 31, Relationship 225, Report 101, Scene 29, Scene day 12, Scene location 6, Scene text 18, Scenes 1, Settings guide 45, Shoot period 18, Shot 37, Sides 117, Speaking characters 1, Story day 114, Storyboard 16, Task 176, Together 3, Your private note 18.

## Entries not found

| Id | Type | Leaves | Message | Not found | Facts |
|---|---|---|---|---|---|
| sec-x-story-characters-014 | count | story/characters | How many relationships are recorded? | 12 | 4 |
| sec-x-story-characters-017 | count | story/characters | Which character has the most scenes? | Anjali; 28 | 1 |
| sec-x-story-characters-019 | count | story/characters | How many scenes does Jose have? | 10 | 4 |
| sec-x-screenplay-scenes-007 | count | screenplay/scenes | How many night scenes are there? | 15 | 1 |
| sec-x-screenplay-scenes-012 | describe | screenplay/scenes | Describe scene 35. | storm | 7 |
| sec-x-screenplay-scenes-013 | brief | screenplay/scenes | Brief me on scene 32. | storm | 7 |
| sec-x-screenplay-scenes-014 | story_explanation | screenplay/scenes | Explain the story of LOW TIDE RADIO from beginning to end. | Babu | 5 |
| sec-x-screenplay-scenes-021 | lookup | screenplay/scenes | What does Salim give Anjali in scene 30? | call log | 6 |
| sec-s-production-overview-003 | count | production/overview | How many characters does the production count? | 13 | 1 |
| sec-x-production-catalog-004 | list | production/catalog | Which scenes use the rubber stamp? | 43 | 48 |
| sec-s-production-locations-005 | describe | production/locations | Describe the Coast Radio Station location. | Fr. Thomas | 3 |
| sec-x-production-locations-020 | describe | production/locations | Describe Kadalpuram Fish Landing Centre. | Harbour Clerk office | 6 |
| sec-s-production-locations-006 | describe | production/locations | what is the noise problem at the fish landing centre | Boat engines | 21 |
| sec-s-production-cast-crew-006 | lookup | production/cast-crew | Who plays Sudheesh and what does he need on set? | chair | 1 |
| sec-x-production-shot-lists-010 | count | production/shot-lists | How many close-ups are there? | 3 | 1 |
| sec-s-production-shot-lists-004 | count | production/shot-lists | What size is the shot of Jose bailing? | Close-up | 1 |
| sec-x-production-shot-lists-012 | list | production/shot-lists | Which scenes have three shots? | 41 | 6 |
| sec-x-production-schedule-004 | lookup | production/schedule | Where do we shoot on Day 7? | Coast Radio Station Roof | 6 |
| sec-x-production-schedule-012 | lookup | production/schedule | Which day has the most scenes? | Day 2 | 7 |
| sec-x-production-schedule-016 | lookup | production/schedule | Who is needed at two locations on Shoot Day 12? | Harbour Clerk | 6 |
| sec-s-production-sides-006 | brief | production/sides | In the Day 14 sides, what happens to the boat? | engine | 2 |
| sec-x-production-budget-007 | compare | production/budget | Which line has the largest planned amount? | Lead cast | 1 |
| sec-x-production-budget-019 | lookup | production/budget | What is the total planned for the Cast category? | 6200000 | 2 |
| sec-x-production-budget-020 | lookup | production/budget | What is the total planned for Equipment? | 2450000 | 2 |
| sec-x-call-sheets-call-sheets-006 | lookup | call-sheets/call-sheets | Which call sheet is for a night shoot? | Shoot Day 6; 16:00 | 6 |
| sec-x-files-files-002 | lookup | files/files | What does the beach permit letter say? | 1 to 14 March 2027; 6 am to 9 pm | 4 |
| sec-s-files-files-002 | list | files/files | What is in the harbour inspection checklist? | life jackets; safety boat; radio check; weather log | 4 |
| sec-s-files-files-003 | lookup | files/files | How tall is the coast station mast? | 42 metres | 4 |
| sec-s-files-files-004 | lookup | files/files | When is low water in March 2027? | 1, 8 and 14 March | 4 |
| sec-x-files-files-004 | lookup | files/files | What lamps does the mast have? | red obstruction lamps | 4 |
| sec-s-files-files-005 | lookup | files/files | Which side of the mast does the feed run on? | east face | 4 |
| sec-s-help-product-guide-004 | help | help/product-guide | What do I press to take back my last change? | Ctrl+Z | 3 |
| sec-x-help-product-guide-012 | help | help/product-guide | How do I search the whole project? | Ctrl+K | 3 |
| sec-s-pair-002 | count | screenplay/scenes,production/cast-crew | How many scenes is Young Anjali in, and what limits does her actor have? | guardian | 2 |
| sec-s-pair-004 | list | screenplay/scenes,story/characters | Which scenes feature the rival of Mariamma? | 24; 28; 29 | 12 |
| sec-s-pair-005 | relation | story/beat-board,screenplay/scenes | Which scene is linked to the card where Babu cuts the power, and who is in it? | Dr. Leela | 22 |
| sec-x-pair-006 | lookup | screenplay/scenes,production/schedule | Which day is the scene where Jose confesses shot, and who else is in it? | Anjali | 3 |
| sec-s-pair-011 | list | production/schedule,production/locations | Which shoot days use a location that is only an idea? | 12; 13 | 23 |
| sec-x-pair-010 | lookup | production/schedule,call-sheets/call-sheets | Is there a call sheet for the day scene 35 is shot? | no call sheet | 8 |
| sec-s-pair-024 | list | production/locations,breakdown/breakdown | What does the roof location require, and which scenes use it? | harness | 43 |
| sec-s-conv-003 | brief | screenplay/scenes | Walk me through scene 32. | storm | 7 |
| sec-s-conv-009 | describe | screenplay/scenes,production/cast-crew | Describe Salim. | Nandan | 7 |
| sec-s-conv-022 | story_explanation | screenplay/scenes | Why does Mariamma not tell Anjali how Varghese died? Explain. | storm | 9 |
| sec-s-turn-004 | list | production/schedule | what else is shot that day? | 30 | 7 |
| sec-x-turn-009 | list | production/locations,production/schedule | and on which days are those? | 12; 13 | 28 |
| sec-x-turn-012 | lookup | call-sheets/call-sheets | what time is the crew called on the first draft? | 05:45 | 6 |
| sec-s-turn-017 | navigate | production/cast-crew | give me their emails | vinod.chandrahas@lowtideradio.example; mini.sebastian@lowtideradio.example | 5 |
| sec-s-turn-020 | help | help/product-guide | How do I get the AI? | Download Straft AI | 0 |

## Abstain cases not covered

| Id | Message | Statements |
|---|---|---|
| sec-x-neg-058 | What is the budget for the helicopter? |  |
| sec-s-neg-050 | Who is in the scene with the lighthouse keeper? |  |
| sec-x-neg-060 | What is the status of the Mumbai studio? |  |
| sec-x-neg-061 | Which task is about the drone permit? |  |
| sec-s-neg-051 | Who is the lighthouse keeper character? |  |
