# End-to-end scenarios in plain steps

This folder holds every scenario of the desktop end-to-end suite (`tests/e2e/specs/`) as a numbered list of plain steps. The Mac track (Chapter 1B: a SwiftUI shell over the same Rust core, driven by XCUITest, ADR-0027) implements the same scenarios again from these files, without reading the JavaScript. The Windows suite stays the source: these files describe what it does, not more.

## The rule

A new spec, or a change to what a spec does, changes its scenario file in the same change. A spec without a scenario file here, or a scenario file without a spec, is a mistake to fix.

## Format

Each file is named like its spec without `.test.mjs` (for example `01-journey.md` for `01-journey.test.mjs`). Each file has:

- a title and one line naming the spec file;
- the header tags of the spec;
- a "Before" line with the starting state;
- one section per test of the spec, with a short heading in plain words and a numbered list of steps;
- "Notes" for Mac keys and anything a reader must know, and "Platform notes" for everything that is Windows only, with the Mac equivalent when it is obvious, else "Mac: to decide in phase M-track".

Each step is either an action a person takes ("Click New Project.", "Type 'Night Bus' in the project title field.", "Press Ctrl+Z.") or a check that starts with "Check:". Text typed by the person is in single quotes. Text the app shows is in double quotes, exactly as the spec checks it. Waits appear only inside checks ("Check: within 4 minutes the answer shows ..."). Key names are the Windows ones; each file's notes say where the Mac uses Cmd.

Some steps ask the core directly ("Ask the core ...", "Through the core: ..."). The spec does that to skip a system file dialog or to set up data quickly, the same way the screen calls the core. A Mac run may do the same, or drive the screens by hand where XCUITest can.

## Data folders

Each scenario file starts in its own empty data folder, unless it says "shared journey data". The shared journey data is one folder used, in this order, by `01-journey` (which creates the demo project "Night Bus"), `02-restart`, `90-visual-baseline`, `91-appearance-themes` and `95-reachability`. Those files depend on each other only through that folder and that order. Every other file creates its own project and never sees the shared folder, so any of them can run alone.

OpenFrame AI is downloaded after install (ADR-0026). Every test build is made with the local model host of the run (`127.0.0.1:47811`, `tests/e2e/lib/model-host.mjs`) in place of Hugging Face; it serves the pinned files at their Hugging Face paths (ADR-0032). Each data folder starts with the model set installed as a finished download leaves it, unless its file has the tag `@e2e-ai-models: none`. "The host" in a scenario is that local model host; a scenario can make it cut a file short, change a byte, send slowly or refuse every request, and can read what it was asked for.

## Header tags

| Tag | Meaning |
|---|---|
| `@e2e-data-root: own` | The spec gets its own empty data folder (see above). |
| `@e2e-fake-media` | The web view starts with a fake microphone and camera: no device, no permission prompt, so a voice note can be recorded. |
| `@e2e-ai-models: none` | The spec starts with OpenFrame AI not downloaded. Every other data folder starts with the model set installed the way a finished download leaves it (ADR-0026). |
| `@e2e-net-log` | The web view writes a network log of its own network code into the spec's data folder, so the spec can list every host it asked for. |
| `@e2e-debug-only: <reason>` | The spec needs a switch that the release build refuses (ADR-0031). A release run lists it with the reason and skips it. |

## Shared steps

These short forms appear in many files.

**Do the first launch** (with a name and a title): check the screen shows "Welcome to OpenFrame Studio"; select all text in the name field and type the name; click Continue in the dialog; click Create Project; type the title in the project title field; click Create in the dialog; check the Workspaces navigation shows and the window title is "<title> - OpenFrame Studio".

**Team password** (gated release run only): every start of the app first shows the team password screen. Type the run's password in the team password field and click Open OpenFrame Studio; check the screen closes.

**Check: the screen is quiet**: no console error or warning was recorded since the last check, every visible control has an accessible name (a label, a title or visible text), and neither the window nor the workspace scrolls sideways.

**Check: the AI Assistant docks cleanly**: the panel is open, covers no part of the page, does not run past the window, and no control of the page is cut off beside it.

**Quit the app and start it again on the same data**: end the app, wait about 1 second, start it again, and check Application Home shows ("Your projects") with no project open.

**Click the project row "Open X"**: on Application Home, click the project row whose name starts with "Open X", check the Workspaces navigation shows, wait about 1 second, and check no recovery question ("We found a recovery state") shows, because the project was closed cleanly.

**Open the project menu and choose X**: click the button whose name starts with "Project: " in the top bar, then choose the menu item that starts with X.

**Move focus off any text field**: the global Ctrl+Z and Ctrl+Y skip text fields, so focus leaves the field first.

**[em dash]**: stands for the em dash character (U+2014). Some specs type it on purpose in a scene heading or an act title. The repository writing check does not allow the character in these files, so it is written this way.

**Import the Night Shift screenplay**: click Create, choose Import Screenplay, paste the text below into the paste box, click Preview, check the screen shows "Import Screenplay · Preview", click Import, check the screen shows "Import complete", click Open Screenplay, and check the Screenplay workspace opens with the draft in the editor.

```
Title: Night Shift

INT. CONTROL ROOM - NIGHT

Kavya studies a flickering monitor and lifts a torch.

KAVYA
Someone is in the tunnel.

EXT. TUNNEL MOUTH - NIGHT

Ravi drags a heavy toolbox through the rain.

RAVI
Hold the gate.
```

**Import the Night Bus screenplay**: the same steps with this text (three scenes, used by the AI scenarios):

