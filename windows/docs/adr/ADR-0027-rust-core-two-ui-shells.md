# ADR-0027: One Rust core, two shells (Tauri on Windows, SwiftUI on the Mac)

- **Status:** Accepted 2026-10-15 (owner, in chat). Open sub-decisions are listed at the end and block only the Mac track.
- **Date:** 2026-10-15
- **Amends:** [ADR-0001](ADR-0001-tauri-react-rust.md) (the Mac shell is no longer Tauri) and
  [ADR-0017](ADR-0017-cross-platform-windows-and-macos.md) (one codebase and two platform adapters becomes one Rust core
  and two shells). ADR-0017 decisions 2 (Windows 11 only) and 3 (Apple silicon only) stay in force.
- **Related:** [ADR-0004](ADR-0004-single-ipc-dispatcher.md) (the single door stays the contract between shell and core),
  [ADR-0026](ADR-0026-offline-ai-download-after-install.md), [ADR-0028](ADR-0028-ai-quality-program-and-harness.md),
  [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md), [Mac phase plan](../roadmap/chapter-1b-mac-phases.md).

## Context

OpenFrame was built as React screens inside a Tauri shell over a large Rust core (about 195,000 lines of Rust, 41,000
lines of screens). The owner wants the Mac application to be a native SwiftUI application that can use Apple's own
on-device model, and wants the Windows application finished completely first.

A full rewrite of the core in Swift would throw away the most tested part of the product and would have to be proved on
hardware that is not yet available. The screens already reach the core through one door (`of_invoke`, ADR-0004) with JSON
in and JSON out, plus an event stream. That narrow contract is what makes a second shell possible without touching the
rules, the database or the AI safety path.

## Decision

1. **The Rust core is shared.** All rules, storage, undo, search, parsers, exports and the AI safety path stay in the Rust
   crates and are used unchanged by both platforms. No logic is rewritten in Swift.
2. **Windows shell: Tauri v2 with React and TypeScript.** Unchanged.
3. **macOS shell: SwiftUI.** The Mac application is a native Swift application. It calls the Rust core through a narrow
   interface generated with UniFFI (a function that sends an operation name and arguments and returns a result, plus a
   callback for events), packaged as an XCFramework. The Mac shell holds no rules. The Tauri Mac shell is not shipped and
   is retired in the first Mac phase.
4. **Same contract, one source of truth.** The operation registry stays the only door. The generated TypeScript bindings and
   the generated Swift types come from the same registry metadata, so the two shells cannot drift.
5. **Platform-specific code stays behind the platform boundary** ([37-platform-boundary.md](../engineering/37-platform-boundary.md)).
   Feature code in the core never contains operating-system calls.
6. **Order of work.** The Windows application is finished, tested end to end, built as an internal `.exe` and tested by the
   team before the Mac shell is built ([ADR-0030](ADR-0030-verification-ladder-and-release-flow.md)). The Mac shell starts
   from a frozen, proved Windows core.
7. **Release targets do not change.** Windows 11 only. Apple silicon only. The owner's Intel MacBook is a development and
   verification machine, never a release target (the Mac phase plan says what it can and cannot prove).
8. **AI engine behind a provider interface.** The core talks to the AI through an `InferenceProvider` interface (chat with a
   grammar or typed output, embeddings, health, capabilities such as context size and languages). The first provider is the
   bundled `llama.cpp` runtime. An Apple Foundation Models provider is added on the Mac as an opt-in choice
   ([ADR-0028](ADR-0028-ai-quality-program-and-harness.md)). The interface is built on the Windows track so the Mac track
   adds a provider and does not restructure the harness.

## Consequences

- The Mac track is mostly a new screen layer. Its cost is dominated by the screenplay editor, the stripboard and calendar
  interactions, the storyboard canvas and the PDF preview. The editor is risk-ordered first inside the Mac track.
- Every Windows end-to-end scenario is listed in a shared scenario file. The Mac track re-implements each scenario in
  Apple's UI testing framework against the same list, so parity is checkable.
- Two shells mean two sets of screen tests. The Rust core tests stay single.
- Engineering documents 28, 29 and 37 describe the Tauri-on-both plan. Their Mac statements are history; the Mac phase plan
  is current.

## Open sub-decisions (owner; decided inside the Mac track)

1. **Screenplay editor in SwiftUI:** a native text engine, or the existing editor kept in a small web view. The owner chose
   "explain the trade-offs first". Decided after the editor spike in Mac phase 4, with the spike's numbers.
2. **How the Mac app runs the AI engine:** a separate helper process (crash isolation) or `llama.cpp` linked as a library
   (simpler under the App Sandbox). Decided in the provider design (Mac phase 8) after reading Apple's sandbox rules.
3. **The macOS version of the owner's Intel MacBook** decides which Xcode and which checks can run there. Owner task.
