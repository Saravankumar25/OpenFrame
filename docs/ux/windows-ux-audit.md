# Windows UX audit (October 2026)

> Written 2026-10-20 for the owner's UX audit brief. Every finding below was measured in the running app or reported by
> the owner; nothing is estimated. The measuring spec is `tests/e2e/specs/98-layout-geometry.test.mjs` and its numbers
> are in `tests/e2e/artifacts/layout-geometry.json` (regenerated on every run). Rules that came out of the audit are in
> `layout-and-scroll-contract.md`; the full list of changes is in `change-log.md`.

## Method

1. The owner's change lists (`all_changes.docx`, `openframe-chnages.docx`) and the AI brief screenshots: each item
   reproduced in the app, then fixed (see `change-log.md`).
2. Layout geometry spec: the journey project "Night Bus" plus stress text (a 300-character card title, a 200-character
   unbroken address, mixed Telugu, Tamil, Devanagari, emoji and accented Latin, a 62-character character name). It
   opens Home, Idea Vault, the four Story views, Screenplay, Breakdown, all 14 Production tabs, Call Sheets, Files and
   Settings at 1200 by 720 (the window minimum), 1280 by 720 and 1920 by 1080, with the AI panel closed and open: 144
   screen states per run. On each it
   measures page scroll, panel overlap, controls outside the window, cut off or covered, and text cut off with no
   tooltip or label carrying the full text.

## Findings

| # | Severity | Where | Finding | Root cause | State |
|---|---|---|---|---|---|
| 1 | High | All workspaces | Top bar stuck off screen with a white strip below after typing (owner item 5) | Editor scrolled an `overflow: hidden` frame | Fixed: `overflow: clip` |
| 2 | High | Screenplay, 1280 by 720 | Compare, Export, Lock and View and more covered by the scene list; with the AI panel open also Find, Notes, Comments and Drafts | Toolbar wrapped to a second row inside a fixed 46 px height | Fixed: toolbar grows |
| 3 | High | Production, 1280 by 720, AI open | Moodboards or Sides tab scrolled out of sight on 9 of 14 tabs | Tab strip scrolled sideways | Fixed: strip wraps |
| 4 | Medium | Story Board | Act columns did not fill the height; long cards cut off (owner document item 2) | Columns sized to content; card text clamped | Fixed |
| 5 | Medium | Story Outline | Long card titles and unbroken text ended in an ellipsis with no way to read them | `.truncate` on the reading view | Fixed: outline lines wrap |
| 6 | Medium | Project Home, Screenplay suggestions and scene list, Files, Notes, Tasks, Idea Vault, stripboard, Story Board and cards, Characters, Timeline, Moodboards, Shot Lists, Search, top bar | Shortened names and lines with no tooltip (28 places) | Ellipsis without `title` | Fixed: full text in tooltip |
| 7 | Medium | Breakdown | Side panel overflow and no way to hide panels (owner items) | Fixed widths | Fixed |
| 8 | Low | AI answers | Long list items ended in an ellipsis on unbroken text | `text-overflow` on list items | Fixed: items wrap |
| 9 | Low | Dark themes | OpenFrame Default dark was a copy of Studio Amber; screenplay suggestions low contrast | Shared seeds | Fixed: own palette |

Measured after the fixes: page sideways scroll 0, AI panel overlap 0 and navigation overlap 0 on every screen state;
no control outside the window, cut off or covered; no text cut without its full text available.

## Open (not verified, stated plainly)

- Display scaling above 100 percent, Windows Snap layouts and a maximised window on a second monitor were not run by
  automation; the team hand test covers them.
- IME composition (typing Telugu or Hindi with an input method) is not automated; pasted Indic text is.
- Contrast is enforced by the theme contrast pairs in `crates/openframe-application/src/modules/appearance/theme.rs`
  and the theme spec (91); there is no per-pixel contrast scan of every screen.
- Before and after screenshots exist only as the geometry numbers and the visual baseline; no HTML mock-ups were made.
