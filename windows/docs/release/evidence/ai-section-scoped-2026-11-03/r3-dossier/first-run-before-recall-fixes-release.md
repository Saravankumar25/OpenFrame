# Dossier fetch (spec 5.4, slice S5)

706 conversation cases on the Low Tide fixtures. Recall: how many of the entries a case expects of the answer (each required fact, each answer_has group with any alternative) are in the dossier or in the statements about what is missing. Abstain: the statements name the thing. This measures the fetch, not the writing.

| Family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown-breakdown | 31 | 91.5% (54/59) | 90.3% (28/31) | none | 4.1 | 248 | 878 |
| call-sheets-call-sheets | 19 | 90.9% (30/33) | 89.5% (17/19) | none | 6.8 | 407 | 496 |
| conv | 24 | 95.9% (71/74) | 87.5% (21/24) | none | 9.7 | 702 | 973 |
| files-files | 10 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 116 |
| help-product-guide | 20 | 82.9% (34/41) | 85.0% (17/20) | none | 1.5 | 263 | 428 |
| help-settings | 13 | 11.5% (3/26) | 0.0% (0/13) | none | 1.4 | 55 | 110 |
| idea-vault-global | 9 | 86.7% (13/15) | 77.8% (7/9) | none | 5.0 | 168 | 168 |
| idea-vault-this-project | 12 | 87.5% (14/16) | 83.3% (10/12) | none | 5.0 | 178 | 178 |
| neg | 113 | 100.0% (9/9) | 100.0% (8/8) | 28.6% (2/7) | 9.4 | 374 | 1484 |
| notes-tasks-notes | 13 | 88.9% (16/18) | 84.6% (11/13) | none | 4.3 | 199 | 303 |
| notes-tasks-tasks | 14 | 91.3% (21/23) | 85.7% (12/14) | none | 8.6 | 245 | 286 |
| pair | 45 | 92.7% (140/151) | 80.0% (36/45) | none | 14.5 | 590 | 1468 |
| production-budget | 26 | 90.0% (27/30) | 88.5% (23/26) | none | 1.3 | 49 | 98 |
| production-cast-crew | 33 | 89.7% (35/39) | 90.6% (29/32) | 100.0% (1/1) | 5.2 | 160 | 505 |
| production-catalog | 10 | 59.1% (13/22) | 70.0% (7/10) | none | 54.8 | 1487 | 1497 |
| production-daily-view | 14 | 78.8% (26/33) | 78.6% (11/14) | none | 1.0 | 104 | 145 |
| production-day-out-of-days | 15 | 96.8% (30/31) | 93.3% (14/15) | none | 14.0 | 710 | 730 |
| production-locations | 29 | 93.8% (45/48) | 89.7% (26/29) | none | 10.1 | 399 | 504 |
| production-moodboards | 10 | 100.0% (18/18) | 100.0% (10/10) | none | 5.0 | 431 | 431 |
| production-overview | 7 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production-reports | 12 | 65.5% (19/29) | 58.3% (7/12) | none | 6.1 | 548 | 886 |
| production-schedule | 29 | 94.2% (49/52) | 89.7% (26/29) | none | 5.8 | 423 | 727 |
| production-shot-lists | 19 | 90.0% (27/30) | 84.2% (16/19) | none | 5.1 | 378 | 670 |
| production-sides | 10 | 95.7% (22/23) | 90.0% (9/10) | none | 3.3 | 298 | 602 |
| production-storyboards | 10 | 87.5% (14/16) | 80.0% (8/10) | none | 2.7 | 195 | 380 |
| screenplay-scenes | 31 | 89.6% (60/67) | 76.7% (23/30) | 100.0% (1/1) | 4.4 | 466 | 892 |
| story-beat-board | 18 | 100.0% (35/35) | 100.0% (18/18) | none | 12.7 | 613 | 670 |
| story-characters | 30 | 87.8% (36/41) | 86.7% (26/30) | none | 7.4 | 355 | 688 |
| story-outline | 10 | 100.0% (19/19) | 100.0% (10/10) | none | 12.9 | 641 | 846 |
| story-timeline | 14 | 100.0% (28/28) | 100.0% (14/14) | none | 7.1 | 260 | 256 |
| triple | 5 | 94.1% (16/17) | 80.0% (4/5) | none | 16.4 | 590 | 923 |
| turn | 51 | 81.2% (69/85) | 81.2% (39/48) | none | 6.5 | 289 | 712 |
| all | 706 | 87.3% (1005/1151) | 83.4% (497/596) | 44.4% (4/9) | 7.8 | 372 | 926 |

