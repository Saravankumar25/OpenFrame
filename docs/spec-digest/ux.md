# OpenFrame Studio — UX/UI Digest

**Sources used:**
- `OpenFrame_Studio_UX_UI_Specification_Updated.md` (4549 lines, read in full). Cited as "Spec §N".
- `HTML_Mockups/assets/openframe.css` (382 lines, read in full).
- `HTML_Mockups/index.html` and all 181 mockups. The required set was read in full; the others were read as extracted text. Cited as "Mock NNN".
- Short cross-checks against the FSD (§15–16, §51, §127.2) and the PRD (§35, §103), only where the UX spec defers to them.

**Conventions:**
- Labels in `code` or "quotes" are verbatim.
- **[GAP]** means neither the spec nor the mockups define the item. It needs a decision; do not invent it.
- **[MOCK≠SPEC]** marks a place where the mockup and the spec disagree.

**Authority order:** PRD → FSD → UX/UI Spec → implementation (Spec §0.1).
- The mockup CSS header says: `OpenFrame Studio — mock UI design system (prototype only; visual design is placeholder and will be replaced later)`.
- The index says: "Screens are rendered at 1200 × 720 (a Windows 11 desktop app window). The visual design is a placeholder; behaviour and content follow the OpenFrame PRD, FSD, UX/UI, Domain/Data, AI, Import/Export, Offline/Collaboration and Security specifications. Sample project: BLACK RAIN."
- So treat mockup **tokens** as the working baseline, and mockup **labels and behaviour** as authoritative wherever they agree with the spec.

**Guiding statements** (use as a design test):
- "One film, progressively structured: messy ideas → story outline → screenplay → production plan." (§0)
- "OpenFrame Studio should feel smaller than it is." (§112)
- Workflow: "Capture → Arrange → Write → Review → Lock → Break Down → Visualize → Schedule → Call Sheet."
- "Simplicity as a functional requirement."

**Spec numbering gap:** Spec sections **18–22 do not exist**; the numbering jumps from §17 Scene Card Detail to §23 Screenplay. PRD traceability (§98) still maps these topics:
- PRD-STORY-005 → "Build Screenplay Preview"
- PRD-STORY-006 → "Characters; Character Relationships"
- PRD-STORY-007 (P1) → "Story Timeline"

These screens are defined **only** by Mocks 063–070. §92 still has a page-action row for Characters.

---

## 1. Design Tokens

### 1.1 CSS custom properties (verbatim from `openframe.css` `:root`)

```css
:root{
  --bg:#f3f1ec; --panel:#ffffff; --ink:#1e2330; --ink2:#48505f; --muted:#7a8291; --line:#e1ddd4; --line2:#cfc9bd;
  --side:#1a1d26; --side2:#252936; --side-ink:#c5cad6; --side-mut:#7f8799;
  --accent:#d9822b; --accent-d:#b96a1a; --accent-soft:#fcefdc; --accent-ink:#ffffff;
  --blue:#3b6fd4; --blue-soft:#e6eefc; --green:#2f8f5b; --green-soft:#e0f3e8; --red:#c8443a; --red-soft:#fbe6e3;
  --purple:#7a5ac8; --purple-soft:#eee8fb; --yellow:#d9a520; --yellow-soft:#fcf2d2; --teal:#1f8a8a; --teal-soft:#dcf1f1;
  --shadow:0 8px 28px rgba(20,24,38,.22); --shadow-s:0 1px 3px rgba(20,24,38,.12);
}
```

| Token | Value | Usage in mockups |
|---|---|---|
| `--bg` | `#f3f1ec` | App window background (warm off-white) |
| `--panel` | `#ffffff` | Top bar, sub-nav, cards, dialogs |
| `--ink` | `#1e2330` | Primary text; selected segment background; toast background; multi-select bar |
| `--ink2` | `#48505f` | Secondary text, labels, icon buttons |
| `--muted` | `#7a8291` | Hints, meta text, uppercase section heads |
| `--line` | `#e1ddd4` | Light borders and dividers |
| `--line2` | `#cfc9bd` | Stronger borders (inputs, buttons, dialogs, drawers) |
| `--side` | `#1a1d26` | Left navigation background (dark) |
| `--side2` | `#252936` | Nav hover |
| `--side-ink` | `#c5cad6` | Nav text |
| `--side-mut` | `#7f8799` | Nav footer text |
| `--accent` | `#d9822b` | Brand orange: primary buttons, active nav, focus ring, Scene Card left border, checkboxes |
| `--accent-d` | `#b96a1a` | Primary-button hover and border; accent text |
| `--accent-soft` | `#fcefdc` | Selected rows and list items, "on" icon buttons, menu hover, accent chips |
| `--accent-ink` | `#ffffff` | Text on accent |
| `--blue` / `--blue-soft` | `#3b6fd4` / `#e6eefc` | Selection outline for cards, shots and tiles; insertion line; info banners; drop highlight; tags |
| `--green` / `--green-soft` | `#2f8f5b` / `#e0f3e8` | Saved dot, ok banners, "Locked", "Confirmed", "Finalized", "Ready" chips, toggles on |
| `--red` / `--red-soft` | `#c8443a` / `#fbe6e3` | Danger buttons, errors, "Needs breakdown", warning chips, conflict strip outline, required `*` |
| `--purple` / `--purple-soft` | `#7a5ac8` / `#eee8fb` | Private notes (banner `.priv`, comment `.priv`), audio waveform, quote cards |
| `--yellow` / `--yellow-soft` | `#d9a520` / `#fcf2d2` | Warn banners, "Needs review", "Suggested" chips, amber dot |
| `--teal` / `--teal-soft` | `#1f8a8a` / `#dcf1f1` | URL file icon, teal chip |
| `--shadow` | `0 8px 28px rgba(20,24,38,.22)` | Dialogs, menus, toasts, dragged card |
| `--shadow-s` | `0 1px 3px rgba(20,24,38,.12)` | Cards, strips, shots, panels |

### 1.2 Hard-coded (non-token) colours worth tokenising

**Shell surfaces**
- Title bar: `#eceae5`, bottom border `#d8d4cb`, text `#333`; close-button hover `#c42b1c` with white text.
- Logo gradient: `linear-gradient(135deg,#d9822b,#a4471a)`.
- Status bar: `#e9e6df`, top border `var(--line2)`.
- Nav separator: `#2d3242`.
- Project selector background: `#faf9f6`. Search box background: `#f7f6f2`.
- Icon-button hover: `#efece5`. "On" icon-button border: `#f0d3ae`.
- Avatar colours: default `#5a6b9c`; `.b` `#2f8f5b`; `.c` `#b5507a`; `.d` `#c47a1c`; `.e` `#6d55b8`.
- Dot colours: default green; `.amber` yellow; `.red`; `.blue`; `.gray` `#9aa1ae`; `.purple`.

**Buttons, dialogs and tables**
- Button hover: `#f6f4ef`. Ghost-button hover: `#ece9e2`.
- Danger button: background `var(--red)`, border `#a5342b`.
- "On" button: background accent-soft, border `#eec99a`.
- Dialog, drawer and table-header footer background: `#faf9f6`.
- Scrim: `rgba(18,21,32,.5)`. Tooltip: `#111`. Toast link: `#ffce8a`.

**Chip text colours**
- `.g` `#1f6b41`; `.b` `#2a57ad`; `.r` `#9b2e25`; `.y` `#8a6510`; `.p` `#5b3fa8`; `.t` `#146b6b`.
- `.dark` uses an ink background with white text. `.out` is transparent with a line2 border.
- Default chip: background `#ebe8e0`, text ink2.

**Banners**

| Banner | Background | Border | Text |
|---|---|---|---|
| info | blue-soft | `#c3d6f6` | `#22499a` |
| warn | yellow-soft | `#ecd68a` | `#7a570b` |
| err | red-soft | `#f1bcb6` | `#8f2b22` |
| ok | green-soft | `#b6e0c7` | `#1c6440` |
| priv | purple-soft | `#d5c8f3` | `#4c33a0` |

**Idea Vault**
- Note card: background `#fff9e8`, border `#efdfae`.
- Quote card: background `#f4f0fb`, border `#dcd2f2`.
- "Unavailable" badge: red background, white text.
- File-type icon colours:
  - PDF `#f8d7d3` / `#a22`
  - DOC `#d6e2f8` / `#2a57ad`
  - AUD `#e7dcf8` / `#5b3fa8`
  - VID `#d5efe6` / `#1a6a4a`
  - URL `#dbeff2` / `#146b6b`
  - Generic `#e9e5db` / `#555`

**Story Board**
- Act band: `#ece8df`, border line2.
- Sequence container: `#f8f6f1`. Sequence count pill: `#ece8df`.
- Scene Card: white, border line2, **4px left border in accent**.
- Beat: background `#fbf8ef`, dashed border `#d5c78a`, 4px left border `#d9b73a`, text `#5c4d12`.
- Beat `.blue` variant: `#f0f5fd` / `#bcd0f1` / `#5a86dd` / `#274b8f`.
- Parking Lot and Unscheduled pool: `repeating-linear-gradient(135deg,#f0ede6 0 10px,#e9e5db 10px 20px)`, `2px dashed #bdb6a6`.
- Parking Lot cards: left border `#aaa`, opacity `.92`.
- Insertion line: 3px blue with an 8px dot.

**Screenplay**
- Page backdrop: `#dcd8cf`. Page: white, shadow `0 2px 12px #0003`.
- Current element: background `#fff6e6` with a `-4px 0 0 var(--accent)` left bar.
- Search highlight `.hl`: `#ffe27a`. Comment highlight `.hlc`: `#cfe0ff` with a 2px blue underline.
- Diff added: `#d9f2e2` / `#14532d`. Diff removed: `#fbdcd8` / `#7f1d1d` with strikethrough.

**Breakdown**
- Suggested element row: `repeating-linear-gradient(135deg,#fffbee 0 8px,#fff6d8 8px 16px)` with a dashed top border `#e4cd7c`. This keeps "Suggested" visibly different from confirmed (Spec §83.3).

**Stripboard strip colours** (INT/EXT × Day/Night)

| Class | Meaning | Background | Border |
|---|---|---|---|
| `.xd` | EXT·D | `#f7f0b5` | `#d9c85a` |
| `.xn` | EXT·N | `#c9dcf7` | `#8fb0e3` |
| `.id` | INT·D | `#ffffff` | `#bbb` |
| `.in` | INT·N | `#dfe6ec` | `#9fb0c2` |

- Alternates in the CSS: `.ied` `#fff3b0`, `.ien` `#bfd7f5`.
- Warning strip: `outline:2px solid var(--red)`.
- Break marker strip `.mk`: `#e5e1d6`, dashed, centred, weight 800, letter-spacing 1px, colour `#555`.
- Day column `.daycol`: `#f8f6f1`. Drop target `.hot`: blue border plus `0 0 0 3px #3b6fd433`.

**Call-sheet paper**
- Section title bar `.st`: `#1e2330` background, white uppercase 11px text.
- Table borders `#bbb`; header background `#eee`.
- **Editable field** `.edt`: `#fff8e6`, `1px dashed #e0b562`.
- **Missing field** `.need`: `#fde4e1`, dashed `#d7746a`, text `#8f2b22`.

**Moodboard and calendar**
- Moodboard canvas: `#fbfaf7` with a dot grid `radial-gradient(#d9d4c8 1px,transparent 1px)` at 18px.
- Calendar shoot-day cell: accent-soft with border `#eec99a`. Off-day cell: `#efece5`.

