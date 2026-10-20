# Windows UX change log (final polish, October 2026)

> Sources: the owner's change list (`changes/all_changes.docx`), the 2026-10-20 change document
> (`openframe-chnages.docx`), the AI repair brief (`OpenFrameAI.docx`) and the UX audit brief. None of these source
> files are committed. Owner decision 2026-10-19: the Windows app keeps its own design and is not unified with the Mac
> app; only the listed changes are made, in the Windows style. Detailed plan: `docs/implementation/final-polish-plan.md`.

| Area | Change | Root cause or reason | Where |
|---|---|---|---|
| Frame | The top bar no longer gets stuck off screen with a white strip below | The editor scrolled an `overflow: hidden` ancestor while bringing the caret into view; frames now use `overflow: clip` | `styles/app.css` |
| Top bar | Logo opens all projects; Back and Forward replace Undo and Redo (Undo and Redo moved to the project menu, Ctrl+Z and Ctrl+Y unchanged); a gear opens Settings; Alt+Left and Alt+Right | Owner change list | `app/shell/Shell.tsx`, `app/stores.ts` |
| Home | Settings dialog with Appearance, Keyboard shortcuts, About and support | Owner change list | `app/home/GlobalSettingsDialog.tsx` |
| Navigation | Rail collapses to icons and opens again; choice kept on this computer | Owner request 2026-10-18 | `Shell.tsx`, `app.css` |
| Reload | A reload keeps the current page for the session | Owner change list | `app/stores.ts` |
| Zoom | Ctrl+Plus, Ctrl+Minus, Ctrl+0 zoom the window; Screenplay and Breakdown pages zoom with Ctrl+wheel and pinch | Owner change list | `tauri.conf.json`, `ScreenplayView.tsx`, `ScriptPane.tsx` |
| Focus mode | Hides everything except the screenplay page | Owner change list | `Shell.tsx`, `ScreenplayView.tsx` |
| Screenplay | Tab cycles every element and wraps; scene list can be hidden; Review section removed; long names shortened with the full name in the tooltip | Owner change list | `editor/rules.ts`, `ScreenplayView.tsx` |
| Screenplay toolbar | Grows to a second row in a narrow window | Measured: fixed 46 px height let the second row slide under the scene list at 1280 by 720 | `screenplay.css` |
| Beat Board | Act columns fill the height and scroll on their own; cards show their whole text; Parking Lot column scrolls | 2026-10-20 change document item 2 | `story.css` |
| Characters | Free-text relationships with suggestions; relationship map without crossing labels | Owner change list | `Characters.tsx` |
| Breakdown | Scene list and side panel can be hidden; page zoom; sidebar no longer overflows | Owner change list | `breakdown/*` |
| Production tabs | Fourteen tabs wrap to a second row | Measured: with the AI panel open at 1280 by 720 the strip scrolled and hid Moodboards or Sides | `production/index.tsx`, `app.css` |
| Shot Lists | List and Grid views | Owner change list | `production/tabs/shots.tsx` |
| Daily View | Export button makes its own document, "Shoot Day N - Daily View" (PDF or plain text), with exactly what the page shows: scenes, location, cast, call sheet, important notes, breakdown items, scene order | Owner change list item 12 first opened the Shooting Schedule export for one day; the page and the printout could differ. Both now come from one read (`schedule::ops::daily_view`) | `production/tabs/daily.tsx`, `features/export/buttons.tsx`, `modules/exports/daily_view.rs` |
| Story Outline | The outline list scrolls inside the page (wheel, arrow and Page keys); the toolbar stays in place | The Beat Board rule `overflow-y: hidden` on `.story-canvas` also applied to the outline, which has no columns to scroll, so its rows past the window were unreachable | `story/OutlineView.tsx`, `story.css` |
| Screenplay Compare drafts | The draft pickers and the view controls sit in two groups in a bar that grows with its rows; nothing overlaps the changes list or the summary | The compare bar kept the 46 px height of the screenplay toolbar and its full-width `select.select` pickers wrapped onto a second row that ran over the content | `screenplay/CompareView.tsx`, `screenplay.css`, `app.css` |
| Settings Offline AI | Technical details takes a row of its own at the full width; hashes and addresses wrap in their cells and show in full in a tooltip | The component table was squeezed into a 420 px card, its long values forced it wider than the card and it ran under the AI health card | `settings/OfflineAi.tsx`, `ai/ai.css` |
| Screenplay Find | Find next, previous and typing scroll the page to the current match in both scopes; Replace moves on to the next match after the replaced text; the count follows edits, undo and Replace All | The editor's own scroll-into-view follows the browser selection, which sits in the Find box, so it scrolled the wrong area; the count was only updated by the Find bar's own buttons | `editor/find.ts`, `editor/ScreenplayEditor.tsx` |
| Stripboard | EXT Night green, INT Night light blue, EXT Day yellow, INT Day white | 2026-10-20 change document item 1 | `openframe.css` |
| Packages | Breakdown, Shot List, Schedule and Call Sheet Review packages removed (export refused with a clear message; old files still open) | Owner change list | `features/packages/*`, `modules/packages/exchange.rs` |
| New project | Episodic and Series removed for new projects; existing projects keep their type | Owner change list | `NewProjectDialog.tsx`, `ProjectSettingsForm.tsx` |
| Themes | OpenFrame Default has its own dark palette; new Black and White theme (black on white in Light, white on black in Dark); dark contrast fixes | Owner change list, UX brief | `themes.json` |
| AI panel | Resizable; plain answers first, details behind "Details"; long lists show eight items and "Show all" | AI repair brief | `ai/AiPanel.tsx`, `ai/Conversation.tsx` |

## Regressions found and resolved during the work

- A Mac-style shell was started and fully reverted on the owner's word (2026-10-19).
- Theme tests assumed list positions; the Black and White theme shifted them. Tests now look themes up by id.
- The layout geometry spec found the two covered-control defects above on its first run; both are fixed.

## Remaining

See `windows-ux-audit.md`, section "Open".
