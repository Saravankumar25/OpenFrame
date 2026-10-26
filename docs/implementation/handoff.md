# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-10-26 (conversational assistant:
> diagnosis, ADR-0037, spec, first spike; documents only, no code changed).

## State

1. **`main` holds the whole 2026-10-24 upgrade** (merge 94b1942b, local, not pushed) and `release-out/OpenFrame-4.exe`
   (installed on this PC, 40 end-to-end specs passed). Details in the 2026-10-24 entry of `progress-log.md`.
2. **The AI was found to be unusable in real use** (owner export of 40 exchanges on a project with two screenplays).
   Causes and numbers: `progress-log.md` entry of 2026-10-26 and section 2 of the spec. Short version: several
   screenplays means no current draft, so the understanding step is skipped and every question falls to the weakest
   path; no conversation state; the benches never contained a multi-screenplay project; the tool bench's 98.8% mostly
   measures regex routes.
3. **Decided by the owner (2026-10-26, chat):** the assistant becomes a conversational chat assistant with focus, memory
   and per-turn fetched context; deterministic answers are cut to the few that need exactness or safety; model stays
   Gemma 3 1B; Laya is not used; build starts in a new session. Records: [ADR-0037](../adr/ADR-0037-conversational-assistant-focus-and-dossiers.md),
   [ai-conversational-harness.md](../specs/ai-conversational-harness.md), `owner-decisions.md`.
4. **Spike S1 done** (`docs/release/evidence/ai-conversational-spike-2026-10-26/`): conditional go. 7 of 15 answers good,
   2 passed the script but were wrong, 6 failed; answer type read 20 of 28. Short facts from an isolated record work;
   paragraphs embellish; brainstorming echoes the request. The findings are already in the spec.
5. Checks run today: `node scripts/check-links.mjs` OK, `npm run check:writing` OK. Not run: any code test, any eval.

## Next items, in order (each its own work item; measure before the next)

1. **S0.** Fixture `low-tide-two-screenplays` (and `thin-project`, `big-project`), the new `Expect` flags (spec 7.4), the
   200 conversations (spec 7.3, including the 40 exchanges of the export as behaviour cases), then a **baseline run of the
   current build**. Eval command: `node scripts/cargo-queued.mjs run --release -p openframe-ai-eval --bin ai-eval -- --gold
   crates/openframe-ai-eval/gold --out <folder>`. It needs the engine in `.build-cache/ai-bundle` (present) and the pinned
   models in `.ai-assets` (not checked; the installed models are in `%LOCALAPPDATA%\OpenFrame\models\m-6fec34abd495\`).
   The conversation bench took 44 minutes for 60 cases on 2026-10-25.
2. **S1b.** Second spike on the revised design (spec 5.3 to 5.6): facts-first dossiers, required facts, answer type plus
   domain, chat and brainstorm policies; at least 60 cases, three runs each. The spike script is in the evidence folder;
   start the engine with `.build-cache/ai-bundle/runtimes/llama-cpp-b11259-win-x64-cpu/llama-server.exe -m <gguf> -c 8192
   --port 8099 --host 127.0.0.1`. Do not start S3 before S1b meets the bars.
3. **S2.** Close the coverage inventory (spec 6): read tools for themes, app settings, budget lines, product help; review
   hidden queries; confirm what "C3" means and where Parking Lot and Quick Capture live.

## Open blockers

- Owner to approve: the bars in spec 7.5; token streaming (ADR-0037 open question 5); the defaults for long-term memory
  (visible and deletable in Settings) and one extra fetch round.
- The gold sets of 2026-10-24 still need the owner's approval before their numbers count as official.

## Do not

- Do not push without the owner's word. Do not `git pull` origin/main here. Do not edit `OpenFrameMac/`. Do not build an
  installer.
- Do not add a deterministic answer without an entry in spec section 4.1.
- Do not claim a result that was not measured. State what was run, on what data, how many times.
- Do not start S3 (the build of the index and focus) before S0 and S1b are saved.
- Tool gates in this workspace ask for a short statement of facts before the first write or edit of a file; give it, then retry.
