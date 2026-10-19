# Layout and scroll contract (Windows)

> Written 2026-10-20 for the owner's UX audit brief. It states the layout rules the Windows app now follows and the test
> that measures them. Code is the source of truth: `apps/desktop/src/styles/app.css`, `openframe.css` and each
> workspace stylesheet. The measuring test is `tests/e2e/specs/98-layout-geometry.test.mjs`.

## 1. The frame never scrolls

- `html`, `body`, `#root`, `.app-root`, `.layout`, `.main`, `.content.fill` and the screenplay frame use
  `overflow: clip`, not `overflow: hidden`. A hidden box can still be scrolled by script; the editor's
  "scroll the caret into view" did exactly that and left the top bar stuck off screen with a white strip at the bottom
  (owner change list item 5). A clipped box cannot be scrolled at all.
- The page itself has no horizontal or vertical scroll (measured: `pageOverflowX` and `pageOverflowY` are 0 on every
  screen at both sizes, with and without the AI panel).

## 2. One scroll owner per region

| Region | Scroll owner | Direction |
|---|---|---|
| Navigation rail | `.nav` | vertical, only when the window is shorter than the list |
| Workspace content | the workspace's own content box (`.content`, `.spx-scroll`, `.bdw` panes) | vertical |
| Story Board | `.story-canvas` scrolls sideways across act columns; each `.act-body` scrolls down on its own | both, split by owner |
| Screenplay page | `.spx-wrap > .spx-scroll` | vertical, sideways only when zoomed wider than the pane |
| Scene list | `.spx-nav-list` | vertical |
| AI Assistant | the conversation list | vertical |
| Dialogs | the dialog body; header and buttons stay put | vertical |

Two-way scrolling is allowed only where the content is a two-dimensional surface: the Story Board, the stripboard and
Day Out of Days grids, and a zoomed screenplay page.

## 3. Toolbars and tab strips wrap, they do not hide controls

- A toolbar that does not fit grows to a second row. It never scrolls sideways and never lets a control slide under a
  neighbour. The screenplay toolbar had a fixed 46 px height, so its second row went under the scene list (measured at
  1280 by 720: Compare, Export, Lock and View and more were covered). It now grows (`height: auto; min-height: 46px`).
- The Production tab strip (fourteen tabs) wraps to a second row (`.subnav.wrap`). With the AI panel open at 1280 by
  720 it used to scroll, and Moodboards or Sides were out of sight.
- Segmented controls never wrap inside themselves; the row around them wraps.
- Long names (draft, screenplay) are shortened with an ellipsis and carry the full name in the tooltip.

## 4. Panels

- The navigation rail collapses to an icon rail and opens again (button at the foot of the rail; the choice is kept on
  this computer). Focus mode hides the top bar, rail, status bar and AI panel.
- The AI panel is docked beside the workspace, never over it. Its width can be dragged or set with the arrow keys on its
  edge (double-click resets) and is kept between 300 px and the smaller of 640 px and 45 percent of the window.
- The Screenplay scene list and the Breakdown scene list and side panel each have a Show/Hide button.

## 5. Window sizes

The geometry test runs at 1280 by 720 (the smallest supported working size) and 1920 by 1080, with the AI panel closed
and open, over Home, every workspace, every Production tab and Settings. It fails on any of: page overflow, the AI panel
or rail overlapping the workspace, a control outside the window, a control cut off by its container, or a control
covered by another element. The measured numbers are written to `tests/e2e/artifacts/layout-geometry.json`.
