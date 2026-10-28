# Spike S1b, 2026-10-26

The revised conversational turn (harness spec sections 5.3 to 5.6) on Gemma 3 1B, processor build of llama.cpp b11259.
Findings and verdict: section 13 of `docs/specs/ai-conversational-harness.md`.

| File | What it is |
|---|---|
| `spike2-v1.mjs`, `spike2-results-v1.json`, `spike2-run-v1.log` | Round 1: 70 cases, three runs, first checker |
| `spike2-v2.mjs`, `spike2-results-v2.json`, `spike2-run-v2.log` | Round 2: normalised checker, two policy lines, v2 type rules, 30 held-out messages |
| `spike2.mjs`, `spike2-results-v3-types.json`, `spike2-run-v3-types.log` | Round 3: answer type only (`--types-only`), chat decided by social words, the model reads the rest |

Run: start `llama-server.exe -m <gemma gguf> -c 8192 --port 8099 --host 127.0.0.1`, then `node spike2.mjs [--runs 3]
[--only <id part>] [--types-only]`. Data is fictional; checks are code only.
