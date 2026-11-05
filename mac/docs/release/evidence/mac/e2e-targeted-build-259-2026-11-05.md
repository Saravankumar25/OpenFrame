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

## Not run on build 259

- The rest of the complete run. It rests on build 256 and the diff above.
- The Welcome window on the installed app: it opens only after a purchase, in the store build or a scripted development
  run. Clicking an account through to a browser was not done.
- Build 259 on the owner's real data. It waits at "Opening Straft Studio" for the macOS Documents prompt, which cannot be answered on a locked screen. A copy of the data was backed up first, and `library.json` was unchanged.
- `cargo deny` and `cargo audit`: neither is installed on this Mac.
- AI evaluation: not run after the name change.
