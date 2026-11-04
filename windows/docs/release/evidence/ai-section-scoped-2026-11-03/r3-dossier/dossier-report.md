# Dossier fetch (spec 5.4, slice S5)

706 conversation cases on the Low Tide fixtures. Recall: how many of the entries a case expects of the answer (each required fact, each answer_has group with any alternative) are in the dossier or in the statements about what is missing. Abstain: the statements name the thing. This measures the fetch, not the writing.

| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown-breakdown | 31 | 100.0% (59/59) | 100.0% (31/31) | none | 4.8 | 218 | 881 |
| call-sheets-call-sheets | 19 | 90.9% (30/33) | 89.5% (17/19) | none | 6.8 | 407 | 496 |
| conv | 24 | 95.9% (71/74) | 87.5% (21/24) | none | 9.7 | 702 | 973 |
| files-files | 10 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 116 |
| help-product-guide | 20 | 97.6% (40/41) | 95.0% (19/20) | none | 3.0 | 496 | 654 |
| help-settings | 13 | 88.5% (23/26) | 84.6% (11/13) | none | 3.4 | 452 | 584 |
| idea-vault-global | 9 | 100.0% (15/15) | 100.0% (9/9) | none | 5.0 | 183 | 183 |
| idea-vault-this-project | 12 | 100.0% (16/16) | 100.0% (12/12) | none | 5.0 | 197 | 197 |
| neg | 113 | 100.0% (9/9) | 100.0% (8/8) | 28.6% (2/7) | 9.3 | 406 | 1464 |
| notes-tasks-notes | 13 | 88.9% (16/18) | 84.6% (11/13) | none | 4.3 | 199 | 303 |
| notes-tasks-tasks | 14 | 91.3% (21/23) | 85.7% (12/14) | none | 8.6 | 245 | 286 |
| pair | 45 | 94.0% (142/151) | 84.4% (38/45) | none | 14.3 | 604 | 1463 |
| production-budget | 26 | 90.0% (27/30) | 88.5% (23/26) | none | 1.3 | 49 | 98 |
| production-cast-crew | 33 | 97.4% (38/39) | 96.9% (31/32) | 100.0% (1/1) | 5.2 | 162 | 505 |
| production-catalog | 10 | 100.0% (22/22) | 100.0% (10/10) | none | 42.4 | 1246 | 1474 |
| production-daily-view | 14 | 100.0% (33/33) | 100.0% (14/14) | none | 1.0 | 225 | 283 |
| production-day-out-of-days | 15 | 96.8% (30/31) | 93.3% (14/15) | none | 14.0 | 710 | 730 |
| production-locations | 29 | 93.8% (45/48) | 89.7% (26/29) | none | 10.1 | 399 | 504 |
| production-moodboards | 10 | 100.0% (18/18) | 100.0% (10/10) | none | 5.0 | 431 | 431 |
| production-overview | 7 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production-reports | 12 | 100.0% (29/29) | 100.0% (12/12) | none | 6.0 | 826 | 1486 |
| production-schedule | 29 | 94.2% (49/52) | 89.7% (26/29) | none | 5.8 | 423 | 727 |
| production-shot-lists | 19 | 90.0% (27/30) | 84.2% (16/19) | none | 5.1 | 378 | 670 |
| production-sides | 10 | 95.7% (22/23) | 90.0% (9/10) | none | 3.3 | 298 | 602 |
| production-storyboards | 10 | 87.5% (14/16) | 80.0% (8/10) | none | 2.7 | 195 | 380 |
| screenplay-scenes | 31 | 92.5% (62/67) | 83.3% (25/30) | 100.0% (1/1) | 4.8 | 476 | 892 |
| story-beat-board | 18 | 100.0% (35/35) | 100.0% (18/18) | none | 12.7 | 613 | 670 |
| story-characters | 30 | 90.2% (37/41) | 90.0% (27/30) | none | 7.5 | 366 | 688 |
| story-outline | 10 | 100.0% (19/19) | 100.0% (10/10) | none | 12.9 | 641 | 846 |
| story-timeline | 14 | 100.0% (28/28) | 100.0% (14/14) | none | 7.1 | 260 | 256 |
| triple | 5 | 94.1% (16/17) | 80.0% (4/5) | none | 16.4 | 590 | 923 |
| turn | 51 | 84.7% (72/85) | 87.5% (42/48) | none | 6.5 | 301 | 712 |
| all | 706 | 93.6% (1077/1151) | 90.3% (538/596) | 44.4% (4/9) | 7.8 | 397 | 961 |

## Required-fact recall per leaf (section-scoped cases)

