# Mac AI baseline before the Windows sync (ADR-0028: measure first)

| Item | Value |
|---|---|
| Code | commit `cdc0922f` (branch `mac/windows-sync` before the core sync): the Mac assistant with its eight deterministic routes |
| Binary | `cargo build --release -p openframe-ai-eval --bin ai-eval`, copied aside before any core change |
| Command | `OPENFRAME_AI_BUNDLE_DIR=.build-cache/ai-runtime/macos-arm64 OPENFRAME_AI_ASSETS_DIR=.ai-assets ai-eval --gold <gold copy> --out <this folder> --only en --graphics auto` |
| Machine | Apple M1, 16 GB, macOS 26.5; engine on Metal (llama.cpp b11259, `-ngl 999 --device MTL0`), Gemma 3 1B q8_0 |
| Data | the Mac gold sets of that commit (`tools.en`, `writing.en`, `retrieval.en`, `safety.en`), 430 cases, run once |
| Wall time | 67.8 min. Rust and Swift builds ran on the same machine for part of the run, so the times are not quiet-machine times; the accuracy measures are deterministic |

The Windows Low Tide fixtures could not be loaded by this eval without the rest of the Windows code (the fixture loader
needs the new index and dossier code), so this baseline is on the Mac's own sets, as the port orders allow.

| Measure | Result | Bar |
|---|---|---|
| Correct tool and arguments | 98.1% (157/160) | 90% |
| Correct end state after apply | 89.9% (71/79 action cases) | 85% |
| Retrieval recall at 5 | 99.4% | |
| Safety | 100% (46/46) | |
| Median / p95 time | 771 ms / 26.4 s | |

Full tables: `report.md`, `summary.json`, `results.jsonl`.
