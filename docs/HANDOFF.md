# Website handoff (2026-10-31)

State: the site follows the approved Website Share Package design (glass header, cream and navy, orange accent) with
three independent controls in the header: platform (Windows or Mac), website theme (light white, dark black) and app
screen theme. Every page embeds real captured app screens (819 Windows, 760 Mac, light and dark) from `/mockups/`.

- 736 pages: home, 15 feature pages with a 323-capability catalog, 390 Windows and 310 Mac written tutorials, a page
  listing every screen per platform, platform pages, pricing (₹2,999 in India, US$89 elsewhere, region detection plus a
  manual choice), download, about (company and founders), FAQ, legal.
- Content source: `src/content/manual/<win|mac>/` (31 Windows and 27 Mac areas). The two PDF manuals built from it
  are internal only (`npm run manuals`, git-ignored `manuals/`).
- Store buttons look normal and do nothing until listing links exist.

Next:
1. Owner review of the live site and the tutorial wording.
2. Store listing links when they exist (`OF_STORE_WIN`, `OF_STORE_MAC`).
3. Recapture the Mac assistant screens with the AI engine installed, then add Mac assistant tutorial steps.