```
Title: Night Bus

INT. POLICE STATION - NIGHT

Arjun confesses to Inspector Rao.

ARJUN
I was there that night.

EXT. BUS STOP - NIGHT

Arjun meets Meera again at a rainy bus stop.

MEERA
You came back.

EXT. RAILWAY STATION - DAY

Meera waits for the late train with a red umbrella.
```

**Ask Q** (AI scenarios): open the AI Assistant if it is closed (click Assistant); wait until the Ask or command box is enabled; type Q and press Enter; check that within 4 minutes a new result shows directly after the question Q and no "working" status remains. The first answer also loads the model; one answer takes 20 to 40 seconds on a laptop processor. The AI runs with fixed sampling (temperature 0, fixed seed), so checks look at the kind of result and its effects, never the wording.

## Files

| Scenario file | Spec file | Scenarios | Platform notes |
|---|---|---|---|
| `00-release-smoke.md` | `00-release-smoke.test.mjs` | 3 | Engine and web view process names, Windows network commands |
| `01-journey.md` | `01-journey.test.mjs` | 15 | Minimum window size from the Windows configuration |
| `02-restart.md` | `02-restart.test.mjs` | 1 | None |
| `04-close-keeps-typing.md` | `04-close-keeps-typing.test.mjs` | 1 | Close button and Alt+F4 sent from outside the web view |
| `05-webview-network.md` | `05-webview-network.test.mjs` | 2 | WebView2 processes, arguments and net log, msedgedriver, Windows network commands |
| `10-home.md` | `10-home.test.mjs` | 6 | Windows Recycle Bin (Mac: Trash) |
| `15-trial-first-run.md` | `15-trial-first-run.test.mjs` | 7 | "Show in File Explorer" (Mac: Finder); debug only |
| `16-about.md` | `16-about.test.mjs` | 2 | Debug only |
| `17-internal-build-release.md` | `17-internal-build-release.test.mjs` | 1 | None |
| `20-idea-vault.md` | `20-idea-vault.test.mjs` | 4 | None |
| `21-idea-vault-flow.md` | `21-idea-vault-flow.test.mjs` | 5 | WebView2 fake microphone; Explorer drag-in not covered |
| `25-recently-deleted-purge.md` | `25-recently-deleted-purge.test.mjs` | 4 | None |
| `26-forced-kill-keeps-work.md` | `26-forced-kill-keeps-work.test.mjs` | 3 | Killing the process tree |
| `30-story.md` | `30-story.test.mjs` | 7 | None |
| `31-story-board-flow.md` | `31-story-board-flow.test.mjs` | 9 | None |
| `40-screenplay.md` | `40-screenplay.test.mjs` | 4 | None |
| `41-screenplay-engine.md` | `41-screenplay-engine.test.mjs` | 8 | System Save and Open dialogs skipped |
| `42-writing-comfort.md` | `42-writing-comfort.test.mjs` | 9 | Windows spelling dictionary |
| `43-several-screenplays.md` | `43-several-screenplays.test.mjs` | 3 | None |
| `44-scene-hub-comments-compare.md` | `44-scene-hub-comments-compare.test.mjs` | 3 | None |
| `50-breakdown.md` | `50-breakdown.test.mjs` | 6 | None |
| `51-breakdown-depth.md` | `51-breakdown-depth.test.mjs` | 6 | None |
| `60-appearance.md` | `60-appearance.test.mjs` | 1 | System mode follows Windows (Mac: macOS appearance) |
| `61-schedule.md` | `61-schedule.test.mjs` | 9 | None |
| `62-call-sheets-exports.md` | `62-call-sheets-exports.test.mjs` | 10 | None |
| `63-every-export.md` | `63-every-export.test.mjs` | 3 | System Save dialogs skipped |
| `70-packages.md` | `70-packages.test.mjs` | 6 | Windows file dialogs through UI Automation |
| `72-ai-assistant.md` | `72-ai-assistant.test.mjs` | 9 | `llama-server.exe` processes and sockets |
| `73-ai-workspaces.md` | `73-ai-workspaces.test.mjs` | 5 | None |
| `74-ai-change-sets.md` | `74-ai-change-sets.test.mjs` | 4 | None |
| `75-ai-crash-safe-apply.md` | `75-ai-crash-safe-apply.test.mjs` | 2 | Killing processes, `llama-server.exe` |
| `76-ai-files-integrity.md` | `76-ai-files-integrity.test.mjs` | 4 | `.dll` engine library, a copy of the build, hard links |
| `78-ai-download.md` | `78-ai-download.test.mjs` | 5 | Free space from Windows |
| `80-notes-tasks.md` | `80-notes-tasks.test.mjs` | 4 | None |
| `85-project-pages.md` | `85-project-pages.test.mjs` | 4 | None |
| `86-visual-planning.md` | `86-visual-planning.test.mjs` | 4 | None |
| `90-visual-baseline.md` | `90-visual-baseline.test.mjs` | 1 | Baseline taken at 150 percent Windows scaling in WebView2 |
| `91-appearance-themes.md` | `91-appearance-themes.test.mjs` | 2 | Colours read from the WebView2 page |
| `95-reachability.md` | `95-reachability.test.mjs` | 2 | None |
| `96-keyboard-and-screen-reader.md` | `96-keyboard-and-screen-reader.test.mjs` | 9 | Windows UI Automation (Narrator's tree) |
| `97-performance.md` | `97-performance.test.mjs` | 4 | Windows performance counters |

Total: 197 scenarios in 41 files.
