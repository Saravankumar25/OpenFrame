> **SUPPLIED PROMPT, kept for reference (2026-10-05).** Method only; it is not a specification. Tool and repository names
> in it are **unverified**: install nothing from a name alone, check source, licence and permissions first, and do not send source code to
> third-party model providers. Plan and rules that apply to this repository: [../README.md](../README.md).

# Claude Code Prompt: Premium Website Redesign + Motion + Anti-AI-Slop

You are acting as a **senior product designer, design engineer, motion designer, and frontend engineer**.

Your job is to take my existing website and make it feel **distinctive, premium, memorable, highly polished, and technically production-ready**.

I want the website to create a genuine “wow” reaction through excellent composition, typography, motion, micro-interactions, visual storytelling, and responsive behavior. I do **not** want a noisy demo page full of random effects.

The final result must still feel like a real product, load fast, remain accessible, and preserve all working functionality.

---

## 1. Start by understanding the existing website

Before changing code:

1. Read the repository structure.
2. Identify the framework, styling system, component library, animation libraries, fonts, routes, assets, and build/deployment setup.
3. Inspect the existing design system, spacing, colors, typography, buttons, navigation, page sections, responsive behavior, and current animations.
4. Identify anything that already works well and should be preserved.
5. Check the desktop and mobile screenshots I provide.
6. Do **not** start by replacing the whole UI or installing a large collection of dependencies.

First produce a short design diagnosis covering:

- visual hierarchy,
- typography,
- layout and grid,
- spacing rhythm,
- brand distinctiveness,
- section composition,
- CTA clarity,
- image treatment,
- mobile behavior,
- micro-interactions,
- animation quality,
- loading and transition states,
- accessibility,
- obvious “AI-generated website” patterns.

Then identify the **5 to 8 highest-impact improvements**.

---

# 2. Use CollectUI as design research, not as a template source

Reference:

https://collectui.com/

Use my selected CollectUI screenshots or references to study:

- hierarchy,
- section structure,
- spacing,
- typography scale,
- CTA placement,
- image composition,
- navigation behavior,
- responsive layout,
- interaction patterns.

Rules:

- Never clone a CollectUI design.
- Never copy another creator’s logo, written copy, images, illustrations, or brand identity.
- Never reproduce an exact layout.
- Extract general design principles and reinterpret them for this product.
- Prefer one strong main reference and at most two supporting references.
- Keep the final composition original.

If I provide screenshots, clearly distinguish:
- current desktop,
- current mobile,
- main inspiration,
- supporting inspiration.

---

# 3. Design intelligence tools and Claude skills

Before installing anything, verify that the current project does not already provide the same capability.

## A. Anthropic Frontend Design

Use Anthropic’s official `frontend-design` plugin for distinctive production-grade frontend design.

Official source:
https://github.com/anthropics/claude-code/tree/main/plugins/frontend-design

Inside Claude Code, if it is not already installed:

```text
/plugin install frontend-design@claude-plugins-official
```

If that marketplace entry is unavailable in this Claude Code version, verify Anthropic’s current official installation instructions before proceeding.

Use this skill throughout the redesign.

---

## B. UI/UX Pro Max

Use **UI/UX Pro Max** as a design-system and UX intelligence layer.

Source:
https://github.com/nextlevelbuilder/ui-ux-pro-max-skill

Preferred Claude Code installation:

```text
/plugin marketplace add nextlevelbuilder/ui-ux-pro-max-skill
/plugin install ui-ux-pro-max@ui-ux-pro-max-skill
```

Alternative CLI installation, if appropriate:

```bash
npm install -g uipro-cli
uipro init --ai claude
```

Use it for:
- typography pairing,
- color-system reasoning,
- layout patterns,
- UX checks,
- accessibility,
- product-specific visual direction.

Do not allow it to override an existing strong brand system without reason.

---

## C. Taste

Evaluate the Taste skills:

https://github.com/tyfarrago-hub/taste

It contains frontend design skills for polishing, auditing, restyling, and creating production-grade interfaces.

Install only if compatible:

```bash
git clone https://github.com/tyfarrago-hub/taste.git
cp -r taste/skills/* ~/.claude/skills/
```

