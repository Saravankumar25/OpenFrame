# ADR-0014: Appearance and theme system (Light, Dark, System, optional themes)

- **Status:** Accepted 2026-10-07 (owner answer in chat: follow the 15 themes, keep the default as it is, and let the user change it). Supersedes ADR-0012 §7 ("light theme only"). The 15 themes, the Light, Dark and System modes and the theme-coloured logo in the owner's `Trial_Licensing_Mocks/assets/themes.css` are the reference.
- **Date:** 2026-10-05
- **Related:** Screenplay and Production Depth Specification §44 and §47A; ADR-0012 §7; `docs/engineering/30-appearance-themes.md`

## Context

ADR-0012 §7 shipped a light theme only and required that a theme could be added later "without refactoring" and
"with a UX spec change and a new ADR". The supplied revised specification now asks for Light, Dark and System
modes plus about 15 built-in themes, user-imported theme files, and a theme-token audit of the whole UI. The
current repository has `apps/desktop/src/styles/openframe.css` with a central palette but no Appearance settings,
no `data-theme` handling and no `prefers-color-scheme` handling. Many colour literals remain in CSS and TSX.

## Decision

1. Mode (Light, Dark, System) and Theme (OpenFrame Default plus presets) are separate settings.
2. **OpenFrame Default is the current appearance, unchanged.** A user who never opens Appearance sees today's UI.
   **Amended 2026-10-07:** except six Light-mode colour pairs that were below 4.5 to 1; see "Amendment 2026-10-07".
3. All themeable UI colours come from semantic tokens. Every existing literal is classified as theme token,
   intentional fixed colour (stripboard day/night, printed call-sheet paper, placeholder imagery, status hues), or
   document/export colour. Only the first class becomes theme-driven.
4. Themes are **pure data**: seeds (neutral hue and tint, accent hue, chroma, lightness nudge) plus an optional
   whitelist of token overrides. The runtime generates one scoped style layer from validated values. Raw theme text
   never reaches the DOM.
5. Imported themes are untrusted: local file or paste only, 10 KB cap, strict schema, id `^[a-z0-9-]{3,40}$`,
   numeric bounds, overrides limited to `#hex` and `oklch(...)`, unknown major version rejected, contrast check
   (WCAG AA 4.5:1 target), preview before apply. A failed import changes nothing.
6. Appearance preferences and saved themes live in `app.sqlite` (new app migration `0002`), never in a project.
7. The theme never recolours screenplay output (PDF, FDX, Fountain, DOCX) or the printed call-sheet paper.
8. Windows High Contrast (forced-colors) must stay usable, as ADR-0012 §7 already requires.

## Consequences

- One tokenization pass over the UI is a prerequisite. A visual-regression baseline of OpenFrame Default is
  captured **before** that pass and must match after it.
- Adding a built-in theme is one registry entry, not component edits.
- No network, account or theme server is needed. The website may host downloadable theme files.
- ADR-0012 §7 gets a pointer to this ADR when it is accepted.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Dark mode only, no theme engine | Would be replaced by the theme engine later, causing rework. |
| Free colour editor | Complexity the spec explicitly avoids. |
| Store theme per project | Appearance is a user preference, not film data. |

## Amendment 2026-10-07 (owner decision)

The owner chose to make OpenFrame Default pass the contrast check in Light mode instead of keeping six recorded
exceptions (muted text on panels 3.86, on the window background 3.42, on table headers 3.67; white button text on the
accent 2.92; selected tab text 3.60; accent text on panels 4.08). Each pair is raised to just over 4.5 to 1 with the
smallest change, to keep the screen calm for long writing sessions: the window, panel and table header backgrounds do
not change, nothing moves to pure black, the muted and accent text get slightly darker, and button text on the accent
switches to the dark accent ink as three other built-in themes already do. Dark mode and the other fourteen themes do
not change. The contrast test no longer allows any exception for OpenFrame Default.
