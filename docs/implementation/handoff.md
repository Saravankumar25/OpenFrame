# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-15 (Windows phase W1, first session).

## State in six lines

1. **Phase W1 is under way** on branch `windows/w1-controls` (local commits only, not merged, not pushed). Three slices
   are done; four remain.
2. **Done:** the Proof column and its checker (65 Verified rows at proof level 2); the team password minimum is 8 in
   `build_hash.rs` with a test written first; the gates `check-secrets.mjs`, the app-address allow-list in
   `check-links.mjs` (`scripts/app-addresses.json`) and `scan-installer.mjs`, wired into `check:governance` and CI.
3. **Corrected in the plan:** `reqwest` is already macOS-only in the shell, so W1 has no code change for it (removed in
   M1). The 39 worktree folders under `.claude/worktrees` (4.2 GB) are orphans: their git folder `C:/dev/OpenFrame/.git`
   is gone, and so are the 31 branches.
4. **Owner answers given 2026-10-15:** start W1; the long description becomes the README sentence ("a local-first desktop
   studio for writers, screenwriters and production teams, covering ..."); the AI instructions keep "filmmakers" until the
   W5 measurement exists (ADR-0028); the secret scan is our own Node script, no download.
5. `scan-installer.mjs` finds 2,271 personal paths in today's debug executable. That is the W3 release-profile item
   ("no personal paths in panic text"); the W9 gate will refuse the build until W3 fixes it.
6. No plan box has been ticked yet: the reviewers (`ecc:rust-reviewer`, `ecc:typescript-reviewer`,
   `ecc:silent-failure-hunter`, security review) run before ticking, once W1 is complete.

## Next items (rest of W1)

1. **Archive moves:** `docs/spec-digest/`, `docs/engineering/AGENT_BRIEF.md`, `SECURITY_REVIEW_2026-09-30.md`, documents 28,
   29, 31, with the exempt list in `scripts/check-writing-standard.mjs`, the two Rust test comments, `DEVELOPMENT.md` and
   ADR links in the same commit.
2. **Conflict sweep and wording:** grep `masterplan.md`, the launch register and the specifications for statements that
   conflict with ADR-0026 to ADR-0030, mark each, add a reconciliation-log row. Replace "filmmakers" in `tauri.conf.json`
   (long and short description), CONTRIBUTING, `AppError` text, privacy policy and code comments; not in AI prompts.
3. **Storage hygiene:** `debug = "line-tables-only"` in `[profile.dev]`; merge the 46 test files of
   `openframe-application/tests` into a few binaries (test count before and after must match); write the worktree review
   table (size, newest date, files that differ from `main`) and stop for the owner's approval. Then the reviewers, tick
   the boxes with evidence, and the W1 exit check.

## Blocked on the owner

- Approval of the worktree review table (after it is written). The macOS version of the Intel MacBook (M1).
- The meaning of "build the website manually" (Chapter 2). Later: model host and manifest key (W8), laptops and virtual
  machine (W7, W9), Apple silicon Mac (M8, M9), the pilot team, and the word to build.

## Do not

- Do not push, merge to `main`, build an installer or a release executable, or delete worktrees without the owner's word.
- Do not change the AI prompts or model before the W5 baseline exists (ADR-0028).
- Do not write shell heredocs that contain backslashes: Git Bash here collapses `\\`, which broke a regular expression
  and a test string this session. Use the Edit and Write tools for code.
- Do not edit `docs/archive/`, `docs/audits/` or `docs/release/evidence/`. The team password is never written to a file.

## Facts a new session cannot see from the code

- The owner is a beginner in Rust, Tauri and Claude Code: explain simply. Windows 11 Home (no Sandbox or Hyper-V).
- Hardware: 15.4 GB memory, 18 logical cores, Intel Core Ultra 5 with integrated graphics; an Intel MacBook; an Apple
  silicon Mac and borrowed laptops are coming.
- Benchmark of 2026-09-30: Gemma 3 1B Q8_0 gave 38.9 percent correct tool and arguments. The release bar is 90 and 85
  percent after the harness work. Target prices: 3,000 rupees, 89 dollars, 89 euros (not final).
