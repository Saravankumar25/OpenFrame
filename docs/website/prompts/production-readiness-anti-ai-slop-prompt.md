> **SUPPLIED PROMPT, kept for reference (2026-10-05).** Method only; it is not a specification. Tool and repository names
> in it are **unverified**: install nothing from a name alone, check source, licence and permissions first, and do not send source code to
> third-party model providers. Plan and rules that apply to this repository: [../README.md](../README.md).

# Claude Code Production-Readiness + Anti-AI-Slop Prompt

Use this prompt at the root of the project. First determine whether the repository is a **website/web app**, a **desktop application**, or a hybrid. Do not blindly apply web-only changes to desktop apps or desktop-only changes to websites.

## 1. First: inspect before changing anything

Read the repository structure, README, package files, build config, environment setup, deployment config, authentication, database layer, API layer, tests, CI/CD, logging, error handling, and current UI system.

Before editing:

- Identify the framework, runtime, package manager, deployment target, database, auth system, and operating-system targets.
- Identify which checks below are already implemented, partially implemented, missing, or irrelevant.
- Preserve existing architecture and visual identity unless a change is required.
- Do not replace working systems just because a plugin suggests a different approach.
- Do not expose secrets, credentials, API keys, tokens, private URLs, or production data.
- Never install a package or plugin only because its name appears in this prompt. Verify its official repository/package, maintenance status, license, compatibility, and install instructions first.
- If a named tool cannot be confidently verified, **do not install a similarly named random package**. Report it as unverified and continue.

---

# 2. Tools, plugins, skills, and reference repositories

Evaluate these resources and use/install only the ones that are legitimate, compatible, useful for this project, and do not conflict with the existing stack.

## Claude Code / coding workflow tools mentioned

### Ponytail
Claimed purpose: reduce unnecessary Claude Code output/token usage.

Instruction:
- Find the official source and documentation.
- Verify that it is a genuine Claude Code-compatible tool.
- Check permissions and what project/context data it can access.
- Install only if the source is trustworthy and the integration is reversible.
- If no authoritative source is found, skip it and report that clearly.

### OmniRoute
Claimed purpose: route requests across multiple model providers and switch when limits are reached.

Instruction:
- Verify the official project first.
- Do not add unknown/free model providers by default.
- Do not send proprietary code to third-party providers without explicit approval.
- Document which providers receive source code, prompts, metadata, or telemetry.
- Never place provider API keys in source control.

### Graphify
Claimed purpose: create a codebase knowledge graph/context layer.

Instruction:
- Verify the exact project before installation because multiple tools may use this name.
- Prefer local indexing when available.
- Exclude secrets, build artifacts, dependencies, generated files, and sensitive data from indexing.
- Do not introduce a graph database or external service unless it provides a clear benefit over the project's current context/indexing approach.

### Agent Skills
Claimed purpose: reusable skills for planning, coding, testing, reviewing, and publishing.

Instruction:
- Verify the exact official skill repository/package.
- Review every skill before enabling it.
- Give skills only the minimum permissions required.
- Prefer skills that improve planning, testing, security review, accessibility, documentation, and release validation.
- Do not enable autonomous publishing/deployment without human approval.

### GSC MCP
Purpose: connect Claude/MCP tooling to Google Search Console data.

Use only for public websites where SEO is relevant.

Instruction:
- Verify the MCP server and authentication flow.
- Use least-privilege Google access.
- Never commit OAuth credentials or tokens.
- Use it for indexing issues, search queries, CTR/impressions, page performance, and SEO diagnostics.
- Do not install for a local-only desktop application.

### Open SEO
Claimed purpose: SEO auditing, keyword research, backlinks, ranking analysis.

Instruction:
- Verify the official project first.
- Prefer read-only/audit capabilities initially.
- Do not generate spam backlinks or manipulate rankings.
- Treat SEO suggestions as recommendations, not automatic truth.
- Skip entirely for desktop-only apps.

### GEO Optimizer / Generative Engine Optimization skill
Claimed purpose: improve discoverability/citation in AI search and answer engines.

Instruction:
- Verify the exact source and documentation.
- Use only after conventional technical SEO, structured content, factual clarity, schema markup, and crawlability are correct.
- Do not add fake authority signals, fabricated statistics, or deceptive AI-targeted copy.

---

# 3. UI, writing, and design resources

Review these repositories. Do not automatically install all of them; use them as references or install them only when their actual integration is useful.