**Collaboration**
- LAN session bar `.sessionbar`: `#1f6b6b`, white text, 12px.
- Soft-lock outline in the mock: `2px solid #b5507a` (the participant's avatar colour).

### 1.3 Typography

**UI type**
- Font family: `"Segoe UI","Segoe UI Variable Text",system-ui,-apple-system,Arial,sans-serif`; base `14px`.
- Spec §3.5: "straightforward interface typography."
- **[GAP]** No macOS-specific family is named beyond `-apple-system`.

**Screenplay type**
- Font family: `"Courier Prime","Courier New",Courier,monospace`.
- Mock size is `12px` (final override), line-height `1.45`, colour `#111`. The value is scaled for the mock (see §4).

**Type scale in the CSS**

| Role | Size / weight / other |
|---|---|
| Page title `.ph h1` | 20px / 700 / letter-spacing -.2px |
| Page subtitle `.ph .sub` | 13px, muted |
| Dialog title `.dialog .dh h2` | 16px / 700; sub 12.5px muted |
| Drawer title | 15px; type label 11px uppercase .6px letter-spacing, 700, muted |
| Empty-state title | 18px (AI panel empty: 15px) |
| Brand | 15px / 700 / .2px |
| Nav items | 14px / 500 |
| Sub-nav | 12.5px / 500 (13px before override); active 600 in accent-d with 2px accent underline |
| Section head `.h4` | 11px uppercase / .7px / 700 / muted |
| Menu header `.mh` | 10.5px uppercase / .6px / 700 |
| Table header | 11px uppercase / .5px / 700 / muted; body 13px |
| Buttons | 13px / 600; `.sm` 12px; `.xs` 11px |
| Segmented control | 12.5px / 600 |
| Chips | 11px / 600, pill (999px) |
| `kbd` | 11px Segoe UI, 1px line2 border with a 2px bottom border, radius 5px |
| Card title `.card h3` | 13px |
| Scene Card text | 12.5px / line-height 1.35, clamped to 2 lines (`-webkit-line-clamp:2`) |
| Beat text | 12px |
| Act header | uppercase, 800, 11px (override), letter-spacing .4px |
| Sequence header | 12px / 700 |
| Strip text | 12px; scene number 13px / 800; INT/EXT 11px / 800 |
| Status bar / title bar | 12px |
| Hints / errors | 11.5px (errors 600, red) |
| Paper (call sheet) | 12px / 1.4; h1 22px, letter-spacing 1px, centred; tables 11.5px |

### 1.4 Spacing, sizing, radii

**Window and shell**
- Mock window: **1200×720**.
- Title bar **30px**; window-control cells 46px wide.
- Top bar **46px**, padding `0 12px`, gap 12px.
- Left nav **172px**, padding `12px 8px`, item padding `9px 12px`, radius 8px, gap 2px.
- Status bar **26px**, padding `0 12px`, gap 16px.
- Content padding `16px 20px`; `.flush` = 0.
- Sub-nav item padding `9px 8px` (override).

**Top-bar elements**
- Brand block 164px wide; logo mark 24×24, radius 6px.
- Project selector min-width 196px, padding `5px 10px`, radius 8px.
- Search: flex, max-width 400px, padding `6px 12px`, radius 8px.
- Icon buttons 30×30, radius 7px. Avatar 26px circle, 11px bold.

**Controls**
- Buttons: padding `7px 13px`, radius 8px. `.sm`: `4px 9px`, radius 6px. `.xs`: `2px 7px`, radius 5px.
- Inputs: padding `7px 10px`, radius 7px, min-height 32px. Textarea min-height 70px.
- Focus ring: `box-shadow:0 0 0 3px #d9822b33` with an accent border. Error ring: `0 0 0 3px #c8443a22` with a red border.
- Checkbox and radio 16px (checkbox radius 4px). Toggle 34×19 with a 15px knob.

**Containers**

| Element | Radius |
|---|---|
| Cards | 10px |
| Project cards | 12px |
| Vault cards | 10px |
| Act bands | 12px |
| Sequences | 9px |
| Scene Cards and beats | 7px |
| Strips | 6px |
| Day columns | 10px |
| Dialogs | 12px |
| Menus | 10px (items 6px) |
| Toast | 9px |
| Banners | 8px |
| Chips | 999px |

**Dialog widths** (Spec §3.7: small for confirmation, medium for create/import/export, large for comparison, exchange review and conflict; avoid layered dialogs)
- `.dialog` 460px default; `.sm` **400**; `.md` **540**; `.lg` **780**; `.xl` **960**; max-height 640px.
- Header padding `14px 18px 8px`, body `6px 18px 12px` at 13px / 1.5, footer `12px 18px` on `#faf9f6` with a top border. Buttons are right-aligned; an optional `.l` item is pushed left.

**Drawers** (Spec §74.3: "consistent medium detail width")
- `.drawer` **390px**; `.w` **450px**; `.n` **340px**.
- Absolute right, full height, `box-shadow:-8px 0 26px rgba(20,24,38,.16)`, z-index 40.

**Other overlays**
- AI panel `.ai`: **360px**, right side, full height, z-index 40.
- Menu: min-width 200px, padding 5px, z-index 60.
- Toast: bottom 40px, centred, z-index 70.
- Tooltip: z-index 80. Scrim: z-index 50.

**Grids**
- Gap 12px; `.g2`–`.g6` column helpers.
- Vault side panel `.vpanel` 178px. Masonry grid: 3 columns by default, gap 10px.

**Screenplay layout**
- Top bar `.sp-top` 46px. Scene navigator `.sp-nav` 210px (Breakdown uses 214px).
- Tool panel `.sp-tools` 270px (Breakdown right pane 352px).

**Story Board layout**
- Act band min-width 220px (232px in Mock 049; 260px in Mock 166). Parking Lot 210px. Board gap 14px.

**Stripboard layout**
- Unscheduled pool 300px (Mock 130). Strip padding `6px 9px`, gap 9px; location min-width 140px; page-count cell 34px.

**Other**
- Call-sheet side panel 290px (Mock 143).
- Calendar: 7 columns, gap 6px, cells min-height 88px.
- Icons: `svg.i` 16px (`.s` 13, `.l` 20, `.xl` 28), stroke 1.9, round caps.

### 1.5 Shadows
- `--shadow` for floating layers (dialogs, menus, toasts, dragged card, find bar).
- `--shadow-s` for resting cards.
- Drawer and AI panel: `-8px 0 26px rgba(20,24,38,.16)`.
- Screenplay page: `0 2px 12px #0003`. Paper: `0 2px 12px #0002`.
- Dragged Scene Card: `transform:rotate(-2deg); box-shadow:var(--shadow); opacity:.95` (Spec §104: "Lifted/moved card").

### 1.6 Theme rules (dark/light)
- **[GAP] No dark theme exists** in `openframe.css`, the UX spec or the mockups.
- The only dark surfaces are fixed:
  - left nav (`--side`)
  - toast and multi-select bar (`--ink`)
  - splash (`radial-gradient(circle at 50% 40%,#2c3350,#12141c)`)
  - LAN session bar
- The spec has no light/dark or high-contrast rule. The only colour rules are:
  - §3.4: "Board colors and personal organization colors are optional. System colors such as production revision colors must remain distinguishable from personal color choices and be accompanied by textual meaning."
  - §104.1: "Do not rely on a color alone. A visible label, icon, count or change in control state should communicate important status as well."
  - §82.6: revision colour "can appear in the script/print preview where supported, but normal writing should not be permanently tinted."
  - Mock 090: "The colour is metadata and export styling only; it is separate from your personal card colours."
- **Recommendation for implementation (not from source):** define all colours as the tokens above so a theme can be added later. Needs a product decision.

### 1.7 Status vocabulary and state visuals

**Status words** (Spec §3.3; do not invent micro-statuses): `Ready`, `Needs attention`, `Draft`, `Locked`, `Stale`, `Unscheduled`, `Complete`, `Archived`.

**State visuals (Spec §104)**

| State | Primary cue | Secondary cue | Interaction |
|---|---|---|---|
| Selected | Focus border / background | Optional check | Actions available |
| Dragging | Lifted / moved card | Insertion line | Drop to commit |
| Parked | Muted container / card | Parking Lot label | Restore |
| Locked | Lock badge | Status text | Start Revision |
| Revision | Revision badge | Colour on marked pages | Open / compare |
| Stale | Attention banner | Source version label | Review / Refresh |
| Suggested | Different suggestion styling | Matched text / category | Accept / Edit / Reject |
| Finalized | Stable document badge | Date / source | Open / export |
| Offline | Status strip | Local-ownership message | Continue core work |
| Conflict | Attention highlight | Two-version comparison | Review / Resolve |
| Unavailable file | File-unavailable cue | Filename / path | Relink / Open location |

**Selection colours (mock)**
- Blue 2px outline for Scene Cards, shots, tiles and table rows.
- Accent outline for selected Vault cards.
- Accent-soft background for selected list and table rows.

**Status lists**
- Project status menu (Mock 015): `Idea`, `Development`, `Writing`, `Rewriting`, `Shooting Draft`, `Pre-Production`, `Shoot Preparation`, `Shooting`, `Completed`, `Archived`.
- Location status (Spec §42): `Idea`, `Shortlisted`, `Confirmed`, `Rejected`.
- Catalog status examples (Mock 113): `Required`, `Confirmed`, `Searching`, `Shortlisted`.
- Call-sheet status (Spec §51–52): `None`, `Draft`, `Source changed` / `Source Changed`, `Ready`, `Finalized`, `Superseded`. The Mock 141 list filter uses `All` · `Draft` · `Needs refresh` · `Finalized`.
- Research verification flags (Mock 045): `Needs Expert Check`, `Checked`, `Unchecked`.

---

## 2. Global Shell Anatomy

Spec §1.1:
```
┌────────────────────────────────────────────────────────────────────────────┐
│ OpenFrame   Project Name ▾     Global Search     Undo Redo   Status   User │
├──────────────────────┬─────────────────────────────────────────────────────┤
│ Home / Idea Vault / Story / Screenplay / Breakdown / Production /           │
│ Call Sheets / Files  │                    ACTIVE WORKSPACE                 │
├──────────────────────┴─────────────────────────────────────────────────────┤
│ Saved / Saving / Offline / Collaboration / contextual status               │
└────────────────────────────────────────────────────────────────────────────┘
```

The shell has three jobs only:
1. Tell the user which project is open.
2. Provide stable navigation.
3. Expose universal actions such as search, undo/redo, save/session state and AI command access.

It "must not become a dashboard containing production metrics from every module." AI command access "does not add a separate product navigation hierarchy."

### 2.1 Title bar (Mock 014, 30px)
- Windows 11 style: logo plus the text `OpenFrame Studio — BLACK RAIN` (the project name). With no project open it reads `OpenFrame Studio`.
- Window controls on the right: `–` `□` `✕` (✕ hover is red).
- Focus Mode title: `OpenFrame Studio — BLACK RAIN — Full-screen writing` (Mock 079).

### 2.2 Top bar (46px), left to right (Mock 014; numbered callouts ps, gs, ur, ai, gnav, st, cn)
1. **Brand**: mark plus `OpenFrame`.
2. **Project selector** (callout 1): two lines, `BLACK RAIN` over `<small>Feature Film</small>`, with a chevron.
   - Opens a menu (Mock 013) with `Recent projects` (rows such as "BLACK RAIN / Feature Film · Writing" with `Open`), `Pinned`, `All projects…`, `Archived…` and `Close project`.
   - Spec §1.3: "compact list of recent projects, pinned projects and an All/Archived area." Switching follows the unsaved-work rule (§2.6 below).
3. **Global search** (callout 2): placeholder `Search this project…` with a `Ctrl+K` kbd hint. It opens the search overlay (Mock 156).
4. **Action icon buttons** (30×30): `+` Quick actions (Mock 020), Undo (callout 3), Redo, AI assistant sparkle (callout 4), Help `?`, then the user avatar `NV`.
   - The Undo/Redo pair is callout 3; the AI entry point is callout 4.
   - The "on" state for an icon button is accent-soft.
- **Home screen variant** (no project open, Mock 003): brand, `Search projects…`, one icon button and the avatar. There is **no left nav** and no project selector.

### 2.3 Left navigation (callout 5; 172px, dark)
**Exact labels and order** (Spec §1.2 and Mock 014):
1. `Home`
2. `Idea Vault`
3. `Story`
4. `Screenplay`
5. `Breakdown`
6. `Production`
7. `Call Sheets`
8. `Files`

Rules:
- The active item has an accent (orange) background and white text. There is "one clear selected state."
- Footer: `Project stored locally` / `on this PC`.
- Do "not create permanent top-level navigation items for each small catalog type."
- Mock icons, for reference: house, lightbulb, columns, document, checklist, film-frame, clipboard, folder.

**Production sub-navigation**
- Spec §1.2 lists: Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Shooting Schedule, Daily Production View, Sides, Reports and Lightweight Budget.
- Spec §54 lists: Catalog, Locations, Cast & Crew, Moodboards, Storyboards, Shot Lists, Shooting Schedule, Daily View, Sides, Reports, Budget, Notes/Tasks.
- **Mock sub-nav** (`.subnav` tab strip at the top of the content area, active tab with a 2px accent underline): `Overview` | `Catalog` | `Locations` | `Cast & Crew` | `Moodboards` | `Storyboards` | `Shot Lists` | `Schedule` | `Daily View` | `Sides` | `Reports` | `Budget` | `Notes`.
- [MOCK≠SPEC] The mock uses `Schedule` and `Notes`; the spec uses "Shooting Schedule" and "Notes/Tasks". The mock adds `Overview` (Production Home).

**Story workspace switcher**
- On the Story Board page: segmented `Board` | `Outline`.
- On the Characters and Timeline pages: segmented `Board` | `Outline` | `Characters` | `Timeline` (Mock 067).

### 2.4 Status bar (26px; callouts 6 and 7)
- **Left:** a dot and the save state (`Saved`, green dot), then a context crumb. Examples:
  - `BLACK RAIN · Feature Film · Writing`
  - `Story Board`
  - `Screenplay · Standard`
  - `Production · Shooting Schedule`
  - `Breakdown · Scene 12`
  - `Call Sheets · Day 4`
  - `Local Session · 2 collaborators connected`
- **Right:** connection state, for example `Offline · Local Project Saved`.
- With no project open: grey dot and `No project open`.
- Values the strip may show (Spec §1.4): Saved, Saving, Save error, Offline, Local collaboration session, External AI connection state, Pending exchange/import operation.
- It "must not block creative work for transient status changes."
- **Clicking it** opens a small panel above the bar (Mock 019, 340px menu):
  - `Project status`
  - `Saved` with `Just now`
  - `Offline — your project is stored locally.`
  - the path `D:\Films\BLACK RAIN`
  - buttons `Create backup`, `Open recovery`, `Open project location`
- Error variants:
  - Save error (Mock 017): left `Save error`, right `Offline · Local save failed`.
  - Drive lost (Mock 172): right `Offline · Project drive unavailable`.

### 2.5 Workspace header pattern
- `h1` title plus a question-style subtitle. Examples:
  - `How does my story fit together? · Drag cards until the story works.`
  - `What things does this film need? Reusable items — not one huge spreadsheet.`
- Primary action top-right in accent colour. A toolbar row follows.
- Four visual levels (§3.1): shell → workspace header → primary work area → secondary info.

### 2.6 Universal commands (Spec §1.5)
- Consistent labels: `New`, `Open`, `Duplicate`, `Rename`, `Delete`, `Undo`, `Redo`, `Import`, `Export`, `Search`.
- Context menus add object-specific actions.

### 2.7 Quick actions menu (`+`, Mock 020)
Header `Create`, then:
- `New Idea` `Ctrl+Shift+N`
- `New Act`
- `New Sequence`
- `New Beat`
- `New Scene Card` `Ctrl+N`
- `New Character`
- `New Location`
- `New Shot List`
- `New Moodboard`
- `Import Screenplay…`

Spec entry points reference "+ command > New Project" (§6) and "Quick Action > New Moodboard" (§44).

### 2.8 Global search overlay (Mock 156)
- Top-anchored `.dialog.xl` at **720px** wide, 70px from the top, on a scrim.
- Large input showing the typed query with an `Esc` kbd.
- Scope segment: `This project` | `+ Global Idea Vault`, plus a result count (for example `5 results`).
- Result rows: type chip (120px wide, for example `SCENE 22`, `PROP`, `IDEA VAULT`, `SHOT 25B`), bold snippet, muted module path (for example `Production · Catalog`), and `Open`.
- Global search is separate from in-screenplay find (§26) and from Home project search (§4).

### 2.9 Toasts (Mock 021)
- Dark ink pill at the bottom centre. Text example: `Moved Scene Card “He finds his childhood photograph.” to Sequence “First Investigation”` followed by the link `Undo` (colour `#ffce8a`).
- Undo labels use human language, for example "Move Scene Card", "Delete Shot" (§2.8).
- No notification centre anywhere (§0.2, §111).

### 2.10 Dialogs, drawers, menus, banners
- **Modal** only for deliberate decisions: creation, import/export setup, conflict confirmation, destructive actions. No stacked modals (§2.3).
- **Drawer** for expanded detail (§2.2, §74).
  - Header: title, object type label (uppercase), close.
  - Body: primary fields, optional fields under `Add Details`, related links, attachments, comments.
  - Footer only when needed: `Save` / `Cancel` / `Delete/Archive`.
  - Simple edits autosave.
  - Opening preserves the underlying scroll and selection; closing returns to exactly the same place.
- **Context menu** (§2.4): common actions first; destructive actions last, separated and red (`.mi.dan`); shortcuts right-aligned in muted text.
- **Banners** (inline): info / warn / err / ok / priv. Used for source banners, stale notices, save errors, private notes.
- **Multi-select bar** (§77.2): dark `.mbar` pinned near the top of the workspace. Examples:
  - `3 selected | Move to Folder  Add to Collection  Tag  Export  Copy to Project  Delete`
  - `3 cards selected | Move  Duplicate  Park  Export  Tag  Delete`

### 2.11 LAN session bar (Mock 166)
- Teal strip **above** the layout: `Working in local shared session` `· BLACK RAIN · 2 collaborators connected`, presence pills (avatar plus name), and `End Session`.
- Status bar right: `Local Session · 2 collaborators connected`.

### 2.12 AI panel placement
- Right-side overlay panel, **360px**, full height, over the workspace. It is **part of the shell**, "not a separate chatbot product" (§71).
- Opened from the top-bar sparkle icon, contextual AI actions, the global command/search area, or workspace AI actions.
- Details in §3.40 below.

### 2.13 Unsaved switch and recovery (shell-level dialogs)
- **Unsaved switch** (Spec §73.6, Mock 018, md):
  - Title `This project has changes that are not saved yet.`
  - Body: `OpenFrame is still trying to save BLACK RAIN . If you leave now you may lose your latest edits.`
  - Buttons: `Stay`, `Retry Save`, `Close Without Leaving`.
  - "Must not use ambiguous button labels such as `OK` and `Cancel`."
- **Recovery offer** (§73.7, Mock 016, md):
  - Title `We found a recent recovery state for this project.`
  - Body: `BLACK RAIN was not closed normally last time. OpenFrame kept a more recent automatic recovery state than your last confirmed save.`
  - Comparison: `Last saved version` (`Yesterday, 11:58 PM` · `Screenplay: 41 scenes`) versus `Recovery state` (`Today, 10:37 AM` · `Screenplay: 42 scenes · 9 changes newer`).
  - Note: `Nothing has been replaced. You choose which one to open.`
  - Buttons: `Compare…`, `Keep Saved Version`, `Open Recovery`. "Do not silently select one version."
- **Save error** (§73.3, Mock 017):
  - Inline banner: `Save error — Your current work is still open. Retry save or open Recovery.` with `Retry Save` and `Open Recovery`.
  - Dialog title: `OpenFrame could not save the latest changes to the project.`
  - Body: `This is a problem with saving to your computer, not with the internet. Your edits are still on screen and have not been lost.` / `Possible causes: the disk is full, the drive was disconnected, or the folder is read-only.`
  - Buttons: `Keep Editing`, `Save Backup Copy`, `Retry Save`.
  - The error "should not imply that an internet connection is necessary."

---

## 3. Screen-by-Screen

Format per screen: layout → controls (verbatim) → empty state → menus → shortcuts → drag/drop → dialogs → errors.

### 3.1 Splash and launch (Mocks 001–002)
- Splash: `OpenFrame Studio` / `Capture the idea. Shape the story. Write the script. Plan the shoot.` / `Opening your local projects…` / `Version 1.0 · Your projects stay on this computer`.

### 3.2 Application Home (Spec §4; Mocks 003–007, 016, 018, 023, 171)
- **Question:** "Which project should I continue or open?"
- **Layout** (Mock 003):
  - Header `Your projects` / `Pick up where you left off, or start something new.`
  - Top-right: a `Search projects` field, `Open Project`, and **`New Project`** (primary, "the dominant action").
  - Section `Pinned` (4-column grid of project cards), then `Recent projects` with a ghost button `Open Archived`.
  - Project card: preview image, title, chips (type such as `Feature Film`, status such as `Writing`, `Pinned`), and `Last modified 2 hours ago`.
  - "No production metadata appears on application Home."
- **Controls (Spec):** New Project, Open Project, Search Projects, Pin/Unpin, Archive, Open Archived, Open existing project.
- **Project card context menu** (Mock 005): `Open`, `Rename` `F2`, `Duplicate Project`, `Pin / Unpin`, `Reveal in File Manager`, `Archive`, then separated `Delete Project…`.
- **Empty state** (Mock 004): `Create your first project.` / `A project holds your ideas, story board, screenplay and production plan for one film. Everything is stored on this computer.` / `Create Project` and `Open Existing Project`.
- **Archived** (Mock 006):
  - Title `Archived projects` / `Archived projects are not deleted. They stay on your computer and can be restored at any time.` / `Back to projects`.
  - Table `Title | Type | Status | Last modified` with row actions `Restore` and `Open`.
- **Project unavailable** (Mock 007, dialog sm):
  - Title `This project could not be opened`.
  - Body: `The project location is unavailable. It may be on an external drive that is not connected, or the folder was moved.` / `Project: Salt Road` / `Last known location: E:\Films\Salt Road` / `OpenFrame has not created a replacement project. Reconnect the drive and try again, or locate another copy of the project.`
  - Buttons: `Cancel`, `Locate Project…`, `Try Again`.
- **Delete project** (Mock 023, md, strong confirmation):
  - Title `Delete “BLACK RAIN”?` / `This is the most destructive action in OpenFrame. The whole project will be removed from this computer.`
  - Checkbox `Create a backup first (recommended) — E:\Backups\BLACK RAIN\`.
  - Field `Type the project name to confirm`.
  - Note: `Archiving instead keeps everything and simply hides the project from your recent list.`
  - Buttons: `Cancel`, `Archive Instead`, `Delete Project` (danger).
- **States:** No projects / Recent projects / Search results / Archived projects / Project unavailable.
- **Shortcuts:** Ctrl/Cmd+N New Project; Ctrl/Cmd+O Open Project.
- **Not on this screen:** production metadata, enterprise dashboards, notification centre.

### 3.3 New Project dialog (Spec §6; Mocks 008–010; dialog md 540px)
- Title `New Project`; sub `What is this project called and what type is it?`
- **Fields:**
  - `Title *` with placeholder `e.g. BLACK RAIN`; validation `A title is required.`
  - `Type *` as a segmented control: `Feature Film` | `Short Film` | `Episodic / Series`.
  - Collapsible `Optional details`: `Language`, `Genre`, `Creator`, each with placeholder `Add later`.
- Note: `You can add or change any of this later. You do not need it to start writing.`
- **Buttons:** `Cancel`, `Create`. Create is enabled when the required fields are valid.
- After creation the user goes straight to Project Home. "There is no setup wizard."
- **Error** (Mock 010): `The project could not be created. Nothing was saved and no partial project was made. Your entries are still here.` with `Cancel` and `Retry`.
- **States:** Blank, Typing, Invalid, Creating, Created, Creation error.
- Entry points: Home > New Project; + command > New Project; New Project > From Template (§61).

### 3.4 Project Home (Spec §5; Mocks 011, 012, 014, 015, 022)
- **Question:** "What do I want to continue doing in this film?"
- **Active project** (Mock 012):
  - Header `BLACK RAIN` / `Feature Film · English · Crime Thriller · Last opened today 10:42`.
  - Right: status chip menu `Writing ▾` and `Export project…`.
  - Left column (1.25fr):
    - `Continue`: 2×2 cards `Continue screenplay` (`Scene 12 · Draft 6`), `Continue Story Board` (`Act 2 · First Investigation`), `Continue breakdown` (`Scene 29`), `Continue schedule` (`Shoot Day 4`).
    - `Quick Access` buttons: `Idea Vault`, `Story Board`, `Screenplay`, `Breakdown`, `Production`, `Call Sheets`.
    - `Recent` list, for example `Scene 12 — INT. POLICE STATION — NIGHT` / `Edited scene · 10:39`.
  - Right column (1fr):
    - `Production status` card, shown only once production content exists: `Script status` (`Shooting Draft 6 · Locked`), `Breakdown progress 28 / 42 scenes` with a bar, `Next shooting day` (`Day 4 · Mon 14 Jun 2027`), `Latest call sheet` (`Day 2 · Finalized`), `Open needs` (`2 locations · 1 cast`).
    - `Project Files` card with `Add File` and a file list.
- **New project** (Mock 011):
  - Header `Feature Film · Status: Idea · Created just now` with `Project settings`.
  - `Start with your ideas, build your story, or open a screenplay.` / `Pick whichever fits where you are. You can move between them at any time.`
  - Buttons: `Open Idea Vault`, `Open Story Board`, `Write Screenplay`, `Import Screenplay`.
  - `Three ways to begin`:
    - `I have an idea` — "Opens Idea Vault and Story Board so you can collect thoughts and arrange scenes."
    - `I already have a screenplay` — "Opens the import / write workflow."
    - `I am ready to plan my shoot` — "Opens screenplay import and Breakdown."
  - Spec: "New projects show three high-value next actions: Open Idea Vault, Build Story, Write/Import Screenplay." No fake metrics.
- **Status menu** (Mock 015): see §1.7.
- **Project settings** (Mock 022, md):
  - Title `Project settings` / `Project identity fields.`
  - Fields: `Title *`, `Type *`, `Language`, `Genre`, `Creator`, `Status`, `Project notes`.
  - Note: `Everything here is optional except title and type. Language, genre and creator are reused on exported documents.` Buttons `Cancel`, `Save`.
- **States:** Brand-new, Development, Writing, Production, Archived.
- **Episodic projects** also expose Project Home > Series (§3.20).

### 3.5 Idea Vault — Project and Global (Spec §7–10, §79; Mocks 024–047)
- **Questions:** "What have I collected for this film?" (Project) and "What do I have that might matter someday?" (Global).
- **Header:**
  - `Idea Vault` plus a project chip `BLACK RAIN`.
  - Sub: `Everything collected for this film. Nothing here is classified or synced to the screenplay.`
  - Links `Quick Capture` and `Recently Deleted`.
  - Scope segment `Global` | `This Project`. The spec marks scope as `Current Project`.
- **Global header** (Mock 040): `Idea Vault Global` / `Ideas across your whole filmmaking life — copy them into a film when they are ready.` Status crumb: `Idea Vault · Global`.
- **Toolbar:** `Add` (primary, menu), `Record Note`, search field `Search ideas, tags, filenames…`, and view segment `Grid` | `Card` | `List` | `Folder`. The spec calls the views Visual Grid / Card View / List View / Folder View.
- **Left panel** (178px):
  - `All items 86`, `Pinned 3`, `Recently added`, `Recently modified`.
  - `Collections` (user-named with counts, for example `Ending ideas 7`) plus `New Collection`.
  - `Folders` (for example `Scenes to explore 14`).
- **Grid view** (Mock 024): `Pinned` section, then `Everything else`, in masonry.
  - Card meta examples: `Note · core concept`, `Image · Visual References`, `URL · Locations`, `PDF · Research`, `Voice note · 0:42 · today`, `Video · 0:58`, `Sketch · Platform`.
- **List view** columns (Mock 026): `Name | Type | Collection | Tags | Added | Modified`.
- **Folder view** (Mock 027): breadcrumb `All items › Scenes to explore`, folder tiles with `14 items`, then `Items in this folder`.
- **Add menu** (Mock 029): `Note`, `Image`, `URL`, `File`, `Audio / Voice Note`, `Video`, separator, `Quick Capture` `Ctrl+Shift+N`.
- **Item types** (Mock 047: "Twelve kinds of item. None needs a title, a category or a folder."):
  1. Text note
  2. Image
  3. URL
  4. PDF
  5. Document
  6. Audio / voice note
  7. Video
  8. Folder / collection
  9. Handwritten / sketch ("No transcription needed")
  10. Quote
  11. Screenshot ("Treated as a normal image")
  12. Any other file ("Stored even if it cannot be previewed")
- **Visual behaviour matrix (§79)**

  | Item | Collapsed | Expanded | Primary action | Rule |
  |---|---|---|---|---|
  | Text note | Text preview | Editable note | Edit | Title may remain blank |
  | Image | Thumbnail + caption | Large preview + metadata | Open | — |
  | URL | Title / URL preview | URL + note + preview | Open | Preview metadata is secondary |
  | PDF | File card + page indicator | Document preview + file info | Open | External open available |
  | Document | Filename / type card | Preview / open externally | Open | — |
  | Audio | Card + duration | Playback + optional transcription | Play | Audio is authoritative |
  | Video | Thumbnail + duration | Local preview | Play | Not a video editor |
  | Sketch | Thumbnail | Large preview | Open | — |
  | Quote | Quoted text | Full quote + note / source | Edit | Source optional |
  | Screenshot | Thumbnail | Large preview + note | Open | — |
  | Other | Filename / type | System preview / open | Open | Unsupported preview ≠ unsupported storage |

  - Images and video get larger previews; notes are text-first (yellow `#fff9e8` card); quotes use a purple-tint card; files get file-type cues.
- **Empty state** (Mock 028; Spec Global `Drop any film idea, reference or file here.`; Spec Project `Start collecting anything about this film.`):
  - Mock copy: `Drop any film idea, reference or file here.` / `This is where you throw anything about the film. Add a note, image, link, file, or voice note. You never have to decide what it is first.`
  - Buttons: `Add`, `Record Note`.
  - [MOCK≠SPEC] The Project Vault spec copy differs.
- **Note editor drawer** (Mock 030, 390px):
  - Type `Note`, title `Untitled note`, field `Title (optional)`, body, and `Saved automatically`.
  - `Collections` and `Tags` (with `Add tag…`).
  - Buttons: `Pin`, `Copy to Project`, `Send to Story`, `Delete`.
- **Item detail drawer** (Spec §9; Mock 031, 450px):
  - Media preview plus filename.
  - Fields: `Title`, `Caption`, `Source link` (`Optional`), tags.
  - Buttons: `Rename`, `Move`, `Add to Collection`, `Open externally`, `Copy to Project`, `Delete`, `Send to Story`.
  - Spec controls: Edit, Rename, Move, Add to Collection, Pin, Copy to Project, Send to Story, Open externally, Delete.
  - Text opens directly into editing. "The interface does not force naming before closing."
- **Add URL** (Mock 032, md):
  - Title `Add URL` / `Paste a link. The link itself is what matters; a preview is just a bonus.`
  - Field `Link`, then `Preview found: “…”`.
  - Field `Your note` and checkbox `Save a thumbnail of the page`.
  - Buttons: `Cancel`, `Add URL`.
- **Record Note** (Mock 033, sm):
  - Timer `0:17` with `Recording…`.
  - Field `Name (optional)` with placeholder `Leave blank to keep it untitled`.
  - Checkbox `Transcribe to text when available (the recording is always kept)`.
  - Buttons: `Cancel`, `Stop & Save`.
- **Drag-in** (Mock 034):
  - Overlay `Drop to add 5 items to the Project Vault` / `No categories needed. They will be stored exactly as they are.`
  - Spec §79.1: show a compact confirmation only when needed; never ask for a category per item.
- **Partial result** (Mock 035, md):
  - Title `5 items added, 1 could not be added` / `Successful items were kept. Only the failed item needs attention.`
  - Rows `Added` or `Not added`, with a reason, for example `The file is in use by another program and could not be read.`
  - Buttons: `Try Failed Item Again`, `Done`.
- **Unavailable external file** (Mock 036):
  - Red badge `Unavailable` on the card.
  - Drawer text: `File unavailable. The original file could not be found at D:\Refs\platform-2.jpg.` / `OpenFrame keeps this item and its notes. Nothing has been deleted.`
  - Buttons: `Relink…`, `Open Location`.
  - Note: `This item links to a file outside the project. Files stored inside the project show a different marker.`
- **Multi-select** (Mock 037): bar `3 selected | Move to Folder, Add to Collection, Tag, Export, Copy to Project, Delete`.
- **Context menu** (Mock 038): `Open` `Enter`, `Rename` `F2`, `Move…`, `Pin`, `Copy to Project`, `Send to Story`, `Export`, separator, `Delete` `Del`.
  - Spec §76 order: Open · Rename · Move · Pin · Copy to Project · Send to Story · Export · Delete.
- **Send to Story Board** (Mock 039, md):
  - Title `Send to Story Board` / `What should this become?` / `This creates a copy . The original stays in your Idea Vault.`
  - Radio options:
    - `Beat` — a small story event
    - `Scene Card` — a compact scene reminder
    - `Sequence idea` — a group of scenes
    - `Character note`
    - `Story note`
  - Field `Place in` (for example `Act 2 — The Discovery ▸ First Investigation`) with `Or leave it in the Parking Lot.`
  - Buttons: `Cancel`, `Create Copy in Story Board`.
- **Copy to Project** (Mock 041, md):
  - `The copy is independent. Editing it will not change the Global original, and the Global original stays where it is.`
  - Fields `Copy into`, `Collection (optional)`. Buttons `Cancel`, `Copy to Project`.
- **Search** (Mock 042):
  - Result header `3 results for “station”`.
  - Match reasons: `matched in caption & tag`, `matched in text`, `Matched in filename`.
- **Quick Capture** (Spec §10; Mock 043, sm):
  - Title `Quick Capture`; context line `Saving to BLACK RAIN · Project Idea Vault`.
  - Single multiline field, auto-focused.
  - Hint `Enter or Ctrl+Enter to save · Esc to cancel`. Buttons `Cancel`, `Save`.
  - Closes immediately after saving. Save errors keep the text.
- **Recently Deleted (Vault)** (Mock 044):
  - Title `Recently Deleted` / `Items you deleted can be restored.` with `Back to Vault`.
  - Info banner: `Deleted items stay here until you permanently delete them. Nothing is gone after one click.`
  - Columns `Item | Type | Deleted | Original place`; actions `Restore`, `Delete Permanently`.
- **Research collection** (Mock 045):
  - Header `Collection` / `Research — Police Procedure` / `8 sources`.
  - Rows carry `Why it matters: …` or `Linked to: …` plus a verification flag (§1.7).
  - Note: `Verification flags are yours to set. OpenFrame never marks research as "true" just because you imported it.`
- **New Collection** (Mock 046, sm):
  - Field `Name *`.
  - Hint: `Examples: Ending ideas · Visual References · Crazy scenes · Locations · Research · Music · Things to remember. OpenFrame never prescribes names.`
  - Buttons `Cancel`, `Create`.
- **Shortcuts:** Ctrl/Cmd+F search; Delete deletes the selection; Enter opens; Quick Capture Enter / Ctrl+Enter save, Esc cancel.
- **Errors:** missing external references stay visible with an unavailable indicator; failed adds report only the failed items.

### 3.6 Story Board — Board view (Spec §11, §80, §81; Mocks 020, 048–066)
- **Question:** "How does my story fit together?"
- **Header** (Mock 049):
  - `Story Board` / `How does my story fit together? · Drag cards until the story works.`
  - Chip: `No scene numbers — they come from the screenplay`.
- **Toolbar:**
  - Segment `Board` | `Outline`.
  - `Add` (primary; menu).
  - Filter input `Search / filter cards` (190px).
  - Zoom `−` `100%` `+`.
  - Spacer, then `Parking Lot` and `Build Screenplay` (primary).
- **Canvas:**
  - Horizontal Act bands (about 232px), for example `Act 1 — The Return` with a count `6 scenes` or `5 items`.
  - Sequence containers with a name and count pill (for example `Hero Introduction 3`).
  - Scene Cards: 2-line clamp, 4px accent left border.
  - Bottom-right indicators: `H` (tooltip `has a scene heading`) and a comment count digit.
  - Beats use a lighter dashed yellow style.
  - Parking Lot: a hatched column on the right with a count chip.
- **Add menu** (Mock 050): `Act`, `Sequence`, `Beat`, `Scene Card` `Ctrl+N`, separator, `From Idea Vault…`.
- **Spec controls:** Add Act, Add Sequence, Add Beat, Add Scene, Board/Outline, Zoom, Search/Filter, Multi-select, Parking Lot, Build Screenplay.
- **Empty state** (Mock 048; §73.1):
  - `Your Story Board is empty.`
  - `Scene Cards are short reminders of what happens in each scene. Drag them until the story works.`
  - `Start shaping the story. Add an Act, Sequence, Beat, or Scene Card — or drag an idea from your Idea Vault.`
  - Buttons: `Add Scene`, `Add Act`. The §73.1 example CTA is `+ Add Scene`.
- **Card rules** (§16, §80.3):
  - Small horizontal rectangles; 2–3 comfortable lines before truncation.
  - **No scene number or production metadata**; "avoid showing ten icons."
  - Single click selects; double-click expands.
  - Instructional text appears only on the empty board.
- **Drag and drop** (§2.7, §81, §81.1):
  - Insertion line (blue, 3px with a dot) inside a sequence; destination highlight across sequences.
  - Commit only on release over a valid target. Invalid release returns the card with no half-move state. Valid drop commits immediately and can be undone.
  - Dragging an Act moves all its children. Moving a Sequence between Acts changes the parent and keeps identities (Mock 055).
  - Parking a card = dragging it to the Parking Lot. Restoring = dragging it out.
  - "Routine card movement does not require confirmation" (§108.2).
- **Scene Card context menu** (Mock 056): `Open` `Enter`, `Duplicate` `Ctrl+D`, `Move…`, `Send to Parking Lot`, `Convert to Screenplay Scene`, `Add Comment`, `Export`, separator, `Delete` `Del`.
- **Other context menus (§76)**
  - Beat: Open/Edit · Convert to Scene · Duplicate · Move · Park · Add Comment · Delete
  - Sequence: Open · Rename · Move · Add Scene · Add Beat · Add Comment · Delete
  - Act: Rename · Move · Add Sequence · Add Scene · Add Comment · Delete
- **Multi-select** (Mock 057): bar `3 cards selected | Move, Duplicate, Park, Export, Tag, Delete`. Modifier+click selects.
- **Parking Lot** (Mock 058; §80.5): different background and dashed border. Card examples: `Cool scene but doesn't fit: the wedding band`, `Cut for now: …`.
- **Duplicate** (Mock 059), callout: `Duplicate and experiment` / `Duplicate creates a brand-new, independent card with the same description, heading, notes and attachments.` / `Both cards live side by side so you can compare them visually and decide which survives. OpenFrame never tries to merge them.`
- **New Act** (Mock 051): inline title-only creation. An empty Act shows `Add a sequence or scene.` An empty Sequence (spec) shows `Drop scenes here.`
- **Beat drawer** (Mock 060, 340px):
  - Type `Beat Card`. Fields `Beat text`, `Note (optional)` (`Add a note…`), `Colour (optional)`.
  - Note: `Convert to Scene creates a new Scene Card from this text. The beat stays behind marked Converted so no thought is lost.`
  - Buttons: `Convert to Scene`, `Duplicate`, `Park`, `Delete`.
  - Beat states: New, Editing, Dragging, Converted, Parked.
- **Delete non-empty Act** (Mock 061, md):
  - Title `Delete “Act 2 — The Discovery”?` / `This Act contains 2 Sequences and 5 cards . Your creative material is not deleted unless you choose that.`
  - Radio `Move its contents to another Act` (tag `Recommended`) — "Then delete only the empty Act."
  - Radio `Delete the Act and everything inside it` — "The cards go to Recently Deleted and can be restored."
  - Field `Move contents to`. Buttons `Cancel`, `Move Contents and Delete Act`.
- **Delete linked card** (Mock 062, sm; §2.18):
  - Title `Delete this Scene Card?`
  - Body: `Its linked screenplay scene (Scene 3 — EXT. STREET — DAY) will remain. Screenplay scenes are never deleted from the Story Board.` / `The card moves to Recently Deleted. You can restore it or press Ctrl+Z.`
  - Buttons `Cancel`, `Delete Card`.
  - Canonical wording: `Delete this Scene Card? Its linked screenplay scene will remain.`
- **Build Screenplay preview** (Mock 063, lg 780px):
  - Title `Build Screenplay from Story Board` / `Preview the order before anything is created.`
  - Info: `Cards are used in Story Board order. Parking Lot cards are excluded. Nothing is created until you confirm.`
  - Table `# | Story Board scene | Heading | Include`. A missing heading shows the input `Add a heading…` with chip `Heading needed`.
  - `How to use the card description`: radio `Use as a scene planning note (default)` or `Insert as temporary action text`.
  - `Destination`: `New Screenplay` or `New Draft in the existing screenplay`.
  - Footer: `7 of 8 cards will be built · 1 needs a heading`, with `Cancel` and `Build Screenplay`.
- **Build when a script exists** (Mock 064, md):
  - Title `A screenplay already exists` / `BLACK RAIN already has a screenplay with written scenes. Building again will not overwrite it.`
  - Options: `Create a new draft` (“Draft 7 — from Story Board” is added; Draft 6 is untouched) or `Build a new screenplay document` (A separate screenplay alongside the current one).
  - Buttons `Cancel`, `Continue`.
- **Reorder warning** (Mock 065, md; HR-UX-003):
  - Title `Apply this order to the written screenplay?` / `Reordering these cards will change the screenplay scene order. Some of these scenes have already been written.`
  - Detail: `You moved “…” before “…”.` / `Screenplay Scene 24 would become Scene 23. Production planning, if any, is not reordered automatically.`
  - Buttons: `Cancel`, `Duplicate as Alternate Outline`, `Apply`.
- **Zoom** (Mock 066; §80.6): zoom changes density only. At low zoom, cards show one-line summaries (`Arjun steps off the night bus…`).
- **Export** (Mock 072, md):
  - Title `Export Story Board`; `Source: current Story Board`.
  - `What are you exporting?`: `Current page (whole Story Board)` / `Selected items only` / `Entire project`.
  - `Format`: `Board PDF` | `Outline PDF`.
  - Checkboxes `Include Parking Lot`, `Include comments`.
  - Note: `Private notes excluded. Export creates a snapshot and does not change your project.`
  - Buttons `Cancel`, `Export…`.
- **Sequence Hub drawer** (Mock 071, 340px): Sequence name and Act context, `Scenes (3)`, `Note`, `Attached references`, `Comments`, `Delete`.
- **States:** Empty, Populated, Dragging, Filtering, Card expanded, Saving, Save error, Build preview.
- **Shortcuts** (§11, §78.2):
  - Ctrl/Cmd+Z undo; Ctrl/Cmd+Shift+Z redo.
  - Arrow keys move focus through cards; Enter opens/edits; Delete deletes; Space toggles selection "if practical"; Modifier+click multi-selects.
  - Escape clears selection or closes detail.

### 3.7 Story Board — Outline view (Spec §12; Mock 052)
- Same toolbar with `Outline` active.
- Indented rows: Act (uppercase, shaded) → `Sequence — Hero Introduction` (indent 22px) → `Scene — Bus Station / Night` or `Beat — Body discovered` (indent 46px). Each row has a drag handle (grip) and expand/collapse.
- Controls: Add, Rename, Drag, Move, Duplicate, Park, Delete, Build Screenplay.
- Edits change the same objects as Board view. No screenplay text or production fields appear.
- Invalid hierarchical drops are rejected with an **explanatory inline message** (text [GAP]).
- The empty state shows "a concise hierarchy starter" (text [GAP]).

### 3.8 Scene Card detail drawer (Spec §17, §81; Mock 053, 390px)
- Type `Scene Card`; title = description.
- Location label `Act 1 — The Return › Railway Station Return`.
- Fields:
  - `Scene heading (optional now — required before it becomes a screenplay scene)`, for example `EXT. OLD RAILWAY STATION — NIGHT`.
  - `Short description` (primary; focused on a new card).
  - `Notes`.
  - `Attachments` (file chip plus `Attach`).
  - `Comments (2)` thread.
- Footer: `Duplicate`, `Send to Parking Lot`, `Convert to Screenplay Scene`, `Delete`, and a `Saved` indicator.
- **Related links** (§74.5): `Open Screenplay`, `Open Breakdown`, `Shots (4)`, `Storyboard (3)`, `Schedule — Day 7`. These navigate; they do not nest views.
- "No production or character fields are introduced here."
- Enter opens details; Escape closes.

### 3.9 Characters (PRD-STORY-006; spec section missing; Mocks 067–069)
- Header `Characters` / `A lightweight directory. Its most useful feature is the list of scenes each character appears in.` Segment `Board | Outline | Characters | Timeline`.
- **Left list** (max 420px): header `Characters 6` with `Add Character` (primary). Rows: round photo, uppercase name, role (for example `Protagonist`), chip `18 scenes`.
- **Right detail:**
  - Name plus subtitle `Protagonist · returning inspector`, with `Edit` and `Open Relationship Map`.
  - Sections: `Scenes this character appears in` (blue chips `Sc 1`…), `Relationships` (for example `Father of Ravi · Friend of Maya · Colleague of Meera`), `Notes`.
- **New Character** (Mock 068, md):
  - Fields `Name *`, `Role / title`, `Image`, `Short description` (placeholder `Add later`).
  - Duplicate check: `Possible match: a character named RAVI (Arjun's son) already exists. Open existing`.
  - Buttons: `Cancel`, `Create Another Character Anyway`, `Use Existing`.
- **Relationship view** (Mock 069):
  - Title `Character relationships`; edges like `ARJUN ── father of ── RAVI`.
  - `Back to Characters`, `Add Relationship`, and the note `Optional visual helper — not a character-analysis system.`
- §92: primary action `Add Character`; secondary Edit, Archive/Delete, Open Scenes, Relationships; forbidden: Payroll/HR.
- §96: Character delete/archive warns if linked; screenplay text remains. Name is required (§94).

### 3.10 Story Timeline (PRD-STORY-007, P1; Mock 070)
- Header `Story Timeline` / `Optional. Useful only when time continuity matters.` with `Assign Story Day`.
- Groups: `DAY 1 3 scenes` with rows such as `Sc 1 EXT. BUS STOP — NIGHT night`.
- `Unassigned 37 scenes` / `Scenes you have not given a story day yet. This is fine — story days are optional.`
- Soft warning card `Continuity note`: `Scene 9 (Day 1) refers to an event in Scene 12 (Day 3).` / `This is a soft warning. OpenFrame never reorders scenes for you.`

### 3.11 Screenplay — Standard mode (Spec §23, §82; Mocks 075–076)
- **Question:** "What is the movie on the page?"
- **Layout** (Mock 076):
  - Top bar (`.sp-top`, 46px):
    - Draft selector button `Draft 6 — Shooting Draft`.
    - Status chip `Locked` (green; `Draft` when unlocked).
    - Spacer, then `.sm` buttons: `Find`, `Notes`, `Comments`, `Drafts`, `Compare`, `Review`, `Export`, `Lock`.
  - Left navigator (210px): header `Scenes` with count chip `42` and a `+` (new scene); rows `12  INT. POLICE STATION — NIGHT`, current row accent-soft.
  - Centre: page on a grey backdrop.
  - Bottom-left of the page area: element indicator pill `Action · Enter → Character` (dark).
  - Right: optional tool panel (270px).
- **Spec controls:** New Scene, Element selector, Scene navigator, Search, Comments, Notes, Drafts, Compare, Export, Lock/Revision.
- **Empty state** (Mock 075):
  - `Write your screenplay or import one you already have.` / `Write from scratch, or build your first screenplay scenes from the Story Board. Start writing or use Build Screenplay.`
  - Buttons: `New Screenplay`, `Build from Story Board`, `Import Screenplay`. Spec text: `Start writing or use Build Screenplay.`
- **Behaviour:**
  - Scene numbers derive from order; the writer never types them.
  - The navigator jumps within the same document.
  - Selecting text shows a nearby `Comment` affordance that does not cover the selection (§82.4).
  - The locked status is a badge in the draft selector and does not dominate (§82.5).
- **Screenplay scene context menu** (Mock 098): `Open in Story Board`, `Break down`, `Create Shot List`, `Create Storyboard`, `Open in Schedule`, `Add Comment`, `Copy Scene`, `Export Scene`.
  - [MOCK≠SPEC] §76 omits `Open in Schedule`; §102 includes "Open in Schedule".
- **Scene Hub drawer** (Spec §38; Mock 097, 340px):
  - Title `Scene 12 — INT. POLICE STATION — NIGHT` / `What exists around this scene?`
  - Rows: `Screenplay` (Draft 6), `Breakdown` (Complete · 6 elements), `Shots (4)` (24A – 24D), `Storyboard (3)` (3 panels), `Schedule` (Shoot Day 7), `Comments (2)` (1 open), `Story Board` (Card found).
  - Note: `The hub is navigation and context, not a master edit screen. Each link opens the dedicated workspace focused on this scene.`
  - Empty: `Nothing created yet` plus the next creation action.
- **Reorder of a scene already in production** (Mock 099, md):
  - Title `This scene is already used in production planning` / `Changing its order will update its script number but will not automatically reorder the shooting schedule.`
  - Example: `Scene 12 — … is used in Breakdown, 4 shots and Shoot Day 7 . It will become Scene 9.` / `Production data follows the scene, not its number. Only the displayed number changes.`
  - Buttons `Cancel`, `Continue`.
- **States:** Loading, Editing, Searching, Comparing, Review overlay, Locked, Revision, Saving, Save error.
- **Shortcuts:** Ctrl/Cmd+F, Ctrl/Cmd+S, Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z, element shortcuts (§4).

### 3.12 Focus mode and Writing Room (Spec §24–25; Mocks 079–080)
- **Focus mode:** the page fills the window.
  - Minimal controls: `Draft 6`, `Exit Focus`, `Saved`.
  - No onboarding. Entering or exiting restores the previous layout. Save errors show discreetly.
- **Writing Room:**
  - Right panel tabs: `Story Board` | `Scene Notes` | `Characters`. The spec adds Comments and Draft Information.
  - Panel title `Story Board (reference only)`, footer `Viewing a card here never edits the screenplay or locks anything.`
  - One or two panels, resizable and closable. With no panel, the screenplay is centred.
  - Controls: Open panel, Switch panel, Resize, Close panel, Focus Mode.
  - The Idea Vault is not a permanent sidebar here.

### 3.13 Find and replace (Spec §26; Mock 078)
- Floating card top-right (390px): query input, `2 of 5`, up/down arrows, close.
- `Replace with…`, `Replace`, `Replace All`; checkboxes `Match case`, `Whole word`.
- Highlight colour `#ffe27a`.
- Empty result: `No matches found`.
- Replace is undoable and atomic (no partial replacement on failure).
- Keys: Enter next, Shift+Enter previous, Escape closes.

### 3.14 Notes and comments (Spec §30; Mocks 081–082)
- **Notes panel** (Mock 081): `Scene notes — Scene 12`.
  - `Private note` card (purple) with badge `Only you`; `Scene note` card.
  - Buttons `Add note`, `Add private note`.
  - Footer: `Notes are not printed and are excluded from PDF/FDX/Fountain/DOCX exports and review packages by default.`
- **Comments panel** (Mock 082): tabs `Open (2)` | `Resolved (1)`.
  - Thread card: author avatar, target (`Scene 12 · text comment`), quoted anchor `“RED FOLDER”`, body, replies (indented with a left rule), actions `Reply`, `Resolve`.
  - Resolved comments are dimmed (opacity .65).
- Spec controls: Comment, Reply, Resolve, Reopen, Private Note, Delete.
- States: Draft comment, Posted, Resolved, Private, Target deleted.
- Empty thread: `No comments yet.`
- Unmappable imported comments go to the Review Queue / Unmapped Review Note.

### 3.15 Drafts, New Draft, Compare, Delete Draft (Spec §27–28; Mocks 083–085, 101)
- **Drafts drawer** (450px):
  - Lineage line `Draft 1 → Draft 2 → … → Draft 6 — Shooting Draft`.
  - Draft rows with chips `Current` and `Locked`, date and note (for example `2 days ago · Locked for production`), and `Open`.
  - Section `Recovery History` with rows `Today 10:39 — automatic point` and `Restore`; note `Automatic history is for recovery. It is not a deliverable draft.`
  - Footer `Compare…`, `New Draft`.
  - Spec controls: New Draft, Open, Compare, Rename, Mark current, Restore recovery.
- **New Draft** (md):
  - `Create from`: `Current draft (Draft 6)` | `Previous named draft…`.
  - Fields `Draft name *`, `Note (optional)`.
  - Note `The source draft stays exactly as it is. The new draft becomes a separate version.`
  - Buttons `Cancel`, `Create Draft`.
- **Compare** (full workspace):
  - Left change navigator `Changes` with rows `Removed` / `Added` / `Changed` / `Same` (for example `Scene 34B added`, `12 lines changed in Scene 34`, `38 scenes unchanged`).
  - Top: A/B selectors (`→ Draft 7 — Director Rewrite`), segment `Scene-only` | `Text-only`, `Previous`, `Next change`.
  - Summary `Summary: 1 scene added · 1 scene removed · 2 scenes changed · 38 unchanged`.
  - Split panes `Draft 6 — before` and `Draft 7 — after` with diff highlights.
  - Read-only. Empty: `No differences found.`
- **Delete draft** (md):
  - Title `Delete “Draft 3 — Copy”?` / `The draft moves to Recently Deleted and can be restored. Your other drafts are not affected.`
  - Note: `The current draft cannot be deleted until another draft is made current. A locked draft cannot be deleted while it is the only production baseline.`
  - Buttons `Cancel`, `Delete Draft`.

### 3.16 Review rounds (Spec §29; Mocks 086–087)
- **Header:** review name (`Draft 3 — Producer Review`) and `Source draft: Draft 3 · Reviewers: Suresh, Priya · Deadline 20 Sep`, with `New Review` and `Add reviewer`.
- **Counters:** `Open notes: 12`, `Resolved: 8`, `In progress`.
- **Body:** comments grouped by scene with status chips `Open` / `In Discussion` / `Resolved`.
- **Side panel:** `Selected thread` with `Reply`, `In Discussion`, `Resolve`.
- **New Review** (md):
  - Fields `Review name *`, `Source draft *`, `Reviewers` (token chips plus `Add reviewer…`), `Deadline (optional)`.
  - Buttons `Cancel`, `Start Review`.
- The spec lifecycle is Open → In Discussion → Resolved, plus Reopen.
- Empty: a new review explains how to add the first note (text [GAP]).

### 3.17 Lock and revisions (Spec §31, §82.5–6; Mocks 088–090)
- **Lock dialog** (md):
  - Title `Lock this draft as the Shooting Draft?` / `Confirm the details, then lock.`
  - Summary rows: `Draft name`, `Current version`, `Unresolved review notes` (`3 open`), `Number of scenes`, `Last modified`.
  - Text: `Locking is a safety state, not a destructive action. The draft stays readable and exportable. Future edits will start a post-lock revision so the production baseline cannot be changed by accident.`
  - Buttons `Cancel`, `Lock as Shooting Draft`.
- **Typing into a locked draft** (sm):
  - Title `This is your locked shooting draft. Create a revision?`
  - Body: `You started typing in Draft 6 — Shooting Draft . To protect the production baseline, your edits will go into a new revision ; the locked draft stays exactly as it is.` / `Nothing has been changed yet.`
  - Buttons `Cancel`, `Start Revision`.
- **Revision panel:**
  - `Revision history` rows (for example `Revision A — Blue 4 Oct 2026 · Reason: Location change for the chase · 6 scenes changed`).
  - `Current revision`; the baseline row reads `Locked Draft 6 — Shooting Draft Locked 26 Sep 2026 · baseline`.
  - `Start Revision` form: `Label` (`Revision A`), `Colour` (`Blue` `Pink` `Yellow` `Green` `Custom…`), `Reason (optional)`.
  - `Changed scenes in Revision A` chips (`Sc 34B new`, `Sc 36 removed`).
- Spec controls: Lock, Start Revision, Set Revision Label/Color, Compare, View History.
- States: Unlocked, Lock dialog, Locked, Revision mode, Saving. A failure leaves the source unchanged.
- **[GAP]** The revision colour order and hex values are not specified. The FSD §177 lists this as an open question.

### 3.18 Screenplay Import (Spec §32; Mocks 091–094)
- **Step 1 — Select source** (lg 780px):
  - Title `Import Screenplay` / `What existing script do you want to bring into OpenFrame?`
  - Stepper `1 Select source · 2 Preview interpretation · 3 Import`.
  - `Choose a file` (`PDF · Final Draft (.fdx) · Fountain · TXT · DOCX`) with `Browse…`, or `…or paste screenplay text` (textarea).
  - Note: `Importing never replaces your current screenplay. If one exists, the script arrives as a new screenplay or a new draft.`
  - Buttons `Cancel`, `Preview`.
- **Step 2 — Preview** (lg):
  - Title `Import Screenplay — Preview` / `Check how the script was interpreted before anything is created.`
  - Stat tiles `Source`, `Scenes`, `Characters`, `Pages`.
  - Warning banner: `2 warnings. This PDF was interpreted with limited certainty. Review the items below before importing. You may continue with warnings.`
  - Table `# | Detected scene heading | Status` with values `OK` / `Uncertain heading` / `Potentially empty`.
  - `Import as`: `New Screenplay` | `New Draft in current screenplay`. Checkbox `Keep the original file in Project Files`.
  - Buttons `Back`, `Cancel`, `Import`.
- **Failure** (md; §73.5):
  - Title `Import failed. Your current project was not changed.`
  - Rows `File`, `Stage` (`Reading text from the PDF`), `Problem`, `Your project` (`Unchanged`), `Source file` (`Untouched`).
  - Buttons `Paste Text Instead`, `Add to Project Files`, `Close`.
- **Report** (md):
  - Title `Import complete` / `Imported as Draft 1 — blackrain_v3 (imported). Your other drafts were not changed.`
  - Rows: Source file, Imported scenes, Detected characters, Pages / length, `Warnings needing attention`.
  - Buttons `Review Warnings`, `Open Screenplay`.
- States: Selecting, Parsing, Preview, Warnings, Ready, Importing, Complete, Failed.
- Imports never create Story Board or production data automatically.

### 3.19 Screenplay Export, Print Preview, Title Page (Spec §33–35, §97, §107; Mocks 095–096, 100)
- **Export Screenplay** (lg):
  - Sub: `What am I exporting, which version, in which format, and where will it be saved?`
  - `Source` card: `Draft 6 — Shooting Draft` `Locked`.
  - `Format`: `PDF` (professional, printable), `Final Draft (.fdx)`, `Fountain`, `DOCX` (editable).
  - `Scope`: `Entire draft` | `Selected scenes`; `Clean script` | `Revision-marked version`.
  - Toggles: `Title page`, `Scene numbers`, `Revision info`, `Include notes`.
  - Note `Private notes excluded. Export creates a snapshot; your project does not change.`
  - Collapsed `Additional formats` (P2; §35: must not dominate or replace the core choices; unsupported formats stay visibly unsupported).
  - Buttons `Cancel`, `Preview…`, `Export…`.
- **Print preview:**
  - Page dominant (title page: title, `Written by`, author, `Blue Revision · 4 Oct 2026`, contact).
  - `Preview settings` with `Page numbers` etc.; `Page 1 of 96`, `Zoom 100%`; `Print`, `Export PDF`.
  - Note `Changing these settings affects only the outgoing document — never the screenplay itself.`
  - Always has a Back/Close action.
- **Title page** (md): fields `Title`, `Written by`, `Contact`, `Draft / revision line`, `Notes on the title page` (`Optional`). Note `Used when you export or print. Revision and draft information can be included by the export options.` Buttons `Cancel`, `Save`.
- Recipients never see internal IDs, comments, private notes or chrome unless the format intends it (§107.1).

### 3.20 Episodic / Series (Spec §36–37; Mocks 073–074)
- **Series home:**
  - `Series: Monsoon Diaries` / `Which episode am I working on?`
  - `Add Season`, `Add Episode`; season segment `Season 1` | `Season 2`; view segment `List` | `Season Board`.
  - Episode rows `E01 Pilot — First Rain` with summary and status chip (`Locked`, `Writing`, `Development`, `Idea`).
  - Side card `Series references`: `Recurring characters`, `Recurring locations`, with the note `Episodes can reference these lightweight lists. OpenFrame does not require you to manage cross-episode continuity.`
  - Nav, top bar and status work as normal (`Series · Monsoon Diaries`).
- **Season Board:** `Season 1 — Board` / `Plan the season at a glance. Drag episode cards to reorder.` with `Back to list`. Cards: `Episode N`, title, summary.
- Empty: `Create your first episode.`
- The continuity view (P2) is hidden when no data exists.

### 3.21 Production Home / Overview (Spec §54; Mocks 102–103)
- Sub-nav `Overview` active. Header `Production` / `What do I need to do next to prepare the shoot?` with `Start Production Setup`.
- Tiles:
  - `Script status` (`Shooting Draft 6` / `Locked · used as Production Source`)
  - `Breakdown progress` (`28 / 42` scenes marked complete)
  - `Unresolved locations` (`2` still Idea or Shortlisted)
  - `Unresolved cast` (`1` character without an actor)
  - `Schedule progress` (`30 / 42` scenes scheduled on 5 days)
  - `Next shooting day` (`Day 4` … `Latest call sheet: Day 2 (Finalized)`)
- `Needs attention` list with actions:
  - `Schedule conflict detected — Arjun is needed at two locations on Shoot Day 3.` `Open`
  - `Call Sheet needs refresh — Day 4 changed since it was generated.` `Review`
  - `3 scenes need breakdown review — Script changed in Scenes 24, 29 and 31.` `Review`
- The spec says only one sub-workspace is primary at a time, and unused tools should not show prominent empty cards. Each sub-workspace owns its empty state. [MOCK≠SPEC] The mock labels this overview "(dashboard)".
- **Start Production Setup** (md):
  - `Handing the script over to production.` / `Which screenplay draft should become the production source ?`
  - Radio cards: `Draft 6 — Shooting Draft` `Locked` `42 scenes · locked 26 Sep 2026`; `Draft 7 — Director Rewrite` `Not locked · 44 scenes`.
  - `OpenFrame will create or initialise`: `Breakdown scenes`, `Production catalog`, `Shot-list scene containers`, `Storyboard scene containers`, `Scheduling source`.
  - Note `You will not recreate any scene by hand. Your screenplay is not changed.`
  - Buttons `Cancel`, `Set as Production Source`.

### 3.22 Breakdown (Spec §39–40, §83; Mocks 104–112)
- **Question:** "What does this scene require to shoot?"
- **Layout** (three panes, content flush):
  - Top info banner: **`Production Source: Shooting Draft 6`** `· Locked · 42 scenes · Scene 12 of 42` with `Change source` (xs).
  - Left scene list (214px): `Scenes 42` plus a `Filter` chip. Rows `12 INT. POLICE STATION — NIGHT` with status chips below: green (done), `6 suggested` (accent), `Needs review` (yellow), `Needs breakdown` (red).
  - Centre: screenplay reading page (400px, 11.5px) with highlighted tagged text.
  - Right pane (352px):
    - Header `Scene 12 breakdown` with `Mark Complete`.
    - Warn banner `Suggested elements — 6` with `Review…`.
    - Collapsible category groups with counts (`Cast 2`, `Location / Set 1`, `Props 2`, `Wardrobe 0`).
    - Suggested rows use a hatched background: chip `Suggested` plus `Wet coat — suggested` with `Accept`, `Edit`, `Reject`.
    - Footer `Add Element`, `Suggest Elements`.
- **Default categories** (exact, 11): `Cast`, `Extras / Background`, `Location / Set`, `Props`, `Wardrobe`, `Vehicles`, `Hair / Makeup`, `Special Effects`, `VFX`, `Sound`, `Animals`. Empty categories stay collapsed or show a small count (§83.2).
- **Spec controls:** Select source, Choose scene, Highlight text, Tag, Add manual, Suggest, Accept/Edit/Reject, Catalog, Complete, Filter.
- **Tag highlighted text** (Mock 106): context menu `Tag “pistol ” as…` listing the 11 categories, then `Existing catalog match` `Pistol (Prop)` `Use existing`.
- **Suggestion review** (Mock 107, lg):
  - Title `Suggested elements — Scene 12` / `Grouped by category, with the text that triggered each suggestion.`
  - Warning `Suggestions are not production data yet. Accept, edit or reject each one.`
  - Rows: checkbox, name, category, `matched: “carrying a pistol”`, `Accept` `Edit` `Reject`.
  - Note `Existing catalog matches are suggested where obvious; ambiguous matches open a chooser.`
  - Footer `6 suggested · 6 selected` with `Dismiss`, `Accept Selected (6)`, `Accept All Safe (5)`.
  - Spec labels: Accept, Reject, Edit, Accept selected, Accept all safe, Dismiss.
  - Empty = a normal state with a reminder that manual tagging remains available. If the service fails, manual breakdown still works.
- **Existing match** (Mock 108, md; §84.3):
  - Title `Possible existing item: Red Folder` / `You are adding “Red Folder” (Props) to Scene 12. The catalog already has something similar.`
  - Options `Use existing: Red Folder` (`Prop · Required · used in Scenes 27, 38`) and `Create new: Red Folder (Scene 12 copy)`.
  - Note `If several items look similar you will see a list with distinguishing details and can choose one.`
  - Buttons `Cancel`, `Use Existing`.
- **Add Element** (Mock 109, md):
  - Fields `Category *`, `Name *` with typeahead (`Red Folder Existing · Prop`, `Create new item “red fold”`), `Notes (optional)` (`e.g. must look worn`).
  - Buttons `Cancel`, `Add to Scene 12`.
- **Script changed** (Mock 110, md):
  - Title `Scene 24 changed. Review breakdown differences.` / `The screenplay changed after this scene was broken down. Your production data has not been changed.`
  - Rows `Removed candidate` / `Added candidate`; `Impact` counts; shot list and storyboard review flags.
  - Buttons `Review Manually`, `Keep Production Data`, `Apply Suggested Update`.
- **Update production source** (Mock 111, lg):
  - Title `Update Production from Draft 7` / `Production is currently based on Draft 6 — Shooting Draft . A newer revision is available. Nothing changes until you confirm.`
  - Tiles `Scenes added`, `Scenes removed`, `Text changed`, `Heading changed`; rows tagged `Needs Breakdown` / `Historical` / `Review`.
  - Buttons `Dismiss`, `Keep Production on Draft 6`, `Review Changes First`, `Update Production Baseline`.
- **Needs Review queue** (Mock 112): `Needs Review` / `Scenes whose script changed after production planning began.` with `Back to Breakdown`, and per-scene impact text plus `Review`.
- **Empty state** (Mock 105; spec): `Choose a screenplay source to begin the breakdown.` / `Open a screenplay to begin breaking down scenes. Choose the draft that production should be based on.` Buttons `Select Source`, `Import Screenplay`.
- **States:** No Source, Loading, Reviewing Scene, Suggesting, Suggestions Ready, Complete, Needs Review, Save Error.
- A revision never silently deletes breakdown data. Complete scenes show a check and can be reopened.

### 3.23 Catalog (Spec §41, §84; Mocks 113–114)
- Header `Catalog` / `What things does this film need? Reusable items — not one huge spreadsheet.` with `Export` and `Add Item`.
- `Search catalog` plus a filter segment `All` `Props` `Wardrobe` `Vehicles` `Cast` `Locations` `More ▾`.
- Table `Name | Category | Status | Used in` (for example `Scenes 12, 27, 38`, `18 scenes`).
- **Detail drawer** (340px):
  - Type `Catalog item`. Fields `Category`, `Status`, `Description`, `Notes` (`Add notes…`), `Used in scenes` (chips).
  - Note `No financial or procurement fields — this is a production directory.`
  - Buttons `Archive`, `Open Scenes`.
- **Archive** (md):
  - Title `Archive “Red Folder”?` / `This item is still used in 3 scenes (12, 27, 38). Archiving keeps those associations readable but the item will no longer be offered for new selections.`
  - `Instead` / `You can also replace it in selected scenes with another catalog item without deleting it.`
  - Buttons `Cancel`, `Replace in Scenes…`, `Archive Item`.
- Empty: `Confirmed breakdown items will appear here, or add one manually.`
- Archived items are muted. Rename updates active references but not historical snapshots.
- Context menu (§76): Open · Rename · Open Scenes · Archive · Add Comment · Delete.

### 3.24 Locations (Spec §42, §85; Mocks 115–117)
- Header `Locations` / `Which places can we use? A practical scouting notebook.` with `Export` and `Add Location`.
- `Search locations` plus a filter segment `All` `Idea` `Shortlisted` `Confirmed` `Rejected`.
- Card grid: photo, name, status chip, `6 scenes`.
- **Detail drawer** (450px):
  - Header name plus the type `Location`; a status segment `Idea` `Shortlisted` `Confirmed` `Rejected` (simple, reversible).
  - Photos with `Add Photos` (multi, reorderable; the first is the thumbnail).
  - Fields `Address / area`, `Contact`.
  - `Practical notes` as labelled lines `Parking:`, `Noise:`, `Permission:`, `Access:`, `Power:`, `Toilets:`. The spec also lists nearby facilities and travel.
  - `Scenes` chips. Buttons `Delete`, `Open Scenes`.
- **Replace rejected location** (md):
  - Title `Replace a rejected location` / `District Hospital is Rejected. It is used in 3 scenes (16, 17, 26).`
  - Field `Practical replacement`; `Apply to scenes` checklist.
  - Note `This updates production associations only. The screenplay text (which still says HOSPITAL) is not changed; edit the script yourself if you want that.`
  - Buttons `Cancel`, `Apply Replacement`.
- Empty: `Add a location you are considering for the film.`
- Only the name is required. Delete warns that schedule history remains readable. Confirming does not auto-assign scenes.
- Context menu: Open · Edit · Open Scenes · Add Comment · Archive/Delete.

### 3.25 Cast & Crew (Spec §43, §86; Mocks 118–120)
- Header `Cast & Crew` / `Who is involved and what role do they have? A small-team directory — no payroll, no contracts.` with `Add Person`.
- Segment `Cast` | `Crew`, plus `Search`.
- **Cast table:** `Person | Character | Contact | Availability note | Scenes`. Unassigned character row: `— unassigned —` `THE STATION MASTER` `—` `Needs casting`.
- **Crew table:** `Person | Role * | Department | Contact`.
  - Footer: `Departments: Direction · Camera · Sound · Art · Costume · Makeup · Production · Editing. This is a production directory, not HR software.`
  - Crew requires Role/Department.
- **Assign character** (md):
  - Field `Person`; `Choose a character` list (`STATION MASTER 3 scenes · no actor`); `Create a new character`.
  - Note `One character has one primary actor by default. Alternates or double-casting can be added when needed.`
  - Buttons `Cancel`, `Assign`.
- Spec controls: Add Person, Assign Character, Edit, Availability, Search, Open Scenes, Archive/Delete.
- Empty: "concise guidance without enterprise language" (text [GAP]). Delete warns about references.

### 3.26 Moodboards (Spec §44, §87.1; Mocks 121–122)
- Header `Moodboards` / `What should this film feel like?` with `New Moodboard`.
- Left board list: `Overall Look`, `Cinematography`, `Production Design`, `Costume`, `Lighting`, `Character`, `Location`.
- Canvas: dot grid, freely positioned tiles with captions; the selected tile has a blue outline and a resize handle.
- Toolbar `Add image`, `Add note`, `Add link`, `Export`. Spec controls also include Move, Resize, Caption.
- Empty: `Drop images, notes and references here.` Missing images show a placeholder.
- **Export** (lg):
  - Title `Export moodboard — Overall Look`, with a preview.
  - `Include`: `Project title and board name`, `Images`, `Captions`, `Internal notes`. Note `Private / internal notes excluded unless you tick Internal notes.`
  - `Format`: `PDF` | `Image sheet`. Buttons `Cancel`, `Export…`.

### 3.27 Storyboards (Spec §45, §87.2–4; Mocks 123–125)
- Header `Storyboards` / `What will the audience see?` with `Export` and `Add Panel`.
- Scene header `Scene 12 — INT. POLICE STATION — NIGHT` / `3 panels`.
- Ordered panel cards: number badge, optional shot badge `12A`, visual, description (`Wide — Arjun enters, rain on his coat`). A trailing `Add Panel` tile.
- **Add Panel menu:** `Import Image`, `Draw / Sketch`, `Empty Panel`.
- **Sketch dialog** (md): `Draw / Sketch panel` with tools `Pen` `Eraser` `Undo`. Note `Basic sketching only. For polished art, import an image made elsewhere.` Buttons `Cancel`, `Save Panel`.
- Reorder by drag. The shot badge navigates; unlinking deletes neither object.
- Empty: `Create panels for this scene.`
- Context menu: Open · Move · Link Shot · Add Comment · Export · Delete.

### 3.28 Shot List (Spec §46, §88; Mocks 126–127)
- Header `Shot Lists` / `How will I capture this scene?` with `Export` and `Add Shot`.
- Grouped by scene (persistent selector); count `4 shots`.
- Shot rows: `12A` (accent, bold), size (`Wide`), description, chips `Static` `WS`.
- Footer hint: `Add only the shots you need. Camera details are optional. Shot letters are generated from order — you never renumber by hand.`
- **Detail drawer** (450px):
  - Title `12B — Medium — Meera looks up from the desk`.
  - `Description *`.
  - `Technical details optional — hidden until you need them`: `Shot size`, `Movement`, `Angle`, `Lens`, `Camera notes`, `Characters`, `Sound note`, `Storyboard` (`panel 2`, `Attach…`).
  - Buttons `Delete`, `Duplicate`.
- Empty: `Add only the shots you need. Camera details are optional.`
- Context menu: Open · Duplicate · Reorder/Move · Attach Storyboard · Add Comment · Export · Delete.

### 3.29 Shooting Schedule — Board, List, Calendar (Spec §47–49, §89–90; Mocks 128–139)
- **Empty** (Mock 128): `Create a shooting schedule from your screenplay.` / `Break down your script first, then move scenes into shooting days.` with `Create Shooting Schedule`. Spec empty (after creation): `Create a shooting day, then drag scenes into it.`
- **Create** (md): `Screenplay source` (`Draft 6 — Shooting Draft (Locked) · 42 scenes`). Note `All 42 scenes will start in the Unscheduled pool. Scenes that need a location or cast will show it from your Breakdown once available.` Buttons `Cancel`, `Create Schedule`.
- **Board** (Mock 130):
  - Info banner `Production Source: Shooting Draft 6 · Shooting Schedule · 42 scenes (30 scheduled, 12 unscheduled)`.
  - Toolbar: segment `Board` | `List` | `Calendar`; `Create Shooting Day` (primary); `Off Day`; `Suggest Grouping`; spacer; red chip `1 warning`; `Export`.
  - Left `Unscheduled 12` pool (300px, hatched).
  - Right: stacked day containers, header `Shoot Day 3 — Sun 13 Jun` with the total `3 scenes · 4h 30m`.
  - Strip anatomy: scene number · `EXT·N` · location · synopsis · cast initials pills (`AR` `ME`) · page count (`1 2/8`). Colour follows INT/EXT and D/N (§1.2).
  - Break markers are inline dashed strips: `MEAL BREAK`; spec examples `MEAL`, `TRAVEL`, `COMPANY MOVE`, `NOTE`.
  - Day header format per spec: `Day 4 — 14 June 2027`. A day may exist without a date.
- **Drag** (§89.3, Mock 133):
  - Insertion line within a day; destination highlight (`.daycol.hot`) across days.
  - Scheduling removes the scene from the pool. Removing it returns it to Unscheduled (the scene is not deleted).
  - Invalid drops revert. There is no automatic optimiser.
- **Markers menu** (Mock 134): `Add to this day`: `Meal Break`, `Travel`, `Company Move`, `Custom Note`, separator, `Set as Off Day`, `Add Day Note`.
- **List view** (Mock 131): `Sc | I/E · D/N | Location | Day | Pages | Est. time | Cast`. The Day column shows `Unscheduled` for pool scenes.
- **Calendar** (Mock 132): month `June 2027`, columns `MON`…`SUN`; cells `Shoot Day 1 4 scenes`, `Call sheet: Finalized`, `OFF`.
- **Shooting Day detail** (Spec §48; Mock 135):
  - Title `Shoot Day 4 — Mon 14 Jun 2027` / `What exactly are we doing on this day?` with `Edit date` and `Add break`.
  - Strips in order with `MEAL — 20:30`.
  - Footer `Total estimate: 5h 30m` with `Export` and `Create Call Sheet`.
  - Side card `Derived from the scenes`: `Locations`, `Cast`, `Estimated duration` (`5h 30m of 10h target`), `Day notes`. Note `Derived values are viewed here, not edited. Change scenes in the schedule.`
  - Empty day: `No scenes scheduled.` plus Add Scene / Off Day.
- **Conflicts** (Mock 136, drawer 340px):
  - Title `Schedule assistance` / `Practical observations`.
  - Example: `Potential actor conflict: Arjun is needed at two locations on Shoot Day 3.` listing the scenes, with `Open affected item`, `Move scene (you decide)`, `Keep anyway`, `Dismiss`.
  - Also `Estimated day duration exceeds the target (11h 00m of 10h).`
  - Footer `Warnings never block you and nothing is fixed automatically.`
  - Spec wording: `Potential location conflict: scenes require two locations at overlapping times.` / `Estimated day duration exceeds the target.` Warnings sit near the affected day or scene, with an acknowledged indicator after "keep".
- **Grouping suggestion** (drawer 340px):
  - `Scenes 12, 18 and 21 all use the same location.` (`Old Railway Station · confirmed`) / `You may reduce location changes by grouping them.`
  - Buttons `Ask for suggestion`, `Apply manually`, `Dismiss`.
  - Footer `Suggestions are advice. OpenFrame never rearranges your schedule silently. If nothing helpful is found it says: “No obvious grouping improvement found.”`
  - Spec controls: Ask for suggestion, Review, Dismiss, Apply manually, Keep anyway.
- **Script changed** (md):
  - Title `Script changed in 3 scheduled scenes` / `The production source script changed. Your schedule has not been changed.`
  - Rows tagged `Text changed` / `Heading changed` / `Estimate may change`.
  - Note `New scenes appear in Unscheduled. Removed scenes stay marked in the schedule history until you confirm removal.`
  - Buttons `Close`, `Open Production Update`.
- **Export** (md):
  - Title `Export Shooting Schedule`; `Source: Shooting Schedule · Shooting Draft 6`.
  - `Scope`: `Entire schedule` / `Current day only` / `Selected days`.
  - `Format`: `PDF` | `Spreadsheet (CSV / XLSX-style)`. Checkbox `Include unscheduled scenes`.
  - Note `Private notes excluded. Export is a snapshot of the schedule; the schedule is not changed.`
- **Responsive:** at medium widths show one day at a time and keep the pool accessible (§105.2).

### 3.30 Daily Production View (Spec §50, P1; Mock 140)
- Title `Today: Shoot Day 4` / `Mon 14 Jun 2027 · What do I need to know for this day?`
- Day selector `Shoot Day 4` and `Print / Export`.
- Summary cards:
  - `Scenes` (`12, 17, 19` · `3 scenes · 5h 30m estimated`)
  - `Location`
  - `Cast` (`Calls 18:15 / 18:45`)
  - `Call sheet` (`Needs refresh` · `Available — the schedule changed after it was created.` · `Open Call Sheet`)
  - `Shot list`
  - `Important notes`
  - `Important breakdown items` (chips)
- Mostly read-only. Empty: `Select or create a shooting day.`
- States: No day, Ready day, Day with issues, Finalized.

### 3.31 Call Sheets list, create, editor, stale, refresh, finalize (Spec §51–53, §91; Mocks 141–147)
- **List** (Mock 141):
  - `Call Sheets` / `Which shooting day document do I need?` with `Create Call Sheet`.
  - `Search`; filter segment `All` `Draft` `Needs refresh` `Finalized`.
  - Table `Shoot day | Date | Status` with `Open` and `Export`. Status examples: `Finalized`, `Draft`, `Source changed — needs refresh`.
  - Empty: `Create a call sheet from a shooting day.`
- **Create** (md):
  - Field `Source shooting day *`.
  - `Will be filled in automatically`: `Title, date, day number`, `Scenes`, `Cast`, `Location + address`, `Notes from the day`.
  - Note `You will only add call times and day-specific practical notes.` Buttons `Cancel`, `Create Call Sheet`.
- **Editor** (Mock 143):
  - Title `Call Sheet — Day 4` / `What does everyone need to know for this day?`
  - Paper document:
    - Centred `BLACK RAIN`, `SHOOT DAY 4 · MONDAY 14 JUNE 2027`, `CREW CALL: 18:00`.
    - Dark section bars `Location`, `Cast` (`Actor | Character | Call`), `Scenes` (`Sc | Heading | Description`), `Practical notes`.
    - Editable values use the dashed amber `.edt` style; missing values use the red `needs input` style.
  - Side card (290px):
    - `Source`, link `Shoot Day 4 — Mon 14 Jun`, chip `Ready`.
    - Hint `Derived fields (scenes, cast, location) come from the schedule. Editable fields (call times, notes, meeting point, attachments) use dashed highlights.`
    - Buttons `Add optional section`, `Refresh from schedule`, `Finalize…` (primary), `Export PDF`.
  - Spec paper order: Header, Production/day identity, Crew call, Cast calls, Location, Scenes, Practical notes, Emergency contacts, Optional weather/attachments. The editor list in §52 puts Scenes before Location/address.
- **Optional sections menu:** `Weather`, `Attachments`, `Reference images`, `Special notes`.
- **Stale banner** (Mock 145): `Source Schedule Changed — Review Update.` `This call sheet is based on an older schedule version. It remains readable and unchanged.` with `Update Call Sheet`.
- **Refresh preview** (md):
  - Title `Update Call Sheet — review changes` / `Nothing is applied until you click Apply Update .`
  - Table `Area | Now in call sheet | After update`.
  - Note `Your edited call times and notes are kept where they still apply.` Buttons `Cancel`, `Apply Update`.
  - Ambiguous refreshes open a review rather than overwriting.
- **Finalize and export** (lg):
  - Title `Finalize and export`, chip `Ready`, `Is this the version I am actually sending?`, small preview.
  - `Finalize` section: `Freezes this document as a stable, issued version. Later schedule changes will mark it Superseded but never change the issued file.`
  - `Export` section: `Creates the PDF snapshot to send.` with `Export PDF` and `Print`.
  - Footer `Refresh`, `Cancel`.
  - Finalize is clearly separate from Export. A finalized sheet shows a `Finalized` badge and reduced editing affordances. A missing required value returns to the editor with the field highlighted.
- Context menu: Open · Refresh · Finalize · Export · Add Comment · Delete.
- Editing the call sheet never syncs back to the schedule.

### 3.32 Sides (Spec §56, P1; Mock 148)
- `Sides` / `What script pages does this team need for this shooting day?`
- Setup panel: `Shooting day` select; scene checklist (`12 — INT. POLICE STATION — NIGHT`); `For`: `All cast` | `Selected cast`; checkboxes `Include call-sheet cover page`, `Show draft / revision name`; buttons `Preview`, `Export PDF`.
- Preview header `BLACK RAIN · SIDES · SHOOT DAY 4 · Draft 6 — Shooting Draft`.
- An unresolvable source asks the user to choose a valid one.

### 3.33 Reports (Spec §55, P1; Mock 149)
- `Reports` / `What concise production information do I need to see or send?` with `Print` and `Export`.
- Picker: `Scene Report`, `Location Report`, `Cast Scene Report`, `Prop Report`, `Schedule Report`, `Breakdown Completeness`.
- Read-only table.
- Empty: `No matching data.` with filters still visible.

### 3.34 Budget (Spec §57, P1; Mock 150)
- `Budget snapshot` / `How much are we roughly planning to spend?` with `Export` and `Add Line`.
- Banner: `Use a simple estimate for planning. Formal accounting is outside OpenFrame.` This is also the spec's empty-state text.
- Tiles `Planned total` (`₹ 12,00,000`), `Contingency` (`10% ₹ 1,20,000`), `Entered so far`.
- Table `Category | Line item | Amount`.
- A line requires a description and a non-negative amount. Negative or malformed values are rejected; valid lines are kept.

### 3.35 Notes and Tasks (Spec §58–59; Mock 151)
- `Notes & Tasks` / `Small things that do not belong anywhere else.` with `New Note` and `New Task`.
- `Project Notes` list: title, preview, date. The detail drawer holds title, body, attachment and related object.
- `Tasks 4 open`: checkbox rows with an optional related-object chip and due date; completed rows show `Done`.
- Spec controls:
  - Notes: New Note, Search, Edit, Attach, Relate, Print/Export, Delete.
  - Tasks: New Task, Mark Done, Reopen, Delete, Filter, Open Related.
- Empty states:
  - Notes: `Add a note for something that doesn't naturally belong in Idea Vault or another workspace.`
  - Tasks: `Add a small task only when you need to remember an action.`
- No Gantt, dependencies or time tracking.

### 3.36 Activity History (Spec §60; Mock 152)
- `Activity History` / `What important actions have happened in this project? Read-only.`
- Filter segment `All` `Drafts` `Story` `Production` `Exchange`.
- Rows: time · actor · action (for example `Nisha Verma locked Draft 6 — Shooting Draft`) · `Open`.
- Empty: `No major activity yet.` No edit or delete. Ordinary typing is never logged.

### 3.37 Templates (Spec §61, P1; Mock 153)
- `Templates` / `Would a saved starting point help me begin faster?` with `Create Template`.
- Sections `Built-in` (`Project starter` Feature · 3 acts; `Story Board starter`; `Call sheet layout`; `Simple budget`) and `Yours`.
- Controls: Preview, Use Template, Create Template, Rename, Delete.
- The blank path is always available.

### 3.38 Files, Permissions, Search, Recently Deleted (Spec §62–63; Mocks 154–157)
- **Files:**
  - `Project Files` / `What files belong with this film?` with `Add File`; `Search files`; segment `List` | `Grid`.
  - Table `Name | Type | Where | Modified`. `Where` values: `Stored in project`, `Linked (external)` + path, `Unavailable` + path. Row action `Open` or `Relink`.
  - Drag files in to add them.
  - Empty: `Keep project documents and attachments here.`
- **Permissions:**
  - `Who can see, edit or comment?` / `Simple project-level access. No enterprise roles.` with `Add Person`.
  - Banner `You are the Owner . Your role: full control. Private notes are visible only to their owner.`
  - Rows: avatar, name, role select, `Remove`.
  - Role legend:
    - `Owner` — "Full control: settings, permissions, sessions, deletion/restore."
    - `Editor` — "Edits allowed project content; not owner-only actions."
    - `Commenter` — "Reads and comments; cannot edit core content."
    - `Viewer` — "Read-only."
    - `Export-only` — "Creates permitted snapshots/packages; cannot join live editing."
  - Empty: `You are the owner.` Reducing permissions needs explicit confirmation.
- **Search:** see §2.8.
- **Recently Deleted (project):**
  - `Everything you delete stays recoverable until you delete it permanently.`
  - Columns `Item | Type | Deleted | Was in`; `Restore`, `Delete Permanently`.
  - Note `Delete Permanently asks for a second, deliberate confirmation. If the original place no longer exists, Restore puts the item in an Unassigned area and tells you.`
- **Delete / archive matrix (§96):** Act and Sequence confirm if non-empty; Scene Card warns if linked; Shooting Day confirm (call-sheet snapshots remain); Draft confirm if current or locked; Project strong confirmation; Task recoverable.

### 3.39 Share / Exchange, LAN, Backup, Portability, Offline (Spec §64–70; Mocks 158–173)
- **Share / Exchange menu** (from the screenplay): `Export Review Package (script)`, `Story Board Package`, `Breakdown Package`, `Shot List Package`, `Schedule Package`, `Call Sheet Review Package`, separator, `Import Exchange Package…`, `Start Collaboration Session…`.
- **Script Review Package** (md):
  - Sub `How do I send this draft to someone for review and safely bring their notes back?`
  - `Source draft`; `Scope`: `Full script` | `Script + comments` | `Specific scenes only`.
  - Toggles `Include comments`, `Include attachments`, and a disabled `Include private notes` (`Not allowed in review packages`).
  - Summary `This package will contain` (Project, Target, Exported by, Date, scenes, comments, `Private notes: excluded`).
  - Buttons `Cancel`, `Preview`, `Export Package…`.
  - Spec scope wording: `Full Script / Selected Scenes`.
- **Reviewer view:** banner `Review package — comments are separate from the original project.` with `Export Response Package`; the draft chip reads `Review package`.
- **Import Exchange Package** (lg):
  - Sub `What exactly will this imported package change?`
  - Tiles `Package type`, `Source project`, `Source version`, `Compatibility` (`Valid`).
  - Table `Changes by object | Count | State | Apply` with states `Safe mapping` / `Ambiguous` / `Unmapped` / `None`.
  - Footer `Validation happened before anything was changed.` with `Cancel`, `Accept All Safe`, `Import Selected`.
  - States: Valid, Stale, Conflict, Ambiguous, Unmapped, Rejected. Validation failure = zero mutation.
- **Stale package** (md):
  - Title `This review was created from Draft 3. Your current project is Draft 4.` / `Staleness does not mean rejection — it means the package needs review.`
  - Options `View the review without importing`, `Import comments only` (`Default`), `Create a separate review record`, `Compare versions`.
- **Review Queue:**
  - `Imported comments that need a decision. No comment is ever silently discarded.`
  - Sections `Ambiguous mapping (2)` (`Attach to Scene 9` / `Attach to Scene 12` / `Choose…`) and `Unmatched Review Notes (1)` (`Attach manually…`).
- **LAN host** (md):
  - Title `Start Collaboration Session` / `Who is joining and what will I share?`
  - `Shared scope`: `Entire Project` | `Story` | `Screenplay` | `Production`, with the scope explanation.
  - `Participants` with roles; `Join information` (`Session: BLACK RAIN · 192.168.1.24 · code 4F7-K2`, `Copy`).
  - Buttons `Cancel`, `Start Session`.
- **LAN join** (md): `Join Local Session` / `What am I allowed to work on?`; `Session`, `Your name`, `You will join as` (role plus scope); `Cancel`, `Join`.
- **Active session:** session bar (§2.11); presence pill on the object (`Priya is editing`) plus a soft-lock outline. It never locks anyone out permanently.
- **Conflict** (lg):
  - Title `Conflict in Scene 24` / `You and Priya changed the same part of this scene at the same time. Nothing has been discarded.`
  - Panes `Your version` / `Incoming version (Priya)` / `Context`.
  - Buttons `Review Differences`, `Manually Combine`, `Use Incoming`, `Keep Mine`.
- **Disconnected:** banner `The shared session is unavailable. Your project has not been deleted or changed. Your local work is safe.` with `Rejoin` and `Export recovery package`.
- **Create Backup** (md):
  - Fields `Backup name`, `Save to` + `Browse…`.
  - Hint `Choose another disk if you can. Project backups are separate from your active project.`
  - Contents table: `Included`, `External — not copied`, `Missing`.
- **Export Full Project** (md):
  - `A Full Project Package moves your whole working project to another PC. It is not a review package and not a PDF collection.`
  - `Include` checklist of workspace groups; `Private project transfer. Private notes are included because this is for you, not for outside reviewers.`
  - Buttons `Cancel`, `Export Project Package…`.
- **Project import collision** (md):
  - Title `This project already exists on this computer`.
  - Options `Open as Copy` (`Default`, creates “BLACK RAIN (copy)”) and `Replace Existing After Backup` (high-impact, re-confirmed).
  - `Import report`: `Validated ✓ · 2 linked external files are missing on this PC (the project still opens).`
  - Import never silently replaces the current project.
- **Drive lost** (md):
  - Title `The project drive was disconnected` / `OpenFrame can no longer reach E:\Films\BLACK RAIN. Your latest edits are kept in memory and have not been lost.` / `No second copy of the project has been created. Reconnect the drive and OpenFrame will continue saving to the same project.`
  - Buttons `Save Backup Copy…`, `Try Again`.
- **Offline** (Spec §69; Mock 173): normal state; the status strip shows `Offline · Local Project Saved`. Spec text: `Offline — your project is stored locally.` All core workflows remain enabled; AI may be unavailable. Never claim a cloud backup exists.

### 3.40 AI Assistant panel states (Spec §71–72; Mocks 174–181)
- **Panel anatomy** (360px, right):
  - Header `AI Assistant` with a mode chip (`Local model` / `Off`) and close.
  - `Scope` select (for example `Current Screenplay`).
  - Provenance line `Using: Draft 7 — Director Rewrite`, `Using: Whole Project`, or `Using: —`.
  - Conversation: user bubbles in ink on the right; assistant bubbles `#f3f0e8` on the left.
  - Input at the bottom: `Ask or command… e.g. “Open Scene 42.”`
- **Scope options:** Current Selection, Current Scene, Current Screenplay, Specific Draft, Story, Selected Idea Vault Items, Production, Shooting Day, Call Sheet, Whole Project. Mocks also show `Story Board`.
- **Answer** (Mock 174): exact results carry the green chip `Exact — from project data`, with actions `Show scenes` and `Copy`. Counts must be application-derived, not model estimates.
- **Navigate** (Mock 175): `Opening Scene 42 — … in the Screenplay.` / `Navigation commands run directly; nothing is changed.` Search result summary with `Open results`.
- **Suggest** (Mock 176): `I prepared a suggestion. Nothing has been added yet.` A change-set card (`Proposed Scene Card`: Description, Place in, Impact `1 new object`) with `Review Changes`, `Apply` (primary), `Cancel` (ghost).
- **Batch rename** (Mock 177):
  - Card `Proposed changes — 31 objects · 5 modules`.
  - Rows: `Character record 1`, `Screenplay Character cues 14`, `Cast / Breakdown references 7`, `Story Board / Shot / Storyboard refs 9`, `Excluded: raw dialogue/action text 12 mentions` (grey), `Locked / not allowed 1 (Draft 6 locked)` (red-soft).
  - The preview must show: counts, modules, structured references, excluded raw text, locked/unauthorised targets, conflicts, resulting state.
- **Stale** (Mock 178): `This proposal is out of date` / `The project changed after this proposal was prepared (2 scenes edited). It has not been applied.` with `Re-check & Review` and `Cancel`; footnote `AI never applies a stale proposal blindly.`
- **External disclosure** (Mock 179, dialog md):
  - Title `Send to an external AI provider?` / `Content from your project will leave OpenFrame and be sent to an external AI service.`
  - Rows `Provider`, `Context`, `Data sent`, `Private notes` (`Excluded`), `Why`.
  - Note `Exact counts (for example how many scenes) are computed inside OpenFrame and never need to be sent.`
  - Buttons `Cancel`, `Review context…`, `Send to External AI`.
  - Spec controls: Review context / Continue / Cancel. States: Awaiting consent, Sending, Canceled.
- **Unavailable** (Mock 180): chip `Off`; empty state `AI is not available right now` / `No local model is configured and there is no connection to an external provider. Everything else in OpenFrame works normally.` with `Set up AI…`.
- **Denied** (Mock 181): `I can't do part of that.` with bullets such as `You are a Commenter on this project, so deleting cards is not available to you.` / `Nothing was changed.` Private request: `The requested information is private and cannot be accessed in this context.`
- **All spec states:** AI off, Ready, Interpreting, Retrieving, Generating, Answer, Mutation preview, Awaiting approval, Applying, Applied, Rejected, Stale, Conflict, Failed, External disclosure, Unavailable.
- **Mutation flow:** resolve target/scope → check permissions/state → prepare exact Change Set → preview → explicit acceptance → apply through the normal action → undo/activity.
- The UI must never imply background monitoring.

### 3.41 Global empty/loading/error grammar (Spec §73)
- **Empty** = (1) what this area is, (2) what the user can do, (3) one primary action. No promo cards or tutorial videos.
- **Loading:** keep the page structure; use lightweight placeholders or a compact `Loading…` in the active pane. Never a full-screen spinner for local operations.
- **Unavailable file:** small badge plus `Relink` / `Open Location`.
- **Import failure:** filename, stage, whether changes were made.

### 3.42 Navigation rules (Spec §75, §102)
- `Open in…` targets:
  - Screenplay scene → Story Board, Screenplay, Breakdown, Shot List, Storyboard, Schedule.
  - Catalog Location → Locations, Scenes using it, Schedule where used.
  - Shooting Day → Schedule, Daily View, Call Sheet, Sides.
- The destination opens with the object focused. A breadcrumb or back action is available.

| Origin → destination | Label |
|---|---|
| Scene Card → Screenplay | `Convert to Screenplay Scene` / `Open Screenplay` |
| Scene Card → Breakdown | `Open Breakdown` |
| Screenplay scene → Story Board | `Open in Story Board` |
| Screenplay scene → Breakdown | `Break Down` |
| Screenplay scene → Shot List | `Create Shot List` |
| Screenplay scene → Storyboard | `Create Storyboard` |
| Screenplay scene → Schedule | `Open in Schedule` |
| Catalog / Location / Person → scenes | `Open Scenes` |
| Shooting Day → Call Sheet | `Create Call Sheet` |
| Call Sheet → Shooting Day | `Open Source Day` |

**Source banner labels (§95):** `Production Source: Draft N` (primary action `Review Production Update`), `Based on Draft N`, `From Shooting Day 4`, `Used in 3 scenes`.

---

## 4. Screenplay Editor Formatting

**Sources:**
- Spec §23, §82 and §78.3.
- FSD §15.5–15.6 and §16.1–16.6.
- PRD §35.
- CSS `.sp-*` rules and Mocks 076/077.

**Element types** (FSD §15.5, Mock 077 menu):

| Element | Mock shortcut | Mock CSS |
|---|---|---|
| `Scene Heading` | `Ctrl+1` | `.sp-h` bold, uppercase, margin 16px 0 8px |
| `Action` | `Ctrl+2` | `.sp-a` full width, margin-bottom 9px |
| `Character` | `Ctrl+3` | `.sp-c` uppercase, left margin **38%**, margin-top 10px |
| `Parenthetical` | `Ctrl+4` | `.sp-p` left margin **28%** |
| `Dialogue` | `Ctrl+5` | `.sp-d` left margin **17%**, width **64%** |
| `Transition` | `Ctrl+6` | `.sp-t` right-aligned, uppercase, margin 10px 0 |
| `Shot` | `Ctrl+7` | `.sp-sh` uppercase, margin-bottom 9px |
| `General note (not printed)` | — | Separated menu item; notes live outside the printed script |

The menu header reads `Element type`. The Ctrl+1…7 numbering comes from Mock 077 only; the spec says exact bindings "can be documented in the product's command list, but the UX must always provide an on-screen alternative."

**Page and font (mock values; the mock is a scaled 1200×720 render)**
- Page width `500px`, padding `30px 44px 0 60px`.
- Font `"Courier Prime","Courier New",Courier,monospace`, `12px`, line-height `1.45`, colour `#111`.
- Page number `12.` top-right (right 38px, top 10px, 11px).
- **Scene numbers on both left and right** of the heading (`.sn` at left:-40px and `.sn.r` at right:-40px), colour `#555`.
- The earlier CSS block (overridden) used 560px / 12.5px / padding 34 50 0 62 and fixed indents (Character 170px, Dialogue 80px + 300px width, Parenthetical 120px).
- The breakdown reading pane uses 400px / 11.5px.

**[GAP] Real print metrics are not in any source.** The Import/Export digest confirms this and FSD §177 lists it as an open question. Not specified:
- page size (Letter / A4), inch margins and indents
- font size in points (Courier 12pt is only an example in the I/E digest)
- lines per page, MORE / CONT'D handling
- A-numbering and revision marks

FSD §16.4 only says: "maintains standard screenplay page layout for PDF/printed output while allowing the writer to work continuously without manually inserting page breaks."

- Reference only, *not from source*: the common industry convention is 12pt Courier on US Letter with a 1.5" left margin and 1" right, top and bottom margins; dialogue at about 2.5"; parenthetical about 3.1"; character cue about 3.7"; transition right-aligned. The mock percentages roughly track this. This needs a product decision before coding.

**Enter / Tab element cycling**
- Enter rules from Mock 077, card `What Enter does`, and PRD §35:
  - `Scene Heading` → `Action`
  - `Character` → `Dialogue` → `Action`
  - The indicator shows `Action · Enter → Character`, so Enter after Action suggests Character. The card chip sequence reads Scene Heading, Action / Character, Dialogue, Action.
  - Card text: `Context rules suggest the next element; you can always override with the selector or a shortcut.`
  - FSD §16.2: "Contextual automatic selection may suggest the next likely type but must remain overridable."
- **[GAP] Tab behaviour is not specified anywhere.** No source mentions Tab cycling, Parenthetical-from-Dialogue, Transition triggers, or auto-uppercase and auto-complete of character names or headings.

**Other editor rules**
- Current paragraph: tinted `#fff6e6` with a 4px accent left bar.
- Persistent element indicator pill at the bottom-left of the page area (§82.3: "subtle element-type indicator").
- Standard cursor, selection, copy/paste and undo behaviour (FSD §16.1).
- New scene can be inserted from the navigator `+` or in-editor; its number comes from position (FSD §16.3).
- Scene headings are recognisable but not oversized (§82.2).
- The page is centred with a readable margin; chrome must not overpower it (§82.1).
- On narrow widths, close panels rather than compress the text (§105.3).
- Revision colours are not tinted into normal writing (§82.6).
- Notes can be shown or hidden and are excluded from export by default (FSD §16.6).
- Title page fields: Title, Written by, Contact, Draft / revision line, Notes (Mock 100; FSD §16.5).

**Screenplay shortcuts:** Ctrl/Cmd+F find (Enter next, Shift+Enter previous, Esc close); Ctrl/Cmd+S save; Ctrl/Cmd+Z undo; Ctrl/Cmd+Shift+Z redo; Ctrl+1…7 elements. Next/previous scene is required by FSD §51.5 and PRD §103, but the binding is a [GAP].

---

## 5. Accessibility Requirements

The UX spec has no dedicated accessibility section. The FSD says "exact implementation resides in the UX/accessibility specification", but that document does not exist. Requirements gathered from all sources:

**Functional requirements (FSD §127.2: "The desktop app must not assume mouse-only use.")**
- Keyboard focus can move between major controls.
- Important commands have keyboard shortcuts where practical.
- Drag/drop has a non-drag alternative where necessary (for example `Move…` menu items, Mocks 038/056; `Apply to scenes` checklists; Outline view).
- Forms expose labels to assistive technologies (mock fields always have visible `<label>`s; required fields marked `*` in red).
- Colour is not the only indicator of state (also Spec §104.1, §3.4). Examples: chips carry text (`Needs review`, `Suggested`); strips carry `INT·N`; revision colours carry labels.
- Selected and focused elements are visibly distinguishable (Spec §2.6 "clear focus treatment"; mock focus ring `0 0 0 3px #d9822b33`; selection outlines 2px).

**UX spec rules**
- Core workflows remain usable with the pointer, and shortcuts only accelerate them (§2.16).
- Tooltips explain unfamiliar icons; important functions keep text labels (§2.17, §3.6). Icon-only buttons only for compact secondary controls.
- Every screenplay shortcut has an on-screen alternative (§78.3).
- Escape closes the current drawer, dialog or panel and clears selection (§78.1, §77.3). No dedicated "Close Selection" button is required.
- Confirmation text states the consequence; no ambiguous `OK` / `Cancel` for risky decisions (§2.18, §73.6).
- Plain language over jargon (§0.3).
- Save and error state is shown as text in the status strip, not only as a dot.
- Responsive desktop (§105): wide / medium / narrow behaviour; collapsible navigation and drawers; fullscreen writing.
- Multi-window is optional; every workflow must work in one window (§106.3).

**Other inputs**
- PRD §2 non-functional: "Accessibility/basic input flexibility."
- The ESD tech architecture names "Radix UI or equivalent accessible primitives" for menus, dialogs and focus behaviour.

**[GAP]** Not specified anywhere:
- contrast ratios
- WCAG target level
- screen-reader announcements for live states (save / offline / toasts)
- reduced motion
- font scaling / zoom
- high-contrast or dark theme
- focus-trap and focus-return rules for dialogs and drawers
- ARIA roles for board, strip and canvas drag/drop

Recommendation (not from source): adopt WCAG 2.2 AA. Check for example `--muted #7a8291` on `#f3f1ec`, which is borderline for small text.

---

## 6. Global Keyboard Shortcuts

| Shortcut | Action | Scope | Source |
|---|---|---|---|
| Ctrl/Cmd+N | New Project (Home) / contextually new object; **New Scene Card** in Story / quick actions | Global / Story | Spec §4, §78.1; Mocks 020, 050 |
| Ctrl/Cmd+O | Open Project / Document | Home / global | Spec §4, §78.1 |
| Ctrl/Cmd+S | Save | Global / Screenplay | Spec §23, §78.1 |
| Ctrl/Cmd+Z | Undo | Global | Spec §11, §23, §78.1 |
| Ctrl/Cmd+Shift+Z | Redo | Global | Spec §11, §23, §78.1 |
| Ctrl/Cmd+F | Contextual search (Vault search; screenplay find bar) | Workspace | Spec §7, §26, §78.1 |
| Ctrl/Cmd+P | Print / Preview where appropriate | Documents | Spec §78.1 |
| **Ctrl+K** | Global project search (`Search this project…`) | Shell | Mock 011+ top bar ([MOCK only]) |
| Ctrl+Shift+N | Quick Capture / New Idea | Global | Mocks 020, 029 |
| Ctrl+D | Duplicate (Scene Card) | Story Board | Mock 056 |
| F2 | Rename (project card, Vault item) | Home, Vault | Mocks 005, 038 |
| Enter | Open selected item / open-edit card / open details | Vault, Story Board | Spec §7, §16, §78.2; Mocks 038, 056 |
| Delete (`Del`) | Delete selected (recoverable) | Vault, Story Board | Spec §7, §78.2 |
| Escape | Close drawer / dialog / panel / find bar; cancel inline edit; clear selection; cancel Quick Capture | Global | Spec §2.5, §16, §26, §77.3, §78.1 |
| Enter (inline edit) | Confirm edit | Global | Spec §2.5 |
| Enter or Ctrl+Enter | Save Quick Capture | Quick Capture | Spec §10; Mock 043 |
| Arrow keys | Move focus through cards | Story Board | Spec §11, §78.2 |
| Space | Toggle selection "if practical" | Story Board | Spec §78.2 |
| Shift / Cmd / Ctrl + click | Extend or toggle multi-select | Boards, lists | Spec §2.6, §78.2 |
| Enter / Shift+Enter | Next / previous match | Screenplay find | Spec §26 |
| Ctrl+1 … Ctrl+7 | Scene Heading, Action, Character, Parenthetical, Dialogue, Transition, Shot | Screenplay | Mock 077 ([MOCK only]) |
| Enter (screenplay) | Advance to the suggested next element (see §4) | Screenplay | PRD §35; Mock 077 |
| "Home shortcut" | Go to Project Home | Project | Spec §5 (binding [GAP]) |

**Required by FSD §51.5 / PRD §103 but with no binding [GAP]:** next / previous scene, create draft, add comment, export, switch to Focus Mode.

**Discoverability:** show shortcuts in tooltips and menus (right-aligned muted `.k`); do not show every shortcut permanently (§78.4). PRD §103 says bindings "should be documented and customizable later."

**macOS:** use Cmd for Ctrl where the spec writes "Ctrl/Cmd". Mock-only bindings are written as Ctrl because the mocks are Windows-only.

---

## 7. Mockup Inventory (182 files: 181 screens + index)

| Range | Topic |
|---|---|
| 001–023 | Getting started: desktop launch, splash, Home (populated / empty / menu / archived / unavailable), New Project (blank / filled / error), Project Home (new / active), project selector, shell anatomy, status menu, recovery offer, save error, unsaved switch, status panel, `+` quick actions, undo toast, project settings, project delete |
| 024–047 | Idea Vault: Grid, Card, List, Folder, empty, Add menu, note editor, image detail, Add URL, voice note, drag-in, partial result, unavailable file, multi-select, context menu, Send to Story, Global Vault, Copy to Project, search, Quick Capture, Recently Deleted, research collection, New Collection, item types |
| 048–074 | Story: empty, Board, Add menu, new Act, Outline, Scene Card drawer, card drag, sequence drag, context menu, multi-select, Parking Lot, duplicate, Beat, delete Act, delete linked card, Build preview, build when script exists, reorder warning, zoom, Characters, New Character, relationships, Timeline, Sequence Hub, Story export, Series home, Season Board |
| 075–101 | Screenplay: empty, editor, element menu, find, Focus, Writing Room, notes, comments, Drafts, New Draft, Compare, Review, New Review, Lock, locked edit, Revision, Import (source / preview / failed / report), Export, Print preview, Scene Hub, scene menu, production reorder warning, title page, delete draft |
| 102–127 | Production: Overview, Production Setup, Breakdown (scene / empty / tag / suggest / match / add / changed), production update, Needs Review, Catalog, catalog archive, Locations, location detail, location replace, Cast, assign, Crew, Moodboard, moodboard export, Storyboards, Add Panel, sketch, Shot List, shot detail |
| 128–157 | Schedule (empty / create / board / list / calendar / drag / markers / day / conflict / suggest / script changed / export), Daily View, Call Sheets (list / create / editor / optional / stale / refresh / finalize), Sides, Reports, Budget, Notes & Tasks, Activity, Templates, Files, Permissions, global search, Recently Deleted |
| 158–173 | Share menu, review package export, reviewer view, exchange import, stale package, Review Queue, LAN host / join / active / conflict / disconnect, backup, full project export / import, drive lost, offline |
| 174–181 | AI: answer, navigate, suggest, rename, stale, disclosure, unavailable, denied |

**Screens requested but not present as a distinct mock or spec section**
- **Recently Deleted**, **Search** and **Templates** exist.
- There is **no separate "Outline" workspace** beyond Story Board Outline view.
- **No "Import/Export dialogs" beyond those listed.**
- **No dedicated Offline page** (by design; §69 says it is a cross-cutting state).
- **No AI settings page** (`Set up AI…` target is a [GAP]).
- **No Help page** (`?` icon target is a [GAP]).

---

## 8. Spec ↔ Mock Discrepancies and Gaps

**Discrepancies**
1. Production sub-nav labels: `Schedule` / `Notes` (mock) versus "Shooting Schedule" / "Notes/Tasks" (spec). The mock adds `Overview`, which the mock index calls a "(dashboard)". The spec warns against dashboards; the mock version is limited to next-step status.
2. The global search shortcut **Ctrl+K** is mock-only. The spec defines Ctrl/Cmd+F as "contextual search".
3. The screenplay scene context menu adds `Open in Schedule` (mock), which is not in §76 but is in §102.
4. Scene Card context menu: the mock uses `Move…` and `Send to Parking Lot`; §76 uses "Move · Send to Parking Lot". The labels are consistent.
5. The Project Vault empty-state copy differs: spec `Start collecting anything about this film.`; mock uses the Global copy.
6. §52 editor order (Scenes before Location) differs from §91 paper order (Location before Scenes). The mock follows §91.
7. Status vocabulary: §3.3 lists 8 words, but mocks also use `Needs review`, `Needs breakdown`, `Needs refresh`, `Suggested`, `Superseded`, `Source changed`. Most of these are spec-sanctioned elsewhere (§39, §51, §83).

**Gaps**
1. Spec §18–22 are missing (Build Screenplay, Characters, Relationships, Timeline). Mocks are the only definition.
2. No dark theme or high-contrast mode.
3. No print metrics (page size, margins, points, MORE/CONT'D, A-numbering).
4. No Tab behaviour in the screenplay editor.
5. No revision colour order or hex values.
6. No accessibility spec (contrast, ARIA, focus management).
7. Many shortcuts have no binding (next/previous scene, create draft, add comment, export, Home).
8. Empty-state text is missing for: Outline view, Review round, Cast & Crew.
9. No mock exists for the Saving state, Offline-specific panel, Help, or AI settings.
