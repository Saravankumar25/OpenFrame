# End-to-end runs on the release build (release gate P3 progress)

> 2026-10-18. All runs drive the gated release executable built by `tests/e2e/run.mjs`: release profile, team password
> screen on, and the test model host compiled in so specs can download from a local host.

There are 39 spec files. Two (15 and 16) are skipped by design: they set the licence state through
`entitlement.debug_set`, which a release build never registers. That leaves 37 runnable spec files.

| Run | Harness | Specs | Passed | Failed | Notes |
|---|---|---|---|---|---|
| 1 | webdriverio 10 | 39 files, 2 skipped | 17 | 22 | See below |
| 2 | webdriverio 10, strict selectors off, `executeAsync` rewritten, rerun of the 22 | 22 | 17 | 5 | See below |
| 3 | webdriverio 9, specs 01, 02, 41, 62, 90, 91, 95 | 7 | 7 | 0 | |
| 4 (final) | webdriverio 9, whole suite | 39 files, 2 skipped | 188 tests | 0 | See below |

## Run 1 (2026-10-18, about 01:50)

The cause of the 22 failures was the webdriverio 10 upgrade: strict selector errors and `executeAsync is not a function`.
The same specs pass on webdriverio 9.

One failure had a second cause. Spec 04 round 20 (Alt+F4): the text was saved and reopened, but under heavy load it was
typed at the start of an existing line. The virtual machine was booting at the time and the spec took 254 seconds, so
the exact-line check missed it. The screenshot showed the text present.

## Run 2

| Spec | Cause |
|---|---|
| 90, 91, 95 | The rerun list left out spec 01, which creates their shared project |
| 41 | Ctrl+Alt+1 after a relaunch: webdriverio 10 key handling |
| 62 | Element click intercepted: webdriverio 10 click handling |

The harness went back to webdriverio 9 (commit 1fbc0db). See [supply-chain.md](supply-chain.md) for the audit exception.

## Run 3

Specs 01, 02, 41, 62, 90, 91 and 95 on webdriverio 9: 7 of 7 passed.

## Run 4 (final)

2026-10-18, about 06:00 to 08:30, gated release executable, nothing else running on the laptop. 39 spec files passed
(15 and 16 skipped by design: they need the debug-only licence switch), 188 tests, 0 failures. Log:
`C:\w9-vm\e2e-gated-final.log`. This covers gate P3 and the performance and crash specs named in P1 and P2: spec 97
(typing, idle and memory budgets), spec 26 (forced kill) and spec 04 (close keeps typing).

Limit. The suite drives an executable that `run.mjs` builds from the same commit with the local test model host compiled
in. It does not drive the installer file itself. The installer's executable is the same code with Hugging Face as the
model source. Code commits after 995bfac changed only tests, the test harness, the AI eval harness and documents.

## Net

Every runnable spec file passed on the gated release build in the final run.