- No AI Slop  
  https://github.com/petergyang/no-ai-slop

- Make Interfaces Feel Better  
  https://github.com/jakubkrehel/make-interfaces-feel-better

- Better Icons  
  https://github.com/better-auth/better-icons

- Humanizer  
  https://github.com/blader/humanizer

- SuperDesign  
  https://github.com/superdesigndev/superdesign

- Anti-AI-Slop Writing  
  https://github.com/jalaalrd/anti-ai-slop-writing

For each repository:
1. Read the README and installation instructions.
2. Check recent maintenance/activity.
3. Check its license.
4. Check whether it is a package, Claude skill, prompt collection, design reference, or executable tool.
5. Avoid adding overlapping tools that solve the same problem.
6. Prefer one strong solution over five redundant dependencies.
7. Report what you installed, what you only used as a reference, and what you skipped.

---

# 4. Mandatory anti-AI-slop writing pass

Audit **all user-visible text** in the website or desktop application.

Remove or rewrite text that sounds generated, generic, inflated, repetitive, fake, or corporate without meaning.

Mandatory rules:

- Remove **all em dashes** from user-facing UI copy. Replace them with a period, comma, colon, semicolon, parentheses, or rewrite the sentence naturally.
- Do not use fake testimonials, fabricated reviews, invented customer counts, fabricated performance numbers, fake security claims, or fake social proof.
- Remove cliché AI copy such as:
  - "revolutionize your workflow"
  - "unlock the power of"
  - "supercharge your productivity"
  - "seamlessly"
  - "next-generation"
  - "game-changing"
  - "cutting-edge"
  - "future-proof"
  - "all-in-one solution"
  - "transform the way you..."
  unless the wording is genuinely necessary and supported.
- Avoid repetitive "It's not X, it's Y" copy.
- Avoid generic hero text that could describe any product.
- Avoid fake urgency.
- Avoid unnecessary exclamation marks.
- Avoid excessive adjectives.
- Avoid robotic three-item sentence patterns when a simpler sentence is better.
- Avoid aphorism-style marketing filler.
- Prefer specific benefits and concrete actions over vague claims.
- Put proof close to a claim when proof exists.
- Make CTA text describe the action or outcome.
- Write for one clear target user instead of everyone.
- Break up large text blocks.
- Keep one main idea per section.
- Use scannable headings and concise supporting copy.
- Keep terminology consistent across the entire product.

After rewriting, perform a second pass only for:
1. em dashes,
2. fake claims,
3. generic AI wording,
4. inconsistent terminology,
5. awkward or overlong UI sentences.

---

# 5. Visual anti-vibe-code audit

Do not blindly remove a visual style just because it is popular. Instead, detect whether the interface looks generic, templated, inconsistent, or disconnected from the product.

Audit for:

- harsh or unnecessary gradients,
- random rainbow colors,
- excessive purple/black AI styling,
- glowing radial orbs,
- unnecessary dot-grid backgrounds,
- overused glassmorphism,
- excessive shadows,
- excessive rounded cards,
- random bento grids,
- generic three-feature-card sections,
- decorative terminal windows with no product value,
- sparkle icons everywhere,
- excessive hover animations,
- animated arrows used only for decoration,
- default icon-library appearance with no visual system,
- inconsistent border radii,
- inconsistent spacing,
- inconsistent typography,
- weak hierarchy,
- poor empty/loading/error states,
- landing pages with screenshots/mockups but no real product demonstration.

The goal is not "remove all modern design." The goal is to make the interface feel intentional, product-specific, coherent, restrained, and real.

---

# 6. Shared production-readiness checks for BOTH web and desktop apps

Implement or validate what is relevant:

## Security
- No secrets in frontend bundles, repositories, logs, crash reports, screenshots, or packaged desktop resources.
- Validate environment-variable handling.
- Search Git history for accidentally committed keys.
- Authentication and authorization enforced server-side where applicable.
- Least-privilege database and service credentials.
- Input validation and sanitization.
- Injection protection.
- File upload type, size, and content validation.
- Rate limiting where abuse is possible.
- Request-size limits.
- Sensitive operations protected from replay/duplication.
- Password-reset links/tokens expire and are one-use.
- Sessions invalidated when appropriate.
- Avoid user enumeration.
- Security-sensitive events logged without leaking secrets.
- Production debug modes disabled.
- Dependency vulnerability review.
- Spending/usage caps for paid APIs and AI providers.
- AI prompt-injection boundaries for any agent/tool system.
- Human approval before destructive or high-impact agent write operations.

