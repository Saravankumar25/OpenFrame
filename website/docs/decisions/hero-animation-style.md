# Decision: hero animation style

Status: decided. The owner chose the photo mosaic on 2026-11-04, and it is built: `src/assets/js/mosaic.js`, `src/assets/css/mosaic.css`,
`src/content/hero-photos.mjs`, `tests/mosaic.test.mjs`.

What shipped: a 1.15 s splash, then a film of 6.5 s in which a wall of 54 photographs (113 WebP files, 1.4 MB) fills the hero. The
tiles clear 7.1 s after load, 7.3 s on a phone, and the words remain. Skip or a key press clears them in under 0.7 s. A later
visit in the same session has no film, and with reduced motion no film runs and no photographs download.

Everything below this line is the proposal of 2026-10-31. It is superseded and kept as history: none of its styles, including the
recommended Aurora Gradient, was built.

## What the hero needs

Straft Studio is a calm, premium tool for writers and film teams. The hero is the first screen of the home page. Its
job is to set a mood and then get out of the way of the product pictures, which are the real proof.

Hard requirements, so any style that fails one is out whatever it scores elsewhere:

1. It respects the reduced-motion setting (`prefers-reduced-motion: reduce`) and shows a good still image instead.
2. It stays light on a low-end phone: no long main-thread work, no large downloads, no drain.
3. It does not compete with the product picture or the headline. Text on top keeps a contrast of at least 4.5 to 1 in
   both website themes (light, dark).
4. Moving content that starts by itself and runs longer than five seconds needs a way to pause, stop or hide it (WCAG
   2.2, success criterion 2.2.2, Level A). A small pause control meets this.
5. It follows the site rules: no inline styles or inline scripts (the content security policy forbids them), nothing
   loaded from another host, and no technology names shown to visitors.

## How the scores work

Each style is scored from 1 to 5 on five points. Higher is always better.

- Brand fit: calm, premium, warm, for writers and filmmakers. The palette is deep navy and cream, with a coral, orange
  and amber accent.
- Story fit: does it say something true about writing, film or planning, or is it decoration?
- Performance cost: 5 means almost free (compositor only), 1 means heavy GPU or CPU work.
- Accessibility: 5 means easy to make safe (no flashing, gentle motion, a clean still fallback), 1 means likely to
  disturb people with motion sensitivity.
- Build effort: 5 means a day or less, 1 means weeks.

Total is out of 25. The scores are judgement, written by one reviewer, and they are for ranking, not measurement.

## Scores

| # | Style | Brand | Story | Perf | A11y | Effort | Total | Verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | Aurora Gradient | 5 | 4 | 5 | 5 | 5 | 24 | Primary |
| 2 | Ink in Water (pre-rendered loop) | 4 | 5 | 4 | 4 | 2 | 19 | Alternative |
| 3 | Spirograph | 3 | 2 | 4 | 4 | 4 | 17 | Keep for small accents |
| 4 | Voronoi Cells | 3 | 3 | 3 | 4 | 3 | 16 | Reject for hero |
| 5 | Stained Glass | 2 | 2 | 4 | 4 | 3 | 15 | Reject |
| 6 | Boids Flocking | 3 | 3 | 3 | 3 | 3 | 15 | Reject |
| 7 | Origami Fold | 3 | 4 | 2 | 3 | 2 | 14 | Reject (cost) |
| 8 | Reaction Diffusion | 3 | 3 | 2 | 4 | 2 | 14 | Reject (cost) |
| 9 | Starfield Warp | 2 | 2 | 3 | 2 | 4 | 13 | Reject |
| 10 | Wireframe Terrain | 2 | 2 | 3 | 3 | 3 | 13 | Reject |
| 11 | Water Ripples | 3 | 3 | 2 | 3 | 2 | 13 | Reject |
| 12 | Slime Mold | 3 | 3 | 2 | 3 | 2 | 13 | Reject |
| 13 | Matrix Rain | 1 | 1 | 3 | 3 | 4 | 12 | Reject |
| 14 | Kaleidoscope | 2 | 2 | 3 | 2 | 3 | 12 | Reject |
| 15 | Chrome Metal | 2 | 2 | 2 | 3 | 2 | 11 | Reject |
| 16 | Low Poly 3D | 2 | 2 | 2 | 3 | 2 | 11 | Reject |
| 17 | Infinite Tunnel | 2 | 2 | 3 | 1 | 3 | 11 | Reject |
| 18 | Smoke Simulation | 3 | 3 | 1 | 3 | 1 | 11 | Reject |
| 19 | Fireworks | 1 | 1 | 3 | 2 | 3 | 10 | Reject |
| 20 | Isometric City | 2 | 1 | 2 | 3 | 1 | 9 | Reject |