| Leaf or family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown/breakdown | 37 | 100.0% (64/64) | 100.0% (34/34) | none | 5.9 | 258 | 881 |
| call-sheets/call-sheets | 26 | 90.2% (37/41) | 87.0% (20/23) | none | 6.6 | 365 | 496 |
| conv | 7 | 95.7% (22/23) | 85.7% (6/7) | none | 11.6 | 889 | 1157 |
| files/files | 13 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 104 |
| help/product-guide | 26 | 95.5% (42/44) | 91.3% (21/23) | none | 2.8 | 477 | 654 |
| help/settings | 16 | 88.5% (23/26) | 84.6% (11/13) | none | 3.3 | 447 | 584 |
| idea-vault/global | 12 | 100.0% (15/15) | 100.0% (9/9) | none | 5.0 | 183 | 183 |
| idea-vault/this-project | 17 | 100.0% (18/18) | 100.0% (14/14) | none | 5.0 | 197 | 197 |
| neg | 14 | none | none | none | 21.4 | 868 | 1494 |
| notes-tasks/notes | 16 | 88.9% (16/18) | 84.6% (11/13) | none | 4.0 | 179 | 303 |
| notes-tasks/tasks | 21 | 92.6% (25/27) | 88.2% (15/17) | 0.0% (0/1) | 9.0 | 259 | 286 |
| pair | 45 | 94.0% (142/151) | 84.4% (38/45) | none | 14.3 | 604 | 1463 |
| production/budget | 36 | 91.4% (32/35) | 90.3% (28/31) | 0.0% (0/1) | 1.2 | 47 | 77 |
| production/cast-crew | 41 | 93.2% (41/44) | 94.4% (34/36) | 100.0% (2/2) | 5.0 | 153 | 536 |
| production/catalog | 13 | 100.0% (22/22) | 100.0% (10/10) | none | 43.7 | 1297 | 1468 |
| production/daily-view | 20 | 100.0% (39/39) | 100.0% (17/17) | none | 1.7 | 386 | 1481 |
| production/day-out-of-days | 20 | 83.7% (36/43) | 88.2% (15/17) | none | 13.3 | 670 | 730 |
| production/locations | 33 | 93.8% (45/48) | 89.7% (26/29) | 0.0% (0/1) | 11.4 | 408 | 503 |
| production/moodboards | 13 | 100.0% (18/18) | 100.0% (10/10) | none | 4.0 | 344 | 431 |
| production/overview | 10 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production/reports | 15 | 100.0% (29/29) | 100.0% (12/12) | none | 6.0 | 697 | 1486 |
| production/schedule | 38 | 91.5% (54/59) | 87.9% (29/33) | 100.0% (1/1) | 5.6 | 391 | 727 |
| production/shot-lists | 26 | 91.4% (32/35) | 87.0% (20/23) | none | 5.0 | 357 | 677 |
| production/sides | 13 | 95.7% (22/23) | 90.0% (9/10) | none | 5.9 | 349 | 602 |
| production/storyboards | 13 | 87.5% (14/16) | 80.0% (8/10) | none | 2.6 | 178 | 380 |
| screenplay/scenes | 44 | 92.6% (87/94) | 81.6% (31/38) | 50.0% (1/2) | 5.0 | 518 | 880 |
| story/beat-board | 25 | 100.0% (43/43) | 100.0% (22/22) | none | 12.1 | 582 | 670 |
| story/characters | 47 | 92.8% (64/69) | 90.7% (39/43) | 0.0% (0/1) | 8.3 | 387 | 688 |
| story/outline | 13 | 100.0% (19/19) | 100.0% (10/10) | none | 12.7 | 634 | 659 |
| story/story-page | 3 | none | none | none | 0.3 | 17 | 51 |
| story/timeline | 19 | 100.0% (33/33) | 100.0% (16/16) | none | 7.1 | 259 | 256 |
| triple | 5 | 94.1% (16/17) | 80.0% (4/5) | none | 16.4 | 590 | 923 |
| turn | 9 | 100.0% (15/15) | 100.0% (9/9) | none | 10.4 | 311 | 992 |
| all sec- cases | 706 | 93.6% (1077/1151) | 90.3% (538/596) | 44.4% (4/9) | 7.8 | 397 | 961 |

