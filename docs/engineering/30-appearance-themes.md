# 30: Appearance and themes (engineering)

> Status: **implemented** (2026-10-07, branch `area/12-themes`), following [ADR-0014](../adr/ADR-0014-appearance-theme-system.md)
> (Accepted 2026-10-07). Product behaviour is defined by Revision 1 §44 and §47A; this document says how it is built.
> Features: THEME-001 to THEME-008 and CORE-016 in the feature status file. Build area: Appearance and themes.
> Reference design: the owner's `Trial_Licensing_Mocks` (`assets/themes.css`, `assets/themes.js`, section 5 "Logo").

## 1. What the user gets

- **Mode:** Light, Dark or System. System follows Windows live (no restart). Mode and theme are separate settings.
- **Theme:** OpenFrame Default (today's look, unchanged), Black and White (added 2026-10-20 at the owner's request: black
  text on white in Light, white text on black in Dark), plus the fifteen themes of the reference mocks: Studio Amber,
  Slate Noir, Midnight Reel, Teal & Orange, Courier & Cream, Cutting Room, Velvet Curtain, Sunset Boulevard, Arctic
  Frame, Neon Reel, Location Scout, Desert Dune, Colorist Neutral, Silver Screen, High Contrast.
- **Import theme:** a `.json` file or pasted text, checked and previewed in Light and Dark before anything is saved.
- **My Themes:** apply, rename, export, delete. Deleting the active theme applies OpenFrame Default at once.
- **Logo:** the film-frame mark and the "OpenFrame" wordmark follow the theme (section 8).
- Where: Settings > Appearance (project menu > Appearance, or the Settings tabs) and Home > Appearance (palette icon).

## 2. Files

| Part | File |
|---|---|
| One list: engine constants and the built-in themes | `crates/openframe-application/src/modules/appearance/themes.json` |
| Validator, contrast, corrections (Rust) | `crates/openframe-application/src/modules/appearance/{theme.rs,color.rs}` |
| Operations and storage | `crates/openframe-application/src/modules/appearance/mod.rs` |
| App migration | `migrations/app/0002_appearance.sql` |
| Style layer, mode, System listener (webview) | `apps/desktop/src/theme/engine.ts`, `useAppearance.ts` |
| Logo | `apps/desktop/src/theme/BrandLogo.tsx`, `.brandlogo` rules in `styles/openframe.css` |
| Settings > Appearance | `apps/desktop/src/features/appearance/` |
| App icon source | `apps/desktop/src-tauri/icons/source.svg` |

## 3. Architecture

```text
themes.json (engine + built-ins) ─┐
My Themes (app.sqlite, validated) ┼─► engine.ts: one <style id="of-theme-layer"> ─► var(--token) in every component
import (file / paste) ─► Rust validator + contrast check ─► preview under data-theme="__preview"
```

- `<html data-theme="<id>" data-mode="light|dark" data-pref="light|dark|system">`. The layer holds: a light and a dark
  ladder (`[data-mode]` rules computing each token from the seeds with `oklch()`), the derived tokens (`color-mix()`),
  the component role tokens, the logo stops, and one block per theme (its seeds, then its per-mode overrides).
- **Adding a built-in theme is one entry in `themes.json`.** No component, CSS or TypeScript change; tests check that
  every theme validates, renders and passes contrast.
- Token families: background, backdrop, panel, surface2, surface3, well, chrome, hover, ink, ink2, muted, line, line2,
  side, side2, side-ink, side-mut, accent, accent-d (strong), accent-soft, accent-line, accent-ring, accent-ink,
  status (blue, green, red, purple, teal, yellow, each with `-soft`, `-t` text and `-line`), paper and paper-ink
  (fixed, section 7), lg1 to lg3 (logo).
- **Role tokens** (`engine.roles`) name a component colour that OpenFrame Default needs exactly but other themes derive
  (for example `beat-ink`, `stripe-a`, `placeholder`, `label-orange`). Every role has an exact OpenFrame Default value.
- WebView2 (Chromium) supports `oklch()`, `color-mix()` and relative colour syntax (used for the logo); the app ships
  on WebView2 only.
- Before the first frame `applyStartupHint()` applies the last built-in choice from a localStorage hint (re-checked
  input; the app database stays the source of truth), so a Dark user does not see a light flash.

## 4. Storage

`app.sqlite` migration `0002`: `sys_appearance_pref(key, value, updated_at)` with keys `mode` and `theme`;
`sys_user_theme(id, name, description, author, seeds_json, overrides_json, format_version, created_at, updated_at)`.
Never in a project. A missing row means System and OpenFrame Default, so an existing installation keeps today's look.
Every read validates stored themes again: a damaged or edited row is listed with its problem, cannot be applied or
exported, and an active theme that is unusable or deleted falls back to OpenFrame Default with a notice.

