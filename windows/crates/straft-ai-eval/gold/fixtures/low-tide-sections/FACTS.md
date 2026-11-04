# Fixture facts: low-tide-sections

`low-tide-sections` is `low-tide-two-screenplays` (see `../low-tide-two-screenplays/FACTS.md`, every fact there still holds) plus the steps of `extra.jsonl`, run after the base setup by `src/fixture.rs`. It exists for the section-scoped gold set (`gold/sections.lowtide.en.jsonl`), for the leaves that the base fixture holds no data for: Story > Timeline and Idea Vault > Global.

## Story Timeline (feature screenplay LOW TIDE RADIO)

Story Days, set with `story.assign_story_day`:

| Story Day | Feature scenes |
|---|---|
| DAY 1 | 1, 2, 3, 4, 5 |
| DAY 2 | 6, 7, 8, 9, 10 |
| DAY 3 | 11, 12, 13, 14 |
| DAY 4 | 15, 16, 17, 18, 19 |
| 1999 | 33, 34, 35, 36 |

Five Story Days; 23 scenes have a day. The other 22 scenes (20 to 32, and 37 to 45) are Unassigned.

Time notes, set with `story.set_time_note`: scene 3 "Late morning", scene 14 "After midnight", scene 45 "Dawn, a week later". Three scenes have a time note.

## Global Idea Vault (shared across projects, 4 items)

- Note "Cold open idea": Start on a close shot of a kettle coming to the boil while a radio hisses behind it.
- Quote "Line to steal": Every harbour keeps a list of the boats that did not come back. (source: Notebook, 2026)
- Link "Lighting reference reel" (https://example.com/lighting-reference-reel)
- Note "Title ideas": Salt Light. The Quiet Coast. Low Water.

The project Idea Vault keeps its own four items (see the base FACTS.md); the two vaults share nothing.

## Not added on purpose

- Story page text: the Story page is not offered to the assistant (`story.page.*` are hidden operations), so no data is added and the gold set has no question for Story > Story Page.
- Episodes: the project is a feature film, and the Episodes leaf is offered only on episodic projects.
