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
| Daily View | Export button for the shown day | Owner change list item 12 | `production/tabs/daily.tsx`, `features/export/buttons.tsx` |
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
