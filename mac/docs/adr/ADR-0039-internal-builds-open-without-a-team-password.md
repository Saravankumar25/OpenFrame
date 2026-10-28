# ADR-0039: Internal builds open without a team password

- **Status:** Accepted 2026-10-27 (owner decision in chat).
- **Date:** 2026-10-27
- **Supersedes in part:** [ADR-0024](ADR-0024-internal-team-builds.md) (the team gate decisions) and
  [ADR-0030](ADR-0030-verification-ladder-and-release-flow.md) decision 5.

## Context

Internal builds (the Windows installer and the Mac disk image) asked for a team password before anything else ran,
so that only the team could open them. The owner decided the gate is no longer needed.

## Decision

1. Internal builds open straight to the app. There is no team password, no password screen and no hash in the binary.
2. The gate is removed with all its code from both apps: the core operations `team_gate.status` and
   `team_gate.unlock`, the per-operation guard, the build-time hash, the password screens, the release scripts'
   password requirement, the end-to-end password plumbing and the team-password secret-scan rule.
3. Internal builds stay "not for sale": licence, trial and store code are unchanged.

## Consequences

- Anyone who has an internal build can open it. Builds are shared with the team only, as before.
- Release scripts no longer need a secret, so a build can be made and installed without one.
- The Windows tree repeats the same removal (ADR-0034); the list is in the Mac progress log entry of 2026-10-27.
