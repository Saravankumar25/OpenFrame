# The new set on today's assistant, before any runtime change

Run on 2026-11-03: the frozen R0 binary (`.build-cache/ai-eval-r0.exe`) on the fixed sample of `sections.lowtide.en.jsonl`
(`--only sec-s-`, 302 cases). That build ignores `sections`, so this measures today's assistant on the new questions. Cases
using the new fixture `low-tide-sections` fail to build their project on it, because the old binary does not know that fixture.
Other builds compiled on the same computer during the run. The answers stay in `.build-cache/ai-r0-sections/`.