## Reliability
- Clear loading, empty, success, and error states.
- Timeouts and retries with sensible limits.
- No infinite retry loops.
- Idempotency where duplicate requests/actions matter.
- Graceful handling of network failure.
- Useful error logging.
- Health checks where appropriate.
- Backup strategy.
- Actually test restore/recovery, not only backup creation.
- Concurrency and simultaneous-user testing where relevant.
- Avoid N+1 queries.
- Add indexes based on real query patterns.
- Paginate large result sets.
- Cache expensive/repeated operations only when cache invalidation is understood.
- Database connection pooling where relevant.

## Performance
- Remove unused dependencies.
- Split large bundles/modules where useful.
- Lazy-load non-critical code/assets.
- Compress images and other large assets.
- Avoid unnecessary rerenders.
- Debounce expensive UI handlers.
- Compress large API responses where useful.
- Measure before and after performance changes.

## Accessibility
- Keyboard navigation.
- Visible focus states.
- Correct button/link semantics.
- Accessible names/labels.
- Color contrast.
- Alt text where images communicate information.
- Skip-to-content links for websites where appropriate.
- No critical interaction that requires only a mouse or hover.
- Respect reduced-motion settings.

## UX
- Clear validation messages.
- Confirmation for destructive actions.
- Password visibility control where passwords exist.
- Meaningful progress/loading indication.
- Proper 404/not-found equivalent.
- Clear contact/support path.
- Avoid dead buttons and placeholder actions.
- Real product states instead of fake/demo data leaking into production.

---

# 7. Website / web-app only checks

Apply these only when this repository serves a public website or web application.

## Legal and privacy
- Privacy Policy.
- Terms and Conditions.
- Cookie policy/consent when required by the actual cookies/trackers used.
- Refund policy when the product takes payments and refunds are applicable.
- Explain what user data is collected and why.
- Review third-party embeds and trackers.
- Avoid fake reviews and misleading claims.
- Review image/font/content licensing.
- Account for applicable local laws rather than copying a generic legal template.

Do not fabricate legal compliance. Mark anything requiring legal review.

## HTTPS and web security
- HTTPS enforced.
- HSTS where appropriate.
- Secure, HttpOnly, SameSite cookies where applicable.
- CSRF protection where the authentication architecture needs it.
- Correct CORS allowlist.
- Security headers.
- Directory listing disabled.
- Admin/internal routes protected.
- Payment webhook signatures verified.
- Prices and entitlement logic enforced server-side.

## SEO and discoverability
- Unique page title.
- Useful meta description.
- Correct canonical URL.
- No accidental `noindex`.
- Valid `robots.txt`.
- Valid `sitemap.xml`.
- Open Graph/social preview image.
- One clear primary H1 per page when appropriate.
- Logical heading hierarchy.
- Descriptive URLs/slugs.
- Internal linking.
- Broken-link scan.
- Structured data/schema only when factually correct.
- `lang` attribute.
- Favicon/app icons.
- Search Console setup.
- Bing Webmaster Tools when useful.
- Analytics only if desired and privacy requirements are handled.
- Core Web Vitals.
- Mobile responsiveness.
- Image compression.
- Review source-map exposure policy for production.
- Review JavaScript bundle size.
- `llms.txt` may be considered, but do not treat it as a substitute for real SEO or as a guaranteed ranking mechanism.
- Backlink strategy must be legitimate. No spam/link schemes.

## Post-launch verification
- Google Search Console.
- Submit sitemap.
- Bing Webmaster Tools if relevant.
- Test robots/indexability.
- Test analytics events.
- Test on real mobile devices.
- Run Lighthouse/PageSpeed.
- Validate social previews.
- Verify real production-domain metadata, not preview/staging URLs.

---

# 8. Desktop-application only checks

Apply these when the product is a native or packaged desktop application.

## Installation and packaging
- Produce a clean installer for supported operating systems.
- Verify fresh install on a clean machine.
- Verify uninstall.
- Verify reinstall.
- Verify upgrade from the previous version without losing user data.
- Store app data separately from application binaries.
- Never overwrite user-created data during upgrades.
- Define versioning clearly.
- Add rollback/recovery strategy for failed migrations or updates.
- Verify file paths containing spaces and non-ASCII characters.