## Notes on each style

- **Aurora Gradient (24).** Slow, soft colour fields drifting behind the content. Matches the calm, premium tone and the
  warm accent colours, works behind text if kept faint, needs no library, and can run entirely on the browser's
  compositor. The only story link is mood (a lit stage, a projector glow), which is why story is 4, not 5.
- **Ink in Water (19 as a pre-rendered loop).** The strongest story of the twenty: ink is how writing starts, and a drop
  blooming into a page-like cloud says "from idea to script" without a word. A real-time fluid simulation scores
  poorly (it needs a graphics shader and sustained GPU work, which would be 2 for performance), so the score assumes a
  short pre-rendered loop played as video, with a still image as fallback. The cost is effort: someone has to make the
  loop, and it has to tile without a visible jump.
- **Spirograph (17).** Geometric line art is cheap and tidy, and echoes the frame mark. But it reads as a toy and says
  nothing about writing or film. Fine as a small decorative detail, wrong for a hero.
- **Voronoi Cells (16).** Pleasing and light enough, but organic cells suggest biology and data science more than
  storyboards. A hand-built grid of frames would tell the film story better than this does.
- **Stained Glass (15).** Static in spirit and cheap, but the look is ornamental and dated for a modern tool.
- **Boids Flocking (15).** Agents moving together is a fair picture of a crew. In practice it is many small moving dots,
  which pull the eye away from the product, need script running every frame, and are hard to make restful.
- **Origami Fold (14).** Paper suits writers, and folding pages fits the story. Done properly it needs 3D transforms or a
  graphics shader, a lot of tuning, and care for low-end phones.
- **Reaction Diffusion (14).** Beautiful and calming, but it is a simulation that must run on the graphics card to be
  smooth, and the pattern language (spots, corals) does not connect to the product.
- **Starfield Warp (13).** Fast outward motion is the classic trigger for motion sickness. It also looks like a game.
- **Wireframe Terrain (13).** Reads as engineering or synthwave. Wrong tone.
- **Water Ripples (13).** Calm in principle, but it asks for pointer interaction and a shader, and phones have no pointer.
- **Slime Mold (13).** Interesting pattern, but it needs a large agent simulation on the graphics card and has no link to the
  product.
- **Matrix Rain (12).** Hacker imagery. Opposite of calm and of writers.
- **Kaleidoscope (12).** Busy, symmetrical, psychedelic. It also makes strong motion patterns that can disturb people.
- **Chrome Metal (11).** A dated look that needs reflections and heavy rendering.
- **Low Poly 3D (11).** Generic 3D, a graphics library to ship, and a mood that belongs to games.
- **Infinite Tunnel (11).** A zoom effect that is one of the worst offenders for motion sensitivity. It scores 1 on
  accessibility, which on its own removes it.
- **Smoke Simulation (11).** Moody, but a fluid simulation is costly and smoke behind text lowers readability.
- **Fireworks (10).** Celebration, not craft. Sudden bright bursts are also a flashing risk.
- **Isometric City (9).** Unrelated to the product and expensive to illustrate and animate well.

## Recommendation

**Primary: Aurora Gradient.** It is the only style that passes every requirement without needing a graphics library, a
large file, or a compromise on motion safety, and it fits a calm, premium, warm brand. It also costs almost nothing to
build and nothing to maintain.

**Alternative: Ink in Water as a pre-rendered loop.** Choose this if the owner wants the hero to say something about
writing, not only set a mood, and is willing to commission or make the loop. It is the better story and the worse
effort. It can also sit under the Aurora layer, so the two are not either-or.

Not recommended as the only hero element: anything in the Reject rows. Spirograph may be kept as a small accent
elsewhere in the page design if the design needs one.

## Implementation sketch: Aurora Gradient

Technique

- One container behind the hero text, `aria-hidden="true"`, no content in it, `pointer-events: none`.
- Three or four layers (child elements or pseudo-elements). Each layer is a very large `radial-gradient` that fades
  to transparent, using the brand colours (coral `#ff5d6c`, orange `#ff9a3d`, amber `#ffc23d`) at low opacity over the
  theme background. Soft edges come from the gradient itself, not from a blur filter. Large blur filters are the most common
  reason a "simple" animated background is slow on a phone, so do not use `filter: blur`.