Build time per dossier: mean 21.2 ms, p95 48.7 ms, max 1068.5 ms.
Chat or change turns that were given facts: 0 (none wanted).
Guard actions across the run: 41.
Dossiers that carry another user's private note: 0 (none allowed).
Facts by kind over the run: Act 97, Appearance 5, Appearances 24, Assumed 37, Breakdown 179, Budget line 25, Budget total 18, Call sheet 111, Call sheet departments 15, Call sheet extras 11, Call sheet place 15, Call sheet times 15, Card 306, Cast 209, Cast calls 15, Catalog item 579, Character 195, Character days 8, Contingency setting 1, Count 683, Crew 56, Crew calls 15, Daily view 48, Day 31, Day out of days 286, File 56, Found 714, Global vault item 48, Guide 73, Idea Vault item 68, Location 389, Mood board 12, Neighbour scene 2, No actual yet 3, Outline act 42, Outline card 132, Over plan 3, Production overview 11, Project 16, Project note 28, Relationship 215, Report 101, Scene 29, Scene day 12, Scene location 6, Scene text 19, Scenes 1, Settings guide 32, Shoot period 18, Shot 37, Sides 117, Speaking characters 1, Story day 114, Storyboard 16, Task 160, Together 3, Your private note 17.

## Entries not found