## Updates
- Decide whether updates are manual, prompted, or automatic.
- Verify update signatures/integrity.
- Do not require an app-store packaging format unless the project actually needs it.
- Test migration from v1 to v2 with real persisted data.
- Preserve user settings across upgrades.

## Local data
- Define where local database/files are stored.
- Handle schema migrations safely.
- Back up important local data before risky migrations.
- Provide export/backup where the product needs it.
- Handle database corruption or partially written state gracefully.
- Use OS-appropriate secure storage for secrets/tokens.

## OS integration
- Correct app icon and metadata.
- Proper window behavior.
- Minimize/maximize/restore behavior.
- DPI/scaling support.
- Keyboard shortcuts.
- File dialogs.
- Notifications only when useful.
- Deep links/file associations only when required.
- Respect OS theme if the design supports it.
- No unnecessary administrator privileges.

## Security
- Code signing where the distribution model requires or benefits from it.
- Do not bundle production secrets into the executable.
- Protect local auth/session tokens.
- Validate data received from local files, clipboard, URLs, plugins, and IPC.
- Secure IPC between processes if the architecture uses multiple processes.
- Restrict dangerous shell/process execution.
- Verify webview/Electron/Tauri permissions if applicable.
- Disable developer tools/debug endpoints in production when appropriate.

## Stability
- Crash handling.
- Local logs with rotation.
- Logs must not contain secrets or private user content unnecessarily.
- Graceful offline behavior where possible.
- Test sleep/wake.
- Test app restart during a pending operation.
- Test forced termination and recovery.
- Test low-disk-space behavior if the app stores large files/models.
- Test antivirus/Defender behavior for the installer.

---

# 9. UI details worth implementing only when they fit the product

Do not add these merely to satisfy a checklist. Add them only if they improve the actual product:

- hover/pressed/disabled button states,
- clean not-found page/state,
- mobile navigation,
- dark-mode toggle,
- full-site search,
- expandable FAQ,
- UTM tracking,
- loading skeletons,
- copy-to-clipboard,
- confirmation modals,
- sticky header,
- floating contact/help action,
- newsletter signup,
- scroll progress indicator,
- print stylesheet,
- updated dates on editorial content.

Avoid feature clutter.

---

# 10. Performance review checklist

Audit for:

- API response caching where safe,
- server-side caching where safe,
- CDN for public static assets when useful,
- database indexes,
- expensive query caching where justified,
- N+1 database queries,
- pagination,
- image compression,
- lazy loading,
- bundle/code splitting,
- deferring non-critical scripts,
- unused dependencies,
- payload compression,
- connection pooling,
- unnecessary rerenders,
- unbounded API responses,
- slow startup paths in desktop applications.

Do not add a load balancer simply because it appears on a checklist. Add infrastructure only when traffic/reliability requirements justify it.

---

# 11. Final validation

After implementation:

1. Run existing tests.
2. Add targeted tests for changed production-critical behavior.
3. Run lint/type checks.
4. Build the production artifact.
5. Test the real production build, not only development mode.
6. Check browser console or desktop runtime logs for errors.
7. Run security/dependency scans that fit the stack.
8. Verify no secrets entered the repository.
9. Verify accessibility basics.
10. Verify performance regressions.
11. Verify all user-visible em dashes are removed.
12. Verify no fake testimonials, metrics, users, reviews, logos, or claims were introduced.
13. Verify the UI still matches the product's existing brand.
14. Verify every new dependency is actually needed.

---

# 12. Output format to me

Do not dump random changes.

Return a concise report with these sections:

### A. Project classification
Website, web app, desktop app, or hybrid.

### B. Installed tools
For each tool:
- source,
- version,
- why it was installed,
- permissions/data access,
- exact files/configuration changed.

### C. Tools reviewed but not installed
Explain why.

### D. Production gaps fixed
Group by:
- security,
- reliability,
- performance,
- accessibility,
- UX,
- SEO/legal for web,
- installer/update/local-data concerns for desktop.

### E. Remaining manual work
Anything requiring:
- legal/business decisions,
- account ownership,
- domain/DNS changes,
- OAuth approval,
- store/distribution certificates,
- code-signing certificates,
- production secrets,
- payment/analytics/Search Console access,
- human design/product decisions.

### F. Validation results
List the tests, build checks, audits, and production verification performed.

Do not claim "production ready" unless the important checks genuinely passed.
