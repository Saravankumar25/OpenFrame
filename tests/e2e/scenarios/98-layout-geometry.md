# Layout geometry

Spec: `tests/e2e/specs/98-layout-geometry.test.mjs`

Tags: none (shared journey data).

Before: the shared data folder after the journey (01), which holds the project "Night Bus". Measurements are written to
`layout-geometry.json` in the artifacts folder.

## Open the project and add long text

1. Set the window to 1440 by 900 and open "Night Bus" from Your projects. If a recovery prompt appears, press
   "Keep Saved Version".
2. Through the core: add three Scene Cards with no act: a 303-character title, a 200-character address with no spaces, and a line that
   mixes Telugu, Tamil, Devanagari, an emoji and accented Latin.
3. Through the core: add a character with a 62-character name.

## Every screen at two window sizes, with the AI Assistant closed and open

This scenario runs six times: 1200 by 720 (the smallest window), 1280 by 720 and 1920 by 1080, each with the AI
Assistant closed and then open.

1. Set the window size. Open or close the AI Assistant with its top bar button.
2. Visit, in order: Home; Idea Vault; Story with its Board, Outline, Characters and Timeline views; Screenplay;
   Breakdown; Production with each of its 14 tabs; Call Sheets; Files; Settings (top bar gear).
3. On each screen, wait about half a second, then:
   - Check: the page itself does not scroll sideways.
   - Check: the AI Assistant and the navigation do not overlap the page.
   - Check: no button, tab or field of the page is outside the window, cut off by a box that cannot scroll, or covered
     by another element.
   - Check: no text is cut off inside its box unless a tooltip or label holds the full text.

## Notes

- A control inside a scrolling area that is scrolled out of view is not a finding; it can be scrolled to.
- Screen-reader-only text and the screenplay page itself are not checked for cut text.
