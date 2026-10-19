# Worktree review (Windows phase W1)

> Written 2026-10-16 for the owner's decision. Nothing has been deleted. Read-only review produced by a script that never
> follows links or junctions. Rule: [development-storage-policy.md](../development-storage-policy.md) and `CLAUDE.md`
> ("Storage and workspace hygiene").

## 0. Outcome

The owner approved removal in chat on 2026-10-16 ("if it is safe you can remove the w1 working trees"). Option 1 was
done: no folder held a junction or link; the 65 files of section 2 (without models and llama.cpp builds) were copied to
`C:OpenFrame.build-cachesalvage-2026-09-30` (773 KB); the 39 folders were removed and `git worktree prune` run.
`git worktree list` shows only `C:OpenFrame`. About 4.0 GB freed.

## 1. What was found

- `C:\OpenFrame\.claude\worktrees` holds 39 folders, about 4.0 GB, all last changed on 2026-09-29 or 2026-09-30.
- Each folder's `.git` file points at `C:/dev/OpenFrame/.git`, which no longer exists. Git cannot see these folders
  (`git worktree list` shows only `C:\OpenFrame`), and the 31 agent branches they were on no longer exist anywhere. Their
  commits cannot be recovered; only the files on disk remain.
- None has a `node_modules` folder or junction, so removing them cannot touch the main checkout's files. One folder
  (`agent-a30f8e94eba53611c`, 2.67 GB) holds downloaded model files and llama.cpp builds in `.dev-models/`.
- Compared with `main` today, every folder shows hundreds of changed files: they are older copies of the code. Most files
  that exist only in these folders were renamed or removed on purpose since then (the specifications moved to
  `docs/specs/`, the download flow was removed by ADR-0025, the web build output and test runs are generated).

## 2. Files that exist only in these folders and might still be useful

| Files | In how many folders | Why they might matter |
|---|---|---|
| `crates/openframe-ai/src/{download,manifest,job}.rs`, `tests/{download,install,lifecycle,security_ai}.rs`, `tools/sign-manifest.mjs`, `manifest/dev-manifest.json(.sig)`, `keys/manifest-dev.pub` | 29 | The model download, signed manifest and install code removed under ADR-0025. ADR-0026 brings the download back in phase W8, so this is a useful reference (the dev key is a test key, not a secret) |
| `apps/desktop/src/ai/OfflineAiSetup.tsx` (with test and styles) | 9 | The old download screen; reference for W8 |
| `docs/manual-audit/*.md` (10 chapters) and tests named `*_manual_audit.rs`, `manual_ch1_projects.rs`, `schedule_manual.rs`, `screenplay_manual_ch4b.rs` | 1 to 2 | A chapter-by-chapter audit of the user manual against the app |
| `crates/openframe-application/src/modules/{access,project_tools}.rs`, `modules/ai/direct.rs`, `modules/idea_vault/{export,links,preview}.rs`, `modules/screenplay/paste.rs`, `crates/openframe-import-export/src/sheet.rs`, `tests/{access,ai_eval,ai_security}.rs`, `crates/openframe-test-support/src/{eval,real_ai}.rs` and a few screens | 1 to 2 | Earlier versions of modules that were later merged, renamed or dropped. `ai_eval.rs` and `eval.rs` may help the W5 evaluation harness |
| `.dev-models/*.gguf`, llama.cpp CPU and Vulkan builds, `bench-cpu.log`, `benchmark-cpu.json` | 1 | Model files (re-downloadable) and the 2026-09-30 benchmark output |

Not worth keeping: `apps/desktop/dist/` (build output), `apps/desktop/src/ipc/generated/` (generated), `tests/e2e/.run/`
and `tests/e2e/artifacts/` (old test runs), `docs/spec-digest/` and the root specification copies (now in the repository).

## 3. Options for the owner

