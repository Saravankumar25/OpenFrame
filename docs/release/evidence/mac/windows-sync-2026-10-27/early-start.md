# Early engine start on the Mac (port orders phase 6, 2026-10-27)

Orders: `docs/roadmap/mac-ai-harness-port-orders.md` section 8, items 1 to 3.

## Machine and method

- Apple M1, 16 GB, macOS 26.5.2. llama.cpp bundle `.build-cache/ai-runtime/macos-arm64` with Metal; Gemma 3 1B q8_0
  (1,069,306,368 bytes, 8,192 tokens of context); deterministic sampling; graphics `auto` (every run placed on
  "the graphics card (Apple M1)").
- Not a quiet machine. Other agents were building and testing Swift during the runs: the end-to-end lock
  (`.build-cache/e2e.lock`) was held for every run, the Mac app was open, and `dsymutil` or `swift-frontend` used a
  full core in pairs 3 to 5. The load average is in each row.
- Two `ai-eval` binaries built from one source snapshot (`git archive` of `5c6aeae5` plus the memory fix below), the
  only difference being `START_ENGINE` in `crates/openframe-application/src/modules/ai/intelligence/warm.rs`. Both
  run with `--warmup on`, so both build the project index and the example vectors before the first question; only
  the "on" binary also starts the chat engine and reads the fixed prompt starts (`AiManager::prime`). Comparing
  `--warmup on` with `--warmup off` on one binary would also have removed the index and example warm-up, so it would
  not isolate the engine start.
- Command per run, a fresh process each time, the order alternating between pairs:
  `ai-eval --gold crates/openframe-ai-eval/gold --out <dir> --only conv-lt-scene-001 --graphics auto --warmup on`
  with absolute `OPENFRAME_AI_BUNDLE_DIR` and `OPENFRAME_AI_ASSETS_DIR`. The first-question time is
  `first_question_ms` of `latency.json` (the time of `ai.ask` for "Explain scene 3 of LOW TIDE RADIO.", three engine
  calls, 2,898 prompt tokens, 444 written).

## Memory reading that blocked the early start

`hardware::inspect().available_ram_bytes` came from sysinfo 0.37.2. On macOS sysinfo computes
`(free_count + inactive_count + purgeable_count - compressor_page_count) * page size` from `host_statistics64`.
The pages the compressor occupies are not part of that sum, so the subtraction takes them off twice, and on a machine
that has been in use the reading falls far below what macOS hands back without pressure.

Three readings in a row (MB):

| Kernel: free (with speculative) + inactive + purgeable | sysinfo | vm_stat free + speculative + inactive + purgeable | Compressor | `kern.memorystatus_level` |
|---|---|---|---|---|
| 5,553 | 1,967 | 882 + 123 + 4,459 + 88 = 5,552 | 3,586 | 63% |
| 5,563 | 1,977 | 885 + 121 + 4,461 + 97 = 5,564 | 3,586 | 63% |
| 5,562 | 1,976 | 884 + 121 + 4,460 + 97 = 5,562 | 3,586 | 63% |

The early start needs `model_mb + EARLY_START_HEADROOM_MB` = 1,275 + 1,536 = 2,811 MB free. With sysinfo (1,967 MB
here, about 900 MB earlier in the day) it never started on this Mac; with the kernel reading it does.

Fix (`crates/openframe-ai/src/hardware.rs`, macOS only): the kernel's free (speculative included), inactive and
purgeable pages from `host_statistics64(HOST_VM_INFO64)` through `libc`, times `sysconf(_SC_PAGESIZE)`, nothing
taken off for the compressor, never below the sysinfo reading and never above the installed memory. Any kernel error
keeps the sysinfo reading. Windows and Linux are unchanged. `libc` was already in the lock (openframe-platform); it is
now a macOS-only dependency of openframe-ai. On the Mac this reading feeds `room_to_start` and Settings; the Metal
placement checks installed memory only, and `placement::plan` with free memory runs on Windows only.

## Item 1: first-question time, early start off and on (ms)

| Pair | Order | Off | On | On minus off | Off: start and other | On: start and other | Load, 1 min (off, on) |
|---|---|---|---|---|---|---|---|
| 1 | off, on | 11,510 | 10,374 | -1,136 | 1,177 | 87 | 3.9, 3.8 |
| 2 | on, off | 16,418 | 10,836 | -5,582 | 1,359 | 93 | 3.6, 4.2 |
| 3 | off, on | 16,052 | 14,038 | -2,014 | 1,708 | 109 | 4.2, 5.7 |
| 4 | on, off | 13,911 | 12,103 | -1,808 | 1,186 | 110 | 7.2, 7.0 |
| 5 | off, on | 14,696 | 15,128 | +432 | 1,722 | 124 | 5.7, 12.9 |
| Mean | | 14,517 | 12,496 | -2,022 | 1,430 | 105 | |

"Start and other" is the first-question time minus the engine's prompt reading and writing times (the rest is the
engine start, when there is one, and about 55 ms of turn and dossier work).

