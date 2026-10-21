# Design principles checklist for the OpenFrame website

Source: "Sorry, Steve" (apple-design skill by Emil Kowalski, 6-page PDF, 17 principles taken from Apple WWDC talks).
We take principles only. We do not copy apple.com layout, photography or colors. The brand stays: warm white, ink
#1d1a2e, one accent #d2441c, Inter / Segoe UI.

Source note. The PDF is a short promo for an interaction-and-motion skill. It gives hard numbers for motion, press
feedback, glass, tracking and drag only. It gives no type scale, line length, grid or spacing numbers. Items marked
[PDF] come from the document. Items marked [House] are our own defaults, chosen to fit the same principles. Change a
[House] value only with a reason.

## 1. Numbers taken from the PDF

| Topic | Value [PDF] |
|---|---|
| Press feedback | `transform: scale(.97)` in 100ms, starts on touch-down, not on release |
| Spring, default UI | damping 1.0, response 0.3 to 0.4 s (no overshoot) |
| Spring, move / reposition | damping 1.0, response 0.4 |
| Spring, rotation | damping 0.8, response 0.4 |
| Spring, drawer / sheet | damping 0.8, response 0.3 |
| Spring after a flick | damping about 0.8 (bounce only after a flick) |
| Drag start threshold | about 10px, then move 1:1 with the pointer and keep the grab point |
| Rubber-band past an edge | constant 0.55 |
| Momentum landing | deceleration rate 0.998 (predict where a throw lands, snap there) |
| Glass | `backdrop-filter: blur(20px) saturate(180%)`, one translucent layer only |
| Big headlines | `letter-spacing: -0.02em`, tracking tightens as size grows |
| Animate only | `transform` and `opacity` |
| Reduced motion | slides become cross-fades, no slide |

## 2. Typography [PDF principle 15 plus House values]

Principle [PDF]: tracking and line height change with size. Big headlines sit tighter than body text. Never one
letter-spacing for every size.

- [ ] Headline tracking tightens as size grows. Display size: -0.02em [PDF]. Body: 0 [House].
- [ ] Use three weights only: 400 body, 500 UI labels and nav, 600 headings [House]. No 800 or 900 shouting.
- [ ] Type scale [House], fluid with `clamp()`:
  - Hero: 44 to 64px, line height 1.05, tracking -0.02em, weight 600.
  - H2 section: 32 to 40px, line height 1.12, tracking -0.015em.
  - H3: 20 to 24px, line height 1.25, tracking -0.01em.
  - Body: 17px, line height 1.5, tracking 0.
  - Small / caption: 13 to 14px, line height 1.4, tracking 0 to +0.01em.
  - Eyebrow label: 12 to 13px caps, tracking +0.06em, weight 500, accent or muted ink.
- [ ] Line height falls as size rises (1.5 body, about 1.1 display). Check every heading, not only the hero.
- [ ] Line length: body copy 60 to 72 characters (about 34 to 40em wide). Docs prose max 72ch. Hero subtitle max 40ch
  [House].
- [ ] Headlines wrap with `text-wrap: balance`. Paragraphs use `text-wrap: pretty`. No single-word last lines.
- [ ] Numbers in tables and pricing use `font-variant-numeric: tabular-nums`.
- [ ] Code in docs: monospace at 0.9em of body, never a different line rhythm than the prose around it.

## 3. Hierarchy and restraint [PDF principles 16 and 17: Purpose, Simplicity, Craft]