1. **Recommended:** copy the files of section 2 (without the model files and llama.cpp builds) into one git-ignored folder,
   `C:\OpenFrame\.build-cache\salvage-2026-09-30\`, keep the two benchmark files with them, then remove the 39 folders.
   Frees about 4.0 GB and keeps every file that might still matter.
2. Remove the 39 folders without copying anything. Frees about 4.0 GB; the download code would be rewritten in W8.
3. Keep everything for now.

## 4. Folder table

| Folder | Size (GB) | Newest file | Files | Differ from main | Only in folder | node_modules | Git folder exists | Examples |
|---|---|---|---|---|---|---|---|---|
| `agent-a056d60c8c58fe10a` | 0.03 | 2026-09-30 | 1129 | 538 | 25 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a121d421037a60ee1` | 0.03 | 2026-09-30 | 1055 | 516 | 24 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a12f9bac5a7a17118` | 0.03 | 2026-09-30 | 763 | 404 | 24 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a1d9f0f257c97598d` | 0.03 | 2026-09-30 | 1395 | 702 | 49 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a1f24b89db7c9930c` | 0.03 | 2026-09-30 | 1465 | 713 | 60 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a22ee4d0c76866377` | 0.03 | 2026-09-30 | 1395 | 699 | 49 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a2ebe5aeedc8c1eba` | 0.03 | 2026-09-30 | 1459 | 716 | 52 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a30f8e94eba53611c` | 2.67 | 2026-09-30 | 1552 | 708 | 159 | none | no | ~.cargo/config.toml +.dev-models/bench-cpu.log +.dev-models/benchmark-cpu.json |
| `agent-a39fbaec2d0ad4d61` | 0.03 | 2026-09-30 | 1393 | 699 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a445bb6ae89883664` | 0.03 | 2026-09-30 | 1482 | 718 | 74 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a448e26a0331f0737` | 0.03 | 2026-09-29 | 433 | 301 | 23 | none | no | ~.cargo/config.toml ~.gitignore +All required docs.txt |
| `agent-a4df1819727d6f551` | 0.03 | 2026-09-30 | 1483 | 736 | 76 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a537c04740ddf616b` | 0.03 | 2026-09-30 | 1390 | 699 | 46 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a542a17030ec2b947` | 0.03 | 2026-09-30 | 1462 | 713 | 55 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a58215c6da549d5bd` | 0.03 | 2026-09-30 | 1393 | 699 | 49 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a7646e3d82654306e` | 0.03 | 2026-09-30 | 1393 | 700 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a8365de4aa1aae86f` | 0.03 | 2026-09-30 | 1405 | 697 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a83e8383611d0a9fb` | 0.03 | 2026-09-30 | 1123 | 552 | 44 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a850f871a3bc113b9` | 0.03 | 2026-09-30 | 758 | 405 | 23 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a903dd54b4634b100` | 0.03 | 2026-09-30 | 1392 | 699 | 46 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a97e6b5196cc14a9d` | 0.03 | 2026-09-30 | 1458 | 714 | 53 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a9ac7687867e2430e` | 0.03 | 2026-09-30 | 1394 | 702 | 46 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-a9c088992afcb8ea1` | 0.03 | 2026-09-30 | 740 | 448 | 23 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ab28abc87d577cb93` | 0.03 | 2026-09-30 | 1301 | 687 | 44 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ab530c9e726e924ed` | 0.03 | 2026-09-30 | 1447 | 708 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ac397d906000ba0ba` | 0.03 | 2026-09-29 | 408 | 312 | 23 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ac3a22f72fbe3c513` | 0.03 | 2026-09-29 | 446 | 294 | 23 | none | no | ~.cargo/config.toml ~.gitignore +All required docs.txt |
| `agent-ac5d5e878c2becd0a` | 0.10 | 2026-09-30 | 1399 | 702 | 46 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ad0a8cd1ce40a005a` | 0.03 | 2026-09-30 | 1431 | 703 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-adc524f412b26be64` | 0.03 | 2026-09-30 | 1392 | 700 | 48 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ae1789659b9c31aa9` | 0.03 | 2026-09-30 | 1461 | 716 | 54 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ae54d9a489645b850` | 0.03 | 2026-09-30 | 1391 | 699 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-ae8d02b3439289f50` | 0.04 | 2026-09-30 | 709 | 378 | 23 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-aedb79c71635a275f` | 0.03 | 2026-09-30 | 1272 | 672 | 44 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-af0fb1a2fbf7174b0` | 0.03 | 2026-09-30 | 1394 | 699 | 50 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-af148aa4fef835aef` | 0.03 | 2026-09-30 | 1456 | 712 | 48 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-af6ea2d0e33b7def2` | 0.03 | 2026-09-29 | 480 | 315 | 23 | none | no | ~.cargo/config.toml ~.gitignore +All required docs.txt |
| `agent-af81d95ac006c197f` | 0.12 | 2026-09-30 | 1289 | 663 | 45 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
| `agent-aff19399e76a98c51` | 0.03 | 2026-09-30 | 1394 | 701 | 47 | none | no | ~.cargo/config.toml ~.github/workflows/ci.yml ~.github/workflows/release.yml |
