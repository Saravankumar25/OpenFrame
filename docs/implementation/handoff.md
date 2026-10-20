# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-21 (WindowsHTML session).

## State

1. **WindowsHTML done** (branch `docs/windows-html`, merged to `main` locally, not pushed): 809 screens and states
   of the Windows app, each in Light and Dark (1,618 pages), captured from the running app. All 1,618 pages match
   their screenshots in the same WebView2 at 1600x1000 (largest difference 0.460 percent, limit 0.5). 61 states that
   need Windows dialogs, devices or forced failures are listed in `WindowsHTML/README.md`. The old `HTML_Mockups/` is
   removed (owner request).
2. **How to regenerate:** build the debug app once, then `node scripts/windows-html/parallel.mjs` (eight parts side by
   side, merge, four-shard page check, about 50 minutes). Tour code: `tests/e2e/capture/` (kit, seed, ten sections),
   `tests/e2e/lib/capture.mjs`, `tests/e2e/specs/zz-capture.test.mjs` (skipped unless `E2E_CAPTURE=1`).
3. **App fix (owner asked, same session):** Application Home now refreshes at once after Pin, Unpin, Archive,
   Restore, Duplicate, Remove from List and Delete (the core emitted no list-change event for them). Rust test and
   the home E2E spec cover it; the spec no longer leaves Home to refresh.
4. **Installers:** `release-out/OpenFrame-3.exe` predates the last two sessions; the next build is `OpenFrame-4.exe`
   (only on the owner's word).
5. **GitHub:** not pushed. `windows/` on GitHub main matches local 6e28b33. Sync only on the owner's word, by the
   subtree method in memory; never `git pull` into `C:\OpenFrame`.

## Next items, in order

1. MacHTML on the Mac: prompt in [next-session-mac-html-prompt.md](next-session-mac-html-prompt.md) (owner time box 1.5 h).
2. Build and install `OpenFrame-4.exe` when the owner asks; owner hand test.
3. Writing grounding (67 percent) and the user manual refresh.

## Open blockers

- None.

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`. Do not commit
  `changes/`, `OpenFrameAI.docx` or `OpenFrameMac/`. Do not run two capture runs on the same `E2E_PORT_BASE`.