- [ ] One job per screen section. If a section has two calls to action of equal weight, remove one.
- [ ] One accent (#d2441c). Use it for the single primary action per viewport, links in prose and focus rings.
  Never for decoration, never for more than about 5 percent of the visible area.
- [ ] Ink #1d1a2e carries hierarchy by weight and size first, then by tint. Muted text steps: 100 percent, 72 percent,
  56 percent ink. Muted text must still pass contrast (section 9).
- [ ] Each section has at most: eyebrow, one headline, one supporting paragraph, one visual, one action.
- [ ] Remove before adding. Delete any element that does not change what the reader does next.
- [ ] No decoration that competes with the product screenshot: no gradient blobs, no stock icons in circles.
- [ ] Honesty (Purpose, Responsibility): show only features that exist in the app. Match the real screens.

## 4. Spacing rhythm and grid [House]

- [ ] One base unit of 4px. Allowed spacing steps: 4, 8, 12, 16, 24, 32, 48, 64, 96, 128. No arbitrary 18px or 37px.
- [ ] Vertical section rhythm: 96px mobile, 128px desktop between major sections. 48 to 64px between a headline block and
  its visual.
- [ ] 12-column grid, max content width 1120px, marketing text column 680px, docs reading column 720px plus a 240px side
  nav and an optional 200px "on this page" rail.
- [ ] Page gutter 16px at phone width, 24px tablet, 32px desktop. No horizontal page scroll at any width.
- [ ] Proximity: space inside a group is at most half the space between groups.
- [ ] Optical alignment: pull large headings left by about 0.04em so the glyph edge, not the box, meets the column edge.
  Align icon and text baselines by eye, not by box. Round shapes (play buttons, avatars) look smaller than squares,
  so size them 4 to 6 percent larger.
- [ ] Radii: one family. Controls 8px, cards 12 to 16px, screenshot frames 12px, pills fully round. Nested radius =
  outer radius minus padding.

## 5. Content density

- [ ] Marketing pages: headline up to 8 words, supporting line up to 25 words, feature blurbs up to 20 words.
- [ ] Feature grids: 3 columns desktop, 1 column phone, never more than 6 items before a visual break.
- [ ] Docs pages: pages answer one task. Step lists use numbered steps of one action each. Callouts max 1 per screen.
- [ ] Tables only for comparable data. Otherwise use a sentence.
- [ ] Anything below the fold is a reward for scrolling, not a repeat of the hero.

## 6. Motion [PDF principles 1 to 9, 11, 14]

Rules.
- [ ] Respond on press, never on release. Buttons, cards and links: `:active { transform: scale(.97); }` with
  `transition: transform 100ms` [PDF]. Release returns on the spring below.
- [ ] Springs, not fixed ease-in-out curves [PDF]. Web default: damping 1.0, response 0.3 to 0.4 s. CSS fallback that
  approximates a critically damped spring: `cubic-bezier(.2, .8, .2, 1)` at 300 to 400ms [House].
- [ ] No bounce by default. Bounce (damping 0.8) only when the user threw something: a drawer or sheet flick [PDF].
- [ ] Interruptible [PDF principle 3]: any animation must be able to reverse mid-flight from its current value. Use CSS
  transitions (they retarget) rather than keyframes that restart. Never `animation-fill-mode` tricks that jump.
- [ ] Spatial consistency [PDF 7]: things leave the way they came in. A menu that slides in from the right slides out to the
  right. A card that expands from a thumbnail collapses back to it.
- [ ] Hint the direction [PDF 8]: in-between frames should point at the destination (a nav underline sliding to the next
  item, not fading in place).
- [ ] Animate only `transform` and `opacity` [PDF 11]. Never animate width, height, top, left, box-shadow or blur values.
  Add `will-change` only on elements that animate right now.
- [ ] Duration budget [House]: micro feedback 100ms; hover and focus 150ms; menus and popovers 200ms; section reveals 400
  to 500ms; nothing over 600ms. Exits are faster than entries (about 70 percent).
- [ ] Scroll reveals: at most one per section, 12 to 16px upward translate plus opacity, fires once, never replays.
  Content must be readable with JS or animation off.
- [ ] No parallax, no scroll-jacking, no autoplaying looping motion that does not carry meaning.
- [ ] Drag-like widgets (screenshot carousel, comparison slider): move 1:1 with the pointer, keep the grab point, start after
  about 10px [PDF], rubber-band at the ends with constant 0.55, use pointer capture, carry flick velocity into the
  settle.
- [ ] Reduced motion [PDF 14]:
  `@media (prefers-reduced-motion: reduce)` turns slides and scale into a 150ms cross-fade. No translate, no scale, no
  autoplay.

## 7. Interaction feedback [PDF principles 1, 10, 13]

- [ ] Every interactive element has five states designed: rest, hover, focus-visible, active (pressed), disabled.
- [ ] Hover is a refinement only (tint shift of about 4 to 6 percent ink, underline offset change). Never hide required
  information behind hover (touch has no hover).
- [ ] Focus ring: 2px solid accent, 2px offset, always visible on keyboard focus, never removed. Use `:focus-visible`.
- [ ] Hit targets at least 44 by 44px on touch. Small text links get padding, not a bigger font.
- [ ] Highlight on touch: use `-webkit-tap-highlight-color: transparent` only when a custom pressed state exists.
- [ ] Download button: after click, state changes at once (label "Starting download", spinner or check). Never a silent click.
- [ ] Forms: validate on blur, message beside the field in plain words, keep what the user typed.
- [ ] Copy-to-clipboard buttons in docs confirm within 100ms ("Copied") and revert after about 1.5s.
- [ ] Motion, sound and haptics fire together or not at all [PDF 13]. For the site: no sound.

## 8. Imagery treatment for product screenshots [House, principles Craft and Purpose]

- [ ] Show real app screens at true proportion, not mock-ups with invented UI. Capture at 2x, export WebP or AVIF with a PNG
  fallback, set `width` and `height` to prevent layout shift.
- [ ] Frame: 12px radius, 1px hairline border at 8 percent ink, one soft shadow (for example
  `0 24px 48px -24px rgb(29 26 46 / .25)`). No device chrome unless it is the real window title bar.
- [ ] One hero screenshot per page, then smaller supporting crops. Crop to the feature, do not shrink a whole window until
  the text is unreadable. Minimum rendered UI text equals about 11px.
- [ ] Light and dark versions: serve the matching one via `<picture>` and `prefers-color-scheme` (the app has both).
- [ ] If the nav or a floating bar uses glass, use one layer only: `backdrop-filter: blur(20px) saturate(180%)` over a
  tinted background of about 70 to 80 percent opacity. Never glass on glass. Provide a solid fallback when
  `prefers-reduced-transparency` is set or `backdrop-filter` is unsupported [PDF 12].
- [ ] Lazy-load below the fold, eager-load and `fetchpriority="high"` the hero image. Always set meaningful `alt`.
- [ ] Captions state what the screen does in one plain sentence, not a slogan.

## 9. Accessibility

- [ ] Text contrast at least 4.5:1 (3:1 for 24px and larger). Verify the accent #d2441c against warm white for body-size
  links. Where it falls short for small text, darken the link variant or underline it.
- [ ] Never use color alone to carry meaning. Links in prose are underlined.
- [ ] Honor `prefers-reduced-motion` (section 6), `prefers-reduced-transparency` and `prefers-contrast: more` [PDF
  prompt: reduced-motion, reduced-transparency and increased-contrast versions of every animation and glass
  surface]. Under increased contrast, borders go to 30 percent ink and glass becomes solid.
- [ ] Semantic landmarks (`header`, `nav`, `main`, `footer`), one `h1` per page, no skipped heading levels.
- [ ] Skip link to main content, first focusable element.
- [ ] Docs side nav and menus work by keyboard (Tab, Arrow, Escape). Disclosures use `button` with `aria-expanded`.
- [ ] Text scales: layout survives 200 percent zoom and 320px width. Use `rem`, never fixed pixel line heights.
- [ ] Test with the keyboard only and with a screen reader on the home page, the download page and one docs page.

## 10. Responsive behaviour

- [ ] Breakpoints [House]: 640, 900, 1200px. Design phone first. Fluid type with `clamp()`, fixed breakpoints only for
  layout changes.
- [ ] Phone: single column, 16px gutter, nav collapses to one button plus the primary action, screenshots full width.
- [ ] Docs on phone: side nav becomes a top "Contents" disclosure. Code blocks scroll inside their own box, never the page.
- [ ] Do not hide content on small screens. Reorder or stack it.
- [ ] Hero headline stays under 4 lines at 375px width.
- [ ] Test at 375, 768, 1280 and 1920px. At 1920px content stays on the 1120px grid, it never stretches.
- [ ] Windows download page detects the platform only to highlight the right button, never to hide the other.

## 11. Copy tone [PDF: Purpose, Simplicity, plain English; project writing standard]

- [ ] Plain words, short sentences, active voice. Say what the app does for a writer or a production team.
- [ ] No em dashes. No AI-sounding phrases ("seamless", "unlock", "elevate", "supercharge", "game-changer", "in today's
  world"). Follow `docs/engineering/35-writing-standard.md` in the main repository.
- [ ] No exaggerated claims. Offline AI, local-first and team password are stated as facts with their limits. The
  builds are unsigned and the AI downloads after install: say so near the download button.
- [ ] Buttons name the action: "Download for Windows", not "Get started". Links say where they go.
- [ ] Headlines state the benefit in under 8 words. Subheads add the one detail that makes it believable.
- [ ] Error and empty states in docs and forms say what happened and the next step.

## 12. Process before shipping [PDF principle 17: prototype, play it in slow motion, test with real people]

Six-point check adapted from the PDF "test before you ship in 60 seconds":
- [ ] Press and hold a button: it shrinks the moment you touch it, before you let go.
- [ ] Open a menu or lightbox and close it halfway: it turns around from where it is, no jump, no restart.
- [ ] Drag or flick a carousel: it moves 1:1, keeps your speed, settles softly.
- [ ] Drag past the end of a carousel: it stretches, resists, springs back.
- [ ] Turn on Reduce Motion: slides become quick fades.
- [ ] Look at the biggest headline: letters sit tighter than the body text.
- [ ] Play every transition at 10 percent speed in dev tools to catch jumps.
- [ ] Run Lighthouse on home, download and one docs page. Keep accessibility at 100 and layout shift under 0.1.

## 13. Anti-patterns (from the PDF, plus site equivalents)

From the PDF "without the skill" column:
- Reacting on release instead of on touch.
- Animations that jump or restart when interrupted mid-flight.
- A fixed ease-in-out for everything, and bounce on everything.
- Dragged items that lag or jump under the finger instead of moving 1:1.
- Hard stops at edges with no resistance, so the page feels frozen.
- Glass on glass, which leaves muddy text.
- The same letter-spacing at every headline size.
- Ignoring Reduce Motion.
- Animating anything other than `transform` and `opacity`.
- Light glass over a light background.

Site-specific additions [House]:
- Copying Apple colors, photography, big black hero bands or SF Pro. We keep our own brand.
- More than one accent color, or the accent used as a fill for large areas.
- Autoplaying hero video with no pause control.
- Scroll-jacking, parallax and scroll-linked pinned sections.
- Screenshots of invented UI, or features the app does not have.
- Hiding the unsigned-build and download-after-install facts.
- Cookie banners, popups and chat widgets over the first screen.
- Walls of text in marketing sections, or marketing fluff in docs.

## 14. Honest limits

- The PDF is a promo for a web-and-iOS interaction skill. It does not cover spacing, grids, line length or type
  size. Those items are [House] defaults and should be tuned against the real pages.
- The skill is not from Apple. Treat the numbers as sound starting points and verify by feel on the real site, using the
  section 12 checks.
