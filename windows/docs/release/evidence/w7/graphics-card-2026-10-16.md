# Windows phase W7 evidence: graphics card (2026-10-16)

Proof level 2 (development computer, ADR-0030): the owner's laptop, Intel Core Ultra 5, 18 logical cores, 15.4 GB,
Windows 11 Home. Branch `windows/w8-download`. Engines: llama.cpp b11259, processor build and Vulkan build, pinned in
`packaging/ai/ai-assets.lock.json`. Model: Gemma 3 1B Instruct Q8_0.

## Unit and scenario tests (every computer)

| What | Where | Result |
|---|---|---|
| Decision table, 34 imagined computers | `crates/openframe-ai/src/gpu.rs` | pass |
| Device match by name, probe pass rule, probe record per adapter, driver and engine (bounded) | `gpu.rs`, `placement.rs` | pass |
| Card launch arguments (`-ngl 999 --device VulkanN`; processor `-ngl 0`, no device) | `tests/supervisor.rs` | pass |
| A card that passes is used and remembered; no second probe next session | `tests/bundled.rs` | pass |
| A slower card, a card with invalid output, and a card the engine does not list are not used, and remembered | `tests/bundled.rs` | pass |
| Off keeps the processor and runs no probe; back to Automatic probes and uses the card | `tests/bundled.rs` | pass |
| A card engine that stops in use: the request is answered on the processor, the card is remembered as failed | `tests/bundled.rs` | pass |
| No card: the processor, no probe, the plain reason "No graphics card was found." | `tests/bundled.rs` | pass |
| Settings, Graphics card (Automatic or Off) and the reason line | `OfflineAi.test.tsx` | pass |

## The whole path on this laptop's integrated graphics

`OPENFRAME_AI_BUNDLE_DIR=.build-cache/ai-bundle cargo test -p openframe-ai --features test-support --test real_runtime
-- --ignored real_graphics_card_probe_and_benchmark` (raw figures in
[intel-arc-integrated-2026-10-16.json](intel-arc-integrated-2026-10-16.json)).

- Detected: Intel(R) Arc(TM) Graphics (vendor 0x8086, 128 MB of its own, 9,009 MB shared, driver 32.0.101.8132) and
  the Microsoft Basic Render Driver (a software renderer, never used). Vulkan loader present.
- The Vulkan engine listed `Vulkan0: Intel(R) Arc(TM) Graphics`; the adapter was matched to it by name.
- The memory rule allowed a probe without the developer flag.
- Probe: valid schema output on the card at **30.3 tokens a second**; the processor reached **32.8**. The card is not
  faster, so the processor is used and the result is remembered for this driver and engine.
- Answers (114 words, a warm engine): 3.9 s in Automatic (processor after the probe), 4.1 s with the setting Off. The
  first answer in Automatic took 25.7 s once, because it ran the probe (two model loads); the next sessions skip it.

**Conclusion.** On this laptop the integrated graphics are checked and not faster; OpenFrame AI runs on the processor.
No speed-up is claimed for any card (ADR-0029 decision 8).

## Not measured here

Benchmarks on borrowed laptops with other cards (NVIDIA, AMD, a dedicated Intel card) need the owner and those
computers. Until then every other card counts as "designed and unit tested". The command above writes the same evidence
on any computer; set `OPENFRAME_EVIDENCE_OUT` to a new file in this folder.
