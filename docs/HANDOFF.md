# Website handoff (2026-10-21, session cut by usage limit)

Done:
- Cloudflare: token and account ID are User env vars (read them with PowerShell
  [Environment]::GetEnvironmentVariable(...,"User")). Pages project `openframe-studio` created (classic Pages, created
  with --force once; never pass --force again). URL after first deploy: https://openframe-studio.pages.dev
- Research in research/: capability-matrix.md, facts.json, design-principles.md, MAPPING-BRIEF.md, probe.mjs,
  tutorials-mac-a.json (done). tutorials-win-a.json, tutorials-win-b.json, tutorials-mac-b.json were still being
  written by agents; check they exist and parse.
- Sources copied (git-ignored): research/WindowsHTML-src, research/MacHTML-src (from origin/main mac/MacHTML),
  research/package-src (Website_Share_Package).
- Skeleton: site.config.mjs, src/lib/html.mjs, src/lib/icons.mjs, src/content/nav.mjs, docs/CONTRACT.md (the plan:
  file owners, page module API, truth rules). Dev deps installed: playwright, esbuild, sharp, axe, Inter font.

Next, in order (fan out agents per docs/CONTRACT.md owners):
1. Lead: src/lib/layout.mjs (head, header with macOS|Windows toggle, footer, shot/osBlocks/kbd/cta), site.css, site.js,
   scripts/build.mjs (dist, hashed assets, sitemap, robots, _headers CSP, _redirects, llms.txt, 404).
2. Parallel: player agent, marketing agent, info agent, tutorial writer agents (win/ and mac/), test agent.
3. Build, test, deploy: npx wrangler@4.151.3 pages deploy dist --project-name openframe-studio --branch main.
4. Push to GitHub main as website/ (owner asked) by the subtree method: temp index from origin/main, read-tree
   --prefix=website/ this repo's HEAD tree, commit-tree with parent origin/main, push.