Each run in detail (engine prompt ms, engine writing ms, cached of prompt tokens):

| Run | Prompt | Writing | Cached |
|---|---|---|---|
| 1 off | 992 | 9,341 | 1,963 of 2,898 |
| 1 on | 980 | 9,307 | 1,964 of 2,898 |
| 2 on | 998 | 9,745 | 1,964 of 2,898 |
| 2 off | 1,058 | 14,001 | 1,963 of 2,898 |
| 3 off | 1,078 | 13,266 | 1,963 of 2,898 |
| 3 on | 1,085 | 12,844 | 1,964 of 2,898 |
| 4 on | 1,204 | 10,789 | 1,964 of 2,898 |
| 4 off | 1,021 | 11,704 | 1,963 of 2,898 |
| 5 off | 1,044 | 11,930 | 1,963 of 2,898 |
| 5 on | 1,063 | 13,941 | 1,964 of 2,898 |

In the "on" runs the warm-up finished 1.2 to 1.7 s after the index with the engine Ready, before the first question.

Reading:

- The early start removed the engine start from the first question in all five pairs: 1.2 to 1.7 s off, about 0.1 s
  on.
- Prompt reading was the same with and without it (980 to 1,204 ms either way). Unlike the Windows measurement, the
  Mac showed no slowdown from the early start. Reading the fixed prompt starts early gained nothing measurable: the
  first question reused 1,964 tokens against 1,963 without it.
- The total was shorter with the early start in four pairs of five (mean 2.0 s shorter, median 1.8 s). Writing time
  (9.3 to 14.0 s for the same 444 tokens) moved with the load from other agents and dominates the spread; pair 5
  "on" ran at a load of 12.9.

Recommendation: the measurement favours the early start on the Mac, but `START_ENGINE` is one constant of the shared
core and Windows measured it slower, so it stays **off** in this change. If the owner agrees, make it Mac-only
(`const START_ENGINE: bool = cfg!(target_os = "macos");`) and confirm with five pairs on a quiet Mac. The cost is the
chat engine's memory (below) held from the moment a project opens with AI in use, whether or not a question follows.

## Item 2: engine memory on Metal against the placement checks

One "on" run sampled every second (`footprint` and `ps`):

| Engine | Peak own memory (phys_footprint) | Peak resident (with the mapped model) |
|---|---|---|
| Chat (Gemma 3 1B, Metal, 8,192 context, prompt store 512 MB) | 259 MB | 1,316 MB |
| Search (BGE small) | 433 MB | 655 MB |

- The placement estimate for the chat model is `model_mb` = 1,019 + 8,192 x 32 / 1,024 = 1,275 MB. Resident at peak:
  1,316 MB with one conversation in the prompt store.
- The prompt store (`--cache-ram 512`) is not in `model_mb`. Full, the chat engine would reach about 1,316 + 512 =
  1,830 MB, 555 MB above the estimate, inside `MARGIN_MB` (1,024).
- Metal check (`choose_metal`): installed memory at least 1,275 + 1,024 + 2,048 = 4,347 MB. It holds on 8 GB and
  16 GB Macs with the full prompt store inside the margin.
- Early start check: 2,811 MB free leaves about 980 MB after a full chat engine. The search engine is already running
  when the check is made, so its memory is already out of the free reading.
- A per-second sampler in the paired runs picked the newest `llama-server`, sometimes the search engine; its column
  was not used.

## Item 3: power and AI off (code and existing tests)

- Warm-up skips when AI is off. It holds. `intelligence::session_changed` calls `warm::start` only when
  `ai_available` (bundled AI available or a local model attached). `AiManager::prime` returns at once when the
  manager is not available. Tests: `warm::tests::without_an_engine_it_ends_quietly_and_a_stopped_index_ends_it_at_once`
  (no engine, then a stopped index ends the warm-up in under 500 ms) and the first half of
  `the_engine_is_prepared_early_without_counting_and_obeys_stop` (no models: nothing starts; `stop` true: the engine
  stays down).
- `power_event(sleeping)` does not stop background work. It does not hold. `CoreHandle::power_event(true)` (openframe-ffi) calls
  `AppCore::on_power_event(Suspending)`, which only folds the write-ahead logs into the database files (test
  `failure_injection.rs`, "project log folded before sleep"). It does not stop the indexer, the warm-up or the
  engines. With `START_ENGINE` off nothing large runs in the background, so this is a gap to close in the shared core
  before an early start is turned on, not a fault today.

## Checks run

- `cargo test -p openframe-ai`: all pass, including the two new hardware tests and
  `the_engine_is_prepared_early_without_counting_and_obeys_stop`.
- `cargo clippy -p openframe-ai --all-targets -- -D warnings`: clean. `cargo fmt -p openframe-ai`: applied.
- Not run: the Windows and Linux builds of `hardware.rs` (the macOS code is behind `cfg(target_os = "macos")`); five
  pairs on a quiet Mac.