## Required-fact recall per leaf (section-scoped cases)

| Leaf or family | Cases | Recall | Cases fully covered | Abstain | Mean facts | Mean tokens | p95 tokens |
|---|---|---|---|---|---|---|---|
| breakdown/breakdown | 37 | 92.2% (59/64) | 91.2% (31/34) | none | 4.7 | 267 | 860 |
| call-sheets/call-sheets | 26 | 90.2% (37/41) | 87.0% (20/23) | none | 6.6 | 365 | 496 |
| conv | 7 | 95.7% (22/23) | 85.7% (6/7) | none | 11.6 | 889 | 1157 |
| files/files | 13 | 37.5% (6/16) | 40.0% (4/10) | none | 4.1 | 105 | 104 |
| help/product-guide | 26 | 81.8% (36/44) | 82.6% (19/23) | none | 1.4 | 260 | 433 |
| help/settings | 16 | 11.5% (3/26) | 0.0% (0/13) | none | 1.3 | 48 | 110 |
| idea-vault/global | 12 | 86.7% (13/15) | 77.8% (7/9) | none | 5.0 | 168 | 168 |
| idea-vault/this-project | 17 | 83.3% (15/18) | 78.6% (11/14) | none | 5.0 | 178 | 178 |
| neg | 14 | none | none | none | 22.0 | 834 | 1497 |
| notes-tasks/notes | 16 | 88.9% (16/18) | 84.6% (11/13) | none | 4.0 | 179 | 303 |
| notes-tasks/tasks | 21 | 92.6% (25/27) | 88.2% (15/17) | 0.0% (0/1) | 9.0 | 259 | 286 |
| pair | 45 | 92.7% (140/151) | 80.0% (36/45) | none | 14.5 | 590 | 1468 |
| production/budget | 36 | 91.4% (32/35) | 90.3% (28/31) | 0.0% (0/1) | 1.2 | 47 | 77 |
| production/cast-crew | 41 | 86.4% (38/44) | 88.9% (32/36) | 100.0% (2/2) | 5.0 | 151 | 536 |
| production/catalog | 13 | 59.1% (13/22) | 70.0% (7/10) | none | 54.6 | 1486 | 1497 |
| production/daily-view | 20 | 79.5% (31/39) | 76.5% (13/17) | none | 2.2 | 231 | 1456 |
| production/day-out-of-days | 20 | 83.7% (36/43) | 88.2% (15/17) | none | 13.3 | 670 | 730 |
| production/locations | 33 | 93.8% (45/48) | 89.7% (26/29) | 0.0% (0/1) | 11.4 | 408 | 503 |
| production/moodboards | 13 | 100.0% (18/18) | 100.0% (10/10) | none | 4.0 | 344 | 431 |
| production/overview | 10 | 85.7% (6/7) | 85.7% (6/7) | none | 1.0 | 90 | 90 |
| production/reports | 15 | 65.5% (19/29) | 58.3% (7/12) | none | 6.1 | 474 | 886 |
| production/schedule | 38 | 91.5% (54/59) | 87.9% (29/33) | 100.0% (1/1) | 5.6 | 391 | 727 |
| production/shot-lists | 26 | 91.4% (32/35) | 87.0% (20/23) | none | 5.0 | 357 | 677 |
| production/sides | 13 | 95.7% (22/23) | 90.0% (9/10) | none | 5.9 | 349 | 602 |
| production/storyboards | 13 | 87.5% (14/16) | 80.0% (8/10) | none | 2.6 | 178 | 380 |
| screenplay/scenes | 44 | 90.4% (85/94) | 76.3% (29/38) | 50.0% (1/2) | 4.8 | 523 | 880 |
| story/beat-board | 25 | 100.0% (43/43) | 100.0% (22/22) | none | 12.1 | 582 | 670 |
| story/characters | 47 | 91.3% (63/69) | 88.4% (38/43) | 0.0% (0/1) | 8.2 | 380 | 688 |
| story/outline | 13 | 100.0% (19/19) | 100.0% (10/10) | none | 12.7 | 634 | 659 |
| story/story-page | 3 | none | none | none | 0.3 | 17 | 51 |
| story/timeline | 19 | 100.0% (33/33) | 100.0% (16/16) | none | 7.1 | 259 | 256 |
| triple | 5 | 94.1% (16/17) | 80.0% (4/5) | none | 16.4 | 590 | 923 |
| turn | 9 | 93.3% (14/15) | 88.9% (8/9) | none | 10.4 | 311 | 992 |
| all sec- cases | 706 | 87.3% (1005/1151) | 83.4% (497/596) | 44.4% (4/9) | 7.8 | 372 | 926 |

