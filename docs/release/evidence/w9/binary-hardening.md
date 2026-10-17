# Binary hardening (release gate S12)

> 2026-10-18, branch `windows/w9-endgame`. Tool: Microsoft BinSkim 4.4.9.13 (NuGet). The Authenticode signature of the
> tool package was valid. Raw results are in [binskim/](binskim/): `app.sarif` (first run on the app),
> `app-after.sarif` (after the fixes in commit 995bfac), `engine.sarif` (the AI engine files).

## The app: `openframe-desktop.exe` (release build, commit 995bfac)

| Rule | Check | Result |
|---|---|---|
| ASLR, high-entropy VA | Address layout randomised, 64-bit address space | Pass |
| DEP | Non-executable data | Pass |
| Control Flow Guard | Indirect call checks | Pass |
| Writable and executable sections | None present | Pass |
| BA2025 CET shadow stack | Was a warning. Release builds now link `/CETCOMPAT`, and `scripts/check-binary.mjs` fails the build without it | Pass (fixed in 995bfac) |
| BA2004 SHA-256 source hashing | C code built by the `cc` crate now uses `/ZH:SHA_256` (995bfac) | Pass for our code; exception below for ring |
| BA2014 stack protection | See exception 2 | Exception |
| BA2024 Spectre mitigations | See exception 3 | Exception |

## The AI engine: 45 files

llama.cpp b11259 upstream prebuilt binaries and libomp, pinned by SHA-256 in `packaging/ai/ai-assets.lock.json`.

| Rule | Result |
|---|---|
| BA2008 Control Flow Guard | Missing on all 45 files. Exception 4 |

## Exceptions (all waiting for the owner)

| # | Rule | Where | Why it is acceptable | Review |
|---|---|---|---|---|
| 1 | BA2004 SHA-256 hashing | The only entries left are the prebuilt NASM assembly objects of the `ring` crate (aes, chacha, sha, p256 and similar `*-nasm.o`), assembled by NASM 2.13.03 with MD5 checksums. TLS uses them for the model download | Warning level. Third-party prebuilt assembly we do not compile | Re-run BinSkim when `ring` is updated |
| 2 | BA2014 stack protection | Flags only Rust standard library functions (`std::sys::fs`, `core::fmt`, `compiler_builtins`) | Stable Rust has no `/GS` stack cookies (stack-protector is nightly only). Rust code is memory safe by construction | Recheck when the option is stable |
| 3 | BA2024 Spectre mitigations | Warning. The rule applies to MSVC C and C++ code built with `/Qspectre` | Our code is Rust. The C we build is small | Recheck with the next toolchain |
| 4 | BA2008 Control Flow Guard on the engine | All 45 engine files | The fix is to build llama.cpp ourselves with `/guard:cf`, which changes the pinned files and the AI evaluation baseline. Mitigations below | Owner decides whether to build the engine in house |

Mitigations for exception 4:

- The engine runs as a separate process with an empty environment.
- It listens on loopback only, with a per-launch key.
- A Job Object with kill-on-close ends it when the app exits.
- A start that cannot be verified fails closed.
- Every engine file hash is checked before each launch (`an_engine_whose_files_changed_after_install_is_not_started` in
  `crates/openframe-ai/tests/supervisor.rs`).
- Its inputs are only hash-verified model files and local requests.