- Each layer drifts with a slow `transform` (translate and a small scale) on a long, eased, alternating loop, 30 to
  60 seconds. Only `transform` and `opacity` are animated, so the work stays on the compositor and the main thread is not
  involved.
- Colours come from CSS custom properties, so the website light and dark themes each get a tuned version. In the light
  theme the gradient is a faint warm wash on cream; in the dark theme it is a glow on navy. Keep the layer opacity
  low enough that the headline passes 4.5 to 1 against the brightest part of the gradient. Check this with the
  accessibility test already in `tests/`.
- Everything lives in the stylesheet. No inline style, no new script host, so the content security policy is untouched.

Budget

- Size: under 1.5 KB of CSS, no JavaScript for the effect itself, no image, no font, no request.
- CPU: no main-thread cost while it runs. Three or four composited layers; on a phone this means a few megabytes of
  graphics memory and a small, steady GPU load.
- Low-end phones: at narrow widths use two layers, and use a longer loop. Pause the animation when the tab is hidden
  (browsers already do this for compositor animations) and when the hero scrolls out of view.
- Page weight and Largest Contentful Paint should not change. Confirm with Speed Observatory and Lighthouse before and
  after, and keep the change only if the numbers hold.

Reduced motion and pause

- `@media (prefers-reduced-motion: reduce)`: remove all animation. The layers stay at a well-composed resting position,
  so the still hero is a deliberate image, not a missing effect.
- A small pause control for the effect (success criterion 2.2.2 applies to anything that moves automatically for
  more than five seconds). It needs a real button with a visible label or an accessible name, keyboard operation, and a
  remembered choice in local storage wrapped so that a blocked store does not break the page. Without that control, limit the
  animation to five seconds and rest.
- Also stop the animation when the visitor has asked their device to save data (`navigator.connection.saveData`), where
  that is available.

Verification before it ships

- Home page Lighthouse performance and accessibility unchanged from the static hero.
- Browser test: with reduced motion emulated, no element has a running animation.
- Contrast check of headline and body text over the brightest and darkest part of the effect, in both website themes
  and both app-screen themes.
- Real phone check on a low-end Android device, not only a desktop with throttling.

## Implementation sketch: Ink in Water (alternative)

Technique

- Make a loop that joins cleanly, of 6 to 10 seconds: a few drops of coral and amber ink blooming and dissolving in clear navy. Make it
  in a 3D or compositing tool, or with a fluid generator, then render it once. The site never simulates fluid at run
  time.
- Encode as WebM (VP9) and MP4 (H.264) for coverage. 960 by 540 or smaller, no sound track, low frame rate (24),
  tuned for large soft gradients (use a high compression setting and test for banding).
- Markup: a `video` element with `muted`, `loop`, `playsinline`, `autoplay`, `preload="metadata"` and a `poster`
  image, `aria-hidden="true"`. Because the content security policy has no media rule, it falls back to
  `default-src 'self'`, which already allows files from this site. No policy change is needed.
- Start playback only when the hero is in view, and only if the visitor has not asked for reduced motion or data saving.
  A small script swaps the video in; without it the poster image shows.

Budget

- Size: about 300 to 600 KB for the video, 20 to 40 KB for the poster in AVIF or WebP. Do not exceed 700 KB total. This
  is the main cost, and it is paid only by people who can play it.
- CPU: video decoding uses the phone's hardware decoder, so it is usually light, but battery use is higher than a CSS
  gradient. Some phones in low-power mode refuse to autoplay; the poster covers that.
- Files: two or three added files out of the Pages Free limit of 20,000 (8,621 used on 2026-10-31).

Reduced motion and pause

- Reduced motion, data saving, low-power mode or script failure: show the poster image only. Make the poster a
  complete, good-looking still.
- Same pause control as above. Pausing sets the video to its first frame.

## What the owner needs to decide

1. Aurora Gradient as the hero, yes or no.
2. If Ink in Water is wanted, who produces the loop. Until then, nothing is blocked: Aurora can ship alone and Ink can
   be layered later.
3. Whether a pause control is shown on the page or the motion is limited to five seconds. The pause control is the
   friendlier choice.

Reference: WCAG 2.2, Understanding Success Criterion 2.2.2 Pause, Stop, Hide
(https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html).