Build time per dossier: mean 15.8 ms, p95 38.7 ms, max 562.4 ms.
Chat or change turns that were given facts: 0 (none wanted).
Guard actions across the run: 12.
Dossiers that carry another user's private note: 0 (none allowed).
Facts by kind over the run: Act 97, Appearance 5, Appearances 24, Assumed 37, Breakdown 43, Budget line 25, Budget total 18, Call sheet 111, Call sheet departments 15, Call sheet extras 11, Call sheet place 15, Call sheet times 15, Card 306, Cast 209, Cast calls 15, Catalog item 754, Character 195, Character days 8, Contingency setting 1, Count 670, Crew 56, Crew calls 15, Daily view 71, Day 31, Day out of days 286, File 56, Found 779, Global vault item 48, Guide 37, Idea Vault item 68, Location 389, Mood board 12, Neighbour scene 2, No actual yet 3, Outline act 42, Outline card 132, Over plan 3, Production overview 11, Project 16, Project note 28, Relationship 215, Report 102, Scene 29, Scene day 12, Scene location 6, Scene text 19, Scenes 1, Shoot period 18, Shot 37, Sides 117, Speaking characters 1, Story day 114, Storyboard 16, Task 160, Together 3, Your private note 17.

## Entries not found

| Id | Type | Leaves | Message | Not found | Facts |
|---|---|---|---|---|---|
| sec-x-idea-vault-global-001 | lookup | idea-vault/global | Where does the Line to steal come from? | Notebook, 2026 | 5 |
| sec-x-idea-vault-global-002 | lookup | idea-vault/global | What is the Lighting reference reel? | lighting-reference-reel | 5 |
| sec-x-idea-vault-this-project-001 | lookup | idea-vault/this-project | Who is the sea quote from? | Varghese | 5 |
| sec-x-idea-vault-this-project-002 | lookup | idea-vault/this-project | What is the link saved in the vault? | shortwave-listening-guide | 5 |
| sec-x-story-characters-014 | count | story/characters | How many relationships are recorded? | 12 | 1 |
| sec-x-story-characters-017 | count | story/characters | Which character has the most scenes? | Anjali; 28 | 1 |
| sec-x-story-characters-019 | count | story/characters | How many scenes does Jose have? | 10 | 4 |
| sec-x-story-characters-020 | chat | story/characters | who is varkey | Varghese | 0 |
| sec-s-screenplay-scenes-002 | count | screenplay/scenes | How many interior scenes are there? | 26 | 1 |
| sec-x-screenplay-scenes-006 | count | screenplay/scenes | How many exterior scenes are there? | 19 | 1 |
| sec-x-screenplay-scenes-007 | count | screenplay/scenes | How many night scenes are there? | 15 | 1 |
| sec-x-screenplay-scenes-012 | explain | screenplay/scenes | Describe scene 35. | storm | 7 |
| sec-x-screenplay-scenes-013 | explain | screenplay/scenes | Brief me on scene 32. | storm | 7 |
| sec-x-screenplay-scenes-014 | explain | screenplay/scenes | Explain the story of LOW TIDE RADIO from beginning to end. | Babu | 5 |
| sec-x-screenplay-scenes-021 | lookup | screenplay/scenes | What does Salim give Anjali in scene 30? | call log | 6 |
| sec-s-breakdown-breakdown-001 | list | breakdown/breakdown | Which scenes have a VFX element? | 13 | 6 |
| sec-x-breakdown-breakdown-009 | list | breakdown/breakdown | Which scenes have no wardrobe? | 12; 14; 29 | 6 |
| sec-x-breakdown-breakdown-021 | count | breakdown/breakdown | How many scenes have no sound element? | 18 | 1 |
| sec-s-production-overview-003 | count | production/overview | How many characters does the production count? | 13 | 1 |
| sec-s-production-catalog-003 | lookup | production/catalog | Which prop is used in the most scenes? | Steel tea glasses | 54 |
| sec-x-production-catalog-003 | list | production/catalog | Which vehicles are in the catalog? | Fishing boats; Ferry boat; Pickup truck; Auto rickshaw; Inspector's jeep; Little Flower | 54 |
| sec-s-production-catalog-006 | list | production/catalog | Which props are used in more than two scenes? | Steel tea glasses; Rubber stamp | 58 |
| sec-s-production-locations-005 | explain | production/locations | Describe the Coast Radio Station location. | Fr. Thomas | 3 |
| sec-x-production-locations-020 | explain | production/locations | Describe Kadalpuram Fish Landing Centre. | Harbour Clerk office | 6 |
| sec-s-production-locations-006 | lookup | production/locations | what is the noise problem at the fish landing centre | Boat engines | 21 |
| sec-x-production-cast-crew-003 | count | production/cast-crew | How many crew members are there? | 14 | 14 |
| sec-x-production-cast-crew-024 | explain | production/cast-crew | Tell me about Beena Chacko. | 11 pm; Temple festival | 3 |
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
| sec-x-production-daily-view-003 | list | production/daily-view | What props are needed on Day 7? | Rope and harness; Spanner set; Portable generator; Signal lamp | 1 |
| sec-x-production-daily-view-005 | list | production/daily-view | What special effects are needed on Day 7? | Rain machine | 1 |
| sec-s-production-daily-view-006 | list | production/daily-view | What vehicles are needed on Day 12? | Pickup truck; Ferry boat | 1 |
| sec-s-production-sides-006 | explain | production/sides | In the Day 14 sides, what happens to the boat? | engine | 2 |
| sec-s-production-reports-002 | lookup | production/reports | In the Scene Report, what is the heading of scene 20? | CHAAYA TEA SHOP - EVENING | 6 |
| sec-x-production-reports-004 | list | production/reports | In the Breakdown Completeness report, which scenes have no wardrobe? | 25; 29; 39 | 6 |
| sec-s-production-reports-003 | count | production/reports | How many scenes have no sound in the Breakdown Completeness report? | 18 | 6 |
| sec-s-production-reports-005 | list | production/reports | In the Prop Report, which scenes use a rubber stamp? | 25; 43 | 6 |
| sec-s-production-reports-006 | lookup | production/reports | cast scene report for Young Anjali | 33; 34; 36 | 7 |
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
| sec-x-help-product-guide-010 | list | help/product-guide | Which languages does the assistant understand? | Hindi; Telugu; Tamil; Kannada; Malayalam | 1 |
| sec-s-help-product-guide-004 | help | help/product-guide | What do I press to take back my last change? | Ctrl+Z | 2 |
| sec-x-help-product-guide-012 | help | help/product-guide | How do I search the whole project? | Ctrl+K | 2 |
| sec-s-help-settings-001 | help | help/settings | What tabs does Settings have? | Project settings; Writing; Keyboard shortcuts; Templates; Offline AI; Advanced; Appearance | 1 |
| sec-x-help-settings-001 | list | help/settings | Which Settings tab do I use to switch to dark mode? | Settings | 2 |
| sec-x-help-settings-002 | lookup | help/settings | What modes can I choose for the appearance? | Light; Dark | 2 |
| sec-s-help-settings-002 | help | help/settings | What does the Graphics card setting do? | Automatic; Off | 1 |
| sec-s-help-settings-003 | list | help/settings | What is in Project settings? | logline; genre | 1 |
| sec-x-help-settings-003 | navigate | help/settings | Where do I remove Straft AI? | Offline AI | 1 |
| sec-s-help-settings-004 | lookup | help/settings | Do keyboard shortcuts change in every project? | every project | 1 |
| sec-x-help-settings-004 | help | help/settings | What does the Writing tab have? | spell checking; personal dictionary | 1 |
| sec-s-help-settings-005 | explain | help/settings | Does the theme change the project? | does not change the project | 2 |
| sec-x-help-settings-005 | help | help/settings | Do themes change printed call sheets? | black on white | 2 |
| sec-x-help-settings-006 | help | help/settings | How do I import a model file for Straft AI? | Import a model file | 1 |
| sec-x-help-settings-007 | help | help/settings | Can I import my own theme? | My Themes | 2 |
| sec-s-help-settings-006 | help | help/settings | Can I change the keyboard shortcut for Quick Capture? | Keyboard shortcuts | 1 |
| sec-s-pair-001 | count | screenplay/scenes,production/cast-crew | How many scenes does Sudheesh play in, and describe his character? | Anjali | 3 |
| sec-s-pair-002 | count | screenplay/scenes,production/cast-crew | How many scenes is Young Anjali in, and what limits does her actor have? | guardian | 2 |
| sec-s-pair-004 | list | screenplay/scenes,story/characters | Which scenes feature the rival of Mariamma? | 29 | 12 |
| sec-s-pair-005 | relation | story/beat-board,screenplay/scenes | Which scene is linked to the card where Babu cuts the power, and who is in it? | Dr. Leela | 22 |
| sec-x-pair-006 | lookup | screenplay/scenes,production/schedule | Which day is the scene where Jose confesses shot, and who else is in it? | Anjali | 3 |
| sec-x-pair-010 | lookup | production/schedule,call-sheets/call-sheets | Is there a call sheet for the day scene 35 is shot? | no call sheet | 8 |
| sec-s-pair-020 | list | breakdown/breakdown,production/catalog | Which scenes use the prop that appears in the most scenes? | Steel tea glasses | 54 |
| sec-x-pair-015 | list | production/locations,breakdown/breakdown | Which scenes are at Kadalpuram Beach and what is the permit situation there? | 33; 40 | 17 |
| sec-s-pair-028 | list | production/daily-view,production/sides | For Day 12, which scenes are in the sides and which props are needed? | Suitcase; Substation key | 6 |
| sec-s-triple-004 | count | production/budget,production/schedule,call-sheets/call-sheets | What was planned for Lead cast, how many shoot days are there and how many call sheets are final? | 14 | 7 |
| sec-s-conv-003 | explain | screenplay/scenes | Walk me through scene 32. | storm | 7 |
| sec-s-conv-009 | explain | screenplay/scenes,production/cast-crew | Describe Salim. | Nandan | 7 |
| sec-s-conv-022 | explain | screenplay/scenes | Why does Mariamma not tell Anjali how Varghese died? Explain. | storm | 9 |
| sec-s-turn-004 | list | production/schedule | what else is shot that day? | 29; 30 | 7 |
| sec-s-turn-008 | chat | story/characters | and his daughter? | Anjali | 0 |
| sec-x-turn-008 | list | production/locations,production/schedule | which scenes use them? | 21 | 23 |
| sec-x-turn-012 | lookup | call-sheets/call-sheets | what time is the crew called on the first draft? | 05:45 | 6 |
| sec-s-turn-017 | lookup | production/cast-crew | give me their emails | vinod.chandrahas@lowtideradio.example; mini.sebastian@lowtideradio.example | 17 |
| sec-s-turn-020 | help | help/product-guide | How do I get the AI? | Download Straft AI | 0 |
| sec-x-turn-016 | lookup | idea-vault/this-project | who said it? | Varghese | 5 |
| sec-s-turn-026 | lookup | production/daily-view | what special effects? | Rain machine | 1 |
| sec-s-turn-030 | chat | production/day-out-of-days | and Nandan? | 2; 4; 5; 6; 7; 8 | 0 |

## Abstain cases not covered

| Id | Message | Statements |
|---|---|---|
| sec-x-neg-058 | What is the budget for the helicopter? |  |
| sec-s-neg-050 | Who is in the scene with the lighthouse keeper? |  |
| sec-x-neg-060 | What is the status of the Mumbai studio? |  |
| sec-x-neg-061 | Which task is about the drone permit? |  |
| sec-s-neg-051 | Who is the lighthouse keeper character? |  |
