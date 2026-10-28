> **Moved to history on 2026-10-28.** Finished: the MacHTML recapture ran on 2026-10-28 (see `MacHTML/README.md` and the progress log entry of that date). Not a current instruction.

# Next session prompt: MacHTML from the real app, then cleanup

> Written 2026-10-28 at the end of the Mac sync and release session, on the owner's order. Paste everything below the
> line into a new Claude Code session opened in this repository (the Mac tree, branch `mac/windows-sync`).

---

## Mission

Replace every MacHTML page with new pages captured today from the real OpenFrame Studio app, with the Screening Room
default theme, in Light and in Dark, covering the whole app including the features added on 2026-10-27. Then clean
up the repository. Time budget: about 2 to 3 hours for MacHTML.

## Why

Every page in the current `MacHTML/` is wrong: it was captured with the old, bland default theme, the window was not
maximized (some captures were from a small or inactive window), and some screens were not captured properly. None of
the old pages may remain.

## Read first, nothing more

1. `CLAUDE.md`, then `docs/implementation/handoff.md`.
2. `MacHTML/README.md` (how the pages are built: the window picture cut into its containers plus real HTML for every
   accessibility element), `apps/macos/E2E/capture.sh`, `scripts/mac-html/`, and the capture tour scenarios
   `apps/macos/Sources/OpenFrameE2EDriver/Scenarios/CaptureTour*.swift`.
3. The theme: `docs/engineering/30-appearance-themes.md` section 6 and
   `apps/macos/Sources/OpenFrameUI/Resources/macDefaultTheme.json`.

## Orders

1. **Wipe the old pages.** Delete every file in `MacHTML/` (pages, pictures, index) before the new capture, so not one
   old page is left. Keep only what the generator itself needs.
2. **Capture from the real app.** Build the current app (`apps/macos/scripts/bundle-dev-app.sh`) from the latest
   commit. Every state is captured from that running app; nothing is drawn by hand or copied from an older capture.
3. **Maximized window, always.** Before each state the project window fills the whole visible screen of this MacBook
   (zoomed to the full visible frame, not minimized, not the default size, not a small window). The window is key,
   frontmost and active (not dimmed) when the picture is taken. Change the capture tour so it does this for every
   state, and check it in the first few pages before running the rest.
4. **Both themes.** The default theme is Screening Room. Capture every state in Light and in Dark.
5. **Cover the whole app, old and new.** Every workspace, tab, sheet, dialog, menu state and empty, filled and error
   state the capture tour knows today, plus the features added on 2026-10-27, for example:
   - first-start library setup (Use the default location, Choose a new location, Locate existing data with its
     preview, the missing-library recovery screen);
   - Settings > Storage (library Move and Locate, folder for new projects, Global Idea Vault Back Up, Export Package,
     Restore with preview, Move, Locate, Add Them Back, app data) and Project Settings storage (Move Project, Make Them
     Mine) and the task sheets with progress;
   - Story page on the A4 sheet with formatting, colours, highlight, zoom and the Dark page;
   - Beat Board Vertical and Horizontal; relationship maps (one character, whole film);
   - screenplay Dark page and Light page; page setup with the first-page number box;
   - call sheet in the PDF order, Sides with Selected cast, Daily View export sheet with the suggested file name;
   - Home with a cover photo and "Found on this Mac";
   - the AI panel: progress lines, Cancel, See sources, Save conversation, and Settings > Offline AI > Assistant
     memory;
   - no team password screen anywhere (it was removed on 2026-10-27).
   Compare the capture list with the screens in the app and add any screen that has no page yet.
6. **One runner, nothing else on screen.** Run the capture with one runner (no `OPENFRAME_E2E_PARALLEL`), the screen
   unlocked, no other window over the app, the Mac kept awake (`caffeinate`). Do not run builds or tests while it runs.
7. **Check every page.** After the run, open every page (Light and Dark) and compare it with the real app at the same
   state: the whole maximized window, the right screen, the active look, the new theme, readable text, nothing cut
   off. Recapture any wrong page until it is right. Never leave a known-bad page. Run the existing page check
   (`scripts/mac-html/check-pages.swift` or its runner) and report its numbers.
8. **README.** Rewrite `MacHTML/README.md`: the date, the commit, the window size, the theme, how many states, what each
   page shows, how to regenerate, and any limit that remains (say it plainly).
9. **Commit** locally in a few commits (wipe, new pages, README). No push, no merge without the owner's word.

## Then: cleanup (the owner asked for it)

1. Merge `mac/windows-sync` into `main` locally (fast-forward), run the Rust, Swift and web tests on `main`, then delete
   the merged branches. Delete `HTML_Mockups/` and the branch `claude/website-html-mockups-9ecfa7` only after asking
   the owner in one plain line.
2. Remove finished builds and caches: `target/`, the app copies in `.build-cache`, old `release-out/e2e` runs, the
   release staging folder `$(getconf DARWIN_USER_TEMP_DIR)openframe-release-stage`. Keep `release-out/mac/OpenFrame-5.dmg`.
3. Dead code pass on the Swift app and the core; the two Word manuals in `docs/manual/` still describe the removed team
   password.
4. The open end-to-end items listed in the handoff.

## Rules

- No em dashes anywhere; plain words (`docs/engineering/35-writing-standard.md`).
- Clear build caches when free disk drops under about 20 GB (owner decision 2026-10-27).
- Simple changes may go to Sonnet subagents; decisions and deep debugging stay with Opus.
- End the session with the handoff, the progress log entry, `graphify update .`, local commits and a short report with
  a progress table.
