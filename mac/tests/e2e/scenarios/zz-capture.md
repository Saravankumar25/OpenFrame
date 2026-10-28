# WindowsHTML capture tour

Spec: `tests/e2e/specs/zz-capture.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. This spec is a development tool, not a product check: it is skipped
unless `E2E_CAPTURE=1` is set, so ordinary end-to-end runs report it as skipped.

## Capture every screen in Light and Dark and check each page against its screenshot

1. Seed the invented demo project "Night Bus" in the spec's own data folder.
2. Run every section in `tests/e2e/capture/sections/` in file-name order at a fixed 1600 by 1000 window.
3. Record each state in Light mode and in Dark mode.
4. Write the WindowsHTML pages.
5. Check: every page, opened in the same WebView2 at the same size, differs from its screenshot by less than the
   spec's maximum pixel ratio.
