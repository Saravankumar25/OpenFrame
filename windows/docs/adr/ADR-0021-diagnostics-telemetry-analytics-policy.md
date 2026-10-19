# ADR-0021: Diagnostics, telemetry and analytics policy

- **Status:** Accepted 2026-10-05 (owner decision: diagnostics, telemetry and analytics; the owner delegated the choice to the recorded recommendation in the first governance session).
- **Date:** 2026-10-05
- **Related:** README "Privacy and data ownership"; Engineering Package Index locked decision "Telemetry";
  `docs/engineering/26-observability.md`; `docs/store/privacy-policy.md`;
  `docs/release/launch-readiness-register.md` (SUP-003, PRIV-006, GTM-010, EXE-13-*)

## Context

Three statements conflict. README: no telemetry, analytics or crash upload, and no background network requests.
Engineering Package Index: "Off by default; optional anonymous diagnostics/crash reports with explicit consent".
Supplied launch register: crash IDs, crash aggregation, a product health scorecard (activation, trial conversion),
and website analytics. The privacy policy and the Store privacy answers must describe exactly one behaviour.

## Decision

1. **The app sends nothing automatically.** No telemetry, analytics, crash upload or update ping without a user click.
2. Support diagnostics are **user-initiated**: an *Export diagnostics* action writes a local bundle (version, OS,
   architecture, error codes, timings, redacted log tail) that the user can open and inspect before attaching it to a
   support request. Redaction tests are release gates. No screenplay text, notes, contacts, prompts or paths.
3. Crash-loop protection and local crash IDs are local features, not uploads.
4. Product health metrics come from **Store dashboards** (Partner Center acquisitions, crashes, ratings) and support
   tickets, not from in-app collection.
5. Website analytics, if used, must be privacy-friendly, disclosed in the policy, and optional to block. No tracking in the app.
6. If the owner later wants opt-in crash reporting, that is a new ADR with consent UX, a named processor and policy changes.

## Consequences

- The privacy policy can state "no data leaves your computer except what you choose to send".
- The health scorecard in the launch register uses Store and support data only.
