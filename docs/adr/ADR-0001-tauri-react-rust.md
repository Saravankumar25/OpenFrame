# ADR-0001: Tauri v2 + React/TypeScript + Rust

- **Status:** Accepted (implemented)
- **Date:** 2026-09-30
- **Deciders:** Engineering, per the locked decisions in the Engineering Package Index
- **Related:** ESD §2–§5, §15; ADR-0002, ADR-0004

## Context

OpenFrame is a local-first desktop application for Windows 11 first, with macOS-ready boundaries. It must:

- start fast and use little memory when idle;
- run heavy structured editing (Story Board, screenplay, stripboard);
- keep user data safe under crashes;
- work fully offline;
- later host a local AI sidecar. (LAN collaboration was removed; see ADR-0007.)

The Engineering Package Index locks the shell (Tauri v2), the UI (React + TypeScript) and the rule that Rust owns
domain logic and React is presentation and input only.

## Decision

- **Shell:** Tauri v2 (`apps/desktop/src-tauri`). It uses the OS WebView2 on Windows and a single Rust process.
- **UI:** React 19 + TypeScript + Vite 7, Tailwind CSS v4 with OpenFrame design tokens (`styles/openframe.css`
  reuses mock-up classes), Radix primitives for accessible menus and dialogs, Zustand for view/session state
  only, TanStack Query as an IPC cache (never truth), TanStack Virtual, dnd-kit and ProseMirror.
- **Core:** a Rust workspace (edition 2024, MSRV 1.85). Tauri is an adapter only (`src-tauri/src/lib.rs`, ~200
  lines). All behavior lives in the `openframe-*` crates, which do not depend on Tauri. The whole application
  layer can therefore be tested headless through `openframe-test-support::TestEnv`.
- **Dependency direction:** `openframe-domain` (no I/O) ← `persistence` / `project-format` / `security` ←
  `application` ← `src-tauri`. React talks only to `of_invoke` (ADR-0004).
- **Types across the boundary:** generated from Rust with `ts-rs` into `apps/desktop/src/ipc/generated`.

## Consequences

- Small installers and low idle memory compared with bundling Chromium. WebView2 is a runtime dependency: it ships
  with Windows 11, and the installer uses the WebView2 download bootstrapper where it is missing.
- There are two languages. The contract between them is generated (ts-rs), and CI fails on drift.
- The webview is untrusted by design. A strict CSP, `freezePrototype` and a minimal capability set are practical
  because no business logic needs webview privileges.
- A macOS build later needs only the adapter, installer and path adapters (ESD §14). Domain, persistence, project
  format and IPC contract are portable.
- UI performance depends on the webview. Budgets are in `docs/engineering/21-performance-budgets.md`.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Electron | Bundles Chromium + Node (≈ 150 MB+, higher idle RAM). A Node main process with fs/shell authority is a larger attack surface. Explicitly rejected in ESD §15. |
| Native WinUI 3 / WPF | No macOS path, and slower iteration on a mock-heavy UI |
| Flutter / Qt | Weaker fit for rich-text screenplay editing (ProseMirror) and the team's web design system. Qt adds licensing complexity. |
| Next.js / server runtime, Python backend | A server process is not needed for a single-user desktop app. Rejected in ESD §15. |