For this project, prioritize skills related to:
- production-grade UI,
- premium landing pages,
- redesigning existing projects,
- high-end visual design,
- motion and interaction,
- final polish.

Do not blindly combine conflicting style systems. Pick one visual direction and stay coherent.

---

# 4. UI component and inspiration ecosystem

These are **sources of patterns and components**, not a shopping list.

Use only what improves this project.

## Core foundation

### shadcn/ui
https://ui.shadcn.com/

Use for clean source-owned React components when the stack fits.

### Radix Primitives
https://www.radix-ui.com/primitives

Use for accessible low-level interactions such as dialogs, menus, popovers, tabs, selects, tooltips, and focus management.

Do not rebuild mature accessibility behavior from scratch when a proven primitive fits.

---

## High-quality animated component sources

### Magic UI
https://magicui.design/

Useful for polished animated React/Tailwind components and effects.

### Aceternity UI
https://ui.aceternity.com/

Useful for visually strong React/Next.js interactions, hero effects, cards, parallax, text effects, shaders, and high-impact motion.

### Cult UI
https://www.cult-ui.com/

Useful for motion-rich, niche shadcn-compatible components and experimental design-engineering patterns.

### React Bits
https://reactbits.dev/

Use as a source of animated React effects and interaction ideas.

### 21st.dev
https://21st.dev/

Use as a broad component discovery library. Search for specific needs rather than browsing randomly.

Examples:
- animated hero,
- premium navbar,
- interactive feature section,
- product demo section,
- magnetic button,
- text reveal,
- image reveal,
- shader,
- interactive CTA,
- pricing transition,
- tabs,
- cards,
- responsive navigation.

Important:
- Do not combine components from five different libraries just because they look impressive.
- Adapt selected components to one design system.
- Normalize typography, spacing, radius, color, and animation timing.
- Prefer source-owned components over heavy runtime dependencies where practical.

---

# 5. Motion stack

## Motion for React

Use **Motion for React**, formerly Framer Motion, as the default React motion library when the project is React-based.

Official docs:
https://motion.dev/docs/react

Use it for:
- entrance choreography,
- staggered reveals,
- shared layout transitions,
- hover and tap feedback,
- spring motion,
- scroll-linked motion,
- element transitions,
- route-level polish.

Do not add animation to every object.

---

## GSAP

Official:
https://gsap.com/

Use GSAP only where Motion/CSS is not enough.

Good uses:
- cinematic hero sequences,
- complex scroll storytelling,
- SVG path animation,
- layered parallax,
- timeline choreography,
- sophisticated pinned sections,
- text splitting/reveal sequences,
- 3D/WebGL sequencing.

Avoid running Motion and GSAP on the same element unless there is a deliberate reason.

---

## Lenis

Official repository:
https://github.com/darkroomengineering/lenis

Use Lenis only when smooth-scroll behavior genuinely improves the experience, especially for:
- scroll-linked storytelling,
- parallax,
- WebGL synchronization.

Do not make ordinary product pages feel sluggish.
Do not break keyboard navigation, anchors, browser history, or accessibility.

---

## Native View Transitions

Consider the browser View Transition API before adding another dependency for simple page/state transitions.

Reference:
https://developer.mozilla.org/en-US/docs/Web/API/View_Transition_API

Use progressive enhancement and preserve graceful fallbacks.

---

# 6. Optional high-impact visual technologies

Use these only if they support the brand and story.

## React Three Fiber / Three.js

Use for meaningful 3D/WebGL hero scenes, interactive product visualizations, spatial interfaces, or immersive storytelling.

Do not add 3D merely because it is impressive.

3D must:
- have a clear visual purpose,
- degrade gracefully,
- not block page interaction,
- be lazy-loaded,
- avoid destroying mobile performance.

---

## Rive

https://rive.app/

Consider Rive for:
- interactive vector illustrations,
- state-driven animations,
- animated product mascots,
- highly polished lightweight UI motion.

Prefer it over video when an interactive vector animation is the better tool.

---

## Spline

https://spline.design/

Use Spline when a designed 3D scene meaningfully strengthens the hero or product story.

