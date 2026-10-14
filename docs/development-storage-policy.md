# Development storage and workspace hygiene

Permanent policy for every developer and every Claude agent working in `C:\OpenFrame`. Amended 2026-10-09: root causes and the final rules are in sections 7 and 8. It exists because the repository once grew to about 273 GB: roughly 211 GB in `.claude\worktrees` and 60 GB in `target\`, almost all of it duplicated Rust `target/` and `node_modules` trees left behind by temporary agent worktrees. Removing only the reproducible artifacts recovered about 205 GB. This must not happen again.

OpenFrame does not use Docker. Do not add it for storage reasons.

## What is permanent and what is not

Permanent, never deleted by cleanup: source code, commits and branches, local databases, user project data, assets, configuration.

Reproducible, safe to delete and rebuild: Rust `target/`, `node_modules/`, generated packages, caches, logs and temporary outputs. `cargo build` and `npm ci` bring them back.

## Worktrees

- `C:\OpenFrame` is the canonical checkout. Do normal development there.
- Create a worktree only when parallel isolated work needs one. Reuse an existing one before creating another.
- Every agent worktree is temporary. Never leave a finished one in place, and never create a loose source copy without a cleanup plan.
- To remove a worktree, first check it is safe:
  1. `git -C <worktree> status --porcelain` is empty, or its changes are committed or copied.
  2. `git rev-list --count <canonical-branch>..HEAD` shows no unique commits, or the branch is kept.
- Then remove it with Git, never with raw deletion: `git worktree remove <path>`, followed by `git worktree prune`.
- First unlink the worktree's `node_modules` junction on its own: `cmd //c rmdir <path>\node_modules` (this removes the link, not its target). A worktree's `node_modules` is a junction to `C:\OpenFrame\node_modules`, and the npm workspace links inside it lead to `apps/desktop` and `tests/e2e`. On 2026-10-08 a Git Bash `rm -rf` of a leftover worktree folder followed those junctions, emptied the main `node_modules` and deleted 1088 tracked files of the main checkout; they were restored with `git restore` and `npm ci`. Never `rm -rf` a worktree folder.
- Keep a worktree that still holds unique commits or uncommitted work.

## Build artifacts

