# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. It exists so the next session starts from the facts
> instead of from a long history, which is how context rot is avoided. If this page is older than the newest progress log
> entry, trust the log and fix this page first. Last rewritten: 2026-10-18, end of the W9 endgame session.

## State

1. **`main`** holds W1 to W9 (`windows/w9-endgame` merged locally on 2026-10-18 at the owner's word, fast-forward to d4acb83). Nothing pushed. The final
   installer was built from commit 028d8b6; later commits changed only tests, documents and the pilot kit.
2. **Final internal installer:** `release-out\OpenFrame-Studio-0.1.0-internal-028d8b6.exe`, 27,016,597 bytes, SHA-256
   `97b712214109e5812ce87cf226a4bd234f25b3fac65b68735e97fea0d787d50f`. Clean tree, team gate on, scan-installer OK
   (53 files), check-binary OK, Defender no threats. It replaces 995bfac, which was rebuilt after a security review found
   that the uninstaller could remove the whole app data folder (fixed in 6f9a280).
3. **Release gate** ([windows-internal-release-gate.md](../release/windows-internal-release-gate.md)), 38 rows: 20 Pass,
   18 Exception (owner approved), 0 Open. G2 passed with one written deviation (spec 04 helper precondition on the
   installed program, no text lost; [installed-run.md](../release/evidence/w9/installed-run.md)). P1 passed with notes
   ([memory-soak.md](../release/evidence/w9/memory-soak.md)).
4. **Pilot kit** (`docs/testing/pilot-kit/`) is simplified for non-technical writers: no terminal or hash for testers.
   The owner checks the fingerprint once.
5. **Mac package:** `release-out\openframe-dmg.zip` (69,954,965 bytes, SHA-256
   `bf65166b5f786b7a7098bcace13166203af281e799c1b6008b41707103ea045b`): source at 028d8b6, a verified git bundle with all
   branches, and `MAC-START-HERE.md`. gitleaks flagged only fake sandbox tokens in
   `services/licence-server/tests/fixtures/mac-validate-v1.json` (signed by a made-up test root CA). The Mac and App
   Sandbox readiness audit of the core was not done this session. It is the first M1 work, listed in `MAC-START-HERE.md`.
6. **Storage:** the OpenFrame folder is about 12 GB (it was about 110 GB; Explorer showed 347 GB because of hard-linked
   models). The mockups worktree is removed; its branch `claude/website-html-mockups-9ecfa7` keeps 462 HTML snapshots,
   not merged. Rule 11 in [development-storage-policy.md](../development-storage-policy.md).

6. **Manual Testing Guide** (owner request, 2026-10-18): `docs/manual/OpenFrame_Studio_Manual_Testing_Guide.docx` (181 figures,
   1,889 test cases) with a CSV and `docs/manual/test-data/`. Give it to the W10 testers with the installer. Findings for the
   owner are in the 2026-10-18 progress log entry and its appendix K.

## Next three items

1. Give the 028d8b6 installer and the pilot kit to the team (W10).
2. Collect the reports, triage with `docs/testing/pilot-kit/triage.md`, fix, and rebuild with `npm run installer:internal`
   (only when the owner says so; a rebuild re-opens G1 to G4).
3. After sign-off, freeze with `docs/release/w10-freeze-steps.md`. Then start the Mac track from `MAC-START-HERE.md`.

## Open blockers

- None on the owner for the Windows gate. The team pilot reports are the next input.

## Do not

- Do not push and do not merge to `main` without the owner's word.
- Never share the 995bfac or d271894 installer files.
- Do not ask for the team password (decided; the owner set it on the build laptop).
- Do not build a new installer unless app code changes and the owner says so.
- Do not re-work W1 to W8 (owner decision 2026-10-18): no AI language tuning, no 679-case rerun, no re-fuzzing.
- Do not raise licensing or ask about the model set again (decided).
- Delete `tests/e2e/.run` after each final run.