Do not ship a large 3D scene without testing:
- loading time,
- mobile GPU usage,
- interaction latency,
- fallback visuals.

---

# 7. Motion direction: impressive, not chaotic

Create a **motion language** before adding effects.

Define:

- entrance duration,
- exit duration,
- easing,
- spring behavior,
- stagger interval,
- hover response,
- scroll response,
- page transition style,
- reduced-motion behavior.

Use a small number of recurring motion motifs.

Examples of strong effects:

- hero elements entering in a deliberate sequence,
- headline mask/reveal,
- image crop reveal,
- subtle character or word animation,
- magnetic CTA with restraint,
- shared-element transitions,
- scroll-triggered content progression,
- sticky storytelling section,
- depth/parallax with 2 to 3 layers,
- image hover depth,
- cursor-aware highlight,
- animated data/product demonstration,
- dynamic navigation transformation,
- section transition tied to content,
- SVG line/path animation,
- cinematic but short first-load sequence.

Avoid:

- every section fading upward,
- infinite marquee everywhere,
- random floating blobs,
- constant glowing borders,
- excessive particle fields,
- gratuitous cursor followers,
- text animation that hurts readability,
- 3D tilt on every card,
- motion that delays access to content.

A user should remember the **website and product**, not merely the animation library.

---

# 8. Anti-AI-slop visual rules

Actively remove generic “vibe-coded” design patterns where they do not fit.

Audit for:

- every section being a rounded card,
- identical feature cards,
- random glassmorphism,
- purple-on-black AI styling without brand reason,
- harsh gradients,
- rainbow gradients,
- excessive drop shadows,
- glowing radial blobs,
- dot-grid backgrounds,
- sparkle icons,
- generic bento grids,
- fake terminal windows,
- generic three-column feature layouts,
- giant empty hero areas,
- random decorative pills,
- weak typographic hierarchy,
- default Lucide icons used without thought,
- overly soft corner radii,
- unnecessary hover animation,
- decorative effects copied from component galleries.

Do not remove an element merely because it appears on this list. Keep it if it genuinely fits the brand.

---

# 9. Anti-AI-slop copy rules

Review every user-visible sentence.

Mandatory:

- Remove **all em dashes** from UI copy.
- Remove generic AI-style marketing language.
- Remove fake testimonials.
- Remove invented metrics.
- Remove fake customer logos.
- Remove fake urgency.
- Remove unsupported claims.

Avoid phrases such as:

- unlock the power of
- revolutionize
- supercharge your workflow
- seamlessly
- next-generation
- cutting-edge
- game-changing
- transform the way you
- future-proof
- all-in-one solution

Avoid repeated:
- “It’s not X, it’s Y”
- over-clever aphorisms,
- three-adjective marketing sentences,
- vague claims that could describe any SaaS.

Prefer:
- specific headlines,
- concrete benefits,
- clear verbs,
- short supporting text,
- proof near claims,
- one idea per section,
- direct CTA wording.

---

# 10. Typography and visual system

Do not treat typography as an afterthought.

Create a clear system for:

- display headline,
- page heading,
- section heading,
- body,
- small text,
- labels,
- buttons,
- numbers/data.

Check:
- font personality,
- weight hierarchy,
- line height,
- line length,
- letter spacing,
- optical balance,
- desktop and mobile scaling.

Avoid using Inter automatically unless it is genuinely right for the brand.

Build a small tokenized system for:

- spacing,
- radius,
- border,
- shadows,
- surface colors,
- text colors,
- typography,
- motion.

The final website should feel intentionally art-directed.

---

# 11. Mobile is a separate composition pass

After desktop improvements, perform a dedicated mobile pass.

Test approximately:

- 375px
- 390px
- 430px
- 768px
- 1024px
- 1440px

Fix:

- horizontal overflow,
- awkward heading wrapping,
- compressed navigation,
- tiny tap targets,
- desktop-only animation assumptions,
- huge fixed heights,
- broken sticky sections,
- poor image crops,
- excessive animation on low-power devices,
- WebGL that should be simplified or disabled,
- text hidden behind visual effects.

Mobile should feel intentionally designed, not scaled down.

---

# 12. Performance rules

The “wow” effect is worthless if the page stutters.

