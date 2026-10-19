# 26 — Observability: Logs, Diagnostics & Crash Reporting

> **Status legend:** ✅ implemented · 📋 planned · ⛔ blocked on an open decision.
> Source: `apps/desktop/src-tauri/src/lib.rs` (`init_logging`, `of_invoke` timing),
> `crates/openframe-security/src/lib.rs` (`redact`), `openframe-application` (`tracing` calls).

## 1. Principles

1. **Local only.** Logs never leave the machine unless the user explicitly exports them. There is no telemetry,
   analytics or remote crash reporting in the product today.
2. **No project content in logs.** Log events record operation names, error codes, durations, counts and internal
   identifiers, never titles, script text, notes, file names chosen by the user, or search text. `AppError.detail`
   must never contain project content (CONTRIBUTING §3). Redaction (`security::redact`) is defence in depth, not the
   primary control. **Known gap:** a few warnings include an OS error string (e.g. "could not remove purged file" in
   `store.rs`). Such strings can contain the project folder path, and the folder name is the project title. These
   should log the error kind/code only (small foundation change).
3. **Opt-in only** for anything that could ever be transmitted (Engineering Index: "optional anonymous
   diagnostics/crash reports with explicit consent"). The collection service is an open decision ⛔.

## 2. Local logs ✅

| Property | Value |
|---|---|
| Location | `%LOCALAPPDATA%\OpenFrame\logs\openframe.YYYY-MM-DD.log` |
| Format | JSON lines (`tracing-subscriber` JSON formatter): `timestamp`, `level`, `target`, `fields`, `message` |
| Rotation / retention | Daily rotation, **14 files** kept (`tracing-appender` rolling builder) |
| Writer | Non-blocking background writer. The guard is held for the process lifetime and flushed on exit. |
| Level control | `OPENFRAME_LOG` env var, `EnvFilter` syntax, default `info` (e.g. `OPENFRAME_LOG=debug`, `info,openframe_application=debug`) |
| Open from the app | `of_reveal_path { kind: "logs" }` reveals the folder in Explorer |

**What is logged (info):** startup with app version; operations that **failed** (`op`, `code`, `ms`); operations
slower than **250 ms** (`op`, `ms`); storage failures in the mutation pipeline (`code`, `action`); project not closing
cleanly; failed event delivery; background task failures (`code`, `kind`); search rebuild failure; migration checksum
drift; lock release (debug).

**Debug level:** every operation with its duration.

**Panics:** a panic hook logs `location` (source file:line) and the panic message passed through `redact()` (home paths
→ `%USERPROFILE%`, e-mails → `[email]`, `token|key|secret|password|authorization=…` → `[redacted]`). The default hook
runs afterwards. Background task panics are caught and reported as `internal.unexpected` without crashing the app.

**Frontend:** `reportError` shows the human message and writes `[op] code detail` to the webview console (visible in
dev tools only, not persisted). Frontend logs are not written to disk in v1.

## 3. User-facing diagnostics 📋

- **Help → "Open logs folder"** (uses `of_reveal_path logs`) and **"Copy diagnostic summary"**: app version,
  Windows version, WebView2 version, schema version of the open project, and the last N error codes with times. No
  content, no paths beyond `%USERPROFILE%`-relative ones.
- **"Create diagnostic bundle"**: a ZIP the user saves and sends manually, containing the redacted logs and the
  summary above. It is never uploaded automatically. The user can inspect it before sending.
- **Project health check**: `integrity_problems(full=true)` + `foreign_key_problems()` + asset availability scan,
  reported in plain language (read-only, no repair without confirmation).

## 4. Crash reporting ⛔📋

The Security spec leaves crash-report consent unspecified (digest §26 item 6), and the Engineering Index lists the
collection service as an open decision. Until both are decided:

- **No crash report is ever transmitted.** Native crashes (outside Rust panics) are handled by Windows Error
  Reporting as for any app. OpenFrame does not configure WER upload.
- If adopted later: off by default; explicit per-install consent in settings with a preview of exactly what is sent;
  a minidump/stack trace only, with no project content and no file paths beyond redacted forms; a random per-install
  id that can be reset; documented retention. This needs an ADR and a privacy review first.

## 5. Metrics

There are no runtime metrics collection or counters shipped to anyone. Performance budgets are measured in
development and release-candidate testing (21-performance-budgets.md), using the `slow operation` log lines and
traces.