- Do not let each worktree keep its own full `target/` or `node_modules` after its task ends.
- **Changed 2026-10-08 (owner decision): one target directory per active checkout**, all under `C:\OpenFrame\.build-cache\`. The canonical checkout uses `.build-cache\target`; each agent worktree uses `.build-cache\target-<name>` (for example `target-area14`), set per command with `CARGO_TARGET_DIR`, and compiles through sccache (`RUSTC_WRAPPER=sccache`) so dependency builds are shared. Reason: Cargo hashes workspace crates by their path inside the workspace, so worktrees sharing one target overwrite each other's compiled crates, and Cargo can then treat another branch's crate as up to date; a branch was tested against another branch's code. Delete a worktree's `target-<name>` when the worktree is removed. The next bullet is the earlier rule, superseded where it conflicts.
- ~~One shared Cargo target directory serves the canonical checkout and every worktree~~ (superseded 2026-10-08): `C:\OpenFrame\.build-cache\target`. It is set as the user-level environment variable `CARGO_TARGET_DIR` on the development machine, and `/.build-cache/` is in `.gitignore`. Cargo locks the directory, so parallel builds queue instead of corrupting it. The e2e and MSIX scripts already honor the variable.
- On a new machine, set it once: `[Environment]::SetEnvironmentVariable('CARGO_TARGET_DIR','C:\OpenFrame\.build-cache\target','User')`, then restart the terminal and the Claude app so they pick it up.
- Windows Defender: exclude `C:\OpenFrame\.build-cache` the same way `C:\OpenFrame\target` was excluded, or builds slow down. Only the owner can change that.
- Use a separate directory only when a change alters dependencies or features in a way that would thrash the shared one, and delete it when that work ends.
- Do not run a full dependency install or full compile in every worktree when the canonical build can be reused.
- Periodically delete `target/` and `node_modules` from inactive worktrees.

## Build tools (2026-10-08)

- `.cargo/config.toml` compiles through sccache (`[build] rustc-wrapper = "sccache"`) and links Windows dev and test builds with `rust-lld.exe`. Both apply to every checkout and worktree that has the file.
- Release builds keep the Microsoft linker so the binary hardening flags of the security standard apply: `npm run build`, `npm run bundle` and `npm run perf` run through `scripts/with-msvc-linker.mjs`, and `.github/workflows/release.yml` sets `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=link.exe`. Never build a shipped binary with rust-lld.
- CI runners have no sccache: `ci.yml`, `mac.yml` and `release.yml` set `RUSTC_WRAPPER` to an empty string, which turns the wrapper off.
- Agent builds set `CARGO_INCREMENTAL=0` (sccache cannot cache incremental builds) and use their own `CARGO_TARGET_DIR` under `.build-cache`.
- The sccache cache is capped at 30 GB through the user-level variable `SCCACHE_CACHE_SIZE=30G`. A running sccache server keeps its old limit until it restarts (`sccache --stop-server` when no build runs).

## Build queue (2026-10-08)

- The machine has 16 GB of RAM and 18 logical cores. Five parallel workspace builds left 0.4 GB free and every build swapped. Agents may edit code in parallel, but every memory-heavy cargo command (build, check, clippy, test) runs through `node C:/OpenFrame/scripts/cargo-queued.mjs <cargo args>`.
- The script holds one lock for the whole machine (`.build-cache/cargo-queue.lock`, owner pid, folder, arguments and start time in `owner.json`), so only one heavy build runs at a time. It caps the build at 4 jobs (`OPENFRAME_CARGO_JOBS` overrides) and 4 test threads, releases the lock when cargo ends or on Ctrl+C, and takes over a lock whose holder process is gone. Waiting builds print who holds the lock.
- sccache puts every `CARGO_*` environment variable in its cache key, so a different `CARGO_TARGET_DIR` per worktree made every build miss (measured 2026-10-08 on `openframe-domain`: 0 of 30 hits and 61 to 67 s with the variable, 22 of 30 hits and 31 to 32 s with the flag). The queue therefore removes `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS` from the environment and passes `--target-dir` and `-j` as flags. Build through the queue, or pass `--target-dir` yourself; never rely on a per-worktree `CARGO_TARGET_DIR`.
- `npm run lint` calls cargo clippy directly, so agents run the queued clippy and `npm --workspace apps/desktop run lint` separately. `cargo fmt`, typecheck, Vitest and the governance checks need no queue.
- Testing order (owner decisions 2026-10-08): focused package and test-target runs while building; one full workspace run per branch after merging the latest session branch into it; one full run and E2E on the session branch at the end. Each worktree keeps its own target folder; correctness wins over the extra compile time. Joining test binaries into fewer suites waits until session 2 reaches `main`.

## Storage budget

- Check disk impact before launching several agents or worktrees.
- When temporary worktrees and build output together exceed about 10 GB beyond expected dependencies, inspect and clean before starting more.
- Never hold 50 GB or more of temporary build or worktree data without the owner's explicit approval. The owner approved up to about 100 GB on 2026-10-08 while several worktrees build in parallel; return under 50 GB when that work ends.
- Measure with Bash `du -sh <folder>`. Do not use recursive PowerShell `Get-ChildItem` on `target/`: it can run for over an hour.

## End of task

When a task is integrated into the canonical codebase:

1. Confirm its commits and uncommitted work are preserved.
2. Remove its completed worktrees with `git worktree remove`.
3. Run `git worktree prune`.
4. Delete the `target/`, `node_modules` and cache output that belonged to the completed work.
5. Keep active worktrees that still hold unique work.
6. Confirm OpenFrame still builds and runs locally.
7. Report briefly what was removed and about how much space came back.

## 7. Root causes, measured on 2026-10-09

Why the folders grew, from the files themselves (no deletion was done to find out):

| Cause | Evidence | Effect |
|---|---|---|
| Debug information in test programs | `Cargo.toml` sets `debug = 1` for the development profile. In `.build-cache/target/debug/deps`, 231 `.pdb` files take 18 GB of 31 GB (largest 187 MB) | More than half of the build folder is debug files nobody reads |
| Too many test programs | 46 integration test files in `openframe-application`, 16 in `openframe-import-export`, 5 in `openframe-ai`, 2 in `openframe-security`; each is its own program that links the 150,000-line application crate. 184 `.exe` files take 6.3 GB | Every full test run links the crate dozens of times |
| One huge application crate | `openframe-application` is 150,000 of 195,000 lines of Rust (77 percent); a change to it rebuilds and relinks every test | Slow small changes |
| 632 locked packages and dependencies built at optimisation level 2 in the development profile | `Cargo.lock`; `[profile.dev.package."*"] opt-level = 2` | Slower first builds, larger libraries (5.8 GB) |
| A target folder per worktree | Oct 7: 273 GB (211 GB in worktrees, 60 GB in `target`). Oct 8: each worktree target 29 to 41 GB, up to 100 GB approved | Hundreds of gigabytes within a day |
| Worktrees outside git's list | 39 folders under `.claude/worktrees` (4.2 GB, one holds a 2.7 GB model copy); their `.git` files point to `C:/dev/OpenFrame`, which does not exist; `git worktree list` shows only `main` | Orphan source copies git cannot manage |
| Memory pressure | 15.4 GB of memory and 18 logical cores; `CARGO_BUILD_JOBS` unset; five parallel builds left 0.4 GB free; on 2026-10-05 a Windows service held 39 GB and builds failed with "memory allocation failed" | Swapping, hours-long builds |
| sccache not proved | `sccache --show-stats` shows 0 requests on 2026-10-09; the queue script already passes flags so the cache can hit | Unknown cache benefit |
| Parallel agents | 31 `worktree-agent-*` branches from 2026-09-30, 37 more merged and deleted on 2026-10-09; many "restore LF" commits | Disk, merge work and Windows line-ending churn |

## 8. Final rules (owner approval of 2026-10-09 pending for the build setting)

1. **No worktrees by default.** Work in `C:\OpenFrame` on a branch. A worktree is created only when two pieces of work must
   run at the same time and cannot share one branch; it has an owner, a reason written in the handoff file, and an end.
2. **No parallel agents** unless the owner asks, and then only with a file-ownership map and not more than two at once on
   this machine.
3. **One target folder** (`.build-cache/target`), built through `scripts/cargo-queued.mjs` (4 jobs, one heavy build at a
   time). Never a `target` inside a worktree.
4. **Lower the debug level** of the development profile (`debug = 0`, or `line-tables-only`) and the test profile. Expected
   saving: about 18 GB. This is a code-configuration change: it needs the owner's approval and one before-and-after
   measurement of build time and of debugging usefulness.
5. **Fewer test programs.** Merge the integration test files of each crate into a few programs (already a build plan item,
   area 5). Expected saving: several GB and much less linking.
6. **Test by size of change.** Targeted tests while coding; the full set once before a merge; the end-to-end suite once per
   set of areas ([testing strategy](testing/README.md)).
7. **Finish, test, merge, verify, remove.** A worktree is removed only after: its work is merged and the merged code was
   tested on `main`; `git status` in it is clean or the changes are saved; `git rev-list --count main..<branch>` is zero or
   the branch is kept on purpose; its `node_modules` junction is unlinked first. A branch that "looks merged" is not enough.
8. **Dead worktree folders** (those git does not list) are reviewed one by one: compare against the matching branch
   (`git diff --no-index`), save any unique file, and only then remove the folder. The review table is made first and
   approved by the owner; nothing is deleted before.
9. **Budget alarm.** Above 10 GB beyond dependencies, stop and inspect. Above 50 GB, stop and ask. Check the sizes at the
   start and end of every session with `du -sh .build-cache .claude/worktrees`.
10. **Reproducible, not valuable:** `target`, `node_modules`, `.build-cache`, `graphify-out`, packages, caches, logs. **Valuable:**
    source, commits, branches, databases, user data, `.ai-assets` (1.1 GB, not in git, needed to build), evidence files.