Operations (application level, hidden from the assistant, no undo step because they are preferences, only the local
person, never package changes or the assistant): `appearance.get`, `appearance.set_mode`, `appearance.set_theme`,
`appearance.check_theme` (reads the file the user picked or pasted text, saves nothing), `appearance.import_theme`
(reads and checks the source again), `appearance.rename_theme`, `appearance.export_theme` (writes where the user
chose, never inside OpenFrame's own folders, atomic write), `appearance.delete_theme` (destructive, confirmed).
My Themes holds at most 100 themes.

## 5. Validation (Rust)

File format (format version 1):

```json
{
  "formatVersion": 1,
  "id": "harbour-night",
  "name": "Harbour Night",
  "description": "Blue-grey surfaces with a lamp-orange accent.",
  "author": "Someone",
  "seeds": { "neutralHue": 230, "neutralTint": 0.02, "accentHue": 55, "accentChroma": 0.14, "accentLightnessNudge": 0 },
  "overrides": { "light": { "accent": "#c4562a" }, "dark": { "accent": "oklch(74% 0.14 55)" } }
}
```

- At most 10 KB (a file is read with a bounded read from the opened handle, which must be a regular file, so a file
  that grows or is swapped after the checks is still refused); UTF-8 with an optional byte order mark; one JSON object.
  A theme file on a network share is refused before anything is opened (no SMB connection is made for a theme).
- Only the person at this computer may check, import, rename, export or delete themes.
- Unknown keys anywhere are refused. `formatVersion` must be 1; a higher number says "made for a newer version".
  At most 20 problems are listed.
- `id` matches `^[a-z0-9-]{3,40}$`, is not a built-in id and is not already in My Themes.
- `name` 1 to 60 characters, `description` up to 200, `author` up to 80; no control, text-direction, invisible,
  filler, private-use or tag characters and no `<` or `>` (zero-width joiner and non-joiner stay allowed for Indian
  scripts); a name needs at least one letter or digit and may not be a built-in theme's name. Error messages never
  echo control or direction characters.
- The preview returns a SHA-256 `digest` of the checked text; saving passes it back and is refused if the file changed
  in between, so the saved theme is the one the user saw.
- Seeds (all required, numbers only): `neutralHue` 0 to 360, `neutralTint` 0 to 0.04, `accentHue` 0 to 360,
  `accentChroma` 0 to 0.25, `accentLightnessNudge` -12 to 16 (lightness points). JSON cannot carry NaN or Infinity;
  out-of-range numbers are refused.
- Overrides: only `bg panel surface2 surface3 hover well chrome ink ink2 muted line line2 side side2 side-ink side-mut
  accent accent-ink accent-d lg1 lg2 lg3`, each `#rgb`, `#rrggbb` or `oklch(L% C H)` with 0 to 100 %, chroma 0 to 0.4
  and hue 0 to 360. Anything else (`url(`, `@import`, `;`, braces, `var()`, `calc()`, names, `rgb()`, HTML) is refused.
  Built-in themes (reviewed source) may also set the derived, status and role tokens and `#rrggbbaa`.
- Values reach the webview as canonical text rebuilt from the parsed numbers (the contrast check runs on that same
  rounded text); the webview checks each value again against the same patterns and the same token list
  (`engine.importableTokens`) before writing the layer with `textContent`.
- Contrast (WCAG 2, 4.5 to 1) in Light and Dark for 21 pairs: main, secondary and muted text on the window background,
  panels and table headers; text on raised areas and hovered rows; status bar text; sidebar text; button text on the
  accent; selected and accent text; inverse text; the six status labels. Deterministic corrections that do not change
  a theme's character: button text switches between white and the dark accent ink; the accent moves up to 4 lightness points; the strong accent text takes more of the text
  colour (down to 40 % accent). The preview lists each correction. Anything else that fails is refused for use.

## 6. Built-in theme corrections and the default contrast

The fifteen themes use the mock seeds. Where the mock colours were below 4.5 to 1 in Light mode, the correction above
is stored in `themes.json` (Studio Amber, Cutting Room and Location Scout get dark button text as Velvet Curtain and
High Contrast already had; Slate Noir, Midnight Reel, Sunset Boulevard, Neon Reel and Colorist Neutral get the accent
1 or 2 points darker; Teal & Orange, Velvet Curtain and High Contrast get a darker strong accent text). Every built-in
theme then passes all pairs in both modes (`theme.rs` test `contrast_every_builtin_theme_passes_in_light_and_dark`).

**OpenFrame Default in Light mode is today's palette with three colours raised to 4.5 to 1** (owner decision
2026-10-07, ADR-0014 amendment). The six pairs that were below the line now pass with the smallest change, and no
background moves: muted text `#7a8291` becomes `#666e7c` (4.55 to 1 on the window background, was 3.42; on panels and
table headers too), the strong accent text `#b96a1a` becomes `#a15c17` (4.56 to 1 on the selected tab, was 3.60, and on
panels, was 4.08), and button text on the accent changes from white (2.92) to the theme's own ink `#1e2330` (5.36), as
Studio Amber, Cutting Room and Location Scout already do. Nothing becomes pure black, so the page stays calm for long
writing sessions. The contrast test allows no exception for any theme. The default-look screenshots are regenerated
with this reason in the final E2E run.
OpenFrame Default in Dark mode uses the Studio Amber seeds and passes every pair. High Contrast keeps 7 to 1 or more for
main and muted text in both modes.

