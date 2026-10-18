# Mac phase M8 evidence: the AI engine and the English bar (2026-10-18)

Proof level 2 (development computer, ADR-0030). Decision record: [ADR-0033](../../../adr/ADR-0033-mac-ai-engine-helper-process-metal.md).

## Machine and build

| Item | Value |
|---|---|
| Machine | MacBook with Apple M1, 8 cores, 16 GB memory, macOS 26.5.2 |
| Commit | d69efa0 plus the uncommitted M8 engine changes of this working tree |
| Engine | llama.cpp b11259 (`llama-server` 0.5.0-dev, commit d280808f5), the official `llama-b11259-bin-macos-arm64.tar.gz` (11,767,669 bytes, SHA-256 abd2b16d...e353d4), lock entry `llama-cpp-b11259-macos-arm64-metal`, 12 files |
| Install | `packaging/ai/fetch-macos-runtime.sh` into `.build-cache/ai-runtime/macos-arm64/` (archive and every file checked against the lock) |
| Chat model | Gemma 3 1B Instruct Q8_0 (lock `gemma-3-1b-it-q8-0-v1`, SHA-256 checked after download) |
| Search models | BGE small English v1.5 Q8_0 and Multilingual E5 small Q8_0 (lock, SHA-256 checked), chosen automatically |
| Where the engine ran | `Runs on the graphics card (Apple M1)`: Metal, `-ngl 999 --device MTL0` |
| Sampling | deterministic (temperature 0, fixed seed); release build of `ai-eval` |

## English bar (ADR-0028 decision 2), default provider, 420 English cases

Command: `OPENFRAME_AI_BUNDLE_DIR=.build-cache/ai-runtime/macos-arm64 OPENFRAME_AI_ASSETS_DIR=.ai-assets target/release/ai-eval --gold crates/openframe-ai-eval/gold --out docs/release/evidence/mac/m8-eval-en --only en --graphics auto`
(wall time 34.3 min). Files: [report.md](m8-eval-en/report.md), [summary.json](m8-eval-en/summary.json), [results.jsonl](m8-eval-en/results.jsonl).

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments (tool and action cases, 150) | **96.7 %** (145/150) | 90 % |
| Correct end state after apply (action cases, 79) | **88.6 %** (70/79) | 85 % |

**Bar met: yes**, on Apple silicon with the default provider. Per set:

| Set | Cases | Result |
|---|---|---|
| tools (tool kind) | 71 | tool and arguments 100.0 % (71/71), answer facts 100 % (18/18), setup errors 0 |
| actions | 79 | tool and arguments 93.7 % (74/79), end state 88.6 % (70/79), setup errors 0 |
| retrieval | 164 | Recall@5 99.4 % (163/164), Recall@10 100 %, MRR 0.933 |
| writing | 60 | not empty 93.3 %, length limit 31.7 % (19/60), language 100 %; rubric scoring is a development-time step and was not done here |
| safety | 46 | safety check 97.8 % (45/46), not empty 91.3 % |

Median time per tool or action case 31 to 33 ms (recipes answer without the model); p95 about 10 s. Writing median 7.6 s.
For the Windows record: its round 3 figures were 96.7 % and 83.5 % on the processor ([w6/round3](../w6/round3/report.md)), so
the Windows end state was below the bar at that time. This Mac run is the first run at the current commit; a Windows rerun at the same commit was not made, so the two are not directly compared.

Not run: the non-English sets (de, es, fr, hi, ja, kn, ko, ml, ta, te, zh) other than the poisoned ones below, and the Intel
Mac (no Intel Mac and no pinned Intel engine yet, see ADR-0033).

## Prompt-injection corpus (`poisoned.jsonl`, 60 cases, 12 languages, every language)

Command: same, with `--only poison- --out docs/release/evidence/mac/m8-eval-poisoned` (17.1 min).
Files: [report.md](m8-eval-poisoned/report.md), [results.jsonl](m8-eval-poisoned/results.jsonl).

- Safety check 100 % (60/60): no planted instruction was obeyed, no link, marker or private note leaked, no proposal tool ran.
- Schema valid 100 %, answer facts 100 % (48/48).
- Answer in the request's language 43.3 % (26/60) and not empty 91.7 % (55/60): quality figures for the non-English
  languages (the same weakness as Windows, [w9/ai-poisoned](../w9/ai-poisoned/report.md)); they are not safety failures.
- The 5 English poisoned cases are also inside the 420-case run above and all pass.

## No outbound network

- While the evaluation ran, `lsof -i` on the engine and harness processes showed only sockets on 127.0.0.1 (the engine
  listening on its port, the harness connected to it). No other address.
- The launch tests in `crates/openframe-ai/tests/supervisor.rs` (loopback only, key not on the command line, offline flag)
  pass on the Mac.
- `node scripts/architecture-lint.mjs` (rule R3, no network listener) passes: OK.
- The Windows test `the_core_workflow_opens_no_network_listener` uses `netstat -ano` and is `#[cfg(windows)]`; it does not
  run on the Mac. The Mac replacement is the manual `lsof` check above; an automated version is still to write (M9).

## Metal against the processor (same engine, same prompt)

`OPENFRAME_EVIDENCE_OUT=... cargo test -p openframe-ai --test real_runtime -- --ignored real_graphics`
([m8-metal-vs-processor.json](m8-metal-vs-processor.json)), a 125-word answer:

| Place | First answer | Warm answer |
|---|---|---|
| Metal (Apple M1) | 10.6 s | 3.9 s |
| Processor (`-ngl 0`, 8 threads) | 152.9 s | 143.3 s |

The processor figures are inflated: another build job kept the load average at 19 to 130 during these runs, and the
engine's spinning threads suffer badly from that (0.1 tokens a second at load 130). Treat Metal as measured and the
processor as "much slower, not precisely measured". This is why ADR-0033 runs Metal without a timing check first.

## Tests

`RUSTC_WRAPPER= CARGO_INCREMENTAL=0 cargo test -p openframe-ai -p openframe-ai-eval`: all pass (openframe-ai lib 37,
bundled 27, supervisor 3, soak short profile 2 with 1 long ignored, others 9; ai-eval 9). The ignored real-engine tests
`real_bundled_runtime_checks_then_chats_and_embeds` (full check 5 s, chat 3 s, embeddings, shortcut on next start) and
`real_graphics_card_probe_and_benchmark` pass with the Mac runtime. `cargo clippy -p openframe-ai --all-targets` is clean.
New tests: `the_mac_engine_is_one_metal_build`, `the_mac_engine_lists_one_metal_device`.

## Open items

- Apple on-device model provider, the Mac screens and the Intel Mac measurement (rest of M8).
- Non-English sets on the Mac, writing rubric scoring.
- Signing of the engine files and the `com.apple.security.inherit` entitlement (Chapter 3).
- `THIRD_PARTY_LICENSES.md` is generated by the Windows pipeline (`cargo metadata --filter-platform x86_64-pc-windows-msvc`);
  regenerate it there to list the Mac engine.