| Id | Type | Leaves | Message | Not found | Facts |
|---|---|---|---|---|---|
| sec-x-story-characters-017 | count | story/characters | Which character has the most scenes? | Anjali; 28 | 1 |
| sec-x-story-characters-019 | count | story/characters | How many scenes does Jose have? | 10 | 4 |
| sec-x-story-characters-020 | chat | story/characters | who is varkey | Varghese | 0 |
| sec-x-screenplay-scenes-007 | count | screenplay/scenes | How many night scenes are there? | 15 | 5 |
| sec-x-screenplay-scenes-012 | explain | screenplay/scenes | Describe scene 35. | storm | 7 |
| sec-x-screenplay-scenes-013 | explain | screenplay/scenes | Brief me on scene 32. | storm | 7 |
| sec-x-screenplay-scenes-014 | explain | screenplay/scenes | Explain the story of LOW TIDE RADIO from beginning to end. | Babu | 5 |
| sec-x-screenplay-scenes-021 | lookup | screenplay/scenes | What does Salim give Anjali in scene 30? | call log | 6 |
| sec-s-production-overview-003 | count | production/overview | How many characters does the production count? | 13 | 1 |
| sec-s-production-locations-005 | explain | production/locations | Describe the Coast Radio Station location. | Fr. Thomas | 3 |
| sec-x-production-locations-020 | explain | production/locations | Describe Kadalpuram Fish Landing Centre. | Harbour Clerk office | 6 |
| sec-s-production-locations-006 | lookup | production/locations | what is the noise problem at the fish landing centre | Boat engines | 21 |
| sec-s-production-cast-crew-006 | lookup | production/cast-crew | Who plays Sudheesh and what does he need on set? | chair | 1 |
| sec-s-production-storyboards-001 | lookup | production/storyboards | Which scene is the Roof transmission storyboard for? | 41 | 6 |
| sec-s-production-storyboards-005 | lookup | production/storyboards | Do the panels have drawn images yet? | empty | 1 |
| sec-x-production-shot-lists-010 | count | production/shot-lists | How many close-ups are there? | 3 | 1 |
| sec-s-production-shot-lists-004 | count | production/shot-lists | What size is the shot of Jose bailing? | Close-up | 1 |
| sec-x-production-shot-lists-012 | list | production/shot-lists | Which scenes have three shots? | 41 | 6 |
| sec-x-production-schedule-004 | lookup | production/schedule | Where do we shoot on Day 7? | Coast Radio Station Roof | 6 |
| sec-x-production-schedule-012 | lookup | production/schedule | Which day has the most scenes? | Day 2 | 7 |
| sec-x-production-schedule-016 | lookup | production/schedule | Who is needed at two locations on Shoot Day 12? | Harbour Clerk | 6 |
| sec-x-production-day-out-of-days-004 | lookup | production/day-out-of-days | Does Young Anjali work on Day 8? | yes | 14 |
| sec-s-production-sides-006 | explain | production/sides | In the Day 14 sides, what happens to the boat? | engine | 2 |
| sec-x-production-budget-007 | compare | production/budget | Which line has the largest planned amount? | Lead cast | 1 |
| sec-x-production-budget-019 | lookup | production/budget | What is the total planned for the Cast category? | 6200000 | 2 |
| sec-x-production-budget-020 | lookup | production/budget | What is the total planned for Equipment? | 2450000 | 2 |
| sec-s-call-sheets-call-sheets-002 | list | call-sheets/call-sheets | Which shoot days have no call sheet? | 14 | 6 |
| sec-x-call-sheets-call-sheets-006 | lookup | call-sheets/call-sheets | Which call sheet is for a night shoot? | Shoot Day 6; 16:00 | 6 |
| sec-x-files-files-002 | lookup | files/files | What does the beach permit letter say? | 1 to 14 March 2027; 6 am to 9 pm | 4 |
| sec-s-files-files-002 | list | files/files | What is in the harbour inspection checklist? | life jackets; safety boat; radio check; weather log | 4 |
| sec-s-files-files-003 | lookup | files/files | How tall is the coast station mast? | 42 metres | 4 |
| sec-s-files-files-004 | lookup | files/files | When is low water in March 2027? | 1, 8 and 14 March | 4 |
| sec-x-files-files-004 | lookup | files/files | What lamps does the mast have? | red obstruction lamps | 4 |
| sec-s-files-files-005 | lookup | files/files | Which side of the mast does the feed run on? | east face | 4 |
| sec-x-notes-tasks-notes-002 | change | notes-tasks/notes | At what wind speed do we call off the sea day? | 20 knots | 0 |
| sec-s-notes-tasks-notes-001 | lookup | notes-tasks/notes | Which note is pinned? | Sea safety plan | 5 |
| sec-x-notes-tasks-tasks-003 | change | notes-tasks/tasks | What is due on 2027-03-05? | Order car batteries | 0 |
| sec-x-notes-tasks-tasks-008 | change | notes-tasks/tasks | whats left to do before the roof night | Order car batteries | 0 |
| sec-x-help-product-guide-012 | help | help/product-guide | How do I search the whole project? | Ctrl+K | 3 |
| sec-s-help-settings-004 | lookup | help/settings | Do keyboard shortcuts change in every project? | every project | 3 |
| sec-x-help-settings-004 | help | help/settings | What does the Writing tab have? | spell checking; personal dictionary | 3 |
| sec-s-pair-001 | count | screenplay/scenes,production/cast-crew | How many scenes does Sudheesh play in, and describe his character? | Anjali | 3 |
| sec-s-pair-002 | count | screenplay/scenes,production/cast-crew | How many scenes is Young Anjali in, and what limits does her actor have? | guardian | 2 |
| sec-s-pair-004 | list | screenplay/scenes,story/characters | Which scenes feature the rival of Mariamma? | 24; 28; 43 | 12 |
| sec-s-pair-005 | relation | story/beat-board,screenplay/scenes | Which scene is linked to the card where Babu cuts the power, and who is in it? | Dr. Leela | 22 |
| sec-x-pair-006 | lookup | screenplay/scenes,production/schedule | Which day is the scene where Jose confesses shot, and who else is in it? | Anjali | 3 |
| sec-x-pair-010 | lookup | production/schedule,call-sheets/call-sheets | Is there a call sheet for the day scene 35 is shot? | no call sheet | 8 |
| sec-s-pair-024 | list | production/locations,breakdown/breakdown | What does the roof location require, and which scenes use it? | harness | 43 |
| sec-s-triple-004 | count | production/budget,production/schedule,call-sheets/call-sheets | What was planned for Lead cast, how many shoot days are there and how many call sheets are final? | 14 | 7 |
| sec-s-conv-003 | explain | screenplay/scenes | Walk me through scene 32. | storm | 7 |
| sec-s-conv-009 | explain | screenplay/scenes,production/cast-crew | Describe Salim. | Nandan | 7 |
| sec-s-conv-022 | explain | screenplay/scenes | Why does Mariamma not tell Anjali how Varghese died? Explain. | storm | 9 |
| sec-s-turn-004 | list | production/schedule | what else is shot that day? | 29; 30 | 7 |
| sec-s-turn-008 | chat | story/characters | and his daughter? | Anjali | 0 |
| sec-x-turn-012 | lookup | call-sheets/call-sheets | what time is the crew called on the first draft? | 05:45 | 6 |
| sec-s-turn-017 | lookup | production/cast-crew | give me their emails | vinod.chandrahas@lowtideradio.example; mini.sebastian@lowtideradio.example | 17 |
| sec-s-turn-020 | help | help/product-guide | How do I get the AI? | Download Straft AI | 0 |
| sec-s-turn-030 | chat | production/day-out-of-days | and Nandan? | 2; 4; 5; 6; 7; 8 | 0 |

## Abstain cases not covered

| Id | Message | Statements |
|---|---|---|
| sec-x-neg-058 | What is the budget for the helicopter? |  |
| sec-s-neg-050 | Who is in the scene with the lighthouse keeper? |  |
| sec-x-neg-060 | What is the status of the Mumbai studio? |  |
| sec-x-neg-061 | Which task is about the drone permit? |  |
| sec-s-neg-051 | Who is the lighthouse keeper character? |  |
