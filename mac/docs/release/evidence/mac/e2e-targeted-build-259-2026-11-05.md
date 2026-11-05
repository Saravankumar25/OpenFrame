# Build 259 on top of the complete run on build 256

2026-11-05. The owner asked that scenarios already passed on unchanged code are not repeated, and that only changed areas
are tested. So build 259 has no complete run of its own. This page says what was run and what it rests on.

## What changed between build 256 and build 259

Commit d886ab56 (build 256, [complete run](e2e-final-build-256-2026-11-05.md)) to 0d048fd3 (build 259, `Straft-12.dmg`). In
the app: the Welcome window's accounts (`Store/SocialLinks.swift`, `Store/LicenceScreens.swift`, the old list out of
`Store/LicenceRules.swift`). In the core: nothing, one test added. Commit cacc08e9 changes only the test driver.

## What was run on the changed code

| Check | Result |
|---|---|
| Swift unit tests, with `theEightOfficialAccountsHaveTheirExactAddresses` and `onlyOfficialAddressesOpen` | 817 of 817 |
| `straft-security` tests, official addresses pass `web_link` | passed |
| `99-store` on the development build (the Welcome window is reachable only there) | 7 of 7 |
| `71-storage/first-start` on the development build | 2 of 2 |
| Scenarios on installed build 259 before the run was stopped on purpose | 58 passed, 0 failed |

The 58 are the first scenarios of a complete run, stopped once it was clear that it repeated build 256: release smoke,
journey, Story page, Idea Vault, forced kill, screenplay, writing comfort, several screenplays, scene hub, keyboard and
screen reader.

The installed executable is byte-identical to the one inside `Straft-12.dmg` (`cmp`).

## The three drag scenarios, by hand

`86-visual-planning/last-panel-is-dragged-to-the-front`, `moodboard-note-is-dragged` and
`note-and-panel-order-survive-a-restart` need a real mouse and were blocked on build 256. They are still not automated
passes: the driver's helper needs computer control of the test copy, whose identifier is
`app.straftstudio.studio.e2e1`, and that access request was refused. The same steps were made by hand with a real mouse on
the installed app, with `STRAFT_DATA_ROOT` set to a throwaway folder:

| Step | Seen |
|---|---|
| Moodboard note dragged down and to the right | The note moved and stayed |
| Storyboard panel 3 dragged onto panel 1 | "Moved panel"; the order became empty, sketch, empty |
| Quit and start again on the same folder | The note is where it was dropped; the order is unchanged |
| The owner's `library.json` after the session | Same SHA-256 as before |

The window frame this session wrote to the app's preferences was put back from an export made beforehand.

## Seen by eye on build 259

Dock icon, menu bar, About window (Straft logo, name, version 0.1.0, "Internal build, not for sale."), Welcome screen,
first project.

## MacHTML

771 screens in Light and Dark from build 259, every page passing its check: 1,542 pages, no broken reference, no old name, no temporary path. Library setup comes from the development build of the same code. Comparing the new pictures with the earlier set found 19 pages per mode showing Straft AI as missing, because that set came from a development build, which has no AI engine.

## Security scans

`cargo audit` 0.22.2: 0 vulnerabilities; warnings only (7 unmaintained crates, 1 unsound, 1 yanked). `cargo deny` 0.20.2: advisories, bans, licences and sources pass. It failed first on one ban: `native-tls`, which the Mac uses on purpose so the model download goes through Apple's Security framework. The ban now allows it only through `reqwest`, `hyper-tls` and `tokio-native-tls`. `cargo tree` shows it absent from the Windows graph and `openssl-sys` absent from the Mac graph.

## AI evaluation

The prompts carry the product name, so the name change reached what the model reads. Same pinned model, deterministic sampling, Metal on the M1, 2026-11-05:

| Set | Measure | Before the name change | Now |
|---|---|---|---|
| Section answers, 393 cases ([report](../../ai-reharness/build-259/sections/report.md)) | Answer facts | 322 of 336 | 324 of 336 (96.4%) |
| | Length | 39 of 40 | 39 of 40 |
| | Not empty | 393 of 393 | 393 of 393 |
| Prompt injection, 60 cases ([report](../../ai-reharness/build-259/poisoned/report.md)) | Safety | | 60 of 60 |
| | Answer facts | | 48 of 48 |
| | Answer in the request's language | | 36 of 60 |

## Not run on build 259

- The rest of the complete run. It rests on build 256 and the diff above.
- The Welcome window on the installed app: it opens only after a purchase, in the store build or a scripted development
  run. Clicking an account through to a browser was not done.
- Build 259 on the owner's real data. It waits at "Opening Straft Studio" for the macOS Documents prompt, which cannot be answered on a locked screen. A copy of the data was backed up first, and `library.json` was unchanged.
- The older AI sets (844 English cases, 107 minutes). They measure the old road; whether they still count is an open owner decision.
- The Reddit address from this Mac: the browser here refuses that site and scripted requests get 403. LinkedIn was opened and shows the company page.
