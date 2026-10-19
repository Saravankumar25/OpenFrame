# Windows phase W5 evidence: measuring the Offline AI (2026-10-16)

Proof level 2 (development computer, ADR-0030): the owner's laptop, Intel Core Ultra 5 125H (14 cores, 18 logical),
15.4 GB, Windows 11 Home, on mains power (checked before every run), Balanced power plan. Engines: llama.cpp b11259,
processor and Vulkan builds. Chat model: Gemma 3 1B Instruct Q8_0 (the pinned file).

## Method

1. `ai-eval` ran 30 tool cases of the approved gold sets through the real assistant pipeline on a release build and
   recorded every request body the assistant sent and the engine's own numbers per case
   ([record-results.jsonl](record-results.jsonl); the bodies stay in the git-ignored build cache).
2. `engine-bench` (`crates/openframe-ai-eval/src/bin/engine-bench.rs`) replayed those exact requests against
   `llama-server`, one setting changed at a time, after one warm-up request, and wrote medians, quartiles and ranges
   from the engine's `timings` (prompt and generation separately) plus the wall time. Each JSON file below is the raw
   output; every run can be repeated with the command in the tool's header.

## Results

**Where the time goes** (30 real cases, per case): prompt reading 20.9 s median (12.2 to 25.1 interquartile, 2.2 to 31.9
range); writing 2.8 s (2.5 to 3.6); prompts 2,851 tokens per call (1,525 to 3,779); answers 78 tokens. **86 percent of the
model time is reading the prompt.** Two model calls per case (median).

| Experiment (file) | Setting | Prompt tokens/s | Writing tokens/s | Request time | Verdict |
|---|---|---|---|---|---|
| Threads ([threads.json](threads.json)), 5 prompts × 9 runs each | 14 (old rule) | 117 (113 to 124) | 26 | 25.7 s (24.7 to 26.4) | |
| | **12** | **138 (137 to 139)** | 30 | **21.7 s (21.5 to 22.0)** | adopted for chips with 12 or more cores |
| | 10 / 8 / 6 | 129 / 123 / 108 | 27 / 28 / 26 | 23.3 / 24.6 / 28.1 s | |
| Graphics card ([card.json](card.json)), 3 prompts × 9 runs | processor, 12 threads | 111 (110 to 112) | 26 | 27.2 s (26.7 to 27.7) | |
| | **Intel Arc, every layer** | **494 (483 to 501)** | 27 | **7.1 s (6.8 to 7.4)** | the card check now measures this (probe version 2) |
| Structured output ([schema.json](schema.json)) | JSON schema on | 109 | 26.6 | 27.6 s | kept: about 3 percent of a request, guarantees valid answers |
| | off | 111 | 29.3 | 26.7 s | |
| Quantisation ([quant.json](quant.json)) | Q8_0 (1,020 MB) | 117 | 23.7 | 26.4 s (24.3 to 27.5) | kept (owner's model set) |
| | Q4_K_M (769 MB) | 125 | 23.3 | 24.2 s (24.1 to 26.8) | about 8 percent, ranges overlap |
| Prompt reuse ([reuse.json](reuse.json)), 4 two-step cases × 2 | cache on (the engine's default) | | | 30 s per case | the second step reuses 96 percent (86 to 97) of its prompt |
| | cache off | | | 57 s per case | nothing to change; the app already gets this |

Across new questions the first part of the prompt is shared (median 53 percent of the characters between consecutive
requests); reordering the prompt for more reuse is a prompt change and waits for the W6 gold-set rule (ADR-0028).

**Search models** ([embed.json](embed.json)): ten screenplay sentences in eleven languages, each compared with the ten
English originals. BGE small English: German 9, Spanish 8, French 6, Chinese 3, Japanese 3, Korean 1, Hindi, Telugu,
Tamil, Kannada and Malayalam 1 each (chance). Multilingual E5 small (TwinSunsLLC GGUF, `b6cac96`): 10 of 10 in every
language. The `cstr/multilingual-e5-small-GGUF` file does not load in llama.cpp b11259 ("bert model needs to define token
type count").

**Retrieval gold sets, BGE against E5** ([retrieval/](retrieval/); 252 cases, the vectors ready in every case, one indexed
project per sample script): E5 232 of 252 (92 percent), BGE 202 of 252 (80 percent). English 163 of 164 against 161;
Hindi, Kannada, Malayalam and Tamil 7 or 8 of 8 against 2; Telugu 7 against 5; German 8 against 5; Spanish 7 against 6;
French, Japanese, Korean and Chinese 4 or 5 of 8 with either model (a ranking weakness for W6, not a model one).

## Changes made from these figures

- The chat engine uses 12 threads on chips with 12 or more cores (`manager.rs` `threads`).
- The graphics card check sends about 2,000 tokens of context and compares tokens handled per second over the whole
  call (`placement.rs`, probe version 2); every card is checked again (ADR-0029 note 7).
- Multilingual E5 small is the third pinned model; the search index uses it when the app pins it (ADR-0032 decision 8),
  since it is at least as good in English and far better elsewhere. BGE stays pinned (owner's model set) and is used
  when no multilingual model is pinned or when the English model is chosen on purpose.

## Not measured here

The gold-set accuracy of each change (the W5 baseline report, next), other computers and cards (the owner's decision:
only computers that can be tested), the time to the first word (answers are not streamed).
