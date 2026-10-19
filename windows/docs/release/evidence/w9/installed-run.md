# Installed program run (release gate G2)

> 2026-10-18. The final installer, installed per user, driven by the release suite. Gate: [windows-internal-release-gate.md](../../windows-internal-release-gate.md).

## The installed copy

| Item | Value |
|---|---|
| Installer | `release-out\OpenFrame-Studio-0.1.0-internal-028d8b6.exe`, 27,016,597 bytes, SHA-256 `97b712214109e5812ce87cf226a4bd234f25b3fac65b68735e97fea0d787d50f` |
| Install | Per user, to `%LOCALAPPDATA%\Programs\OpenFrame Studio` |
| Installed `openframe-desktop.exe` SHA-256 | `91bac2bccf38eaffe811eea9aa24bea2952330ca9b94c4119e66d7af1f5306c2` |
| Plain release build | `0d380442...` |
| Difference | 3 bytes at Tauri's `__TAURI_BUNDLE_TYPE` marker (UNK becomes NSS). Byte comparison done. The installer sets the marker; the code is the same |
| Uninstall of an earlier copy | Silent uninstall kept `Documents\OpenFrame` (projects kept: True) |
| Reinstall | Worked |

## The suite

Command: `run.mjs --app=<installed exe> --release=gated` (the mode was added in commit 7e11775).

| Result | Count |
|---|---|
| Tests | 179 |
| Passed | 178 |
| Failed | 1 |
| Spec files passed | 35 |
| Skipped, specs 76 and 78 | They need the local test model host. The real download is covered by the W10 pilot |
| Skipped, specs 15 and 16 | Debug-only |

## The one failure

Spec 04 close-keeps-typing. On the installed program its own precondition stopped a round: the test helper must send the
close within 100 ms of the last key, and it took 100 to 111 ms in reruns, also on a quiet machine (round 15 at 105 ms,
after 28 closes that all kept the text). No text was ever lost. On the release build of the same commit spec 04 passes
all 40 rounds (final run, [e2e-release-runs.md](e2e-release-runs.md)).

## Status

Pass, with this deviation: one test helper precondition missed on the installed program, no data loss in any round.