## 7. Fixed and document colours

A colour literal in the React app must be a token, or be marked on its line or the line above as
`fixed colour: <reason>` or `document colour: <reason>`; `scripts/architecture-lint.mjs` rule T1 fails otherwise.
Only neutral black shadows are allowed unmarked. Shadows are tokens: `--shadow`, `--shadow-s` and `--shadow-side` (the side
panels and drawers). In Light mode they keep OpenFrame Default's navy tint, set once in `openframe.css` and marked as a fixed
colour so the default look stays unchanged; in Dark mode they are neutral black. A tinted shadow written anywhere else fails T1.
The marked classes:

| Class | Where | Reason |
|---|---|---|
| Document colour | screenplay page (`.spx-page`, `.sp-page` and their scene numbers, notes, current line, comment and search marks), print preview pages, call-sheet paper (`.paper`, `.edt`, `.need`), printed sides, sketch ink and paper, revision page colours | Pages look like paper in every theme and mode; the PDF and printed sheets never use the theme (ADR-0014 point 7). Tokens `--paper` and `--paper-ink` are set once and never by a theme. |
| Fixed colour | stripboard strips (`.strip.*`), placeholder imagery (`.photo.*`), avatar identity colours, the highlighter yellow (`.hl`), the dialog overlay, white text on the danger red, the switch knob, check boxes over image thumbnails, the image preview backdrop, the start-up screen | Recognisable conventions or imagery that must not follow the theme. |
| Theme token | everything else | |

Story card colour labels use fixed hues (`label-orange`, `label-pink`, `label-gray`, and the status hues), so a card
labelled orange stays orange in every theme.

## 8. Logo

Owner request 2026-10-07 (mocks section 5). `BrandLogo` renders the film-frame mark (24 by 24 grid, stroke 1.6, round
caps and joins) and the wordmark "OpenFrame" ("Open" weight 300, "Frame" weight 700, letter spacing -0.02em, no STUDIO
line). The gradient stops are `--lg1`, `--lg2`, `--lg3`: OpenFrame Default and Studio Amber use the website stops
`#ff5d6c`, `#ff9a3d`, `#ffc23d`; every other theme derives them from its accent (`oklch(from var(--accent) ...)`).
There is no separate logo setting. Each mark has its own gradient id (`useId`). Shown on Home, the project top bar,
the start-up screen and the theme preview cards. The window title bar uses the app icon; the app has no About box yet.

App icons: `apps/desktop/src-tauri/icons/source.svg` (the mark in the default gradient on the dark sidebar colour).
Regenerate the set with `npx tauri icon src-tauri/icons/source.svg` in `apps/desktop` and copy only the files that
already exist in `icons/` (the command also writes Android and iOS sets the app does not use); `icons/source.png` is
the transparent 1024 px render (`--ios-color "#00000000"`, `ios/AppIcon-512@2x.png`) that
`node scripts/build-msix.mjs --generate-assets` uses for `packaging/msix/Assets`.

## 9. Tests

| Area | Tests |
|---|---|
| Import safety | `tests/appearance.rs::security_hostile_theme_files_are_refused_and_change_nothing` (57 hostile inputs by text and file: unknown keys, ids, built-in and duplicate ids, oversize, invalid JSON, NaN, Infinity, overflow, out-of-range and non-numeric seeds, `url()`, `@import`, `;`, braces, `var()`, HTML, control and direction characters, contrast failures; preferences, My Themes and the open project unchanged), `security_review_findings_names_paths_actors_and_stale_previews` (invisible and built-in names, network paths, other users, bounded problem list, a file changed after its preview), `a_corrected_import_always_survives_being_read_back`, `the_ten_kilobyte_limit_is_exact`, `color.rs` parser tests, `engine.test.ts` style-layer guards |
| Persistence | `mode_and_theme_survive_a_restart`, `import_from_text_preview_save_apply_and_restart`, `a_damaged_or_missing_saved_theme_falls_back_to_default`, `rename_export_reimport_and_delete`, `migration_fixtures.rs::every_app_database_version_upgrades_and_keeps_recents_and_settings` |
| Contrast | `theme.rs` contrast tests; E2E `91-appearance-themes` measures rendered text in all 17 themes, both modes, 15 screens |
| Paper | `exports.rs::screenplay_pdf_and_call_sheet_paper_never_change_with_the_theme` (byte-identical PDF and Fountain under four theme and mode combinations), `engine.test.ts` paper test, E2E paper check |
| Default look | E2E `90-visual-baseline` (OpenFrame Default against the captured baseline) |
| UI | `AppearancePanel.test.tsx` (includes keyboard use of the mode and theme groups), E2E `60-appearance` (own data root, includes a relaunch), E2E `91-appearance-themes` (relaunch) |
| E2E status | The E2E specs above are written; the whole suite runs once at the end of session 2, so the feature rows stay Reachable until it passes. |
| Architecture | lint rule T1 with governance tests; `engine.test.ts` checks every `var(--token)` used by a screen is defined |
