# Triage

How the owner and the next session sort pilot reports. One report, one row.

## Severity, in order

Any one of the five blocking classes blocks the Windows freeze until it is fixed and re-checked.

Blocking classes:
1. Loses data.
2. Corrupts a project.
3. Fails a migration or a recovery.
4. The AI changes the project on its own.
5. Exposes private notes.

Then:
- **Major:** a main workflow cannot be finished, or a result is wrong.
- **Minor:** the workflow can be finished but is confusing or slow.
- **Cosmetic:** looks wrong, nothing else.

Installation warnings (SmartScreen) and ideas are logged but are not defects.

## Rules

- Read the report and the attached diagnostics. Try to reproduce it. Note "not reproduced" if you cannot.
- Every fix gets a test that fails without the fix and passes with it.
- The person who found the problem re-checks the fix in the new build. Nobody else closes it.
- A fix build needs the owner's word. Never push without it.
- Blocking reports stop work for that tester until a fix or a workaround exists.

## Tracking table

| id | reporter | severity | area | build | status | fixed in | rechecked by |
|---|---|---|---|---|---|---|---|
| P-001 | | blocking / major / minor / cosmetic | | internal-<commit> | open / fixed / rechecked / wont-fix | internal-<commit> | |

Status words: open, fixed (waiting for re-check), rechecked (closed), wont-fix (needs an owner decision, written down).
