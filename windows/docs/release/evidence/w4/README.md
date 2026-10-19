# Windows phase W4 evidence: release proof (2026-10-17)

Proof level 3 (ADR-0030): the whole desktop end-to-end suite drives the **release executable** (`tauri build
--no-bundle`, gated variant with a throwaway team password made for the run), not the debug test build. No installer was
built (owner rule). The owner's laptop, Intel Core Ultra 5 125H, Windows 11 Home, on mains power, left alone during the
run. Branch `windows/w8-download` at the commit that follows this page.

## Result

**39 spec files, 227 checks, 0 failures** ([release-run-2026-10-17-gated.txt](release-run-2026-10-17-gated.txt), the
spec and check lines of the run log). Specs 15 and 16 are skipped by design on a release build: they set the licence state
through `entitlement.debug_set`, which a release build never registers.

What the run covers on the release executable includes: start, create, type, close and reopen (00); closing within
100 ms by the close button and Alt+F4, 20 rounds (04); the web view's own network use (05); forced kill keeps work (26);
every workspace and export; the AI assistant with the real models, sources, navigation, loopback-only sockets and
relaunch (72 to 75); missing, damaged and restored model files (76); the download after install from the local test host
at Hugging Face paths, resume, offline answers, restart offline, remove, damaged file set aside, pause and resume (78);
visual planning (86); visual baseline, themes and reachability (90, 91, 95); keyboard and screen reader (96); performance
budgets for typing, idle and memory (97).

## Fixed on the way (all in the commits of 2026-10-16 and 2026-10-17)

| Found by the release runs | Fix |
|---|---|
| A fresh download thrown away when the first graphics check restarted the engine during the health check | The health check decides the engine's place first; flow test `a_download_checked_while_the_app_warms_up_keeps_the_new_set` |
| A scene's first sketch panel saved but not shown | The storyboard view refreshes itself after a create |
| Close helper error hidden by PowerShell progress output; Windows refusing the foreground window | Progress output silenced; foreground retries |
| Specs 72 and 76 describing the retired bundled AI; spec 72 expecting the model for a plain count | Rewritten for the download design and the W6 recipes |

The open variant (store build, no team password) is recorded in
[release-run-2026-10-17-open.txt](release-run-2026-10-17-open.txt).
