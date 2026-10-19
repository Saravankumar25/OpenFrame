# Mac phase M4: screenplay editor spike

Recorded 2026-10-17 on the development Mac (Apple M1, macOS 26.5.2). Test:
`apps/macos/Tests/OpenFrameUITests/EditorSpikeTests.swift` (`bash apps/macos/scripts/test.sh --filter EditorSpikeTests`).

## What was measured

The same script in both engines: 110 scenes, 2,420 paragraphs (scene headings, action, character,
parenthetical, dialogue, transitions), about 120 pages. The caret sits in the middle of the script and 300
keystrokes are typed one by one. Each sample is the engine's own work for one keystroke: the edit plus the
layout of what is on screen. The screen refresh adds the same amount to both engines.

| Engine | Load | Typing p50 | Typing p95 | Worst keystroke |
|---|---|---|---|---|
| Native text engine (TextKit 2, `ScreenplayTextView`) | 174 ms | 5.9 ms | 7.9 ms | 16.4 ms |
| Web view (WKWebView, contenteditable, same layout rules) | 71 ms | 1 ms | 13 ms | 64 ms |

The Windows typing budget is 16 ms a keystroke (`typingBudget.test.ts`). The native engine stays inside it
at p95 and its worst keystroke is at the budget. The web view is faster in the median but has a long tail:
its worst keystroke is four times the budget, which a writer feels as a stutter.

## What the numbers do not show

| Question | Native text engine | Existing web editor in a web view |
|---|---|---|
| Input methods (Hindi, Telugu, Tamil, Kannada, Malayalam, Chinese, Japanese, Korean) | The system's own text input client; marked text and candidate windows work as in every Mac app | Works in WebKit, but the editor's composition handling was tuned for WebView2 on Windows |
| Spelling | The system spelling checker and the person's Mac dictionary, underlines and suggestions built in | Needs the Windows spelling bridge rebuilt for WebKit |
| VoiceOver | Native text view accessibility: lines, words, characters, selection | Web content accessibility inside a native window; workable, less familiar to Mac users |
| Undo grouping | NSTextView undo with system grouping; project undo stays in the core | ProseMirror history, as on Windows |
| Reuse | The diff, sync, rules, keyboard and suggestion logic ported to Swift (88 tests, `ScreenplayModelTests`); about 1,500 lines | About 4,750 lines of editor code reused, plus a new JavaScript bridge in place of Tauri's |
| App Sandbox (Chapter 3) | Nothing to add | A web view with local content and a script bridge, reviewed again for the store |

Not measured on this machine: input methods and VoiceOver were not driven by a person in this session. Both
are system features of NSTextView; they are listed for the M9 accessibility pass and the M10 pilot.

## Choice (provisional, for the owner to confirm)

ADR-0027 open item 1 is the owner's decision. On 2026-10-17 the owner asked for M1 to M5 to be finished
while away and for decisions to be taken independently. The editor is therefore built on the **native text
engine (TextKit 2)**: it meets the typing budget without a long tail, gets input methods, spelling and
VoiceOver from the system, and keeps the shell free of a web view. The choice is recorded as provisional in
`docs/implementation/owner-decisions.md`; reversing it would keep the Swift model port (it is shared logic)
and replace only the text view.
