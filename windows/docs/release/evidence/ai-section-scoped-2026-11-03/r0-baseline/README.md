# R0 baseline of the section-scoped assistant work

Run on 2026-11-03 before any assistant code changed: `ai-eval --gold crates/straft-ai-eval/gold --only lt-`, release
build of commit 136ad3d0 (assistant code as of 283e9e32), Gemma 3 1B on the processor, deterministic sampling. 274 cases, 82
minutes. Other builds compiled on the same computer during part of the run, so the times are a little high.

Headline numbers: correct tool and arguments 73.9% (bar 90%), correct end state 85.7% (bar 85%), answer facts 69.1%,
invented names 34.0%, median 5.3 s and 95th percentile 28.1 s over all answers, paragraph answers median 16.1 s and 95th
percentile 40.0 s.

The results files with the answers stay in `.build-cache/ai-r0-baseline/` on the build computer.
