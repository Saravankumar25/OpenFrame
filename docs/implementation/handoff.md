# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-21.

## State

1. **Merged to `main` locally (not pushed):** Story Board renamed Beat Board everywhere visible; a Story tab (plain
   writing page with TXT, PDF and Word export); the five items of `new changes.docx` (Daily View export, Outline
   scroll, Compare bar, Offline AI layout, Find scrolling); AI answers as plain text with See sources. Record: top
   entry of the [progress log](progress-log.md).
2. **Checks:** web 640, Rust suites green, governance 90 of 90, every E2E spec passes on the final code, real-model AI
   evaluation 98.1 percent tool and arguments, 91.2 percent end state, safety 46 of 46.
3. **Installers:** `release-out/OpenFrame-3.exe` is installed but predates this work; the next build is
   `OpenFrame-4.exe` (only on the owner's word).
4. **GitHub:** `main` has `windows/` and `mac/`. `windows/` matches local 6e28b33, not this work. Sync only on the
   owner's word, by the subtree method in memory; never `git pull` into `C:\OpenFrame`.

## Next items, in order

1. WindowsHTML: at least 400 screens in Light and Dark. Prompt: [next-session-windows-html-prompt.md](next-session-windows-html-prompt.md).
2. Build and install `OpenFrame-4.exe` when the owner asks; owner hand test.
3. Writing grounding (67 percent) and the user manual refresh.

## Open blockers

- None.

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`. Do not commit
  `changes/`, `OpenFrameAI.docx` or `OpenFrameMac/`.