Measure and protect:

- LCP,
- CLS,
- INP,
- bundle size,
- image weight,
- animation frame rate,
- mobile performance.

Requirements:

- animate `transform` and `opacity` where possible,
- avoid layout-thrashing animation,
- lazy-load heavy animation/3D sections,
- optimize images,
- preload only critical assets,
- remove unused dependencies,
- avoid shipping entire animation libraries for one trivial effect,
- test production mode,
- test throttled mobile performance.

If an effect causes obvious jank, simplify or remove it.

---

# 13. Accessibility and reduced motion

Every impressive interaction must remain usable.

Required:

- keyboard navigation,
- visible focus states,
- proper semantic elements,
- accessible labels,
- correct contrast,
- touch-friendly targets,
- no hover-only critical functionality,
- no content permanently hidden behind animation.

Respect:

```css
@media (prefers-reduced-motion: reduce)
```

When reduced motion is enabled:
- remove large parallax,
- remove unnecessary transforms,
- shorten or eliminate cinematic transitions,
- keep content immediately available.

---

# 14. Implementation workflow

Follow this sequence.

## Phase 1: Audit

Do not modify files yet.

Return:
1. current design strengths,
2. 5 to 8 biggest weaknesses,
3. visual direction,
4. proposed motion language,
5. tool/library choices,
6. dependencies you want to add and why,
7. implementation order.

Wait for my approval.

---

## Phase 2: Establish the design system

After approval:

- define typography,
- color roles,
- spacing,
- radius,
- borders,
- surfaces,
- component hierarchy,
- motion tokens.

Preserve existing brand colors unless there is a strong reason to adjust them.

---

## Phase 3: Highest-impact sections

Improve in this order unless the audit suggests otherwise:

1. navigation,
2. hero,
3. primary product demonstration,
4. key feature storytelling,
5. CTA,
6. supporting sections,
7. footer.

Do not spend hours polishing a footer while the hero remains weak.

---

## Phase 4: Motion and micro-interactions

Add motion only after the static composition is strong.

Use:
- CSS first for trivial transitions,
- Motion for normal React interaction and layout animation,
- GSAP for complex sequences,
- Lenis only when smooth-scroll coordination is needed,
- WebGL/Rive/Spline only when justified.

---

## Phase 5: Mobile and accessibility

Perform the dedicated mobile and accessibility pass.

---

## Phase 6: Final polish

Review:

- typography,
- alignment,
- spacing,
- color balance,
- CTA hierarchy,
- icon quality,
- image crops,
- transitions,
- motion consistency,
- loading states,
- empty/error states,
- responsive behavior.

Remove anything decorative that does not improve the experience.

---

# 15. Final engineering validation

Run the project’s actual commands.

Typical React/Next.js example:

```bash
npm run lint
npm run build
```

Also perform, where available:

- type checking,
- unit tests,
- component tests,
- Lighthouse,
- accessibility audit,
- browser console review,
- broken-link review.

Do not claim completion while the production build fails.

---

# 16. Final deliverable

At the end, report:

## Visual direction
Describe the final design language in a few sentences.

## Major improvements
List the most important changes.

## Motion system
Explain the major animation patterns and why they exist.

## Tools used
List which of these were actually used:
- CollectUI
- Anthropic Frontend Design
- UI/UX Pro Max
- Taste
- shadcn/ui
- Radix
- Magic UI
- Aceternity UI
- Cult UI
- React Bits
- 21st.dev
- Motion
- GSAP
- Lenis
- View Transitions
- React Three Fiber
- Rive
- Spline

Do not claim a tool was used if it was not.

## Dependencies added
Explain why each new dependency was necessary.

## Files changed
List the important files.

## Validation
Report lint/build/test/accessibility/performance results.

## Remaining manual work
List anything that still needs human review.

---

# Core principle

Do not make the website “more designed” by adding more effects.

Make it **more intentional**.

The ideal result combines:

**strong art direction + excellent typography + clear hierarchy + restrained but memorable motion + real product storytelling + fast implementation + responsive behavior + accessibility.**

I want moments that make people say “that is beautiful,” but those moments must support the product rather than compete with it.
