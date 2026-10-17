# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-17 afternoon, after W6 rounds 2 and 3.

## State

1. **`main`** holds W1 to W8 (W6 accepted on the bar by the owner, merged locally 2026-10-17). Nothing pushed.
2. **Release proof (W4):** the whole suite on the gated release executable, 39 spec files, 227 checks, 0 failures
   ([evidence](../release/evidence/w4/README.md)); 64 feature rows at proof level 3. Rust workspace (60 test binaries)
   and vitest (596) green on the same code.
3. **AI (W5, W6, W7, W8):** downloads from Hugging Face at pinned commits with import (ADR-0032); three pinned models,
   E5 the default search model (232 of 252 retrieval cases against BGE's 202); card probe version 2; 12 threads on large
   hybrid chips. W6 English bar met: correct tool 96.7 percent (bar 90), end state 88.8 percent (bar 85); answers in the
   request's language 25 to 69 percent. Evidence: [w5](../release/evidence/w5/README.md),
   [w6](../release/evidence/w6/README.md).
4. **Owner decisions 2026-10-16** are all in `owner-decisions.md` (model set, hardware scope, licensing owner-handled,
   worktree clean-up done, GSD and `AGENTS.md` removed, claude-mem kept, gold sets approved).

## Next three items

1. On the owner's word only: W9 (installer build and clean-machine proof).
2. Remaining W6 items if the owner asks: memory v1 and its screen, the poisoned-retrieval corpus, recipes for bigger jobs,
   French, Japanese, Korean and Chinese retrieval (4 or 5 of 8).
3. The nine English end-state misses listed in the [w6 evidence](../release/evidence/w6/README.md).

## Do not

- Do not raise licensing or ask about the model set or borrowed laptops again (decided).
- Do not build an installer and do not push without the owner's word in the conversation.
- Do not run measurements while the release suite runs (it times typing and idle CPU); keep the laptop on mains power
  for both (`Get-CimInstance -Namespace root\wmi -ClassName BatteryStatus`).
- Compare `ai-eval` timings only between runs with the same engine place (the report's "Engine place" line).
